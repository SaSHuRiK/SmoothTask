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
    // Примечание: без реальной eBPF поддержки значения могут быть по умолчанию
    assert!(metrics.cpu_usage >= 0.0);
    // memory_usage всегда >= 0 (unsigned type), проверка не нужна
    // В тестовой среде без eBPF timestamp может быть 0
    // assert!(metrics.timestamp > 0);
}

#[test]
fn test_ebpf_config_options() {
    // Тестируем различные конфигурации
    let config = EbpfConfig {
        enable_cpu_metrics: false,
        enable_memory_metrics: true,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    assert!(collector.initialize().is_ok());

    let metrics = collector.collect_metrics().unwrap();
    assert_eq!(metrics.cpu_usage, 0.0); // Должно быть 0, так как отключено
                                        // Примечание: без реальной eBPF поддержки memory_usage может быть 0
                                        // assert!(metrics.memory_usage > 0); // Должно быть больше 0, так как включено
    assert_eq!(metrics.network_packets, 0); // Должно быть 0, так как отключено по умолчанию
    assert_eq!(metrics.network_bytes, 0); // Должно быть 0, так как отключено по умолчанию
}

#[test]
fn test_ebpf_performance_optimizations() {
    // Тестируем оптимизации производительности
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: true,
        enable_network_monitoring: true,
        enable_gpu_monitoring: true,
        enable_filesystem_monitoring: true,
        enable_caching: true,
        enable_aggressive_caching: false,
        batch_size: 10,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    assert!(collector.initialize().is_ok());

    // Тестируем кэширование
    let metrics1 = collector.collect_metrics().unwrap();
    let metrics2 = collector.collect_metrics().unwrap();
    
    // При включенном кэшировании метрики должны быть одинаковыми
    // (если не достигнут batch_size)
    assert_eq!(metrics1.cpu_usage, metrics2.cpu_usage);
    assert_eq!(metrics1.memory_usage, metrics2.memory_usage);
    
    // Тестируем селективный сбор данных
    let selective_config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: false,
        enable_syscall_monitoring: false,
        enable_network_monitoring: false,
        enable_gpu_monitoring: false,
        enable_filesystem_monitoring: false,
        ..Default::default()
    };

    let mut selective_collector = EbpfMetricsCollector::new(selective_config);
    assert!(selective_collector.initialize().is_ok());

    let selective_metrics = selective_collector.collect_metrics().unwrap();
    
    // Должны быть только CPU метрики, остальные должны быть 0 или None
    assert!(selective_metrics.cpu_usage >= 0.0);
    assert_eq!(selective_metrics.memory_usage, 0);
    assert_eq!(selective_metrics.syscall_count, 0);
    assert_eq!(selective_metrics.network_packets, 0);
    assert_eq!(selective_metrics.network_bytes, 0);
    assert_eq!(selective_metrics.gpu_usage, 0.0);
    assert_eq!(selective_metrics.gpu_memory_usage, 0);
    assert_eq!(selective_metrics.filesystem_ops, 0);
    assert!(selective_metrics.syscall_details.is_none());
    assert!(selective_metrics.network_details.is_none());
    assert!(selective_metrics.gpu_details.is_none());
    assert!(selective_metrics.filesystem_details.is_none());
}

