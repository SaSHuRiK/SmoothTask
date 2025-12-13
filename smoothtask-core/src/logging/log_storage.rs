//! Модуль для хранения и управления логами приложения.
//!
//! Этот модуль предоставляет функциональность для хранения логов приложения
//! в памяти и предоставления их через API для мониторинга и отладки.
//!
//! # Основные компоненты
//!
//! - **LogStorage**: Основная структура для хранения логов
//! - **LogEntry**: Структура для представления одной записи лога
//! - **LogLevel**: Уровни логирования (совместимые с tracing)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Уровень логирования, совместимый с tracing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub enum LogLevel {
    /// Трейс - очень подробная отладочная информация
    Trace,
    /// Отладка - отладочная информация
    Debug,
    /// Информация - информационные сообщения о нормальной работе
    Info,
    /// Предупреждения - потенциальные проблемы или неоптимальные состояния
    Warn,
    /// Ошибки - критические проблемы, требующие немедленного внимания
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Trace => write!(f, "TRACE"),
        }
    }
}

impl From<tracing::Level> for LogLevel {
    fn from(level: tracing::Level) -> Self {
        match level {
            tracing::Level::ERROR => LogLevel::Error,
            tracing::Level::WARN => LogLevel::Warn,
            tracing::Level::INFO => LogLevel::Info,
            tracing::Level::DEBUG => LogLevel::Debug,
            tracing::Level::TRACE => LogLevel::Trace,
        }
    }
}

/// Запись лога.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Временная метка создания записи
    pub timestamp: DateTime<Utc>,
    /// Уровень логирования
    pub level: LogLevel,
    /// Модуль или компонент, создавший запись
    pub target: String,
    /// Сообщение лога
    pub message: String,
    /// Дополнительные поля (опционально)
    pub fields: Option<serde_json::Value>,
}

impl LogEntry {
    /// Создаёт новую запись лога.
    pub fn new(level: LogLevel, target: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            target: target.into(),
            message: message.into(),
            fields: None,
        }
    }

    /// Добавляет дополнительные поля к записи лога.
    pub fn with_fields(mut self, fields: serde_json::Value) -> Self {
        self.fields = Some(fields);
        self
    }
}

/// Хранилище логов.
#[derive(Debug, Default)]
pub struct LogStorage {
    /// Максимальное количество хранимых записей
    max_entries: usize,
    /// Максимальный возраст записей в секундах (0 = без ограничения)
    max_age_seconds: u64,
    /// Вектор записей логов (самые новые в конце)
    entries: Vec<LogEntry>,
    /// Счётчик ошибок логирования
    error_count: usize,
    /// Счётчик предупреждений
    warning_count: usize,
    /// Временная метка последней ошибки
    last_error_time: Option<DateTime<Utc>>,
    /// Временная метка последней очистки
    last_cleanup_time: Option<DateTime<Utc>>,
}

