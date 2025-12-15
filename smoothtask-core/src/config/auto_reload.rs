//! Модуль автоматического обновления конфигурации.
//!
//! Предоставляет функциональность для автоматической перезагрузки конфигурации
//! при обнаружении изменений в конфигурационных файлах. Интегрируется с ConfigWatcher
//! для мониторинга изменений и предоставляет механизм безопасной перезагрузки
//! конфигурации без перезапуска демона.

use anyhow::{Context, Result};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::{watch, RwLock};

use crate::config::config_struct::Config;
use crate::config::watcher::ConfigWatcher;

/// Структура для управления автоматической перезагрузкой конфигурации.
/// Отслеживает изменения в конфигурационных файлах и автоматически перезагружает
/// конфигурацию при обнаружении изменений.
#[derive(Debug)]
pub struct ConfigAutoReload {
    /// Текущая конфигурация.
    current_config: Arc<RwLock<Config>>,
    /// Монитор изменений конфигурационного файла.
    config_watcher: ConfigWatcher,
    /// Канал для уведомления о перезагрузке конфигурации.
    reload_sender: watch::Sender<bool>,
    /// Приёмная часть канала (для внешнего использования).
    reload_receiver: watch::Receiver<bool>,
    /// Флаг, указывающий, что перезагрузка в процессе.
    is_reloading: Arc<RwLock<bool>>,
}

impl ConfigAutoReload {
    /// Создаёт новый ConfigAutoReload для указанного конфигурационного файла.
    ///
    /// # Аргументы
    /// * `config_path` - Путь к конфигурационному файлу для мониторинга.
    /// * `initial_config` - Начальная конфигурация.
    ///
    /// # Возвращает
    /// `Result<Self>` - ConfigAutoReload, готовый к использованию.
    ///
    /// # Ошибки
    /// Возвращает ошибку, если:
    /// - Указанный путь не существует
    /// - Указанный путь не является файлом
    /// - Нет прав на чтение файла
    pub fn new(config_path: impl Into<String>, initial_config: Config) -> Result<Self> {
        let config_watcher = ConfigWatcher::new(config_path.into())?;
        let (reload_sender, reload_receiver) = watch::channel(false);
        
        Ok(Self {
            current_config: Arc::new(RwLock::new(initial_config)),
            config_watcher,
            reload_sender,
            reload_receiver,
            is_reloading: Arc::new(RwLock::new(false)),
        })
    }

    /// Возвращает текущую конфигурацию.
    pub async fn get_current_config(&self) -> Config {
        self.current_config.read().await.clone()
    }

    /// Возвращает приёмник для уведомлений о перезагрузке конфигурации.
    /// Когда конфигурация перезагружается, приёмник получит сигнал.
    pub fn reload_receiver(&self) -> watch::Receiver<bool> {
        self.reload_receiver.clone()
    }

    /// Запускает задачу для мониторинга изменений конфигурационного файла
    /// и автоматической перезагрузки конфигурации.
    ///
    /// # Возвращает
    /// `tokio::task::JoinHandle<Result<()>>` - Handle для управления задачей мониторинга.
    ///
    /// # Примечания
    /// Задача будет работать в фоновом режиме и автоматически перезагружать
    /// конфигурацию при обнаружении изменений. Задача завершится, когда будет
    /// отменена (через Drop handle) или при ошибке мониторинга.
    pub fn start_auto_reload(&self) -> tokio::task::JoinHandle<Result<()>> {
        let config_path = self.config_watcher.config_path().to_string();
        let change_receiver = self.config_watcher.change_receiver();
        let reload_sender = self.reload_sender.clone();
        let current_config = self.current_config.clone();
        let is_reloading = self.is_reloading.clone();

        tokio::spawn(async move { 
            Self::auto_reload_task(
                config_path, 
                change_receiver, 
                reload_sender, 
                current_config, 
                is_reloading
            ).await 
        })
    }

