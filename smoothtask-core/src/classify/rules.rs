//! Классификация процессов и AppGroup по паттернам из конфигурационных файлов.
//!
//! Паттерны загружаются из YAML файлов в директории patterns/ и используются
//! для определения типа процесса (process_type) и тегов (tags).

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::num::NonZeroUsize;
use std::sync::{Arc, Mutex};
use tracing::{debug, error, info, warn};

use lru::LruCache;

#[cfg(any(feature = "catboost", feature = "onnx"))]
use crate::classify::ml_classifier::MLClassifier;


use crate::logging::snapshots::{AppGroupRecord, ProcessRecord};

/// Категория паттернов (browser, ide, terminal, batch, и т.д.).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PatternCategory(pub String);

/// Паттерн для одного приложения.
///
/// Паттерн определяет правила сопоставления процесса с приложением на основе
/// имени исполняемого файла, desktop ID и cgroup пути.
///
/// # Примеры
///
/// ```yaml
/// name: firefox
/// label: Mozilla Firefox
/// exe_patterns:
///   - "firefox"
///   - "firefox-*-bin"
/// desktop_patterns:
///   - "firefox.desktop"
/// cgroup_patterns:
///   - "*firefox*"
/// tags:
///   - "browser"
///   - "gui"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppPattern {
    /// Уникальное имя приложения (например, "firefox", "vscode").
    pub name: String,
    /// Человекочитаемое название.
    pub label: String,
    /// Паттерны для сопоставления с exe/comm процесса.
    /// Поддерживаются wildcard символы: `*` (любые символы) и `?` (один символ).
    #[serde(default)]
    pub exe_patterns: Vec<String>,
    /// Паттерны для сопоставления с desktop-файлом.
    /// Поддерживаются wildcard символы: `*` и `?`.
    #[serde(default)]
    pub desktop_patterns: Vec<String>,
    /// Паттерны для сопоставления с cgroup_path.
    /// Поддерживаются wildcard символы: `*` и `?`.
    #[serde(default)]
    pub cgroup_patterns: Vec<String>,
    /// Теги, которые будут присвоены процессу при совпадении.
    /// Теги используются для классификации и фильтрации процессов.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Файл с паттернами одной категории.
///
/// Каждый YAML файл в директории паттернов должен содержать структуру PatternFile
/// с категорией и списком паттернов приложений.
///
/// # Пример структуры YAML файла
///
/// ```yaml
/// category: browser
/// apps:
///   - name: firefox
///     label: Mozilla Firefox
///     exe_patterns: ["firefox", "firefox-*-bin"]
///     tags: ["browser", "gui"]
///   - name: chromium
///     label: Chromium
///     exe_patterns: ["chromium", "chromium-browser"]
///     tags: ["browser", "gui"]
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternFile {
    /// Категория паттернов (например, "browser", "ide", "terminal").
    pub category: PatternCategory,
    /// Список паттернов приложений в этой категории.
    pub apps: Vec<AppPattern>,
}

/// Результат обновления паттернов.
///
/// Содержит статистику об изменениях при перезагрузке паттерн-базы.
/// База паттернов для классификации процессов.
///
/// PatternDatabase загружает паттерны из YAML файлов и предоставляет методы
/// для поиска паттернов, соответствующих процессам.
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::classify::rules::PatternDatabase;
/// use std::path::Path;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// // Загрузка паттернов из директории
/// let db = PatternDatabase::load(Path::new("/path/to/patterns"))?;
///
/// // Поиск паттернов для процесса
/// let matches = db.match_process(
///     Some("firefox"),
///     Some("firefox.desktop"),
///     Some("/user.slice/user-1000.slice/session-2.scope")
/// );
///
/// for (category, pattern) in matches {
///     println!("Found pattern: {} in category {}", pattern.name, category.0);
/// }
/// # Ok(())
/// # }
/// ```
/// Тип для ключа кэша сопоставления паттернов.
/// Представляет комбинацию идентификаторов процесса: исполняемый файл, desktop ID и путь cgroup.
#[allow(clippy::type_complexity)]
type PatternMatchKey = (Option<String>, Option<String>, Option<String>);

/// Тип для результата сопоставления паттернов.
/// Представляет список категорий и паттернов, соответствующих процессу.
type PatternMatchResult = Vec<(PatternCategory, AppPattern)>;
#[derive(Clone, Debug)]
pub struct PatternDatabase {
    /// Маппинг категория -> список паттернов.
    patterns_by_category: HashMap<PatternCategory, Vec<AppPattern>>,
    /// Плоский список всех паттернов для быстрого поиска.
    all_patterns: Vec<(PatternCategory, AppPattern)>,
    /// Кэш для результатов сопоставления паттернов.
    /// Ключ: (exe, desktop_id, cgroup_path), Значение: Vec<(PatternCategory, AppPattern)>
    match_cache: LruCache<PatternMatchKey, PatternMatchResult>,
}

impl Default for PatternDatabase {
    fn default() -> Self {
        Self {
            patterns_by_category: HashMap::new(),
            all_patterns: Vec::new(),
            match_cache: LruCache::new(NonZeroUsize::new(512).unwrap()),
        }
    }
}



impl PatternDatabase {
    /// Загружает паттерны из директории с YAML файлами.
    ///
    /// Функция сканирует указанную директорию, находит все YAML файлы (`.yml` или `.yaml`),
    /// парсит их и загружает паттерны в базу данных.
    ///
    /// # Аргументы
    ///
    /// * `patterns_dir` - путь к директории с YAML файлами паттернов
    ///
    /// # Возвращает
    ///
    /// База данных паттернов или ошибку при загрузке/парсинге.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::classify::rules::PatternDatabase;
    /// use std::path::Path;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let db = PatternDatabase::load(Path::new("configs/patterns"))?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Ошибки
    ///
    /// Функция возвращает ошибку, если:
    /// - директория не существует или недоступна для чтения
    /// - YAML файл имеет неверный формат
    /// - структура паттерна не соответствует ожидаемой
    /// - паттерн содержит недопустимые данные
    pub fn load(patterns_dir: impl AsRef<Path>) -> Result<Self> {
        let patterns_dir = patterns_dir.as_ref();
        let mut patterns_by_category = HashMap::new();
        let mut all_patterns = Vec::new();

        info!("Загрузка паттернов из директории: {:?}", patterns_dir);

        // Проверяем, существует ли директория
        if !patterns_dir.exists() {
            error!("Директория с паттернами не существует: {:?}", patterns_dir);
            return Err(anyhow!(
                "Директория с паттернами не существует: {:?}",
                patterns_dir
            ));
        }

        if !patterns_dir.is_dir() {
            error!("Путь не является директорией: {:?}", patterns_dir);
            return Err(anyhow!("Путь не является директорией: {:?}", patterns_dir));
        }

        let entries = fs::read_dir(patterns_dir).with_context(|| {
            format!(
                "Не удалось прочитать директорию с паттернами: {:?}",
                patterns_dir
            )
        })?;

        let mut total_patterns = 0;
        let mut total_files = 0;
        let mut invalid_files = 0;

        for entry in entries {
            let entry = entry.with_context(|| {
                format!("Ошибка при чтении записи в директории {:?}", patterns_dir)
            })?;
            let path = entry.path();

            // Пропускаем не-YAML файлы
            if path.extension().and_then(|s| s.to_str()) != Some("yml")
                && path.extension().and_then(|s| s.to_str()) != Some("yaml")
            {
                debug!("Пропущен не-YAML файл: {:?}", path);
                continue;
            }

            total_files += 1;
            debug!("Обработка файла паттернов: {:?}", path);

            // Загружаем и парсим YAML файл
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Не удалось прочитать файл паттернов: {:?}", path))?;

            let pattern_file: PatternFile = serde_yaml::from_str(&content).with_context(|| {
                format!("Не удалось разобрать YAML в файле паттернов: {:?}", path)
            })?;

            // Валидация паттернов
            if !Self::validate_pattern_file(&pattern_file) {
                error!("Файл паттернов содержит недопустимые данные: {:?}", path);
                invalid_files += 1;
                continue;
            }

            // Добавляем паттерны в базу
            let category = pattern_file.category.clone();
            let apps = pattern_file.apps;

            if apps.is_empty() {
                warn!("Файл паттернов не содержит приложений: {:?}", path);
            }

            for app in apps.clone() {
                if !Self::validate_app_pattern(&app) {
                    error!(
                        "Некорректный паттерн приложения в файле {:?}: {:?}",
                        path, app.name
                    );
                    continue;
                }
                all_patterns.push((category.clone(), app));
                total_patterns += 1;
            }

            patterns_by_category.insert(category, apps);
        }

        info!(
            "Загрузка паттернов завершена: {} файлов, {} паттернов, {} недопустимых файлов",
            total_files, total_patterns, invalid_files
        );

        if total_patterns == 0 {
            warn!(
                "Не найдено ни одного допустимого паттерна в директории {:?}",
                patterns_dir
            );
        }

        Ok(Self {
            patterns_by_category,
            all_patterns,
            match_cache: LruCache::new(NonZeroUsize::new(512).unwrap()),
        })
    }

