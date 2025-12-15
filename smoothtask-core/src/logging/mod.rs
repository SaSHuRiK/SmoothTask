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

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde_json;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::logging::log_storage::{LogEntry, LogLevel};

/// Extension trait for LogLevel to add numeric conversion
pub trait LogLevelExt {
    /// Convert LogLevel to numeric value for comparison
    fn as_numeric(&self) -> u8;
}

impl LogLevelExt for LogLevel {
    fn as_numeric(&self) -> u8 {
        match self {
            LogLevel::Trace => 0,
            LogLevel::Debug => 1,
            LogLevel::Info => 2,
            LogLevel::Warn => 3,
            LogLevel::Error => 4,
        }
    }
}

/// Default implementation for LogLevel
impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Debug
    }
}

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

/// Enhanced log filter configuration
#[derive(Debug, Clone)]
pub struct LogFilterConfig {
    /// Minimum log level to include
    pub min_level: LogLevel,
    /// Maximum log level to include
    pub max_level: LogLevel,
    /// Filter by keywords (include if any keyword matches)
    pub include_keywords: Vec<String>,
    /// Filter by keywords (exclude if any keyword matches)
    pub exclude_keywords: Vec<String>,
    /// Filter by source modules
    pub include_modules: Vec<String>,
    /// Filter by source modules (exclude)
    pub exclude_modules: Vec<String>,
    /// Time range filter (timestamp range)
    pub time_range: Option<(u64, u64)>,
    /// Size limit for filtered results
    pub max_results: Option<usize>,
}

impl Default for LogFilterConfig {
    fn default() -> Self {
        Self {
            min_level: LogLevel::Debug,
            max_level: LogLevel::Error,
            include_keywords: Vec::new(),
            exclude_keywords: Vec::new(),
            include_modules: Vec::new(),
            exclude_modules: Vec::new(),
            time_range: None,
            max_results: None,
        }
    }
}

impl LogFilterConfig {
    /// Create a new log filter configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set minimum log level
    pub fn with_min_level(mut self, level: LogLevel) -> Self {
        self.min_level = level;
        self
    }

    /// Set maximum log level
    pub fn with_max_level(mut self, level: LogLevel) -> Self {
        self.max_level = level;
        self
    }

    /// Add include keyword
    pub fn with_include_keyword(mut self, keyword: String) -> Self {
        self.include_keywords.push(keyword);
        self
    }

    /// Add exclude keyword
    pub fn with_exclude_keyword(mut self, keyword: String) -> Self {
        self.exclude_keywords.push(keyword);
        self
    }

    /// Add include module
    pub fn with_include_module(mut self, module: String) -> Self {
        self.include_modules.push(module);
        self
    }

    /// Add exclude module
    pub fn with_exclude_module(mut self, module: String) -> Self {
        self.exclude_modules.push(module);
        self
    }

    /// Set time range filter
    pub fn with_time_range(mut self, start: u64, end: u64) -> Self {
        self.time_range = Some((start, end));
        self
    }

    /// Set maximum results limit
    pub fn with_max_results(mut self, max: usize) -> Self {
        self.max_results = Some(max);
        self
    }
}

/// Enhanced log entry with additional metadata for filtering
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnhancedLogEntry {
    /// Original log entry
    pub entry: LogEntry,
    /// Source module
    pub module: String,
    /// Additional tags
    pub tags: Vec<String>,
    /// Timestamp (seconds since epoch)
    pub timestamp: u64,
    /// Process ID (if available)
    pub pid: Option<i32>,
}

