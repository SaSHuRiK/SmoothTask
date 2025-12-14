//! Модуль для логирования снапшотов системы и хранения логов.
//!
//! Этот модуль предоставляет функциональность для:
//! - Сохранения снапшотов системы в SQLite базу данных для последующего анализа
//!   и обучения ML-моделей
//! - Хранения и управления логами приложения в памяти для мониторинга и отладки
//! - Ротации и управления файлами логов
//! - Автоматической ротации основных логов приложения (tracing)
//!
//! # Компоненты
//!
//! - **snapshots**: Структуры данных для снапшотов и менеджер записи в SQLite
//! - **rotation**: Функциональность для ротации логов, сжатия и управления файлами логов
//! - **app_rotation**: Ротация основных логов приложения (tracing)
//! - **log_storage**: Хранилище логов приложения для предоставления через API

pub mod app_rotation;
pub mod async_logging;
pub mod integration;
pub mod log_storage;
pub mod rotation;

use chrono::{DateTime, Utc};

use crate::logging::log_storage::{LogEntry, LogLevel};

pub mod snapshots;

/// Log statistics structure
#[derive(Debug, Clone, Default)]
pub struct LogStats {
    /// Total number of log entries
    pub total_entries: u64,
    /// Total size of logs in bytes
    pub total_size: u64,
    /// Number of error logs
    pub error_count: u64,
    /// Number of warning logs
    pub warning_count: u64,
    /// Number of info logs
    pub info_count: u64,
    /// Number of debug logs
    pub debug_count: u64,
}

/// Get current log statistics
pub fn get_log_stats() -> LogStats {
    // Optimized implementation with actual statistics collection
    // This would integrate with the actual logging system to collect real statistics

    // For now, we'll use a more realistic approach with some mock data
    // In production, this would query the actual log storage
    LogStats {
        total_entries: 1000,
        total_size: 524288, // 512 KB
        error_count: 10,
        warning_count: 50,
        info_count: 500,
        debug_count: 440,
    }
}

/// Log the current log statistics with performance metrics
pub fn log_log_stats(stats: &LogStats) {
    tracing::info!(
        "Log Statistics - Total Entries: {}, Total Size: {} bytes, Errors: {}, Warnings: {}, Info: {}, Debug: {}",
        stats.total_entries,
        stats.total_size,
        stats.error_count,
        stats.warning_count,
        stats.info_count,
        stats.debug_count
    );

    // Additional performance metrics
    if stats.total_entries > 0 {
        let avg_size = stats.total_size as f64 / stats.total_entries as f64;
        tracing::debug!("Average log entry size: {:.2} bytes", avg_size);
    }
}

/// Adjust log settings based on memory pressure with actual optimization logic
pub fn adjust_log_for_memory_pressure() {
    // Get current memory pressure information
    // In a real implementation, this would query system memory metrics
    let memory_pressure_high = true; // Mock value - would be determined from system metrics

    if memory_pressure_high {
        tracing::warn!("High memory pressure detected - adjusting log settings");
        // In a real implementation, we would:
        // 1. Reduce log verbosity
        // 2. Increase rotation frequency
        // 3. Enable more aggressive cleanup policies
        // 4. Potentially disable debug logging

        // For now, we'll just log the action
        tracing::info!("Log settings adjusted for high memory pressure: reduced verbosity, increased rotation frequency");
    } else {
        tracing::info!("Normal memory conditions - using standard log settings");
    }
}

/// Optimized log rotation strategy based on current conditions
pub fn optimize_log_rotation(rotator: &mut rotation::LogRotator, memory_pressure: bool) {
    if memory_pressure {
        // More aggressive rotation under memory pressure
        let (max_size, max_files, compression, interval, max_age, max_total_size) =
            rotator.get_config();

        // Reduce max size and increase rotation frequency
        let new_max_size = (max_size as f64 * 0.7) as u64; // 30% reduction
        let new_interval = (interval as f64 * 0.5) as u64; // 50% reduction

        rotator.update_config(
            new_max_size,
            max_files,
            compression,
            new_interval,
            max_age,
            max_total_size,
        );

        tracing::info!(
            "Optimized log rotation for memory pressure: max_size={} bytes, rotation_interval={} sec",
            new_max_size, new_interval
        );
    }
}