    /// Возвращает все паттерны для указанной категории.
    pub fn patterns_for_category(&self, category: &PatternCategory) -> &[AppPattern] {
        self.patterns_by_category
            .get(category)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Возвращает все паттерны из базы.
    pub fn all_patterns(&self) -> &[(PatternCategory, AppPattern)] {
        &self.all_patterns
    }

    /// Перезагружает паттерны из директории.
    ///
    /// Эта функция позволяет обновить паттерны без создания нового экземпляра PatternDatabase.
    /// Полезно для автообновления паттернов во время работы демона.
    ///
    /// # Аргументы
    ///
    /// * `patterns_dir` - путь к директории с YAML файлами паттернов
    ///
    /// # Возвращает
    ///
    /// `Result<PatternUpdateResult>` - результат обновления с информацией об изменениях
    ///
    /// # Ошибки
    ///
    /// Функция возвращает ошибку, если:
    /// - директория не существует или недоступна для чтения
    /// - YAML файл имеет неверный формат
    /// - структура паттерна не соответствует ожидаемой
    pub fn reload(&mut self, patterns_dir: impl AsRef<Path>) -> Result<crate::classify::pattern_watcher::PatternUpdateResult> {
        let patterns_dir = patterns_dir.as_ref();
        let mut new_patterns_by_category = HashMap::new();
        let mut new_all_patterns = Vec::new();

        info!("Перезагрузка паттернов из директории: {:?}", patterns_dir);

        // Проверяем, существует ли директория
        if !patterns_dir.exists() {
            error!("Директория с паттернами не существует: {:?}", patterns_dir);
            return Err(anyhow!(
                "Директория с паттернами не существует: {:?}",
                patterns_dir
            ));
        }

        if !patterns_dir.is_dir() {
            error!("Путь не является директорией: {:?}", patterns_dir);
            return Err(anyhow!("Путь не является директорией: {:?}", patterns_dir));
        }

        let entries = fs::read_dir(patterns_dir).with_context(|| {
            format!(
                "Не удалось прочитать директорию с паттернами: {:?}",
                patterns_dir
            )
        })?;

        let mut total_patterns = 0;
        let mut total_files = 0;
        let mut invalid_files = 0;
        let mut changed_files = 0;

        // Собираем текущие имена файлов для сравнения
        let current_files: HashSet<_> = self
            .all_patterns
            .iter()
            .map(|(_, pattern)| pattern.name.clone())
            .collect();

        let mut new_file_names = HashSet::new();

        for entry in entries {
            let entry = entry.with_context(|| {
                format!("Ошибка при чтении записи в директории {:?}", patterns_dir)
            })?;
            let path = entry.path();

            // Пропускаем не-YAML файлы
            if path.extension().and_then(|s| s.to_str()) != Some("yml")
                && path.extension().and_then(|s| s.to_str()) != Some("yaml")
            {
                debug!("Пропущен не-YAML файл: {:?}", path);
                continue;
            }

            total_files += 1;
            debug!("Обработка файла паттернов: {:?}", path);

            // Загружаем и парсим YAML файл
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Не удалось прочитать файл паттернов: {:?}", path))?;

            let pattern_file: PatternFile = serde_yaml::from_str(&content).with_context(|| {
                format!("Не удалось разобрать YAML в файле паттернов: {:?}", path)
            })?;

            // Валидация паттернов
            if !Self::validate_pattern_file(&pattern_file) {
                error!("Файл паттернов содержит недопустимые данные: {:?}", path);
                invalid_files += 1;
                continue;
            }

            // Добавляем паттерны в новую базу
            let category = pattern_file.category.clone();
            let apps = pattern_file.apps;

            if apps.is_empty() {
                warn!("Файл паттернов не содержит приложений: {:?}", path);
            }

            for app in apps.clone() {
                if !Self::validate_app_pattern(&app) {
                    error!(
                        "Некорректный паттерн приложения в файле {:?}: {:?}",
                        path, app.name
                    );
                    continue;
                }

                // Отслеживаем новые имена паттернов
                new_file_names.insert(app.name.clone());

                // Проверяем, существует ли паттерн уже в базе
                let existing_pattern = self
                    .all_patterns
                    .iter()
                    .find(|(_, existing_pattern)| existing_pattern.name == app.name);

                match existing_pattern {
                    Some((_, existing_app)) => {
                        // Паттерн существует, проверяем изменился ли он
                        if existing_app != &app {
                            changed_files += 1;
                            debug!("Паттерн {} был изменён", app.name);
                        }
                    }
                    None => {
                        // Это совершенно новый паттерн
                        debug!("Обнаружен новый паттерн: {}", app.name);
                    }
                }

                new_all_patterns.push((category.clone(), app));
                total_patterns += 1;
            }

            new_patterns_by_category.insert(category, apps);
        }

        // Проверяем удалённые паттерны
        let removed_patterns: Vec<_> = current_files.difference(&new_file_names).collect();
        let removed_count = removed_patterns.len();

        if !removed_patterns.is_empty() {
            debug!(
                "Обнаружено {} удалённых паттернов: {:?}",
                removed_count, removed_patterns
            );
        }

        // Проверяем новые паттерны
        let added_patterns: Vec<_> = new_file_names.difference(&current_files).collect();
        let new_files = added_patterns.len();

        if new_files > 0 {
            debug!(
                "Обнаружено {} новых паттернов: {:?}",
                new_files, added_patterns
            );
        }

        info!("Перезагрузка паттернов завершена: {} файлов, {} паттернов, {} недопустимых файлов, {} изменений, {} новых паттернов, {} удалённых паттернов",
              total_files, total_patterns, invalid_files, changed_files, new_files, removed_count);

        // Обновляем внутреннее состояние
        let old_count = self.all_patterns.len();
        self.patterns_by_category = new_patterns_by_category;
        self.all_patterns = new_all_patterns;

        Ok(crate::classify::pattern_watcher::PatternUpdateResult {
            total_files,
            total_patterns,
            invalid_files,
            changed_files,
            new_files,
            removed_files: removed_count,
            patterns_before: old_count,
            patterns_after: total_patterns,
        })
    }

    /// Проверяет, изменились ли паттерны в директории.
    ///
    /// Эта функция выполняет быструю проверку без полной перезагрузки паттернов.
    /// Полезно для оптимизации автообновления.
    ///
    /// # Аргументы
    ///
    /// * `patterns_dir` - путь к директории с YAML файлами паттернов
    ///
    /// # Возвращает
    ///
    /// `Result<bool>` - `true`, если обнаружены изменения, `false` в противном случае
    ///
    /// # Ошибки
    ///
    /// Функция возвращает ошибку, если директория не существует или недоступна для чтения
    pub fn has_changes(&self, patterns_dir: impl AsRef<Path>) -> Result<bool> {
        let patterns_dir = patterns_dir.as_ref();

        // Проверяем, существует ли директория
        if !patterns_dir.exists() {
            return Err(anyhow!(
                "Директория с паттернами не существует: {:?}",
                patterns_dir
            ));
        }

        if !patterns_dir.is_dir() {
            return Err(anyhow!("Путь не является директорией: {:?}", patterns_dir));
        }

        let entries = fs::read_dir(patterns_dir).with_context(|| {
            format!(
                "Не удалось прочитать директорию с паттернами: {:?}",
                patterns_dir
            )
        })?;

        let mut file_count = 0;
        let mut total_patterns = 0;
        let mut current_patterns: HashSet<String> = HashSet::new();

        for entry in entries {
            let entry = entry.with_context(|| {
                format!("Ошибка при чтении записи в директории {:?}", patterns_dir)
            })?;
            let path = entry.path();

            // Пропускаем не-YAML файлы
            if path.extension().and_then(|s| s.to_str()) != Some("yml")
                && path.extension().and_then(|s| s.to_str()) != Some("yaml")
            {
                continue;
            }

            file_count += 1;

            // Загружаем и парсим YAML файл
            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(e) => {
                    warn!("Не удалось прочитать файл паттернов {:?}: {}", path, e);
                    continue;
                }
            };

            let pattern_file: PatternFile = match serde_yaml::from_str(&content) {
                Ok(pattern_file) => pattern_file,
                Err(e) => {
                    warn!(
                        "Не удалось разобрать YAML в файле паттернов {:?}: {}",
                        path, e
                    );
                    continue;
                }
            };

            // Собираем имена паттернов из файла
            for app in pattern_file.apps {
                if Self::validate_app_pattern(&app) {
                    current_patterns.insert(app.name);
                    total_patterns += 1;
                }
            }
        }

        // Сравниваем с текущими паттернами
        let current_names: HashSet<_> = self
            .all_patterns
            .iter()
            .map(|(_, pattern)| pattern.name.clone())
            .collect();

        // Есть изменения, если:
        // 1. Количество файлов изменилось
        // 2. Количество паттернов изменилось
        // 3. Имена паттернов изменились
        let files_changed = file_count != self.patterns_by_category.len();
        let patterns_changed = total_patterns != self.all_patterns.len();
        let names_changed = current_patterns != current_names;

        let has_changes = files_changed || patterns_changed || names_changed;

        debug!("Проверка изменений паттернов: файлов={} (было={}), паттернов={} (было={}), изменений={}",
               file_count, self.patterns_by_category.len(), total_patterns, self.all_patterns.len(), has_changes);

        Ok(has_changes)
    }

    /// Валидирует файл паттернов.
    fn validate_pattern_file(pattern_file: &PatternFile) -> bool {
        // Проверяем, что категория не пустая
        if pattern_file.category.0.trim().is_empty() {
            error!("Категория паттерна не может быть пустой");
            return false;
        }

        // Проверяем, что категория содержит только допустимые символы
        if !pattern_file
            .category
            .0
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            error!(
                "Категория паттерна содержит недопустимые символы: {}",
                pattern_file.category.0
            );
            return false;
        }

