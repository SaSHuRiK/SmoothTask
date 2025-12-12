//! Mock тесты для eBPF метрик (без реальных eBPF зависимостей)

use smoothtask_core::metrics::ebpf::{EbpfConfig, EbpfMetrics, EbpfMetricsCollector};
use std::time::Duration;

#[test]
fn test_ebpf_config_default() {
    let config = EbpfConfig::default();
    assert!(config.enable_cpu_metrics);
    assert!(config.enable_memory_metrics);
    assert!(!config.enable_syscall_monitoring);
    assert_eq!(config.collection_interval, Duration::from_secs(1));
}

#[test]
fn test_ebpf_metrics_default() {
    let metrics = EbpfMetrics::default();
    assert_eq!(metrics.cpu_usage, 0.0);
    assert_eq!(metrics.memory_usage, 0);
    assert_eq!(metrics.syscall_count, 0);
    assert_eq!(metrics.timestamp, 0);
}

#[test]
fn test_ebpf_collector_creation() {
    let config = EbpfConfig::default();
    let mut collector = EbpfMetricsCollector::new(config);

    // Проверяем, что коллектор создан успешно
    assert!(collector.initialize().is_ok());
    assert!(collector.collect_metrics().is_ok());
}

#[test]
fn test_ebpf_collector_with_custom_config() {
    let mut config = EbpfConfig::default();
    config.enable_cpu_metrics = false;
    config.enable_memory_metrics = true;
    config.collection_interval = Duration::from_secs(5);

    let mut collector = EbpfMetricsCollector::new(config);
    assert!(collector.initialize().is_ok());

    let metrics = collector.collect_metrics().unwrap();
    // Проверяем, что конфигурация применена корректно
    assert_eq!(metrics.cpu_usage, 0.0); // CPU метрики отключены
}

#[test]
fn test_ebpf_feature_detection() {
    let enabled = EbpfMetricsCollector::is_ebpf_enabled();

    // В этом тесте eBPF поддержка может быть как включена, так и отключена
    // в зависимости от того, как собран crate
    println!("eBPF поддержка: {}", enabled);

    // Главное, что функция не паникует и возвращает булево значение
    assert!(matches!(enabled, true | false));
}

#[test]
fn test_ebpf_support_check() {
    let supported = EbpfMetricsCollector::check_ebpf_support();

    // Функция должна вернуть результат без паники
    assert!(supported.is_ok());

    let supported = supported.unwrap();
    println!("Поддержка eBPF в системе: {}", supported);

    // На Linux может быть как true, так и false в зависимости от окружения
    // На других платформах должно быть false
    #[cfg(not(target_os = "linux"))]
    {
        assert_eq!(supported, false);
    }
}

#[test]
fn test_ebpf_multiple_initializations() {
    let config = EbpfConfig::default();
    let mut collector = EbpfMetricsCollector::new(config);

    // Первая инициализация
    assert!(collector.initialize().is_ok());

    // Вторая инициализация должна пройти успешно (idempotent)
    assert!(collector.initialize().is_ok());

    // Сбор метрик должен работать
    assert!(collector.collect_metrics().is_ok());
}

#[test]
fn test_ebpf_metrics_structure() {
    let metrics = EbpfMetrics {
        cpu_usage: 25.5,
        memory_usage: 1024 * 1024 * 512, // 512 MB
        syscall_count: 100,
        network_packets: 250,
        network_bytes: 1024 * 1024 * 5,
        active_connections: 10,
        gpu_usage: 0.0,
        gpu_memory_usage: 0,
        gpu_compute_units: 0,
        gpu_power_usage: 0,
        gpu_temperature: 0,
        cpu_temperature: 0,
        cpu_max_temperature: 0,
        filesystem_ops: 0,
        active_processes: 5,
        timestamp: 1234567890,
        syscall_details: None,
        network_details: None,
        connection_details: None,
        gpu_details: None,
        cpu_temperature_details: None,
        filesystem_details: None,
        process_details: None,
    };

    // Проверяем, что структура корректно хранит данные
    assert_eq!(metrics.cpu_usage, 25.5);
    assert_eq!(metrics.memory_usage, 1024 * 1024 * 512);
    assert_eq!(metrics.syscall_count, 100);
    assert_eq!(metrics.network_packets, 250);
    assert_eq!(metrics.network_bytes, 1024 * 1024 * 5);
    assert_eq!(metrics.timestamp, 1234567890);
    assert!(metrics.syscall_details.is_none());
    assert!(metrics.network_details.is_none());
}

#[test]
fn test_ebpf_config_cloning() {
    let config1 = EbpfConfig::default();
    let config2 = config1.clone();

    // Проверяем, что клонирование работает корректно
    assert_eq!(config1.enable_cpu_metrics, config2.enable_cpu_metrics);
    assert_eq!(config1.enable_memory_metrics, config2.enable_memory_metrics);
    assert_eq!(config1.collection_interval, config2.collection_interval);
}

#[test]
fn test_ebpf_metrics_cloning() {
    let metrics1 = EbpfMetrics {
        cpu_usage: 10.0,
        memory_usage: 2048,
        syscall_count: 50,
        network_packets: 100,
        network_bytes: 1024,
        active_connections: 5,
        gpu_usage: 0.0,
        gpu_memory_usage: 0,
        gpu_compute_units: 0,
        gpu_power_usage: 0,
        gpu_temperature: 0,
        cpu_temperature: 0,
        cpu_max_temperature: 0,
        filesystem_ops: 0,
        active_processes: 3,
        timestamp: 1000,
        syscall_details: None,
        network_details: None,
        connection_details: None,
        gpu_details: None,
        cpu_temperature_details: None,
        filesystem_details: None,
        process_details: None,
    };

    let metrics2 = metrics1.clone();

    // Проверяем, что клонирование работает корректно
    assert_eq!(metrics1.cpu_usage, metrics2.cpu_usage);
    assert_eq!(metrics1.memory_usage, metrics2.memory_usage);
    assert_eq!(metrics1.syscall_count, metrics2.syscall_count);
    assert_eq!(metrics1.timestamp, metrics2.timestamp);
    assert_eq!(metrics1.syscall_details, metrics2.syscall_details);
}
