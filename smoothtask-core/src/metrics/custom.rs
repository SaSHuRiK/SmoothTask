//! Модуль для пользовательских метрик и мониторинга.
//!
//! Этот модуль предоставляет функциональность для определения и сбора
//! пользовательских метрик, которые могут быть настроены пользователем
//! через конфигурацию или API.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

/// Тип пользовательской метрики.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CustomMetricType {
    /// Числовая метрика (целое число)
    #[serde(rename = "integer")]
    Integer,
    /// Числовая метрика с плавающей точкой
    #[serde(rename = "float")]
    Float,
    /// Булева метрика
    #[serde(rename = "boolean")]
    Boolean,
    /// Строковая метрика
    #[serde(rename = "string")]
    String,
}

/// Значение пользовательской метрики.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum CustomMetricValue {
    /// Целочисленное значение
    Integer(i64),
    /// Значение с плавающей точкой
    Float(f64),
    /// Булево значение
    Boolean(bool),
    /// Строковое значение
    String(String),
}

/// Конфигурация пользовательской метрики.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMetricConfig {
    /// Уникальный идентификатор метрики
    pub id: String,
    /// Отображаемое имя метрики
    pub name: String,
    /// Описание метрики
    pub description: String,
    /// Тип метрики
    pub metric_type: CustomMetricType,
    /// Источник данных для метрики
    pub source: CustomMetricSource,
    /// Интервал обновления в секундах
    pub update_interval_sec: u64,
    /// Включена ли метрика
    pub enabled: bool,
}

/// Источник данных для пользовательской метрики.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CustomMetricSource {
    /// Метрика из файла
    File {
        /// Путь к файлу
        path: PathBuf,
        /// Формат данных в файле
        format: FileMetricFormat,
    },
    /// Метрика из команды
    Command {
        /// Команда для выполнения
        command: String,
        /// Аргументы команды
        args: Vec<String>,
        /// Формат вывода команды
        format: CommandMetricFormat,
    },
    /// Метрика из HTTP API
    Http {
        /// URL для запроса
        url: String,
        /// Метод HTTP
        method: String,
        /// Заголовки
        headers: HashMap<String, String>,
        /// Тело запроса
        body: Option<String>,
        /// Путь к значению в JSON ответе
        json_path: String,
    },
    /// Статическая метрика (задаётся вручную)
    Static {
        /// Статическое значение
        value: CustomMetricValue,
    },
}

/// Формат данных в файле для метрики.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "snake_case")]
pub enum FileMetricFormat {
    /// Простое числовое значение
    PlainNumber,
    /// JSON файл с указанным путем к значению
    Json { path: String },
    /// Текстовый файл, где значение извлекается по регулярному выражению
    Regex { pattern: String },
}

/// Формат вывода команды для метрики.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "snake_case")]
pub enum CommandMetricFormat {
    /// Простое числовое значение
    PlainNumber,
    /// JSON вывод с указанным путем к значению
    Json { path: String },
    /// Текстовый вывод, где значение извлекается по регулярному выражению
    Regex { pattern: String },
}

/// Текущее значение пользовательской метрики.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMetricValueWithTimestamp {
    /// Значение метрики
    pub value: CustomMetricValue,
    /// Временная метка
    pub timestamp: u64,
    /// Статус метрики
    pub status: MetricStatus,
}

/// Статус метрики.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetricStatus {
    /// Метрика успешно обновлена
    #[serde(rename = "ok")]
    Ok,
    /// Ошибка при обновлении метрики
    #[serde(rename = "error")]
    Error { message: String },
    /// Метрика отключена
    #[serde(rename = "disabled")]
    Disabled,
}

/// Менеджер пользовательских метрик.
#[derive(Debug)]
pub struct CustomMetricsManager {
    /// Конфигурации метрик
    metrics_config: Arc<RwLock<HashMap<String, CustomMetricConfig>>>,
    /// Текущие значения метрик
    metrics_values: Arc<RwLock<HashMap<String, CustomMetricValueWithTimestamp>>>,
    /// Флаг работы менеджера
    running: Arc<RwLock<bool>>,
}