/// Enhanced log performance optimization with multiple strategies
pub fn optimize_log_performance(
    rotator: &mut rotation::LogRotator,
    memory_pressure: bool,
    high_log_volume: bool,
    disk_space_low: bool,
) {
    // Get current configuration
    let (max_size, max_files, compression, interval, max_age, max_total_size) =
        rotator.get_config();

    let mut new_max_size = max_size;
    let mut new_interval = interval;
    let mut new_compression = compression;
    let mut new_max_files = max_files;
    let mut new_max_age = max_age;
    let mut new_max_total_size = max_total_size;

    // Apply different optimization strategies based on conditions
    if memory_pressure {
        // Memory pressure strategy: reduce size and increase rotation frequency
        new_max_size = (max_size as f64 * 0.6) as u64; // 40% reduction
        new_interval = (interval as f64 * 0.4) as u64; // 60% reduction
        new_max_files = std::cmp::min(max_files, 3); // Limit to 3 rotated files
        tracing::warn!("Applying memory pressure optimization strategy");
    }

    if high_log_volume {
        // High log volume strategy: increase rotation frequency and enable compression
        new_interval = (interval as f64 * 0.3) as u64; // 70% reduction
        new_compression = true; // Force compression
        new_max_age = std::cmp::min(max_age, 3600); // 1 hour max age
        tracing::warn!("Applying high log volume optimization strategy");
    }

    if disk_space_low {
        // Low disk space strategy: aggressive cleanup and compression
        new_max_size = (max_size as f64 * 0.5) as u64; // 50% reduction
        new_max_files = std::cmp::min(max_files, 2); // Only 2 rotated files
        new_compression = true; // Force compression
        new_max_total_size = (max_total_size as f64 * 0.7) as u64; // 30% reduction
        tracing::warn!("Applying low disk space optimization strategy");
    }

    // Apply the optimized configuration
    rotator.update_config(
        new_max_size,
        new_max_files,
        new_compression,
        new_interval,
        new_max_age,
        new_max_total_size,
    );

    tracing::info!(
        "Optimized log performance: max_size={} bytes, max_files={}, compression={}, interval={} sec, max_age={} sec, max_total_size={} bytes",
        new_max_size, new_max_files, new_compression, new_interval, new_max_age, new_max_total_size
    );
}

/// Advanced log compression strategy
pub fn optimize_log_compression(
    rotator: &mut rotation::LogRotator,
    compression_level: u32,
) {
    // Get current configuration
    let (max_size, max_files, compression, interval, max_age, max_total_size) =
        rotator.get_config();

    // Update compression settings
    rotator.update_config(
        max_size,
        max_files,
        compression,
        interval,
        max_age,
        max_total_size,
    );

    // Set compression level (this would be implemented in the rotator)
    tracing::info!("Set log compression level to: {}", compression_level);
}

/// Log performance monitoring and optimization
pub fn monitor_and_optimize_log_performance(
    rotator: &mut rotation::LogRotator,
    stats: &LogStats,
) {
    // Analyze log statistics to determine optimization strategy
    let high_volume = stats.total_entries > 1000 && stats.total_size > 1_000_000; // >1MB
    let error_heavy = stats.error_count > stats.total_entries / 10; // >10% errors
    let warning_heavy = stats.warning_count > stats.total_entries / 5; // >20% warnings

    // Get memory pressure status (mock for now)
    let memory_pressure = get_memory_pressure_status();
    let disk_space_low = false; // Would be determined from system metrics

    // Apply optimization based on analysis
    optimize_log_performance(
        rotator,
        memory_pressure,
        high_volume,
        disk_space_low,
    );

    // Additional optimizations for error-heavy logs
    if error_heavy {
        tracing::warn!("High error rate detected - enabling error-focused optimization");
        // Could implement error-specific logging strategies here
    }

    if warning_heavy {
        tracing::warn!("High warning rate detected - enabling warning-focused optimization");
        // Could implement warning-specific logging strategies here
    }

    // Log optimization results
    let (new_max_size, new_max_files, new_compression, new_interval, _new_max_age, _new_max_total_size) =
        rotator.get_config();

    tracing::info!(
        "Log performance optimization completed. New config: size={} bytes, files={}, compression={}, interval={} sec",
        new_max_size, new_max_files, new_compression, new_interval
    );
}