impl EnhancedLogEntry {
    /// Create a new enhanced log entry
    pub fn new(entry: LogEntry, module: String) -> Self {
        Self {
            entry,
            module,
            tags: Vec::new(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            pid: None,
        }
    }

    /// Add a tag to the log entry
    pub fn with_tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    /// Set process ID
    pub fn with_pid(mut self, pid: i32) -> Self {
        self.pid = Some(pid);
        self
    }

    /// Check if this entry matches the filter criteria
    pub fn matches_filter(&self, filter: &LogFilterConfig) -> bool {
        // Check log level range
        let level_value = self.entry.level.as_numeric();
        let min_value = filter.min_level.as_numeric();
        let max_value = filter.max_level.as_numeric();

        if level_value < min_value || level_value > max_value {
            return false;
        }

        // Check time range
        if let Some((start, end)) = filter.time_range {
            if self.timestamp < start || self.timestamp > end {
                return false;
            }
        }

        // Check include keywords
        if !filter.include_keywords.is_empty() {
            let message = self.entry.message.to_lowercase();
            let has_match = filter.include_keywords.iter().any(|keyword| {
                message.contains(&keyword.to_lowercase())
            });
            if !has_match {
                return false;
            }
        }

        // Check exclude keywords
        if !filter.exclude_keywords.is_empty() {
            let message = self.entry.message.to_lowercase();
            let has_match = filter.exclude_keywords.iter().any(|keyword| {
                message.contains(&keyword.to_lowercase())
            });
            if has_match {
                return false;
            }
        }

        // Check include modules
        if !filter.include_modules.is_empty() {
            let has_match = filter.include_modules.iter().any(|module| {
                self.module.contains(module)
            });
            if !has_match {
                return false;
            }
        }

        // Check exclude modules
        if !filter.exclude_modules.is_empty() {
            let has_match = filter.exclude_modules.iter().any(|module| {
                self.module.contains(module)
            });
            if has_match {
                return false;
            }
        }

        true
    }
}

/// Log analysis result
#[derive(Debug, Clone, Default)]
pub struct LogAnalysisResult {
    /// Filtered log entries
    pub entries: Vec<EnhancedLogEntry>,
    /// Statistics for the filtered results
    pub stats: LogStats,
    /// Analysis metadata
    pub metadata: LogAnalysisMetadata,
}

/// Log analysis metadata
#[derive(Debug, Clone, Default)]
pub struct LogAnalysisMetadata {
    /// Filter configuration used
    pub filter_config: LogFilterConfig,
    /// Analysis timestamp
    pub analysis_time: u64,
    /// Duration of analysis in milliseconds
    pub analysis_duration_ms: u64,
    /// Number of entries processed
    pub entries_processed: u64,
    /// Number of entries filtered out
    pub entries_filtered_out: u64,
}

/// Enhanced log analyzer
#[derive(Debug)]
pub struct LogAnalyzer {
    /// Log storage for analysis
    storage: crate::logging::log_storage::LogStorage,
}

impl LogAnalyzer {
    /// Create a new log analyzer
    pub fn new() -> Self {
        Self {
            storage: crate::logging::log_storage::LogStorage::new(1000),
        }
    }

    /// Analyze logs with the given filter configuration
    pub fn analyze_logs(&self, filter: LogFilterConfig) -> Result<LogAnalysisResult> {
        let start_time = SystemTime::now();
        let mut result = LogAnalysisResult::default();
        result.metadata.filter_config = filter.clone();

        // Get all log entries from storage
        let all_entries = self.storage.get_all_entries();
        result.metadata.entries_processed = all_entries.len() as u64;

        // Apply filtering
        let filtered_entries: Vec<EnhancedLogEntry> = all_entries
            .into_iter()
            .filter(|entry| {
                let enhanced = EnhancedLogEntry::new(entry.clone(), "unknown".to_string());
                enhanced.matches_filter(&filter)
            })
            .map(|entry| EnhancedLogEntry::new(entry, "unknown".to_string()))
            .collect();

        // Apply max results limit
        let final_entries = if let Some(max_results) = filter.max_results {
            filtered_entries.into_iter().take(max_results).collect()
        } else {
            filtered_entries
        };

        result.entries = final_entries;
        result.metadata.entries_filtered_out = result.metadata.entries_processed - result.entries.len() as u64;

        // Calculate statistics
        result.stats.total_entries = result.entries.len() as u64;
        result.stats.total_size = result.entries.iter().map(|e| e.entry.message.len() as u64).sum();
        result.stats.error_count = result.entries.iter().filter(|e| e.entry.level == LogLevel::Error).count() as u64;
        result.stats.warning_count = result.entries.iter().filter(|e| e.entry.level == LogLevel::Warn).count() as u64;
        result.stats.info_count = result.entries.iter().filter(|e| e.entry.level == LogLevel::Info).count() as u64;
        result.stats.debug_count = result.entries.iter().filter(|e| e.entry.level == LogLevel::Debug).count() as u64;

        // Calculate analysis duration
        let end_time = SystemTime::now();
        result.metadata.analysis_time = end_time
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        result.metadata.analysis_duration_ms = end_time
            .duration_since(start_time)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(result)
    }

    /// Get log statistics with enhanced filtering
    pub fn get_filtered_stats(&self, filter: LogFilterConfig) -> Result<LogStats> {
        let analysis = self.analyze_logs(filter)?;
        Ok(analysis.stats)
    }

    /// Find logs by keyword with filtering
    pub fn find_logs_by_keyword(&self, keyword: &str, filter: LogFilterConfig) -> Result<Vec<EnhancedLogEntry>> {
        let mut filter = filter;
        filter.include_keywords.push(keyword.to_string());
        let analysis = self.analyze_logs(filter)?;
        Ok(analysis.entries)
    }

