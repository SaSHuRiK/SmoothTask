//! Интеграционные тесты для модуля мониторинга Thunderbolt
//!
//! Эти тесты проверяют интеграцию модуля thunderbolt_monitor с другими компонентами системы.

use smoothtask_core::metrics::thunderbolt_monitor::{ThunderboltMonitor, ThunderboltMonitorConfig};

#[test]
fn test_thunderbolt_monitor_integration_with_real_devices() {
    // Создаем конфигурацию для монитора
    let monitor_config = ThunderboltMonitorConfig {
        monitoring_interval_secs: 60,
        enable_extended_monitoring: true,
        enable_performance_monitoring: true,
        enable_problem_detection: true,
        enable_parameter_optimization: true,
        enable_auto_authorization: false,
        max_controllers: 10,
        max_devices: 50,
        latency_warning_threshold_us: 1000,
        error_warning_threshold: 10,
        warning_threshold: 5,
        enable_auto_optimization: true,
        optimization_aggressiveness: 0.5,
    };

    // Создаем монитор Thunderbolt
    let mut monitor = ThunderboltMonitor::new(monitor_config);

    // Собираем метрики мониторинга
    let result = monitor.collect_thunderbolt_metrics();
    
    assert!(result.is_ok());
    let metrics = result.unwrap();

    // Проверяем, что метрики собраны корректно
    assert!(metrics.total_controllers > 0);
    assert!(metrics.total_devices > 0);
    assert!(metrics.active_connections > 0);
    assert!(metrics.total_throughput_mbps > 0.0);
    assert!(metrics.average_latency_us >= 0.0);
    assert!(!metrics.topologies.is_empty());

    // Проверяем, что метрики по типам устройств присутствуют
    assert!(!metrics.device_type_metrics.is_empty());

    // Проверяем, что метрики по состояниям устройств присутствуют
    assert!(!metrics.device_state_metrics.is_empty());

    // Проверяем, что можно экспортировать метрики в JSON
    let json_result = monitor.export_metrics_to_json(&metrics);
    assert!(json_result.is_ok());
    let json_string = json_result.unwrap();
    assert!(json_string.contains("total_controllers"));
    assert!(json_string.contains("total_devices"));
    assert!(json_string.contains("topologies"));
}

#[test]
fn test_thunderbolt_monitor_optimization_recommendations() {
    // Создаем конфигурацию для монитора
    let monitor_config = ThunderboltMonitorConfig {
        latency_warning_threshold_us: 500,
        error_warning_threshold: 5,
        warning_threshold: 3,
        ..Default::default()
    };
    let monitor = ThunderboltMonitor::new(monitor_config);

    // Создаем метрики с высокой задержкой
    let mut metrics = smoothtask_core::metrics::thunderbolt_monitor::ThunderboltMonitorMetrics::default();
    metrics.average_latency_us = 1500.0; // Above threshold
    metrics.error_count = 15; // Above threshold
    metrics.warning_count = 10; // Above threshold

    // Генерируем рекомендации
    let recommendations = monitor.generate_optimization_recommendations(&metrics);
    
    assert!(!recommendations.is_empty());
    assert!(recommendations.iter().any(|r| r.contains("High average Thunderbolt latency")));
    assert!(recommendations.iter().any(|r| r.contains("High error count")));
    assert!(recommendations.iter().any(|r| r.contains("High warning count")));
}

#[test]
fn test_thunderbolt_monitor_problem_detection() {
    // Создаем конфигурацию для монитора
    let monitor_config = ThunderboltMonitorConfig {
        latency_warning_threshold_us: 1000,
        error_warning_threshold: 10,
        warning_threshold: 5,
        ..Default::default()
    };
    let monitor = ThunderboltMonitor::new(monitor_config);

    // Создаем метрики с проблемами
    let mut metrics = smoothtask_core::metrics::thunderbolt_monitor::ThunderboltMonitorMetrics::default();
    metrics.average_latency_us = 2000.0; // Above threshold
    metrics.error_count = 25; // Above threshold

    // Обнаруживаем проблемы
    let problems = monitor.detect_thunderbolt_problems(&metrics);
    
    assert!(problems.is_ok());
    let problems = problems.unwrap();
    assert!(!problems.is_empty());
    assert!(problems.iter().any(|p| matches!(p.problem_type, smoothtask_core::metrics::thunderbolt_monitor::ThunderboltProblemType::HighLatency)));
    assert!(problems.iter().any(|p| matches!(p.problem_type, smoothtask_core::metrics::thunderbolt_monitor::ThunderboltProblemType::HighErrorRate)));
}