impl LogStorage {
    /// Создаёт новое хранилище логов с указанным максимальным количеством записей.
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            max_age_seconds: 0, // По умолчанию без ограничения по возрасту
            entries: Vec::with_capacity(max_entries),
            error_count: 0,
            warning_count: 0,
            last_error_time: None,
            last_cleanup_time: None,
        }
    }

    /// Создаёт новое хранилище логов с конфигурацией ротации.
    ///
    /// # Аргументы
    ///
    /// * `max_entries` - максимальное количество хранимых записей
    /// * `max_age_seconds` - максимальный возраст записей в секундах (0 = без ограничения)
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр LogStorage
    pub fn new_with_rotation(max_entries: usize, max_age_seconds: u64) -> Self {
        Self {
            max_entries,
            max_age_seconds,
            entries: Vec::with_capacity(max_entries),
            error_count: 0,
            warning_count: 0,
            last_error_time: None,
            last_cleanup_time: None,
        }
    }

    /// Добавляет новую запись в хранилище.
    /// Если превышено максимальное количество записей, самая старая запись удаляется.
    /// Также выполняет очистку старых записей, если включено ограничение по возрасту.
    pub fn add_entry(&mut self, entry: LogEntry) {
        // Обновляем счётчики в зависимости от уровня лога
        match entry.level {
            LogLevel::Error => {
                self.error_count += 1;
                self.last_error_time = Some(Utc::now());
            }
            LogLevel::Warn => {
                self.warning_count += 1;
            }
            _ => {}
        }
        
        // Выполняем очистку старых записей, если включено ограничение по возрасту
        self.cleanup_old_entries();
        
        // Применяем ограничение по количеству записей
        if self.entries.len() >= self.max_entries && self.max_entries > 0 {
            self.entries.remove(0); // Удаляем самую старую запись
        }
        self.entries.push(entry);
    }

    /// Добавляет новую запись в хранилище с проверкой использования памяти.
    /// Если превышено максимальное количество записей или лимит памяти, самая старая запись удаляется.
    /// Также выполняет очистку старых записей, если включено ограничение по возрасту.
    ///
    /// # Аргументы
    ///
    /// * `entry` - запись лога для добавления
    /// * `memory_limit_bytes` - максимальный размер памяти в байтах (0 = без ограничения)
    pub fn add_entry_with_memory_check(&mut self, entry: LogEntry, memory_limit_bytes: usize) {
        // Обновляем счётчики в зависимости от уровня лога
        match entry.level {
            LogLevel::Error => {
                self.error_count += 1;
                self.last_error_time = Some(Utc::now());
            }
            LogLevel::Warn => {
                self.warning_count += 1;
            }
            _ => {}
        }
        
        // Выполняем очистку старых записей, если включено ограничение по возрасту
        self.cleanup_old_entries();
        
        // Выполняем очистку на основе использования памяти
        self.cleanup_by_memory(memory_limit_bytes);
        
        // Применяем ограничение по количеству записей
        if self.entries.len() >= self.max_entries && self.max_entries > 0 {
            self.entries.remove(0); // Удаляем самую старую запись
        }
        self.entries.push(entry);
    }

    /// Выполняет очистку старых записей на основе максимального возраста.
    /// Удаляет записи, которые старше max_age_seconds.
    fn cleanup_old_entries(&mut self) {
        if self.max_age_seconds == 0 {
            return; // Очистка по возрасту отключена
        }

        let now = Utc::now();
        let cutoff_time = now - chrono::Duration::seconds(self.max_age_seconds as i64);

        // Находим индекс первой записи, которая должна остаться
        let first_valid_index = self.entries.iter().position(|entry| {
            entry.timestamp >= cutoff_time
        });

        // Удаляем все записи до первой валидной
        if let Some(index) = first_valid_index {
            if index > 0 {
                self.entries.drain(0..index);
            }
        } else {
            // Все записи слишком старые, очищаем всё
            self.entries.clear();
        }

        // Обновляем время последней очистки
        self.last_cleanup_time = Some(Utc::now());
    }

    /// Добавляет пакет записей в хранилище с оптимизацией производительности.
    /// Выполняет очистку только один раз для всего пакета вместо каждого вызова.
    ///
    /// # Аргументы
    ///
    /// * `entries` - вектор записей лога для добавления
    /// * `memory_limit_bytes` - максимальный размер памяти в байтах (0 = без ограничения)
    pub fn add_entries_batch(&mut self, entries: Vec<LogEntry>, memory_limit_bytes: usize) {
        if entries.is_empty() {
            return;
        }

        // Обновляем счётчики в зависимости от уровня лога (оптимизация: один проход)
        for entry in &entries {
            match entry.level {
                LogLevel::Error => {
                    self.error_count += 1;
                    self.last_error_time = Some(Utc::now());
                }
                LogLevel::Warn => {
                    self.warning_count += 1;
                }
                _ => {}
            }
        }

        // Выполняем очистку старых записей только один раз для всего пакета
        self.cleanup_old_entries();

        // Выполняем очистку на основе использования памяти только один раз
        self.cleanup_by_memory(memory_limit_bytes);

        // Применяем ограничение по количеству записей
        let capacity = if self.max_entries > 0 {
            self.max_entries - self.entries.len()
        } else {
            entries.len()
        };

        // Добавляем только те записи, которые помещаются в лимит
        let entries_to_add = if capacity > 0 {
            entries.into_iter().take(capacity).collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        self.entries.extend(entries_to_add);
    }

    /// Выполняет очистку на основе использования памяти.
    /// Удаляет старые записи, если использование памяти превышает лимит.
    ///
    /// # Аргументы
    ///
    /// * `memory_limit_bytes` - максимальный размер памяти в байтах
    fn cleanup_by_memory(&mut self, memory_limit_bytes: usize) {
        if memory_limit_bytes == 0 {
            return; // Очистка по памяти отключена
        }

        // Оцениваем текущее использование памяти
        // Используем приблизительную оценку: каждая запись занимает около 200-300 байт
        let estimated_memory_per_entry = 250; // байт на запись
        let estimated_current_memory = self.entries.len() * estimated_memory_per_entry;

        if estimated_current_memory <= memory_limit_bytes {
            return; // Память не превышена
        }

        // Вычисляем, сколько записей нужно удалить
        let target_entries = memory_limit_bytes / estimated_memory_per_entry;
        if target_entries >= self.entries.len() {
            return; // Уже в пределах лимита
        }

        // Удаляем старые записи, чтобы освободить память
        let entries_to_remove = self.entries.len() - target_entries;
        if entries_to_remove > 0 {
            self.entries.drain(0..entries_to_remove);
            info!(
                "Memory cleanup: removed {} old entries to stay within {} bytes limit",
                entries_to_remove, memory_limit_bytes
            );
        }

        // Обновляем время последней очистки
        self.last_cleanup_time = Some(Utc::now());
    }

    /// Возвращает оценку текущего использования памяти в байтах.
    pub fn estimate_memory_usage(&self) -> usize {
        // Приблизительная оценка: каждая запись занимает около 200-300 байт
        let estimated_memory_per_entry = 250; // байт на запись
        self.entries.len() * estimated_memory_per_entry
    }

    /// Возвращает все записи логов.
    pub fn get_all_entries(&self) -> Vec<LogEntry> {
        self.entries.clone()
    }

    /// Возвращает записи логов, отфильтрованные по уровню.
    pub fn get_entries_by_level(&self, min_level: LogLevel) -> Vec<LogEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.level >= min_level)
            .cloned()
            .collect()
    }

    /// Возвращает последние N записей.
    pub fn get_recent_entries(&self, limit: usize) -> Vec<LogEntry> {
        if limit == 0 {
            return Vec::new();
        }
        let start = if self.entries.len() > limit {
            self.entries.len() - limit
        } else {
            0
        };
        self.entries[start..].to_vec()
    }

    /// Очищает все записи логов.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Возвращает текущее количество записей.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Возвращает true, если хранилище пустое.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Возвращает максимальное количество записей.
    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    /// Возвращает количество ошибок в логах.
    pub fn error_count(&self) -> usize {
        self.error_count
    }

    /// Возвращает количество предупреждений в логах.
    pub fn warning_count(&self) -> usize {
        self.warning_count
    }

    /// Возвращает временную метку последней ошибки.
    pub fn last_error_time(&self) -> Option<DateTime<Utc>> {
        self.last_error_time
    }

    /// Возвращает временную метку последней очистки.
    pub fn last_cleanup_time(&self) -> Option<DateTime<Utc>> {
        self.last_cleanup_time
    }

    /// Возвращает максимальный возраст записей в секундах.
    pub fn max_age_seconds(&self) -> u64 {
        self.max_age_seconds
    }

    /// Обновляет конфигурацию ротации.
    ///
    /// # Аргументы
    ///
    /// * `max_entries` - новое максимальное количество записей
    /// * `max_age_seconds` - новый максимальный возраст записей в секундах
    pub fn update_rotation_config(&mut self, max_entries: usize, max_age_seconds: u64) {
        self.max_entries = max_entries;
        self.max_age_seconds = max_age_seconds;
        // Выполняем очистку сразу после обновления конфигурации
        self.cleanup_old_entries();
    }

    /// Возвращает статистику по уровням логов.
    pub fn get_level_statistics(&self) -> HashMap<LogLevel, usize> {
        let mut stats = HashMap::new();
        
        for entry in &self.entries {
            *stats.entry(entry.level).or_insert(0) += 1;
        }
        
        stats
    }

    /// Возвращает мониторинговые метрики для системы наблюдения.
    pub fn get_monitoring_metrics(&self) -> serde_json::Value {
        let level_stats = self.get_level_statistics();
        
        json!({
            "total_entries": self.entries.len(),
            "max_capacity": self.max_entries,
            "max_age_seconds": self.max_age_seconds,
            "estimated_memory_usage_bytes": self.estimate_memory_usage(),
            "error_count": self.error_count,
            "warning_count": self.warning_count,
            "last_error_time": self.last_error_time.map(|t| t.to_rfc3339()),
            "last_cleanup_time": self.last_cleanup_time.map(|t| t.to_rfc3339()),
            "level_distribution": {
                "error": level_stats.get(&LogLevel::Error).unwrap_or(&0),
                "warn": level_stats.get(&LogLevel::Warn).unwrap_or(&0),
                "info": level_stats.get(&LogLevel::Info).unwrap_or(&0),
                "debug": level_stats.get(&LogLevel::Debug).unwrap_or(&0),
                "trace": level_stats.get(&LogLevel::Trace).unwrap_or(&0),
            },
            "health_status": self.get_health_status()
        })
    }

    /// Возвращает статус здоровья системы логирования.
    pub fn get_health_status(&self) -> String {
        if self.error_count > 0 {
            if self.last_error_time.is_some_and(|t| t > Utc::now() - chrono::Duration::minutes(5)) {
                "critical".to_string()
            } else {
                "warning".to_string()
            }
        } else if self.warning_count > 10 {
            "warning".to_string()
        } else {
            "healthy".to_string()
        }
    }
}

