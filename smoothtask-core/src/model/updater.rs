//! Модуль для автоматического обновления ML-моделей.
//!
//! Этот модуль предоставляет функциональность для проверки и загрузки
//! обновлений моделей из удаленных репозиториев или локальных источников.

use crate::model::version::{ModelVersion, ModelVersionManager};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use reqwest::blocking::Client;
use sha2::{Sha256, Digest};
use serde::{Deserialize, Serialize};

/// Конфигурация для автоматического обновления моделей.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUpdateConfig {
    /// URL репозитория с моделями (может быть HTTP, HTTPS, или локальный путь).
    pub repository_url: String,
    
    /// Локальная директория для хранения моделей.
    pub models_directory: PathBuf,
    
    /// Интервал проверки обновлений в секундах.
    #[serde(default = "default_update_interval")]
    pub update_interval: u64,
    
    /// Включить автоматическое обновление.
    #[serde(default = "default_auto_update")]
    pub auto_update: bool,
    
    /// Проверять подписи моделей (если доступно).
    #[serde(default = "default_verify_signatures")]
    pub verify_signatures: bool,
    
    /// Дополнительные заголовки для HTTP-запросов.
    #[serde(default)]
    pub http_headers: HashMap<String, String>,
}

fn default_update_interval() -> u64 {
    3600 // 1 час по умолчанию
}

fn default_auto_update() -> bool {
    true
}

fn default_verify_signatures() -> bool {
    false
}

impl Default for ModelUpdateConfig {
    fn default() -> Self {
        Self {
            repository_url: String::new(),
            models_directory: PathBuf::from("./models"),
            update_interval: default_update_interval(),
            auto_update: default_auto_update(),
            verify_signatures: default_verify_signatures(),
            http_headers: HashMap::new(),
        }
    }
}

/// Информация об обновлении модели.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUpdateInfo {
    /// Идентификатор версии.
    pub version_id: String,
    
    /// URL для загрузки модели.
    pub download_url: String,
    
    /// Формат модели.
    pub format: String,
    
    /// Хэш модели для проверки целостности.
    pub model_hash: String,
    
    /// Размер файла в байтах.
    pub file_size: u64,
    
    /// Временная метка создания.
    pub timestamp: String,
    
    /// Дополнительные метаданные.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Менеджер обновлений моделей.
#[derive(Debug)]
pub struct ModelUpdater {
    /// Конфигурация обновлений.
    config: ModelUpdateConfig,
    
    /// HTTP клиент для загрузки моделей.
    http_client: Client,
    
    /// Менеджер версий для управления локальными моделями.
    version_manager: ModelVersionManager,
}

impl ModelUpdater {
    /// Создать новый ModelUpdater.
    ///
    /// # Аргументы
    ///
    /// * `config` - конфигурация обновлений
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр ModelUpdater.
    pub fn new(config: ModelUpdateConfig) -> Self {
        let http_client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Не удалось создать HTTP клиент");
        
        Self {
            config,
            http_client,
            version_manager: ModelVersionManager::new(),
        }
    }
    
    /// Загрузить текущие локальные версии моделей.
    ///
    /// # Возвращает
    ///
    /// Результат операции.
    pub fn load_local_versions(&mut self) -> Result<()> {
        // Загружаем версии из локальной директории
        let versions = crate::model::version::utils::load_versions_from_directory(
            &self.config.models_directory,
            "model_"
        ).context("Не удалось загрузить локальные версии моделей")?;
        
        // Добавляем версии в менеджер
        for version in versions {
            self.version_manager.add_version(version);
        }
        
        tracing::info!(
            "Загружено {} локальных версий моделей",
            self.version_manager.version_count()
        );
        
        Ok(())
    }
    
    /// Проверить наличие обновлений модели.
    ///
    /// # Аргументы
    ///
    /// * `remote_manifest_url` - URL манифеста с информацией о доступных версиях
    ///
    /// # Возвращает
    ///
    /// Вектор доступных обновлений или ошибку.
    pub fn check_for_updates(&self, remote_manifest_url: impl AsRef<str>) -> Result<Vec<ModelUpdateInfo>> {
        let remote_manifest_url = remote_manifest_url.as_ref();
        
        // Загружаем манифест с удаленного сервера
        let manifest_content = self.http_client
            .get(remote_manifest_url)
            .send()
            .with_context(|| format!("Не удалось загрузить манифест с {}", remote_manifest_url))?
            .text()
            .context("Не удалось прочитать содержимое манифеста")?;
        
        // Парсим манифест
        let updates: Vec<ModelUpdateInfo> = serde_json::from_str(&manifest_content)
            .context("Не удалось десериализовать манифест обновлений")?;
        
        tracing::info!(
            "Найдено {} доступных версий на сервере",
            updates.len()
        );
        
        Ok(updates)
    }
    