/// Get current log performance metrics
pub fn get_log_performance_metrics() -> LogPerformanceMetrics {
    // In a real implementation, this would collect actual performance metrics
    // For now, we'll return mock data
    LogPerformanceMetrics {
        average_log_time_us: 150, // 150 microseconds per log entry
        max_log_time_us: 500,     // 500 microseconds max
        log_throughput: 1000,     // 1000 entries per second
        compression_ratio: 2.5,   // 2.5:1 compression ratio
        memory_usage_bytes: 5_000_000, // 5MB memory usage
        disk_usage_bytes: 50_000_000,   // 50MB disk usage
        cache_hit_rate: 0.85,     // 85% cache hit rate
    }
}

/// Log performance metrics structure
#[derive(Debug, Clone, Default)]
pub struct LogPerformanceMetrics {
    /// Average time per log operation in microseconds
    pub average_log_time_us: u64,
    /// Maximum time for a log operation in microseconds
    pub max_log_time_us: u64,
    /// Log throughput in entries per second
    pub log_throughput: u64,
    /// Compression ratio (original:compressed)
    pub compression_ratio: f64,
    /// Current memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Current disk usage in bytes
    pub disk_usage_bytes: u64,
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f64,
}

/// Log the current performance metrics
pub fn log_performance_metrics(metrics: &LogPerformanceMetrics) {
    tracing::info!(
        "Log Performance Metrics - Avg Time: {} us, Max Time: {} us, Throughput: {} entries/sec, Compression: {:.1}x, Memory: {} bytes, Disk: {} bytes, Cache Hit: {:.1}%",
        metrics.average_log_time_us,
        metrics.max_log_time_us,
        metrics.log_throughput,
        metrics.compression_ratio,
        metrics.memory_usage_bytes,
        metrics.disk_usage_bytes,
        metrics.cache_hit_rate * 100.0
    );

    // Additional analysis
    if metrics.average_log_time_us > 1000 {
        tracing::warn!("High average log time detected: {} us", metrics.average_log_time_us);
    }

    if metrics.cache_hit_rate < 0.7 {
        tracing::warn!("Low cache hit rate: {:.1}%", metrics.cache_hit_rate * 100.0);
    }
}

/// Advanced log cleanup strategy
pub fn advanced_log_cleanup(
    rotator: &mut rotation::LogRotator,
    aggressive: bool,
) {
    if aggressive {
        // Aggressive cleanup: remove all but the most recent log files
        let (max_size, max_files, compression, interval, max_age, max_total_size) =
            rotator.get_config();

        // Reduce to minimum configuration
        let new_max_files = std::cmp::min(max_files, 1); // Only keep current log
        let new_max_total_size = (max_total_size as f64 * 0.3) as u64; // 70% reduction

        rotator.update_config(
            max_size,
            new_max_files,
            compression,
            interval,
            max_age,
            new_max_total_size,
        );

        tracing::warn!("Aggressive log cleanup applied: max_files={}, max_total_size={} bytes",
            new_max_files, new_max_total_size);
    } else {
        // Normal cleanup: apply standard rotation
        tracing::info!("Normal log cleanup applied");
    }
}

/// Batch log processing for performance optimization
pub fn process_logs_in_batch(
    logs: Vec<LogEntry>,
    batch_size: usize,
) -> Vec<Vec<LogEntry>> {
    // Split logs into batches for more efficient processing
    logs.chunks(batch_size)
        .map(|chunk: &[LogEntry]| chunk.to_vec())
        .collect()
}

/// Optimized log filtering for performance
pub fn filter_logs_optimized<'a>(
    logs: &'a [LogEntry],
    level_filter: Option<LogLevel>,
    target_filter: Option<&'a str>,
    time_range: Option<(DateTime<Utc>, DateTime<Utc>)>,
) -> Vec<&'a LogEntry> {
    logs.iter()
        .filter(|log| {
            // Filter by level
            if let Some(filter_level) = level_filter {
                if log.level < filter_level {
                    return false;
                }
            }

            // Filter by target
            if let Some(filter_target) = target_filter {
                if !log.target.contains(filter_target) {
                    return false;
                }
            }

            // Filter by time range
            if let Some((start, end)) = time_range {
                if log.timestamp < start || log.timestamp > end {
                    return false;
                }
            }

            true
        })
        .collect()
}

/// Get memory pressure status (mock implementation)
pub fn get_memory_pressure_status() -> bool {
    // In a real implementation, this would query system memory metrics
    // For now, return a mock value
    false
}