        true
    }

    /// Валидирует паттерн приложения.
    fn validate_app_pattern(app_pattern: &AppPattern) -> bool {
        // Проверяем, что имя не пустое
        if app_pattern.name.trim().is_empty() {
            error!("Имя паттерна приложения не может быть пустым");
            return false;
        }

        // Проверяем, что имя содержит только допустимые символы
        if !app_pattern
            .name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        {
            error!(
                "Имя паттерна приложения содержит недопустимые символы: {}",
                app_pattern.name
            );
            return false;
        }

        // Проверяем, что хотя бы один паттерн задан
        if app_pattern.exe_patterns.is_empty()
            && app_pattern.desktop_patterns.is_empty()
            && app_pattern.cgroup_patterns.is_empty()
        {
            error!(
                "Паттерн приложения {} не содержит ни одного правила сопоставления",
                app_pattern.name
            );
            return false;
        }

        // Проверяем, что паттерны не пустые
        for pattern in &app_pattern.exe_patterns {
            if pattern.trim().is_empty() {
                error!(
                    "Паттерн exe не может быть пустым в приложении {}",
                    app_pattern.name
                );
                return false;
            }
        }

        for pattern in &app_pattern.desktop_patterns {
            if pattern.trim().is_empty() {
                error!(
                    "Паттерн desktop не может быть пустым в приложении {}",
                    app_pattern.name
                );
                return false;
            }
        }

        for pattern in &app_pattern.cgroup_patterns {
            if pattern.trim().is_empty() {
                error!(
                    "Паттерн cgroup не может быть пустым в приложении {}",
                    app_pattern.name
                );
                return false;
            }
        }

        true
    }

    /// Находит паттерны, которые соответствуют процессу.
    ///
    /// # Аргументы
    ///
    /// * `exe` - исполняемый файл процесса (например, "firefox")
    /// * `desktop_id` - desktop ID процесса (например, "firefox.desktop")
    /// * `cgroup_path` - путь cgroup процесса (например, "/user.slice/user-1000.slice/session-2.scope")
    ///
    /// # Возвращает
    ///
    /// Список совпадающих паттернов с их категориями.
    pub fn match_process(
        &mut self,
        exe: Option<&str>,
        desktop_id: Option<&str>,
        cgroup_path: Option<&str>,
    ) -> Vec<(&PatternCategory, &AppPattern)> {
        // Создаем кэш-ключ
        let cache_key = (
            exe.map(|s| s.to_string()),
            desktop_id.map(|s| s.to_string()),
            cgroup_path.map(|s| s.to_string()),
        );
        
        // Пробуем получить результат из кэша
        if let Some(cached_matches) = self.match_cache.get(&cache_key) {
            // Преобразуем кэшированные результаты в нужный формат
            let mut matches = Vec::new();
            for (category, pattern) in cached_matches {
                // Находим соответствующие паттерны в all_patterns
                if let Some((found_category, found_pattern)) = self.all_patterns.iter()
                    .find(|(cat, pat)| cat.0 == category.0 && pat.name == pattern.name) {
                    matches.push((found_category, found_pattern));
                }
            }
            return matches;
        }

        // Кэш промах - выполняем полное сопоставление
        let mut matches = Vec::new();
        let mut cacheable_matches = Vec::new();

        for (category, pattern) in &self.all_patterns {
            if Self::pattern_matches(pattern, exe, desktop_id, cgroup_path) {
                matches.push((category, pattern));
                cacheable_matches.push((category.clone(), pattern.clone()));
            }
        }

        // Сохраняем результат в кэш
        self.match_cache.put(cache_key, cacheable_matches);

        matches
    }

    /// Проверяет, соответствует ли паттерн процессу.
    fn pattern_matches(
        pattern: &AppPattern,
        exe: Option<&str>,
        desktop_id: Option<&str>,
        cgroup_path: Option<&str>,
    ) -> bool {
        // Проверяем exe_patterns
        if let Some(exe_str) = exe {
            for exe_pattern in &pattern.exe_patterns {
                if Self::matches_pattern(exe_str, exe_pattern) {
                    return true;
                }
            }
        }

        // Проверяем desktop_patterns
        if let Some(desktop_str) = desktop_id {
            for desktop_pattern in &pattern.desktop_patterns {
                if Self::matches_pattern(desktop_str, desktop_pattern) {
                    return true;
                }
            }
        }

        // Проверяем cgroup_patterns
        if let Some(cgroup_str) = cgroup_path {
            for cgroup_pattern in &pattern.cgroup_patterns {
                if Self::matches_pattern(cgroup_str, cgroup_pattern) {
                    return true;
                }
            }
        }

        false
    }

    /// Улучшенный алгоритм обнаружения приложений с расширенными эвристиками.
    /// 
    /// Этот метод использует дополнительные эвристики для обнаружения приложений:
    /// 1. Анализ аргументов командной строки
    /// 2. Обнаружение контейнеров и sandbox
    /// 3. Анализ пути исполняемого файла
    /// 4. Обнаружение по системным сервисам
    ///
    /// # Аргументы
    ///
    /// * `process` - процесс для анализа
    /// * `pattern_db` - база данных паттернов
    /// * `desktop_id` - desktop ID процесса (опционально)
    ///
    /// # Возвращает
    ///
    /// Список совпадающих паттернов с их категориями.
    pub fn detect_application_enhanced(
        &mut self,
        process: &ProcessRecord,
        desktop_id: Option<&str>,
    ) -> Vec<(PatternCategory, AppPattern)> {
        // Сначала пытаемся базовое сопоставление
        let basic_matches = self.match_process(
            process.exe.as_deref(),
            desktop_id,
            process.cgroup_path.as_deref(),
        );

        if !basic_matches.is_empty() {
            // Конвертируем ссылки в owned данные
            return basic_matches.into_iter().map(|(cat, pat)| (cat.clone(), pat.clone())).collect();
        }

        // Если базовое сопоставление не сработало, применяем улучшенные эвристики
        // Используем статические методы, которые не требуют mutable self
        // Используем drop чтобы освободить mutable borrow перед вызовом статических методов
        drop(basic_matches);
        
        // Получаем все паттерны из текущей базы
        let all_patterns = self.all_patterns();
        
        // Используем статический метод, который не требует доступа к self
        let enhanced_matches = Self::apply_enhanced_detection_no_self(
            process,
            desktop_id,
            all_patterns,
        );

        enhanced_matches
    }

    /// Применяет улучшенные эвристики для обнаружения приложений без доступа к self.
    ///
    /// Этот метод использует загруженные паттерны напрямую.
    ///
    /// # Аргументы
    ///
    /// * `process` - процесс для анализа
    /// * `desktop_id` - desktop ID процесса (опционально)
    /// * `all_patterns` - список всех паттернов из текущей базы
    ///
    /// # Возвращает
    ///
    /// Список совпадающих паттернов с их категориями.
    fn apply_enhanced_detection_no_self(
        process: &ProcessRecord,
        _desktop_id: Option<&str>,
        all_patterns: &[(PatternCategory, AppPattern)],
    ) -> Vec<(PatternCategory, AppPattern)> {
        // 1. Обнаружение контейнеров и sandbox
        if let Some(exe) = &process.exe {
            if Self::is_container_or_sandbox_process(exe) {
                // Пытаемся обнаружить реальное приложение внутри контейнера
                if let Some(cmdline) = &process.cmdline {
                    let container_matches = Self::detect_container_application_static(cmdline, all_patterns);
                    if !container_matches.is_empty() {
                        // Конвертируем ссылки в owned данные
                        return container_matches.into_iter().map(|(cat, pat)| (cat.clone(), pat.clone())).collect();
                    }
                }
            }
        }

        // 2. Анализ пути исполняемого файла
        if let Some(exe) = &process.exe {
            let path_matches = Self::detect_by_executable_path_static(exe, all_patterns);
            if !path_matches.is_empty() {
                // Конвертируем ссылки в owned данные
                return path_matches.into_iter().map(|(cat, pat)| (cat.clone(), pat.clone())).collect();
            }
        }

        // 3. Обнаружение по системным сервисам
        if let Some(systemd_unit) = &process.systemd_unit {
            let service_matches = Self::detect_by_systemd_service_static(systemd_unit, all_patterns);
            if !service_matches.is_empty() {
                // Конвертируем ссылки в owned данные
                return service_matches.into_iter().map(|(cat, pat)| (cat.clone(), pat.clone())).collect();
            }
        }

        // 4. Обнаружение по аргументам командной строки
        if let Some(cmdline) = &process.cmdline {
            let cmdline_matches = Self::detect_by_command_line_arguments_static(cmdline, all_patterns);
            if !cmdline_matches.is_empty() {
                // Конвертируем ссылки в owned данные
                return cmdline_matches.into_iter().map(|(cat, pat)| (cat.clone(), pat.clone())).collect();
            }
        }

        Vec::new()
    }

    /// Применяет улучшенные эвристики для обнаружения приложений (статическая версия).
    ///
    /// Этот метод не требует mutable self и работает напрямую с паттернами.
    ///
    /// # Аргументы
    ///
    /// * `process` - процесс для анализа
    /// * `all_patterns` - список всех паттернов
    ///
    /// # Возвращает
    ///
    /// Список совпадающих паттернов с их категориями.
    fn apply_enhanced_detection_static<'a>(
        process: &ProcessRecord,
        all_patterns: &'a [(PatternCategory, AppPattern)],
    ) -> Vec<(&'a PatternCategory, &'a AppPattern)> {
        // 1. Обнаружение контейнеров и sandbox
        if let Some(exe) = &process.exe {
            if Self::is_container_or_sandbox_process(exe) {
                // Пытаемся обнаружить реальное приложение внутри контейнера
                if let Some(cmdline) = &process.cmdline {
                    let container_matches = Self::detect_container_application_static(cmdline, all_patterns);
                    if !container_matches.is_empty() {
                        return container_matches;
                    }
                }
            }
        }

        // 2. Анализ пути исполняемого файла
        if let Some(exe) = &process.exe {
            let path_matches = Self::detect_by_executable_path_static(exe, all_patterns);
            if !path_matches.is_empty() {
                return path_matches;
            }
        }

        // 3. Обнаружение по системным сервисам
        if let Some(systemd_unit) = &process.systemd_unit {
            let service_matches = Self::detect_by_systemd_service_static(systemd_unit, all_patterns);
            if !service_matches.is_empty() {
                return service_matches;
            }
        }

        // 4. Обнаружение по аргументам командной строки
        if let Some(cmdline) = &process.cmdline {
            let cmdline_matches = Self::detect_by_command_line_arguments_static(cmdline, all_patterns);
            if !cmdline_matches.is_empty() {
                return cmdline_matches;
            }
        }

        Vec::new()
    }

    /// Обнаружение по аргументам командной строки (статическая версия).
    ///
    /// # Аргументы
    ///
    /// * `cmdline` - командная строка процесса
    /// * `all_patterns` - список всех паттернов
    ///
    /// # Возвращает
    ///
    /// Список совпадающих паттернов с их категориями.
    fn detect_by_command_line_arguments_static<'a>(
        cmdline: &str,
        all_patterns: &'a [(PatternCategory, AppPattern)],
    ) -> Vec<(&'a PatternCategory, &'a AppPattern)> {
        let mut matches = Vec::new();

        // Извлекаем аргументы из командной строки
        let args: Vec<&str> = cmdline.split_whitespace().collect();

        // Пытаемся найти совпадения в аргументах
        for arg in args {
            // Пропускаем аргументы, которые являются флагами или путями
            if arg.starts_with('-') || arg.starts_with('/') {
                continue;
            }

            // Пытаемся сопоставить аргумент с паттернами
            for (category, pattern) in all_patterns {
                for exe_pattern in &pattern.exe_patterns {
                    if Self::matches_pattern(arg, exe_pattern) {
                        matches.push((category, pattern));
                        break;
                    }
                }
            }

            if !matches.is_empty() {
                break;
            }
        }

        matches
    }

    /// Проверяет, является ли процесс контейнером или sandbox.
    ///
    /// # Аргументы
    ///
    /// * `exe` - имя исполняемого файла
    ///
    /// # Возвращает
    ///
    /// `true`, если процесс является контейнером или sandbox.
    fn is_container_or_sandbox_process(exe: &str) -> bool {
        let container_exes = [
            "docker", "podman", "lxc", "lxd", "containerd", "runc",
            "flatpak", "snap", "firejail", "bubblewrap", "nsjail",
        ];

        container_exes.contains(&exe)
    }

    /// Обнаружение приложения внутри контейнера (статическая версия).
    ///
    /// # Аргументы
    ///
    /// * `cmdline` - командная строка процесса
    /// * `all_patterns` - список всех паттернов
    ///
    /// # Возвращает
    ///
    /// Список совпадающих паттернов с их категориями.
    fn detect_container_application_static<'a>(
        cmdline: &str,
        all_patterns: &'a [(PatternCategory, AppPattern)],
    ) -> Vec<(&'a PatternCategory, &'a AppPattern)> {
        let mut matches = Vec::new();

        // Извлекаем аргументы из командной строки
        let args: Vec<&str> = cmdline.split_whitespace().collect();

        // Пытаемся найти реальное приложение в аргументах
        for arg in args {
            // Пропускаем аргументы, которые являются флагами или путями
            if arg.starts_with('-') || arg.starts_with('/') {
                continue;
            }

            // Пытаемся сопоставить аргумент с паттернами
            for (category, pattern) in all_patterns {
                for exe_pattern in &pattern.exe_patterns {
                    if Self::matches_pattern(arg, exe_pattern) {
                        matches.push((category, pattern));
                        break;
                    }
                }
            }

            if !matches.is_empty() {
                break;
            }
        }

        matches
    }

    /// Обнаружение по пути исполняемого файла (статическая версия).
    ///
    /// # Аргументы
    ///
    /// * `exe` - полный путь к исполняемому файлу
    /// * `all_patterns` - список всех паттернов
    ///
    /// # Возвращает
    ///
    /// Список совпадающих паттернов с их категориями.
    fn detect_by_executable_path_static<'a>(
        exe: &str,
        all_patterns: &'a [(PatternCategory, AppPattern)],
    ) -> Vec<(&'a PatternCategory, &'a AppPattern)> {
        let mut matches = Vec::new();

        // Извлекаем имя файла из пути
        if let Some(file_name) = exe.split('/').last() {
            // Пытаемся сопоставить имя файла с паттернами
            for (category, pattern) in all_patterns {
                for exe_pattern in &pattern.exe_patterns {
                    if Self::matches_pattern(file_name, exe_pattern) {
                        matches.push((category, pattern));
                        break;
                    }
                }
            }
        }

        matches
    }

    /// Обнаружение по системному сервису (статическая версия).
    ///
    /// # Аргументы
    ///
    /// * `systemd_unit` - имя системного сервиса
    /// * `all_patterns` - список всех паттернов
    ///
    /// # Возвращает
    ///
    /// Список совпадающих паттернов с их категориями.
    fn detect_by_systemd_service_static<'a>(
        systemd_unit: &str,
        all_patterns: &'a [(PatternCategory, AppPattern)],
    ) -> Vec<(&'a PatternCategory, &'a AppPattern)> {
        let mut matches = Vec::new();

        // Извлекаем базовое имя сервиса (без .service суффикса)
        let service_name = systemd_unit
            .strip_suffix(".service")
            .unwrap_or(systemd_unit);

        // Пытаемся сопоставить имя сервиса с паттернами
        for (category, pattern) in all_patterns {
            for exe_pattern in &pattern.exe_patterns {
                if Self::matches_pattern(service_name, exe_pattern) {
                    matches.push((category, pattern));
                    break;
                }
            }
        }

        matches
    }



    /// Проверяет, соответствует ли строка паттерну.
    ///
    /// Поддерживает glob паттерны:
    /// - `*` - любая последовательность символов (включая пустую)
    /// - `?` - один символ
    /// - точное совпадение, если нет wildcard символов
    ///
    /// Примеры:
    /// - `firefox*` соответствует `firefox`, `firefox-bin`, `firefox-esr`
    /// - `*firefox` соответствует `firefox`, `something-firefox`
    /// - `*firefox*` соответствует `firefox`, `firefox-bin`, `something-firefox-bin`
    /// - `firefox-?-bin` соответствует `firefox-a-bin`, `firefox-1-bin`
    /// - `firefox-*-bin` соответствует `firefox-esr-bin`, `firefox-123-bin`
    pub fn matches_pattern(text: &str, pattern: &str) -> bool {
        // Если паттерн не содержит wildcard символов, используем точное совпадение
        if !pattern.contains('*') && !pattern.contains('?') {
            return text == pattern;
        }

        // Используем оптимизированный алгоритм сопоставления glob паттернов
        Self::glob_match_optimized(text, pattern)
    }

    /// Оптимизированная функция для сопоставления glob паттернов.
    ///
    /// Алгоритм:
    /// - `*` соответствует любой последовательности символов (включая пустую)
    /// - `?` соответствует одному символу
    /// - Обычные символы должны совпадать точно
    ///
    /// Оптимизации:
    /// - Итеративный подход вместо рекурсивного для лучшей производительности
    /// - Раннее завершение при несовпадении
    fn glob_match_optimized(text: &str, pattern: &str) -> bool {
        // Преобразование в байты для быстрого сравнения
        let text_bytes = text.as_bytes();
        let pattern_bytes = pattern.as_bytes();
        
        let mut text_idx = 0;
        let mut pattern_idx = 0;
        let mut star_idx = None;
        let mut match_idx = 0;
        
        while text_idx < text_bytes.len() {
            if pattern_idx < pattern_bytes.len() && 
               (pattern_bytes[pattern_idx] == b'?' || 
                pattern_bytes[pattern_idx] == text_bytes[text_idx]) {
                // Совпадение символа или ?
                text_idx += 1;
                pattern_idx += 1;
            } else if pattern_idx < pattern_bytes.len() && pattern_bytes[pattern_idx] == b'*' {
                // Нашли звездочку - запоминаем позицию
                star_idx = Some(pattern_idx);
                match_idx = text_idx;
                pattern_idx += 1;
            } else if let Some(star_pos) = star_idx {
                // Попробуем продолжить после звездочки
                pattern_idx = star_pos + 1;
                match_idx += 1;
                text_idx = match_idx;
            } else {
                // Нет совпадения
                return false;
            }
        }
        
        // Пропускаем оставшиеся звездочки
        while pattern_idx < pattern_bytes.len() && pattern_bytes[pattern_idx] == b'*' {
            pattern_idx += 1;
        }
        
        pattern_idx == pattern_bytes.len()
    }
}