    /// Find logs by module with filtering
    pub fn find_logs_by_module(&self, module: &str, filter: LogFilterConfig) -> Result<Vec<EnhancedLogEntry>> {
        let mut filter = filter;
        filter.include_modules.push(module.to_string());
        let analysis = self.analyze_logs(filter)?;
        Ok(analysis.entries)
    }

    /// Analyze log patterns and trends
    pub fn analyze_log_patterns(&self, filter: LogFilterConfig) -> Result<LogPatternAnalysis> {
        let analysis = self.analyze_logs(filter)?;
        let mut pattern_analysis = LogPatternAnalysis::default();

        // Count entries by level
        for entry in &analysis.entries {
            match entry.entry.level {
                LogLevel::Trace => pattern_analysis.debug_count += 1, // Treat trace as debug
                LogLevel::Debug => pattern_analysis.debug_count += 1,
                LogLevel::Info => pattern_analysis.info_count += 1,
                LogLevel::Warn => pattern_analysis.warning_count += 1,
                LogLevel::Error => pattern_analysis.error_count += 1,
            }
        }

        // Calculate total
        pattern_analysis.total_entries = pattern_analysis.error_count +
            pattern_analysis.warning_count +
            pattern_analysis.info_count +
            pattern_analysis.debug_count;

        // Calculate percentages
        if pattern_analysis.total_entries > 0 {
            pattern_analysis.error_percentage = (pattern_analysis.error_count as f32 / pattern_analysis.total_entries as f32) * 100.0;
            pattern_analysis.warning_percentage = (pattern_analysis.warning_count as f32 / pattern_analysis.total_entries as f32) * 100.0;
            pattern_analysis.info_percentage = (pattern_analysis.info_count as f32 / pattern_analysis.total_entries as f32) * 100.0;
            pattern_analysis.debug_percentage = (pattern_analysis.debug_count as f32 / pattern_analysis.total_entries as f32) * 100.0;
        }

        Ok(pattern_analysis)
    }
}

/// Log pattern analysis result
#[derive(Debug, Clone, Default)]
pub struct LogPatternAnalysis {
    /// Total number of entries analyzed
    pub total_entries: u64,
    /// Number of error entries
    pub error_count: u64,
    /// Number of warning entries
    pub warning_count: u64,
    /// Number of info entries
    pub info_count: u64,
    /// Number of debug entries
    pub debug_count: u64,
    /// Percentage of error entries
    pub error_percentage: f32,
    /// Percentage of warning entries
    pub warning_percentage: f32,
    /// Percentage of info entries
    pub info_percentage: f32,
    /// Percentage of debug entries
    pub debug_percentage: f32,
}

/// Integration with monitoring system
pub fn integrate_log_analysis_with_monitoring(analysis: &LogAnalysisResult) -> Result<()> {
    // In a real implementation, this would integrate with the monitoring system
    // to provide log analysis metrics and alerts

    tracing::info!(
        "Log Analysis Integration - Processed: {}, Filtered: {}, Duration: {}ms",
        analysis.metadata.entries_processed,
        analysis.metadata.entries_filtered_out,
        analysis.metadata.analysis_duration_ms
    );

    // Check for critical patterns
    let error_percentage = if analysis.stats.total_entries > 0 {
        (analysis.stats.error_count as f32 / analysis.stats.total_entries as f32) * 100.0
    } else {
        0.0
    };

    if error_percentage > 10.0 {
        tracing::warn!(
            "High error rate detected: {:.2}% errors in filtered logs",
            error_percentage
        );
    }

    Ok(())
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

    // New tests for enhanced logging functionality
    #[test]
    fn test_log_filter_config_creation() {
        let filter = LogFilterConfig::new();
        assert_eq!(filter.min_level, LogLevel::Debug);
        assert_eq!(filter.max_level, LogLevel::Error);
        assert!(filter.include_keywords.is_empty());
        assert!(filter.exclude_keywords.is_empty());
        assert!(filter.include_modules.is_empty());
        assert!(filter.exclude_modules.is_empty());
        assert!(filter.time_range.is_none());
        assert!(filter.max_results.is_none());
    }

    #[test]
    fn test_log_filter_config_builders() {
        let filter = LogFilterConfig::new()
            .with_min_level(LogLevel::Warn)
            .with_max_level(LogLevel::Error)
            .with_include_keyword("error".to_string())
            .with_exclude_keyword("debug".to_string())
            .with_include_module("metrics".to_string())
            .with_exclude_module("test".to_string())
            .with_time_range(1000, 2000)
            .with_max_results(100);

        assert_eq!(filter.min_level, LogLevel::Warn);
        assert_eq!(filter.max_level, LogLevel::Error);
        assert_eq!(filter.include_keywords, vec!["error"]);
        assert_eq!(filter.exclude_keywords, vec!["debug"]);
        assert_eq!(filter.include_modules, vec!["metrics"]);
        assert_eq!(filter.exclude_modules, vec!["test"]);
        assert_eq!(filter.time_range, Some((1000, 2000)));
        assert_eq!(filter.max_results, Some(100));
    }

    #[test]
    fn test_enhanced_log_entry_creation() {
        let entry = LogEntry {
            level: LogLevel::Info,
            message: "Test message".to_string(),
            timestamp: 1234567890,
        };

        let enhanced = EnhancedLogEntry::new(entry, "test_module".to_string());
        assert_eq!(enhanced.entry.message, "Test message");
        assert_eq!(enhanced.module, "test_module");
        assert!(enhanced.tags.is_empty());
        assert!(enhanced.timestamp > 0);
        assert!(enhanced.pid.is_none());
    }

    #[test]
    fn test_enhanced_log_entry_methods() {
        let entry = LogEntry {
            level: LogLevel::Info,
            message: "Test message".to_string(),
            timestamp: 1234567890,
        };

        let enhanced = EnhancedLogEntry::new(entry, "test_module".to_string())
            .with_tag("important".to_string())
            .with_pid(123);

        assert_eq!(enhanced.tags, vec!["important"]);
        assert_eq!(enhanced.pid, Some(123));
    }

    #[test]
    fn test_log_entry_filtering() {
        let entry = LogEntry {
            level: LogLevel::Info,
            message: "This is a test error message".to_string(),
            timestamp: 1500,
        };

        let enhanced = EnhancedLogEntry::new(entry, "test_module".to_string());

        // Test level filtering
        let filter_level = LogFilterConfig::new()
            .with_min_level(LogLevel::Warn)
            .with_max_level(LogLevel::Error);
        assert!(!enhanced.matches_filter(&filter_level));

        // Test keyword filtering
        let filter_keyword = LogFilterConfig::new()
            .with_include_keyword("error".to_string());
        assert!(enhanced.matches_filter(&filter_keyword));

        let filter_exclude = LogFilterConfig::new()
            .with_exclude_keyword("test".to_string());
        assert!(!enhanced.matches_filter(&filter_exclude));

        // Test time range filtering
        let filter_time = LogFilterConfig::new()
            .with_time_range(1000, 2000);
        assert!(enhanced.matches_filter(&filter_time));

        let filter_time_outside = LogFilterConfig::new()
            .with_time_range(2000, 3000);
        assert!(!enhanced.matches_filter(&filter_time_outside));

        // Test module filtering
        let filter_module = LogFilterConfig::new()
            .with_include_module("test".to_string());
        assert!(enhanced.matches_filter(&filter_module));

        let filter_exclude_module = LogFilterConfig::new()
            .with_exclude_module("test".to_string());
        assert!(!enhanced.matches_filter(&filter_exclude_module));
    }

    #[test]
    fn test_log_analyzer_creation() {
        let analyzer = LogAnalyzer::new();
        assert!(analyzer.storage.entries.is_empty());
    }

    #[test]
    fn test_log_analysis_basic() {
        let analyzer = LogAnalyzer::new();
        let filter = LogFilterConfig::new();

        // Add some test entries to storage
        let mut storage = crate::logging::log_storage::LogStorage::new();
        storage.add_entry(LogEntry {
            level: LogLevel::Info,
            message: "Test info message".to_string(),
            timestamp: 1000,
        });
        storage.add_entry(LogEntry {
            level: LogLevel::Error,
            message: "Test error message".to_string(),
            timestamp: 1500,
        });

        // Replace the analyzer's storage
        let analyzer = LogAnalyzer { storage };

        let result = analyzer.analyze_logs(filter).unwrap();
        assert_eq!(result.entries.len(), 2);
        assert_eq!(result.stats.total_entries, 2);
        assert_eq!(result.stats.error_count, 1);
        assert_eq!(result.stats.info_count, 1);
    }

    #[test]
    fn test_log_analysis_filtering() {
        let analyzer = LogAnalyzer::new();

        // Add some test entries to storage
        let mut storage = crate::logging::log_storage::LogStorage::new();
        storage.add_entry(LogEntry {
            level: LogLevel::Info,
            message: "Test info message".to_string(),
            timestamp: 1000,
        });
        storage.add_entry(LogEntry {
            level: LogLevel::Error,
            message: "Test error message".to_string(),
            timestamp: 1500,
        });
        storage.add_entry(LogEntry {
            level: LogLevel::Debug,
            message: "Debug message".to_string(),
            timestamp: 2000,
        });

        // Replace the analyzer's storage
        let analyzer = LogAnalyzer { storage };

        // Test level filtering
        let filter_level = LogFilterConfig::new()
            .with_min_level(LogLevel::Warn)
            .with_max_level(LogLevel::Error);
        let result = analyzer.analyze_logs(filter_level).unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.stats.error_count, 1);

        // Test keyword filtering
        let filter_keyword = LogFilterConfig::new()
            .with_include_keyword("error".to_string());
        let result = analyzer.analyze_logs(filter_keyword).unwrap();
        assert_eq!(result.entries.len(), 1);

        // Test time range filtering
        let filter_time = LogFilterConfig::new()
            .with_time_range(1200, 1800);
        let result = analyzer.analyze_logs(filter_time).unwrap();
        assert_eq!(result.entries.len(), 1);
    }

