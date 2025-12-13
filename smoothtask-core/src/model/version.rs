//! Модуль для управления версиями ML-моделей.
//!
//! Этот модуль предоставляет функциональность для управления версиями ML-моделей,
//! включая хранение, загрузку и откат к предыдущим версиям.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};

/// Информация о версии модели.
///
/// Структура содержит метаданные о конкретной версии модели, включая
/// путь к файлу модели, версию, временную метку и дополнительные метаданные.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelVersion {
    /// Уникальный идентификатор версии (например, "v1.0.0", "v2.0.0").
    pub version_id: String,
    
    /// Путь к файлу модели для этой версии.
    pub model_path: PathBuf,
    
    /// Временная метка создания версии.
    pub timestamp: DateTime<Utc>,
    
    /// Формат модели (ONNX, CatBoost JSON и т.д.).
    pub format: String,
    
    /// Дополнительные метаданные о модели.
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    
    /// Хэш модели для проверки целостности.
    #[serde(default)]
    pub model_hash: Option<String>,
    
    /// Размер файла модели в байтах.
    #[serde(default)]
    pub file_size: Option<u64>,
}

impl ModelVersion {
    /// Создать новую версию модели.
    ///
    /// # Аргументы
    ///
    /// * `version_id` - уникальный идентификатор версии
    /// * `model_path` - путь к файлу модели
    /// * `format` - формат модели
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр ModelVersion с текущей временной меткой.
    pub fn new(version_id: impl Into<String>, model_path: impl AsRef<Path>, format: impl Into<String>) -> Self {
        let model_path = model_path.as_ref().to_path_buf();
        let timestamp = Utc::now();
        
        Self {
            version_id: version_id.into(),
            model_path,
            timestamp,
            format: format.into(),
            metadata: HashMap::new(),
            model_hash: None,
            file_size: None,
        }
    }
    
    /// Создать версию модели с дополнительными метаданными.
    ///
    /// # Аргументы
    ///
    /// * `version_id` - уникальный идентификатор версии
    /// * `model_path` - путь к файлу модели
    /// * `format` - формат модели
    /// * `metadata` - дополнительные метаданные
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр ModelVersion с текущей временной меткой и метаданными.
    pub fn with_metadata(
        version_id: impl Into<String>,
        model_path: impl AsRef<Path>,
        format: impl Into<String>,
        metadata: HashMap<String, String>,
    ) -> Self {
        let mut version = Self::new(version_id, model_path, format);
        version.metadata = metadata;
        version
    }
    
    /// Добавить метаданные к версии.
    ///
    /// # Аргументы
    ///
    /// * `key` - ключ метаданных
    /// * `value` - значение метаданных
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }
    
    /// Вычислить хэш модели (SHA256).
    ///
    /// # Возвращает
    ///
    /// Хэш модели в шестнадцатеричном формате или ошибку.
    pub fn compute_hash(&mut self) -> Result<&str> {
        use sha2::{Sha256, Digest};
        
        let model_data = std::fs::read(&self.model_path)
            .with_context(|| format!("Не удалось прочитать файл модели: {}", self.model_path.display()))?;
        
        let mut hasher = Sha256::new();
        hasher.update(&model_data);
        let hash = format!("{:x}", hasher.finalize());
        
        self.model_hash = Some(hash.clone());
        self.file_size = Some(model_data.len() as u64);
        
        Ok(self.model_hash.as_deref().unwrap())
    }
    
    /// Проверить целостность модели.
    ///
    /// # Возвращает
    ///
    /// `true`, если модель существует и имеет валидный размер, `false` в противном случае.
    pub fn validate(&self) -> bool {
        self.model_path.exists() && 
        self.model_path.is_file() &&
        self.model_path.metadata().map_or(false, |m| m.len() > 0)
    }
    
    /// Получить информацию о модели в читаемом формате.
    ///
    /// # Возвращает
    ///
    /// Строка с информацией о версии модели.
    pub fn info_string(&self) -> String {
        let mut info = format!(
            "ModelVersion {{ version_id: {}, path: {}, format: {}, timestamp: {} ",
            self.version_id,
            self.model_path.display(),
            self.format,
            self.timestamp.format("%Y-%m-%d %H:%M:%S")
        );
        
        if let Some(hash) = &self.model_hash {
            info.push_str(&format!(", hash: {} ", hash));
        }
        
        if let Some(size) = self.file_size {
            info.push_str(&format!(", size: {} bytes ", size));
        }
        
        if !self.metadata.is_empty() {
            info.push_str(", metadata: {");
            for (i, (key, value)) in self.metadata.iter().enumerate() {
                if i > 0 {
                    info.push_str(", ");
                }
                info.push_str(&format!("{}: {}", key, value));
            }
            info.push('}');
        }
        
        info.push_str("} ");
        info
    }
}