/// Классифицирует процесс по паттернам и заполняет process_type и tags.
///
/// # Аргументы
///
/// * `process` - процесс для классификации (будет изменён in-place)
/// * `pattern_db` - база данных паттернов
/// * `ml_classifier` - опциональный ML-классификатор для дополнительной классификации
/// * `desktop_id` - desktop ID процесса (опционально, из window_info или systemd_unit)
///
/// # Логирование
///
/// Функция логирует процесс классификации, включая:
/// - Информацию о совпадающих паттернах
/// - Результаты ML-классификации
/// - Окончательный результат классификации
///
/// # Ошибки
///
/// Функция не возвращает ошибок, но логирует проблемы:
/// - Отсутствие совпадающих паттернов
/// - Ошибки ML-классификации
/// - Конфликты классификации
#[cfg(any(feature = "catboost", feature = "onnx"))]
pub fn classify_process(
    process: &mut ProcessRecord,
    pattern_db: &Arc<Mutex<PatternDatabase>>,
    ml_classifier: Option<&mut dyn MLClassifier>,
    desktop_id: Option<&str>,
) {
    debug!(
        "Классификация процесса PID {}: exe={:?}, desktop_id={:?}",
        process.pid, process.exe, desktop_id
    );

    // Извлекаем desktop_id из systemd_unit, если не передан явно
    let desktop_id = desktop_id.or_else(|| {
        process
            .systemd_unit
            .as_ref()
            .and_then(|unit| unit.strip_suffix(".service"))
            .map(|s| s as &str)
    });

    debug!("Используемый desktop_id: {:?}", desktop_id);

    // Ищем совпадающие паттерны с использованием улучшенного алгоритма обнаружения
    let mut pattern_db_lock = pattern_db.lock().unwrap();
    let matches = pattern_db_lock.detect_application_enhanced(
        process,
        desktop_id,
    );

    if matches.is_empty() {
        debug!(
            "Для процесса PID {} не найдено совпадающих паттернов",
            process.pid
        );
    } else {
        debug!(
            "Найдено {} совпадающих паттернов для процесса PID {}",
            matches.len(),
            process.pid
        );
        for (category, pattern) in &matches {
            debug!("  - Категория: {}, Паттерн: {}", category.0, pattern.name);
        }
    }

    // Собираем все теги из совпадающих паттернов
    let mut all_tags = HashSet::new();
    for (_, pattern) in &matches {
        for tag in &pattern.tags {
            all_tags.insert(tag.clone());
        }
    }

    // Выбираем process_type из первой категории (можно улучшить логику выбора)
    let mut process_type = matches.first().map(|(category, _)| category.0.clone());

    // Применяем ML-классификацию, если доступен
    if let Some(classifier) = ml_classifier {
        let ml_result = classifier.classify(process);

        debug!(
            "Результаты ML-классификации для PID {}: type={:?}, confidence={:.2}, tags={:?}",
            process.pid, ml_result.process_type, ml_result.confidence, ml_result.tags
        );

        // Объединяем результаты паттерн-классификации и ML-классификации
        if let Some(ml_type) = ml_result.process_type {
            // Если ML уверен в предсказании, используем его тип
            if ml_result.confidence > 0.7 {
                if let Some(ref pattern_type) = process_type {
                    if pattern_type != &ml_type {
                        info!(
                            "ML-классификатор переопределил тип процесса PID {}: {} -> {}",
                            process.pid, pattern_type, ml_type
                        );
                    }
                }
                process_type = Some(ml_type);
            } else {
                debug!(
                    "ML-классификатор предложил тип {} с низкой уверенностью ({:.2})",
                    ml_type, ml_result.confidence
                );
            }
        }

        // Добавляем теги из ML-классификации
        for tag in ml_result.tags {
            all_tags.insert(tag);
        }
    }

    // Заполняем результаты
    process.process_type = process_type;
    process.tags = all_tags.into_iter().collect();
    process.tags.sort();

    if let Some(ref process_type) = process.process_type {
        info!(
            "Процесс PID {} классифицирован как '{}' с тегами: {:?}",
            process.pid, process_type, process.tags
        );
    } else {
        debug!(
            "Процесс PID {} не классифицирован (теги: {:?})",
            process.pid, process.tags
        );
    }
}

#[cfg(not(any(feature = "catboost", feature = "onnx")))]
pub fn classify_process(
    process: &mut ProcessRecord,
    pattern_db: &Arc<Mutex<PatternDatabase>>,
    _ml_classifier: Option<&dyn std::any::Any>,
    desktop_id: Option<&str>,
) {
    debug!(
        "Классификация процесса PID {}: exe={:?}, desktop_id={:?}",
        process.pid, process.exe, desktop_id
    );

    // Извлекаем desktop_id из systemd_unit, если не передан явно
    let desktop_id = desktop_id.or(process.systemd_unit.as_deref());

    // Паттерн-базированная классификация с улучшенным обнаружением (без ML)
    // Блокируем базу паттернов для чтения
    let mut pattern_db_lock = pattern_db.lock().unwrap();
    let matches = pattern_db_lock.detect_application_enhanced(
        process,
        desktop_id,
    );

    // Собираем все теги из совпадающих паттернов
    let mut all_tags = HashSet::new();
    for (_, pattern) in &matches {
        for tag in &pattern.tags {
            all_tags.insert(tag.clone());
        }
    }

    // Выбираем process_type из первой категории
    if let Some((category, _)) = matches.first() {
        process.process_type = Some(category.0.clone());
        process.tags = all_tags.into_iter().collect();
        debug!(
            "Процесс PID {} классифицирован как {:?} (теги: {:?})",
            process.pid, process.process_type, process.tags
        );
    } else {
        debug!(
            "Процесс PID {} не классифицирован (теги: {:?})",
            process.pid, process.tags
        );
    }
}