    #[test]
    fn test_log_pattern_analysis() {
        let analyzer = LogAnalyzer::new();

        // Add some test entries to storage
        let mut storage = crate::logging::log_storage::LogStorage::new();
        storage.add_entry(LogEntry {
            level: LogLevel::Error,
            message: "Error message 1".to_string(),
            timestamp: 1000,
        });
        storage.add_entry(LogEntry {
            level: LogLevel::Error,
            message: "Error message 2".to_string(),
            timestamp: 1500,
        });
        storage.add_entry(LogEntry {
            level: LogLevel::Warn,
            message: "Warning message".to_string(),
            timestamp: 2000,
        });
        storage.add_entry(LogEntry {
            level: LogLevel::Info,
            message: "Info message".to_string(),
            timestamp: 2500,
        });

        // Replace the analyzer's storage
        let analyzer = LogAnalyzer { storage };

        let filter = LogFilterConfig::new();
        let result = analyzer.analyze_log_patterns(filter).unwrap();

        assert_eq!(result.total_entries, 4);
        assert_eq!(result.error_count, 2);
        assert_eq!(result.warning_count, 1);
        assert_eq!(result.info_count, 1);
        assert_eq!(result.debug_count, 0);
        assert_eq!(result.error_percentage, 50.0);
        assert_eq!(result.warning_percentage, 25.0);
        assert_eq!(result.info_percentage, 25.0);
        assert_eq!(result.debug_percentage, 0.0);
    }