impl CustomMetricsManager {
    /// Создаёт новый менеджер пользовательских метрик.
    pub fn new() -> Self {
        Self {
            metrics_config: Arc::new(RwLock::new(HashMap::new())),
            metrics_values: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Добавляет новую пользовательскую метрику.
    pub fn add_metric(&self, config: CustomMetricConfig) -> Result<()> {
        let mut configs = self.metrics_config.write().map_err(|e| {
            anyhow::anyhow!("Не удалось заблокировать конфигурации метрик: {}", e)
        })?;

        if configs.contains_key(&config.id) {
            return Err(anyhow::anyhow!(
                "Метрика с идентификатором '{}' уже существует",
                config.id
            ));
        }

        configs.insert(config.id.clone(), config);
        
        // Инициализируем значение метрики
        let mut values = self.metrics_values.write().map_err(|e| {
            anyhow::anyhow!("Не удалось заблокировать значения метрик: {}", e)
        })?;

        values.insert(
            config.id,
            CustomMetricValueWithTimestamp {
                value: CustomMetricValue::String("N/A".to_string()),
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
                status: MetricStatus::Ok,
            },
        );

        Ok(())
    }

    /// Удаляет пользовательскую метрику.
    pub fn remove_metric(&self, metric_id: &str) -> Result<()> {
        let mut configs = self.metrics_config.write().map_err(|e| {
            anyhow::anyhow!("Не удалось заблокировать конфигурации метрик: {}", e)
        })?;

        if !configs.contains_key(metric_id) {
            return Err(anyhow::anyhow!(
                "Метрика с идентификатором '{}' не найдена",
                metric_id
            ));
        }

        configs.remove(metric_id);
        
        let mut values = self.metrics_values.write().map_err(|e| {
            anyhow::anyhow!("Не удалось заблокировать значения метрик: {}", e)
        })?;

        values.remove(metric_id);

        Ok(())
    }

    /// Возвращает конфигурацию метрики.
    pub fn get_metric_config(&self, metric_id: &str) -> Result<Option<CustomMetricConfig>> {
        let configs = self.metrics_config.read().map_err(|e| {
            anyhow::anyhow!("Не удалось заблокировать конфигурации метрик: {}", e)
        })?;

        Ok(configs.get(metric_id).cloned())
    }

    /// Возвращает текущее значение метрики.
    pub fn get_metric_value(&self, metric_id: &str) -> Result<Option<CustomMetricValueWithTimestamp>> {
        let values = self.metrics_values.read().map_err(|e| {
            anyhow::anyhow!("Не удалось заблокировать значения метрик: {}", e)
        })?;

        Ok(values.get(metric_id).cloned())
    }

    /// Возвращает все конфигурации метрик.
    pub fn get_all_metrics_config(&self) -> Result<HashMap<String, CustomMetricConfig>> {
        let configs = self.metrics_config.read().map_err(|e| {
            anyhow::anyhow!("Не удалось заблокировать конфигурации метрик: {}", e)
        })?;

        Ok(configs.clone())
    }

    /// Возвращает все текущие значения метрик.
    pub fn get_all_metrics_values(&self) -> Result<HashMap<String, CustomMetricValueWithTimestamp>> {
        let values = self.metrics_values.read().map_err(|e| {
            anyhow::anyhow!("Не удалось заблокировать значения метрик: {}", e)
        })?;

        Ok(values.clone())
    }

    /// Обновляет значение метрики.
    pub async fn update_metric_value(&self, metric_id: &str) -> Result<()> {
        let configs = self.metrics_config.read().map_err(|e| {
            anyhow::anyhow!("Не удалось заблокировать конфигурации метрик: {}", e)
        })?;

        let config = match configs.get(metric_id) {
            Some(cfg) => cfg,
            None => return Err(anyhow::anyhow!(
                "Метрика с идентификатором '{}' не найдена",
                metric_id
            )),
        };

        if !config.enabled {
            let mut values = self.metrics_values.write().map_err(|e| {
                anyhow::anyhow!("Не удалось заблокировать значения метрик: {}", e)
            })?;

            if let Some(value) = values.get_mut(metric_id) {
                value.status = MetricStatus::Disabled;
            }

            return Ok(());
        }

        let value_result = match &config.source {
            CustomMetricSource::File { path, format } => {
                self.read_file_metric(path, format).await
            }
            CustomMetricSource::Command { command, args, format } => {
                self.execute_command_metric(command, args, format).await
            }
            CustomMetricSource::Http { url, method, headers, body, json_path } => {
                self.fetch_http_metric(url, method, headers, body, json_path).await
            }
            CustomMetricSource::Static { value } => {
                Ok(value.clone())
            }
        };

        let mut values = self.metrics_values.write().map_err(|e| {
            anyhow::anyhow!("Не удалось заблокировать значения метрик: {}", e)
        })?;

        match value_result {
            Ok(value) => {
                values.insert(
                    metric_id.to_string(),
                    CustomMetricValueWithTimestamp {
                        value,
                        timestamp: SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs(),
                        status: MetricStatus::Ok,
                    },
                );
            }
            Err(e) => {
                if let Some(existing_value) = values.get_mut(metric_id) {
                    existing_value.status = MetricStatus::Error {
                        message: format!("Ошибка обновления метрики: {}", e),
                    };
                } else {
                    values.insert(
                        metric_id.to_string(),
                        CustomMetricValueWithTimestamp {
                            value: CustomMetricValue::String("ERROR".to_string()),
                            timestamp: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs(),
                            status: MetricStatus::Error {
                                message: format!("Ошибка обновления метрики: {}", e),
                            },
                        },
                    );
                }
            }
        }

        Ok(())
    }

    /// Считывает метрику из файла.
    async fn read_file_metric(
        &self,
        path: &PathBuf,
        format: &FileMetricFormat,
    ) -> Result<CustomMetricValue> {
        let content = fs::read_to_string(path)
            .await
            .with_context(|| {
                format!(
                    "Не удалось прочитать файл {}: проверьте путь и права доступа",
                    path.display()
                )
            })?;

        match format {
            FileMetricFormat::PlainNumber => {
                let trimmed = content.trim();
                if let Ok(int_val) = trimmed.parse::<i64>() {
                    Ok(CustomMetricValue::Integer(int_val))
                } else if let Ok(float_val) = trimmed.parse::<f64>() {
                    Ok(CustomMetricValue::Float(float_val))
                } else {
                    Err(anyhow::anyhow!(
                        "Не удалось разобрать числовое значение из файла {}: '{}'",
                        path.display(),
                        trimmed
                    ))
                }
            }
            FileMetricFormat::Json { path: json_path } => {
                let parsed: serde_json::Value = serde_json::from_str(&content)
                    .with_context(|| {
                        format!(
                            "Не удалось разобрать JSON из файла {}: проверьте формат JSON",
                            path.display()
                        )
                    })?;

                let value = self.extract_json_value(&parsed, json_path)?;
                Ok(value)
            }
            FileMetricFormat::Regex { pattern } => {
                let re = regex::Regex::new(pattern)
                    .with_context(|| {
                        format!("Не удалось скомпилировать регулярное выражение: {}", pattern)
                    })?;

                if let Some(captures) = re.captures(&content) {
                    if let Some(match_str) = captures.get(1) {
                        let matched = match_str.as_str().trim();
                        if let Ok(int_val) = matched.parse::<i64>() {
                            Ok(CustomMetricValue::Integer(int_val))
                        } else if let Ok(float_val) = matched.parse::<f64>() {
                            Ok(CustomMetricValue::Float(float_val))
                        } else {
                            Ok(CustomMetricValue::String(matched.to_string()))
                        }
                    } else {
                        Err(anyhow::anyhow!(
                            "Регулярное выражение не содержит групп захвата: {}",
                            pattern
                        ))
                    }
                } else {
                    Err(anyhow::anyhow!(
                        "Регулярное выражение не нашло совпадений в файле {}: {}",
                        path.display(),
                        pattern
                    ))
                }
            }
        }
    }

    /// Выполняет команду и извлекает метрику.
    async fn execute_command_metric(
        &self,
        command: &str,
        args: &[String],
        format: &CommandMetricFormat,
    ) -> Result<CustomMetricValue> {
        use tokio::process::Command as TokioCommand;

        let output = TokioCommand::new(command)
            .args(args)
            .output()
            .await
            .with_context(|| {
                format!(
                    "Не удалось выполнить команду '{}': проверьте, что команда существует и доступна",
                    command
                )
            })?;

        if !output.status.success() {
            return Err(anyhow::anyhow!(
                "Команда '{}' завершилась с ошибкой: {}",
                command,
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let stdout = String::from_utf8(output.stdout)
            .with_context(|| {
                format!(
                    "Не удалось преобразовать вывод команды '{}' в UTF-8",
                    command
                )
            })?;

        match format {
            CommandMetricFormat::PlainNumber => {
                let trimmed = stdout.trim();
                if let Ok(int_val) = trimmed.parse::<i64>() {
                    Ok(CustomMetricValue::Integer(int_val))
                } else if let Ok(float_val) = trimmed.parse::<f64>() {
                    Ok(CustomMetricValue::Float(float_val))
                } else {
                    Err(anyhow::anyhow!(
                        "Не удалось разобрать числовое значение из вывода команды '{}': '{}'",
                        command,
                        trimmed
                    ))
                }
            }
            CommandMetricFormat::Json { path } => {
                let parsed: serde_json::Value = serde_json::from_str(&stdout)
                    .with_context(|| {
                        format!(
                            "Не удалось разобрать JSON из вывода команды '{}': проверьте формат JSON",
                            command
                        )
                    })?;

                let value = self.extract_json_value(&parsed, path)?;
                Ok(value)
            }
            CommandMetricFormat::Regex { pattern } => {
                let re = regex::Regex::new(pattern)
                    .with_context(|| {
                        format!("Не удалось скомпилировать регулярное выражение: {}", pattern)
                    })?;

                if let Some(captures) = re.captures(&stdout) {
                    if let Some(match_str) = captures.get(1) {
                        let matched = match_str.as_str().trim();
                        if let Ok(int_val) = matched.parse::<i64>() {
                            Ok(CustomMetricValue::Integer(int_val))
                        } else if let Ok(float_val) = matched.parse::<f64>() {
                            Ok(CustomMetricValue::Float(float_val))
                        } else {
                            Ok(CustomMetricValue::String(matched.to_string()))
                        }
                    } else {
                        Err(anyhow::anyhow!(
                            "Регулярное выражение не содержит групп захвата: {}",
                            pattern
                        ))
                    }
                } else {
                    Err(anyhow::anyhow!(
                        "Регулярное выражение не нашло совпадений в выводе команды '{}': {}",
                        command,
                        pattern
                    ))
                }
            }
        }
    }

    /// Извлекает значение из HTTP API.
    async fn fetch_http_metric(
        &self,
        url: &str,
        method: &str,
        headers: &HashMap<String, String>,
        body: &Option<String>,
        json_path: &str,
    ) -> Result<CustomMetricValue> {
        let client = reqwest::Client::new();

        let method = match method.to_uppercase().as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            _ => reqwest::Method::GET,
        };

        let mut request = client.request(method, url);

        // Добавляем заголовки
        for (key, value) in headers {
            request = request.header(key, value);
        }

        // Добавляем тело запроса, если есть
        if let Some(body_content) = body {
            request = request.body(body_content.clone());
        }

        let response = request
            .send()
            .await
            .with_context(|| {
                format!("Не удалось выполнить HTTP запрос к {}: проверьте URL и сеть", url)
            })?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "HTTP запрос к {} завершился с ошибкой: {}",
                url,
                response.status()
            ));
        }

        let content = response
            .text()
            .await
            .with_context(|| {
                format!("Не удалось прочитать ответ от {}: ошибка чтения", url)
            })?;

        let parsed: serde_json::Value = serde_json::from_str(&content)
            .with_context(|| {
                format!(
                    "Не удалось разобрать JSON ответ от {}: проверьте формат JSON",
                    url
                )
            })?;

        let value = self.extract_json_value(&parsed, json_path)?;
        Ok(value)
    }

    /// Извлекает значение из JSON по указанному пути.
    fn extract_json_value(
        &self,
        json: &serde_json::Value,
        path: &str,
    ) -> Result<CustomMetricValue> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = json;

        for part in parts {
            if let Some(obj) = current.as_object() {
                if let Some(value) = obj.get(part) {
                    current = value;
                } else {
                    return Err(anyhow::anyhow!(
                        "Путь JSON '{}' не найден: часть '{}' не существует",
                        path,
                        part
                    ));
                }
            } else if let Some(arr) = current.as_array() {
                if let Ok(index) = part.parse::<usize>() {
                    if index < arr.len() {
                        current = &arr[index];
                    } else {
                        return Err(anyhow::anyhow!(
                            "Путь JSON '{}' не найден: индекс {} вне диапазона",
                            path,
                            index
                        ));
                    }
                } else {
                    return Err(anyhow::anyhow!(
                        "Путь JSON '{}' некорректен: '{}' не является индексом массива",
                        path,
                        part
                    ));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "Путь JSON '{}' некорректен: текущее значение не является объектом или массивом",
                    path
                ));
            }
        }

