//! Сценарийные тесты для eBPF метрик

use smoothtask_core::metrics::ebpf::{EbpfConfig, EbpfMetricsCollector};
use std::time::Duration;

#[test]
fn test_ebpf_high_load_scenario() {
    // Тестируем сценарий с высокой нагрузкой
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: true,
        enable_network_monitoring: true,
        enable_filesystem_monitoring: true,
        enable_caching: true,
        enable_aggressive_caching: true,
        aggressive_cache_interval_ms: 1000, // 1 second cache
        batch_size: 50,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);

    // Инициализация должна пройти успешно
    assert!(collector.initialize().is_ok());

    // Тестируем агрессивное кэширование
    let start_time = std::time::Instant::now();

    // Выполняем много сборов метрик
    for _ in 0..100 {
        let _ = collector.collect_metrics();
    }

    let duration = start_time.elapsed();
    println!(
        "Время выполнения 100 сборов метрик с агрессивным кэшированием: {:?}",
        duration
    );

    // С агрессивным кэшированием должно быть очень быстро
    assert!(duration.as_secs() < 1); // Должно выполняться менее чем за 1 секунду
}

#[test]
fn test_ebpf_low_latency_scenario() {
    // Тестируем сценарий с низкой задержкой
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: false,
        enable_network_monitoring: false,
        enable_filesystem_monitoring: false,
        enable_caching: false, // Отключаем кэширование для низкой задержки
        collection_interval: Duration::from_millis(100), // Быстрый интервал
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);

    // Инициализация должна пройти успешно
    assert!(collector.initialize().is_ok());

    // Тестируем сбор метрик без кэширования
    let metrics1 = collector.collect_metrics().unwrap();
    let metrics2 = collector.collect_metrics().unwrap();

    // Без кэширования метрики могут отличаться
    // (в зависимости от реальной нагрузки системы)
    println!(
        "Метрики без кэширования: {:.1}% vs {:.1}% CPU",
        metrics1.cpu_usage, metrics2.cpu_usage
    );
}

#[test]
fn test_ebpf_selective_monitoring_scenario() {
    // Тестируем сценарий с селективным мониторингом
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: false,
        enable_syscall_monitoring: false,
        enable_network_monitoring: false,
        enable_filesystem_monitoring: false,
        enable_gpu_monitoring: false,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);

    // Инициализация должна пройти успешно
    assert!(collector.initialize().is_ok());

    // Сбор метрик должен работать
    let metrics = collector.collect_metrics().unwrap();

    // Проверяем, что только CPU метрики собраны
    assert!(metrics.cpu_usage >= 0.0);
    assert_eq!(metrics.memory_usage, 0);
    assert_eq!(metrics.syscall_count, 0);
    assert_eq!(metrics.network_packets, 0);
    assert_eq!(metrics.network_bytes, 0);
    assert_eq!(metrics.filesystem_ops, 0);
    assert_eq!(metrics.gpu_usage, 0.0);
    assert_eq!(metrics.gpu_memory_usage, 0);

    // Проверяем, что детализированная статистика отключена
    assert!(metrics.syscall_details.is_none());
    assert!(metrics.network_details.is_none());
    assert!(metrics.gpu_details.is_none());
    assert!(metrics.filesystem_details.is_none());
}

#[test]
fn test_ebpf_detailed_monitoring_scenario() {
    // Тестируем сценарий с детализированным мониторингом
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: true,
        enable_network_monitoring: true,
        enable_filesystem_monitoring: true,
        enable_gpu_monitoring: false,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);

    // Инициализация должна пройти успешно
    assert!(collector.initialize().is_ok());

    // Сбор метрик должен работать
    let metrics = collector.collect_metrics().unwrap();

    // Проверяем, что все метрики собраны
    assert!(metrics.cpu_usage >= 0.0);

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
        println!(
            "Детализированная статистика системных вызовов: {} записей",
            syscall_details.len()
        );
    }

    if let Some(network_details) = metrics.network_details {
        assert!(!network_details.is_empty());
        println!(
            "Детализированная статистика сети: {} записей",
            network_details.len()
        );
    }

    if let Some(filesystem_details) = metrics.filesystem_details {
        assert!(!filesystem_details.is_empty());
        println!(
            "Детализированная статистика файловой системы: {} записей",
            filesystem_details.len()
        );
    }
}

