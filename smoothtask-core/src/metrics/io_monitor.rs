//! Модуль расширенного мониторинга ввода-вывода
//!
//! Этот модуль предоставляет расширенные функции для мониторинга и анализа
//! операций ввода-вывода в системе. Основные возможности:
//! - Мониторинг операций ввода-вывода на уровне системы
//! - Анализ производительности дисков и файловой системы
//! - Отслеживание операций ввода-вывода отдельных процессов
//! - Обнаружение узких мест в подсистеме ввода-вывода
//! - Расширенная статистика и метрики ввода-вывода

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::SystemTime;
use tracing::{debug, info, warn};

/// Структура для хранения метрик ввода-вывода системы
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemIOMetrics {
    /// Временная метка сбора метрик
    pub timestamp: SystemTime,
    /// Общее количество операций чтения
    pub total_read_operations: u64,
    /// Общее количество операций записи
    pub total_write_operations: u64,
    /// Общее количество байт прочитано
    pub total_bytes_read: u64,
    /// Общее количество байт записано
    pub total_bytes_written: u64,
    /// Общее время операций ввода-вывода в микросекундах
    pub total_io_time_us: u64,
    /// Среднее время операции чтения в микросекундах
    pub average_read_time_us: f64,
    /// Среднее время операции записи в микросекундах
    pub average_write_time_us: f64,
    /// Текущая загрузка подсистемы ввода-вывода (0.0 - 1.0)
    pub io_load: f64,
    /// Количество операций ввода-вывода в секунду (IOPS)
    pub iops: f64,
    /// Пропускная способность в байтах в секунду
    pub throughput_bytes_per_sec: f64,
    /// Метрики по устройствам
    pub device_metrics: HashMap<String, DeviceIOMetrics>,
    /// Метрики по типам операций
    pub operation_type_metrics: HashMap<String, OperationTypeMetrics>,
    /// Метрики по приоритетам операций
    pub priority_metrics: HashMap<u32, PriorityIOMetrics>,
    /// Тренды производительности ввода-вывода
    pub performance_trends: IOPerformanceTrends,
    /// Рекомендации по оптимизации
    pub optimization_recommendations: Vec<String>,
}

impl Default for SystemIOMetrics {
    fn default() -> Self {
        Self {
            timestamp: SystemTime::now(),
            total_read_operations: 0,
            total_write_operations: 0,
            total_bytes_read: 0,
            total_bytes_written: 0,
            total_io_time_us: 0,
            average_read_time_us: 0.0,
            average_write_time_us: 0.0,
            io_load: 0.0,
            iops: 0.0,
            throughput_bytes_per_sec: 0.0,
            device_metrics: HashMap::new(),
            operation_type_metrics: HashMap::new(),
            priority_metrics: HashMap::new(),
            performance_trends: IOPerformanceTrends::default(),
            optimization_recommendations: Vec::new(),
        }
    }
}

/// Метрики ввода-вывода для отдельного устройства
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceIOMetrics {
    /// Имя устройства
    pub device_name: String,
    /// Тип устройства
    pub device_type: DeviceType,
    /// Общее количество операций чтения
    pub read_operations: u64,
    /// Общее количество операций записи
    pub write_operations: u64,
    /// Общее количество байт прочитано
    pub bytes_read: u64,
    /// Общее количество байт записано
    pub bytes_written: u64,
    /// Общее время операций ввода-вывода в микросекундах
    pub io_time_us: u64,
    /// Среднее время операции чтения в микросекундах
    pub average_read_time_us: f64,
    /// Среднее время операции записи в микросекундах
    pub average_write_time_us: f64,
    /// Текущая загрузка устройства (0.0 - 1.0)
    pub device_load: f64,
    /// Количество операций ввода-вывода в секунду (IOPS)
    pub device_iops: f64,
    /// Пропускная способность в байтах в секунду
    pub device_throughput: f64,
    /// Количество операций в очереди
    pub queue_length: u32,
    /// Среднее время ожидания в очереди в микросекундах
    pub average_queue_time_us: f64,
    /// Процент использования устройства
    pub utilization_percent: f64,
    /// Состояние здоровья устройства
    pub health_status: DeviceHealthStatus,
}

/// Тип устройства
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceType {
    HDD,
    SSD,
    NVMe,
    Virtual,
    Network,
    RAMDisk,
    Unknown,
}

/// Состояние здоровья устройства
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeviceHealthStatus {
    Healthy,
    Warning,
    Critical,
    Failed,
    Unknown,
}

/// Метрики по типам операций ввода-вывода
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OperationTypeMetrics {
    /// Тип операции
    pub operation_type: String,
    /// Количество операций
    pub operation_count: u64,
    /// Общее количество байт
    pub total_bytes: u64,
    /// Общее время операций в микросекундах
    pub total_time_us: u64,
    /// Среднее время операции в микросекундах
    pub average_time_us: f64,
    /// Средняя пропускная способность в байтах в секунду
    pub average_throughput: f64,
}

/// Метрики по приоритетам операций ввода-вывода
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriorityIOMetrics {
    /// Приоритет операций
    pub priority: u32,
    /// Количество операций
    pub operation_count: u64,
    /// Общее количество байт
    pub total_bytes: u64,
    /// Общее время операций в микросекундах
    pub total_time_us: u64,
    /// Среднее время операции в микросекундах
    pub average_time_us: f64,
    /// Среднее время ожидания в микросекундах
    pub average_wait_time_us: f64,
}

/// Тренды производительности ввода-вывода
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IOPerformanceTrends {
    /// Тренд количества операций чтения
    pub read_operations_trend: f64,
    /// Тренд количества операций записи
    pub write_operations_trend: f64,
    /// Тренд пропускной способности
    pub throughput_trend: f64,
    /// Тренд загрузки подсистемы ввода-вывода
    pub io_load_trend: f64,
    /// Тренд среднего времени операции
    pub average_time_trend: f64,
    /// Тренд количества операций в очереди
    pub queue_length_trend: f64,
}