/// Менеджер версий моделей.
///
/// Управляет несколькими версиями моделей, предоставляя функциональность
/// для добавления, удаления и отката к предыдущим версиям.
#[derive(Debug, Default)]
pub struct ModelVersionManager {
    /// Список доступных версий моделей.
    versions: Vec<ModelVersion>,
    
    /// Текущая активная версия.
    current_version: Option<String>,
}

impl ModelVersionManager {
    /// Создать новый менеджер версий моделей.
    pub fn new() -> Self {
        Self {
            versions: Vec::new(),
            current_version: None,
        }
    }
    
    /// Добавить новую версию модели.
    ///
    /// # Аргументы
    ///
    /// * `version` - версия модели для добавления
    ///
    /// # Возвращает
    ///
    /// `true`, если версия была добавлена, `false`, если такая версия уже существует.
    pub fn add_version(&mut self, version: ModelVersion) -> bool {
        if self.versions.iter().any(|v| v.version_id == version.version_id) {
            return false;
        }
        
        // Если это первая версия, делаем её текущей
        if self.versions.is_empty() {
            self.current_version = Some(version.version_id.clone());
        }
        
        self.versions.push(version);
        true
    }
    
    /// Установить текущую активную версию.
    ///
    /// # Аргументы
    ///
    /// * `version_id` - идентификатор версии для активации
    ///
    /// # Возвращает
    ///
    /// `true`, если версия была успешно активирована, `false`, если версия не найдена.
    pub fn set_current_version(&mut self, version_id: impl Into<String>) -> bool {
        let version_id = version_id.into();
        if self.versions.iter().any(|v| v.version_id == version_id) {
            self.current_version = Some(version_id);
            true
        } else {
            false
        }
    }
    
    /// Получить текущую активную версию.
    ///
    /// # Возвращает
    ///
    /// Опциональная ссылка на текущую версию или `None`, если версия не установлена.
    pub fn get_current_version(&self) -> Option<&ModelVersion> {
        self.current_version
            .as_ref()
            .and_then(|version_id| self.versions.iter().find(|v| &v.version_id == version_id))
    }
    
    /// Получить версию по идентификатору.
    ///
    /// # Аргументы
    ///
    /// * `version_id` - идентификатор версии
    ///
    /// # Возвращает
    ///
    /// Опциональная ссылка на версию или `None`, если версия не найдена.
    pub fn get_version(&self, version_id: impl AsRef<str>) -> Option<&ModelVersion> {
        self.versions.iter().find(|v| v.version_id == version_id.as_ref())
    }
    
    /// Получить все доступные версии.
    ///
    /// # Возвращает
    ///
    /// Ссылка на список всех версий.
    pub fn get_all_versions(&self) -> &[ModelVersion] {
        &self.versions
    }
    
    /// Удалить версию модели.
    ///
    /// # Аргументы
    ///
    /// * `version_id` - идентификатор версии для удаления
    ///
    /// # Возвращает
    ///
    /// `true`, если версия была удалена, `false`, если версия не найдена.
    ///
    /// # Примечания
    ///
    /// - Нельзя удалить текущую активную версию
    /// - Файл модели не удаляется с диска
    pub fn remove_version(&mut self, version_id: impl AsRef<str>) -> bool {
        let version_id = version_id.as_ref();
        
        // Нельзя удалить текущую версию
        if self.current_version.as_deref() == Some(version_id) {
            return false;
        }
        
        let initial_len = self.versions.len();
        self.versions.retain(|v| v.version_id != version_id);
        initial_len != self.versions.len()
    }
    