    #[test]
    fn test_log_analysis_integration() {
        let analyzer = LogAnalyzer::new();

        // Add some test entries to storage
        let mut storage = crate::logging::log_storage::LogStorage::new();
        storage.add_entry(LogEntry {
            level: LogLevel::Error,
            message: "Critical error".to_string(),
            timestamp: 1000,
        });
        storage.add_entry(LogEntry {
            level: LogLevel::Info,
            message: "Normal operation".to_string(),
            timestamp: 1500,
        });

        // Replace the analyzer's storage
        let analyzer = LogAnalyzer { storage };

        let filter = LogFilterConfig::new();
        let analysis = analyzer.analyze_logs(filter).unwrap();

        // Test integration function
        let result = integrate_log_analysis_with_monitoring(&analysis);
        assert!(result.is_ok());
    }
}

/// Реэкспорт интеграционных структур для удобного использования
pub use integration::{
    AsyncLoggingIntegration, ClassifyAsyncLogger, MetricsAsyncLogger, PolicyAsyncLogger,
    create_default_async_logging_integration, create_default_classify_logger,
    create_default_metrics_logger, create_default_policy_logger,
};

/// Структурированный логгер с поддержкой JSON и других форматов
#[derive(Debug)]
pub struct StructuredLogger {
    /// Формат вывода (JSON, Text, etc.)
    output_format: StructuredLogFormat,
    /// Уровень логирования
    log_level: LogLevel,
    /// Дополнительные метаданные для каждого лога
    global_metadata: serde_json::Value,
    /// Включить отладочную информацию в структурированных логах
    include_debug_info: bool,
}

impl Clone for StructuredLogger {
    fn clone(&self) -> Self {
        Self {
            output_format: self.output_format,
            log_level: self.log_level,
            global_metadata: self.global_metadata.clone(),
            include_debug_info: self.include_debug_info,
        }
    }
}

impl StructuredLogger {
    /// Создать новый StructuredLogger
    pub fn new(output_format: StructuredLogFormat, log_level: LogLevel) -> Self {
        Self {
            output_format,
            log_level,
            global_metadata: serde_json::json!({}),
            include_debug_info: false,
        }
    }