/// Классифицирует AppGroup, агрегируя теги и типы из процессов группы.
///
/// # Аргументы
///
/// * `app_group` - группа приложений для классификации (будет изменена in-place)
/// * `processes` - все процессы (для поиска процессов группы)
/// * `_pattern_db` - база данных паттернов (зарезервировано для будущего использования)
///
/// # Логирование
///
/// Функция логирует процесс классификации группы, включая:
/// - Информацию о найденных процессах группы
/// - Агрегированные теги и типы
/// - Конфликты типов процессов
pub fn classify_app_group(
    app_group: &mut AppGroupRecord,
    processes: &[ProcessRecord],
    _pattern_db: &Arc<Mutex<PatternDatabase>>,
) {
    debug!(
        "Классификация AppGroup {} (root_pid={})",
        app_group.app_group_id, app_group.root_pid
    );

    // Находим процессы этой группы
    let group_processes: Vec<&ProcessRecord> = processes
        .iter()
        .filter(|p| p.app_group_id.as_deref() == Some(app_group.app_group_id.as_str()))
        .collect();

    if group_processes.is_empty() {
        debug!("AppGroup {} не содержит процессов", app_group.app_group_id);
        return;
    }

    debug!(
        "Найдено {} процессов в AppGroup {}",
        group_processes.len(),
        app_group.app_group_id
    );

    // Собираем все теги и типы из процессов группы
    let mut all_tags = HashSet::new();
    let mut process_types = HashSet::new();

    for process in group_processes {
        debug!(
            "  - Процесс PID {}: type={:?}, tags={:?}",
            process.pid, process.process_type, process.tags
        );

        // Добавляем теги процесса
        for tag in &process.tags {
            all_tags.insert(tag.clone());
        }

        // Добавляем тип процесса
        if let Some(ref process_type) = process.process_type {
            process_types.insert(process_type.clone());
        }
    }

    // Заполняем теги группы (уникальные, отсортированные)
    app_group.tags = all_tags.into_iter().collect();
    app_group.tags.sort();

    debug!(
        "Агрегированные теги для AppGroup {}: {:?}",
        app_group.app_group_id, app_group.tags
    );

    // Если все процессы имеют один тип, можно установить app_name
    // (это можно улучшить позже)
    if process_types.len() == 1 {
        // Можно использовать тип как app_name, если он не установлен
        // Но лучше оставить app_name как есть, так как он может быть более специфичным
        let _ = process_types.iter().next();
    } else if process_types.len() > 1 {
        debug!(
            "AppGroup {} содержит процессы с разными типами: {:?}",
            app_group.app_group_id, process_types
        );
    }

    info!(
        "AppGroup {} классифицирована с тегами: {:?}",
        app_group.app_group_id, app_group.tags
    );
}

/// Классифицирует все процессы и группы в снапшоте.
///
/// Это удобная функция-обёртка, которая классифицирует все процессы,
/// а затем агрегирует теги для групп.
#[cfg(any(feature = "catboost", feature = "onnx"))]
pub fn classify_all(
    processes: &mut [ProcessRecord],
    app_groups: &mut [AppGroupRecord],
    pattern_db: &Arc<Mutex<PatternDatabase>>,
    ml_classifier: Option<&dyn MLClassifier>,
) {
    // Классифицируем все процессы
    for process in processes.iter_mut() {
        classify_process(process, pattern_db, ml_classifier, None);
    }

    // Классифицируем все группы (агрегируем теги из процессов)
    for app_group in app_groups.iter_mut() {
        classify_app_group(app_group, processes, pattern_db);
    }
}

#[cfg(not(any(feature = "catboost", feature = "onnx")))]
pub fn classify_all(
    processes: &mut [ProcessRecord],
    app_groups: &mut [AppGroupRecord],
    pattern_db: &Arc<Mutex<PatternDatabase>>,
    _ml_classifier: Option<&dyn std::any::Any>,
) {
    // Классифицируем все процессы (без ML)
    for process in processes.iter_mut() {
        classify_process(process, pattern_db, None, None);
    }

    // Классифицируем все группы (агрегируем теги из процессов)
    for app_group in app_groups.iter_mut() {
        classify_app_group(app_group, processes, pattern_db);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[cfg(any(feature = "catboost", feature = "onnx"))]
    use crate::classify::ml_classifier::StubMLClassifier;
    use crate::classify::pattern_watcher::PatternUpdateResult;

    fn create_test_pattern_file(dir: &Path, filename: &str, content: &str) -> PathBuf {
        let file_path = dir.join(filename);
        fs::write(&file_path, content).expect("write test pattern file");
        file_path
    }

    #[test]
    fn loads_patterns_from_directory() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        create_test_pattern_file(
            patterns_dir,
            "browsers.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox", "firefox-bin"]
    desktop_patterns: ["firefox.desktop"]
    tags: ["browser", "gui_interactive"]
  - name: "chrome"
    label: "Google Chrome"
    exe_patterns: ["google-chrome", "chrome"]
    tags: ["browser", "chromium-family"]
"#,
        );

        create_test_pattern_file(
            patterns_dir,
            "ide.yml",
            r#"
category: ide
apps:
  - name: "vscode"
    label: "Visual Studio Code"
    exe_patterns: ["code"]
    desktop_patterns: ["code.desktop"]
    tags: ["ide", "gui_interactive"]
"#,
        );

        let db = PatternDatabase::load(patterns_dir).expect("load patterns");

        // Проверяем, что паттерны загружены
        let browser_category = PatternCategory("browser".to_string());
        let browser_patterns = db.patterns_for_category(&browser_category);
        assert_eq!(browser_patterns.len(), 2);
        assert_eq!(browser_patterns[0].name, "firefox");
        assert_eq!(browser_patterns[1].name, "chrome");

        let ide_category = PatternCategory("ide".to_string());
        let ide_patterns = db.patterns_for_category(&ide_category);
        assert_eq!(ide_patterns.len(), 1);
        assert_eq!(ide_patterns[0].name, "vscode");
    }

    #[test]
    fn matches_process_by_exe() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        create_test_pattern_file(
            patterns_dir,
            "browsers.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox", "firefox-bin"]
    tags: ["browser"]
"#,
        );

        let mut db = PatternDatabase::load(patterns_dir).expect("load patterns");

        // Совпадение по exe
        let matches = db.match_process(Some("firefox"), None, None);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].1.name, "firefox");

        // Нет совпадения
        let matches = db.match_process(Some("chrome"), None, None);
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn matches_process_by_desktop() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        create_test_pattern_file(
            patterns_dir,
            "browsers.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    desktop_patterns: ["firefox.desktop"]
    tags: ["browser"]
"#,
        );

        let mut db = PatternDatabase::load(patterns_dir).expect("load patterns");

        let matches = db.match_process(None, Some("firefox.desktop"), None);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].1.name, "firefox");
    }

    #[test]
    fn matches_process_by_cgroup() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        create_test_pattern_file(
            patterns_dir,
            "batch.yml",
            r#"
category: batch
apps:
  - name: "systemd-service"
    label: "Systemd Service"
    cgroup_patterns: ["/system.slice/my-service.service"]
    tags: ["batch"]
"#,
        );

        let mut db = PatternDatabase::load(patterns_dir).expect("load patterns");

        let matches = db.match_process(None, None, Some("/system.slice/my-service.service"));
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].1.name, "systemd-service");
    }

    #[test]
    fn matches_pattern_with_wildcards() {
        // Тестируем функцию matches_pattern напрямую
        // Базовые случаи с *
        assert!(PatternDatabase::matches_pattern("firefox-bin", "firefox*"));
        assert!(PatternDatabase::matches_pattern("firefox", "firefox*"));
        assert!(PatternDatabase::matches_pattern(
            "something-firefox",
            "*firefox"
        ));
        assert!(PatternDatabase::matches_pattern(
            "something-firefox-bin",
            "*firefox*"
        ));
        assert!(PatternDatabase::matches_pattern("firefox", "firefox"));
        assert!(!PatternDatabase::matches_pattern("chrome", "firefox"));

        // Тесты с ? (один символ)
        assert!(PatternDatabase::matches_pattern(
            "firefox-a-bin",
            "firefox-?-bin"
        ));
        assert!(PatternDatabase::matches_pattern(
            "firefox-1-bin",
            "firefox-?-bin"
        ));
        assert!(!PatternDatabase::matches_pattern(
            "firefox-ab-bin",
            "firefox-?-bin"
        ));
        assert!(!PatternDatabase::matches_pattern(
            "firefox--bin",
            "firefox-?-bin"
        ));

        // Тесты с множественными *
        assert!(PatternDatabase::matches_pattern(
            "firefox-esr-bin",
            "firefox-*-bin"
        ));
        assert!(PatternDatabase::matches_pattern(
            "firefox-123-bin",
            "firefox-*-bin"
        ));
        assert!(PatternDatabase::matches_pattern(
            "firefox--bin",
            "firefox-*-bin"
        ));
        assert!(!PatternDatabase::matches_pattern(
            "firefox-esr",
            "firefox-*-bin"
        ));
        assert!(!PatternDatabase::matches_pattern(
            "chrome-esr-bin",
            "firefox-*-bin"
        ));

        // Тесты с комбинациями * и ?
        assert!(PatternDatabase::matches_pattern(
            "firefox-a-esr-bin",
            "firefox-?-*-bin"
        ));
        assert!(PatternDatabase::matches_pattern(
            "firefox-1-123-bin",
            "firefox-?-*-bin"
        ));
        assert!(!PatternDatabase::matches_pattern(
            "firefox-esr-bin",
            "firefox-?-*-bin"
        ));

        // Тесты с несколькими ?
        assert!(PatternDatabase::matches_pattern(
            "firefox-12-bin",
            "firefox-??-bin"
        ));
        assert!(!PatternDatabase::matches_pattern(
            "firefox-1-bin",
            "firefox-??-bin"
        ));
        assert!(!PatternDatabase::matches_pattern(
            "firefox-123-bin",
            "firefox-??-bin"
        ));

        // Тесты с * в середине
        assert!(PatternDatabase::matches_pattern(
            "firefox-esr-bin",
            "fire*bin"
        ));
        assert!(PatternDatabase::matches_pattern("firefox-bin", "fire*bin"));
        assert!(!PatternDatabase::matches_pattern("firefox", "fire*bin"));

        // Тесты с пустыми строками
        assert!(PatternDatabase::matches_pattern("", "*"));
        assert!(PatternDatabase::matches_pattern("", "**"));
        assert!(!PatternDatabase::matches_pattern("", "?"));
        assert!(!PatternDatabase::matches_pattern("a", ""));
        assert!(PatternDatabase::matches_pattern("", ""));

        // Тесты с только *
        assert!(PatternDatabase::matches_pattern("anything", "*"));
        assert!(PatternDatabase::matches_pattern("", "*"));
        assert!(PatternDatabase::matches_pattern("a", "*"));
    }

    #[test]
    fn handles_empty_patterns() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        create_test_pattern_file(
            patterns_dir,
            "empty.yml",
            r#"
category: test
apps: []
"#,
        );

        let db = PatternDatabase::load(patterns_dir).expect("load patterns");
        let test_category = PatternCategory("test".to_string());
        let patterns = db.patterns_for_category(&test_category);
        assert_eq!(patterns.len(), 0);
    }

    #[test]
    fn handles_missing_optional_fields() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        create_test_pattern_file(
            patterns_dir,
            "minimal.yml",
            r#"
category: test
apps:
  - name: "minimal"
    label: "Minimal App"
"#,
        );

        let db = PatternDatabase::load(patterns_dir).expect("load patterns");
        let test_category = PatternCategory("test".to_string());
        let patterns = db.patterns_for_category(&test_category);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].name, "minimal");
        assert!(patterns[0].exe_patterns.is_empty());
        assert!(patterns[0].tags.is_empty());
    }

    #[test]
    fn classify_process_sets_type_and_tags() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        create_test_pattern_file(
            patterns_dir,
            "browsers.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser", "gui_interactive"]