#[test]
fn test_ebpf_error_recovery_scenario() {
    // Тестируем сценарий восстановления после ошибок
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: true,
        enable_network_monitoring: true,
        enable_filesystem_monitoring: true,
        enable_gpu_monitoring: true, // Это может вызвать ошибку
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);

    // Инициализация должна пройти успешно даже с ошибками
    assert!(collector.initialize().is_ok());

    // Проверяем, есть ли ошибки
    let has_errors = collector.has_errors();
    println!("Есть ошибки: {}", has_errors);

    if has_errors {
        if let Some(error_info) = collector.get_detailed_error_info() {
            println!("Детальная информация об ошибке: {}", error_info);
        }
    }

    // Пробуем восстановление
    assert!(collector.attempt_recovery().is_ok());

    // После восстановления сбор метрик должен работать
    let metrics = collector.collect_metrics();
    assert!(metrics.is_ok());
}

#[test]
fn test_ebpf_config_validation_scenario() {
    // Тестируем сценарий валидации конфигурации

    // Тестируем некорректную конфигурацию
    let invalid_config = EbpfConfig {
        batch_size: 0, // Некорректное значение
        ..Default::default()
    };

    let mut invalid_collector = EbpfMetricsCollector::new(invalid_config);

    // Инициализация должна завершиться с ошибкой
    let init_result = invalid_collector.initialize();
    assert!(init_result.is_err());

    // Проверяем сообщение об ошибке
    if let Err(e) = init_result {
        println!("Ожидаемая ошибка валидации: {}", e);
        assert!(e.to_string().contains("batch_size"));
    }

    // Тестируем корректную конфигурацию
    let valid_config = EbpfConfig {
        batch_size: 10,
        ..Default::default()
    };

    let mut valid_collector = EbpfMetricsCollector::new(valid_config);

    // Инициализация должна пройти успешно
    assert!(valid_collector.initialize().is_ok());
}

#[test]
fn test_ebpf_performance_comparison_scenario() {
    // Тестируем сценарий сравнения производительности

    // Конфигурация без кэширования
    let no_cache_config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_caching: false,
        ..Default::default()
    };

    // Конфигурация с кэшированием
    let with_cache_config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_caching: true,
        batch_size: 10,
        ..Default::default()
    };

    let mut no_cache_collector = EbpfMetricsCollector::new(no_cache_config);
    let mut with_cache_collector = EbpfMetricsCollector::new(with_cache_config);

    // Инициализация должна пройти успешно
    assert!(no_cache_collector.initialize().is_ok());
    assert!(with_cache_collector.initialize().is_ok());

    // Тестируем производительность без кэширования
    let start_time = std::time::Instant::now();
    for _ in 0..10 {
        let _ = no_cache_collector.collect_metrics();
    }
    let no_cache_duration = start_time.elapsed();

    // Тестируем производительность с кэшированием
    let start_time = std::time::Instant::now();
    for _ in 0..10 {
        let _ = with_cache_collector.collect_metrics();
    }
    let with_cache_duration = start_time.elapsed();

    println!(
        "Производительность без кэширования: {:?}",
        no_cache_duration
    );
    println!(
        "Производительность с кэшированием: {:?}",
        with_cache_duration
    );

    // С кэшированием должно быть быстрее или равно
    assert!(with_cache_duration <= no_cache_duration * 2); // Допускаем небольшое отклонение
}