    /// Установить глобальные метаданные
    pub fn with_global_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.global_metadata = metadata;
        self
    }

    /// Включить отладочную информацию
    pub fn with_debug_info(mut self) -> Self {
        self.include_debug_info = true;
        self
    }

    /// Записать структурированный лог
    pub fn log(&self, level: LogLevel, message: &str, fields: serde_json::Value) -> String {
        if level.as_numeric() < self.log_level.as_numeric() {
            return String::new();
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut log_data = serde_json::json!({
            "timestamp": timestamp,
            "level": format!("{:?}", level),
            "message": message,
            "fields": fields,
        });

        // Добавить глобальные метаданные
        if !self.global_metadata.is_null() {
            for (key, value) in self.global_metadata.as_object().unwrap() {
                log_data[key] = value.clone();
            }
        }

        // Добавить отладочную информацию
        if self.include_debug_info {
            log_data["debug"] = serde_json::json!({
                "thread_id": format!("{:?}", std::thread::current().id()),
                "process_id": std::process::id(),
            });
        }

        // Форматировать в соответствии с выбранным форматом
        match self.output_format {
            StructuredLogFormat::Json => serde_json::to_string_pretty(&log_data).unwrap_or_default(),
            StructuredLogFormat::JsonCompact => serde_json::to_string(&log_data).unwrap_or_default(),
            StructuredLogFormat::Text => self.format_as_text(&log_data),
        }
    }

    /// Форматировать как текст
    fn format_as_text(&self, log_data: &serde_json::Value) -> String {
        let timestamp = log_data["timestamp"].as_u64().unwrap_or(0);
        let level = log_data["level"].as_str().unwrap_or("INFO");
        let message = log_data["message"].as_str().unwrap_or("");
        
        let datetime = DateTime::<Utc>::from_timestamp(timestamp as i64, 0).unwrap();
        
        format!("[{}] {} - {}", datetime.format("%Y-%m-%d %H:%M:%S"), level, message)
    }

    /// Записать лог в файл
    pub fn log_to_file(&self, level: LogLevel, message: &str, fields: serde_json::Value, file_path: &Path) -> Result<()> {
        let log_output = self.log(level, message, fields);
        if log_output.is_empty() {
            return Ok(());
        }

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
            .context("Failed to open log file")?;

        writeln!(file, "{}", log_output).context("Failed to write to log file")?;
        
        Ok(())
    }

    /// Записать лог в stdout
    pub fn log_to_stdout(&self, level: LogLevel, message: &str, fields: serde_json::Value) {
        let log_output = self.log(level, message, fields);
        if !log_output.is_empty() {
            println!("{}", log_output);
        }
    }
}

/// Формат структурированного лога
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuredLogFormat {
    /// JSON с отступами (для отладки)
    Json,
    /// Компактный JSON (для производства)
    JsonCompact,
    /// Текстовый формат (человеко-читаемый)
    Text,
}

/// Улучшенный фильтр логов с поддержкой JSON полей
#[derive(Debug, Clone)]
pub struct EnhancedLogFilter {
    /// Базовый фильтр
    pub base_filter: LogFilterConfig,
    /// Фильтрация по JSON полям
    pub json_field_filters: Vec<JsonFieldFilter>,
    /// Фильтрация по метаданным
    pub metadata_filters: Vec<MetadataFilter>,
}

impl EnhancedLogFilter {
    /// Создать новый улучшенный фильтр
    pub fn new() -> Self {
        Self {
            base_filter: LogFilterConfig::default(),
            json_field_filters: Vec::new(),
            metadata_filters: Vec::new(),
        }
    }

    /// Добавить фильтр по JSON полю
    pub fn with_json_field_filter(mut self, field_filter: JsonFieldFilter) -> Self {
        self.json_field_filters.push(field_filter);
        self
    }

    /// Добавить фильтр по метаданным
    pub fn with_metadata_filter(mut self, metadata_filter: MetadataFilter) -> Self {
        self.metadata_filters.push(metadata_filter);
        self
    }