impl Default for IOPerformanceTrends {
    fn default() -> Self {
        Self {
            read_operations_trend: 0.0,
            write_operations_trend: 0.0,
            throughput_trend: 0.0,
            io_load_trend: 0.0,
            average_time_trend: 0.0,
            queue_length_trend: 0.0,
        }
    }
}

/// Конфигурация мониторинга ввода-вывода
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IOMonitorConfig {
    /// Интервал мониторинга в секундах
    pub monitoring_interval_secs: u64,
    /// Включить расширенный мониторинг
    pub enable_extended_monitoring: bool,
    /// Включить мониторинг на уровне процессов
    pub enable_process_level_monitoring: bool,
    /// Включить анализ производительности
    pub enable_performance_analysis: bool,
    /// Включить обнаружение узких мест
    pub enable_bottleneck_detection: bool,
    /// Включить оптимизацию параметров
    pub enable_parameter_optimization: bool,
    /// Максимальное количество устройств для мониторинга
    pub max_devices: usize,
    /// Максимальное количество процессов для мониторинга
    pub max_processes: usize,
    /// Пороговое значение загрузки для предупреждений
    pub load_warning_threshold: f64,
    /// Пороговое значение времени ожидания для предупреждений
    pub latency_warning_threshold_us: u64,
    /// Пороговое значение длины очереди для предупреждений
    pub queue_length_warning_threshold: u32,
    /// Включить автоматическую оптимизацию
    pub enable_auto_optimization: bool,
    /// Агрессивность оптимизации (0.0 - 1.0)
    pub optimization_aggressiveness: f64,
}

impl Default for IOMonitorConfig {
    fn default() -> Self {
        Self {
            monitoring_interval_secs: 60,
            enable_extended_monitoring: true,
            enable_process_level_monitoring: true,
            enable_performance_analysis: true,
            enable_bottleneck_detection: true,
            enable_parameter_optimization: true,
            max_devices: 10,
            max_processes: 100,
            load_warning_threshold: 0.8,
            latency_warning_threshold_us: 10000, // 10ms
            queue_length_warning_threshold: 10,
            enable_auto_optimization: true,
            optimization_aggressiveness: 0.5,
        }
    }
}

/// Основная структура мониторинга ввода-вывода
pub struct IOMonitor {
    /// Конфигурация мониторинга
    config: IOMonitorConfig,
    /// История метрик для анализа трендов
    metrics_history: Vec<SystemIOMetrics>,
    /// Максимальный размер истории
    max_history_size: usize,
    /// Последние собранные метрики
    last_metrics: Option<SystemIOMetrics>,
    /// Кэш информации об устройствах
    device_cache: HashMap<String, DeviceInfo>,
}

impl IOMonitor {
    /// Создать новый экземпляр мониторинга ввода-вывода
    pub fn new(config: IOMonitorConfig) -> Self {
        info!(
            "Creating I/O monitor with config: interval={}s, extended={}",
            config.monitoring_interval_secs, config.enable_extended_monitoring
        );

        Self {
            config,
            metrics_history: Vec::new(),
            max_history_size: 10,
            last_metrics: None,
            device_cache: HashMap::new(),
        }
    }

    /// Создать новый экземпляр с конфигурацией по умолчанию
    pub fn new_default() -> Self {
        Self::new(IOMonitorConfig::default())
    }

    /// Создать новый экземпляр с кастомным размером истории
    pub fn with_history_size(config: IOMonitorConfig, history_size: usize) -> Self {
        Self {
            config,
            metrics_history: Vec::new(),
            max_history_size: history_size,
            last_metrics: None,
            device_cache: HashMap::new(),
        }
    }

    /// Собрать метрики ввода-вывода системы
    pub fn collect_io_metrics(&mut self) -> Result<SystemIOMetrics> {
        let mut metrics = SystemIOMetrics::default();
        metrics.timestamp = SystemTime::now();

        // Собираем базовую статистику из /proc/diskstats
        let disk_stats = self.read_disk_stats()?;

        // Обрабатываем статистику для каждого устройства
        for (device_name, stats) in &disk_stats {
            let device_metrics = self.process_device_stats(device_name, stats)?;
            metrics.device_metrics.insert(device_name.clone(), device_metrics);
        }

        // Рассчитываем общие метрики
        self.calculate_overall_metrics(&mut metrics);

        // Анализируем тренды, если есть история
        if !self.metrics_history.is_empty() {
            metrics.performance_trends = self.analyze_io_trends(&metrics);
        }

        // Генерируем рекомендации по оптимизации
        if self.config.enable_performance_analysis {
            metrics.optimization_recommendations = self.generate_optimization_recommendations(&metrics);
        }

        // Сохраняем метрики в историю
        self.metrics_history.push(metrics.clone());
        if self.metrics_history.len() > self.max_history_size {
            self.metrics_history.remove(0);
        }

        // Сохраняем последние метрики
        self.last_metrics = Some(metrics.clone());

        info!(
            "I/O monitoring metrics collected: {} devices, {} read ops, {} write ops, load={:.2}%",
            metrics.device_metrics.len(),
            metrics.total_read_operations,
            metrics.total_write_operations,
            metrics.io_load * 100.0
        );

        Ok(metrics)
    }