    /// Откатить к предыдущей версии.
    ///
    /// # Возвращает
    ///
    /// `true`, если откат был успешным, `false`, если нет предыдущей версии.
    pub fn rollback(&mut self) -> bool {
        if self.versions.len() < 2 {
            return false;
        }
        
        // Найти текущую версию
        let current_index = self.current_version
            .as_ref()
            .and_then(|current_id| self.versions.iter().position(|v| &v.version_id == current_id));
        
        match current_index {
            Some(index) => {
                // Откатываемся к предыдущей версии
                let previous_index = if index > 0 { index - 1 } else { self.versions.len() - 1 };
                self.current_version = Some(self.versions[previous_index].version_id.clone());
                true
            }
            None => {
                // Если текущая версия не установлена, устанавливаем последнюю версию
                self.current_version = Some(self.versions.last().unwrap().version_id.clone());
                true
            }
        }
    }
    
    /// Получить количество доступных версий.
    ///
    /// # Возвращает
    ///
    /// Количество версий.
    pub fn version_count(&self) -> usize {
        self.versions.len()
    }
    
    /// Проверить, существует ли версия.
    ///
    /// # Аргументы
    ///
    /// * `version_id` - идентификатор версии
    ///
    /// # Возвращает
    ///
    /// `true`, если версия существует, `false` в противном случае.
    pub fn has_version(&self, version_id: impl AsRef<str>) -> bool {
        self.versions.iter().any(|v| v.version_id == version_id.as_ref())
    }
    
    /// Получить информацию о всех версиях в читаемом формате.
    ///
    /// # Возвращает
    ///
    /// Строка с информацией о всех версиях.
    pub fn versions_info(&self) -> String {
        let mut info = String::new();
        
        for (i, version) in self.versions.iter().enumerate() {
            let marker = if Some(&version.version_id) == self.current_version.as_ref() {
                "[CURRENT] "
            } else {
                "         "
            };
            
            info.push_str(&format!("{}Version {}: {}", marker, i + 1, version.info_string()));
            
            if i < self.versions.len() - 1 {
                info.push('\n');
            }
        }
        
        info
    }
}

/// Утилиты для работы с версиями моделей.
pub mod utils {
    use super::*;
    use std::fs;
    
