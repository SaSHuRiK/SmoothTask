//! Модуль для мониторинга изменений конфигурационных файлов.
//!
//! Предоставляет функциональность для отслеживания изменений в конфигурационных файлах
//! и уведомления об этих изменениях. Используется для реализации динамической перезагрузки
//! конфигурации без перезапуска демона.

use anyhow::{Context, Result};
use std::path::Path;

use tokio::sync::watch;

/// Структура для мониторинга изменений конфигурационного файла.
/// Использует библиотеку notify для отслеживания изменений в файловой системе.
#[derive(Debug)]
pub struct ConfigWatcher {
    /// Путь к конфигурационному файлу.
    config_path: String,
    /// Канал для уведомления о изменениях.
    /// Когда файл изменяется, отправляется сигнал через этот канал.
    change_sender: watch::Sender<bool>,
    /// Приёмная часть канала (для внешнего использования).
    change_receiver: watch::Receiver<bool>,
}

impl ConfigWatcher {
    /// Создаёт новый ConfigWatcher для указанного конфигурационного файла.
    /// 
    /// # Аргументы
    /// * `config_path` - Путь к конфигурационному файлу для мониторинга.
    /// 
    /// # Возвращает
    /// `Result<Self>` - ConfigWatcher, готовый к использованию.
    /// 
    /// # Ошибки
    /// Возвращает ошибку, если:
    /// - Указанный путь не существует
    /// - Указанный путь не является файлом
    /// - Нет прав на чтение файла
    pub fn new(config_path: impl Into<String>) -> Result<Self> {
        let config_path = config_path.into();
        let path = Path::new(&config_path);
        
        // Проверяем, что файл существует и доступен для чтения
        if !path.exists() {
            anyhow::bail!(
                "Config file does not exist: {}",
                path.display()
            );
        }
        
        if !path.is_file() {
            anyhow::bail!(
                "Config path is not a file: {}",
                path.display()
            );
        }
        
        // Проверяем права на чтение
        if let Err(e) = std::fs::metadata(path) {
            anyhow::bail!(
                "Cannot access config file {}: {}",
                path.display(),
                e
            );
        }
        
        // Создаём канал для уведомлений об изменениях
        let (change_sender, change_receiver) = watch::channel(false);
        
        Ok(Self {
            config_path,
            change_sender,
            change_receiver,
        })
    }
    
    /// Возвращает путь к конфигурационному файлу.
    pub fn config_path(&self) -> &str {
        &self.config_path
    }
    
    /// Возвращает приёмник для уведомлений об изменениях.
    /// Когда файл изменяется, приёмник получит сигнал.
    pub fn change_receiver(&self) -> watch::Receiver<bool> {
        self.change_receiver.clone()
    }
    
    /// Запускает задачу для мониторинга изменений конфигурационного файла.
    /// 
    /// # Возвращает
    /// `tokio::task::JoinHandle<Result<()>>` - Handle для управления задачей мониторинга.
    /// 
    /// # Примечания
    /// Задача будет работать в фоновом режиме и отправлять уведомления через канал,
    /// когда файл будет изменён. Задача завершится, когда будет отменена (через Drop handle)
    /// или при ошибке мониторинга.
    pub fn start_watching(&self) -> tokio::task::JoinHandle<Result<()>> {
        let config_path = self.config_path.clone();
        let change_sender = self.change_sender.clone();
        
        tokio::spawn(async move {
            Self::watch_config_file(config_path, change_sender).await
        })
    }
    