    /// Прочитать статистику дисков из /proc/diskstats
    fn read_disk_stats(&self) -> Result<HashMap<String, DiskStats>> {
        let diskstats_path = Path::new("/proc/diskstats");
        
        if !diskstats_path.exists() {
            warn!("/proc/diskstats not found, using simulated data");
            return Ok(self.generate_simulated_disk_stats());
        }

        let content = std::fs::read_to_string(diskstats_path)
            .context("Не удалось прочитать /proc/diskstats")?;

        let mut disk_stats = HashMap::new();

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            
            if parts.len() < 14 {
                continue;
            }

            let major = parts[0].parse::<u32>()?;
            let minor = parts[1].parse::<u32>()?;
            let device_name = parts[2].to_string();
            
            // Пропускаем виртуальные устройства типа loop, ram, etc.
            if device_name.starts_with("loop") || device_name.starts_with("ram") || device_name.starts_with("dm-") {
                continue;
            }

            let reads_completed = parts[3].parse::<u64>()?;
            let reads_merged = parts[4].parse::<u64>()?;
            let sectors_read = parts[5].parse::<u64>()?;
            let time_spent_reading_ms = parts[6].parse::<u64>()?;
            let writes_completed = parts[7].parse::<u64>()?;
            let writes_merged = parts[8].parse::<u64>()?;
            let sectors_written = parts[9].parse::<u64>()?;
            let time_spent_writing_ms = parts[10].parse::<u64>()?;
            let ios_in_progress = parts[11].parse::<u64>()?;
            let time_spent_doing_io_ms = parts[12].parse::<u64>()?;
            let weighted_time_spent_doing_io_ms = parts[13].parse::<u64>()?;

            let stats = DiskStats {
                major,
                minor,
                reads_completed,
                reads_merged,
                sectors_read,
                time_spent_reading_ms,
                writes_completed,
                writes_merged,
                sectors_written,
                time_spent_writing_ms,
                ios_in_progress,
                time_spent_doing_io_ms,
                weighted_time_spent_doing_io_ms,
            };

            disk_stats.insert(device_name, stats);
        }