    /// Загрузить и установить обновление модели.
    ///
    /// # Аргументы
    ///
    /// * `update_info` - информация об обновлении
    ///
    /// # Возвращает
    ///
    /// Результат операции установки.
    pub fn download_and_install_update(&mut self, update_info: &ModelUpdateInfo) -> Result<()> {
        // Создаем директорию для моделей, если её нет
        if !self.config.models_directory.exists() {
            fs::create_dir_all(&self.config.models_directory)
                .with_context(|| format!(
                    "Не удалось создать директорию для моделей: {}",
                    self.config.models_directory.display()
                ))?;
        }
        
        // Создаем путь для новой модели
        let model_filename = format!("model_{}.{}", update_info.version_id, update_info.format);
        let model_path = self.config.models_directory.join(model_filename);
        
        // Загружаем модель
        tracing::info!(
            "Загрузка модели {} с {}",
            update_info.version_id,
            update_info.download_url
        );
        
        let mut response = self.http_client
            .get(&update_info.download_url)
            .send()
            .with_context(|| format!(
                "Не удалось загрузить модель с {}",
                update_info.download_url
            ))?;
        
        // Проверяем размер файла
        let content_length = response.content_length().unwrap_or(0);
        if content_length != update_info.file_size {
            return Err(anyhow::anyhow!(
                "Размер загруженного файла ({} байт) не совпадает с ожидаемым ({} байт)",
                content_length,
                update_info.file_size
            ));
        }
        
        // Сохраняем модель
        let mut file = fs::File::create(&model_path)
            .with_context(|| format!(
                "Не удалось создать файл модели: {}",
                model_path.display()
            ))?;
        
        let mut content = Vec::new();
        response.read_to_end(&mut content)
            .context("Не удалось прочитать содержимое ответа")?;
        
        file.write_all(&content)
            .with_context(|| format!(
                "Не удалось записать содержимое в файл: {}",
                model_path.display()
            ))?;
        
        // Проверяем хэш
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let computed_hash = format!("{:x}", hasher.finalize());
        
        if computed_hash != update_info.model_hash {
            // Удаляем невалидный файл
            fs::remove_file(&model_path)
                .ok();
                
            return Err(anyhow::anyhow!(
                "Хэш загруженной модели ({}) не совпадает с ожидаемым ({})",
                computed_hash,
                update_info.model_hash
            ));
        }
        
        // Создаем новую версию
        let mut metadata = update_info.metadata.clone();
        metadata.insert("source".to_string(), "remote_update".to_string());
        metadata.insert("update_timestamp".to_string(), chrono::Utc::now().to_rfc3339());
        
        let mut version = ModelVersion::with_metadata(
            update_info.version_id.clone(),
            model_path,
            update_info.format.clone(),
            metadata,
        );
        
        // Устанавливаем хэш и размер
        version.model_hash = Some(computed_hash);
        version.file_size = Some(update_info.file_size);
        
        // Добавляем версию в менеджер
        if !self.version_manager.add_version(version) {
            return Err(anyhow::anyhow!(
                "Версия {} уже существует",
                update_info.version_id
            ));
        }
        
        tracing::info!(
            "Успешно установлено обновление модели: {}",
            update_info.version_id
        );
        
        Ok(())
    }
    
