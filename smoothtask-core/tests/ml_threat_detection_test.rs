// Integration tests for ML-based threat detection functionality
use chrono::Utc;
use smoothtask_core::health::security_monitoring::*;

#[tokio::test]
async fn test_ml_threat_detection_basic() {
    // Create security monitor with default config
    let config = SecurityMonitorConfig::default();
    let security_monitor = SecurityMonitorImpl::new(config);

    // Test basic ML threat detection functionality
    let security_status = security_monitor.get_security_status().await.unwrap();
    assert_eq!(security_status.overall_status, SecurityStatus::Unknown);
    assert_eq!(security_status.event_history.len(), 0);

    // Test ML-based threat detection
    let mut test_security_monitor = SecurityMonitor::default();
    security_monitor
        .ml_based_threat_detection(&mut test_security_monitor)
        .await
        .unwrap();

    println!("✅ Basic ML threat detection test passed");
}

#[tokio::test]
async fn test_ml_threat_detection_patterns() {
    // Create security monitor with default config
    let config = SecurityMonitorConfig::default();
    let security_monitor = SecurityMonitorImpl::new(config);

    // Test ML threat detection with specific patterns
    let behavior = ProcessBehavior {
        pid: 1234,
        child_count: 15,
        thread_count: 150,
        open_files_count: 50,
        network_connections_count: 60,
        start_time: Some(Utc::now()),
        parent_pid: Some(1),
        parent_name: Some("init".to_string()),
        cpu_usage: 92.0,
        memory_usage: 85.0,
        device_name: "test_process".to_string(),
        child_creation_rate: 3.5,
    };

    // Test ML threat detection
    let threats = security_monitor
        .detect_ml_threats(&behavior)
        .await
        .unwrap();

    // Should detect multiple threats based on the behavior
    assert!(!threats.is_empty(), "Should detect threats for anomalous behavior");

    // Check that we have the expected types of threats
    let threat_types: Vec<SecurityEventType> = threats
        .iter()
        .map(|t| t.threat_type)
        .collect();

    // Should detect anomalous resource usage and suspicious network connections
    assert!(threat_types.contains(&SecurityEventType::AnomalousResourceUsage));
    assert!(threat_types.contains(&SecurityEventType::SuspiciousNetworkConnection));

    // Check confidence scores are reasonable
    for threat in &threats {
        assert!(threat.confidence_score > 0.0);
        assert!(threat.confidence_score <= 100.0);
    }

    println!("✅ ML threat detection patterns test passed: detected {} threats", threats.len());
}

#[tokio::test]
async fn test_ml_threat_detection_high_confidence() {
    // Create security monitor with default config
    let config = SecurityMonitorConfig::default();
    let security_monitor = SecurityMonitorImpl::new(config);

    // Test ML threat detection with high-confidence patterns
    let behavior = ProcessBehavior {
        pid: 5678,
        child_count: 25,
        thread_count: 250,
        open_files_count: 250,
        network_connections_count: 120,
        start_time: Some(Utc::now()),
        parent_pid: Some(1),
        parent_name: Some("init".to_string()),
        cpu_usage: 98.0,
        memory_usage: 95.0,
        device_name: "test_process".to_string(),
        child_creation_rate: 6.0,
    };

    // Test ML threat detection
    let threats = security_monitor
        .detect_ml_threats(&behavior)
        .await
        .unwrap();

    // Should detect multiple high-confidence threats
    assert!(!threats.is_empty(), "Should detect high-confidence threats");

    // Check that all threats have high confidence scores
    for threat in &threats {
        assert!(threat.confidence_score >= 70.0, "High-confidence threat should have score >= 70");
        
        // Check severity levels are appropriate
        match threat.threat_type {
            SecurityEventType::AnomalousResourceUsage => {
                assert!(threat.severity == SecurityEventSeverity::High || 
                       threat.severity == SecurityEventSeverity::Critical);
            }
            SecurityEventType::SuspiciousNetworkConnection => {
                assert!(threat.severity == SecurityEventSeverity::High);
            }
            SecurityEventType::SuspiciousFilesystemActivity => {
                assert!(threat.severity == SecurityEventSeverity::Medium || 
                       threat.severity == SecurityEventSeverity::High);
            }
            _ => {}
        }
    }

    println!("✅ ML threat detection high-confidence test passed: detected {} high-confidence threats", threats.len());
}

#[tokio::test]
async fn test_ml_threat_detection_normal_behavior() {
    // Create security monitor with default config
    let config = SecurityMonitorConfig::default();
    let security_monitor = SecurityMonitorImpl::new(config);

    // Test ML threat detection with normal behavior (should not detect threats)
    let behavior = ProcessBehavior {
        pid: 9999,
        child_count: 2,
        thread_count: 10,
        open_files_count: 20,
        network_connections_count: 5,
        start_time: Some(Utc::now()),
        parent_pid: Some(1),
        parent_name: Some("init".to_string()),
        cpu_usage: 15.0,
        memory_usage: 20.0,
        device_name: "normal_process".to_string(),
        child_creation_rate: 0.1,
    };

    // Test ML threat detection
    let threats = security_monitor
        .detect_ml_threats(&behavior)
        .await
        .unwrap();

    // Should not detect any threats for normal behavior
    assert_eq!(threats.len(), 0, "Should not detect threats for normal behavior");

    println!("✅ ML threat detection normal behavior test passed: no threats detected for normal process");
}

#[tokio::test]
async fn test_ml_threat_event_creation() {
    // Create security monitor with default config
    let config = SecurityMonitorConfig::default();
    let security_monitor = SecurityMonitorImpl::new(config);

    // Test ML threat detection and event creation
    let behavior = ProcessBehavior {
        pid: 1111,
        child_count: 20,
        thread_count: 200,
        open_files_count: 150,
        network_connections_count: 80,
        start_time: Some(Utc::now()),
        parent_pid: Some(1),
        parent_name: Some("init".to_string()),
        cpu_usage: 95.0,
        memory_usage: 90.0,
        device_name: "suspicious_process".to_string(),
        child_creation_rate: 4.0,
    };

    // Test ML threat detection
    let threats = security_monitor
        .detect_ml_threats(&behavior)
        .await
        .unwrap();

    // Create security events from detected threats
    let mut test_security_monitor = SecurityMonitor::default();
    
    for threat in threats {
        let event = SecurityEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type: threat.threat_type,
            severity: threat.severity,
            status: SecurityEventStatus::New,
            process_name: Some("suspicious_process".to_string()),
            process_id: Some(1111),
            description: format!("ML-based threat detected: {}", threat.description),
            details: Some(format!(
                "Threat type: {}\nConfidence: {}%\nPattern: {}",
                threat.threat_type,
                threat.confidence_score,
                threat.pattern_description
            )),
            recommendations: Some(
                "Investigate this ML-detected threat immediately and take appropriate action"
                    .to_string(),
            ),
            resolved_time: None,
        };

        security_monitor.add_security_event(event).await.unwrap();
    }

    // Verify events were added
    let status = security_monitor.get_security_status().await.unwrap();
    assert!(status.event_history.len() > 0, "Should have created security events from ML threats");

    println!("✅ ML threat event creation test passed: created {} security events", status.event_history.len());
}