"#,
        );

        let db = PatternDatabase::load(patterns_dir).expect("load patterns");

        let mut process = ProcessRecord {
            pid: 1000,
            ppid: 1,
            uid: 1000,
            gid: 1000,
            exe: Some("firefox".to_string()),
            cmdline: None,
            cgroup_path: None,
            systemd_unit: None,
            app_group_id: None,
            state: "R".to_string(),
            start_time: 0,
            uptime_sec: 100,
            tty_nr: 0,
            has_tty: false,
            cpu_share_1s: None,
            cpu_share_10s: None,
            io_read_bytes: None,
            io_write_bytes: None,
            io_read_operations: None,
            io_write_operations: None,
            io_total_operations: None,
            io_last_update_ns: None,
            io_data_source: None,
            rss_mb: None,
            swap_mb: None,
            voluntary_ctx: None,
            involuntary_ctx: None,
            has_gui_window: false,
            is_focused_window: false,
            window_state: None,
            env_has_display: false,
            env_has_wayland: false,
            env_term: None,
            env_ssh: false,
            is_audio_client: false,
            has_active_stream: false,
            process_type: None,
            tags: Vec::new(),
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
            energy_uj: None,
            power_w: None,
            energy_timestamp: None,
            network_rx_bytes: None,
            network_tx_bytes: None,
            network_rx_packets: None,
            network_tx_packets: None,
            network_tcp_connections: None,
            network_udp_connections: None,
            network_last_update_ns: None,
            network_data_source: None,
            gpu_utilization: None,
            gpu_memory_bytes: None,
            gpu_time_us: None,
            gpu_api_calls: None,
            gpu_last_update_ns: None,
            gpu_data_source: None,
        };

        // Тест без ML-классификатора
        let pattern_db = Arc::new(Mutex::new(db));
        classify_process(&mut process, &pattern_db, None, None);

        assert_eq!(process.process_type, Some("browser".to_string()));
        assert!(process.tags.contains(&"browser".to_string()));
        assert!(process.tags.contains(&"gui_interactive".to_string()));
    }

    #[test]
    fn classify_process_without_match_leaves_empty() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        create_test_pattern_file(
            patterns_dir,
            "browsers.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser"]
"#,
        );

        let db = PatternDatabase::load(patterns_dir).expect("load patterns");

        let mut process = ProcessRecord::default();
        process.pid = 1000;
        process.ppid = 1;
        process.uid = 1000;
        process.gid = 1000;
        process.exe = Some("unknown-app".to_string());
        process.state = "R".to_string();
        process.start_time = 0;
        process.uptime_sec = 100;
        process.tty_nr = 0;
        process.has_tty = false;

        let pattern_db = Arc::new(Mutex::new(db));
        classify_process(&mut process, &pattern_db, None, None);

        assert_eq!(process.process_type, None);
        assert!(process.tags.is_empty());
    }

    #[test]
    fn classify_app_group_aggregates_tags() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        create_test_pattern_file(
            patterns_dir,
            "browsers.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox", "firefox-bin"]
    tags: ["browser", "gui_interactive"]
"#,
        );

        let db = PatternDatabase::load(patterns_dir).expect("load patterns");

        let processes = vec![
            {
                let mut record = ProcessRecord::default();
                record.pid = 1000;
                record.ppid = 1;
                record.uid = 1000;
                record.gid = 1000;
                record.exe = Some("firefox".to_string());
                record.app_group_id = Some("group1".to_string());
                record.state = "R".to_string();
                record.start_time = 0;
                record.uptime_sec = 100;
                record.tty_nr = 0;
                record.has_tty = false;
                record.process_type = Some("browser".to_string());
                record.tags = vec!["browser".to_string(), "gui_interactive".to_string()];
                record
            },
            {
                let mut record = ProcessRecord::default();
                record.pid = 1001;
                record.ppid = 1000;
                record.uid = 1000;
                record.gid = 1000;
                record.exe = Some("firefox-bin".to_string());
                record.app_group_id = Some("group1".to_string());
                record.state = "R".to_string();
                record.start_time = 0;
                record.uptime_sec = 100;
                record.tty_nr = 0;
                record.has_tty = false;
                record.process_type = Some("browser".to_string());
                record.tags = vec!["browser".to_string(), "gui_interactive".to_string()];
                record
            },
        ];

        let mut app_group = AppGroupRecord {
            app_group_id: "group1".to_string(),
            root_pid: 1000,
            process_ids: vec![1000, 1001],
            app_name: None,
            total_cpu_share: None,
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: None,
            has_gui_window: false,
            is_focused_group: false,
            tags: Vec::new(),
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        };

        let pattern_db = Arc::new(Mutex::new(db));
        classify_app_group(&mut app_group, &processes, &pattern_db);

        // Теги должны быть агрегированы (уникальные)
        assert_eq!(app_group.tags.len(), 2);
        assert!(app_group.tags.contains(&"browser".to_string()));
        assert!(app_group.tags.contains(&"gui_interactive".to_string()));
    }

    #[test]
    fn classify_all_processes_and_groups() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        create_test_pattern_file(
            patterns_dir,
            "browsers.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser", "gui_interactive"]
"#,
        );

        let db = PatternDatabase::load(patterns_dir).expect("load patterns");

        let mut processes = vec![{
            let mut record = ProcessRecord::default();
            record.pid = 1000;
            record.ppid = 1;
            record.uid = 1000;
            record.gid = 1000;
            record.exe = Some("firefox".to_string());
            record.app_group_id = Some("group1".to_string());
            record.state = "R".to_string();
            record.start_time = 0;
            record.uptime_sec = 100;
            record.tty_nr = 0;
            record.has_tty = false;
            record
        }];

        let mut app_groups = vec![AppGroupRecord {
            app_group_id: "group1".to_string(),
            root_pid: 1000,
            process_ids: vec![1000],
            app_name: None,
            total_cpu_share: None,
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: None,
            has_gui_window: false,
            is_focused_group: false,
            tags: Vec::new(),
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        }];

        // Тест без ML-классификатора
        let pattern_db = Arc::new(Mutex::new(db));
        classify_all(&mut processes, &mut app_groups, &pattern_db, None);

        // Процесс должен быть классифицирован
        assert_eq!(processes[0].process_type, Some("browser".to_string()));
        assert!(processes[0].tags.contains(&"browser".to_string()));

        // Группа должна иметь агрегированные теги
        assert!(!app_groups[0].tags.is_empty());
        assert!(app_groups[0].tags.contains(&"browser".to_string()));
    }

    #[test]
    #[cfg(any(feature = "catboost", feature = "onnx"))]
    fn classify_process_with_ml_classifier() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        create_test_pattern_file(
            patterns_dir,
            "browsers.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser"]
"#,
        );

        let db = PatternDatabase::load(patterns_dir).expect("load patterns");
        let ml_classifier = StubMLClassifier::new();

        let mut process = ProcessRecord::default();
        process.pid = 1000;
        process.ppid = 1;
        process.uid = 1000;
        process.gid = 1000;
        process.exe = Some("firefox".to_string());
        process.state = "R".to_string();
        process.start_time = 0;
        process.uptime_sec = 100;
        process.tty_nr = 0;
        process.has_tty = false;
        process.cpu_share_10s = Some(0.5); // Высокий CPU для ML-классификации

        // Классификация с ML-классификатором
        classify_process(&mut process, &mut db, Some(&ml_classifier), None);

        // Должны быть теги и от паттернов, и от ML
        assert!(process.tags.contains(&"browser".to_string())); // от паттернов
        assert!(process.tags.contains(&"high_cpu".to_string())); // от ML

        // Тип должен быть от паттернов (так как ML уверенность < 0.7 для cpu_intensive)
        assert_eq!(process.process_type, Some("browser".to_string()));
    }

    #[test]
    #[cfg(any(feature = "catboost", feature = "onnx"))]
    fn classify_process_ml_overrides_pattern() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        create_test_pattern_file(
            patterns_dir,
            "browsers.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser"]