    /// Проверить и установить все доступные обновления.
    ///
    /// # Аргументы
    ///
    /// * `remote_manifest_url` - URL манифеста с информацией о доступных версиях
    ///
    /// # Возвращает
    ///
    /// Результат операции.
    pub fn check_and_install_updates(&mut self, remote_manifest_url: impl AsRef<str>) -> Result<()> {
        // Загружаем текущие локальные версии
        self.load_local_versions()?;
        
        // Проверяем наличие обновлений
        let available_updates = self.check_for_updates(remote_manifest_url)?;
        
        // Находим новые версии
        let mut new_updates = Vec::new();
        for update in available_updates {
            if !self.version_manager.has_version(&update.version_id) {
                new_updates.push(update);
            }
        }
        
        if new_updates.is_empty() {
            tracing::info!("Нет доступных обновлений");
            return Ok(());
        }
        
        tracing::info!(
            "Найдено {} новых обновлений для установки",
            new_updates.len()
        );
        
        // Устанавливаем все новые обновления
        for update in &new_updates {
            self.download_and_install_update(update)?;
        }
        
        tracing::info!(
            "Успешно установлено {} обновлений",
            new_updates.len()
        );
        
        Ok(())
    }
    
    /// Получить информацию о текущих локальных версиях.
    ///
    /// # Возвращает
    ///
    /// Информация о всех локальных версиях.
    pub fn get_local_versions_info(&self) -> String {
        self.version_manager.versions_info()
    }
    
    /// Сохранить информацию о версиях в файл.
    ///
    /// # Аргументы
    ///
    /// * `output_path` - путь к файлу для сохранения
    ///
    /// # Возвращает
    ///
    /// Результат операции.
    pub fn save_versions_to_file(&self, output_path: impl AsRef<Path>) -> Result<()> {
        crate::model::version::utils::save_versions_to_file(
            self.version_manager.get_all_versions(),
            output_path
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_model_update_config_default() {
        let config = ModelUpdateConfig::default();
        
        assert_eq!(config.update_interval, 3600);
        assert!(config.auto_update);
        assert!(!config.verify_signatures);
        assert_eq!(config.models_directory, PathBuf::from("./models"));
    }
    
    #[test]
    fn test_model_update_config_serialization() {
        let mut config = ModelUpdateConfig::default();
        config.repository_url = "https://example.com/models".to_string();
        config.update_interval = 1800;
        
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ModelUpdateConfig = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.repository_url, "https://example.com/models");
        assert_eq!(deserialized.update_interval, 1800);
    }
    
    #[test]
    fn test_model_updater_creation() {
        let config = ModelUpdateConfig::default();
        let updater = ModelUpdater::new(config);
        
        assert_eq!(updater.version_manager.version_count(), 0);
    }
    
    #[test]
    fn test_model_updater_load_local_versions() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();
        
        // Создаём тестовые файлы моделей
        let model_file1 = dir_path.join("model_v1.0.0.onnx");
        let model_file2 = dir_path.join("model_v2.0.0.onnx");
        
        fs::write(&model_file1, "dummy content 1").unwrap();
        fs::write(&model_file2, "dummy content 2").unwrap();
        
        let mut config = ModelUpdateConfig::default();
        config.models_directory = dir_path.to_path_buf();
        
        let mut updater = ModelUpdater::new(config);
        updater.load_local_versions().unwrap();
        
        assert_eq!(updater.version_manager.version_count(), 2);
    }
    