/// Потокобезопасная версия хранилища логов.
#[derive(Debug, Clone)]
pub struct SharedLogStorage {
    inner: Arc<RwLock<LogStorage>>,
}

impl SharedLogStorage {
    /// Создаёт новое потокобезопасное хранилище логов.
    pub fn new(max_entries: usize) -> Self {
        Self {
            inner: Arc::new(RwLock::new(LogStorage::new(max_entries))),
        }
    }

    /// Создаёт новое потокобезопасное хранилище логов с конфигурацией ротации.
    ///
    /// # Аргументы
    ///
    /// * `max_entries` - максимальное количество хранимых записей
    /// * `max_age_seconds` - максимальный возраст записей в секундах (0 = без ограничения)
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр SharedLogStorage
    pub fn new_with_rotation(max_entries: usize, max_age_seconds: u64) -> Self {
        Self {
            inner: Arc::new(RwLock::new(LogStorage::new_with_rotation(max_entries, max_age_seconds))),
        }
    }

    /// Добавляет новую запись в хранилище.
    pub async fn add_entry(&self, entry: LogEntry) {
        let mut storage = self.inner.write().await;
        storage.add_entry(entry);
    }

    /// Добавляет новую запись в хранилище с проверкой использования памяти.
    ///
    /// # Аргументы
    ///
    /// * `entry` - запись лога для добавления
    /// * `memory_limit_bytes` - максимальный размер памяти в байтах (0 = без ограничения)
    pub async fn add_entry_with_memory_check(&self, entry: LogEntry, memory_limit_bytes: usize) {
        let mut storage = self.inner.write().await;
        storage.add_entry_with_memory_check(entry, memory_limit_bytes);
    }

