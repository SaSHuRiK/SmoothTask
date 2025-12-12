//! Интеграционные тесты для реального сбора данных eBPF

use smoothtask_core::metrics::ebpf::{EbpfConfig, EbpfMetricsCollector};
use std::time::Duration;

#[test]
fn test_ebpf_real_cpu_metrics_collection() {
    // Тестируем реальный сбор CPU метрик через eBPF
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: false,
        enable_syscall_monitoring: false,
        enable_network_monitoring: false,
        enable_gpu_monitoring: false,
        enable_filesystem_monitoring: false,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    
    // Инициализация должна пройти успешно
    assert!(collector.initialize().is_ok());
    
    // Сбор метрик должен работать
    let metrics = collector.collect_metrics();
    assert!(metrics.is_ok());
    
    let metrics = metrics.unwrap();
    println!("Реальные CPU метрики: {:?}", metrics);
    
    // Проверяем, что метрики имеют разумные значения
    assert!(metrics.cpu_usage >= 0.0);
    assert!(metrics.cpu_usage <= 100.0); // CPU usage should be between 0-100%
}

#[test]
fn test_ebpf_real_memory_metrics_collection() {
    // Тестируем реальный сбор метрик памяти через eBPF
    let config = EbpfConfig {
        enable_cpu_metrics: false,
        enable_memory_metrics: true,
        enable_syscall_monitoring: false,
        enable_network_monitoring: false,
        enable_gpu_monitoring: false,
        enable_filesystem_monitoring: false,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    
    // Инициализация должна пройти успешно
    assert!(collector.initialize().is_ok());
    
    // Сбор метрик должен работать
    let metrics = collector.collect_metrics();
    assert!(metrics.is_ok());
    
    let metrics = metrics.unwrap();
    println!("Реальные метрики памяти: {:?}", metrics);
    
    // Проверяем, что метрики имеют разумные значения
    #[cfg(feature = "ebpf")]
    {
        assert!(metrics.memory_usage > 0); // Memory usage should be positive with real eBPF
    }
    
    #[cfg(not(feature = "ebpf"))]
    {
        // Без eBPF поддержки memory_usage будет 0
        assert_eq!(metrics.memory_usage, 0);
    }
}

#[test]
fn test_ebpf_real_syscall_monitoring() {
    // Тестируем реальный мониторинг системных вызовов через eBPF
    let config = EbpfConfig {
        enable_cpu_metrics: false,
        enable_memory_metrics: false,
        enable_syscall_monitoring: true,
        enable_network_monitoring: false,
        enable_gpu_monitoring: false,
        enable_filesystem_monitoring: false,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    
    // Инициализация должна пройти успешно
    assert!(collector.initialize().is_ok());
    
    // Сбор метрик должен работать
    let metrics = collector.collect_metrics();
    assert!(metrics.is_ok());
    
    let metrics = metrics.unwrap();
    println!("Реальные метрики системных вызовов: {:?}", metrics);
    
    // Проверяем, что метрики имеют разумные значения
    #[cfg(feature = "ebpf")]
    {
        assert!(metrics.syscall_count >= 0);
    }
    
    #[cfg(not(feature = "ebpf"))]
    {
        // Без eBPF поддержки syscall_count будет 0
        assert_eq!(metrics.syscall_count, 0);
    }
    
    // Проверяем детализированную статистику системных вызовов
    if let Some(syscall_details) = metrics.syscall_details {
        println!("Детализированная статистика системных вызовов: {:?}", syscall_details);
        assert!(!syscall_details.is_empty());
        
        for detail in syscall_details {
            assert!(detail.count > 0);
            assert!(detail.total_time_ns > 0);
            assert!(detail.avg_time_ns > 0);
        }
    }
}

#[test]
fn test_ebpf_real_network_monitoring() {
    // Тестируем реальный мониторинг сетевой активности через eBPF
    let config = EbpfConfig {
        enable_cpu_metrics: false,
        enable_memory_metrics: false,
        enable_syscall_monitoring: false,
        enable_network_monitoring: true,
        enable_gpu_monitoring: false,
        enable_filesystem_monitoring: false,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    
    // Инициализация должна пройти успешно
    assert!(collector.initialize().is_ok());
    
    // Сбор метрик должен работать
    let metrics = collector.collect_metrics();
    assert!(metrics.is_ok());
    
    let metrics = metrics.unwrap();
    println!("Реальные метрики сетевой активности: {:?}", metrics);
    
    // Проверяем, что метрики имеют разумные значения
    assert!(metrics.network_packets >= 0);
    assert!(metrics.network_bytes >= 0);
    
    // Проверяем детализированную статистику сети
    if let Some(network_details) = metrics.network_details {
        println!("Детализированная статистика сети: {:?}", network_details);
        assert!(!network_details.is_empty());
        
        for detail in network_details {
            assert!(detail.packets_sent >= 0);
            assert!(detail.packets_received >= 0);
            assert!(detail.bytes_sent >= 0);
            assert!(detail.bytes_received >= 0);
        }
    }
}

#[test]
fn test_ebpf_real_filesystem_monitoring() {
    // Тестируем реальный мониторинг файловой системы через eBPF
    let config = EbpfConfig {
        enable_cpu_metrics: false,
        enable_memory_metrics: false,
        enable_syscall_monitoring: false,
        enable_network_monitoring: false,
        enable_gpu_monitoring: false,
        enable_filesystem_monitoring: true,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    
    // Инициализация должна пройти успешно
    assert!(collector.initialize().is_ok());
    
    // Сбор метрик должен работать
    let metrics = collector.collect_metrics();
    assert!(metrics.is_ok());
    
    let metrics = metrics.unwrap();
    println!("Реальные метрики файловой системы: {:?}", metrics);
    
    // Проверяем, что метрики имеют разумные значения
    assert!(metrics.filesystem_ops >= 0);
    
    // Проверяем детализированную статистику файловой системы
    if let Some(filesystem_details) = metrics.filesystem_details {
        println!("Детализированная статистика файловой системы: {:?}", filesystem_details);
        assert!(!filesystem_details.is_empty());
        
        for detail in filesystem_details {
            assert!(detail.read_count >= 0);
            assert!(detail.write_count >= 0);
            assert!(detail.open_count >= 0);
            assert!(detail.close_count >= 0);
            assert!(detail.bytes_read >= 0);
            assert!(detail.bytes_written >= 0);
        }
    }
}

#[test]
fn test_ebpf_real_comprehensive_monitoring() {
    // Тестируем комплексный мониторинг всех метрик через eBPF
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: true,
        enable_network_monitoring: true,
        enable_filesystem_monitoring: true,
        enable_gpu_monitoring: false, // GPU monitoring may not be available in test environment
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    
    // Инициализация должна пройти успешно
    assert!(collector.initialize().is_ok());
    
    // Проверяем статистику инициализации
    let (success_count, error_count) = collector.get_initialization_stats();
    println!("Статистика инициализации: {} успешных, {} ошибок", success_count, error_count);
    
    // Сбор метрик должен работать
    let metrics = collector.collect_metrics();
    assert!(metrics.is_ok());
    
    let metrics = metrics.unwrap();
    println!("Комплексные метрики: {:?}", metrics);
    
    // Проверяем, что все метрики имеют разумные значения
    assert!(metrics.cpu_usage >= 0.0);
    assert!(metrics.cpu_usage <= 100.0);
    
    #[cfg(feature = "ebpf")]
    {
        assert!(metrics.memory_usage > 0);
        assert!(metrics.syscall_count >= 0);
        assert!(metrics.network_packets >= 0);
        assert!(metrics.network_bytes >= 0);
        assert!(metrics.filesystem_ops >= 0);
    }
    
    #[cfg(not(feature = "ebpf"))]
    {
        // Без eBPF поддержки все метрики будут 0
        assert_eq!(metrics.memory_usage, 0);
        assert_eq!(metrics.syscall_count, 0);
        assert_eq!(metrics.network_packets, 0);
        assert_eq!(metrics.network_bytes, 0);
        assert_eq!(metrics.filesystem_ops, 0);
    }
    
    // Проверяем детализированную статистику
    if let Some(syscall_details) = metrics.syscall_details {
        assert!(!syscall_details.is_empty());
    }
    
    if let Some(network_details) = metrics.network_details {
        assert!(!network_details.is_empty());
    }
    
    if let Some(filesystem_details) = metrics.filesystem_details {
        assert!(!filesystem_details.is_empty());
    }
}

#[test]
fn test_ebpf_real_performance_optimizations() {
    // Тестируем оптимизации производительности с реальными eBPF метриками
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: true,
        enable_network_monitoring: true,
        enable_filesystem_monitoring: true,
        enable_caching: true,
        enable_aggressive_caching: false,
        batch_size: 10,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    
    // Инициализация должна пройти успешно
    assert!(collector.initialize().is_ok());
    
    // Измеряем время выполнения нескольких операций
    let start_time = std::time::Instant::now();
    
    // Выполняем несколько сборов метрик
    for _ in 0..5 {
        let _ = collector.collect_metrics();
    }
    
    let duration = start_time.elapsed();
    println!("Время выполнения 5 сборов метрик: {:?}", duration);
    
    // В реальной реализации это должно быть быстро
    assert!(duration.as_secs() < 2); // Должно выполняться менее чем за 2 секунды
    
    // Тестируем кэширование
    let metrics1 = collector.collect_metrics().unwrap();
    let metrics2 = collector.collect_metrics().unwrap();
    
    // При включенном кэшировании метрики должны быть одинаковыми
    // (если не достигнут batch_size)
    assert_eq!(metrics1.cpu_usage, metrics2.cpu_usage);
    assert_eq!(metrics1.memory_usage, metrics2.memory_usage);
}

#[test]
fn test_ebpf_real_error_handling() {
    // Тестируем обработку ошибок в реальных eBPF сценариях
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: true,
        enable_network_monitoring: true,
        enable_filesystem_monitoring: true,
        enable_gpu_monitoring: true, // Это может вызвать ошибку в тестовой среде
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    
    // Инициализация должна пройти успешно даже если некоторые программы не загрузились
    assert!(collector.initialize().is_ok());
    
    // Проверяем, есть ли ошибки
    if let Some(error) = collector.get_last_error() {
        println!("Ошибка eBPF: {}", error);
        // Это нормально, если есть ошибка в тестовой среде
    }
    
    // Сбор метрик должен работать даже с ошибками
    let metrics = collector.collect_metrics();
    assert!(metrics.is_ok());
    
    // Проверяем, что метрики имеют разумные значения
    let metrics = metrics.unwrap();
    assert!(metrics.cpu_usage >= 0.0);
    
    #[cfg(feature = "ebpf")]
    {
        assert!(metrics.memory_usage > 0);
    }
    
    #[cfg(not(feature = "ebpf"))]
    {
        // Без eBPF поддержки memory_usage будет 0
        assert_eq!(metrics.memory_usage, 0);
    }
}

#[test]
fn test_ebpf_real_multiple_collectors() {
    // Тестируем работу нескольких коллекторов одновременно
    let config1 = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: false,
        ..Default::default()
    };
    
    let config2 = EbpfConfig {
        enable_cpu_metrics: false,
        enable_memory_metrics: true,
        ..Default::default()
    };

    let mut collector1 = EbpfMetricsCollector::new(config1);
    let mut collector2 = EbpfMetricsCollector::new(config2);
    
    // Оба коллектора должны инициализироваться успешно
    assert!(collector1.initialize().is_ok());
    assert!(collector2.initialize().is_ok());
    
    // Оба коллектора должны собирать метрики
    let metrics1 = collector1.collect_metrics();
    let metrics2 = collector2.collect_metrics();
    
    assert!(metrics1.is_ok());
    assert!(metrics2.is_ok());
    
    let metrics1 = metrics1.unwrap();
    let metrics2 = metrics2.unwrap();
    
    // Проверяем, что метрики соответствуют конфигурации
    assert!(metrics1.cpu_usage >= 0.0);
    assert_eq!(metrics1.memory_usage, 0); // Memory metrics disabled
    
    assert_eq!(metrics2.cpu_usage, 0.0); // CPU metrics disabled
    
    #[cfg(feature = "ebpf")]
    {
        assert!(metrics2.memory_usage > 0); // Memory metrics enabled
    }
    
    #[cfg(not(feature = "ebpf"))]
    {
        // Без eBPF поддержки memory_usage будет 0 даже если включено
        assert_eq!(metrics2.memory_usage, 0);
    }
}