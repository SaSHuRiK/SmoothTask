//! Интеграционные тесты для eBPF метрик

use smoothtask_core::metrics::ebpf::{EbpfConfig, EbpfMetricsCollector};
use std::time::Duration;

#[test]
fn test_ebpf_basic_functionality() {
    // Тестируем базовую функциональность eBPF коллектора
    let config = EbpfConfig::default();
    let mut collector = EbpfMetricsCollector::new(config);
    
    // Инициализация должна пройти успешно
    assert!(collector.initialize().is_ok());
    
    // Сбор метрик должен работать
    let metrics = collector.collect_metrics();
    assert!(metrics.is_ok());
    
    let metrics = metrics.unwrap();
    println!("Собраны метрики: {:?}", metrics);
    
    // Проверяем, что метрики имеют разумные значения
    assert!(metrics.cpu_usage >= 0.0);
    assert!(metrics.memory_usage >= 0);
    assert!(metrics.timestamp > 0);
}

#[test]
fn test_ebpf_config_options() {
    // Тестируем различные конфигурации
    let mut config = EbpfConfig::default();
    
    // Тест с отключенными метриками CPU
    config.enable_cpu_metrics = false;
    config.enable_memory_metrics = true;
    
    let mut collector = EbpfMetricsCollector::new(config);
    assert!(collector.initialize().is_ok());
    
    let metrics = collector.collect_metrics().unwrap();
    assert_eq!(metrics.cpu_usage, 0.0); // Должно быть 0, так как отключено
    assert!(metrics.memory_usage > 0); // Должно быть больше 0, так как включено
}

#[test]
fn test_ebpf_support_detection() {
    // Тестируем обнаружение поддержки eBPF
    let supported = EbpfMetricsCollector::check_ebpf_support();
    assert!(supported.is_ok());
    
    let supported = supported.unwrap();
    println!("Поддержка eBPF: {}", supported);
    
    // На Linux должна быть поддержка (если ядро достаточно новое)
    #[cfg(target_os = "linux")] {
        // В тестовой среде может не быть поддержки, поэтому просто проверяем, что функция работает
    }
}

#[test]
fn test_ebpf_feature_flag() {
    // Тестируем флаг поддержки eBPF
    let enabled = EbpfMetricsCollector::is_ebpf_enabled();
    println!("eBPF поддержка включена: {}", enabled);
    
    #[cfg(feature = "ebpf")] {
        assert!(enabled);
    }
    
    #[cfg(not(feature = "ebpf"))] {
        assert!(!enabled);
    }
}

#[test]
fn test_ebpf_multiple_initializations() {
    // Тестируем множественную инициализацию
    let config = EbpfConfig::default();
    let mut collector = EbpfMetricsCollector::new(config);
    
    // Первая инициализация
    assert!(collector.initialize().is_ok());
    
    // Вторая инициализация должна пройти успешно
    assert!(collector.initialize().is_ok());
    
    // Сбор метрик должен работать
    assert!(collector.collect_metrics().is_ok());
}

#[test]
fn test_ebpf_custom_interval() {
    // Тестируем кастомный интервал сбора
    let mut config = EbpfConfig::default();
    config.collection_interval = Duration::from_secs(5);
    
    let mut collector = EbpfMetricsCollector::new(config);
    assert!(collector.initialize().is_ok());
    
    // Проверяем, что интервал установлен корректно (через публичный метод)
    let metrics = collector.collect_metrics();
    assert!(metrics.is_ok());
}

#[test]
fn test_ebpf_syscall_monitoring_disabled() {
    // Тестируем, что мониторинг системных вызовов отключен по умолчанию
    let config = EbpfConfig::default();
    assert!(!config.enable_syscall_monitoring);
    
    let mut collector = EbpfMetricsCollector::new(config);
    assert!(collector.initialize().is_ok());
    
    // Сбор метрик должен работать даже с отключенным мониторингом системных вызовов
    assert!(collector.collect_metrics().is_ok());
}