"#,
        );

        let db = PatternDatabase::load(patterns_dir).expect("load patterns");
        let ml_classifier = StubMLClassifier::new();

        let mut process = ProcessRecord {
            pid: 1000,
            ppid: 1,
            uid: 1000,
            gid: 1000,
            exe: Some("firefox".to_string()),
            cmdline: None,
            cgroup_path: None,
            systemd_unit: None,
            app_group_id: None,
            state: "R".to_string(),
            start_time: 0,
            uptime_sec: 100,
            tty_nr: 0,
            has_tty: false,
            cpu_share_1s: None,
            cpu_share_10s: None,
            io_read_bytes: None,
            io_write_bytes: None,
            rss_mb: None,
            swap_mb: None,
            voluntary_ctx: None,
            involuntary_ctx: None,
            has_gui_window: false,
            is_focused_window: false,
            window_state: None,
            env_has_display: false,
            env_has_wayland: false,
            env_term: None,
            env_ssh: false,
            is_audio_client: true, // Аудио клиент для ML-классификации с высокой уверенностью
            has_active_stream: false,
            process_type: None,
            tags: Vec::new(),
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
            energy_uj: None,
            power_w: None,
            energy_timestamp: None,
            network_rx_bytes: None,
            network_tx_bytes: None,
            network_rx_packets: None,
            network_tx_packets: None,
            network_tcp_connections: None,
            network_udp_connections: None,
            network_last_update_ns: None,
            network_data_source: None,
        };

        // Классификация с ML-классификатором
        classify_process(&mut process, &mut db, Some(&ml_classifier), None);

        // Должны быть теги и от паттернов, и от ML
        assert!(process.tags.contains(&"browser".to_string())); // от паттернов
        assert!(process.tags.contains(&"audio".to_string())); // от ML
        assert!(process.tags.contains(&"realtime".to_string())); // от ML

        // Тип должен быть от ML (так как уверенность > 0.7 для audio)
        assert_eq!(process.process_type, Some("audio".to_string()));
    }

    #[test]
    fn test_pattern_validation() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Тест с недопустимой категорией (пустая)
        create_test_pattern_file(
            patterns_dir,
            "invalid_category.yml",
            r#"
category: ""
apps:
  - name: "test"
    label: "Test App"
    exe_patterns: ["test"]
    tags: ["test"]
"#,
        );

        // Это должно загрузиться, но с предупреждением в логах
        let db = PatternDatabase::load(patterns_dir);
        // Теперь валидация происходит во время загрузки, поэтому это должно быть OK
        // но с предупреждениями в логах
        assert!(db.is_ok()); // Должно загрузиться, но с предупреждениями

        // Тест с недопустимым именем приложения
        let temp_dir2 = TempDir::new().expect("temp dir");
        let patterns_dir2 = temp_dir2.path();

        create_test_pattern_file(
            patterns_dir2,
            "invalid_app.yml",
            r#"
category: test
apps:
  - name: ""
    label: "Test App"
    exe_patterns: ["test"]
    tags: ["test"]
"#,
        );

        let db2 = PatternDatabase::load(patterns_dir2);
        // Должно загрузиться, но с предупреждениями
        assert!(db2.is_ok());

        // Тест с приложением без паттернов
        let temp_dir3 = TempDir::new().expect("temp dir");
        let patterns_dir3 = temp_dir3.path();

        create_test_pattern_file(
            patterns_dir3,
            "no_patterns.yml",
            r#"
category: test
apps:
  - name: "test"
    label: "Test App"
    tags: ["test"]
"#,
        );

        let db3 = PatternDatabase::load(patterns_dir3);
        // Должно загрузиться, но с предупреждениями
        assert!(db3.is_ok());
    }

    #[test]
    fn test_nonexistent_patterns_directory() {
        let nonexistent_dir = Path::new("/nonexistent/patterns/directory");
        let result = PatternDatabase::load(nonexistent_dir);
        assert!(result.is_err());

        // Проверяем, что ошибка содержит информативное сообщение
        let err = result.unwrap_err();
        assert!(err.to_string().contains("не существует"));
    }

    #[test]
    fn test_empty_patterns_directory() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаем пустую директорию
        let db = PatternDatabase::load(patterns_dir);
        assert!(db.is_ok()); // Должно успешно загрузиться, но без паттернов

        let db = db.unwrap();
        assert_eq!(db.all_patterns().len(), 0);
    }

    #[test]
    fn test_patterns_with_empty_files() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаем пустой YAML файл
        create_test_pattern_file(patterns_dir, "empty.yml", "");

        let db = PatternDatabase::load(patterns_dir);
        assert!(db.is_err()); // Должна быть ошибка при парсинге пустого YAML
    }

    #[test]
    fn test_patterns_with_invalid_yaml() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаем файл с недопустимым YAML
        create_test_pattern_file(patterns_dir, "invalid.yml", "this is not valid yaml: [");

        let db = PatternDatabase::load(patterns_dir);
        assert!(db.is_err()); // Должна быть ошибка при парсинге YAML
    }

    #[test]
    fn test_patterns_with_special_characters() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаем файл с недопустимыми символами в категории
        create_test_pattern_file(
            patterns_dir,
            "special_chars.yml",
            r#"
category: "test@category"
apps:
  - name: "test"
    label: "Test App"
    exe_patterns: ["test"]
    tags: ["test"]
"#,
        );

        let db = PatternDatabase::load(patterns_dir);
        // Должно загрузиться, но с предупреждениями в логах
        assert!(db.is_ok());
    }

    #[test]
    fn test_classify_process_with_empty_patterns() {
        let temp_dir = TempDir::new().expect("temp dir");
        let _patterns_dir = temp_dir.path();

        // Создаем базу с пустыми паттернами
        let db = PatternDatabase {
            patterns_by_category: HashMap::new(),
            all_patterns: Vec::new(),
            match_cache: LruCache::new(NonZeroUsize::new(512).unwrap()),
        };

        let mut process = ProcessRecord::default();
        process.pid = 1000;
        process.ppid = 1;
        process.uid = 1000;
        process.gid = 1000;
        process.exe = Some("unknown-app".to_string());
        process.state = "R".to_string();
        process.start_time = 0;
        process.uptime_sec = 100;
        process.tty_nr = 0;
        process.has_tty = false;

        // Классификация с пустой базой паттернов
        let pattern_db = Arc::new(Mutex::new(db));
        classify_process(&mut process, &pattern_db, None, None);

        // Должен остаться неклассифицированным
        assert_eq!(process.process_type, None);
        assert!(process.tags.is_empty());
    }

    #[test]
    fn test_classify_app_group_with_no_processes() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        let db = PatternDatabase::load(patterns_dir).expect("load patterns");

        let processes = vec![];

        let mut app_group = AppGroupRecord {
            app_group_id: "empty_group".to_string(),
            root_pid: 1000,
            process_ids: vec![],
            app_name: None,
            total_cpu_share: None,
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: None,
            has_gui_window: false,
            is_focused_group: false,
            tags: Vec::new(),
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        };

        // Классификация группы без процессов
        let pattern_db = Arc::new(Mutex::new(db));
        classify_app_group(&mut app_group, &processes, &pattern_db);

        // Должна остаться без тегов
        assert!(app_group.tags.is_empty());
    }

    #[test]
    fn test_classify_app_group_with_mixed_types() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        create_test_pattern_file(
            patterns_dir,
            "mixed.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser"]
"#,
        );

        create_test_pattern_file(
            patterns_dir,
            "ide.yml",
            r#"
category: ide
apps:
  - name: "vscode"
    label: "Visual Studio Code"
    exe_patterns: ["code"]
    tags: ["ide"]
"#,
        );

        let db = PatternDatabase::load(patterns_dir).expect("load patterns");

        let processes = vec![
            {
                let mut record = ProcessRecord::default();
                record.pid = 1000;
                record.ppid = 1;
                record.uid = 1000;
                record.gid = 1000;
                record.exe = Some("firefox".to_string());
                record.app_group_id = Some("mixed_group".to_string());
                record.state = "R".to_string();
                record.start_time = 0;
                record.uptime_sec = 100;
                record.tty_nr = 0;
                record.has_tty = false;
                record.process_type = Some("browser".to_string());
                record.tags = vec!["browser".to_string()];
                record
            },
            {
                let mut record = ProcessRecord::default();
                record.pid = 1001;
                record.ppid = 1;
                record.uid = 1000;
                record.gid = 1000;
                record.exe = Some("code".to_string());
                record.app_group_id = Some("mixed_group".to_string());
                record.state = "R".to_string();
                record.start_time = 0;
                record.uptime_sec = 100;
                record.tty_nr = 0;
                record.has_tty = false;
                record.process_type = Some("ide".to_string());
                record.tags = vec!["ide".to_string()];
                record.nice = 0;
                record
            },
        ];

        let mut app_group = AppGroupRecord {
            app_group_id: "mixed_group".to_string(),
            root_pid: 1000,
            process_ids: vec![1000, 1001],
            app_name: None,
            total_cpu_share: None,
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: None,
            has_gui_window: false,
            is_focused_group: false,
            tags: Vec::new(),
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        };

        // Классификация группы с разными типами процессов
        let pattern_db = Arc::new(Mutex::new(db));
        classify_app_group(&mut app_group, &processes, &pattern_db);

        // Должны быть агрегированы теги из разных типов
        assert_eq!(app_group.tags.len(), 2);
        assert!(app_group.tags.contains(&"browser".to_string()));
        assert!(app_group.tags.contains(&"ide".to_string()));
    }

    #[test]
    fn test_pattern_database_reload() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаём начальные паттерны
        create_test_pattern_file(
            patterns_dir,
            "browsers.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser"]
"#,
        );

        let mut db = PatternDatabase::load(patterns_dir).expect("load initial patterns");
        let initial_count = db.all_patterns().len();
        assert_eq!(initial_count, 1);

        // Добавляем новый паттерн
        create_test_pattern_file(
            patterns_dir,
            "ide.yml",
            r#"
category: ide
apps:
  - name: "vscode"
    label: "Visual Studio Code"
    exe_patterns: ["code"]
    tags: ["ide"]
"#,
        );

        // Перезагружаем паттерны
        let result = db.reload(patterns_dir).expect("reload patterns");

        assert!(result.has_changes());
        assert_eq!(result.new_files, 1);
        assert_eq!(result.changed_files, 0);
        assert_eq!(result.removed_files, 0);
        assert_eq!(result.patterns_before, 1);
        assert_eq!(result.patterns_after, 2);

        // Проверяем, что новый паттерн загружен
        let updated_count = db.all_patterns().len();
        assert_eq!(updated_count, 2);

        // Проверяем, что можно найти новый паттерн
        let matches = db.match_process(Some("code"), None, None);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].1.name, "vscode");
    }

    #[test]
    fn test_pattern_database_reload_with_changes() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаём начальные паттерны
        create_test_pattern_file(
            patterns_dir,
            "browser.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser"]
"#,
        );

        let mut db = PatternDatabase::load(patterns_dir).expect("load initial patterns");

        // Изменяем существующий паттерн
        create_test_pattern_file(
            patterns_dir,
            "browser.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox Updated"
    exe_patterns: ["firefox", "firefox-bin"]
    tags: ["browser", "updated"]
"#,
        );

        // Перезагружаем паттерны
        let result = db.reload(patterns_dir).expect("reload patterns");

        assert!(result.has_changes());
        assert_eq!(result.changed_files, 1);
        assert_eq!(result.new_files, 0);
        assert_eq!(result.removed_files, 0);

        // Проверяем, что паттерн обновлён
        let matches = db.match_process(Some("firefox-bin"), None, None);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].1.exe_patterns, vec!["firefox", "firefox-bin"]);
        assert!(matches[0].1.tags.contains(&"updated".to_string()));
    }

    #[test]
    fn test_pattern_database_reload_with_removals() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаём начальные паттерны
        create_test_pattern_file(
            patterns_dir,
            "browser.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser"]
  - name: "chrome"
    label: "Google Chrome"
    exe_patterns: ["chrome"]
    tags: ["browser"]