        match current {
            serde_json::Value::Number(num) => {
                if num.is_i64() {
                    Ok(CustomMetricValue::Integer(num.as_i64().unwrap()))
                } else {
                    Ok(CustomMetricValue::Float(num.as_f64().unwrap()))
                }
            }
            serde_json::Value::Bool(b) => Ok(CustomMetricValue::Boolean(*b)),
            serde_json::Value::String(s) => Ok(CustomMetricValue::String(s.clone())),
            _ => Ok(CustomMetricValue::String(current.to_string())),
        }
    }

    /// Запускает цикл обновления метрик.
    pub fn start_update_loop(&self) {
        let mut running = self.running.write().unwrap();
        if *running {
            return;
        }
        *running = true;
        drop(running);

        let manager = self.clone();

        tokio::spawn(async move {
            loop {
                let running = {
                    let r = manager.running.read().unwrap();
                    *r
                };

                if !running {
                    break;
                }

                let configs = {
                    let c = manager.metrics_config.read().unwrap();
                    c.clone()
                };

                for (metric_id, config) in configs.iter() {
                    if config.enabled && config.update_interval_sec > 0 {
                        if let Err(e) = manager.update_metric_value(metric_id).await {
                            tracing::error!(
                                "Ошибка обновления пользовательской метрики '{}': {}",
                                metric_id,
                                e
                            );
                        }
                    }
                }

                // Ждём перед следующей итерацией
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
    }

    /// Останавливает цикл обновления метрик.
    pub fn stop_update_loop(&self) {
        let mut running = self.running.write().unwrap();
        *running = false;
    }

    /// Возвращает статус работы менеджера.
    pub fn is_running(&self) -> bool {
        let running = self.running.read().unwrap();
        *running
    }
}

impl Clone for CustomMetricsManager {
    fn clone(&self) -> Self {
        Self {
            metrics_config: Arc::clone(&self.metrics_config),
            metrics_values: Arc::clone(&self.metrics_values),
            running: Arc::clone(&self.running),
        }
    }
}

impl Default for CustomMetricsManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::{NamedTempFile, TempDir};

    #[test]
    fn test_custom_metric_creation() {
        let manager = CustomMetricsManager::new();
        
        let config = CustomMetricConfig {
            id: "test_metric".to_string(),
            name: "Test Metric".to_string(),
            description: "A test metric".to_string(),
            metric_type: CustomMetricType::Integer,
            source: CustomMetricSource::Static {
                value: CustomMetricValue::Integer(42),
            },
            update_interval_sec: 60,
            enabled: true,
        };

        let result = manager.add_metric(config);
        assert!(result.is_ok(), "Should be able to add metric");
    }

    #[test]
    fn test_duplicate_metric() {
        let manager = CustomMetricsManager::new();
        
        let config = CustomMetricConfig {
            id: "test_metric".to_string(),
            name: "Test Metric".to_string(),
            description: "A test metric".to_string(),
            metric_type: CustomMetricType::Integer,
            source: CustomMetricSource::Static {
                value: CustomMetricValue::Integer(42),
            },
            update_interval_sec: 60,
            enabled: true,
        };

        assert!(manager.add_metric(config.clone()).is_ok());
        assert!(manager.add_metric(config).is_err(), "Should not allow duplicate metric");
    }

    #[test]
    fn test_get_metric_config() {
        let manager = CustomMetricsManager::new();
        
        let config = CustomMetricConfig {
            id: "test_metric".to_string(),
            name: "Test Metric".to_string(),
            description: "A test metric".to_string(),
            metric_type: CustomMetricType::Integer,
            source: CustomMetricSource::Static {
                value: CustomMetricValue::Integer(42),
            },
            update_interval_sec: 60,
            enabled: true,
        };

        manager.add_metric(config.clone()).unwrap();
        let retrieved = manager.get_metric_config("test_metric").unwrap();
        assert!(retrieved.is_some(), "Should retrieve metric config");
        assert_eq!(retrieved.unwrap().id, "test_metric");
    }

    #[test]
    fn test_remove_metric() {
        let manager = CustomMetricsManager::new();
        
        let config = CustomMetricConfig {
            id: "test_metric".to_string(),
            name: "Test Metric".to_string(),
            description: "A test metric".to_string(),
            metric_type: CustomMetricType::Integer,
            source: CustomMetricSource::Static {
                value: CustomMetricValue::Integer(42),
            },
            update_interval_sec: 60,
            enabled: true,
        };

        manager.add_metric(config).unwrap();
        let result = manager.remove_metric("test_metric");
        assert!(result.is_ok(), "Should be able to remove metric");
        assert!(manager.get_metric_config("test_metric").unwrap().is_none(), "Metric should be removed");
    }

    #[tokio::test]
    async fn test_file_metric_plain() {
        let manager = CustomMetricsManager::new();
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_metric.txt");
        
        // Создаём тестовый файл
        let mut file = fs::File::create(&file_path).await.unwrap();
        writeln!(file, "12345").unwrap();
        drop(file);

        let config = CustomMetricConfig {
            id: "file_metric".to_string(),
            name: "File Metric".to_string(),
            description: "A file-based metric".to_string(),
            metric_type: CustomMetricType::Integer,
            source: CustomMetricSource::File {
                path: file_path.clone(),
                format: FileMetricFormat::PlainNumber,
            },
            update_interval_sec: 60,
            enabled: true,
        };

        manager.add_metric(config).unwrap();
        manager.update_metric_value("file_metric").await.unwrap();
        
        let value = manager.get_metric_value("file_metric").unwrap();
        assert!(value.is_some(), "Should have metric value");
        
        if let Some(CustomMetricValueWithTimestamp { value: metric_value, status, .. }) = value {
            assert!(matches!(status, MetricStatus::Ok), "Status should be OK");
            assert!(matches!(metric_value, CustomMetricValue::Integer(12345)), "Should parse integer value");
        }
    }

    #[tokio::test]
    async fn test_static_metric() {
        let manager = CustomMetricsManager::new();
        
        let config = CustomMetricConfig {
            id: "static_metric".to_string(),
            name: "Static Metric".to_string(),
            description: "A static metric".to_string(),
            metric_type: CustomMetricType::String,
            source: CustomMetricSource::Static {
                value: CustomMetricValue::String("Hello World".to_string()),
            },
            update_interval_sec: 60,
            enabled: true,
        };

        manager.add_metric(config).unwrap();
        manager.update_metric_value("static_metric").await.unwrap();
        
        let value = manager.get_metric_value("static_metric").unwrap();
        assert!(value.is_some(), "Should have metric value");
        
        if let Some(CustomMetricValueWithTimestamp { value: metric_value, status, .. }) = value {
            assert!(matches!(status, MetricStatus::Ok), "Status should be OK");
            assert!(matches!(metric_value, CustomMetricValue::String(ref s) if s == "Hello World"), "Should have static string value");
        }
    }

    #[test]
    fn test_manager_clone() {
        let manager1 = CustomMetricsManager::new();
        let manager2 = manager1.clone();
        
        let config = CustomMetricConfig {
            id: "test_metric".to_string(),
            name: "Test Metric".to_string(),
            description: "A test metric".to_string(),
            metric_type: CustomMetricType::Integer,
            source: CustomMetricSource::Static {
                value: CustomMetricValue::Integer(42),
            },
            update_interval_sec: 60,
            enabled: true,
        };

        manager1.add_metric(config).unwrap();
        let value1 = manager1.get_metric_config("test_metric").unwrap();
        let value2 = manager2.get_metric_config("test_metric").unwrap();
        
        assert!(value1.is_some(), "Manager1 should have metric");
        assert!(value2.is_some(), "Manager2 should have metric");
        assert_eq!(value1.unwrap().id, value2.unwrap().id, "Both managers should have same metric");
    }

    #[test]
    fn test_disabled_metric() {
        let manager = CustomMetricsManager::new();
        
        let config = CustomMetricConfig {
            id: "disabled_metric".to_string(),
            name: "Disabled Metric".to_string(),
            description: "A disabled metric".to_string(),
            metric_type: CustomMetricType::Integer,
            source: CustomMetricSource::Static {
                value: CustomMetricValue::Integer(42),
            },
            update_interval_sec: 60,
            enabled: false,
        };

        manager.add_metric(config).unwrap();
        let result = manager.update_metric_value("disabled_metric");
        assert!(result.is_ok(), "Should handle disabled metric without error");
        
        let value = manager.get_metric_value("disabled_metric").unwrap();
        assert!(value.is_some(), "Should have metric value");
        
        if let Some(CustomMetricValueWithTimestamp { status, .. }) = value {
            assert!(matches!(status, MetricStatus::Disabled), "Status should be Disabled");
        }
    }
}