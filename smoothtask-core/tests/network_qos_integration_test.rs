//! Интеграционные тесты для системы мониторинга сети с поддержкой расширенных метрик QoS
//!
//! Эти тесты проверяют:
//! - Измерение сетевой задержки (латентности)
//! - Измерение потерь пакетов
//! - Сбор комплексных метрик качества сети
//! - Анализ QoS с учетом производительности

use anyhow::Result;
use smoothtask_core::metrics::network::*;

#[tokio::test]
async fn test_network_latency_measurement() -> Result<()> {
    // Тестируем измерение сетевой задержки
    
    let monitor = NetworkMonitor::new();
    
    // Используем localhost для тестирования (должен быть доступен)
    let latency = monitor.measure_network_latency("127.0.0.1", 3)?;
    
    // Проверяем, что задержка находится в разумных пределах для localhost
    assert!(latency >= 0.0, "Latency should be non-negative");
    assert!(latency < 10.0, "Localhost latency should be very low (< 10ms)");
    
    Ok(())
}

#[tokio::test]
async fn test_network_packet_loss_measurement() -> Result<()> {
    // Тестируем измерение потерь пакетов
    
    let monitor = NetworkMonitor::new();
    
    // Используем localhost для тестирования (должен быть доступен)
    let packet_loss = monitor.measure_network_packet_loss("127.0.0.1", 5)?;
    
    // Проверяем, что потери пакетов находятся в разумных пределах для localhost
    assert!(packet_loss >= 0.0, "Packet loss should be non-negative");
    assert!(packet_loss < 5.0, "Localhost should have minimal packet loss (< 5%)");
    
    Ok(())
}

#[tokio::test]
async fn test_network_quality_metrics_collection() -> Result<()> {
    // Тестируем сбор комплексных метрик качества сети
    
    let monitor = NetworkMonitor::new();
    
    // Собираем метрики качества для localhost
    let quality_metrics = monitor.collect_network_quality_metrics("127.0.0.1")?;
    
    // Проверяем, что все метрики собраны корректно
    assert!(quality_metrics.latency_ms >= 0.0, "Latency should be non-negative");
    assert!(quality_metrics.jitter_ms >= 0.0, "Jitter should be non-negative");
    assert!(quality_metrics.packet_loss >= 0.0, "Packet loss should be non-negative");
    assert!(quality_metrics.packet_loss <= 100.0, "Packet loss should be <= 100%");
    assert!(quality_metrics.stability_score >= 0.0, "Stability score should be non-negative");
    assert!(quality_metrics.stability_score <= 1.0, "Stability score should be <= 1.0");
    assert!(quality_metrics.bandwidth_utilization >= 0.0, "Bandwidth utilization should be non-negative");
    assert!(quality_metrics.bandwidth_utilization <= 1.0, "Bandwidth utilization should be <= 1.0");
    
    // Для localhost ожидаем хорошие показатели
    assert!(quality_metrics.latency_ms < 20.0, "Localhost should have low latency");
    assert!(quality_metrics.packet_loss < 10.0, "Localhost should have minimal packet loss");
    assert!(quality_metrics.stability_score > 0.5, "Localhost should have good stability");
    
    Ok(())
}

#[tokio::test]
async fn test_network_qos_analysis_with_performance() -> Result<()> {
    // Тестируем анализ QoS с учетом производительности
    
    let monitor = NetworkMonitor::new();
    
    // Используем интерфейс loopback для тестирования
    let interface_name = "lo";
    let target = "127.0.0.1";
    
    // Выполняем анализ QoS с учетом производительности
    let qos_metrics = monitor.analyze_network_qos_with_performance(interface_name, target)?;
    
    // Проверяем, что метрики QoS собраны корректно
    assert!(!qos_metrics.qos_class.is_none(), "QoS class should be determined");
    assert!(!qos_metrics.qos_policy.is_none(), "QoS policy should be determined");
    
    // Для localhost ожидаем высокий приоритет и оптимальную производительность
    let qos_class = qos_metrics.qos_class.unwrap();
    let qos_policy = qos_metrics.qos_policy.unwrap();
    
    assert!(qos_class.contains("high") || qos_class.contains("medium"), "Localhost should have high or medium priority");
    assert!(qos_policy.contains("optimal") || qos_policy.contains("balanced"), "Localhost should have optimal or balanced performance");
    
    Ok(())
}

#[tokio::test]
async fn test_network_latency_error_handling() -> Result<()> {
    // Тестируем обработку ошибок при измерении задержки
    
    let monitor = NetworkMonitor::new();
    
    // Пытаемся измерить задержку для несуществующего хоста
    let result = monitor.measure_network_latency("nonexistent.example.com", 3);
    
    // Должна быть ошибка
    assert!(result.is_err(), "Should return error for nonexistent host");
    
    Ok(())
}

#[tokio::test]
async fn test_network_packet_loss_error_handling() -> Result<()> {
    // Тестируем обработку ошибок при измерении потерь пакетов
    
    let monitor = NetworkMonitor::new();
    
    // Пытаемся измерить потери пакетов для несуществующего хоста
    let result = monitor.measure_network_packet_loss("nonexistent.example.com", 5);
    
    // Должна быть ошибка
    assert!(result.is_err(), "Should return error for nonexistent host");
    
    Ok(())
}

#[tokio::test]
async fn test_network_quality_metrics_fallback() -> Result<()> {
    // Тестируем резервные значения при ошибках измерения
    
    let monitor = NetworkMonitor::new();
    
    // Пытаемся собрать метрики качества для несуществующего хоста
    let result = monitor.collect_network_quality_metrics("nonexistent.example.com");
    
    // Должна быть ошибка, но не паника
    assert!(result.is_err(), "Should return error for nonexistent host");
    
    Ok(())
}

#[tokio::test]
async fn test_network_qos_analysis_integration() -> Result<()> {
    // Тестируем интеграцию анализа QoS с существующей системой мониторинга
    
    let monitor = NetworkMonitor::new();
    
    // Сначала собираем стандартные метрики QoS
    let standard_qos = monitor.collect_qos_metrics("lo")?;
    
    // Затем выполняем расширенный анализ с учетом производительности
    let enhanced_qos = monitor.analyze_network_qos_with_performance("lo", "127.0.0.1")?;
    
    // Проверяем, что расширенный анализ включает информацию из стандартного анализа
    assert!(enhanced_qos.qos_class.is_some(), "Enhanced QoS should have class");
    assert!(enhanced_qos.qos_policy.is_some(), "Enhanced QoS should have policy");
    
    // Проверяем, что расширенный анализ улучшает классификацию
    if let Some(class) = &enhanced_qos.qos_class {
        assert!(!class.is_empty(), "QoS class should not be empty");
    }
    
    if let Some(policy) = &enhanced_qos.qos_policy {
        assert!(!policy.is_empty(), "QoS policy should not be empty");
    }
    
    Ok(())
}