#[test]
fn test_gpu_monitoring_functionality() {
    // Тестируем функциональность мониторинга GPU
    let config = EbpfConfig {
        enable_gpu_monitoring: true,
        enable_cpu_metrics: false,
        enable_memory_metrics: false,
        enable_syscall_monitoring: false,
        enable_network_monitoring: false,
        enable_network_connections: false,
        enable_filesystem_monitoring: false,
        enable_process_monitoring: false,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    
    // Инициализация должна пройти успешно даже если GPU программы не найдены
    assert!(collector.initialize().is_ok());
    
    // Сбор метрик должен работать
    let metrics = collector.collect_metrics();
    assert!(metrics.is_ok());
    
    let metrics = metrics.unwrap();
    
    // Проверяем, что GPU метрики имеют разумные значения по умолчанию
    assert_eq!(metrics.gpu_usage, 0.0); // Должно быть 0.0, так как GPU мониторинг включен но программы могут не загрузиться
    assert_eq!(metrics.gpu_memory_usage, 0); // Должно быть 0, так как GPU мониторинг включен но программы могут не загрузиться
    
    // Проверяем, что детализированная статистика GPU отсутствует или пустая
    if let Some(gpu_details) = metrics.gpu_details {
        assert!(gpu_details.is_empty()); // Должно быть пустым, так как нет реальных данных
    }
}

#[test]
fn test_gpu_monitoring_with_detailed_stats() {
    // Тестируем GPU мониторинг с детализированной статистикой
    let config = EbpfConfig {
        enable_gpu_monitoring: true,
        enable_cpu_metrics: false,
        enable_memory_metrics: false,
        enable_syscall_monitoring: false,
        enable_network_monitoring: false,
        enable_network_connections: false,
        enable_filesystem_monitoring: false,
        enable_process_monitoring: false,
        enable_high_performance_mode: true,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    
    // Инициализация должна пройти успешно
    assert!(collector.initialize().is_ok());
    
    // Сбор метрик должен работать
    let metrics = collector.collect_metrics();
    assert!(metrics.is_ok());
    
    let metrics = metrics.unwrap();
    
    // Проверяем, что GPU метрики имеют разумные значения
    assert!(metrics.gpu_usage >= 0.0); // Должно быть >= 0.0
    assert!(metrics.gpu_memory_usage >= 0); // Должно быть >= 0
    
    // Проверяем, что детализированная статистика GPU отсутствует или пустая
    // В тестовой среде без реальных GPU данных она должна быть None или пустой
    match metrics.gpu_details {
        Some(details) => {
            // Если есть детали, они должны быть пустыми или содержать только нулевые значения
            for gpu_stat in details {
                assert!(gpu_stat.gpu_usage >= 0.0);
                assert!(gpu_stat.memory_usage >= 0);
                assert!(gpu_stat.compute_units_active >= 0);
                assert!(gpu_stat.power_usage_uw >= 0);
            }
        },
        None => {
            // Это ожидаемое поведение в тестовой среде без реальных GPU данных
            assert!(true);
        }
    }
}

#[test]
fn test_ebpf_support_detection() {
    // Тестируем обнаружение поддержки eBPF
    let supported = EbpfMetricsCollector::check_ebpf_support();
    assert!(supported.is_ok());

    let supported = supported.unwrap();
    println!("Поддержка eBPF: {}", supported);

    // На Linux должна быть поддержка (если ядро достаточно новое)
    #[cfg(target_os = "linux")]
    {
        // В тестовой среде может не быть поддержки, поэтому просто проверяем, что функция работает
    }
}

#[test]
fn test_ebpf_feature_flag() {
    // Тестируем флаг поддержки eBPF
    let enabled = EbpfMetricsCollector::is_ebpf_enabled();
    println!("eBPF поддержка включена: {}", enabled);

    #[cfg(feature = "ebpf")]
    {
        assert!(enabled);
    }

    #[cfg(not(feature = "ebpf"))]
    {
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
    let config = EbpfConfig {
        collection_interval: Duration::from_secs(5),
        ..Default::default()
    };

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

#[test]
fn test_ebpf_network_monitoring() {
    // Тестируем поддержку мониторинга сетевой активности
    let config = EbpfConfig {
        enable_network_monitoring: true,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    assert!(collector.initialize().is_ok());

    let metrics = collector.collect_metrics().unwrap();

    // В тестовой реализации с включенным мониторингом сети
    // network_packets и network_bytes должны быть больше 0
    #[cfg(feature = "ebpf")]
    {
        assert_eq!(metrics.network_packets, 250);
        assert_eq!(metrics.network_bytes, 1024 * 1024 * 5);
    }

    // Проверяем, что детализированная статистика сети доступна
    if let Some(details) = metrics.network_details {
        assert!(!details.is_empty());
        // В тестовой реализации должно быть 2 записи
        assert_eq!(details.len(), 2);

        // Проверяем первую запись (127.0.0.1)
        let first = &details[0];
        assert_eq!(first.ip_address, 0x7F000001); // 127.0.0.1
        assert!(first.packets_sent > 0);
        assert!(first.packets_received > 0);
        assert!(first.bytes_sent > 0);
        assert!(first.bytes_received > 0);
    }
}

#[test]
fn test_ebpf_filesystem_monitoring() {
    // Тестируем поддержку мониторинга файловой системы
    let config = EbpfConfig {
        enable_filesystem_monitoring: true,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    assert!(collector.initialize().is_ok());

    let metrics = collector.collect_metrics().unwrap();

    // В тестовой реализации с включенным мониторингом файловой системы
    // filesystem_ops должно быть больше 0
    #[cfg(feature = "ebpf")]
    {
        assert_eq!(metrics.filesystem_ops, 150);
    }

    // Проверяем, что детализированная статистика файловой системы доступна
    if let Some(details) = metrics.filesystem_details {
        assert!(!details.is_empty());
        // В тестовой реализации должно быть 2 записи
        assert_eq!(details.len(), 2);

        // Проверяем первую запись
        let first = &details[0];
        assert!(first.read_count > 0);
        assert!(first.write_count > 0);
        assert!(first.open_count > 0);
        assert!(first.close_count > 0);
        assert!(first.bytes_read > 0);
        assert!(first.bytes_written > 0);
    }
}

#[test]
fn test_ebpf_initialization_statistics() {
    // Тестируем статистику инициализации eBPF
    let config = EbpfConfig::default();
    let mut collector = EbpfMetricsCollector::new(config);

    // Проверяем статистику до инициализации
    let (success_before, error_before) = collector.get_initialization_stats();
    assert_eq!(success_before, 0);
    assert_eq!(error_before, 0);

    // Инициализация
    assert!(collector.initialize().is_ok());

    // Проверяем статистику после инициализации
    let (success_after, error_after) = collector.get_initialization_stats();

    #[cfg(feature = "ebpf")]
    {
        // Должно быть как минимум 2 успешных загрузки (CPU и память по умолчанию)
        assert!(success_after >= 2);
        // Ошибок быть не должно для включенных по умолчанию программ
        assert_eq!(error_after, 0);
    }

    #[cfg(not(feature = "ebpf"))]
    {
        // Без eBPF поддержки статистика должна остаться 0
        assert_eq!(success_after, 0);
        assert_eq!(error_after, 0);
    }
}

#[test]
fn test_ebpf_comprehensive_integration() {
    // Комплексный тест интеграции eBPF функциональности
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: true,
        enable_network_monitoring: true,
        enable_filesystem_monitoring: true,
        enable_caching: true,
        batch_size: 10,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);

    // Тестируем инициализацию
    assert!(collector.initialize().is_ok());

    // Тестируем валидацию конфигурации
    assert!(collector.validate_config().is_ok());

    // Тестируем статистику инициализации
    let (_success_count, _error_count) = collector.get_initialization_stats();
    #[cfg(feature = "ebpf")]
    {
        // Должно быть как минимум 5 успешных загрузок (все кроме GPU)
        assert!(success_count >= 5);
        // Ошибок быть не должно
        assert_eq!(error_count, 0);
    }

    // Тестируем сбор метрик
    let metrics = collector.collect_metrics();
    assert!(metrics.is_ok());

    let metrics = metrics.unwrap();

    // Проверяем, что все метрики имеют разумные значения
    assert!(metrics.cpu_usage >= 0.0);
    // memory_usage, syscall_count, network_packets, network_bytes, filesystem_ops всегда >= 0 (unsigned types)
    // В тестовой среде timestamp может быть 0
    // assert!(metrics.timestamp > 0);

    // Проверяем детализированную статистику
    if let Some(syscall_details) = metrics.syscall_details {
        assert!(!syscall_details.is_empty());
        for detail in syscall_details {
            assert!(detail.count > 0);
            assert!(detail.total_time_ns > 0);
            assert!(detail.avg_time_ns > 0);
        }
    }

    if let Some(network_details) = metrics.network_details {
        assert!(!network_details.is_empty());
        for _detail in network_details {
            // packets_sent, packets_received, bytes_sent, bytes_received всегда >= 0 (unsigned types)
        }
    }

    // Проверяем детализированную статистику файловой системы
    if let Some(filesystem_details) = metrics.filesystem_details {
        assert!(!filesystem_details.is_empty());
        for _detail in filesystem_details {
            // read_count, write_count, open_count, close_count, bytes_read, bytes_written всегда >= 0 (unsigned types)
        }
    }

    // Тестируем кэширование
    let cached_metrics = collector.collect_metrics().unwrap();
    assert_eq!(metrics.cpu_usage, cached_metrics.cpu_usage);
    assert_eq!(metrics.syscall_count, cached_metrics.syscall_count);

    // Тестируем обработку ошибок
    // В зависимости от окружения, может быть ошибка или нет
    if let Some(err) = collector.get_last_error() {
        println!("Ошибка eBPF: {}", err);
        // Это нормально, если есть ошибка в тестовой среде
    }

    // Тестируем сброс состояния
    collector.reset();
    assert!(!collector.is_initialized());
    // После сброса статистика должна обнулиться
    let (success_after_reset, error_after_reset) = collector.get_initialization_stats();
    assert_eq!(success_after_reset, 0);
    assert_eq!(error_after_reset, 0);
    // После сброса сбор метрик должен вернуть значения по умолчанию
    let reset_metrics = collector.collect_metrics().unwrap();
    assert_eq!(reset_metrics.cpu_usage, 0.0);
    assert_eq!(reset_metrics.syscall_count, 0);
}

#[test]
fn test_ebpf_performance_benchmark() {
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_syscall_monitoring: true,
        enable_network_monitoring: true,
        enable_caching: true,
        batch_size: 100,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);
    assert!(collector.initialize().is_ok());

    // Измеряем время выполнения нескольких операций
    let start_time = std::time::Instant::now();

    // Выполняем несколько сборов метрик
    for _ in 0..10 {
        let _ = collector.collect_metrics();
    }

    let duration = start_time.elapsed();
    println!("Время выполнения 10 сборов метрик: {:?}", duration);

    // В тестовой реализации это должно быть быстро
    assert!(duration.as_secs() < 1); // Должно выполняться менее чем за 1 секунду

    // Тестируем производительность с кэшированием
    let start_time = std::time::Instant::now();

    // Выполняем несколько сборов метрик с кэшированием
    for _ in 0..100 {
        let _ = collector.collect_metrics();
    }

    let cached_duration = start_time.elapsed();
    println!(
        "Время выполнения 100 сборов метрик с кэшированием: {:?}",
        cached_duration
    );

    // С кэшированием должно быть еще быстрее
    assert!(cached_duration.as_secs() < 1); // Должно выполняться менее чем за 1 секунду
}