"#,
        );

        let mut db = PatternDatabase::load(patterns_dir).expect("load initial patterns");
        assert_eq!(db.all_patterns().len(), 2);

        // Удаляем один паттерн
        create_test_pattern_file(
            patterns_dir,
            "browser.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser"]
"#,
        );

        // Перезагружаем паттерны
        let result = db.reload(patterns_dir).expect("reload patterns");

        assert!(result.has_changes());
        assert_eq!(result.removed_files, 1);
        assert_eq!(result.patterns_before, 2);
        assert_eq!(result.patterns_after, 1);

        // Проверяем, что паттерн удалён
        assert_eq!(db.all_patterns().len(), 1);
        let matches = db.match_process(Some("chrome"), None, None);
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_pattern_database_has_changes() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаём начальные паттерны
        create_test_pattern_file(
            patterns_dir,
            "browser.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser"]
"#,
        );

        let db = PatternDatabase::load(patterns_dir).expect("load initial patterns");

        // Проверяем, что изменений нет
        let has_changes = db.has_changes(patterns_dir).expect("check changes");
        assert!(!has_changes);

        // Добавляем новый паттерн
        create_test_pattern_file(
            patterns_dir,
            "ide.yml",
            r#"
category: ide
apps:
  - name: "vscode"
    label: "Visual Studio Code"
    exe_patterns: ["code"]
    tags: ["ide"]
"#,
        );

        // Проверяем, что изменения обнаружены
        let has_changes = db.has_changes(patterns_dir).expect("check changes");
        assert!(has_changes);
    }

    #[test]
    fn test_pattern_database_has_changes_no_changes() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаём начальные паттерны
        create_test_pattern_file(
            patterns_dir,
            "browser.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser"]
"#,
        );

        let db = PatternDatabase::load(patterns_dir).expect("load initial patterns");

        // Проверяем, что изменений нет
        let has_changes = db.has_changes(patterns_dir).expect("check changes");
        assert!(!has_changes);

        // Проверяем ещё раз (без изменений)
        let has_changes = db.has_changes(patterns_dir).expect("check changes");
        assert!(!has_changes);
    }

    #[test]
    fn test_pattern_update_result() {
        let result = PatternUpdateResult {
            total_files: 2,
            total_patterns: 5,
            invalid_files: 1,
            changed_files: 1,
            new_files: 2,
            removed_files: 1,
            patterns_before: 4,
            patterns_after: 5,
        };

        assert!(result.has_changes());
        assert_eq!(
            result.summary(),
            "Updated: 1 changed, 2 new, 1 removed (5 patterns)"
        );

        let no_change_result = PatternUpdateResult {
            total_files: 1,
            total_patterns: 3,
            invalid_files: 0,
            changed_files: 0,
            new_files: 0,
            removed_files: 0,
            patterns_before: 3,
            patterns_after: 3,
        };

        assert!(!no_change_result.has_changes());
        assert_eq!(
            no_change_result.summary(),
            "No changes detected (3 patterns)"
        );
    }

    #[test]
    fn test_pattern_reload_error_handling() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаём начальные паттерны
        create_test_pattern_file(
            patterns_dir,
            "browser.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser"]
"#,
        );

        let mut db = PatternDatabase::load(patterns_dir).expect("load initial patterns");

        // Проверяем обработку ошибок для несуществующей директории
        let result = db.reload("/nonexistent/patterns");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("не существует"));
    }

    #[test]
    fn test_pattern_has_changes_error_handling() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаём начальные паттерны
        create_test_pattern_file(
            patterns_dir,
            "browser.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser"]
"#,
        );

        let db = PatternDatabase::load(patterns_dir).expect("load initial patterns");

        // Проверяем обработку ошибок для несуществующей директории
        let result = db.has_changes("/nonexistent/patterns");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("не существует"));
    }

    #[test]
    fn test_pattern_reload_preserves_existing_on_error() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаём начальные паттерны
        create_test_pattern_file(
            patterns_dir,
            "browser.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser"]
"#,
        );

        let mut db = PatternDatabase::load(patterns_dir).expect("load initial patterns");
        let initial_patterns = db.all_patterns().len();

        // Пытаемся перезагрузить из несуществующей директории
        let result = db.reload("/nonexistent/patterns");
        assert!(result.is_err());

        // Проверяем, что существующие паттерны сохранены
        assert_eq!(db.all_patterns().len(), initial_patterns);
    }

    #[test]
    fn test_enhanced_detection_container_applications() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаём паттерны для тестирования обнаружения контейнеров
        create_test_pattern_file(
            patterns_dir,
            "browser.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox", "firefox-bin"]
    tags: ["browser"]
"#,
        );

        create_test_pattern_file(
            patterns_dir,
            "containers.yml",
            r#"
category: container
apps:
  - name: "docker-container"
    label: "Docker Container"
    exe_patterns: ["docker"]
    tags: ["container", "sandbox"]
"#,
        );

        let mut db = PatternDatabase::load(patterns_dir).expect("load patterns");

        // Тестируем обнаружение приложения внутри Docker контейнера
        let process = ProcessRecord {
            pid: 1234,
            exe: Some("docker".to_string()),
            cmdline: Some("docker run -it firefox".to_string()),
            ..Default::default()
        };

        let matches = db.detect_application_enhanced(&process, None);
        
        // Должны обнаружить Docker контейнер
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].1.name, "docker-container");
        assert_eq!(matches[0].0 .0, "container");
    }

    #[test]
    fn test_enhanced_detection_command_line_arguments() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаём паттерны для тестирования обнаружения по аргументам
        create_test_pattern_file(
            patterns_dir,
            "browser.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox", "firefox-bin"]
    tags: ["browser"]
"#,
        );

        let mut db = PatternDatabase::load(patterns_dir).expect("load patterns");

        // Тестируем обнаружение по аргументам командной строки
        let process = ProcessRecord {
            pid: 5678,
            exe: Some("unknown-wrapper".to_string()),
            cmdline: Some("/usr/bin/unknown-wrapper firefox --new-window".to_string()),
            ..Default::default()
        };

        let matches = db.detect_application_enhanced(&process, None);
        
        // Должны обнаружить Firefox по аргументу командной строки
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].1.name, "firefox");
        assert_eq!(matches[0].0 .0, "browser");
    }

    #[test]
    fn test_enhanced_detection_executable_path() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаём паттерны для тестирования обнаружения по пути
        create_test_pattern_file(
            patterns_dir,
            "browser.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox", "firefox-bin"]
    tags: ["browser"]
"#,
        );

        let mut db = PatternDatabase::load(patterns_dir).expect("load patterns");

        // Тестируем обнаружение по полному пути к исполняемому файлу
        let process = ProcessRecord {
            pid: 9012,
            exe: Some("/usr/lib/firefox/firefox-bin".to_string()),
            ..Default::default()
        };

        let matches = db.detect_application_enhanced(&process, None);
        
        // Должны обнаружить Firefox по имени файла в пути
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].1.name, "firefox");
        assert_eq!(matches[0].0 .0, "browser");
    }

    #[test]
    fn test_enhanced_detection_systemd_service() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаём паттерны для тестирования обнаружения по systemd сервису
        create_test_pattern_file(
            patterns_dir,
            "browser.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox", "firefox-bin"]
    tags: ["browser"]
"#,
        );

        let mut db = PatternDatabase::load(patterns_dir).expect("load patterns");

        // Тестируем обнаружение по systemd сервису
        let process = ProcessRecord {
            pid: 3456,
            systemd_unit: Some("firefox.service".to_string()),
            ..Default::default()
        };

        let matches = db.detect_application_enhanced(&process, None);
        
        // Должны обнаружить Firefox по имени сервиса
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].1.name, "firefox");
        assert_eq!(matches[0].0 .0, "browser");
    }

    #[test]
    fn test_container_detection_logic() {
        // Тестируем логику обнаружения контейнеров
        assert!(PatternDatabase::is_container_or_sandbox_process("docker"));
        assert!(PatternDatabase::is_container_or_sandbox_process("podman"));
        assert!(PatternDatabase::is_container_or_sandbox_process("flatpak"));
        assert!(PatternDatabase::is_container_or_sandbox_process("snap"));
        assert!(PatternDatabase::is_container_or_sandbox_process("firejail"));
        assert!(PatternDatabase::is_container_or_sandbox_process("bubblewrap"));

        // Тестируем, что обычные приложения не обнаруживаются как контейнеры
        assert!(!PatternDatabase::is_container_or_sandbox_process("firefox"));
        assert!(!PatternDatabase::is_container_or_sandbox_process("chrome"));
        assert!(!PatternDatabase::is_container_or_sandbox_process("bash"));
    }

    #[test]
    fn test_enhanced_detection_fallback_behavior() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаём паттерны для тестирования fallback поведения
        create_test_pattern_file(
            patterns_dir,
            "browser.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser"]
"#,
        );

        let mut db = PatternDatabase::load(patterns_dir).expect("load patterns");

        // Тестируем процесс без совпадений - должен вернуться пустой список
        let process = ProcessRecord {
            pid: 7890,
            exe: Some("unknown-app".to_string()),
            ..Default::default()
        };

        let matches = db.detect_application_enhanced(&process, None);
        
        // Не должно быть совпадений
        assert!(matches.is_empty());
    }

    #[test]
    fn test_enhanced_detection_priority() {
        let temp_dir = TempDir::new().expect("temp dir");
        let patterns_dir = temp_dir.path();

        // Создаём паттерны для тестирования приоритета обнаружения
        create_test_pattern_file(
            patterns_dir,
            "browser.yml",
            r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    desktop_patterns: ["firefox.desktop"]
    tags: ["browser"]
"#,
        );

        let mut db = PatternDatabase::load(patterns_dir).expect("load patterns");

        // Тестируем, что базовое сопоставление имеет приоритет над улучшенными эвристиками
        let process = ProcessRecord {
            pid: 1111,
            exe: Some("firefox".to_string()),
            cmdline: Some("firefox --new-window".to_string()),
            ..Default::default()
        };

        let matches = db.detect_application_enhanced(&process, None);
        
        // Должно быть найдено через базовое сопоставление (по exe)
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].1.name, "firefox");
    }
}