        Ok(disk_stats)
    }

    /// Сгенерировать симулированные данные дисков для тестирования
    fn generate_simulated_disk_stats(&self) -> HashMap<String, DiskStats> {
        let mut disk_stats = HashMap::new();

        // Добавляем симулированные данные для нескольких устройств
        let devices = vec!["sda", "sdb", "nvme0n1"];

        for (i, device) in devices.iter().enumerate() {
            let base_reads = 1000 + i as u64 * 500;
            let base_writes = 500 + i as u64 * 250;
            let base_sectors = 10000 + i as u64 * 5000;

            let stats = DiskStats {
                major: 8,
                minor: i as u32,
                reads_completed: base_reads,
                reads_merged: base_reads / 10,
                sectors_read: base_sectors,
                time_spent_reading_ms: base_reads * 2,
                writes_completed: base_writes,
                writes_merged: base_writes / 10,
                sectors_written: base_sectors / 2,
                time_spent_writing_ms: base_writes * 3,
                ios_in_progress: i as u64,
                time_spent_doing_io_ms: (base_reads + base_writes) * 2,
                weighted_time_spent_doing_io_ms: (base_reads + base_writes) * 3,
            };

            disk_stats.insert(device.to_string(), stats);
        }

        disk_stats
    }

    /// Обработать статистику для отдельного устройства
    fn process_device_stats(&self, device_name: &str, stats: &DiskStats) -> Result<DeviceIOMetrics> {
        let sector_size = 512; // Стандартный размер сектора
        
        let bytes_read = stats.sectors_read * sector_size;
        let bytes_written = stats.sectors_written * sector_size;
        let total_bytes = bytes_read + bytes_written;
        let total_operations = stats.reads_completed + stats.writes_completed;

        let average_read_time_us = if stats.reads_completed > 0 {
            (stats.time_spent_reading_ms as f64 * 1000.0) / stats.reads_completed as f64
        } else {
            0.0
        };

        let average_write_time_us = if stats.writes_completed > 0 {
            (stats.time_spent_writing_ms as f64 * 1000.0) / stats.writes_completed as f64
        } else {
            0.0
        };

        let average_io_time_us = if total_operations > 0 {
            (stats.time_spent_doing_io_ms as f64 * 1000.0) / total_operations as f64
        } else {
            0.0
        };

        // Определяем тип устройства
        let device_type = self.determine_device_type(device_name);

        // Определяем состояние здоровья
        let health_status = self.determine_device_health(stats, average_io_time_us);

        let device_metrics = DeviceIOMetrics {
            device_name: device_name.to_string(),
            device_type,
            read_operations: stats.reads_completed,
            write_operations: stats.writes_completed,
            bytes_read,
            bytes_written,
            io_time_us: stats.time_spent_doing_io_ms * 1000,
            average_read_time_us,
            average_write_time_us,
            device_load: self.calculate_device_load(stats),
            device_iops: total_operations as f64 / self.config.monitoring_interval_secs as f64,
            device_throughput: total_bytes as f64 / self.config.monitoring_interval_secs as f64,
            queue_length: stats.ios_in_progress as u32,
            average_queue_time_us: average_io_time_us,
            utilization_percent: self.calculate_utilization_percent(stats),
            health_status,
        };

        Ok(device_metrics)
    }

    /// Определить тип устройства
    fn determine_device_type(&self, device_name: &str) -> DeviceType {
        if device_name.starts_with("nvme") {
            DeviceType::NVMe
        } else if device_name.starts_with("sd") || device_name.starts_with("hd") || device_name.starts_with("vd") {
            // Для SSD и HDD нужно более сложное определение
            // В реальной системе можно использовать /sys/block/<device>/queue/rotational
            if device_name.starts_with("sd") {
                DeviceType::SSD // Упрощение: предполагаем, что sda/sdb - это SSD
            } else {
                DeviceType::HDD
            }
        } else if device_name.starts_with("loop") {
            DeviceType::Virtual
        } else if device_name.starts_with("ram") {
            DeviceType::RAMDisk
        } else {
            DeviceType::Unknown
        }
    }

    /// Определить состояние здоровья устройства
    fn determine_device_health(&self, stats: &DiskStats, average_io_time_us: f64) -> DeviceHealthStatus {
        // Проверяем время ожидания
        if average_io_time_us > self.config.latency_warning_threshold_us as f64 * 2.0 {
            return DeviceHealthStatus::Critical;
        }

        // Проверяем количество операций в очереди
        if stats.ios_in_progress > self.config.queue_length_warning_threshold as u64 * 2 {
            return DeviceHealthStatus::Warning;
        }

        // Проверяем время выполнения операций
        if stats.time_spent_doing_io_ms > 1000 { // Более 1 секунды
            return DeviceHealthStatus::Warning;
        }

        DeviceHealthStatus::Healthy
    }

    /// Рассчитать загрузку устройства
    fn calculate_device_load(&self, stats: &DiskStats) -> f64 {
        let total_time = stats.time_spent_doing_io_ms as f64;
        let monitoring_interval = self.config.monitoring_interval_secs as f64 * 1000.0; // в миллисекундах
        
        if monitoring_interval > 0.0 {
            (total_time / monitoring_interval).min(1.0)
        } else {
            0.0
        }
    }

    /// Рассчитать процент использования устройства
    fn calculate_utilization_percent(&self, stats: &DiskStats) -> f64 {
        let total_time = stats.time_spent_doing_io_ms as f64;
        let monitoring_interval = self.config.monitoring_interval_secs as f64 * 1000.0; // в миллисекундах
        
        if monitoring_interval > 0.0 {
            (total_time / monitoring_interval * 100.0).min(100.0)
        } else {
            0.0
        }
    }

    /// Рассчитать общие метрики
    fn calculate_overall_metrics(&self, metrics: &mut SystemIOMetrics) {
        let mut total_read_ops = 0u64;
        let mut total_write_ops = 0u64;
        let mut total_bytes_read = 0u64;
        let mut total_bytes_written = 0u64;
        let mut total_io_time_us = 0u64;
        let mut total_read_time_us = 0.0;
        let mut total_write_time_us = 0.0;

        for device_metrics in metrics.device_metrics.values() {
            total_read_ops += device_metrics.read_operations;
            total_write_ops += device_metrics.write_operations;
            total_bytes_read += device_metrics.bytes_read;
            total_bytes_written += device_metrics.bytes_written;
            total_io_time_us += device_metrics.io_time_us;
            total_read_time_us += device_metrics.average_read_time_us * device_metrics.read_operations as f64;
            total_write_time_us += device_metrics.average_write_time_us * device_metrics.write_operations as f64;
        }

        metrics.total_read_operations = total_read_ops;
        metrics.total_write_operations = total_write_ops;
        metrics.total_bytes_read = total_bytes_read;
        metrics.total_bytes_written = total_bytes_written;
        metrics.total_io_time_us = total_io_time_us;

        if total_read_ops > 0 {
            metrics.average_read_time_us = total_read_time_us / total_read_ops as f64;
        }

        if total_write_ops > 0 {
            metrics.average_write_time_us = total_write_time_us / total_write_ops as f64;
        }

        // Рассчитываем общую загрузку
        let total_operations = total_read_ops + total_write_ops;
        let monitoring_interval = self.config.monitoring_interval_secs as f64;

        metrics.iops = total_operations as f64 / monitoring_interval;
        metrics.throughput_bytes_per_sec = (total_bytes_read + total_bytes_written) as f64 / monitoring_interval;

        // Рассчитываем общую загрузку как среднюю по устройствам
        if !metrics.device_metrics.is_empty() {
            let total_load: f64 = metrics.device_metrics.values()
                .map(|d| d.device_load)
                .sum();
            metrics.io_load = total_load / metrics.device_metrics.len() as f64;
        }

        // Группируем метрики по типам операций
        self.group_metrics_by_operation_type(metrics);
        
        // Группируем метрики по приоритетам
        self.group_metrics_by_priority(metrics);
    }

    /// Группировать метрики по типам операций
    fn group_metrics_by_operation_type(&self, metrics: &mut SystemIOMetrics) {
        let mut operation_type_metrics = HashMap::new();

        // Добавляем метрики для операций чтения
        let read_metrics = OperationTypeMetrics {
            operation_type: "read".to_string(),
            operation_count: metrics.total_read_operations,
            total_bytes: metrics.total_bytes_read,
            total_time_us: (metrics.average_read_time_us * metrics.total_read_operations as f64) as u64,
            average_time_us: metrics.average_read_time_us,
            average_throughput: if metrics.total_read_operations > 0 {
                metrics.total_bytes_read as f64 / metrics.total_read_operations as f64
            } else {
                0.0
            },
        };
        operation_type_metrics.insert("read".to_string(), read_metrics);

        // Добавляем метрики для операций записи
        let write_metrics = OperationTypeMetrics {
            operation_type: "write".to_string(),
            operation_count: metrics.total_write_operations,
            total_bytes: metrics.total_bytes_written,
            total_time_us: (metrics.average_write_time_us * metrics.total_write_operations as f64) as u64,
            average_time_us: metrics.average_write_time_us,
            average_throughput: if metrics.total_write_operations > 0 {
                metrics.total_bytes_written as f64 / metrics.total_write_operations as f64
            } else {
                0.0
            },
        };
        operation_type_metrics.insert("write".to_string(), write_metrics);

        metrics.operation_type_metrics = operation_type_metrics;
    }

    /// Группировать метрики по приоритетам
    fn group_metrics_by_priority(&self, metrics: &mut SystemIOMetrics) {
        let mut priority_metrics = HashMap::new();

        // Упрощение: предполагаем, что все операции имеют приоритет 1
        let priority = 1u32;
        let total_operations = metrics.total_read_operations + metrics.total_write_operations;
        let total_bytes = metrics.total_bytes_read + metrics.total_bytes_written;
        let total_time_us = metrics.total_io_time_us;

        let average_time_us = if total_operations > 0 {
            total_time_us as f64 / total_operations as f64
        } else {
            0.0
        };

        let priority_io_metrics = PriorityIOMetrics {
            priority,
            operation_count: total_operations,
            total_bytes,
            total_time_us,
            average_time_us,
            average_wait_time_us: average_time_us * 0.5, // Упрощение
        };

        priority_metrics.insert(priority, priority_io_metrics);
        metrics.priority_metrics = priority_metrics;
    }

    /// Анализировать тренды производительности ввода-вывода
    fn analyze_io_trends(&self, current_metrics: &SystemIOMetrics) -> IOPerformanceTrends {
        if self.metrics_history.is_empty() {
            return IOPerformanceTrends::default();
        }

        let previous_metrics = &self.metrics_history[self.metrics_history.len() - 1];
        let mut trends = IOPerformanceTrends::default();

        // Рассчитываем тренды
        trends.read_operations_trend = current_metrics.total_read_operations as f64 - previous_metrics.total_read_operations as f64;
        trends.write_operations_trend = current_metrics.total_write_operations as f64 - previous_metrics.total_write_operations as f64;
        trends.throughput_trend = current_metrics.throughput_bytes_per_sec - previous_metrics.throughput_bytes_per_sec;
        trends.io_load_trend = current_metrics.io_load - previous_metrics.io_load;

        let current_avg_time = if current_metrics.total_read_operations + current_metrics.total_write_operations > 0 {
            (current_metrics.average_read_time_us + current_metrics.average_write_time_us) / 2.0
        } else {
            0.0
        };

        let previous_avg_time = if previous_metrics.total_read_operations + previous_metrics.total_write_operations > 0 {
            (previous_metrics.average_read_time_us + previous_metrics.average_write_time_us) / 2.0
        } else {
            0.0
        };

        trends.average_time_trend = current_avg_time - previous_avg_time;

        // Рассчитываем тренд длины очереди
        let current_queue_length: f64 = current_metrics.device_metrics.values()
            .map(|d| d.queue_length as f64)
            .sum();
        let previous_queue_length: f64 = previous_metrics.device_metrics.values()
            .map(|d| d.queue_length as f64)
            .sum();

        trends.queue_length_trend = current_queue_length - previous_queue_length;

        debug!(
            "I/O trends analyzed: read_ops={:.2}, write_ops={:.2}, throughput={:.2}, load={:.4}",
            trends.read_operations_trend, trends.write_operations_trend, trends.throughput_trend, trends.io_load_trend
        );

        trends
    }

    /// Генерировать рекомендации по оптимизации
    fn generate_optimization_recommendations(&self, metrics: &SystemIOMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Проверяем высокую загрузку
        if metrics.io_load > self.config.load_warning_threshold {
            recommendations.push(format!(
                "High I/O load ({:.2}%) - consider optimizing disk usage or adding more disks",
                metrics.io_load * 100.0
            ));
        }

        // Проверяем высокое среднее время операций
        let average_time_us = if metrics.total_read_operations + metrics.total_write_operations > 0 {
            (metrics.average_read_time_us + metrics.average_write_time_us) / 2.0
        } else {
            0.0
        };

        if average_time_us > self.config.latency_warning_threshold_us as f64 {
            recommendations.push(format!(
                "High average I/O operation time ({:.2} μs) - consider upgrading to faster storage",
                average_time_us
            ));
        }

        // Проверяем длинные очереди
        let total_queue_length: u32 = metrics.device_metrics.values()
            .map(|d| d.queue_length)
            .sum();

        if total_queue_length > self.config.queue_length_warning_threshold {
            recommendations.push(format!(
                "High I/O queue length ({}) - consider optimizing disk scheduling or adding more disks",
                total_queue_length
            ));
        }

        // Проверяем устройства с проблемами здоровья
        for (device_name, device_metrics) in &metrics.device_metrics {
            match device_metrics.health_status {
                DeviceHealthStatus::Warning => {
                    recommendations.push(format!(
                        "Device '{}' health warning - monitor performance",
                        device_name
                    ));
                }
                DeviceHealthStatus::Critical => {
                    recommendations.push(format!(
                        "Device '{}' health critical - consider replacing or investigating",
                        device_name
                    ));
                }
                _ => {}
            }
        }

        // Анализируем тренды
        if !metrics.performance_trends.io_load_trend.is_nan() && metrics.performance_trends.io_load_trend > 0.1 {
            recommendations.push(format!(
                "Increasing I/O load trend ({:.4}) - monitor system performance",
                metrics.performance_trends.io_load_trend
            ));
        }

        if !metrics.performance_trends.average_time_trend.is_nan() && metrics.performance_trends.average_time_trend > 1000.0 {
            recommendations.push(format!(
                "Increasing average I/O time trend ({:.2} μs) - investigate potential bottlenecks",
                metrics.performance_trends.average_time_trend
            ));
        }

        debug!(
            "Generated {} I/O optimization recommendations",
            recommendations.len()
        );

        recommendations
    }

    /// Обнаружить узкие места в подсистеме ввода-вывода
    pub fn detect_io_bottlenecks(&self, metrics: &SystemIOMetrics) -> Result<Vec<IOBottleneck>> {
        let mut bottlenecks = Vec::new();

        // Проверяем общие узкие места
        if metrics.io_load > self.config.load_warning_threshold * 1.2 {
            bottlenecks.push(IOBottleneck {
                bottleneck_type: IOBottleneckType::HighLoad,
                severity: IOBottleneckSeverity::Critical,
                description: format!(
                    "Overall I/O load is very high: {:.2}% (threshold: {:.2}%)",
                    metrics.io_load * 100.0, self.config.load_warning_threshold * 100.0
                ),
                affected_devices: "All devices".to_string(),
                recommendation: "Consider adding more disks, optimizing disk usage, or upgrading to faster storage".to_string(),
            });
        }

        // Проверяем узкие места на уровне устройств
        for (device_name, device_metrics) in &metrics.device_metrics {
            if device_metrics.device_load > self.config.load_warning_threshold * 1.2 {
                bottlenecks.push(IOBottleneck {
                    bottleneck_type: IOBottleneckType::HighLoad,
                    severity: IOBottleneckSeverity::Critical,
                    description: format!(
                        "Device '{}' has very high load: {:.2}%",
                        device_name, device_metrics.device_load * 100.0
                    ),
                    affected_devices: device_name.clone(),
                    recommendation: format!(
                        "Consider optimizing usage of device {} or adding more disks",
                        device_name
                    ),
                });
            }

            if device_metrics.average_queue_time_us > self.config.latency_warning_threshold_us as f64 * 2.0 {
                bottlenecks.push(IOBottleneck {
                    bottleneck_type: IOBottleneckType::HighLatency,
                    severity: IOBottleneckSeverity::Warning,
                    description: format!(
                        "Device '{}' has high average queue time: {:.2} μs",
                        device_name, device_metrics.average_queue_time_us
                    ),
                    affected_devices: device_name.clone(),
                    recommendation: format!(
                        "Consider optimizing disk scheduling or upgrading device {} to faster storage",
                        device_name
                    ),
                });
            }

            if device_metrics.queue_length > self.config.queue_length_warning_threshold * 2 {
                bottlenecks.push(IOBottleneck {
                    bottleneck_type: IOBottleneckType::LongQueue,
                    severity: IOBottleneckSeverity::Warning,
                    description: format!(
                        "Device '{}' has long queue: {}",
                        device_name, device_metrics.queue_length
                    ),
                    affected_devices: device_name.clone(),
                    recommendation: format!(
                        "Consider optimizing I/O scheduling or adding more disks for device {}",
                        device_name
                    ),
                });
            }
        }

        // Проверяем узкие места по типам операций
        if let Some(read_metrics) = metrics.operation_type_metrics.get("read") {
            if read_metrics.average_time_us > self.config.latency_warning_threshold_us as f64 * 1.5 {
                bottlenecks.push(IOBottleneck {
                    bottleneck_type: IOBottleneckType::HighLatency,
                    severity: IOBottleneckSeverity::Warning,
                    description: format!(
                        "Read operations have high latency: {:.2} μs",
                        read_metrics.average_time_us
                    ),
                    affected_devices: "All devices".to_string(),
                    recommendation: "Consider optimizing read operations or upgrading storage".to_string(),
                });
            }
        }

        if let Some(write_metrics) = metrics.operation_type_metrics.get("write") {
            if write_metrics.average_time_us > self.config.latency_warning_threshold_us as f64 * 1.5 {
                bottlenecks.push(IOBottleneck {
                    bottleneck_type: IOBottleneckType::HighLatency,
                    severity: IOBottleneckSeverity::Warning,
                    description: format!(
                        "Write operations have high latency: {:.2} μs",
                        write_metrics.average_time_us
                    ),
                    affected_devices: "All devices".to_string(),
                    recommendation: "Consider optimizing write operations or upgrading storage".to_string(),
                });
            }
        }

        if bottlenecks.is_empty() {
            debug!("No I/O bottlenecks detected");
        } else {
            warn!(
                "Detected {} I/O bottlenecks: {} critical, {} warnings",
                bottlenecks.len(),
                bottlenecks.iter().filter(|b| b.severity == IOBottleneckSeverity::Critical).count(),
                bottlenecks.iter().filter(|b| b.severity == IOBottleneckSeverity::Warning).count()
            );
        }

        Ok(bottlenecks)
    }

    /// Оптимизировать параметры ввода-вывода
    pub fn optimize_io_parameters(&self, metrics: &SystemIOMetrics) -> Result<Vec<IOOptimizationRecommendation>> {
        let mut optimizations = Vec::new();

        // Анализируем каждый тип устройства
        let mut device_type_stats: HashMap<DeviceType, Vec<&DeviceIOMetrics>> = HashMap::new();
        
        for device_metrics in metrics.device_metrics.values() {
            device_type_stats
                .entry(device_metrics.device_type.clone())
                .or_insert_with(Vec::new)
                .push(device_metrics);
        }

        for (device_type, devices) in device_type_stats {
            let total_load: f64 = devices.iter().map(|d| d.device_load).sum();
            let avg_load = total_load / devices.len() as f64;
            let total_latency: f64 = devices.iter().map(|d| d.average_queue_time_us).sum();
            let avg_latency = total_latency / devices.len() as f64;

            let mut optimization = IOOptimizationRecommendation {
                device_type: device_type.clone(),
                device_count: devices.len(),
                current_load: avg_load,
                recommended_load: avg_load,
                current_latency_us: avg_latency,
                recommended_latency_us: avg_latency,
                priority: 1,
                reason: String::new(),
            };

            // Оптимизируем нагрузку
            if avg_load > self.config.load_warning_threshold {
                // Рекомендуем уменьшить нагрузку
                let reduction_factor = 1.0 - (self.config.optimization_aggressiveness * 0.3);
                optimization.recommended_load = avg_load * reduction_factor;
                optimization.reason.push_str("High load; ");
            }

            // Оптимизируем задержку
            if avg_latency > self.config.latency_warning_threshold_us as f64 {
                // Рекомендуем уменьшить задержку
                let reduction_factor = 1.0 - (self.config.optimization_aggressiveness * 0.4);
                optimization.recommended_latency_us = avg_latency * reduction_factor;
                optimization.reason.push_str("High latency; ");
            }

            // Убираем последний "; " если есть
            if !optimization.reason.is_empty() {
                optimization.reason.pop();
                optimization.reason.pop();
            }

            if optimization.recommended_load != optimization.current_load ||
               optimization.recommended_latency_us != optimization.current_latency_us {
                optimizations.push(optimization);
            }
        }

        info!(
            "Generated {} I/O optimization recommendations",
            optimizations.len()
        );

        Ok(optimizations)
    }

    /// Получить последние метрики
    pub fn get_last_metrics(&self) -> Option<SystemIOMetrics> {
        self.last_metrics.clone()
    }

    /// Получить историю метрик
    pub fn get_metrics_history(&self) -> Vec<SystemIOMetrics> {
        self.metrics_history.clone()
    }

    /// Очистить историю метрик
    pub fn clear_metrics_history(&mut self) {
        self.metrics_history.clear();
        debug!("I/O metrics history cleared");
    }

    /// Экспортировать метрики в JSON
    pub fn export_metrics_to_json(&self, metrics: &SystemIOMetrics) -> Result<String> {
        use serde_json::to_string;

        let json_data = serde_json::json!({
            "timestamp": metrics.timestamp,
            "total_read_operations": metrics.total_read_operations,
            "total_write_operations": metrics.total_write_operations,
            "total_bytes_read": metrics.total_bytes_read,
            "total_bytes_written": metrics.total_bytes_written,
            "total_io_time_us": metrics.total_io_time_us,
            "average_read_time_us": metrics.average_read_time_us,
            "average_write_time_us": metrics.average_write_time_us,
            "io_load": metrics.io_load,
            "iops": metrics.iops,
            "throughput_bytes_per_sec": metrics.throughput_bytes_per_sec,
            "device_metrics": metrics.device_metrics,
            "operation_type_metrics": metrics.operation_type_metrics,
            "priority_metrics": metrics.priority_metrics,
            "performance_trends": metrics.performance_trends,
            "optimization_recommendations": metrics.optimization_recommendations,
        });

        to_string(&json_data).context("Не удалось сериализовать метрики ввода-вывода в JSON")
    }
}