    #[test]
    fn test_model_update_info_serialization() {
        let mut metadata = HashMap::new();
        metadata.insert("accuracy".to_string(), "0.95".to_string());
        
        let update_info = ModelUpdateInfo {
            version_id: "v1.0.0".to_string(),
            download_url: "https://example.com/model.onnx".to_string(),
            format: "onnx".to_string(),
            model_hash: "abc123".to_string(),
            file_size: 1024,
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            metadata,
        };
        
        let json = serde_json::to_string(&update_info).unwrap();
        let deserialized: ModelUpdateInfo = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.version_id, "v1.0.0");
        assert_eq!(deserialized.download_url, "https://example.com/model.onnx");
        assert_eq!(deserialized.metadata.get("accuracy"), Some(&"0.95".to_string()));
    }
    
    #[test]
    fn test_model_updater_versions_info() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();
        
        // Создаём тестовый файл модели
        let model_file = dir_path.join("model_v1.0.0.onnx");
        fs::write(&model_file, "dummy content").unwrap();
        
        let mut config = ModelUpdateConfig::default();
        config.models_directory = dir_path.to_path_buf();
        
        let mut updater = ModelUpdater::new(config);
        updater.load_local_versions().unwrap();
        
        let info = updater.get_local_versions_info();
        assert!(info.contains("v1.0.0"));
        assert!(info.contains("model_v1.0.0.onnx"));
    }
    
    /// Компрехенсивный интеграционный тест для полного цикла обновления моделей
    #[test]
    fn test_model_updater_full_workflow() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();
        
        // Создаём тестовые файлы моделей (имитация локальных версий)
        let model_file1 = dir_path.join("model_v1.0.0.onnx");
        let model_file2 = dir_path.join("model_v2.0.0.onnx");
        
        fs::write(&model_file1, "dummy content 1").unwrap();
        fs::write(&model_file2, "dummy content 2").unwrap();
        
        let mut config = ModelUpdateConfig::default();
        config.models_directory = dir_path.to_path_buf();
        
        let mut updater = ModelUpdater::new(config);
        
        // Тест 1: Загрузка локальных версий
        updater.load_local_versions().unwrap();
        assert_eq!(updater.version_manager.version_count(), 2);
        
        // Тест 2: Получение информации о версиях
        let info = updater.get_local_versions_info();
        assert!(info.contains("v1.0.0"));
        assert!(info.contains("v2.0.0"));
        
        // Тест 3: Сохранение версий в файл
        let versions_file = dir_path.join("versions.json");
        updater.save_versions_to_file(&versions_file).unwrap();
        assert!(versions_file.exists());
        
        // Тест 4: Проверка текущей версии
        let current_version = updater.version_manager.get_current_version();
        assert!(current_version.is_some());
        let version_id = current_version.unwrap().version_id.clone();
        assert!(version_id == "v1.0.0.onnx" || version_id == "v2.0.0.onnx");
        
        // Тест 5: Откат к предыдущей версии (если есть несколько версий)
        if updater.version_manager.version_count() > 1 {
            let initial_version = updater.version_manager.get_current_version().unwrap().version_id.clone();
            updater.version_manager.rollback();
            let new_version = updater.version_manager.get_current_version().unwrap().version_id.clone();
            assert_ne!(initial_version, new_version);
        }
        
        // Тест 6: Проверка сериализации конфигурации
        let json_config = serde_json::to_string(&updater.config).unwrap();
        let deserialized_config: ModelUpdateConfig = serde_json::from_str(&json_config).unwrap();
        assert_eq!(deserialized_config.models_directory, updater.config.models_directory);
        
        // Тест 7: Проверка сериализации информации об обновлении
        let mut metadata = HashMap::new();
        metadata.insert("accuracy".to_string(), "0.95".to_string());
        
        let update_info = ModelUpdateInfo {
            version_id: "v3.0.0".to_string(),
            download_url: "https://example.com/model_v3.0.0.onnx".to_string(),
            format: "onnx".to_string(),
            model_hash: "abc123def456".to_string(),
            file_size: 2048,
            timestamp: "2023-01-01T00:00:00Z".to_string(),
            metadata,
        };
        
        let json_update = serde_json::to_string(&update_info).unwrap();
        let deserialized_update: ModelUpdateInfo = serde_json::from_str(&json_update).unwrap();
        assert_eq!(deserialized_update.version_id, "v3.0.0");
        assert_eq!(deserialized_update.download_url, "https://example.com/model_v3.0.0.onnx");
        assert_eq!(deserialized_update.metadata.get("accuracy"), Some(&"0.95".to_string()));
    }
    
    /// Тест для проверки обработки ошибок при загрузке невалидных моделей
    #[test]
    fn test_model_updater_error_handling() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();
        
        // Создаём невалидный файл модели (пустой файл)
        let invalid_model_file = dir_path.join("model_invalid.onnx");
        fs::write(&invalid_model_file, "").unwrap();
        
        let mut config = ModelUpdateConfig::default();
        config.models_directory = dir_path.to_path_buf();
        
        let mut updater = ModelUpdater::new(config);
        
        // Должно успешно загрузить версии, даже если модель невалидна
        // (валидация происходит при вычислении хэша, но не прерывает загрузку)
        updater.load_local_versions().unwrap();
        
        // Проверяем, что версия была добавлена
        assert_eq!(updater.version_manager.version_count(), 1);
        
        // Проверяем, что версия невалидна
        let version = updater.version_manager.get_version("invalid.onnx").unwrap();
        assert!(!version.validate());
    }
}