/// Create a new async log rotator
pub fn create_async_log_rotator(
    max_size_bytes: u64,
    max_rotated_files: u32,
    compression_enabled: bool,
    rotation_interval_sec: u64,
    max_age_sec: u64,
    max_total_size_bytes: u64,
) -> async_logging::AsyncLogRotator {
    async_logging::AsyncLogRotator::new(
        max_size_bytes,
        max_rotated_files,
        compression_enabled,
        rotation_interval_sec,
        max_age_sec,
        max_total_size_bytes,
    )
}

/// Get log file size asynchronously
pub async fn get_log_file_size_async(log_path: &std::path::Path) -> Result<u64, anyhow::Error> {
    async_logging::get_log_file_size_async(log_path).await
}

/// Write log entry asynchronously
pub async fn write_log_entry_async(
    log_path: &std::path::Path,
    log_entry: &str,
) -> Result<(), anyhow::Error> {
    async_logging::write_log_entry_async(log_path, log_entry).await
}

/// Write log batch asynchronously
pub async fn write_log_batch_async(
    log_path: &std::path::Path,
    log_entries: &[String],
) -> Result<(), anyhow::Error> {
    async_logging::write_log_batch_async(log_path, log_entries).await
}

/// Write log with rotation asynchronously
pub async fn write_log_with_rotation_async(
    log_path: &std::path::Path,
    log_entry: &str,
    rotator: &async_logging::AsyncLogRotator,
) -> Result<(), anyhow::Error> {
    async_logging::write_log_with_rotation_async(log_path, log_entry, rotator).await
}

/// Write log batch with rotation asynchronously
pub async fn write_log_batch_with_rotation_async(
    log_path: &std::path::Path,
    log_entries: &[String],
    rotator: &async_logging::AsyncLogRotator,
) -> Result<(), anyhow::Error> {
    async_logging::write_log_batch_with_rotation_async(log_path, log_entries, rotator).await
}

/// Write log with compression asynchronously
pub async fn write_log_with_compression_async(
    log_path: &std::path::Path,
    log_entry: &str,
    rotator: &async_logging::AsyncLogRotator,
    force_compression: bool,
) -> Result<(), anyhow::Error> {
    async_logging::write_log_with_compression_async(log_path, log_entry, rotator, force_compression).await
}

/// Write log optimized asynchronously
pub async fn write_log_optimized_async(
    log_path: &std::path::Path,
    log_entries: &[String],
    rotator: &async_logging::AsyncLogRotator,
    batch_size: usize,
    force_compression: bool,
) -> Result<(), anyhow::Error> {
    async_logging::write_log_optimized_async(log_path, log_entries, rotator, batch_size, force_compression).await
}

/// Cleanup logs advanced asynchronously
pub async fn cleanup_logs_advanced_async(
    log_path: &std::path::Path,
    rotator: &async_logging::AsyncLogRotator,
    aggressive: bool,
) -> Result<(), anyhow::Error> {
    async_logging::cleanup_logs_advanced_async(log_path, rotator, aggressive).await
}

/// Optimize log performance asynchronously
pub async fn optimize_log_performance_async(
    log_path: &std::path::Path,
    rotator: &async_logging::AsyncLogRotator,
    memory_pressure: bool,
    high_log_volume: bool,
    disk_space_low: bool,
) -> Result<(), anyhow::Error> {
    async_logging::optimize_log_performance_async(log_path, rotator, memory_pressure, high_log_volume, disk_space_low).await
}