/// Структура для хранения статистики диска из /proc/diskstats
#[derive(Debug, Clone)]
struct DiskStats {
    major: u32,
    minor: u32,
    reads_completed: u64,
    reads_merged: u64,
    sectors_read: u64,
    time_spent_reading_ms: u64,
    writes_completed: u64,
    writes_merged: u64,
    sectors_written: u64,
    time_spent_writing_ms: u64,
    ios_in_progress: u64,
    time_spent_doing_io_ms: u64,
    weighted_time_spent_doing_io_ms: u64,
}

/// Информация об устройстве для кэша
#[derive(Debug, Clone)]
struct DeviceInfo {
    device_name: String,
    device_type: DeviceType,
    last_seen: SystemTime,
}

/// Узкое место в подсистеме ввода-вывода
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IOBottleneck {
    /// Тип узкого места
    pub bottleneck_type: IOBottleneckType,
    /// Серьезность узкого места
    pub severity: IOBottleneckSeverity,
    /// Описание узкого места
    pub description: String,
    /// Затрагиваемые устройства
    pub affected_devices: String,
    /// Рекомендация по устранению
    pub recommendation: String,
}

/// Тип узкого места в подсистеме ввода-вывода
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IOBottleneckType {
    HighLoad,
    HighLatency,
    LongQueue,
    LowThroughput,
    HighErrorRate,
    DeviceFailure,
}

