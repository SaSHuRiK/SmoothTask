//! Классификация процессов и AppGroup по паттернам из конфигурационных файлов.
//!
//! Паттерны загружаются из YAML файлов в директории patterns/ и используются
//! для определения типа процесса (process_type) и тегов (tags).

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::logging::snapshots::{AppGroupRecord, ProcessRecord};

/// Категория паттернов (browser, ide, terminal, batch, и т.д.).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PatternCategory(pub String);

/// Паттерн для одного приложения.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppPattern {
    /// Уникальное имя приложения (например, "firefox", "vscode").
    pub name: String,
    /// Человекочитаемое название.
    pub label: String,
    /// Паттерны для сопоставления с exe/comm процесса.
    #[serde(default)]
    pub exe_patterns: Vec<String>,
    /// Паттерны для сопоставления с desktop-файлом.
    #[serde(default)]
    pub desktop_patterns: Vec<String>,
    /// Паттерны для сопоставления с cgroup_path.
    #[serde(default)]
    pub cgroup_patterns: Vec<String>,
    /// Теги, которые будут присвоены процессу при совпадении.
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Файл с паттернами одной категории.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatternFile {
    /// Категория паттернов.
    pub category: PatternCategory,
    /// Список паттернов приложений в этой категории.
    pub apps: Vec<AppPattern>,
}

/// База паттернов для классификации процессов.
#[derive(Debug, Clone)]
pub struct PatternDatabase {
    /// Маппинг категория -> список паттернов.
    patterns_by_category: HashMap<PatternCategory, Vec<AppPattern>>,
    /// Плоский список всех паттернов для быстрого поиска.
    all_patterns: Vec<(PatternCategory, AppPattern)>,
}

impl PatternDatabase {
    /// Загружает паттерны из директории с YAML файлами.
    ///
    /// # Аргументы
    ///
    /// * `patterns_dir` - путь к директории с YAML файлами паттернов
    ///
    /// # Возвращает
    ///
    /// База данных паттернов или ошибку при загрузке/парсинге.
    pub fn load(patterns_dir: impl AsRef<Path>) -> Result<Self> {
        let patterns_dir = patterns_dir.as_ref();
        let mut patterns_by_category = HashMap::new();
        let mut all_patterns = Vec::new();

        let entries = fs::read_dir(patterns_dir)
            .with_context(|| format!("Failed to read patterns directory: {:?}", patterns_dir))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Пропускаем не-YAML файлы
            if path.extension().and_then(|s| s.to_str()) != Some("yml")
                && path.extension().and_then(|s| s.to_str()) != Some("yaml")
            {
                continue;
            }

            // Загружаем и парсим YAML файл
            let content = fs::read_to_string(&path)
                .with_context(|| format!("Failed to read pattern file: {:?}", path))?;

            let pattern_file: PatternFile = serde_yaml::from_str(&content)
                .with_context(|| format!("Failed to parse pattern file: {:?}", path))?;

            // Добавляем паттерны в базу
            let category = pattern_file.category.clone();
            let apps = pattern_file.apps;

            for app in apps.clone() {
                all_patterns.push((category.clone(), app));
            }

            patterns_by_category.insert(category, apps);
        }

