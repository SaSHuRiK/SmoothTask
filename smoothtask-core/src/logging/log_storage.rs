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
use serde::{Serialize, Deserialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Уровень логирования, совместимый с tracing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LogLevel {
    /// Ошибки - критические проблемы, требующие немедленного внимания
    Error,
    /// Предупреждения - потенциальные проблемы или неоптимальные состояния
    Warn,
    /// Информация - информационные сообщения о нормальной работе
    Info,
    /// Отладка - отладочная информация
    Debug,
    /// Трейс - очень подробная отладочная информация
    Trace,
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
    /// Вектор записей логов (самые новые в конце)
    entries: Vec<LogEntry>,
}

impl LogStorage {
    /// Создаёт новое хранилище логов с указанным максимальным количеством записей.
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            entries: Vec::with_capacity(max_entries),
        }
    }

    /// Добавляет новую запись в хранилище.
    /// Если превышено максимальное количество записей, самая старая запись удаляется.
    pub fn add_entry(&mut self, entry: LogEntry) {
        if self.entries.len() >= self.max_entries && self.max_entries > 0 {
            self.entries.remove(0); // Удаляем самую старую запись
        }
        self.entries.push(entry);
    }

    /// Возвращает все записи логов.
    pub fn get_all_entries(&self) -> Vec<LogEntry> {
        self.entries.clone()
    }

    /// Возвращает записи логов, отфильтрованные по уровню.
    pub fn get_entries_by_level(&self, min_level: LogLevel) -> Vec<LogEntry> {
        self.entries.iter()
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

    /// Добавляет новую запись в хранилище.
    pub async fn add_entry(&self, entry: LogEntry) {
        let mut storage = self.inner.write().await;
        storage.add_entry(entry);
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
                    let meta = &meta_part[start_bracket+1..];
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
                    }
                }
            }
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
        let entry = LogEntry::new(
            LogLevel::Info,
            "test_module",
            "Test message"
        );

        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.target, "test_module");
        assert_eq!(entry.message, "Test message");
        assert!(entry.timestamp <= Utc::now());
    }

    #[test]
    fn test_log_entry_with_fields() {
        let mut entry = LogEntry::new(
            LogLevel::Debug,
            "test_module",
            "Debug message"
        );

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
                        format!("Message {}-{}", i, j)
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
}