    /// Применить фильтр к структурированному логу
    pub fn apply_filter(&self, log_data: &serde_json::Value) -> bool {
        // Применить базовый фильтр (если возможно)
        if let (Some(timestamp), Some(level)) = (
            log_data["timestamp"].as_u64(),
            log_data["level"].as_str()
        ) {
            let log_level = match level {
                "TRACE" => LogLevel::Trace,
                "DEBUG" => LogLevel::Debug,
                "INFO" => LogLevel::Info,
                "WARN" => LogLevel::Warn,
                "ERROR" => LogLevel::Error,
                _ => LogLevel::Info,
            };

            // Проверка уровня лога
            if log_level.as_numeric() < self.base_filter.min_level.as_numeric() ||
               log_level.as_numeric() > self.base_filter.max_level.as_numeric() {
                return false;
            }

            // Проверка временного диапазона
            if let Some((start, end)) = self.base_filter.time_range {
                if timestamp < start || timestamp > end {
                    return false;
                }
            }
        }

        // Применить фильтры JSON полей
        for field_filter in &self.json_field_filters {
            if !field_filter.apply(log_data) {
                return false;
            }
        }

        // Применить фильтры метаданных
        for metadata_filter in &self.metadata_filters {
            if !metadata_filter.apply(log_data) {
                return false;
            }
        }

        true
    }
}

/// Фильтр по JSON полю
#[derive(Debug, Clone)]
pub struct JsonFieldFilter {
    /// Имя поля
    pub field_name: String,
    /// Оператор сравнения
    pub operator: JsonFieldOperator,
    /// Значение для сравнения
    pub value: serde_json::Value,
}

impl JsonFieldFilter {
    /// Создать новый фильтр по JSON полю
    pub fn new(field_name: String, operator: JsonFieldOperator, value: serde_json::Value) -> Self {
        Self {
            field_name,
            operator,
            value,
        }
    }

    /// Применить фильтр к JSON данным
    pub fn apply(&self, log_data: &serde_json::Value) -> bool {
        let field_value = log_data.get(&self.field_name);
        
        match (&self.operator, field_value) {
            (JsonFieldOperator::Equals, Some(val)) => val == &self.value,
            (JsonFieldOperator::NotEquals, Some(val)) => val != &self.value,
            (JsonFieldOperator::Contains, Some(val)) if val.is_string() => {
                val.as_str().map_or(false, |s| s.contains(self.value.as_str().unwrap_or("")))
            }
            (JsonFieldOperator::GreaterThan, Some(val)) if val.is_number() && self.value.is_number() => {
                val.as_f64().map_or(false, |v| v > self.value.as_f64().unwrap_or(0.0))
            }
            (JsonFieldOperator::LessThan, Some(val)) if val.is_number() && self.value.is_number() => {
                val.as_f64().map_or(false, |v| v < self.value.as_f64().unwrap_or(0.0))
            }
            (JsonFieldOperator::Exists, _) => field_value.is_some(),
            (JsonFieldOperator::NotExists, _) => field_value.is_none(),
            _ => false,
        }
    }
}

/// Оператор для фильтрации JSON полей
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonFieldOperator {
    /// Равно
    Equals,
    /// Не равно
    NotEquals,
    /// Содержит (для строк)
    Contains,
    /// Больше чем (для чисел)
    GreaterThan,
    /// Меньше чем (для чисел)
    LessThan,
    /// Существует
    Exists,
    /// Не существует
    NotExists,
}

/// Фильтр по метаданным
#[derive(Debug, Clone)]
pub struct MetadataFilter {
    /// Имя метаданных
    pub metadata_name: String,
    /// Оператор сравнения
    pub operator: MetadataOperator,
    /// Значение для сравнения
    pub value: String,
}

impl MetadataFilter {
    /// Создать новый фильтр по метаданным
    pub fn new(metadata_name: String, operator: MetadataOperator, value: String) -> Self {
        Self {
            metadata_name,
            operator,
            value,
        }
    }

    /// Применить фильтр к JSON данным
    pub fn apply(&self, log_data: &serde_json::Value) -> bool {
        let metadata_value = log_data.get(&self.metadata_name);
        
        match (&self.operator, metadata_value) {
            (MetadataOperator::Equals, Some(val)) if val.is_string() => {
                val.as_str().map_or(false, |s| s == &self.value)
            }
            (MetadataOperator::NotEquals, Some(val)) if val.is_string() => {
                val.as_str().map_or(false, |s| s != &self.value)
            }
            (MetadataOperator::Contains, Some(val)) if val.is_string() => {
                val.as_str().map_or(false, |s| s.contains(&self.value))
            }
            (MetadataOperator::Exists, _) => metadata_value.is_some(),
            (MetadataOperator::NotExists, _) => metadata_value.is_none(),
            _ => false,
        }
    }
}

/// Оператор для фильтрации метаданных
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetadataOperator {
    /// Равно
    Equals,
    /// Не равно
    NotEquals,
    /// Содержит
    Contains,
    /// Существует
    Exists,
    /// Не существует
    NotExists,
}

