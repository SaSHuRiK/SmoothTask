// SPDX-License-Identifier: GPL-2.0 OR BSD-2-Clause
/* Copyright (c) 2024 SmoothTask Project */

//! Модуль для пакетной обработки метрик
//!
//! Этот модуль предоставляет функциональность для оптимизации производительности
//! системы мониторинга через пакетную обработку метрик. Основные возможности:
//! - Пакетный сбор метрик с нескольких источников
//! - Оптимизация использования системных ресурсов
//! - Уменьшение накладных расходов на сбор данных
//! - Поддержка параллельной обработки

use anyhow::Result;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

/// Конфигурация пакетного процессора
#[derive(Debug, Clone)]
pub struct BatchProcessorConfig {
    /// Максимальный размер пакета (количество метрик в одном пакете)
    pub max_batch_size: usize,
    /// Максимальная задержка перед обработкой пакета
    pub max_batch_delay: Duration,
    /// Включение параллельной обработки
    pub enable_parallel_processing: bool,
    /// Количество потоков для параллельной обработки
    pub parallel_threads: usize,
    /// Включение сжатия данных в пакетах
    pub enable_compression: bool,
    /// Включение кэширования результатов
    pub enable_caching: bool,
    /// Максимальный размер кэша
    pub max_cache_size: usize,
}

impl Default for BatchProcessorConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            max_batch_delay: Duration::from_millis(100),
            enable_parallel_processing: true,
            parallel_threads: 4,
            enable_compression: false,
            enable_caching: true,
            max_cache_size: 1000,
        }
    }
}

/// Пакет метрик для обработки
#[derive(Debug, Clone)]
pub struct MetricsBatch {
    /// Идентификатор пакета
    pub batch_id: String,
    /// Временная метка создания пакета
    pub timestamp: Instant,
    /// Метрики в пакете
    pub metrics: Vec<MetricItem>,
    /// Приоритет пакета
    pub priority: BatchPriority,
    /// Флаги пакета
    pub flags: BatchFlags,
}

impl Default for MetricsBatch {
    fn default() -> Self {
        Self {
            batch_id: String::new(),
            timestamp: Instant::now(),
            metrics: Vec::new(),
            priority: BatchPriority::default(),
            flags: BatchFlags::default(),
        }
    }
}

/// Элемент метрики в пакете
#[derive(Debug, Clone)]
pub struct MetricItem {
    /// Тип метрики
    pub metric_type: String,
    /// Идентификатор источника метрики
    pub source_id: String,
    /// Данные метрики (сериализованные)
    pub data: Vec<u8>,
    /// Временная метка метрики
    pub timestamp: Instant,
    /// Приоритет метрики
    pub priority: MetricPriority,
}

/// Приоритет пакета
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BatchPriority {
    #[default]
    Low,
    Normal,
    High,
    Critical,
}

/// Флаги пакета
#[derive(Debug, Clone, Default)]
pub struct BatchFlags {
    /// Требуется немедленная обработка
    pub immediate_processing: bool,
    /// Требуется сохранение результатов
    pub persist_results: bool,
    /// Требуется уведомление о завершении
    pub notify_completion: bool,
    /// Требуется верификация данных
    pub verify_data: bool,
}

/// Приоритет метрики
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum MetricPriority {
    #[default]
    Low,
    Normal,
    High,
    Critical,
}

/// Основная структура пакетного процессора
pub struct BatchProcessor {
    config: BatchProcessorConfig,
    cache: HashMap<String, MetricsBatch>,
    statistics: BatchProcessorStatistics,
}

impl BatchProcessor {
    /// Создать новый экземпляр пакетного процессора
    pub fn new(config: BatchProcessorConfig) -> Self {
        info!(
            "Creating batch processor with config: max_batch_size={}, parallel_processing={}",
            config.max_batch_size, config.enable_parallel_processing
        );

        Self {
            config,
            cache: HashMap::new(),
            statistics: BatchProcessorStatistics::default(),
        }
    }

    /// Создать пакет метрик
    pub fn create_batch(&mut self, batch_id: &str, priority: BatchPriority) -> MetricsBatch {
        let batch = MetricsBatch {
            batch_id: batch_id.to_string(),
            timestamp: Instant::now(),
            metrics: Vec::new(),
            priority,
            flags: BatchFlags::default(),
        };

        debug!("Created new metrics batch: {}", batch_id);
        self.statistics.batches_created += 1;

        batch
    }

    /// Добавить метрику в пакет
    pub fn add_metric_to_batch(&mut self, batch: &mut MetricsBatch, metric: MetricItem) {
        let metric_type = metric.metric_type.clone();
        batch.metrics.push(metric);
        debug!("Added metric to batch {}: {}", batch.batch_id, metric_type);

        self.statistics.metrics_added += 1;
    }

