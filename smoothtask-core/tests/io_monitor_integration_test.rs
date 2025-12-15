//! Интеграционные тесты для модуля мониторинга ввода-вывода
//!
//! Эти тесты проверяют интеграцию модуля io_monitor с другими компонентами системы.

use smoothtask_core::metrics::io_monitor::{IOMonitor, IOMonitorConfig};

#[test]
fn test_io_monitor_integration_with_real_devices() {
    // Создаем конфигурацию для монитора
    let monitor_config = IOMonitorConfig {
        monitoring_interval_secs: 60,
        enable_extended_monitoring: true,
        enable_process_level_monitoring: true,
        enable_performance_analysis: true,
        enable_bottleneck_detection: true,
        enable_parameter_optimization: true,
        max_devices: 10,
        max_processes: 100,
        load_warning_threshold: 0.8,
        latency_warning_threshold_us: 10000,
        queue_length_warning_threshold: 10,
        enable_auto_optimization: true,
        optimization_aggressiveness: 0.5,
    };

    // Создаем монитор ввода-вывода
    let mut monitor = IOMonitor::new(monitor_config);

    // Собираем метрики мониторинга
    let result = monitor.collect_io_metrics();
    
    assert!(result.is_ok());
    let metrics = result.unwrap();

    // Проверяем, что метрики собраны корректно
    assert!(metrics.total_read_operations > 0);
    assert!(metrics.total_write_operations > 0);
    assert!(metrics.total_bytes_read > 0);
    assert!(metrics.total_bytes_written > 0);
    assert!(metrics.io_load >= 0.0);
    assert!(metrics.iops >= 0.0);
    assert!(metrics.throughput_bytes_per_sec >= 0.0);
    assert!(!metrics.device_metrics.is_empty());

    // Проверяем, что метрики по типам операций присутствуют
    assert!(metrics.operation_type_metrics.contains_key("read"));
    assert!(metrics.operation_type_metrics.contains_key("write"));

    // Проверяем, что метрики по приоритетам присутствуют
    assert!(!metrics.priority_metrics.is_empty());

    // Проверяем, что можно экспортировать метрики в JSON
    let json_result = monitor.export_metrics_to_json(&metrics);
    assert!(json_result.is_ok());
    let json_string = json_result.unwrap();
    assert!(json_string.contains("total_read_operations"));
    assert!(json_string.contains("total_write_operations"));
    assert!(json_string.contains("device_metrics"));
}

#[test]
fn test_io_monitor_optimization_recommendations() {
    // Создаем конфигурацию для монитора
    let monitor_config = IOMonitorConfig {
        load_warning_threshold: 0.7,
        latency_warning_threshold_us: 5000,
        queue_length_warning_threshold: 5,
        ..Default::default()
    };
    let monitor = IOMonitor::new(monitor_config);

    // Создаем метрики с высокой загрузкой
    let mut metrics = smoothtask_core::metrics::io_monitor::SystemIOMetrics::default();
    metrics.io_load = 0.9; // Above threshold
    metrics.average_read_time_us = 15000.0; // Above threshold
    metrics.average_write_time_us = 15000.0;

    // Генерируем рекомендации
    let recommendations = monitor.generate_optimization_recommendations(&metrics);
    
    assert!(!recommendations.is_empty());
    assert!(recommendations.iter().any(|r| r.contains("High I/O load")));
    assert!(recommendations.iter().any(|r| r.contains("High average I/O operation time")));
}

#[test]
fn test_io_monitor_bottleneck_detection() {
    // Создаем конфигурацию для монитора
    let monitor_config = IOMonitorConfig {
        load_warning_threshold: 0.8,
        latency_warning_threshold_us: 10000,
        queue_length_warning_threshold: 10,
        ..Default::default()
    };
    let monitor = IOMonitor::new(monitor_config);

    // Создаем метрики с проблемами
    let mut metrics = smoothtask_core::metrics::io_monitor::SystemIOMetrics::default();
    metrics.io_load = 0.9; // Above threshold

    // Обнаруживаем узкие места
    let bottlenecks = monitor.detect_io_bottlenecks(&metrics);
    
    assert!(bottlenecks.is_ok());
    let bottlenecks = bottlenecks.unwrap();
    assert!(!bottlenecks.is_empty());
    assert!(bottlenecks.iter().any(|b| matches!(b.bottleneck_type, smoothtask_core::metrics::io_monitor::IOBottleneckType::HighLoad)));
}