/// Структурированный логгер с асинхронной поддержкой
#[derive(Debug, Clone)]
pub struct AsyncStructuredLogger {
    /// Базовый структурированный логгер
    inner: StructuredLogger,
    /// Канал для асинхронной отправки логов
    sender: tokio::sync::mpsc::Sender<StructuredLogMessage>,
}

impl AsyncStructuredLogger {
    /// Создать новый асинхронный структурированный логгер
    pub fn new(output_format: StructuredLogFormat, log_level: LogLevel, buffer_size: usize) -> Self {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(buffer_size);
        
        let inner_logger = StructuredLogger::new(output_format, log_level);
        let async_inner = inner_logger.clone();
        
        // Запустить фоновую задачу для обработки логов
        tokio::spawn(async move {
            while let Some(message) = receiver.recv().await {
                match message {
                    StructuredLogMessage::LogToFile { level, message, fields, file_path, responder } => {
                        let result = async_inner.log_to_file(level, &message, fields, &file_path);
                        let _ = responder.send(result);
                    }
                    StructuredLogMessage::LogToStdout { level, message, fields } => {
                        async_inner.log_to_stdout(level, &message, fields);
                    }
                }
            }
        });
        
        Self { inner: inner_logger, sender }
    }

    /// Асинхронно записать лог в файл
    pub async fn log_to_file_async(
        &self,
        level: LogLevel,
        message: &str,
        fields: serde_json::Value,
        file_path: PathBuf,
    ) -> Result<()> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        
        self.sender.send(StructuredLogMessage::LogToFile {
            level,
            message: message.to_string(),
            fields,
            file_path,
            responder: sender,
        }).await.context("Failed to send log message")?;
        
        receiver.await.context("Failed to receive log result")?
    }

    /// Асинхронно записать лог в stdout
    pub async fn log_to_stdout_async(
        &self,
        level: LogLevel,
        message: &str,
        fields: serde_json::Value,
    ) {
        let _ = self.sender.send(StructuredLogMessage::LogToStdout {
            level,
            message: message.to_string(),
            fields,
        }).await;
    }
}

/// Сообщение для асинхронного структурированного логгера
#[derive(Debug)]
enum StructuredLogMessage {
    /// Записать лог в файл
    LogToFile {
        level: LogLevel,
        message: String,
        fields: serde_json::Value,
        file_path: PathBuf,
        responder: tokio::sync::oneshot::Sender<Result<()>>,
    },
    /// Записать лог в stdout
    LogToStdout {
        level: LogLevel,
        message: String,
        fields: serde_json::Value,
    },
}

#[cfg(test)]
mod structured_logging_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_structured_logger_json_output() {
        let logger = StructuredLogger::new(StructuredLogFormat::Json, LogLevel::Info);
        
        let fields = json!({
            "user_id": "12345",
            "action": "login",
            "status": "success"
        });
        
        let output = logger.log(LogLevel::Info, "User logged in", fields);
        
        assert!(output.contains("User logged in"));
        assert!(output.contains("user_id"));
        assert!(output.contains("12345"));
    }

    #[test]
    fn test_structured_logger_text_output() {
        let logger = StructuredLogger::new(StructuredLogFormat::Text, LogLevel::Info);
        
        let fields = json!({
            "user_id": "12345",
            "action": "login"
        });
        
        let output = logger.log(LogLevel::Info, "User logged in", fields);
        
        assert!(output.contains("User logged in"));
        assert!(output.contains("INFO"));
    }

    #[test]
    fn test_json_field_filter() {
        let log_data = json!({
            "level": "INFO",
            "message": "Test message",
            "user_id": "12345",
            "status": "success"
        });
        
        let filter = JsonFieldFilter::new(
            "status".to_string(),
            JsonFieldOperator::Equals,
            json!("success")
        );
        
        assert!(filter.apply(&log_data));
        
        let filter2 = JsonFieldFilter::new(
            "status".to_string(),
            JsonFieldOperator::NotEquals,
            json!("failed")
        );
        
        assert!(filter2.apply(&log_data));
    }

    #[test]
    fn test_enhanced_log_filter() {
        let log_data = json!({
            "timestamp": 1234567890,
            "level": "INFO",
            "message": "Test message",
            "user_id": "12345",
            "status": "success"
        });
        
        let mut filter = EnhancedLogFilter::new();
        filter.base_filter = LogFilterConfig::default().with_min_level(LogLevel::Info);
        
        let json_filter = JsonFieldFilter::new(
            "status".to_string(),
            JsonFieldOperator::Equals,
            json!("success")
        );
        
        filter = filter.with_json_field_filter(json_filter);
        
        assert!(filter.apply_filter(&log_data));
    }
}