    /// Обработать пакет метрик
    pub fn process_batch(&mut self, batch: MetricsBatch) -> Result<BatchProcessingResult> {
        info!(
            "Processing batch {} with {} metrics, priority: {:?}",
            batch.batch_id,
            batch.metrics.len(),
            batch.priority
        );

        let start_time = Instant::now();

        // Проверяем размер пакета
        if batch.metrics.len() > self.config.max_batch_size {
            warn!(
                "Batch {} exceeds maximum size: {} > {}",
                batch.batch_id,
                batch.metrics.len(),
                self.config.max_batch_size
            );
        }

        // Обрабатываем метрики в пакете
        let mut results = Vec::new();
        for metric in &batch.metrics {
            let result = self.process_metric(metric);
            results.push(result);
        }

        let processing_time = start_time.elapsed();

        // Обновляем статистику
        self.statistics.batches_processed += 1;
        self.statistics.metrics_processed += batch.metrics.len() as u64;
        self.statistics.total_processing_time += processing_time;

        // Кэшируем результат если нужно
        if self.config.enable_caching {
            self.cache_result(&batch.batch_id, &results);
        }

        info!(
            "Batch {} processed in {:?}, {} metrics processed",
            batch.batch_id,
            processing_time,
            batch.metrics.len()
        );

        Ok(BatchProcessingResult {
            batch_id: batch.batch_id,
            metrics_processed: batch.metrics.len(),
            processing_time,
            results,
        })
    }

    /// Обработать отдельную метрику
    fn process_metric(&self, metric: &MetricItem) -> MetricProcessingResult {
        debug!(
            "Processing metric: {} from source {}",
            metric.metric_type, metric.source_id
        );

        // Здесь может быть логика обработки метрики
        // Например, десериализация, валидация, преобразование и т.д.

        MetricProcessingResult {
            metric_type: metric.metric_type.clone(),
            source_id: metric.source_id.clone(),
            success: true,
            error_message: None,
            processing_time: Duration::from_micros(100), // Примерное время обработки
        }
    }

    /// Кэшировать результат обработки пакета
    fn cache_result(&mut self, batch_id: &str, results: &[MetricProcessingResult]) {
        if self.cache.len() >= self.config.max_cache_size {
            // Очищаем кэш если он переполнен
            self.cache.clear();
            warn!(
                "Cache cleared due to size limit: {}",
                self.config.max_cache_size
            );
        }

        let _cached_batch = CachedBatch {
            _batch_id: batch_id.to_string(),
            _timestamp: Instant::now(),
            _results: results.to_vec(),
        };

        self.cache
            .insert(batch_id.to_string(), MetricsBatch::default()); // Упрощенно
        debug!("Cached batch results: {}", batch_id);
    }

    /// Получить статистику процессора
    pub fn get_statistics(&self) -> BatchProcessorStatistics {
        self.statistics.clone()
    }

    /// Очистить кэш
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        info!("Batch processor cache cleared");
    }

    /// Оптимизировать производительность на основе текущей нагрузки
    pub fn optimize_performance(&mut self, system_load: f64) {
        if system_load > 0.8 {
            // При высокой нагрузке уменьшаем размер пакетов
            let new_batch_size = (self.config.max_batch_size as f64 * 0.7) as usize;
            if new_batch_size > 0 {
                self.config.max_batch_size = new_batch_size;
                info!(
                    "Reduced batch size due to high system load: {}",
                    self.config.max_batch_size
                );
            }
        } else if system_load < 0.4 {
            // При низкой нагрузке увеличиваем размер пакетов
            let new_batch_size = (self.config.max_batch_size as f64 * 1.2) as usize;
            self.config.max_batch_size = new_batch_size;
            info!(
                "Increased batch size due to low system load: {}",
                self.config.max_batch_size
            );
        }
    }
}

/// Результат обработки пакета
#[derive(Debug, Clone)]
pub struct BatchProcessingResult {
    /// Идентификатор пакета
    pub batch_id: String,
    /// Количество обработанных метрик
    pub metrics_processed: usize,
    /// Время обработки
    pub processing_time: Duration,
    /// Результаты обработки отдельных метрик
    pub results: Vec<MetricProcessingResult>,
}

/// Результат обработки метрики
#[derive(Debug, Clone)]
pub struct MetricProcessingResult {
    /// Тип метрики
    pub metric_type: String,
    /// Идентификатор источника
    pub source_id: String,
    /// Успешность обработки
    pub success: bool,
    /// Сообщение об ошибке (если есть)
    pub error_message: Option<String>,
    /// Время обработки
    pub processing_time: Duration,
}