    /// Возвращает все записи логов.
    pub async fn get_all_entries(&self) -> Vec<LogEntry> {
        let storage = self.inner.read().await;
        storage.get_all_entries()
    }

    /// Возвращает записи логов, отфильтрованные по уровню.
    pub async fn get_entries_by_level(&self, min_level: LogLevel) -> Vec<LogEntry> {
        let storage = self.inner.read().await;
        storage.get_entries_by_level(min_level)
    }

    /// Возвращает последние N записей.
    pub async fn get_recent_entries(&self, limit: usize) -> Vec<LogEntry> {
        let storage = self.inner.read().await;
        storage.get_recent_entries(limit)
    }

    /// Очищает все записи логов.
    pub async fn clear(&self) {
        let mut storage = self.inner.write().await;
        storage.clear();
    }

    /// Возвращает текущее количество записей.
    pub async fn len(&self) -> usize {
        let storage = self.inner.read().await;
        storage.len()
    }

    /// Возвращает true, если хранилище пустое.
    pub async fn is_empty(&self) -> bool {
        let storage = self.inner.read().await;
        storage.is_empty()
    }

    /// Возвращает максимальное количество записей.
    pub async fn max_entries(&self) -> usize {
        let storage = self.inner.read().await;
        storage.max_entries()
    }

    /// Возвращает максимальный возраст записей в секундах.
    pub async fn max_age_seconds(&self) -> u64 {
        let storage = self.inner.read().await;
        storage.max_age_seconds()
    }

    /// Обновляет конфигурацию ротации.
    ///
    /// # Аргументы
    ///
    /// * `max_entries` - новое максимальное количество записей
    /// * `max_age_seconds` - новый максимальный возраст записей в секундах
    pub async fn update_rotation_config(&self, max_entries: usize, max_age_seconds: u64) {
        let mut storage = self.inner.write().await;
        storage.update_rotation_config(max_entries, max_age_seconds);
    }

    /// Возвращает временную метку последней очистки.
    pub async fn last_cleanup_time(&self) -> Option<DateTime<Utc>> {
        let storage = self.inner.read().await;
        storage.last_cleanup_time()
    }

    /// Возвращает количество ошибок в логах.
    pub async fn error_count(&self) -> usize {
        let storage = self.inner.read().await;
        storage.error_count()
    }

    /// Возвращает количество предупреждений в логах.
    pub async fn warning_count(&self) -> usize {
        let storage = self.inner.read().await;
        storage.warning_count()
    }

    /// Возвращает временную метку последней ошибки.
    pub async fn last_error_time(&self) -> Option<DateTime<Utc>> {
        let storage = self.inner.read().await;
        storage.last_error_time()
    }

    /// Возвращает мониторинговые метрики для системы наблюдения.
    pub async fn get_monitoring_metrics(&self) -> serde_json::Value {
        let storage = self.inner.read().await;
        storage.get_monitoring_metrics()
    }

    /// Возвращает статус здоровья системы логирования.
    pub async fn get_health_status(&self) -> String {
        let storage = self.inner.read().await;
        storage.get_health_status()
    }
}

/// Интеграция с tracing для автоматического логирования.
///
/// Этот макрос позволяет легко интегрировать LogStorage с tracing,
/// автоматически добавляя все события tracing в хранилище логов.
#[macro_export]
macro_rules! setup_tracing_with_log_storage {
    ($log_storage:expr) => {
        use tracing_subscriber::{fmt, EnvFilter};

        // Создаем слой для логирования в консоль
        let console_layer = fmt::layer()
            .with_target(true)
            .with_line_number(true)
            .with_thread_ids(true)
            .with_thread_names(true);

        // Создаем слой для логирования в хранилище
        let storage_layer = tracing_subscriber::fmt::layer()
            .with_writer(move || {
                let storage = $log_storage.clone();
                TracingLogWriter { storage }
            })
            .with_ansi(false)
            .with_target(true);

        // Устанавливаем оба слоя
        tracing_subscriber::registry()
            .with(console_layer)
            .with(storage_layer)
            .with(EnvFilter::from_default_env())
            .init();
    };
}

/// Внутренний writer для интеграции с tracing.
///
/// Используется в макросе `setup_tracing_with_log_storage!` для перенаправления
/// логов tracing в хранилище логов.
#[allow(dead_code)]
struct TracingLogWriter {
    storage: SharedLogStorage,
}