    /// Основная функция мониторинга изменений конфигурационного файла.
    /// 
    /// # Аргументы
    /// * `config_path` - Путь к конфигурационному файлу.
    /// * `change_sender` - Отправитель для уведомлений об изменениях.
    /// 
    /// # Возвращает
    /// `Result<()>` - Ok, если мониторинг завершён успешно, иначе ошибка.
    async fn watch_config_file(
        config_path: String,
        change_sender: watch::Sender<bool>,
    ) -> Result<()> {
        use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
        
        let path = Path::new(&config_path);
        let parent_dir = path.parent().with_context(
            || format!("Cannot determine parent directory for config file: {}", config_path)
        )?;
        
        // Создаём watcher
        let (tx, mut rx) = std::sync::mpsc::channel();
        let mut watcher: RecommendedWatcher = Watcher::new(
            tx,
            notify::Config::default()
                .with_poll_interval(std::time::Duration::from_secs(1)),
        )?;
        
        // Начинаем наблюдение за родительской директорией
        // (наблюдаем за директорией, а не за файлом, чтобы не пропустить события)
        watcher.watch(parent_dir, RecursiveMode::NonRecursive)?;
        
        tracing::info!(
            "Started watching config file for changes: {}",
            config_path
        );
        
        // Основной цикл обработки событий
        while let Ok(event) = rx.recv() {
            match event {
                Ok(Event {
                    kind: EventKind::Modify(modify_kind),
                    paths,
                    ..
                }) => {
                    // Проверяем, что событие относится к нашему конфигурационному файлу
                    if let Some(event_path) = paths.first() {
                        if event_path == path {
                            // Игнорируем события модификации, которые не являются изменением содержимого
                            // (например, изменение метаданных)
                            match modify_kind {
                                notify::event::ModifyKind::Data(_) => {
                                    tracing::info!(
                                        "Config file changed: {}",
                                        config_path
                                    );
                                    
                                    // Отправляем сигнал об изменении
                                    if change_sender.send(true).is_err() {
                                        tracing::warn!(
                                            "Failed to send config change notification - no active receivers"
                                        );
                                    }
                                }
                                _ => {
                                    // Игнорируем другие типы модификаций
                                    tracing::debug!(
                                        "Ignoring non-data modification event for config file: {}",
                                        config_path
                                    );
                                }
                            }
                        }
                    }
                }
                Ok(Event {
                    kind: EventKind::Remove(_),
                    paths,
                    ..
                }) => {
                    // Если файл был удалён, это критическая ситуация
                    if let Some(event_path) = paths.first() {
                        if event_path == path {
                            tracing::error!(
                                "Config file was removed: {}",
                                config_path
                            );
                            anyhow::bail!(
                                "Config file was removed during monitoring: {}",
                                config_path
                            );
                        }
                    }
                }
                Ok(Event {
                    kind: EventKind::Create(_),
                    paths,
                    ..
                }) => {
                    // Если файл был создан (например, после удаления и повторного создания)
                    if let Some(event_path) = paths.first() {
                        if event_path == path {
                            tracing::info!(
                                "Config file was recreated: {}",
                                config_path
                            );
                            
                            // Отправляем сигнал об изменении
                            if change_sender.send(true).is_err() {
                                tracing::warn!(
                                    "Failed to send config change notification - no active receivers"
                                );
                            }
                        }
                    }
                }
                Ok(_) => {
                    // Игнорируем другие типы событий
                    tracing::debug!(
                        "Ignoring unrelated filesystem event for config file: {}",
                        config_path
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Error receiving filesystem event: {}",
                        e
                    );
                    anyhow::bail!(
                        "Filesystem watcher error: {}",
                        e
                    );
                }
            }
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_watcher_creation() {
        let temp_file = NamedTempFile::new().expect("tempfile");
        let file_path = temp_file.path().to_str().unwrap().to_string();
        
        let watcher = ConfigWatcher::new(&file_path).expect("watcher creation");
        assert_eq!(watcher.config_path(), file_path);
    }
    
    #[test]
    fn test_config_watcher_rejects_nonexistent_file() {
        let result = ConfigWatcher::new("/nonexistent/config.yml");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("does not exist"));
    }
    
    #[test]
    fn test_config_watcher_rejects_directory() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let dir_path = temp_dir.path().to_str().unwrap().to_string();
        
        let result = ConfigWatcher::new(&dir_path);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("not a file"));
    }
    
    #[tokio::test]
    async fn test_config_watcher_receiver() {
        let temp_file = NamedTempFile::new().expect("tempfile");
        let file_path = temp_file.path().to_str().unwrap().to_string();
        
        let watcher = ConfigWatcher::new(&file_path).expect("watcher creation");
        let mut receiver = watcher.change_receiver();
        
        // Проверяем, что изначально нет изменений
        assert!(!*receiver.borrow());
    }
    
    #[tokio::test]
    async fn test_config_watcher_start_watching() {
        let temp_file = NamedTempFile::new().expect("tempfile");
        let file_path = temp_file.path().to_str().unwrap().to_string();
        
        let watcher = ConfigWatcher::new(&file_path).expect("watcher creation");
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
}