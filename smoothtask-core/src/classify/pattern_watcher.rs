//! Модуль для мониторинга изменений в паттерн-базе.
//!
//! Предоставляет функциональность для отслеживания изменений в директории с паттернами
//! и автоматической перезагрузки паттерн-базы при обнаружении изменений.

use anyhow::Result;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::sync::watch;
use tokio::time::{self, Duration};
use tracing::{debug, error, info, warn};

use crate::classify::rules::PatternDatabase;

/// Результат обновления паттернов.
///
/// Содержит статистику об изменениях в паттерн-базе после обновления.
#[derive(Debug, Clone)]
pub struct PatternUpdateResult {
    /// Общее количество файлов паттернов.
    pub total_files: usize,
    /// Общее количество паттернов.
    pub total_patterns: usize,
    /// Количество недопустимых файлов.
    pub invalid_files: usize,
    /// Количество изменённых файлов.
    pub changed_files: usize,
    /// Количество новых файлов.
    pub new_files: usize,
    /// Количество удалённых файлов.
    pub removed_files: usize,
    /// Количество паттернов до обновления.
    pub patterns_before: usize,
    /// Количество паттернов после обновления.
    pub patterns_after: usize,
}

impl PatternUpdateResult {
    /// Проверяет, есть ли значительные изменения.
    pub fn has_changes(&self) -> bool {
        self.changed_files > 0 || self.new_files > 0 || self.removed_files > 0
    }

    /// Возвращает краткое описание результата обновления.
    pub fn summary(&self) -> String {
        if self.has_changes() {
            format!(
                "Updated: {} changed, {} new, {} removed ({} patterns)",
                self.changed_files, self.new_files, self.removed_files, self.patterns_after
            )
        } else {
            format!("No changes detected ({} patterns)", self.patterns_after)
        }
    }
}

/// Структура для мониторинга изменений в директории с паттернами.
///
/// Использует комбинацию файлового мониторинга и периодической проверки
/// для обнаружения изменений в паттернах.
#[derive(Debug)]
pub struct PatternWatcher {
    /// Путь к директории с паттернами.
    patterns_dir: String,
    /// Совместно используемая база паттернов.
    pattern_db: Arc<Mutex<PatternDatabase>>,
    /// Канал для уведомления о изменениях.
    change_sender: watch::Sender<PatternUpdateResult>,
    /// Приёмная часть канала (для внешнего использования).
    change_receiver: watch::Receiver<PatternUpdateResult>,
    /// Конфигурация автообновления.
    config: PatternWatcherConfig,
}

/// Конфигурация для PatternWatcher.
#[derive(Debug, Clone)]
pub struct PatternWatcherConfig {
    /// Включить мониторинг изменений.
    pub enabled: bool,
    /// Интервал проверки изменений в секундах.
    pub interval_sec: u64,
    /// Включить уведомления об обновлении паттернов.
    pub notify_on_update: bool,
}

impl PatternWatcher {
    /// Создаёт новый PatternWatcher для указанной директории с паттернами.
    ///
    /// # Аргументы
    ///
    /// * `patterns_dir` - Путь к директории с паттернами для мониторинга.
    /// * `pattern_db` - Совместно используемая база паттернов для обновления.
    /// * `config` - Конфигурация мониторинга.
    ///
    /// # Возвращает
    ///
    /// `Result<Self>` - PatternWatcher, готовый к использованию.
    ///
    /// # Ошибки
    ///
    /// Возвращает ошибку, если:
    /// - Указанный путь не существует
    /// - Указанный путь не является директорией
    /// - Нет прав на чтение директории
    pub fn new(
        patterns_dir: impl Into<String>,
        pattern_db: Arc<Mutex<PatternDatabase>>,
        config: PatternWatcherConfig,
    ) -> Result<Self> {
        let patterns_dir = patterns_dir.into();
        let path = Path::new(&patterns_dir);

        // Проверяем, что директория существует и доступна для чтения
        if !path.exists() {
            anyhow::bail!("Patterns directory does not exist: {}", path.display());
        }

        if !path.is_dir() {
            anyhow::bail!("Patterns path is not a directory: {}", path.display());
        }

        // Проверяем права на чтение
        if let Err(e) = std::fs::metadata(path) {
            anyhow::bail!("Cannot access patterns directory {}: {}", path.display(), e);
        }

        // Создаём канал для уведомлений об изменениях
        let initial_result = PatternUpdateResult {
            total_files: 0,
            total_patterns: 0,
            invalid_files: 0,
            changed_files: 0,
            new_files: 0,
            removed_files: 0,
            patterns_before: 0,
            patterns_after: 0,
        };

        let (change_sender, change_receiver) = watch::channel(initial_result);

        Ok(Self {
            patterns_dir,
            pattern_db,
            change_sender,
            change_receiver,
            config,
        })
    }