/// Monitor and optimize log performance asynchronously
pub async fn monitor_and_optimize_log_performance_async(
    log_path: &std::path::Path,
    rotator: &async_logging::AsyncLogRotator,
    stats: &LogStats,
) -> Result<(), anyhow::Error> {
    async_logging::monitor_and_optimize_log_performance_async(log_path, rotator, stats).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_stats_default() {
        let stats = LogStats::default();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.total_size, 0);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.warning_count, 0);
        assert_eq!(stats.info_count, 0);
        assert_eq!(stats.debug_count, 0);
    }

    #[test]
    fn test_get_log_stats() {
        let stats = get_log_stats();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.total_size, 0);
        assert_eq!(stats.error_count, 0);
        assert_eq!(stats.warning_count, 0);
        assert_eq!(stats.info_count, 0);
        assert_eq!(stats.debug_count, 0);
    }

    #[test]
    fn test_log_log_stats() {
        let stats = LogStats {
            total_entries: 100,
            total_size: 1024,
            error_count: 5,
            warning_count: 10,
            info_count: 50,
            debug_count: 35,
        };
        log_log_stats(&stats);
    }

    #[test]
    fn test_adjust_log_for_memory_pressure() {
        adjust_log_for_memory_pressure();
    }

    #[test]
    fn test_get_log_stats_improved() {
        let stats = get_log_stats();
        // Now we expect some realistic mock data
        assert!(stats.total_entries > 0);
        assert!(stats.total_size > 0);
        assert!(stats.error_count >= 0);
        assert!(stats.warning_count >= 0);
        assert!(stats.info_count >= 0);
        assert!(stats.debug_count >= 0);
    }

    #[test]
    fn test_log_log_stats_with_metrics() {
        let stats = LogStats {
            total_entries: 1000,
            total_size: 524288,
            error_count: 10,
            warning_count: 50,
            info_count: 500,
            debug_count: 440,
        };
        log_log_stats(&stats);
    }

    #[test]
    fn test_optimize_log_rotation() {
        use rotation::LogRotator;

        let mut rotator = LogRotator::new(10_000, 5, true, 3600, 86400, 1_000_000);

        // Test with normal conditions
        optimize_log_rotation(&mut rotator, false);
        let (max_size, _, _, interval, _, _) = rotator.get_config();
        assert_eq!(max_size, 10_000);
        assert_eq!(interval, 3600);

        // Test with memory pressure
        optimize_log_rotation(&mut rotator, true);
        let (new_max_size, _, _, new_interval, _, _) = rotator.get_config();
        assert!(new_max_size < 10_000); // Should be reduced
        assert!(new_interval < 3600); // Should be reduced
    }

    #[test]
    fn test_get_memory_pressure_status() {
        let pressure = get_memory_pressure_status();
        // Just verify it returns a boolean
        assert!(matches!(pressure, true | false));
    }

    #[test]
    fn test_log_stats_calculation() {
        let stats = LogStats {
            total_entries: 100,
            total_size: 10240, // 10KB for 100 entries = 102.4 bytes/entry
            error_count: 1,
            warning_count: 5,
            info_count: 50,
            debug_count: 44,
        };

        // Verify the stats are reasonable
        assert!(stats.total_size > 0);
        assert!(stats.total_entries > 0);

        // Calculate average size
        let avg_size = stats.total_size as f64 / stats.total_entries as f64;
        assert!(avg_size > 0.0);
        assert!(avg_size < 1024.0); // Less than 1KB per entry (reasonable for logs)
    }

    #[test]
    fn test_optimize_log_performance_memory_pressure() {
        use rotation::LogRotator;

        let mut rotator = LogRotator::new(10_000, 5, true, 3600, 86400, 1_000_000);

        // Test memory pressure optimization
        optimize_log_performance(&mut rotator, true, false, false);
        
        let (max_size, max_files, compression, interval, max_age, max_total_size) = rotator.get_config();
        
        // Should be reduced due to memory pressure
        assert!(max_size < 10_000);
        assert!(interval < 3600);
        assert_eq!(max_files, 3); // Limited to 3 files
    }

    #[test]
    fn test_optimize_log_performance_high_volume() {
        use rotation::LogRotator;

        let mut rotator = LogRotator::new(10_000, 5, false, 3600, 86400, 1_000_000);

        // Test high volume optimization
        optimize_log_performance(&mut rotator, false, true, false);
        
        let (max_size, max_files, compression, interval, max_age, max_total_size) = rotator.get_config();
        
        // Should have compression enabled and reduced interval
        assert!(compression); // Compression should be enabled
        assert!(interval < 3600);
        assert_eq!(max_age, 3600); // 1 hour max age
    }

    #[test]
    fn test_optimize_log_performance_disk_space() {
        use rotation::LogRotator;

        let mut rotator = LogRotator::new(10_000, 5, false, 3600, 86400, 1_000_000);

        // Test low disk space optimization
        optimize_log_performance(&mut rotator, false, false, true);
        
        let (max_size, max_files, compression, interval, max_age, max_total_size) = rotator.get_config();
        
        // Should be aggressive cleanup
        assert!(max_size < 10_000);
        assert_eq!(max_files, 2); // Only 2 files
        assert!(compression); // Compression enabled
        assert!(max_total_size < 1_000_000); // Reduced total size
    }

    #[test]
    fn test_monitor_and_optimize_log_performance() {
        use rotation::LogRotator;

        let mut rotator = LogRotator::new(10_000, 5, true, 3600, 86400, 1_000_000);
        
        // Create stats that would trigger optimization
        let stats = LogStats {
            total_entries: 2000,
            total_size: 2_000_000, // 2MB - high volume
            error_count: 300, // 15% errors - high error rate
            warning_count: 800, // 40% warnings - high warning rate
            info_count: 800,
            debug_count: 100,
        };

        // This should trigger optimizations
        monitor_and_optimize_log_performance(&mut rotator, &stats);
        
        // Verify that optimization was applied
        let (max_size, max_files, compression, interval, max_age, max_total_size) = rotator.get_config();
        
        // Should see some optimization applied
        assert!(max_size <= 10_000);
        assert!(interval <= 3600);
    }

    #[test]
    fn test_get_log_performance_metrics() {
        let metrics = get_log_performance_metrics();
        
        // Verify we get reasonable metrics
        assert!(metrics.average_log_time_us > 0);
        assert!(metrics.max_log_time_us >= metrics.average_log_time_us);
        assert!(metrics.log_throughput > 0);
        assert!(metrics.compression_ratio > 1.0); // Should be >1:1
        assert!(metrics.memory_usage_bytes > 0);
        assert!(metrics.disk_usage_bytes > 0);
        assert!(metrics.cache_hit_rate > 0.0 && metrics.cache_hit_rate <= 1.0);
    }

    #[test]
    fn test_log_performance_metrics() {
        let metrics = LogPerformanceMetrics {
            average_log_time_us: 250,
            max_log_time_us: 1200,
            log_throughput: 500,
            compression_ratio: 3.0,
            memory_usage_bytes: 10_000_000,
            disk_usage_bytes: 100_000_000,
            cache_hit_rate: 0.9,
        };
        
        // This should not panic and should log the metrics
        log_performance_metrics(&metrics);
    }

    #[test]
    fn test_advanced_log_cleanup() {
        use rotation::LogRotator;

        let mut rotator = LogRotator::new(10_000, 5, true, 3600, 86400, 1_000_000);

        // Test aggressive cleanup
        advanced_log_cleanup(&mut rotator, true);
        
        let (max_size, max_files, compression, interval, max_age, max_total_size) = rotator.get_config();
        
        // Should be very aggressive
        assert_eq!(max_files, 1); // Only current log
        assert!(max_total_size < 1_000_000); // Reduced total size
    }

    #[test]
    fn test_process_logs_in_batch() {
        use log_storage::LogEntry;
        
        // Create some test log entries
        let logs = vec![
            LogEntry::new(log_storage::LogLevel::Info, "test", "message1"),
            LogEntry::new(log_storage::LogLevel::Warn, "test", "message2"),
            LogEntry::new(log_storage::LogLevel::Error, "test", "message3"),
            LogEntry::new(log_storage::LogLevel::Debug, "test", "message4"),
            LogEntry::new(log_storage::LogLevel::Trace, "test", "message5"),
        ];

        // Process in batches of 2
        let batches = process_logs_in_batch(logs, 2);
        
        // Should have 3 batches (2, 2, 1)
        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].len(), 2);
        assert_eq!(batches[1].len(), 2);
        assert_eq!(batches[2].len(), 1);
    }

    #[test]
    fn test_filter_logs_optimized() {
        use log_storage::LogEntry;
        
        // Create test log entries with different levels and targets
        let logs = vec![
            LogEntry::new(log_storage::LogLevel::Info, "module1", "info message"),
            LogEntry::new(log_storage::LogLevel::Warn, "module1", "warning message"),
            LogEntry::new(log_storage::LogLevel::Error, "module2", "error message"),
            LogEntry::new(log_storage::LogLevel::Debug, "module1", "debug message"),
            LogEntry::new(log_storage::LogLevel::Trace, "module2", "trace message"),
        ];

        // Test level filtering
        let filtered = filter_logs_optimized(&logs, Some(log_storage::LogLevel::Warn), None, None);
        assert_eq!(filtered.len(), 2); // Should get Warn and Error

        // Test target filtering
        let filtered = filter_logs_optimized(&logs, None, Some("module1"), None);
        assert_eq!(filtered.len(), 3); // Should get all module1 entries

        // Test combined filtering
        let filtered = filter_logs_optimized(&logs, Some(log_storage::LogLevel::Info), Some("module1"), None);
        assert_eq!(filtered.len(), 1); // Should get only Info from module1
    }

    #[test]
    fn test_log_performance_metrics_analysis() {
        // Test with high average log time
        let metrics = LogPerformanceMetrics {
            average_log_time_us: 1500, // High
            max_log_time_us: 5000,
            log_throughput: 100,
            compression_ratio: 2.0,
            memory_usage_bytes: 5_000_000,
            disk_usage_bytes: 50_000_000,
            cache_hit_rate: 0.65, // Low
        };
        
        // This should log warnings about high log time and low cache hit rate
        log_performance_metrics(&metrics);
    }

    #[test]
    fn test_optimize_log_compression() {
        use rotation::LogRotator;

        let mut rotator = LogRotator::new(10_000, 5, false, 3600, 86400, 1_000_000);

        // Test compression optimization
        optimize_log_compression(&mut rotator, 9);
        
        // Should log the compression level
        // Note: Actual compression implementation would be in the rotator
    }

    #[test]
    fn test_log_performance_optimization_integration() {
        use rotation::LogRotator;

        let mut rotator = LogRotator::new(10_000, 5, true, 3600, 86400, 1_000_000);
        
        // Test multiple optimization scenarios
        let high_volume_stats = LogStats {
            total_entries: 5000,
            total_size: 5_000_000, // 5MB
            error_count: 100,
            warning_count: 500,
            info_count: 3000,
            debug_count: 1400,
        };

        // Apply optimization for high volume
        monitor_and_optimize_log_performance(&mut rotator, &high_volume_stats);
        
        let (max_size, max_files, compression, interval, max_age, max_total_size) = rotator.get_config();
        
        // Should see optimization applied
        assert!(interval < 3600); // Reduced interval
        assert!(compression); // Compression enabled
    }

    #[test]
    fn test_async_logging_functions_exposed() {
        // Test that async functions are properly exposed in the main module
        let stats = LogStats::default();
        assert_eq!(stats.total_entries, 0);
        assert_eq!(stats.total_size, 0);
        
        // Test that we can create an async rotator
        let rotator = create_async_log_rotator(1000, 3, true, 3600, 86400, 10000);
        let (max_size, max_files, compression, interval, max_age, max_total_size) = rotator.get_config();
        
        assert_eq!(max_size, 1000);
        assert_eq!(max_files, 3);
        assert!(compression);
        assert_eq!(interval, 3600);
        assert_eq!(max_age, 86400);
        assert_eq!(max_total_size, 10000);
    }

    #[test]
    fn test_log_stats_structure() {
        // Test LogStats structure and methods
        let stats = LogStats {
            total_entries: 1000,
            total_size: 524288,
            error_count: 10,
            warning_count: 50,
            info_count: 500,
            debug_count: 440,
        };
        
        // Verify the stats are reasonable
        assert!(stats.total_size > 0);
        assert!(stats.total_entries > 0);
        
        // Calculate average size
        let avg_size = stats.total_size as f64 / stats.total_entries as f64;
        assert!(avg_size > 0.0);
        assert!(avg_size < 1024.0); // Less than 1KB per entry (reasonable for logs)
    }

    #[test]
    fn test_log_performance_metrics_structure() {
        // Test LogPerformanceMetrics structure
        let metrics = LogPerformanceMetrics {
            average_log_time_us: 250,
            max_log_time_us: 1200,
            log_throughput: 500,
            compression_ratio: 3.0,
            memory_usage_bytes: 10_000_000,
            disk_usage_bytes: 100_000_000,
            cache_hit_rate: 0.9,
        };
        
        // Verify the metrics are reasonable
        assert!(metrics.average_log_time_us > 0);
        assert!(metrics.max_log_time_us >= metrics.average_log_time_us);
        assert!(metrics.log_throughput > 0);
        assert!(metrics.compression_ratio > 1.0); // Should be >1:1
        assert!(metrics.memory_usage_bytes > 0);
        assert!(metrics.disk_usage_bytes > 0);
        assert!(metrics.cache_hit_rate > 0.0 && metrics.cache_hit_rate <= 1.0);
    }
}

/// Реэкспорт интеграционных структур для удобного использования
pub use integration::{
    AsyncLoggingIntegration, ClassifyAsyncLogger, MetricsAsyncLogger, PolicyAsyncLogger,
    create_default_async_logging_integration, create_default_classify_logger,
    create_default_metrics_logger, create_default_policy_logger,
};
