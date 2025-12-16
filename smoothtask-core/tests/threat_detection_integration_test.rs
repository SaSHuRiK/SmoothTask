// Интеграционный тест для модуля обнаружения угроз

use smoothtask_core::health::threat_detection::*;
use chrono::Utc;

#[tokio::test]
async fn test_threat_detection_basic_functionality() {
    // Тестируем создание системы обнаружения угроз
    let config = ThreatDetectionConfig::default();
    let system = ThreatDetectionSystemImpl::new(config);
    
    // Проверяем начальное состояние
    let status = system.get_threat_status().await.unwrap();
    assert_eq!(status.threat_history.len(), 0);
    assert_eq!(status.security_score, 100.0);
    
    // Тестируем добавление угрозы
    let test_threat = ThreatDetection {
        threat_id: "test-threat-1".to_string(),
        timestamp: Utc::now(),
        threat_type: ThreatType::BehavioralAnomaly,
        severity: ThreatDetectionSeverity::High,
        status: ThreatStatus::New,
        process_name: Some("test_process".to_string()),
        process_id: Some(1234),
        description: "Test behavioral anomaly".to_string(),
        details: Some("Test details".to_string()),
        confidence_score: 85.0,
        recommendations: Some("Test recommendations".to_string()),
        resolved_time: None,
    };
    
    system.add_threat_detection(test_threat).await.unwrap();
    
    // Проверяем, что угроза добавлена
    let updated_status = system.get_threat_status().await.unwrap();
    assert_eq!(updated_status.threat_history.len(), 1);
    assert!(updated_status.security_score < 100.0); // Балл безопасности должен снизиться
    
    // Тестируем разрешение угрозы
    system.resolve_threat_detection("test-threat-1").await.unwrap();
    let final_status = system.get_threat_status().await.unwrap();
    assert_eq!(final_status.threat_history[0].status, ThreatStatus::Analyzed);
}

#[tokio::test]
async fn test_threat_detection_configuration() {
    // Тестируем конфигурацию по умолчанию
    let config = ThreatDetectionConfig::default();
    
    assert!(config.enabled);
    assert_eq!(config.check_interval, std::time::Duration::from_secs(60));
    assert_eq!(config.max_threat_history, 1000);
    assert!(config.ml_settings.enabled);
    assert_eq!(config.ml_settings.min_confidence_threshold, 70.0);
    
    // Тестируем пороги аномалий
    assert_eq!(config.anomaly_thresholds.max_new_processes_per_minute, 30);
    assert_eq!(config.anomaly_thresholds.max_cpu_usage_percent, 85.0);
    assert_eq!(config.anomaly_thresholds.max_memory_usage_percent, 75.0);
}

#[tokio::test]
async fn test_threat_types_and_severities() {
    // Тестируем типы угроз
    assert_eq!(format!("{}", ThreatType::NetworkAnomaly), "network_anomaly");
    assert_eq!(format!("{}", ThreatType::FilesystemAnomaly), "filesystem_anomaly");
    assert_eq!(format!("{}", ThreatType::ResourceAnomaly), "resource_anomaly");
    
    // Тестируем уровни серьезности
    assert_eq!(format!("{}", ThreatDetectionSeverity::Critical), "critical");
    assert_eq!(format!("{}", ThreatDetectionSeverity::High), "high");
    assert_eq!(format!("{}", ThreatDetectionSeverity::Medium), "medium");
    assert_eq!(format!("{}", ThreatDetectionSeverity::Low), "low");
    assert_eq!(format!("{}", ThreatDetectionSeverity::Info), "info");
}

#[tokio::test]
async fn test_threat_detection_statistics() {
    let config = ThreatDetectionConfig::default();
    let system = ThreatDetectionSystemImpl::new(config);
    
    // Проверяем начальную статистику
    let stats = system.get_threat_stats().await.unwrap();
    assert_eq!(stats.total_threats, 0);
    assert_eq!(stats.critical_threats, 0);
    assert_eq!(stats.high_threats, 0);
    assert_eq!(stats.medium_threats, 0);
    assert_eq!(stats.low_threats, 0);
    assert_eq!(stats.false_positives, 0);
    
    // Добавляем угрозу и проверяем обновление статистики
    let threat = ThreatDetection {
        threat_id: "stats-test-1".to_string(),
        timestamp: Utc::now(),
        threat_type: ThreatType::NetworkAnomaly,
        severity: ThreatDetectionSeverity::Critical,
        status: ThreatStatus::New,
        process_name: None,
        process_id: None,
        description: "Test threat for statistics".to_string(),
        details: None,
        confidence_score: 95.0,
        recommendations: None,
        resolved_time: None,
    };
    
    system.add_threat_detection(threat).await.unwrap();
    
    let updated_stats = system.get_threat_stats().await.unwrap();
    assert_eq!(updated_stats.total_threats, 1);
    assert_eq!(updated_stats.critical_threats, 1);
    assert!(updated_stats.average_confidence_score > 0.0);
}