impl std::io::Write for TracingLogWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Ok(message) = String::from_utf8(buf.to_vec()) {
            // Парсим сообщение tracing (упрощенный парсинг)
            // Формат: [YYYY-MM-DDTHH:MM:SSZ LEVEL TARGET] message
            if let Some((meta_part, msg_part)) = message.split_once("] ") {
                if let Some(start_bracket) = meta_part.find('[') {
                    let meta = &meta_part[start_bracket + 1..];
                    let parts: Vec<&str> = meta.split_whitespace().collect();

                    if parts.len() >= 3 {
                        let level_str = parts[1];
                        let target = parts[2..].join(" ");

                        let level = match level_str {
                            "ERROR" => LogLevel::Error,
                            "WARN" => LogLevel::Warn,
                            "INFO" => LogLevel::Info,
                            "DEBUG" => LogLevel::Debug,
                            "TRACE" => LogLevel::Trace,
                            _ => LogLevel::Info,
                        };

                        let entry = LogEntry::new(level, target, msg_part.trim());

                        // Добавляем запись в хранилище (асинхронно)
                        let storage = self.storage.clone();
                        tokio::spawn(async move {
                            storage.add_entry(entry).await;
                        });
                    } else {
                        eprintln!("Не удалось разобрать сообщение tracing: недостаточно частей в метаданных (ожидалось >= 3, получено {})", parts.len());
                    }
                } else {
                    eprintln!("Не удалось разобрать сообщение tracing: не найдена открывающая скобка '['");
                }
            } else {
                eprintln!("Не удалось разобрать сообщение tracing: не найден разделитель '] '");
            }
        } else {
            eprintln!("Не удалось декодировать сообщение tracing как UTF-8");
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry::new(LogLevel::Info, "test_module", "Test message");

        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.target, "test_module");
        assert_eq!(entry.message, "Test message");
        assert!(entry.timestamp <= Utc::now());
    }

    #[test]
    fn test_log_entry_with_fields() {
        let mut entry = LogEntry::new(LogLevel::Debug, "test_module", "Debug message");

        let fields = serde_json::json!({
            "key1": "value1",
            "key2": 42
        });

        entry = entry.with_fields(fields.clone());

        assert!(entry.fields.is_some());
        assert_eq!(entry.fields.unwrap(), fields);
    }

    #[test]
    fn test_log_storage_add_and_retrieve() {
        let mut storage = LogStorage::new(10);

        let entry1 = LogEntry::new(LogLevel::Info, "module1", "Message 1");
        let entry2 = LogEntry::new(LogLevel::Warn, "module2", "Message 2");

        storage.add_entry(entry1.clone());
        storage.add_entry(entry2.clone());

        let entries = storage.get_all_entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "Message 1");
        assert_eq!(entries[1].message, "Message 2");
    }

    #[test]
    fn test_log_storage_max_entries() {
        let mut storage = LogStorage::new(2);

        storage.add_entry(LogEntry::new(LogLevel::Info, "m1", "Msg1"));
        storage.add_entry(LogEntry::new(LogLevel::Info, "m2", "Msg2"));
        storage.add_entry(LogEntry::new(LogLevel::Info, "m3", "Msg3"));

        let entries = storage.get_all_entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "Msg2");
        assert_eq!(entries[1].message, "Msg3");
    }

    #[test]
    fn test_log_storage_filter_by_level() {
        let mut storage = LogStorage::new(10);

        storage.add_entry(LogEntry::new(LogLevel::Trace, "m1", "Trace msg"));
        storage.add_entry(LogEntry::new(LogLevel::Debug, "m2", "Debug msg"));
        storage.add_entry(LogEntry::new(LogLevel::Info, "m3", "Info msg"));
        storage.add_entry(LogEntry::new(LogLevel::Warn, "m4", "Warn msg"));
        storage.add_entry(LogEntry::new(LogLevel::Error, "m5", "Error msg"));

        let filtered = storage.get_entries_by_level(LogLevel::Warn);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].message, "Warn msg");
        assert_eq!(filtered[1].message, "Error msg");
    }

    #[test]
    fn test_log_storage_recent_entries() {
        let mut storage = LogStorage::new(10);

        for i in 1..=5 {
            storage.add_entry(LogEntry::new(LogLevel::Info, "m", format!("Msg{}", i)));
        }

        let recent = storage.get_recent_entries(3);
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].message, "Msg3");
        assert_eq!(recent[1].message, "Msg4");
        assert_eq!(recent[2].message, "Msg5");
    }

    #[test]
    fn test_log_storage_clear() {
        let mut storage = LogStorage::new(10);

        storage.add_entry(LogEntry::new(LogLevel::Info, "m", "Msg1"));
        storage.add_entry(LogEntry::new(LogLevel::Info, "m", "Msg2"));

        assert_eq!(storage.len(), 2);

        storage.clear();

        assert_eq!(storage.len(), 0);
        assert!(storage.is_empty());
    }

    #[tokio::test]
    async fn test_shared_log_storage() {
        let storage = SharedLogStorage::new(10);

        let entry1 = LogEntry::new(LogLevel::Info, "module1", "Message 1");
        let entry2 = LogEntry::new(LogLevel::Warn, "module2", "Message 2");

        storage.add_entry(entry1.clone()).await;
        storage.add_entry(entry2.clone()).await;

        let entries = storage.get_all_entries().await;
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "Message 1");
        assert_eq!(entries[1].message, "Message 2");
    }

    #[tokio::test]
    async fn test_shared_log_storage_concurrent() {
        let storage = SharedLogStorage::new(100);

        let mut tasks = Vec::new();

        for i in 0..10 {
            let storage = storage.clone();
            tasks.push(tokio::spawn(async move {
                for j in 0..5 {
                    let entry = LogEntry::new(
                        LogLevel::Info,
                        "test_module",
                        format!("Message {}-{}", i, j),
                    );
                    storage.add_entry(entry).await;
                }
            }));
        }

        // Ждем завершения всех задач
        for task in tasks {
            task.await.expect("Task should complete");
        }

        let entries = storage.get_all_entries().await;
        assert_eq!(entries.len(), 50);
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(format!("{}", LogLevel::Error), "ERROR");
        assert_eq!(format!("{}", LogLevel::Warn), "WARN");
        assert_eq!(format!("{}", LogLevel::Info), "INFO");
        assert_eq!(format!("{}", LogLevel::Debug), "DEBUG");
        assert_eq!(format!("{}", LogLevel::Trace), "TRACE");
    }

    #[test]
    fn test_log_level_ord() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }

    #[test]
    fn test_log_storage_error_tracking() {
        let mut storage = LogStorage::new(100);
        
        assert_eq!(storage.error_count(), 0);
        assert_eq!(storage.warning_count(), 0);
        assert!(storage.last_error_time().is_none());
        
        // Добавляем ошибку
        storage.add_entry(LogEntry::new(LogLevel::Error, "test", "Error message"));
        assert_eq!(storage.error_count(), 1);
        assert_eq!(storage.warning_count(), 0);
        assert!(storage.last_error_time().is_some());
        
        // Добавляем предупреждение
        storage.add_entry(LogEntry::new(LogLevel::Warn, "test", "Warning message"));
        assert_eq!(storage.error_count(), 1);
        assert_eq!(storage.warning_count(), 1);
        
        // Добавляем ещё одну ошибку
        storage.add_entry(LogEntry::new(LogLevel::Error, "test", "Another error"));
        assert_eq!(storage.error_count(), 2);
        assert_eq!(storage.warning_count(), 1);
    }

    #[test]
    fn test_log_storage_level_statistics() {
        let mut storage = LogStorage::new(100);
        
        storage.add_entry(LogEntry::new(LogLevel::Error, "test", "Error"));
        storage.add_entry(LogEntry::new(LogLevel::Warn, "test", "Warning"));
        storage.add_entry(LogEntry::new(LogLevel::Info, "test", "Info"));
        storage.add_entry(LogEntry::new(LogLevel::Debug, "test", "Debug"));
        storage.add_entry(LogEntry::new(LogLevel::Trace, "test", "Trace"));
        
        let stats = storage.get_level_statistics();
        assert_eq!(stats.get(&LogLevel::Error), Some(&1));
        assert_eq!(stats.get(&LogLevel::Warn), Some(&1));
        assert_eq!(stats.get(&LogLevel::Info), Some(&1));
        assert_eq!(stats.get(&LogLevel::Debug), Some(&1));
        assert_eq!(stats.get(&LogLevel::Trace), Some(&1));
    }

    #[test]
    fn test_log_storage_monitoring_metrics() {
        let mut storage = LogStorage::new(100);
        
        storage.add_entry(LogEntry::new(LogLevel::Error, "test", "Error"));
        storage.add_entry(LogEntry::new(LogLevel::Warn, "test", "Warning"));
        storage.add_entry(LogEntry::new(LogLevel::Info, "test", "Info"));
        
        let metrics = storage.get_monitoring_metrics();
        
        assert_eq!(metrics["total_entries"], 3);
        assert_eq!(metrics["max_capacity"], 100);
        assert_eq!(metrics["error_count"], 1);
        assert_eq!(metrics["warning_count"], 1);
        assert!(metrics["last_error_time"].is_string());
        
        let level_dist = &metrics["level_distribution"];
        assert_eq!(level_dist["error"], 1);
        assert_eq!(level_dist["warn"], 1);
        assert_eq!(level_dist["info"], 1);
        assert_eq!(level_dist["debug"], 0);
        assert_eq!(level_dist["trace"], 0);
    }

    #[test]
    fn test_log_storage_health_status() {
        let mut storage = LogStorage::new(100);
        
        // Healthy status (no errors or warnings)
        assert_eq!(storage.get_health_status(), "healthy");
        
        // Add warnings
        for _ in 0..11 {
            storage.add_entry(LogEntry::new(LogLevel::Warn, "test", "Warning"));
        }
        assert_eq!(storage.get_health_status(), "warning");
        
        // Add error
        storage.add_entry(LogEntry::new(LogLevel::Error, "test", "Error"));
        assert_eq!(storage.get_health_status(), "critical");
    }

    #[test]
    fn test_log_storage_rotation_by_age() {
        let mut storage = LogStorage::new_with_rotation(100, 10); // Максимум 10 секунд для теста
        
        // Добавляем записи
        storage.add_entry(LogEntry::new(LogLevel::Info, "test", "Message 1"));
        storage.add_entry(LogEntry::new(LogLevel::Info, "test", "Message 2"));
        
        assert_eq!(storage.len(), 2);
        
        // Имитируем прохождение времени (11 секунд назад)
        let eleven_seconds_ago = Utc::now() - chrono::Duration::seconds(11);
        let old_entry = LogEntry {
            timestamp: eleven_seconds_ago,
            level: LogLevel::Info,
            target: "test".to_string(),
            message: "Old message".to_string(),
            fields: None,
        };
        
        // Добавляем старую запись
        storage.entries.insert(0, old_entry);
        assert_eq!(storage.len(), 3);
        
        // Вручную вызываем очистку
        storage.cleanup_old_entries();
        
        // Проверяем, что старая запись удалена
        assert_eq!(storage.len(), 2);
        assert_eq!(storage.entries[0].message, "Message 1");
        assert_eq!(storage.entries[1].message, "Message 2");
    }

    #[test]
    fn test_log_storage_rotation_config_update() {
        let mut storage = LogStorage::new(10);
        
        // Добавляем записи
        for i in 1..=5 {
            storage.add_entry(LogEntry::new(LogLevel::Info, "test", format!("Message {}", i)));
        }
        
        assert_eq!(storage.len(), 5);
        assert_eq!(storage.max_age_seconds(), 0);
        
        // Обновляем конфигурацию с ограничением по возрасту
        storage.update_rotation_config(10, 10); // 10 секунд
        
        assert_eq!(storage.max_age_seconds(), 10);
        
        // Добавляем старую запись
        let old_entry = LogEntry {
            timestamp: Utc::now() - chrono::Duration::seconds(11),
            level: LogLevel::Info,
            target: "test".to_string(),
            message: "Old message".to_string(),
            fields: None,
        };
        storage.entries.insert(0, old_entry);
        
        // Вручную вызываем очистку
        storage.cleanup_old_entries();
        
        // Старая запись должна быть удалена
        assert_eq!(storage.len(), 5);
        assert_eq!(storage.entries[0].message, "Message 1");
    }

    #[test]
    fn test_log_storage_monitoring_metrics_with_rotation() {
        let mut storage = LogStorage::new_with_rotation(100, 3600);
        
        storage.add_entry(LogEntry::new(LogLevel::Error, "test", "Error"));
        storage.add_entry(LogEntry::new(LogLevel::Warn, "test", "Warning"));
        
        let metrics = storage.get_monitoring_metrics();
        
        assert_eq!(metrics["total_entries"], 2);
        assert_eq!(metrics["max_capacity"], 100);
        assert_eq!(metrics["max_age_seconds"], 3600);
        assert_eq!(metrics["error_count"], 1);
        assert_eq!(metrics["warning_count"], 1);
        assert!(metrics["last_error_time"].is_string());
        assert!(metrics["last_cleanup_time"].is_string());
    }

    #[test]
    fn test_log_storage_cleanup_time_tracking() {
        let mut storage = LogStorage::new_with_rotation(100, 1);
        
        assert!(storage.last_cleanup_time().is_none());
        
        // Добавляем запись, что должно вызвать очистку
        storage.add_entry(LogEntry::new(LogLevel::Info, "test", "Message"));
        
        assert!(storage.last_cleanup_time().is_some());
    }

    #[tokio::test]
    async fn test_shared_log_storage_rotation() {
        let storage = SharedLogStorage::new_with_rotation(100, 2);
        
        storage.add_entry(LogEntry::new(LogLevel::Info, "test", "Message 1")).await;
        storage.add_entry(LogEntry::new(LogLevel::Info, "test", "Message 2")).await;
        
        assert_eq!(storage.len().await, 2);
        assert_eq!(storage.max_age_seconds().await, 2);
        
        // Обновляем конфигурацию
        storage.update_rotation_config(50, 1).await;
        assert_eq!(storage.max_age_seconds().await, 1);
    }

    #[test]
    fn test_log_storage_memory_cleanup() {
        let mut storage = LogStorage::new(1000);
        
        // Добавляем много записей
        for i in 0..500 {
            storage.add_entry(LogEntry::new(LogLevel::Info, "test", format!("Message {}", i)));
        }
        
        assert_eq!(storage.len(), 500);
        
        // Выполняем очистку по памяти (лимит 100 КБ = 100_000 байт)
        // При 250 байт на запись, 100_000 / 250 = 400 записей
        storage.cleanup_by_memory(100_000);
        
        // Должно остаться около 400 записей
        assert!(storage.len() <= 400);
        assert!(storage.len() > 350); // Не должно быть слишком агрессивным
    }

    #[test]
    fn test_log_storage_memory_estimation() {
        let mut storage = LogStorage::new(100);
        
        assert_eq!(storage.estimate_memory_usage(), 0);
        
        // Добавляем записи
        for i in 0..10 {
            storage.add_entry(LogEntry::new(LogLevel::Info, "test", format!("Message {}", i)));
        }
        
        // Ожидаем около 2500 байт (10 записей * 250 байт)
        let memory_usage = storage.estimate_memory_usage();
        assert!(memory_usage >= 2000);
        assert!(memory_usage <= 3000);
    }

    #[test]
    fn test_log_storage_add_entry_with_memory_check() {
        let mut storage = LogStorage::new(100);
        
        // Добавляем записи с проверкой памяти (лимит 1 КБ = 1000 байт)
        // При 250 байт на запись, 1000 / 250 = 4 записи
        for i in 0..10 {
            storage.add_entry_with_memory_check(
                LogEntry::new(LogLevel::Info, "test", format!("Message {}", i)),
                1000
            );
        }
        
        // Должно остаться около 4 записей (может быть немного больше из-за точности оценки)
        assert!(storage.len() <= 6);
        assert!(!storage.is_empty());
    }

    #[tokio::test]
    async fn test_shared_log_storage_memory_cleanup() {
        let storage = SharedLogStorage::new(1000);
        
        // Добавляем много записей
        for i in 0..500 {
            storage.add_entry(LogEntry::new(LogLevel::Info, "test", format!("Message {}", i))).await;
        }
        
        assert_eq!(storage.len().await, 500);
        
        // Выполняем очистку по памяти
        let mut storage_guard = storage.inner.write().await;
        storage_guard.cleanup_by_memory(100_000);
        
        // Должно остаться около 400 записей
        assert!(storage_guard.len() <= 400);
    }

    #[tokio::test]
    async fn test_shared_log_storage_add_entry_with_memory_check() {
        let storage = SharedLogStorage::new(100);
        
        // Добавляем записи с проверкой памяти
        for i in 0..10 {
            storage.add_entry_with_memory_check(
                LogEntry::new(LogLevel::Info, "test", format!("Message {}", i)),
                1000
            ).await;
        }
        
        // Должно остаться около 4 записей (может быть немного больше из-за точности оценки)
        assert!(storage.len().await <= 6);
    }

    #[test]
    fn test_log_storage_monitoring_metrics_with_memory() {
        let mut storage = LogStorage::new(100);
        
        // Добавляем записи
        for i in 0..10 {
            storage.add_entry(LogEntry::new(LogLevel::Info, "test", format!("Message {}", i)));
        }
        
        let metrics = storage.get_monitoring_metrics();
        
        assert_eq!(metrics["total_entries"], 10);
        assert!(metrics["estimated_memory_usage_bytes"].as_u64().unwrap() >= 2000);
        assert!(metrics["estimated_memory_usage_bytes"].as_u64().unwrap() <= 3000);
    }

    #[tokio::test]
    async fn test_shared_log_storage_monitoring() {
        let storage = SharedLogStorage::new(100);
        
        storage.add_entry(LogEntry::new(LogLevel::Error, "test", "Error")).await;
        storage.add_entry(LogEntry::new(LogLevel::Warn, "test", "Warning")).await;
        
        assert_eq!(storage.error_count().await, 1);
        assert_eq!(storage.warning_count().await, 1);
        assert_eq!(storage.get_health_status().await, "critical");
        
        let metrics = storage.get_monitoring_metrics().await;
        assert_eq!(metrics["error_count"], 1);
        assert_eq!(metrics["warning_count"], 1);
    }

    #[test]
    fn test_log_storage_error_context() {
        let mut storage = LogStorage::new(100);
        
        // Test that error tracking works correctly with context
        storage.add_entry(LogEntry::new(LogLevel::Error, "module1", "Critical error occurred"));
        storage.add_entry(LogEntry::new(LogLevel::Error, "module2", "Another error"));
        
        assert_eq!(storage.error_count(), 2);
        assert!(storage.last_error_time().is_some());
        
        // Test health status with multiple errors
        assert_eq!(storage.get_health_status(), "critical");
        
        // Test monitoring metrics include error context
        let metrics = storage.get_monitoring_metrics();
        assert_eq!(metrics["error_count"], 2);
        assert!(metrics["last_error_time"].is_string());
    }

    #[test]
    fn test_log_storage_warning_threshold() {
        let mut storage = LogStorage::new(100);
        
        // Add exactly 10 warnings (threshold is > 10 for warning status)
        for i in 0..10 {
            storage.add_entry(LogEntry::new(LogLevel::Warn, "test", format!("Warning {}", i)));
        }
        
        // Should still be healthy with exactly 10 warnings
        assert_eq!(storage.get_health_status(), "healthy");
        
        // Add one more warning to exceed threshold
        storage.add_entry(LogEntry::new(LogLevel::Warn, "test", "Warning 11"));
        
        // Should now be warning status
        assert_eq!(storage.get_health_status(), "warning");
    }
}