#[test]
fn test_thunderbolt_monitor_history_management() {
    // Создаем конфигурацию для монитора
    let monitor_config = ThunderboltMonitorConfig::default();
    let mut monitor = ThunderboltMonitor::with_history_size(monitor_config, 3);

    // Собираем метрики несколько раз
    for _ in 0..5 {
        let result = monitor.collect_thunderbolt_metrics();
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
fn test_thunderbolt_monitor_trends_analysis() {
    // Создаем конфигурацию для монитора
    let monitor_config = ThunderboltMonitorConfig::default();
    let mut monitor = ThunderboltMonitor::new(monitor_config);

    // Собираем начальные метрики
    let result = monitor.collect_thunderbolt_metrics();
    assert!(result.is_ok());

    // Собираем метрики еще раз для анализа трендов
    let result = monitor.collect_thunderbolt_metrics();
    assert!(result.is_ok());
    let metrics = result.unwrap();

    // Проверяем, что тренды рассчитаны
    assert!(!metrics.performance_trends.device_count_trend.is_nan());
    assert!(!metrics.performance_trends.throughput_trend.is_nan());
    assert!(!metrics.performance_trends.latency_trend.is_nan());
    assert!(!metrics.performance_trends.error_count_trend.is_nan());
    assert!(!metrics.performance_trends.warning_count_trend.is_nan());
}

#[test]
fn test_thunderbolt_monitor_controller_discovery() {
    // Создаем конфигурацию для монитора
    let monitor_config = ThunderboltMonitorConfig::default();
    let mut monitor = ThunderboltMonitor::new(monitor_config);

    // Обнаруживаем контроллеры
    let result = monitor.discover_thunderbolt_controllers();
    assert!(result.is_ok());
    let controllers = result.unwrap();
    assert!(!controllers.is_empty());

    // Проверяем, что контроллеры имеют корректные данные
    for controller in controllers {
        assert!(!controller.controller_id.is_empty());
        assert!(!controller.vendor.is_empty());
        assert!(!controller.device.is_empty());
        assert!(controller.max_speed_mbps > 0);
    }
}

#[test]
fn test_thunderbolt_monitor_device_discovery() {
    // Создаем конфигурацию для монитора
    let monitor_config = ThunderboltMonitorConfig::default();
    let mut monitor = ThunderboltMonitor::new(monitor_config);

    // Обнаруживаем контроллеры
    let controllers = monitor.discover_thunderbolt_controllers().unwrap();
    
    // Обнаруживаем устройства
    let result = monitor.discover_thunderbolt_devices(&controllers);
    assert!(result.is_ok());
    let devices = result.unwrap();
    assert!(!devices.is_empty());

    // Проверяем, что устройства имеют корректные данные
    for device in devices {
        assert!(!device.device_id.is_empty());
        assert!(!device.device_name.is_empty());
        assert!(!device.vendor.is_empty());
        assert!(device.speed_mbps > 0);
    }
}

#[test]
fn test_thunderbolt_monitor_topology_building() {
    // Создаем конфигурацию для монитора
    let monitor_config = ThunderboltMonitorConfig::default();
    let mut monitor = ThunderboltMonitor::new(monitor_config);

    // Обнаруживаем контроллеры
    let controllers = monitor.discover_thunderbolt_controllers().unwrap();
    
    // Обнаруживаем устройства
    let devices = monitor.discover_thunderbolt_devices(&controllers).unwrap();
    
    // Строим топологию
    let result = monitor.build_thunderbolt_topology(&controllers, &devices);
    assert!(result.is_ok());
    let topologies = result.unwrap();
    assert!(!topologies.is_empty());

    // Проверяем, что топология содержит контроллеры и устройства
    for topology in topologies {
        assert!(!topology.controllers.is_empty());
        assert!(!topology.devices.is_empty());
        assert!(!topology.connections.is_empty());
    }
}

#[test]
fn test_thunderbolt_monitor_optimization_parameters() {
    // Создаем конфигурацию для монитора
    let monitor_config = ThunderboltMonitorConfig::default();
    let monitor = ThunderboltMonitor::new(monitor_config);

    // Создаем метрики с низкой пропускной способностью
    let mut metrics = smoothtask_core::metrics::thunderbolt_monitor::ThunderboltMonitorMetrics::default();
    
    // Добавляем метрики для типа устройства
    let mut device_type_metrics = std::collections::HashMap::new();
    let type_metrics = smoothtask_core::metrics::thunderbolt_monitor::ThunderboltDeviceTypeMetrics {
        device_type: "Storage".to_string(),
        device_count: 2,
        total_throughput_mbps: 50.0, // Low throughput
        average_latency_us: 1500.0, // High latency
        error_count: 5,
        warning_count: 2,
    };
    device_type_metrics.insert("Storage".to_string(), type_metrics);
    metrics.device_type_metrics = device_type_metrics;

    // Оптимизируем параметры
    let optimizations = monitor.optimize_thunderbolt_parameters(&metrics);
    
    assert!(optimizations.is_ok());
    let optimizations = optimizations.unwrap();
    assert!(!optimizations.is_empty());
    
    // Проверяем, что рекомендации содержат информацию о низкой пропускной способности и высокой задержке
    for optimization in optimizations {
        if optimization.device_type == "Storage" {
            assert!(optimization.recommended_throughput_mbps > optimization.current_throughput_mbps);
            assert!(optimization.recommended_latency_us < optimization.current_latency_us);
        }
    }
}
