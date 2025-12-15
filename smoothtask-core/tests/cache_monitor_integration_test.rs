//! Интеграционные тесты для модуля мониторинга кэша
//!
//! Эти тесты проверяют интеграцию модуля cache_monitor с другими компонентами системы.

use smoothtask_core::metrics::cache::{MetricsCache, MetricsCacheConfig};
use smoothtask_core::metrics::cache_monitor::{CacheMonitor, CacheMonitorConfig};
use smoothtask_core::metrics::system::SystemMetrics;
use std::collections::HashMap;
use std::path::PathBuf;

#[test]
fn test_cache_monitor_integration_with_real_caches() {
    // Создаем конфигурацию для кэша
    let cache_config = MetricsCacheConfig {
        max_cache_size: 10,
        cache_ttl_seconds: 5,
        enable_caching: true,
        max_memory_bytes: 10_000_000,
        enable_compression: false,
        auto_cleanup_enabled: true,
        enable_performance_metrics: true,
        min_ttl_seconds: 1,
        adaptive_ttl_enabled: true,
        intelligent_ttl_enabled: true,
        max_frequent_access_ttl: 15,
        frequent_access_ttl_factor: 1.8,
        frequent_access_threshold: 1.0,
    };

    // Создаем несколько кэшей
    let cache1 = MetricsCache::new(cache_config.clone());
    let cache2 = MetricsCache::new(cache_config.clone());

    // Добавляем тестовые данные в кэши
    let metrics = SystemMetrics::default();
    let mut source_paths = HashMap::new();
    source_paths.insert("test".to_string(), PathBuf::from("/proc/test"));

    cache1.insert(
        "test_key1".to_string(),
        metrics.clone(),
        source_paths.clone(),
        "test_metrics".to_string(),
    );
    
    cache2.insert(
        "test_key2".to_string(),
        metrics.clone(),
        source_paths.clone(),
        "test_metrics".to_string(),
    );

    // Создаем монитор кэша
    let monitor_config = CacheMonitorConfig::default();
    let mut monitor = CacheMonitor::new(monitor_config);

    // Собираем метрики мониторинга
    let caches = vec![cache1, cache2];
    let result = monitor.collect_cache_metrics(&caches);
    
    assert!(result.is_ok());
    let metrics = result.unwrap();

    // Проверяем, что метрики собраны корректно
    assert_eq!(metrics.total_caches, 2);
    assert!(metrics.total_memory_usage > 0);
    assert!(metrics.overall_hit_rate >= 0.0);
    assert!(metrics.overall_miss_rate >= 0.0);
    assert_eq!(metrics.active_caches, 2);
    assert_eq!(metrics.inactive_caches, 0);

    // Проверяем, что метрики по типам присутствуют
    assert!(!metrics.cache_type_metrics.is_empty());

    // Проверяем, что метрики по приоритетам присутствуют
    assert!(!metrics.cache_priority_metrics.is_empty());

    // Проверяем, что можно экспортировать метрики в JSON
    let json_result = monitor.export_metrics_to_json(&metrics);
    assert!(json_result.is_ok());
    let json_string = json_result.unwrap();
    assert!(json_string.contains("total_caches"));
    assert!(json_string.contains("cache_type_metrics"));
}

#[test]
fn test_cache_monitor_optimization_recommendations() {
    // Создаем конфигурацию для монитора
    let monitor_config = CacheMonitorConfig {
        min_hit_rate_warning: 0.8,
        max_miss_rate_warning: 0.2,
        max_memory_usage_warning: 0.7,
        ..Default::default()
    };
    let monitor = CacheMonitor::new(monitor_config);

    // Создаем метрики с низким hit rate
    let mut metrics = smoothtask_core::metrics::cache_monitor::CacheMonitorMetrics::default();
    metrics.overall_hit_rate = 0.6; // Below threshold
    metrics.overall_miss_rate = 0.4; // Above threshold
    metrics.total_memory_usage = 1000000;
    metrics.max_cache_size = 1000000;
    metrics.inactive_caches = 5;
    metrics.total_caches = 10;

    // Генерируем рекомендации
    let recommendations = monitor.generate_optimization_recommendations(&metrics);
    
    assert!(!recommendations.is_empty());
    assert!(recommendations.iter().any(|r| r.contains("Low cache hit rate")));
    assert!(recommendations.iter().any(|r| r.contains("High cache miss rate")));
    assert!(recommendations.iter().any(|r| r.contains("High number of inactive caches")));
}

#[test]
fn test_cache_monitor_problem_detection() {
    // Создаем конфигурацию для монитора
    let monitor_config = CacheMonitorConfig {
        min_hit_rate_warning: 0.7,
        max_miss_rate_warning: 0.3,
        max_memory_usage_warning: 0.8,
        ..Default::default()
    };
    let monitor = CacheMonitor::new(monitor_config);

    // Создаем метрики с проблемами
    let mut metrics = smoothtask_core::metrics::cache_monitor::CacheMonitorMetrics::default();
    metrics.overall_hit_rate = 0.6; // Below threshold
    metrics.overall_miss_rate = 0.4; // Above threshold
    metrics.total_memory_usage = 1000000;
    metrics.max_cache_size = 1000000;

    // Обнаруживаем проблемы
    let problems = monitor.detect_cache_problems(&metrics);
    
    assert!(problems.is_ok());
    let problems = problems.unwrap();
    assert!(!problems.is_empty());
    assert!(problems.iter().any(|p| matches!(p.problem_type, smoothtask_core::metrics::cache_monitor::CacheProblemType::LowHitRate)));
    assert!(problems.iter().any(|p| matches!(p.problem_type, smoothtask_core::metrics::cache_monitor::CacheProblemType::HighMissRate)));
}

#[test]
fn test_cache_monitor_history_management() {
    // Создаем конфигурацию для монитора
    let monitor_config = CacheMonitorConfig::default();
    let mut monitor = CacheMonitor::with_history_size(monitor_config, 3);

    // Создаем тестовые кэши
    let cache_config = MetricsCacheConfig::default();
    let cache = MetricsCache::new(cache_config);

    // Собираем метрики несколько раз
    for _ in 0..5 {
        let caches = vec![cache.clone()];
        let result = monitor.collect_cache_metrics(&caches);
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
fn test_cache_monitor_trends_analysis() {
    // Создаем конфигурацию для монитора
    let monitor_config = CacheMonitorConfig::default();
    let mut monitor = CacheMonitor::new(monitor_config);

    // Создаем тестовые кэши
    let cache_config = MetricsCacheConfig::default();
    let cache = MetricsCache::new(cache_config);

    // Собираем начальные метрики
    let caches = vec![cache.clone()];
    let result = monitor.collect_cache_metrics(&caches);
    assert!(result.is_ok());

    // Собираем метрики еще раз для анализа трендов
    let result = monitor.collect_cache_metrics(&caches);
    assert!(result.is_ok());
    let metrics = result.unwrap();

    // Проверяем, что тренды рассчитаны
    assert!(!metrics.usage_trends.hit_rate_trend.is_nan());
    assert!(!metrics.usage_trends.memory_usage_trend.is_nan());
    assert!(!metrics.usage_trends.activity_trend.is_nan());
}
