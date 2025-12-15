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

    /// Создаёт запись лога в формате JSON (структурированное логирование).
    ///
    /// # Аргументы
    ///
    /// * `level` - уровень логирования
    /// * `target` - модуль или компонент
    /// * `message` - основное сообщение
    /// * `json_data` - дополнительные данные в формате JSON
    ///
    /// # Возвращает
    ///
    /// Новая запись лога с JSON-полями
    pub fn new_json(
        level: LogLevel,
        target: impl Into<String>,
        message: impl Into<String>,
        json_data: serde_json::Value,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            target: target.into(),
            message: message.into(),
            fields: Some(json_data),
        }
    }

    /// Создаёт запись лога в формате Key-Value (структурированное логирование).
    ///
    /// # Аргументы
    ///
    /// * `level` - уровень логирования
    /// * `target` - модуль или компонент
    /// * `message` - основное сообщение
    /// * `key_value_pairs` - пары ключ-значение для структурированного логирования
    ///
    /// # Возвращает
    ///
    /// Новая запись лога с полями в формате Key-Value
    pub fn new_key_value(
        level: LogLevel,
        target: impl Into<String>,
        message: impl Into<String>,
        key_value_pairs: HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            target: target.into(),
            message: message.into(),
            fields: Some(serde_json::to_value(key_value_pairs).unwrap_or(serde_json::Value::Null)),
        }
    }

    /// Преобразует запись лога в формат JSON.
    ///
    /// # Возвращает
    ///
    /// JSON-представление записи лога
    pub fn to_json(&self) -> serde_json::Value {
        let mut log_json = json!({
            "timestamp": self.timestamp.to_rfc3339(),
            "level": format!("{}", self.level),
            "target": self.target,
            "message": self.message,
        });

        if let Some(fields) = &self.fields {
            log_json["fields"] = fields.clone();
        }

        log_json
    }

    /// Преобразует запись лога в формат Key-Value.
    ///
    /// # Возвращает
    ///
    /// HashMap с парами ключ-значение
    pub fn to_key_value(&self) -> HashMap<String, serde_json::Value> {
        let mut map = HashMap::new();
        map.insert(
            "timestamp".to_string(),
            serde_json::Value::String(self.timestamp.to_rfc3339()),
        );
        map.insert(
            "level".to_string(),
            serde_json::Value::String(format!("{}", self.level)),
        );
        map.insert(
            "target".to_string(),
            serde_json::Value::String(self.target.clone()),
        );
        map.insert(
            "message".to_string(),
            serde_json::Value::String(self.message.clone()),
        );

        if let Some(fields) = &self.fields {
            if let serde_json::Value::Object(fields_map) = fields {
                for (key, value) in fields_map {
                    map.insert(key.clone(), value.clone());
                }
            }
        }

        map
    }

    /// Проверяет, содержит ли запись лога структурированные данные.
    ///
    /// # Возвращает
    ///
    /// `true`, если запись содержит структурированные данные, `false` в противном случае
    pub fn has_structured_data(&self) -> bool {
        self.fields.is_some()
    }

    /// Извлекает значение из структурированных данных по ключу.
    ///
    /// # Аргументы
    ///
    /// * `key` - ключ для поиска
    ///
    /// # Возвращает
    ///
    /// `Option<&serde_json::Value>` - значение, если найдено, иначе `None`
    pub fn get_field(&self, key: &str) -> Option<&serde_json::Value> {
        self.fields.as_ref().and_then(|fields| fields.get(key))
    }

    /// Проверяет, содержит ли запись лога поле с указанным ключом.
    ///
    /// # Аргументы
    ///
    /// * `key` - ключ для проверки
    ///
    /// # Возвращает
    ///
    /// `true`, если поле существует, `false` в противном случае
    pub fn has_field(&self, key: &str) -> bool {
        self.fields
            .as_ref()
            .map_or(false, |fields| fields.get(key).is_some())
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
    /// Флаг включения асинхронного логирования
    async_logging_enabled: bool,
    /// Размер batches для асинхронного логирования
    batch_size: usize,
    /// Текущий batch для асинхронного логирования
    current_batch: Vec<LogEntry>,
    /// Счётчик операций логирования
    operation_count: usize,
    /// Общее время, затраченное на операции логирования (в микросекундах)
    total_logging_time_us: u64,
    /// Максимальное время операции логирования (в микросекундах)
    max_logging_time_us: u64,
    /// Время последней операции очистки (в микросекундах)
    last_cleanup_time_us: Option<u64>,
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
            async_logging_enabled: false,
            batch_size: 100,
            current_batch: Vec::with_capacity(100),
            operation_count: 0,
            total_logging_time_us: 0,
            max_logging_time_us: 0,
            last_cleanup_time_us: None,
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
        Self::new_with_rotation_and_batch_size(max_entries, max_age_seconds, 100)
    }

    /// Создаёт новое хранилище логов с конфигурацией ротации и размером batches.
    ///
    /// # Аргументы
    ///
    /// * `max_entries` - максимальное количество хранимых записей
    /// * `max_age_seconds` - максимальный возраст записей в секундах (0 = без ограничения)
    /// * `batch_size` - размер batches для асинхронного логирования
    ///
    /// # Возвращает
    ///
    /// Новый экземпляр LogStorage
    pub fn new_with_rotation_and_batch_size(
        max_entries: usize,
        max_age_seconds: u64,
        batch_size: usize,
    ) -> Self {
        Self {
            max_entries,
            max_age_seconds,
            entries: Vec::with_capacity(max_entries),
            error_count: 0,
            warning_count: 0,
            last_error_time: None,
            last_cleanup_time: None,
            async_logging_enabled: false,
            batch_size,
            current_batch: Vec::with_capacity(batch_size),
            operation_count: 0,
            total_logging_time_us: 0,
            max_logging_time_us: 0,
            last_cleanup_time_us: None,
        }
    }

    /// Включает асинхронное логирование.
    pub fn enable_async_logging(&mut self) {
        self.async_logging_enabled = true;
    }

    /// Отключает асинхронное логирование.
    pub fn disable_async_logging(&mut self) {
        self.async_logging_enabled = false;
        // Очищаем текущий batch
        self.current_batch.clear();
    }

    /// Проверяет, включено ли асинхронное логирование.
    pub fn is_async_logging_enabled(&self) -> bool {
        self.async_logging_enabled
    }

    /// Возвращает текущий размер batches.
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// Устанавливает размер batches.
    pub fn set_batch_size(&mut self, batch_size: usize) {
        self.batch_size = batch_size;
        self.current_batch.reserve(batch_size);
    }

    /// Возвращает количество операций логирования.
    pub fn operation_count(&self) -> usize {
        self.operation_count
    }

    /// Возвращает среднее время операции логирования в микросекундах.
    pub fn average_logging_time_us(&self) -> f64 {
        if self.operation_count > 0 {
            self.total_logging_time_us as f64 / self.operation_count as f64
        } else {
            0.0
        }
    }

    /// Возвращает максимальное время операции логирования в микросекундах.
    pub fn max_logging_time_us(&self) -> u64 {
        self.max_logging_time_us
    }

    /// Возвращает общее время, затраченное на операции логирования в микросекундах.
    pub fn total_logging_time_us(&self) -> u64 {
        self.total_logging_time_us
    }

    /// Возвращает время последней операции очистки в микросекундах.
    pub fn last_cleanup_time_us(&self) -> Option<u64> {
        self.last_cleanup_time_us
    }

    /// Сбрасывает счётчики производительности.
    pub fn reset_performance_counters(&mut self) {
        self.operation_count = 0;
        self.total_logging_time_us = 0;
        self.max_logging_time_us = 0;
        self.last_cleanup_time_us = None;
    }

    /// Добавляет новую запись в хранилище.
    /// Если превышено максимальное количество записей, самая старая запись удаляется.
    /// Также выполняет очистку старых записей, если включено ограничение по возрасту.
    pub fn add_entry(&mut self, entry: LogEntry) {
        let start_time = std::time::Instant::now();

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

        // Проверяем, включено ли асинхронное логирование
        if self.async_logging_enabled {
            self.current_batch.push(entry);

            // Если batch заполнен, выполняем flush
            if self.current_batch.len() >= self.batch_size {
                self.flush_batch();
            }
        } else {
            // Выполняем очистку старых записей, если включено ограничение по возрасту
            self.cleanup_old_entries();

            // Применяем ограничение по количеству записей
            if self.entries.len() >= self.max_entries && self.max_entries > 0 {
                self.entries.remove(0); // Удаляем самую старую запись
            }
            self.entries.push(entry);
        }

        // Обновляем счётчики производительности
        let elapsed = start_time.elapsed();
        let elapsed_us = elapsed.as_micros() as u64;
        self.operation_count += 1;
        self.total_logging_time_us += elapsed_us;
        if elapsed_us > self.max_logging_time_us {
            self.max_logging_time_us = elapsed_us;
        }
    }

    /// Выполняет flush текущего batches.
    pub fn flush_batch(&mut self) {
        if self.current_batch.is_empty() {
            return;
        }

        let start_time = std::time::Instant::now();

        // Выполняем очистку старых записей перед добавлением нового batches
        self.cleanup_old_entries();

        // Применяем ограничение по количеству записей
        while self.entries.len() + self.current_batch.len() > self.max_entries
            && self.max_entries > 0
        {
            let to_remove = self.entries.len() + self.current_batch.len() - self.max_entries;
            self.entries.drain(0..to_remove.min(self.entries.len()));
        }

        // Добавляем batch в основной вектор
        self.entries.extend(self.current_batch.drain(..));

        // Обновляем счётчики производительности для операции очистки
        let elapsed = start_time.elapsed();
        self.last_cleanup_time_us = Some(elapsed.as_micros() as u64);
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
        let start_time = std::time::Instant::now();

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

        // Проверяем, включено ли асинхронное логирование
        if self.async_logging_enabled {
            self.current_batch.push(entry);

            // Если batch заполнен, выполняем flush
            if self.current_batch.len() >= self.batch_size {
                self.flush_batch_with_memory_check(memory_limit_bytes);
            }
        } else {
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

        // Обновляем счётчики производительности
        let elapsed = start_time.elapsed();
        let elapsed_us = elapsed.as_micros() as u64;
        self.operation_count += 1;
        self.total_logging_time_us += elapsed_us;
        if elapsed_us > self.max_logging_time_us {
            self.max_logging_time_us = elapsed_us;
        }
    }

    /// Выполняет flush текущего batches с проверкой памяти.
    pub fn flush_batch_with_memory_check(&mut self, memory_limit_bytes: usize) {
        if self.current_batch.is_empty() {
            return;
        }

        let start_time = std::time::Instant::now();

        // Выполняем очистку старых записей перед добавлением нового batches
        self.cleanup_old_entries();

        // Выполняем очистку на основе использования памяти
        self.cleanup_by_memory(memory_limit_bytes);

        // Применяем ограничение по количеству записей
        while self.entries.len() + self.current_batch.len() > self.max_entries
            && self.max_entries > 0
        {
            let to_remove = self.entries.len() + self.current_batch.len() - self.max_entries;
            self.entries.drain(0..to_remove.min(self.entries.len()));
        }

        // Добавляем batch в основной вектор
        self.entries.extend(self.current_batch.drain(..));

        // Обновляем счётчики производительности для операции очистки
        let elapsed = start_time.elapsed();
        self.last_cleanup_time_us = Some(elapsed.as_micros() as u64);
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
        let first_valid_index = self
            .entries
            .iter()
            .position(|entry| entry.timestamp >= cutoff_time);

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

    /// Возвращает записи логов, содержащие структурированные данные.
    pub fn get_entries_with_structured_data(&self) -> Vec<LogEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.has_structured_data())
            .cloned()
            .collect()
    }

    /// Возвращает записи логов, содержащие указанное поле в структурированных данных.
    ///
    /// # Аргументы
    ///
    /// * `field_name` - имя поля для поиска
    ///
    /// # Возвращает
    ///
    /// Вектор записей логов, содержащих указанное поле
    pub fn get_entries_with_field(&self, field_name: &str) -> Vec<LogEntry> {
        self.entries
            .iter()
            .filter(|entry| entry.has_field(field_name))
            .cloned()
            .collect()
    }

    /// Фильтрует записи логов по значению поля в структурированных данных.
    ///
    /// # Аргументы
    ///
    /// * `field_name` - имя поля для фильтрации
    /// * `field_value` - значение поля для сравнения
    ///
    /// # Возвращает
    ///
    /// Вектор записей логов, соответствующих фильтру
    pub fn filter_entries_by_field_value(
        &self,
        field_name: &str,
        field_value: &serde_json::Value,
    ) -> Vec<LogEntry> {
        self.entries
            .iter()
            .filter(|entry| {
                entry
                    .get_field(field_name)
                    .map_or(false, |value| value == field_value)
            })
            .cloned()
            .collect()
    }

    /// Поиск записей логов по ключевому слову в сообщении или структурированных данных.
    ///
    /// # Аргументы
    ///
    /// * `keyword` - ключевое слово для поиска
    ///
    /// # Возвращает
    ///
    /// Вектор записей логов, содержащих ключевое слово
    pub fn search_entries(&self, keyword: &str) -> Vec<LogEntry> {
        let keyword_lower = keyword.to_lowercase();
        self.entries
            .iter()
            .filter(|entry| {
                // Поиск в сообщении
                let message_match = entry.message.to_lowercase().contains(&keyword_lower);

                // Поиск в структурированных данных
                let fields_match = entry.fields.as_ref().map_or(false, |fields| {
                    fields.as_object().map_or(false, |obj| {
                        obj.values().any(|value| {
                            value
                                .as_str()
                                .map_or(false, |s| s.to_lowercase().contains(&keyword_lower))
                        })
                    })
                });

                message_match || fields_match
            })
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
            if self
                .last_error_time
                .is_some_and(|t| t > Utc::now() - chrono::Duration::minutes(5))
            {
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

/// Статистика производительности логирования.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LogPerformanceStats {
    /// Количество операций логирования
    pub operation_count: usize,
    /// Среднее время операции логирования в микросекундах
    pub average_logging_time_us: f64,
    /// Максимальное время операции логирования в микросекундах
    pub max_logging_time_us: u64,
    /// Общее время, затраченное на операции логирования в микросекундах
    pub total_logging_time_us: u64,
    /// Время последней операции очистки в микросекундах
    pub last_cleanup_time_us: Option<u64>,
    /// Включено ли асинхронное логирование
    pub async_logging_enabled: bool,
    /// Размер batches
    pub batch_size: usize,
    /// Текущий размер batches
    pub current_batch_size: usize,
}

impl Default for LogPerformanceStats {
    fn default() -> Self {
        Self {
            operation_count: 0,
            average_logging_time_us: 0.0,
            max_logging_time_us: 0,
            total_logging_time_us: 0,
            last_cleanup_time_us: None,
            async_logging_enabled: false,
            batch_size: 100,
            current_batch_size: 0,
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
            inner: Arc::new(RwLock::new(LogStorage::new_with_rotation(
                max_entries,
                max_age_seconds,
            ))),
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

    /// Возвращает записи логов, содержащие структурированные данные.
    pub async fn get_entries_with_structured_data(&self) -> Vec<LogEntry> {
        let storage = self.inner.read().await;
        storage.get_entries_with_structured_data()
    }

    /// Возвращает записи логов, содержащие указанное поле в структурированных данных.
    ///
    /// # Аргументы
    ///
    /// * `field_name` - имя поля для поиска
    ///
    /// # Возвращает
    ///
    /// Вектор записей логов, содержащих указанное поле
    pub async fn get_entries_with_field(&self, field_name: &str) -> Vec<LogEntry> {
        let storage = self.inner.read().await;
        storage.get_entries_with_field(field_name)
    }

    /// Фильтрует записи логов по значению поля в структурированных данных.
    ///
    /// # Аргументы
    ///
    /// * `field_name` - имя поля для фильтрации
    /// * `field_value` - значение поля для сравнения
    ///
    /// # Возвращает
    ///
    /// Вектор записей логов, соответствующих фильтру
    pub async fn filter_entries_by_field_value(
        &self,
        field_name: &str,
        field_value: serde_json::Value,
    ) -> Vec<LogEntry> {
        let storage = self.inner.read().await;
        storage.filter_entries_by_field_value(field_name, &field_value)
    }

    /// Поиск записей логов по ключевому слову в сообщении или структурированных данных.
    ///
    /// # Аргументы
    ///
    /// * `keyword` - ключевое слово для поиска
    ///
    /// # Возвращает
    ///
    /// Вектор записей логов, содержащих ключевое слово
    pub async fn search_entries(&self, keyword: &str) -> Vec<LogEntry> {
        let storage = self.inner.read().await;
        storage.search_entries(keyword)
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
                    eprintln!(
                        "Не удалось разобрать сообщение tracing: не найдена открывающая скобка '['"
                    );
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
            storage.add_entry(LogEntry::new(
                LogLevel::Info,
                "test",
                format!("Message {}", i),
            ));
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

        storage
            .add_entry(LogEntry::new(LogLevel::Info, "test", "Message 1"))
            .await;
        storage
            .add_entry(LogEntry::new(LogLevel::Info, "test", "Message 2"))
            .await;

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
            storage.add_entry(LogEntry::new(
                LogLevel::Info,
                "test",
                format!("Message {}", i),
            ));
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
            storage.add_entry(LogEntry::new(
                LogLevel::Info,
                "test",
                format!("Message {}", i),
            ));
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
                1000,
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
            storage
                .add_entry(LogEntry::new(
                    LogLevel::Info,
                    "test",
                    format!("Message {}", i),
                ))
                .await;
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
            storage
                .add_entry_with_memory_check(
                    LogEntry::new(LogLevel::Info, "test", format!("Message {}", i)),
                    1000,
                )
                .await;
        }

        // Должно остаться около 4 записей (может быть немного больше из-за точности оценки)
        assert!(storage.len().await <= 6);
    }

    #[test]
    fn test_log_storage_monitoring_metrics_with_memory() {
        let mut storage = LogStorage::new(100);

        // Добавляем записи
        for i in 0..10 {
            storage.add_entry(LogEntry::new(
                LogLevel::Info,
                "test",
                format!("Message {}", i),
            ));
        }

        let metrics = storage.get_monitoring_metrics();

        assert_eq!(metrics["total_entries"], 10);
        assert!(metrics["estimated_memory_usage_bytes"].as_u64().unwrap() >= 2000);
        assert!(metrics["estimated_memory_usage_bytes"].as_u64().unwrap() <= 3000);
    }

    #[tokio::test]
    async fn test_shared_log_storage_monitoring() {
        let storage = SharedLogStorage::new(100);

        storage
            .add_entry(LogEntry::new(LogLevel::Error, "test", "Error"))
            .await;
        storage
            .add_entry(LogEntry::new(LogLevel::Warn, "test", "Warning"))
            .await;

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
        storage.add_entry(LogEntry::new(
            LogLevel::Error,
            "module1",
            "Critical error occurred",
        ));
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
            storage.add_entry(LogEntry::new(
                LogLevel::Warn,
                "test",
                format!("Warning {}", i),
            ));
        }

        // Should still be healthy with exactly 10 warnings
        assert_eq!(storage.get_health_status(), "healthy");

        // Add one more warning to exceed threshold
        storage.add_entry(LogEntry::new(LogLevel::Warn, "test", "Warning 11"));

        // Should now be warning status
        assert_eq!(storage.get_health_status(), "warning");
    }

    #[test]
    fn test_log_entry_structured_json() {
        // Тестируем создание записи лога с JSON-данными
        let json_data = json!({
            "request_id": "12345",
            "user_id": "user123",
            "duration_ms": 150
        });

        let entry = LogEntry::new_json(
            LogLevel::Info,
            "api.request",
            "Request processed successfully",
            json_data.clone(),
        );

        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.target, "api.request");
        assert_eq!(entry.message, "Request processed successfully");
        assert!(entry.has_structured_data());
        assert_eq!(
            entry.get_field("request_id").unwrap(),
            &json_data["request_id"]
        );
        assert_eq!(entry.get_field("user_id").unwrap(), &json_data["user_id"]);
        assert_eq!(
            entry.get_field("duration_ms").unwrap(),
            &json_data["duration_ms"]
        );
    }

    #[test]
    fn test_log_entry_structured_key_value() {
        // Тестируем создание записи лога с парами ключ-значение
        let mut key_value_pairs = HashMap::new();
        key_value_pairs.insert(
            "request_id".to_string(),
            serde_json::Value::String("12345".to_string()),
        );
        key_value_pairs.insert(
            "user_id".to_string(),
            serde_json::Value::String("user123".to_string()),
        );
        key_value_pairs.insert(
            "duration_ms".to_string(),
            serde_json::Value::Number(serde_json::Number::from(150)),
        );

        let entry = LogEntry::new_key_value(
            LogLevel::Info,
            "api.request",
            "Request processed successfully",
            key_value_pairs.clone(),
        );

        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.target, "api.request");
        assert_eq!(entry.message, "Request processed successfully");
        assert!(entry.has_structured_data());
        assert!(entry.has_field("request_id"));
        assert!(entry.has_field("user_id"));
        assert!(entry.has_field("duration_ms"));
    }

    #[test]
    fn test_log_entry_to_json() {
        // Тестируем преобразование записи лога в JSON
        let mut key_value_pairs = HashMap::new();
        key_value_pairs.insert(
            "request_id".to_string(),
            serde_json::Value::String("12345".to_string()),
        );
        key_value_pairs.insert(
            "user_id".to_string(),
            serde_json::Value::String("user123".to_string()),
        );

        let entry = LogEntry::new_key_value(
            LogLevel::Info,
            "api.request",
            "Request processed successfully",
            key_value_pairs,
        );

        let json_output = entry.to_json();

        assert!(json_output["timestamp"].is_string());
        assert_eq!(json_output["level"], "INFO");
        assert_eq!(json_output["target"], "api.request");
        assert_eq!(json_output["message"], "Request processed successfully");
        assert!(json_output["fields"].is_object());
        assert_eq!(json_output["fields"]["request_id"], "12345");
        assert_eq!(json_output["fields"]["user_id"], "user123");
    }

    #[test]
    fn test_log_entry_to_key_value() {
        // Тестируем преобразование записи лога в формат Key-Value
        let mut key_value_pairs = HashMap::new();
        key_value_pairs.insert(
            "request_id".to_string(),
            serde_json::Value::String("12345".to_string()),
        );
        key_value_pairs.insert(
            "user_id".to_string(),
            serde_json::Value::String("user123".to_string()),
        );

        let entry = LogEntry::new_key_value(
            LogLevel::Info,
            "api.request",
            "Request processed successfully",
            key_value_pairs,
        );

        let key_value_output = entry.to_key_value();

        assert!(key_value_output.contains_key("timestamp"));
        assert_eq!(key_value_output["level"], "INFO");
        assert_eq!(key_value_output["target"], "api.request");
        assert_eq!(
            key_value_output["message"],
            "Request processed successfully"
        );
        assert_eq!(key_value_output["request_id"], "12345");
        assert_eq!(key_value_output["user_id"], "user123");
    }

    #[test]
    fn test_log_storage_structured_filtering() {
        // Тестируем фильтрацию структурированных логов
        let mut storage = LogStorage::new(100);

        // Добавляем обычные записи
        storage.add_entry(LogEntry::new(LogLevel::Info, "test", "Normal log entry 1"));
        storage.add_entry(LogEntry::new(LogLevel::Info, "test", "Normal log entry 2"));

        // Добавляем структурированные записи
        let mut fields1 = HashMap::new();
        fields1.insert(
            "request_id".to_string(),
            serde_json::Value::String("req123".to_string()),
        );
        storage.add_entry(LogEntry::new_key_value(
            LogLevel::Info,
            "api",
            "Structured log 1",
            fields1,
        ));

        let mut fields2 = HashMap::new();
        fields2.insert(
            "request_id".to_string(),
            serde_json::Value::String("req456".to_string()),
        );
        fields2.insert(
            "user_id".to_string(),
            serde_json::Value::String("user789".to_string()),
        );
        storage.add_entry(LogEntry::new_key_value(
            LogLevel::Info,
            "api",
            "Structured log 2",
            fields2,
        ));

        // Проверяем фильтрацию по структурированным данным
        let structured_entries = storage.get_entries_with_structured_data();
        assert_eq!(structured_entries.len(), 2);
        assert!(structured_entries
            .iter()
            .all(|entry| entry.has_structured_data()));

        // Проверяем фильтрацию по полю
        let entries_with_request_id = storage.get_entries_with_field("request_id");
        assert_eq!(entries_with_request_id.len(), 2);

        let entries_with_user_id = storage.get_entries_with_field("user_id");
        assert_eq!(entries_with_user_id.len(), 1);
    }

    #[test]
    fn test_log_storage_field_value_filtering() {
        // Тестируем фильтрацию по значению поля
        let mut storage = LogStorage::new(100);

        let mut fields1 = HashMap::new();
        fields1.insert(
            "request_id".to_string(),
            serde_json::Value::String("req123".to_string()),
        );
        storage.add_entry(LogEntry::new_key_value(
            LogLevel::Info,
            "api",
            "Request 1",
            fields1,
        ));

        let mut fields2 = HashMap::new();
        fields2.insert(
            "request_id".to_string(),
            serde_json::Value::String("req456".to_string()),
        );
        storage.add_entry(LogEntry::new_key_value(
            LogLevel::Info,
            "api",
            "Request 2",
            fields2,
        ));

        let mut fields3 = HashMap::new();
        fields3.insert(
            "request_id".to_string(),
            serde_json::Value::String("req123".to_string()),
        );
        storage.add_entry(LogEntry::new_key_value(
            LogLevel::Info,
            "api",
            "Request 3",
            fields3,
        ));

        // Фильтруем по значению "req123"
        let filtered_entries = storage.filter_entries_by_field_value(
            "request_id",
            &serde_json::Value::String("req123".to_string()),
        );

        assert_eq!(filtered_entries.len(), 2);
        assert!(filtered_entries
            .iter()
            .all(|entry| entry.get_field("request_id").unwrap() == "req123"));
    }

    #[test]
    fn test_log_storage_search_functionality() {
        // Тестируем функцию поиска
        let mut storage = LogStorage::new(100);

        // Добавляем записи с разными сообщениями и структурированными данными
        storage.add_entry(LogEntry::new(
            LogLevel::Info,
            "test",
            "Error occurred during processing",
        ));
        storage.add_entry(LogEntry::new(
            LogLevel::Warn,
            "test",
            "Warning: high memory usage",
        ));

        let mut fields1 = HashMap::new();
        fields1.insert(
            "error_code".to_string(),
            serde_json::Value::String("E001".to_string()),
        );
        fields1.insert(
            "details".to_string(),
            serde_json::Value::String("Database connection failed".to_string()),
        );
        storage.add_entry(LogEntry::new_key_value(
            LogLevel::Error,
            "db",
            "Database error",
            fields1,
        ));

        let mut fields2 = HashMap::new();
        fields2.insert(
            "error_code".to_string(),
            serde_json::Value::String("E002".to_string()),
        );
        fields2.insert(
            "details".to_string(),
            serde_json::Value::String("Network timeout".to_string()),
        );
        storage.add_entry(LogEntry::new_key_value(
            LogLevel::Error,
            "network",
            "Network error",
            fields2,
        ));

        // Поиск по ключевому слову "error"
        let error_results = storage.search_entries("error");
        assert_eq!(error_results.len(), 3); // "Error occurred", "Database error", "Network error"

        // Поиск по ключевому слову "database"
        let database_results = storage.search_entries("database");
        assert_eq!(database_results.len(), 1); // Только "Database connection failed"

        // Поиск по ключевому слову "E001"
        let e001_results = storage.search_entries("E001");
        assert_eq!(e001_results.len(), 1); // Только запись с error_code: "E001"
    }

    #[test]
    fn test_structured_logging_integration() {
        // Тестируем интеграцию структурированного логирования
        let mut storage = LogStorage::new(100);

        // Создаем структурированные логи разных типов
        let json_log = LogEntry::new_json(
            LogLevel::Info,
            "api.request",
            "Request completed",
            json!({
                "request_id": "abc123",
                "status": "success",
                "duration_ms": 250
            }),
        );

        let mut kv_pairs = HashMap::new();
        kv_pairs.insert(
            "operation".to_string(),
            serde_json::Value::String("database_query".to_string()),
        );
        kv_pairs.insert(
            "rows_affected".to_string(),
            serde_json::Value::Number(serde_json::Number::from(42)),
        );
        let kv_log =
            LogEntry::new_key_value(LogLevel::Debug, "db.operation", "Query executed", kv_pairs);

        // Добавляем логи в хранилище
        storage.add_entry(json_log);
        storage.add_entry(kv_log);

        // Проверяем, что логи успешно добавлены
        assert_eq!(storage.len(), 2);

        // Проверяем фильтрацию
        let structured_logs = storage.get_entries_with_structured_data();
        assert_eq!(structured_logs.len(), 2);

        // Проверяем поиск
        let request_logs = storage.search_entries("request");
        assert_eq!(request_logs.len(), 1);

        let database_logs = storage.search_entries("database");
        assert_eq!(database_logs.len(), 1);

        // Проверяем преобразование в JSON
        let all_entries = storage.get_all_entries();
        for entry in all_entries {
            let json_output = entry.to_json();
            assert!(json_output["timestamp"].is_string());
            assert!(json_output["level"].is_string());
            assert!(json_output["target"].is_string());
            assert!(json_output["message"].is_string());

            if entry.has_structured_data() {
                assert!(json_output["fields"].is_object());
            }
        }
    }
}