    /// Загрузить версии моделей из директории.
    ///
    /// # Аргументы
    ///
    /// * `models_dir` - директория с моделями
    /// * `pattern` - шаблон для поиска файлов моделей
    ///
    /// # Возвращает
    ///
    /// Вектор ModelVersion или ошибку.
    pub fn load_versions_from_directory(
        models_dir: impl AsRef<Path>,
        pattern: impl AsRef<str>,
    ) -> Result<Vec<ModelVersion>> {
        let models_dir = models_dir.as_ref();
        let pattern = pattern.as_ref();
        
        if !models_dir.exists() {
            return Ok(Vec::new());
        }
        
        let mut versions = Vec::new();
        
        for entry in fs::read_dir(models_dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_file() {
                let file_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                if file_name.contains(pattern) {
                    // Извлекаем версию из имени файла (например, "model_v1.0.0.onnx" -> "v1.0.0")
                    let version_id = if let Some(version_part) = file_name.split(pattern).nth(1) {
                        version_part.to_string()
                    } else {
                        file_name.to_string()
                    };
                    
                    let format = path.extension()
                        .and_then(|ext| ext.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    
                    let path_clone = path.clone();
                    let mut version = ModelVersion::new(version_id, path_clone, format);
                    
                    // Вычисляем хэш и размер
                    if let Err(e) = version.compute_hash() {
                        tracing::warn!("Не удалось вычислить хэш для модели {}: {}", file_name, e);
                    }
                    
                    versions.push(version);
                }
            }
        }
        
        // Сортируем версии по времени (новые версии в конце)
        versions.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        
        Ok(versions)
    }
    
    /// Сохранить версии моделей в файл.
    ///
    /// # Аргументы
    ///
    /// * `versions` - список версий для сохранения
    /// * `output_path` - путь к файлу для сохранения
    ///
    /// # Возвращает
    ///
    /// Результат операции.
    pub fn save_versions_to_file(
        versions: &[ModelVersion],
        output_path: impl AsRef<Path>,
    ) -> Result<()> {
        let output_path = output_path.as_ref();
        let json_data = serde_json::to_string_pretty(versions)
            .context("Не удалось сериализовать версии моделей")?;
        
        fs::write(output_path, json_data)
            .with_context(|| format!("Не удалось сохранить версии в файл: {}", output_path.display()))
    }
    
    /// Загрузить версии моделей из файла.
    ///
    /// # Аргументы
    ///
    /// * `input_path` - путь к файлу с версиями
    ///
    /// # Возвращает
    ///
    /// Вектор ModelVersion или ошибку.
    pub fn load_versions_from_file(
        input_path: impl AsRef<Path>,
    ) -> Result<Vec<ModelVersion>> {
        let input_path = input_path.as_ref();
        let json_data = fs::read_to_string(input_path)
            .with_context(|| format!("Не удалось прочитать файл версий: {}", input_path.display()))?;
        
        serde_json::from_str(&json_data)
            .context("Не удалось десериализовать версии моделей")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_model_version_creation() {
        let version = ModelVersion::new("v1.0.0", "models/test.onnx", "onnx");
        
        assert_eq!(version.version_id, "v1.0.0");
        assert_eq!(version.model_path, PathBuf::from("models/test.onnx"));
        assert_eq!(version.format, "onnx");
        assert!(version.timestamp <= Utc::now());
    }
    
    #[test]
    fn test_model_version_with_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("accuracy".to_string(), "0.95".to_string());
        metadata.insert("dataset".to_string(), "training_v1".to_string());
        
        let version = ModelVersion::with_metadata("v1.0.0", "models/test.onnx", "onnx", metadata);
        
        assert_eq!(version.metadata.get("accuracy"), Some(&"0.95".to_string()));
        assert_eq!(version.metadata.get("dataset"), Some(&"training_v1".to_string()));
    }
    
    #[test]
    fn test_model_version_add_metadata() {
        let mut version = ModelVersion::new("v1.0.0", "models/test.onnx", "onnx");
        version.add_metadata("accuracy", "0.95");
        version.add_metadata("dataset", "training_v1");
        
        assert_eq!(version.metadata.get("accuracy"), Some(&"0.95".to_string()));
        assert_eq!(version.metadata.get("dataset"), Some(&"training_v1".to_string()));
    }
    
    #[test]
    fn test_model_version_info_string() {
        let mut version = ModelVersion::new("v1.0.0", "models/test.onnx", "onnx");
        version.add_metadata("accuracy", "0.95");
        
        let info = version.info_string();
        
        assert!(info.contains("v1.0.0"));
        assert!(info.contains("models/test.onnx"));
        assert!(info.contains("onnx"));
        assert!(info.contains("accuracy: 0.95"));
    }
    
    #[test]
    fn test_version_manager_add_version() {
        let mut manager = ModelVersionManager::new();
        let version = ModelVersion::new("v1.0.0", "models/test.onnx", "onnx");
        
        assert!(manager.add_version(version.clone()));
        assert_eq!(manager.version_count(), 1);
        assert!(manager.has_version("v1.0.0"));
        
        // Проверяем, что текущая версия установлена
        assert!(manager.get_current_version().is_some());
        assert_eq!(manager.get_current_version().unwrap().version_id, "v1.0.0");
    }
    
    #[test]
    fn test_version_manager_duplicate_version() {
        let mut manager = ModelVersionManager::new();
        let version1 = ModelVersion::new("v1.0.0", "models/test1.onnx", "onnx");
        let version2 = ModelVersion::new("v1.0.0", "models/test2.onnx", "onnx");
        
        assert!(manager.add_version(version1));
        assert!(!manager.add_version(version2));
        assert_eq!(manager.version_count(), 1);
    }
    
    #[test]
    fn test_version_manager_set_current_version() {
        let mut manager = ModelVersionManager::new();
        let version1 = ModelVersion::new("v1.0.0", "models/test1.onnx", "onnx");
        let version2 = ModelVersion::new("v2.0.0", "models/test2.onnx", "onnx");
        
        manager.add_version(version1);
        manager.add_version(version2);
        
        assert!(manager.set_current_version("v2.0.0"));
        assert_eq!(manager.get_current_version().unwrap().version_id, "v2.0.0");
        
        assert!(!manager.set_current_version("v3.0.0")); // Несуществующая версия
    }
    
    #[test]
    fn test_version_manager_rollback() {
        let mut manager = ModelVersionManager::new();
        let version1 = ModelVersion::new("v1.0.0", "models/test1.onnx", "onnx");
        let version2 = ModelVersion::new("v2.0.0", "models/test2.onnx", "onnx");
        
        manager.add_version(version1);
        manager.add_version(version2);
        manager.set_current_version("v2.0.0");
        
        assert!(manager.rollback());
        assert_eq!(manager.get_current_version().unwrap().version_id, "v1.0.0");
        
        // Проверяем, что откат работает циклически
        assert!(manager.rollback());
        assert_eq!(manager.get_current_version().unwrap().version_id, "v2.0.0");
    }
    
    #[test]
    fn test_version_manager_remove_version() {
        let mut manager = ModelVersionManager::new();
        let version1 = ModelVersion::new("v1.0.0", "models/test1.onnx", "onnx");
        let version2 = ModelVersion::new("v2.0.0", "models/test2.onnx", "onnx");
        
        manager.add_version(version1);
        manager.add_version(version2);
        manager.set_current_version("v1.0.0");
        
        // Нельзя удалить текущую версию
        assert!(!manager.remove_version("v1.0.0"));
        
        // Можно удалить не текущую версию
        assert!(manager.remove_version("v2.0.0"));
        assert_eq!(manager.version_count(), 1);
        assert!(!manager.has_version("v2.0.0"));
    }
    
    #[test]
    fn test_version_manager_versions_info() {
        let mut manager = ModelVersionManager::new();
        let version1 = ModelVersion::new("v1.0.0", "models/test1.onnx", "onnx");
        let version2 = ModelVersion::new("v2.0.0", "models/test2.onnx", "onnx");
        
        manager.add_version(version1);
        manager.add_version(version2);
        manager.set_current_version("v2.0.0");
        
        let info = manager.versions_info();
        
        assert!(info.contains("v1.0.0"));
        assert!(info.contains("v2.0.0"));
        assert!(info.contains("[CURRENT]"));
    }
    
    #[test]
    fn test_utils_load_versions_from_directory() {
        let temp_dir = tempdir().unwrap();
        let dir_path = temp_dir.path();
        
        // Создаём тестовые файлы моделей
        let model_file1 = dir_path.join("model_v1.0.0.onnx");
        let model_file2 = dir_path.join("model_v2.0.0.onnx");
        let other_file = dir_path.join("other_file.txt");
        
        std::fs::write(&model_file1, "dummy content 1").unwrap();
        std::fs::write(&model_file2, "dummy content 2").unwrap();
        std::fs::write(&other_file, "other content").unwrap();
        
        // Загружаем версии
        let versions = utils::load_versions_from_directory(dir_path, "model_").unwrap();
        
        assert_eq!(versions.len(), 2);
        assert!(versions.iter().any(|v| v.version_id == "v1.0.0"));
        assert!(versions.iter().any(|v| v.version_id == "v2.0.0"));
        
        // Проверяем, что хэши вычислены
        for version in versions {
            assert!(version.model_hash.is_some());
            assert!(version.file_size.is_some());
        }
    }
    
    #[test]
    fn test_utils_save_and_load_versions() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("versions.json");
        
        let mut version = ModelVersion::new("v1.0.0", "models/test.onnx", "onnx");
        version.add_metadata("accuracy", "0.95");
        
        let versions = vec![version];
        
        // Сохраняем версии
        utils::save_versions_to_file(&versions, &file_path).unwrap();
        
        // Загружаем версии
        let loaded_versions = utils::load_versions_from_file(&file_path).unwrap();
        
        assert_eq!(loaded_versions.len(), 1);
        assert_eq!(loaded_versions[0].version_id, "v1.0.0");
        assert_eq!(loaded_versions[0].metadata.get("accuracy"), Some(&"0.95".to_string()));
    }
    
    #[test]
    fn test_model_version_validate() {
        let temp_dir = tempdir().unwrap();
        let model_file = temp_dir.path().join("test.onnx");
        
        // Создаём валидный файл
        std::fs::write(&model_file, "dummy content").unwrap();
        
        let version = ModelVersion::new("v1.0.0", model_file, "onnx");
        assert!(version.validate());
        
        // Проверяем невалидный файл
        let invalid_version = ModelVersion::new("v1.0.0", "nonexistent.onnx", "onnx");
        assert!(!invalid_version.validate());
    }
}