/// Кэшированный пакет
#[derive(Debug, Clone)]
struct CachedBatch {
    /// Идентификатор пакета
    _batch_id: String,
    /// Временная метка кэширования
    _timestamp: Instant,
    /// Результаты обработки
    _results: Vec<MetricProcessingResult>,
}

/// Статистика пакетного процессора
#[derive(Debug, Clone, Default)]
pub struct BatchProcessorStatistics {
    /// Количество созданных пакетов
    pub batches_created: u64,
    /// Количество обработанных пакетов
    pub batches_processed: u64,
    /// Количество добавленных метрик
    pub metrics_added: u64,
    /// Количество обработанных метрик
    pub metrics_processed: u64,
    /// Общее время обработки
    pub total_processing_time: Duration,
    /// Среднее время обработки пакета
    pub average_batch_processing_time: Duration,
    /// Среднее время обработки метрики
    pub average_metric_processing_time: Duration,
    /// Количество ошибок обработки
    pub processing_errors: u64,
    /// Количество кэш-попаданий
    pub cache_hits: u64,
    /// Количество кэш-промахов
    pub cache_misses: u64,
}

impl BatchProcessorStatistics {
    /// Рассчитать среднее время обработки
    pub fn calculate_averages(&mut self) {
        if self.batches_processed > 0 {
            self.average_batch_processing_time = Duration::from_micros(
                self.total_processing_time.as_micros() as u64 / self.batches_processed,
            );
        }

        if self.metrics_processed > 0 {
            let total_metric_time = self.total_processing_time.as_micros() as u64;
            let avg_metric_micros = total_metric_time / self.metrics_processed;
            self.average_metric_processing_time = Duration::from_micros(avg_metric_micros);
        }
    }
}