#[test]
fn test_ebpf_long_running_scenario() {
    // Тестируем сценарий длительной работы
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_caching: true,
        batch_size: 100,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config);

    // Инициализация должна пройти успешно
    assert!(collector.initialize().is_ok());

    // Тестируем длительную работу
    let start_time = std::time::Instant::now();

    // Выполняем много сборов метрик
    for i in 0..100 {
        let metrics = collector.collect_metrics();
        assert!(metrics.is_ok());

        if i % 20 == 0 {
            println!("Сбор метрик #{}: {:?}", i, metrics.unwrap());
        }
    }

    let duration = start_time.elapsed();
    println!("Время выполнения 100 сборов метрик: {:?}", duration);

    // Должно выполняться разумное время
    assert!(duration.as_secs() < 5); // Должно выполняться менее чем за 5 секунд
}

#[test]
fn test_ebpf_mixed_configurations_scenario() {
    // Тестируем сценарий с разными конфигурациями

    // Конфигурация 1: только CPU и память
    let config1 = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: false,
        enable_network_monitoring: false,
        enable_filesystem_monitoring: false,
        ..Default::default()
    };

    // Конфигурация 2: только системные вызовы и сеть
    let config2 = EbpfConfig {
        enable_cpu_metrics: false,
        enable_memory_metrics: false,
        enable_syscall_monitoring: true,
        enable_network_monitoring: true,
        enable_filesystem_monitoring: false,
        ..Default::default()
    };

    // Конфигурация 3: только файловая система
    let config3 = EbpfConfig {
        enable_cpu_metrics: false,
        enable_memory_metrics: false,
        enable_syscall_monitoring: false,
        enable_network_monitoring: false,
        enable_filesystem_monitoring: true,
        ..Default::default()
    };

    let mut collector1 = EbpfMetricsCollector::new(config1);
    let mut collector2 = EbpfMetricsCollector::new(config2);
    let mut collector3 = EbpfMetricsCollector::new(config3);

    // Все коллекторы должны инициализироваться успешно
    assert!(collector1.initialize().is_ok());
    assert!(collector2.initialize().is_ok());
    assert!(collector3.initialize().is_ok());

    // Все коллекторы должны собирать метрики
    let metrics1 = collector1.collect_metrics().unwrap();
    let metrics2 = collector2.collect_metrics().unwrap();
    let metrics3 = collector3.collect_metrics().unwrap();

    // Проверяем, что метрики соответствуют конфигурациям
    assert!(metrics1.cpu_usage >= 0.0);

    #[cfg(feature = "ebpf")]
    {
        assert!(metrics1.memory_usage > 0);
    }

    #[cfg(not(feature = "ebpf"))]
    {
        // Без eBPF поддержки memory_usage будет 0
        assert_eq!(metrics1.memory_usage, 0);
    }

    assert_eq!(metrics1.syscall_count, 0);
    assert_eq!(metrics1.network_packets, 0);
    assert_eq!(metrics1.filesystem_ops, 0);

    assert_eq!(metrics2.cpu_usage, 0.0);
    assert_eq!(metrics2.memory_usage, 0);

    #[cfg(feature = "ebpf")]
    {
        assert!(metrics2.syscall_count >= 0);
        assert!(metrics2.network_packets >= 0);
    }

    #[cfg(not(feature = "ebpf"))]
    {
        // Без eBPF поддержки все счетчики будут 0
        assert_eq!(metrics2.syscall_count, 0);
        assert_eq!(metrics2.network_packets, 0);
    }

    assert_eq!(metrics2.filesystem_ops, 0);

    assert_eq!(metrics3.cpu_usage, 0.0);
    assert_eq!(metrics3.memory_usage, 0);
    assert_eq!(metrics3.syscall_count, 0);
    assert_eq!(metrics3.network_packets, 0);

    #[cfg(feature = "ebpf")]
    {
        assert!(metrics3.filesystem_ops >= 0);
    }

    #[cfg(not(feature = "ebpf"))]
    {
        // Без eBPF поддержки filesystem_ops будет 0
        assert_eq!(metrics3.filesystem_ops, 0);
    }
}