#[test]
fn test_io_monitor_history_management() {
    // Создаем конфигурацию для монитора
    let monitor_config = IOMonitorConfig::default();
    let mut monitor = IOMonitor::with_history_size(monitor_config, 3);

    // Собираем метрики несколько раз
    for _ in 0..5 {
        let result = monitor.collect_io_metrics();
        assert!(result.is_ok());
    }

    // Проверяем, что история не превышает максимальный размер
    assert_eq!(monitor.metrics_history.len(), 3);

    // Проверяем, что можно получить последние метрики
    let last_metrics = monitor.get_last_metrics();
    assert!(last_metrics.is_some());

    // Проверяем, что можно получить историю метрик
    let history = monitor.get_metrics_history();
    assert_eq!(history.len(), 3);

    // Проверяем, что можно очистить историю
    monitor.clear_metrics_history();
    assert_eq!(monitor.get_metrics_history().len(), 0);
}

#[test]
fn test_io_monitor_trends_analysis() {
    // Создаем конфигурацию для монитора
    let monitor_config = IOMonitorConfig::default();
    let mut monitor = IOMonitor::new(monitor_config);

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
    assert!(!metrics.performance_trends.io_load_trend.is_nan());
    assert!(!metrics.performance_trends.average_time_trend.is_nan());
    assert!(!metrics.performance_trends.queue_length_trend.is_nan());
}

#[test]
fn test_io_monitor_device_type_detection() {
    // Создаем конфигурацию для монитора
    let monitor_config = IOMonitorConfig::default();
    let monitor = IOMonitor::new(monitor_config);

    // Проверяем определение типов устройств
    assert_eq!(monitor.determine_device_type("nvme0n1"), smoothtask_core::metrics::io_monitor::DeviceType::NVMe);
    assert_eq!(monitor.determine_device_type("sda"), smoothtask_core::metrics::io_monitor::DeviceType::SSD);
    assert_eq!(monitor.determine_device_type("hda"), smoothtask_core::metrics::io_monitor::DeviceType::HDD);
    assert_eq!(monitor.determine_device_type("loop0"), smoothtask_core::metrics::io_monitor::DeviceType::Virtual);
    assert_eq!(monitor.determine_device_type("ram0"), smoothtask_core::metrics::io_monitor::DeviceType::RAMDisk);
    assert_eq!(monitor.determine_device_type("unknown"), smoothtask_core::metrics::io_monitor::DeviceType::Unknown);
}

#[test]
fn test_io_monitor_device_health_detection() {
    // Создаем конфигурацию для монитора
    let monitor_config = IOMonitorConfig::default();
    let monitor = IOMonitor::new(monitor_config);

    // Создаем тестовые статистики
    let stats = smoothtask_core::metrics::io_monitor::DiskStats {
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
    assert_eq!(health, smoothtask_core::metrics::io_monitor::DeviceHealthStatus::Healthy);

    // Тестируем с высокой задержкой
    let high_latency_stats = smoothtask_core::metrics::io_monitor::DiskStats {
        ios_in_progress: 20,
        time_spent_doing_io_ms: 10000,
        ..stats
    };
    let high_latency = monitor.determine_device_health(&high_latency_stats, 20000.0);
    assert_eq!(high_latency, smoothtask_core::metrics::io_monitor::DeviceHealthStatus::Critical);
}

#[test]
fn test_io_monitor_metrics_calculation() {
    // Создаем конфигурацию для монитора
    let monitor_config = IOMonitorConfig::default();
    let mut monitor = IOMonitor::new(monitor_config);

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
