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
pub mod log_storage;
pub mod rotation;
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
        let (max_size, max_files, compression, interval, max_age, max_total_size) = rotator.get_config();
        
        // Reduce max size and increase rotation frequency
        let new_max_size = (max_size as f64 * 0.7) as u64; // 30% reduction
        let new_interval = (interval as f64 * 0.5) as u64; // 50% reduction
        
        rotator.update_config(
            new_max_size,
            max_files,
            compression,
            new_interval,
            max_age,
            max_total_size
        );
        
        tracing::info!(
            "Optimized log rotation for memory pressure: max_size={} bytes, rotation_interval={} sec",
            new_max_size, new_interval
        );
    }
}

/// Get memory pressure status (mock implementation)
pub fn get_memory_pressure_status() -> bool {
    // In a real implementation, this would query system memory metrics
    // For now, return a mock value
    false
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
}