    /// Возвращает путь к директории с паттернами.
    pub fn patterns_dir(&self) -> &str {
        &self.patterns_dir
    }

    /// Возвращает приёмник для уведомлений об изменениях.
    ///
    /// Когда паттерны изменяются, приёмник получит результат обновления.
    pub fn change_receiver(&self) -> watch::Receiver<PatternUpdateResult> {
        self.change_receiver.clone()
    }

    /// Запускает задачу для мониторинга изменений в директории с паттернами.
    ///
    /// # Возвращает
    ///
    /// `tokio::task::JoinHandle<Result<()>>` - Handle для управления задачей мониторинга.
    ///
    /// # Примечания
    ///
    /// Задача будет работать в фоновом режиме и отправлять уведомления через канал,
    /// когда паттерны будут изменены. Задача завершится, когда будет отменена (через Drop handle)
    /// или при ошибке мониторинга.
    pub fn start_watching(&self) -> tokio::task::JoinHandle<Result<()>> {
        let patterns_dir = self.patterns_dir.clone();
        let pattern_db = self.pattern_db.clone();
        let change_sender = self.change_sender.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            Self::watch_patterns_directory(patterns_dir, pattern_db, change_sender, config).await
        })
    }

    /// Основная функция мониторинга изменений в директории с паттернами.
    ///
    /// Использует комбинацию файлового мониторинга и периодической проверки
    /// для надежного обнаружения изменений.
    ///
    /// # Аргументы
    ///
    /// * `patterns_dir` - Путь к директории с паттернами.
    /// * `pattern_db` - Совместно используемая база паттернов для обновления.
    /// * `change_sender` - Отправитель для уведомлений об изменениях.
    /// * `config` - Конфигурация мониторинга.
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если мониторинг завершён успешно, иначе ошибка.
    async fn watch_patterns_directory(
        patterns_dir: String,
        pattern_db: Arc<Mutex<PatternDatabase>>,
        change_sender: watch::Sender<PatternUpdateResult>,
        config: PatternWatcherConfig,
    ) -> Result<()> {
        use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};

        let path = Path::new(&patterns_dir);

        info!(
            "Started watching patterns directory for changes: {} (interval: {}s)",
            patterns_dir, config.interval_sec
        );

        // Создаём watcher для файловой системы
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher: RecommendedWatcher = Watcher::new(
            tx,
            notify::Config::default().with_poll_interval(std::time::Duration::from_secs(1)),
        )?;

        // Начинаем наблюдение за директорией
        watcher.watch(path, RecursiveMode::Recursive)?;

        // Основной цикл мониторинга
        loop {
            // Устанавливаем таймаут для периодической проверки
            let interval = Duration::from_secs(config.interval_sec);
            let timeout = time::sleep(interval);

            // Используем select! без явного pinning для избежания PhantomPinned ошибок
            tokio::select! {
                // Периодическая проверка изменений
                _ = timeout => {
                    debug!(
                        "Performing periodic pattern update check (interval: {}s)",
                        config.interval_sec
                    );

                    // Выполняем проверку и обновление
                    if let Err(e) = Self::check_and_update_patterns(
                        &patterns_dir,
                        &pattern_db,
                        &change_sender,
                        &config,
                    ).await {
                        error!(
                            "Error during periodic pattern update: {}",
                            e
                        );
                    }
                }
            }

            // Обработка событий файловой системы с использованием неблокирующего recv
            match rx.try_recv() {
                Ok(Ok(event)) => {
                    // Проверяем, что событие относится к нашей директории с паттернами
                    if let Some(event_path) = event.paths.first() {
                        if event_path.starts_with(path) {
                            match event.kind {
                                EventKind::Modify(notify::event::ModifyKind::Data(_)) => {
                                    debug!("Patterns directory change detected: {:?}", event_path);
                                    // Выполняем проверку и обновление
                                    if let Err(e) = Self::check_and_update_patterns(
                                        &patterns_dir,
                                        &pattern_db,
                                        &change_sender,
                                        &config,
                                    )
                                    .await
                                    {
                                        error!(
                                            "Error during pattern update after filesystem event: {}",
                                            e
                                        );
                                    }
                                }
                                EventKind::Create(_) => {
                                    debug!(
                                        "New file detected in patterns directory: {:?}",
                                        event_path
                                    );
                                    // Выполняем проверку и обновление
                                    if let Err(e) = Self::check_and_update_patterns(
                                        &patterns_dir,
                                        &pattern_db,
                                        &change_sender,
                                        &config,
                                    )
                                    .await
                                    {
                                        error!(
                                            "Error during pattern update after create event: {}",
                                            e
                                        );
                                    }
                                }
                                EventKind::Remove(_) => {
                                    debug!(
                                        "File removed from patterns directory: {:?}",
                                        event_path
                                    );
                                    // Выполняем проверку и обновление
                                    if let Err(e) = Self::check_and_update_patterns(
                                        &patterns_dir,
                                        &pattern_db,
                                        &change_sender,
                                        &config,
                                    )
                                    .await
                                    {
                                        error!(
                                            "Error during pattern update after remove event: {}",
                                            e
                                        );
                                    }
                                }
                                _ => {
                                    // Игнорируем другие типы событий
                                    debug!(
                                        "Ignoring unrelated filesystem event for patterns directory"
                                    );
                                }
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    error!("Error receiving filesystem event: {}", e);
                    // Продолжаем работу, не завершаем мониторинг
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // Нет событий, продолжаем работу
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    error!("Filesystem watcher channel disconnected");
                    return Ok(());
                }
            }
        }
    }

    /// Проверяет и обновляет паттерны при обнаружении изменений.
    ///
    /// # Аргументы
    ///
    /// * `patterns_dir` - Путь к директории с паттернами.
    /// * `pattern_db` - Совместно используемая база паттернов для обновления.
    /// * `change_sender` - Отправитель для уведомлений об изменениях.
    /// * `config` - Конфигурация мониторинга.
    ///
    /// # Возвращает
    ///
    /// `Result<()>` - Ok, если обновление выполнено успешно, иначе ошибка.
    async fn check_and_update_patterns(
        patterns_dir: &str,
        pattern_db: &Arc<Mutex<PatternDatabase>>,
        change_sender: &watch::Sender<PatternUpdateResult>,
        config: &PatternWatcherConfig,
    ) -> Result<()> {
        // Проверяем, есть ли изменения
        let has_changes = {
            let db = pattern_db.lock().unwrap();
            db.has_changes(patterns_dir)
        }?;

        if !has_changes {
            debug!("No changes detected in patterns directory");
            return Ok(());
        }

        info!("Changes detected in patterns directory, performing update...");

        // Выполняем перезагрузку паттернов
        let mut db = pattern_db.lock().unwrap();
        let update_result = db.reload(patterns_dir)?;

        if update_result.has_changes() {
            info!("Pattern update completed: {}", update_result.summary());

            // Отправляем уведомление об изменении
            if change_sender.send(update_result.clone()).is_err() {
                warn!("Failed to send pattern update notification - no active receivers");
            }

            // Уведомляем пользователя, если включено
            if config.notify_on_update {
                // Здесь можно добавить интеграцию с системой уведомлений
                info!("Pattern database updated: {}", update_result.summary());
            }
        } else {
            debug!("Pattern update completed, but no significant changes detected");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Arc, Mutex};
    use tempfile::tempdir;

    fn create_test_pattern_file(dir: &Path, filename: &str, content: &str) -> std::path::PathBuf {
        let file_path = dir.join(filename);
        fs::write(&file_path, content).expect("write test pattern file");
        file_path
    }

    #[test]
    fn test_pattern_watcher_creation() {
        let temp_dir = tempdir().expect("tempdir");
        let patterns_dir = temp_dir.path().to_str().unwrap().to_string();

        // Создаём тестовый файл паттерна
        create_test_pattern_file(
            temp_dir.path(),
            "test.yml",
            r#"
category: test
apps:
  - name: "test-app"
    label: "Test Application"
    exe_patterns: ["test-app"]
    tags: ["test"]
"#,
        );

        let pattern_db = Arc::new(Mutex::new(
            PatternDatabase::load(&patterns_dir).expect("load patterns"),
        ));

        let config = PatternWatcherConfig {
            enabled: true,
            interval_sec: 60,
            notify_on_update: false,
        };

        let watcher =
            PatternWatcher::new(&patterns_dir, pattern_db, config).expect("watcher creation");

        assert_eq!(watcher.patterns_dir(), patterns_dir);
    }

    #[test]
    fn test_pattern_watcher_rejects_nonexistent_directory() {
        let pattern_db = Arc::new(Mutex::new(PatternDatabase::default()));

        let config = PatternWatcherConfig {
            enabled: true,
            interval_sec: 60,
            notify_on_update: false,
        };

        let result = PatternWatcher::new("/nonexistent/patterns", pattern_db, config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("does not exist"));
    }

    #[test]
    fn test_pattern_watcher_rejects_file_path() {
        let temp_file = tempfile::NamedTempFile::new().expect("tempfile");
        let file_path = temp_file.path().to_str().unwrap().to_string();

        let pattern_db = Arc::new(Mutex::new(PatternDatabase::default()));

        let config = PatternWatcherConfig {
            enabled: true,
            interval_sec: 60,
            notify_on_update: false,
        };

        let result = PatternWatcher::new(&file_path, pattern_db, config);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not a directory"));
    }

    #[tokio::test]
    async fn test_pattern_watcher_receiver() {
        let temp_dir = tempdir().expect("tempdir");
        let patterns_dir = temp_dir.path().to_str().unwrap().to_string();

        // Создаём тестовый файл паттерна
        create_test_pattern_file(
            temp_dir.path(),
            "test.yml",
            r#"
category: test
apps:
  - name: "test-app"
    label: "Test Application"
    exe_patterns: ["test-app"]
    tags: ["test"]
"#,
        );

        let pattern_db = Arc::new(Mutex::new(
            PatternDatabase::load(&patterns_dir).expect("load patterns"),
        ));

        let config = PatternWatcherConfig {
            enabled: true,
            interval_sec: 60,
            notify_on_update: false,
        };

        let watcher =
            PatternWatcher::new(&patterns_dir, pattern_db, config).expect("watcher creation");

        let receiver = watcher.change_receiver();

        // Проверяем, что изначально нет изменений
        let initial_result = receiver.borrow();
        assert!(!initial_result.has_changes());
    }

    #[tokio::test]
    async fn test_pattern_watcher_start_watching() {
        let temp_dir = tempdir().expect("tempdir");
        let patterns_dir = temp_dir.path().to_str().unwrap().to_string();

        // Создаём тестовый файл паттерна
        create_test_pattern_file(
            temp_dir.path(),
            "test.yml",
            r#"
category: test
apps:
  - name: "test-app"
    label: "Test Application"
    exe_patterns: ["test-app"]
    tags: ["test"]
"#,
        );

        let pattern_db = Arc::new(Mutex::new(
            PatternDatabase::load(&patterns_dir).expect("load patterns"),
        ));

        let config = PatternWatcherConfig {
            enabled: true,
            interval_sec: 60,
            notify_on_update: false,
        };

        let watcher =
            PatternWatcher::new(&patterns_dir, pattern_db, config).expect("watcher creation");

        let handle = watcher.start_watching();

        // Даём задаче немного времени для запуска
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Задача должна продолжать работать
        assert!(!handle.is_finished());

        // Отменяем задачу
        handle.abort();

        // Проверяем, что задача завершилась
        assert!(handle.is_finished());
    }

    #[tokio::test]
    async fn test_pattern_update_result() {
        let result = PatternUpdateResult {
            total_files: 2,
            total_patterns: 5,
            invalid_files: 0,
            changed_files: 1,
            new_files: 2,
            removed_files: 0,
            patterns_before: 4,
            patterns_after: 5,
        };

        assert!(result.has_changes());
        assert_eq!(result.summary(), "Updated: 1 changed, 2 new (5 patterns)");

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
}