/// Серьезность узкого места в подсистеме ввода-вывода
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IOBottleneckSeverity {
    Info,
    Warning,
    Critical,
}

/// Рекомендация по оптимизации ввода-вывода
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IOOptimizationRecommendation {
    /// Тип устройства
    pub device_type: DeviceType,
    /// Количество устройств
    pub device_count: usize,
    /// Текущая нагрузка
    pub current_load: f64,
    /// Рекомендуемая нагрузка
    pub recommended_load: f64,
    /// Текущая задержка в микросекундах
    pub current_latency_us: f64,
    /// Рекомендуемая задержка в микросекундах
    pub recommended_latency_us: f64,
    /// Приоритет
    pub priority: u32,
    /// Причина рекомендации
    pub reason: String,
}

/// Тесты для модуля мониторинга ввода-вывода
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_monitor_creation() {
        let config = IOMonitorConfig::default();
        let monitor = IOMonitor::new(config);
        assert_eq!(monitor.config.monitoring_interval_secs, 60);
        assert!(monitor.config.enable_extended_monitoring);
    }

    #[test]
    fn test_io_monitor_default() {
        let monitor = IOMonitor::new_default();
        assert_eq!(monitor.config.monitoring_interval_secs, 60);
        assert!(monitor.config.enable_bottleneck_detection);
    }

    #[test]
    fn test_io_monitor_with_history_size() {
        let config = IOMonitorConfig::default();
        let monitor = IOMonitor::with_history_size(config, 20);
        assert_eq!(monitor.max_history_size, 20);
    }

    #[test]
    fn test_io_metrics_collection() {
        let config = IOMonitorConfig::default();
        let mut monitor = IOMonitor::new(config);

        // Собираем метрики
        let result = monitor.collect_io_metrics();
        assert!(result.is_ok());
        let metrics = result.unwrap();

        assert!(metrics.total_read_operations > 0);
        assert!(metrics.total_write_operations > 0);
        assert!(metrics.total_bytes_read > 0);
        assert!(metrics.total_bytes_written > 0);
        assert!(metrics.io_load >= 0.0);
        assert!(metrics.iops >= 0.0);
        assert!(!metrics.device_metrics.is_empty());
    }

    #[test]
    fn test_io_metrics_empty() {
        let config = IOMonitorConfig::default();
        let mut monitor = IOMonitor::new(config);

        // Собираем метрики (должно использовать симулированные данные)
        let result = monitor.collect_io_metrics();
        assert!(result.is_ok());
        let metrics = result.unwrap();

        // Даже с симулированными данными должны быть метрики
        assert!(metrics.total_read_operations > 0);
        assert!(metrics.total_write_operations > 0);
    }

    #[test]
    fn test_io_optimization_recommendations() {
        let config = IOMonitorConfig::default();
        let monitor = IOMonitor::new(config);

        let mut metrics = SystemIOMetrics::default();
        metrics.io_load = 0.9; // Above threshold
        metrics.average_read_time_us = 15000.0; // Above threshold
        metrics.average_write_time_us = 15000.0;

        let recommendations = monitor.generate_optimization_recommendations(&metrics);
        assert!(!recommendations.is_empty());
        assert!(recommendations.iter().any(|r| r.contains("High I/O load")));
        assert!(recommendations.iter().any(|r| r.contains("High average I/O operation time")));
    }

    #[test]
    fn test_io_bottleneck_detection() {
        let config = IOMonitorConfig::default();
        let monitor = IOMonitor::new(config);

        let mut metrics = SystemIOMetrics::default();
        metrics.io_load = 0.9; // Above threshold

        let bottlenecks = monitor.detect_io_bottlenecks(&metrics);
        assert!(bottlenecks.is_ok());
        let bottlenecks = bottlenecks.unwrap();
        assert!(!bottlenecks.is_empty());
        assert!(bottlenecks.iter().any(|b| matches!(b.bottleneck_type, IOBottleneckType::HighLoad)));
    }

    #[test]
    fn test_io_metrics_history() {
        let config = IOMonitorConfig::default();
        let mut monitor = IOMonitor::with_history_size(config, 3);

        // Собираем метрики несколько раз
        for _ in 0..5 {
            let result = monitor.collect_io_metrics();
            assert!(result.is_ok());
        }

        // Проверяем, что история не превышает максимальный размер
        assert_eq!(monitor.metrics_history.len(), 3);
    }

    #[test]
    fn test_io_metrics_export() {
        let config = IOMonitorConfig::default();
        let mut monitor = IOMonitor::new(config);

        // Собираем метрики
        let result = monitor.collect_io_metrics();
        assert!(result.is_ok());
        let metrics = result.unwrap();

        // Экспортируем в JSON
        let json_result = monitor.export_metrics_to_json(&metrics);
        assert!(json_result.is_ok());
        let json_string = json_result.unwrap();
        assert!(json_string.contains("total_read_operations"));
        assert!(json_string.contains("total_write_operations"));
        assert!(json_string.contains("device_metrics"));
    }

    #[test]
    fn test_io_monitor_trends() {
        let config = IOMonitorConfig::default();
        let mut monitor = IOMonitor::new(config);

        // Собираем начальные метрики
        let result = monitor.collect_io_metrics();
        assert!(result.is_ok());

        // Собираем метрики еще раз для анализа трендов
        let result = monitor.collect_io_metrics();
        assert!(result.is_ok());
        let metrics = result.unwrap();

        // Проверяем, что тренды рассчитаны
        assert!(!metrics.performance_trends.read_operations_trend.is_nan());
        assert!(!metrics.performance_trends.write_operations_trend.is_nan());
        assert!(!metrics.performance_trends.throughput_trend.is_nan());
    }

    #[test]
    fn test_io_device_type_detection() {
        let config = IOMonitorConfig::default();
        let monitor = IOMonitor::new(config);

        // Проверяем определение типов устройств
        assert_eq!(monitor.determine_device_type("nvme0n1"), DeviceType::NVMe);
        assert_eq!(monitor.determine_device_type("sda"), DeviceType::SSD);
        assert_eq!(monitor.determine_device_type("hda"), DeviceType::HDD);
        assert_eq!(monitor.determine_device_type("loop0"), DeviceType::Virtual);
        assert_eq!(monitor.determine_device_type("ram0"), DeviceType::RAMDisk);
        assert_eq!(monitor.determine_device_type("unknown"), DeviceType::Unknown);
    }

    #[test]
    fn test_io_device_health_detection() {
        let config = IOMonitorConfig::default();
        let monitor = IOMonitor::new(config);

        // Создаем тестовые статистики
        let stats = DiskStats {
            major: 8,
            minor: 0,
            reads_completed: 1000,
            reads_merged: 100,
            sectors_read: 10000,
            time_spent_reading_ms: 2000,
            writes_completed: 500,
            writes_merged: 50,
            sectors_written: 5000,
            time_spent_writing_ms: 1000,
            ios_in_progress: 5,
            time_spent_doing_io_ms: 3000,
            weighted_time_spent_doing_io_ms: 4000,
        };

        let average_io_time_us = 3000.0; // 3ms
        let health = monitor.determine_device_health(&stats, average_io_time_us);
        assert_eq!(health, DeviceHealthStatus::Healthy);

        // Тестируем с высокой задержкой
        let high_latency_stats = DiskStats {
            ios_in_progress: 20,
            time_spent_doing_io_ms: 10000,
            ..stats
        };
        let high_latency = monitor.determine_device_health(&high_latency_stats, 20000.0);
        assert_eq!(high_latency, DeviceHealthStatus::Critical);
    }

    #[test]
    fn test_io_metrics_calculation() {
        let config = IOMonitorConfig::default();
        let mut monitor = IOMonitor::new(config);

        // Собираем метрики
        let result = monitor.collect_io_metrics();
        assert!(result.is_ok());
        let metrics = result.unwrap();

        // Проверяем, что метрики рассчитаны корректно
        assert!(metrics.average_read_time_us >= 0.0);
        assert!(metrics.average_write_time_us >= 0.0);
        assert!(metrics.io_load >= 0.0 && metrics.io_load <= 1.0);
        assert!(metrics.iops >= 0.0);
        assert!(metrics.throughput_bytes_per_sec >= 0.0);

        // Проверяем, что метрики по типам операций присутствуют
        assert!(metrics.operation_type_metrics.contains_key("read"));
        assert!(metrics.operation_type_metrics.contains_key("write"));

        // Проверяем, что метрики по приоритетам присутствуют
        assert!(!metrics.priority_metrics.is_empty());
    }
}