    /// Основная задача автоматической перезагрузки конфигурации.
    ///
    /// # Аргументы
    /// * `config_path` - Путь к конфигурационному файлу.
    /// * `change_receiver` - Приёмник уведомлений об изменениях.
    /// * `reload_sender` - Отправитель уведомлений о перезагрузке.
    /// * `current_config` - Текущая конфигурация.
    /// * `is_reloading` - Флаг состояния перезагрузки.
    ///
    /// # Возвращает
    /// `Result<()>` - Ok, если перезагрузка завершена успешно, иначе ошибка.
    async fn auto_reload_task(
        config_path: String,
        mut change_receiver: watch::Receiver<bool>,
        reload_sender: watch::Sender<bool>,
        current_config: Arc<RwLock<Config>>,
        is_reloading: Arc<RwLock<bool>>,
    ) -> Result<()> {
        tracing::info!("Started auto-reload task for config file: {}", config_path);

        loop {
            // Проверяем, есть ли изменения в конфигурационном файле
            if *change_receiver.borrow() {
                tracing::info!("Config file change detected, initiating reload...");
                
                // Проверяем, что перезагрузка не выполняется в данный момент
                {
                    let reloading = *is_reloading.read().await;
                    if reloading {
                        tracing::warn!("Reload already in progress, skipping this change");
                        // Сбрасываем флаг изменения
                        change_receiver.borrow_and_update();
                        continue;
                    }
                }

                // Устанавливаем флаг перезагрузки
                *is_reloading.write().await = true;

                // Пытаемся перезагрузить конфигурацию
                match Self::reload_config(&config_path).await {
                    Ok(new_config) => {
                        tracing::info!("Config reloaded successfully");
                        
                        // Обновляем текущую конфигурацию
                        *current_config.write().await = new_config;
                        
                        // Отправляем уведомление о перезагрузке
                        if reload_sender.send(true).is_err() {
                            tracing::warn!("Failed to send reload notification - no active receivers");
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to reload config: {}", e);
                        // Не обновляем текущую конфигурацию при ошибке
                    }
                }

                // Сбрасываем флаг перезагрузки
                *is_reloading.write().await = false;
                
                // Сбрасываем флаг изменения
                change_receiver.borrow_and_update();
            }

            // Небольшая задержка для избежания busy waiting
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    /// Перезагружает конфигурацию из файла.
    ///
    /// # Аргументы
    /// * `config_path` - Путь к конфигурационному файлу.
    ///
    /// # Возвращает
    /// `Result<Config>` - Загруженная конфигурация или ошибка.
    async fn reload_config(config_path: &str) -> Result<Config> {
        let path = Path::new(config_path);
        
        // Проверяем, что файл существует
        if !path.exists() {
            anyhow::bail!("Config file does not exist during reload: {}", config_path);
        }

        // Чтение и парсинг конфигурационного файла
        let config_content = tokio::fs::read_to_string(path)
            .await
            .context("Failed to read config file during reload")?;

        let config: Config = serde_yaml::from_str(&config_content)
            .context("Failed to parse config file during reload")?;

        // Валидация конфигурации
        config.validate()
            .context("Config validation failed during reload")?;

        tracing::info!("Successfully loaded and validated new config from {}", config_path);
        
        Ok(config)
    }

    /// Запускает мониторинг изменений конфигурационного файла.
    ///
    /// # Возвращает
    /// `tokio::task::JoinHandle<Result<()>>` - Handle для управления задачей мониторинга.
    pub fn start_watching(&self) -> tokio::task::JoinHandle<Result<()>> {
        self.config_watcher.start_watching()
    }

    /// Проверяет, выполняется ли в данный момент перезагрузка конфигурации.
    pub async fn is_reloading(&self) -> bool {
        *self.is_reloading.read().await
    }

    /// Возвращает текущую конфигурацию (Arc<RwLock<Config>>).
    pub fn current_config_arc(&self) -> Arc<RwLock<Config>> {
        self.current_config.clone()
    }

    /// Возвращает путь к конфигурационному файлу.
    pub fn config_path(&self) -> &str {
        self.config_watcher.config_path()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[test]
    fn test_config_auto_reload_creation() {
        let temp_file = NamedTempFile::new().expect("tempfile");
        let file_path = temp_file.path().to_str().unwrap().to_string();

        // Создаём минимальную валидную конфигурацию
        let config_content = r#"
            polling_interval_ms: 1000
            max_candidates: 10
            dry_run_default: true
            enable_snapshot_logging: false
            thresholds:
              cpu_high: 80.0
              memory_high: 80.0
            paths:
              snapshot_db_path: "/tmp/test.db"
        "#;

        // Записываем конфигурацию в временный файл
        std::fs::write(&file_path, config_content).expect("write config");

        let initial_config: Config = serde_yaml::from_str(config_content).expect("parse config");
        
        let auto_reload = ConfigAutoReload::new(&file_path, initial_config).expect("auto reload creation");
        assert_eq!(auto_reload.config_watcher.config_path(), file_path);
    }

    #[tokio::test]
    async fn test_config_auto_reload_get_current_config() {
        let temp_file = NamedTempFile::new().expect("tempfile");
        let file_path = temp_file.path().to_str().unwrap().to_string();

        // Создаём минимальную валидную конфигурацию
        let config_content = r#"
            polling_interval_ms: 1000
            max_candidates: 10
            dry_run_default: true
            enable_snapshot_logging: false
            thresholds:
              cpu_high: 80.0
              memory_high: 80.0
            paths:
              snapshot_db_path: "/tmp/test.db"
        "#;

        // Записываем конфигурацию в временный файл
        std::fs::write(&file_path, config_content).expect("write config");

        let initial_config: Config = serde_yaml::from_str(config_content).expect("parse config");
        
        let auto_reload = ConfigAutoReload::new(&file_path, initial_config).expect("auto reload creation");
        
        let current_config = auto_reload.get_current_config().await;
        assert_eq!(current_config.polling_interval_ms, 1000);
        assert_eq!(current_config.max_candidates, 10);
    }

    #[tokio::test]
    async fn test_config_auto_reload_receiver() {
        let temp_file = NamedTempFile::new().expect("tempfile");
        let file_path = temp_file.path().to_str().unwrap().to_string();

        // Создаём минимальную валидную конфигурацию
        let config_content = r#"
            polling_interval_ms: 1000
            max_candidates: 10
            dry_run_default: true
            enable_snapshot_logging: false
            thresholds:
              cpu_high: 80.0
              memory_high: 80.0
            paths:
              snapshot_db_path: "/tmp/test.db"
        "#;

        // Записываем конфигурацию в временный файл
        std::fs::write(&file_path, config_content).expect("write config");

        let initial_config: Config = serde_yaml::from_str(config_content).expect("parse config");
        
        let auto_reload = ConfigAutoReload::new(&file_path, initial_config).expect("auto reload creation");
        let receiver = auto_reload.reload_receiver();

        // Проверяем, что изначально нет уведомлений о перезагрузке
        assert!(!*receiver.borrow());
    }

    #[tokio::test]
    async fn test_config_auto_reload_start_watching() {
        let temp_file = NamedTempFile::new().expect("tempfile");
        let file_path = temp_file.path().to_str().unwrap().to_string();

        // Создаём минимальную валидную конфигурацию
        let config_content = r#"
            polling_interval_ms: 1000
            max_candidates: 10
            dry_run_default: true
            enable_snapshot_logging: false
            thresholds:
              cpu_high: 80.0
              memory_high: 80.0
            paths:
              snapshot_db_path: "/tmp/test.db"
        "#;

        // Записываем конфигурацию в временный файл
        std::fs::write(&file_path, config_content).expect("write config");

        let initial_config: Config = serde_yaml::from_str(config_content).expect("parse config");
        
        let auto_reload = ConfigAutoReload::new(&file_path, initial_config).expect("auto reload creation");
        let handle = auto_reload.start_watching();

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
    async fn test_config_auto_reload_is_reloading() {
        let temp_file = NamedTempFile::new().expect("tempfile");
        let file_path = temp_file.path().to_str().unwrap().to_string();

        // Создаём минимальную валидную конфигурацию
        let config_content = r#"
            polling_interval_ms: 1000
            max_candidates: 10
            dry_run_default: true
            enable_snapshot_logging: false
            thresholds:
              cpu_high: 80.0
              memory_high: 80.0
            paths:
              snapshot_db_path: "/tmp/test.db"
        "#;

        // Записываем конфигурацию в временный файл
        std::fs::write(&file_path, config_content).expect("write config");

        let initial_config: Config = serde_yaml::from_str(config_content).expect("parse config");
        
        let auto_reload = ConfigAutoReload::new(&file_path, initial_config).expect("auto reload creation");
        
        // Проверяем, что изначально перезагрузка не выполняется
        assert!(!auto_reload.is_reloading().await);
    }

    #[tokio::test]
    async fn test_config_reload_function() {
        let temp_file = NamedTempFile::new().expect("tempfile");
        let file_path = temp_file.path().to_str().unwrap().to_string();

        // Создаём минимальную валидную конфигурацию
        let config_content = r#"
            polling_interval_ms: 1000
            max_candidates: 10
            dry_run_default: true
            enable_snapshot_logging: false
            thresholds:
              cpu_high: 80.0
              memory_high: 80.0
            paths:
              snapshot_db_path: "/tmp/test.db"
        "#;

        // Записываем конфигурацию в временный файл
        std::fs::write(&file_path, config_content).expect("write config");

        // Пытаемся перезагрузить конфигурацию
        let result = ConfigAutoReload::reload_config(&file_path).await;
        assert!(result.is_ok());
        
        let loaded_config = result.unwrap();
        assert_eq!(loaded_config.polling_interval_ms, 1000);
        assert_eq!(loaded_config.max_candidates, 10);
    }

    #[tokio::test]
    async fn test_config_reload_invalid_file() {
        let temp_file = NamedTempFile::new().expect("tempfile");
        let file_path = temp_file.path().to_str().unwrap().to_string();

        // Записываем невалидную конфигурацию в временный файл
        let invalid_content = r#"
            polling_interval_ms: 1000
            max_candidates: 10
            this_is_invalid_field: true
        "#;

        std::fs::write(&file_path, invalid_content).expect("write invalid config");

        // Пытаемся перезагрузить конфигурацию - должно завершиться с ошибкой
        let result = ConfigAutoReload::reload_config(&file_path).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_config_reload_nonexistent_file() {
        let nonexistent_path = "/nonexistent/config.yml";

        // Пытаемся перезагрузить конфигурацию из несуществующего файла
        let result = ConfigAutoReload::reload_config(nonexistent_path).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("does not exist"));
    }
}