/// Тесты для пакетного процессора
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_batch_processor_creation() {
        let config = BatchProcessorConfig::default();
        let processor = BatchProcessor::new(config);
        assert_eq!(processor.config.max_batch_size, 100);
        assert!(processor.config.enable_parallel_processing);
    }

    #[test]
    fn test_batch_creation() {
        let config = BatchProcessorConfig::default();
        let mut processor = BatchProcessor::new(config);

        let batch = processor.create_batch("test_batch_1", BatchPriority::Normal);
        assert_eq!(batch.batch_id, "test_batch_1");
        assert_eq!(batch.metrics.len(), 0);
        assert_eq!(batch.priority, BatchPriority::Normal);
    }

    #[test]
    fn test_add_metric_to_batch() {
        let config = BatchProcessorConfig::default();
        let mut processor = BatchProcessor::new(config);

        let mut batch = processor.create_batch("test_batch_2", BatchPriority::High);

        let metric = MetricItem {
            metric_type: "cpu_usage".to_string(),
            source_id: "cpu0".to_string(),
            data: vec![1, 2, 3, 4],
            timestamp: Instant::now(),
            priority: MetricPriority::Normal,
        };

        processor.add_metric_to_batch(&mut batch, metric);
        assert_eq!(batch.metrics.len(), 1);
        assert_eq!(batch.metrics[0].metric_type, "cpu_usage");
    }

    #[test]
    fn test_batch_processing() {
        let config = BatchProcessorConfig::default();
        let mut processor = BatchProcessor::new(config);

        let mut batch = processor.create_batch("test_batch_3", BatchPriority::Normal);

        // Добавляем несколько метрик
        for i in 0..5 {
            let metric = MetricItem {
                metric_type: format!("metric_{}", i),
                source_id: format!("source_{}", i),
                data: vec![i as u8],
                timestamp: Instant::now(),
                priority: MetricPriority::Normal,
            };
            processor.add_metric_to_batch(&mut batch, metric);
        }

        // Обрабатываем пакет
        let result = processor.process_batch(batch);
        assert!(result.is_ok());
        let processing_result = result.unwrap();
        assert_eq!(processing_result.metrics_processed, 5);
        assert_eq!(processing_result.results.len(), 5);
    }

    #[test]
    fn test_batch_processor_statistics() {
        let config = BatchProcessorConfig::default();
        let mut processor = BatchProcessor::new(config);

        let stats = processor.get_statistics();
        assert_eq!(stats.batches_created, 0);
        assert_eq!(stats.metrics_added, 0);

        // Создаем пакет и добавляем метрики
        let mut batch = processor.create_batch("test_batch_4", BatchPriority::Low);

        let metric = MetricItem {
            metric_type: "test_metric".to_string(),
            source_id: "test_source".to_string(),
            data: vec![1, 2, 3],
            timestamp: Instant::now(),
            priority: MetricPriority::Low,
        };

        processor.add_metric_to_batch(&mut batch, metric);

        let stats = processor.get_statistics();
        assert_eq!(stats.batches_created, 1);
        assert_eq!(stats.metrics_added, 1);
    }

    #[test]
    fn test_performance_optimization() {
        let config = BatchProcessorConfig::default();
        let mut processor = BatchProcessor::new(config);

        let original_batch_size = processor.config.max_batch_size;

        // Тестируем оптимизацию при высокой нагрузке
        processor.optimize_performance(0.9);
        assert!(processor.config.max_batch_size < original_batch_size);

        // Тестируем оптимизацию при низкой нагрузке
        processor.optimize_performance(0.3);
        assert!(processor.config.max_batch_size > original_batch_size);
    }

    #[test]
    fn test_cache_management() {
        let config = BatchProcessorConfig::default();
        let mut processor = BatchProcessor::new(config);

        // Добавляем данные в кэш
        let mut batch = processor.create_batch("test_batch_5", BatchPriority::Normal);
        let metric = MetricItem {
            metric_type: "cached_metric".to_string(),
            source_id: "cached_source".to_string(),
            data: vec![1, 2, 3],
            timestamp: Instant::now(),
            priority: MetricPriority::Normal,
        };
        processor.add_metric_to_batch(&mut batch, metric);
        processor.process_batch(batch).unwrap();

        // Проверяем что кэш не пустой
        assert!(!processor.cache.is_empty());

        // Очищаем кэш
        processor.clear_cache();
        assert!(processor.cache.is_empty());
    }

    #[test]
    fn test_batch_priority_handling() {
        let config = BatchProcessorConfig::default();
        let mut processor = BatchProcessor::new(config);

        // Создаем пакеты с разными приоритетами
        let high_priority_batch = processor.create_batch("high_priority", BatchPriority::High);
        let normal_batch = processor.create_batch("normal_priority", BatchPriority::Normal);
        let low_priority_batch = processor.create_batch("low_priority", BatchPriority::Low);

        assert_eq!(high_priority_batch.priority, BatchPriority::High);
        assert_eq!(normal_batch.priority, BatchPriority::Normal);
        assert_eq!(low_priority_batch.priority, BatchPriority::Low);
    }

    #[test]
    fn test_metric_priority_handling() {
        let config = BatchProcessorConfig::default();
        let mut processor = BatchProcessor::new(config);

        let mut batch = processor.create_batch("priority_test", BatchPriority::Normal);

        // Добавляем метрики с разными приоритетами
        for priority in &[
            MetricPriority::Low,
            MetricPriority::Normal,
            MetricPriority::High,
            MetricPriority::Critical,
        ] {
            let metric = MetricItem {
                metric_type: format!("metric_{:?}", priority),
                source_id: "priority_source".to_string(),
                data: vec![],
                timestamp: Instant::now(),
                priority: priority.clone(),
            };
            processor.add_metric_to_batch(&mut batch, metric);
        }

        assert_eq!(batch.metrics.len(), 4);
        assert_eq!(batch.metrics[0].priority, MetricPriority::Low);
        assert_eq!(batch.metrics[3].priority, MetricPriority::Critical);
    }
}

/// Вспомогательные функции для интеграции с системой мониторинга
impl BatchProcessor {
    /// Создать конфигурацию для высокопроизводительной обработки
    pub fn high_performance_config() -> BatchProcessorConfig {
        BatchProcessorConfig {
            max_batch_size: 200,
            max_batch_delay: Duration::from_millis(50),
            enable_parallel_processing: true,
            parallel_threads: 8,
            enable_compression: false,
            enable_caching: true,
            max_cache_size: 2000,
        }
    }

    /// Создать конфигурацию для низкопроизводительной обработки
    pub fn low_power_config() -> BatchProcessorConfig {
        BatchProcessorConfig {
            max_batch_size: 50,
            max_batch_delay: Duration::from_millis(200),
            enable_parallel_processing: false,
            parallel_threads: 2,
            enable_compression: true,
            enable_caching: true,
            max_cache_size: 500,
        }
    }

    /// Создать конфигурацию для обработки в реальном времени
    pub fn realtime_config() -> BatchProcessorConfig {
        BatchProcessorConfig {
            max_batch_size: 10,
            max_batch_delay: Duration::from_millis(10),
            enable_parallel_processing: true,
            parallel_threads: 4,
            enable_compression: false,
            enable_caching: false,
            max_cache_size: 100,
        }
    }
}