        Ok(Self {
            patterns_by_category,
            all_patterns,
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
        &self,
        exe: Option<&str>,
        desktop_id: Option<&str>,
        cgroup_path: Option<&str>,
    ) -> Vec<(&PatternCategory, &AppPattern)> {
        let mut matches = Vec::new();

        for (category, pattern) in &self.all_patterns {
            if Self::pattern_matches(pattern, exe, desktop_id, cgroup_path) {
                matches.push((category, pattern));
            }
        }

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
    fn matches_pattern(text: &str, pattern: &str) -> bool {
        // Если паттерн не содержит wildcard символов, используем точное совпадение
        if !pattern.contains('*') && !pattern.contains('?') {
            return text == pattern;
        }

        // Используем рекурсивный алгоритм сопоставления glob паттернов
        Self::glob_match_recursive(text.as_bytes(), pattern.as_bytes())
    }

    /// Рекурсивная функция для сопоставления glob паттернов.
    ///
    /// Алгоритм:
    /// - `*` соответствует любой последовательности символов (включая пустую)
    /// - `?` соответствует одному символу
    /// - Обычные символы должны совпадать точно
    fn glob_match_recursive(text: &[u8], pattern: &[u8]) -> bool {
        // Базовый случай: если паттерн пуст, текст тоже должен быть пуст
        if pattern.is_empty() {
            return text.is_empty();
        }

        // Базовый случай: если текст пуст, паттерн должен состоять только из `*`
        if text.is_empty() {
            return pattern.iter().all(|&b| b == b'*');
        }

        match pattern[0] {
            b'*' => {
                // `*` может соответствовать:
                // 1. Пустой строке (пропускаем `*` и продолжаем)
                // 2. Одному или более символам (пропускаем один символ из text и продолжаем)
                // 3. Всей оставшейся строке (если после `*` ничего нет)

                // Оптимизация: если паттерн заканчивается на `*`, он соответствует всему
                if pattern.len() == 1 {
                    return true;
                }

                // Пробуем все возможные совпадения для `*`
                for i in 0..=text.len() {
                    if Self::glob_match_recursive(&text[i..], &pattern[1..]) {
                        return true;
                    }
                }
                false
            }
            b'?' => {
                // `?` соответствует одному символу
                if text.is_empty() {
                    false
                } else {
                    Self::glob_match_recursive(&text[1..], &pattern[1..])
                }
            }
            ch => {
                // Обычный символ должен совпадать точно
                if text.is_empty() || text[0] != ch {
                    false
                } else {
                    Self::glob_match_recursive(&text[1..], &pattern[1..])
                }
            }
        }
    }
}

/// Классифицирует процесс по паттернам и заполняет process_type и tags.
///
/// # Аргументы
///
/// * `process` - процесс для классификации (будет изменён in-place)
/// * `pattern_db` - база данных паттернов
/// * `desktop_id` - desktop ID процесса (опционально, из window_info или systemd_unit)
pub fn classify_process(
    process: &mut ProcessRecord,
    pattern_db: &PatternDatabase,
    desktop_id: Option<&str>,
) {
    // Извлекаем desktop_id из systemd_unit, если не передан явно
    let desktop_id = desktop_id.or_else(|| {
        process
            .systemd_unit
            .as_ref()
            .and_then(|unit| unit.strip_suffix(".service"))
            .map(|s| s as &str)
    });

    // Ищем совпадающие паттерны
    let matches = pattern_db.match_process(
        process.exe.as_deref(),
        desktop_id,
        process.cgroup_path.as_deref(),
    );

    if matches.is_empty() {
        // Процесс не классифицирован
        return;
    }

    // Собираем все теги из совпадающих паттернов
    let mut all_tags = HashSet::new();
    for (_, pattern) in &matches {
        for tag in &pattern.tags {
            all_tags.insert(tag.clone());
        }
    }

    // Выбираем process_type из первой категории (можно улучшить логику выбора)
    if let Some((category, _)) = matches.first() {
        process.process_type = Some(category.0.clone());
    }

    // Заполняем теги (уникальные, отсортированные)
    process.tags = all_tags.into_iter().collect();
    process.tags.sort();
}

/// Классифицирует AppGroup, агрегируя теги и типы из процессов группы.
///
/// # Аргументы
///
/// * `app_group` - группа приложений для классификации (будет изменена in-place)
/// * `processes` - все процессы (для поиска процессов группы)
/// * `_pattern_db` - база данных паттернов (зарезервировано для будущего использования)
pub fn classify_app_group(
    app_group: &mut AppGroupRecord,
    processes: &[ProcessRecord],
    _pattern_db: &PatternDatabase,
) {
    // Находим процессы этой группы
    let group_processes: Vec<&ProcessRecord> = processes
        .iter()
        .filter(|p| p.app_group_id.as_deref() == Some(app_group.app_group_id.as_str()))
        .collect();

    if group_processes.is_empty() {
        return;
    }

    // Собираем все теги и типы из процессов группы
    let mut all_tags = HashSet::new();
    let mut process_types = HashSet::new();

    for process in group_processes {
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

    // Если все процессы имеют один тип, можно установить app_name
    // (это можно улучшить позже)
    if process_types.len() == 1 {
        // Можно использовать тип как app_name, если он не установлен
        // Но лучше оставить app_name как есть, так как он может быть более специфичным
        let _ = process_types.iter().next();
    }
}

/// Классифицирует все процессы и группы в снапшоте.
///
/// Это удобная функция-обёртка, которая классифицирует все процессы,
/// а затем агрегирует теги для групп.
pub fn classify_all(
    processes: &mut [ProcessRecord],
    app_groups: &mut [AppGroupRecord],
    pattern_db: &PatternDatabase,
) {
    // Классифицируем все процессы
    for process in processes.iter_mut() {
        classify_process(process, pattern_db, None);
    }

    // Классифицируем все группы (агрегируем теги из процессов)
    for app_group in app_groups.iter_mut() {
        classify_app_group(app_group, processes, pattern_db);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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

        let db = PatternDatabase::load(patterns_dir).expect("load patterns");

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

        let db = PatternDatabase::load(patterns_dir).expect("load patterns");

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

        let db = PatternDatabase::load(patterns_dir).expect("load patterns");

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
        };

        classify_process(&mut process, &db, None);

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

        let mut process = ProcessRecord {
            pid: 1000,
            ppid: 1,
            uid: 1000,
            gid: 1000,
            exe: Some("unknown-app".to_string()),
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
            is_audio_client: false,
            has_active_stream: false,
            process_type: None,
            tags: Vec::new(),
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
        };

        classify_process(&mut process, &db, None);

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
            ProcessRecord {
                pid: 1000,
                ppid: 1,
                uid: 1000,
                gid: 1000,
                exe: Some("firefox".to_string()),
                cmdline: None,
                cgroup_path: None,
                systemd_unit: None,
                app_group_id: Some("group1".to_string()),
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
                is_audio_client: false,
                has_active_stream: false,
                process_type: Some("browser".to_string()),
                tags: vec!["browser".to_string(), "gui_interactive".to_string()],
                nice: 0,
                ionice_class: None,
                ionice_prio: None,
                teacher_priority_class: None,
                teacher_score: None,
            },
            ProcessRecord {
                pid: 1001,
                ppid: 1000,
                uid: 1000,
                gid: 1000,
                exe: Some("firefox-bin".to_string()),
                cmdline: None,
                cgroup_path: None,
                systemd_unit: None,
                app_group_id: Some("group1".to_string()),
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
                is_audio_client: false,
                has_active_stream: false,
                process_type: Some("browser".to_string()),
                tags: vec!["browser".to_string(), "gui_interactive".to_string()],
                nice: 0,
                ionice_class: None,
                ionice_prio: None,
                teacher_priority_class: None,
                teacher_score: None,
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
            total_rss_mb: None,
            has_gui_window: false,
            is_focused_group: false,
            tags: Vec::new(),
            priority_class: None,
        };

        classify_app_group(&mut app_group, &processes, &db);

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

        let mut processes = vec![ProcessRecord {
            pid: 1000,
            ppid: 1,
            uid: 1000,
            gid: 1000,
            exe: Some("firefox".to_string()),
            cmdline: None,
            cgroup_path: None,
            systemd_unit: None,
            app_group_id: Some("group1".to_string()),
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
            is_audio_client: false,
            has_active_stream: false,
            process_type: None,
            tags: Vec::new(),
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
        }];

        let mut app_groups = vec![AppGroupRecord {
            app_group_id: "group1".to_string(),
            root_pid: 1000,
            process_ids: vec![1000],
            app_name: None,
            total_cpu_share: None,
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_rss_mb: None,
            has_gui_window: false,
            is_focused_group: false,
            tags: Vec::new(),
            priority_class: None,
        }];

        classify_all(&mut processes, &mut app_groups, &db);

        // Процесс должен быть классифицирован
        assert_eq!(processes[0].process_type, Some("browser".to_string()));
        assert!(processes[0].tags.contains(&"browser".to_string()));

        // Группа должна иметь агрегированные теги
        assert!(!app_groups[0].tags.is_empty());
        assert!(app_groups[0].tags.contains(&"browser".to_string()));
    }
}
