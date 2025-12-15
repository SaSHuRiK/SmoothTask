// Integration tests for security monitoring functionality
use chrono::Utc;
use smoothtask_core::health::security_monitoring::*;

#[tokio::test]
async fn test_security_monitoring_comprehensive() {
    // Create security monitor with default config
    let config = SecurityMonitorConfig::default();
    let security_monitor = SecurityMonitorImpl::new(config);

    // Test basic functionality
    let security_status = security_monitor.get_security_status().await.unwrap();
    assert_eq!(security_status.overall_status, SecurityStatus::Unknown);
    assert_eq!(security_status.event_history.len(), 0);
    assert!(security_status.security_score >= 0.0);
    assert!(security_status.security_score <= 100.0);

    // Test security check
    let security_status = security_monitor.check_security().await.unwrap();
    assert_ne!(security_status.overall_status, SecurityStatus::Unknown);
    assert!(security_status.security_score >= 0.0);
    assert!(security_status.security_score <= 100.0);

    // Test security stats
    let security_stats = security_monitor.get_security_stats().await.unwrap();
    assert_eq!(security_stats.total_events, 0);
    assert_eq!(security_stats.critical_events, 0);
    assert_eq!(security_stats.high_events, 0);
    assert_eq!(security_stats.medium_events, 0);
    assert_eq!(security_stats.low_events, 0);
    assert_eq!(security_stats.confirmed_threats, 0);
    assert_eq!(security_stats.false_positives, 0);

    println!("✅ Basic security monitoring test passed");
}

#[tokio::test]
async fn test_security_event_management() {
    let config = SecurityMonitorConfig::default();
    let security_monitor = SecurityMonitorImpl::new(config);

    // Add a security event
    let event = SecurityEvent {
        event_id: "test-event-1".to_string(),
        timestamp: Utc::now(),
        event_type: SecurityEventType::SuspiciousProcess,
        severity: SecurityEventSeverity::High,
        status: SecurityEventStatus::New,
        process_name: Some("test_process".to_string()),
        process_id: Some(1234),
        description: "Test suspicious process".to_string(),
        details: Some("Test details".to_string()),
        recommendations: Some("Test recommendations".to_string()),
        resolved_time: None,
    };

    security_monitor.add_security_event(event).await.unwrap();

    // Verify event was added
    let status = security_monitor.get_security_status().await.unwrap();
    assert_eq!(status.event_history.len(), 1);
    assert_eq!(status.event_history[0].event_id, "test-event-1");

    // Resolve the event
    security_monitor
        .resolve_security_event("test-event-1")
        .await
        .unwrap();

    // Verify event was resolved
    let status = security_monitor.get_security_status().await.unwrap();
    assert_eq!(status.event_history.len(), 1);
    assert_eq!(
        status.event_history[0].status,
        SecurityEventStatus::Analyzed
    );

    // Mark as false positive
    security_monitor
        .mark_event_as_false_positive("test-event-1")
        .await
        .unwrap();

    // Verify event was marked as false positive
    let status = security_monitor.get_security_status().await.unwrap();
    assert_eq!(status.event_history.len(), 1);
    assert_eq!(
        status.event_history[0].status,
        SecurityEventStatus::FalsePositive
    );

    // Clear event history
    security_monitor.clear_event_history().await.unwrap();

    // Verify history was cleared
    let status = security_monitor.get_security_status().await.unwrap();
    assert_eq!(status.event_history.len(), 0);

    println!("✅ Security event management test passed");
}

#[tokio::test]
async fn test_suspicious_process_detection() {
    let config = SecurityMonitorConfig::default();
    let security_monitor = SecurityMonitorImpl::new(config);

    // Test detection of known suspicious processes
    assert!(security_monitor.is_suspicious_process("bitcoin"));
    assert!(security_monitor.is_suspicious_process("minerd"));
    assert!(security_monitor.is_suspicious_process("xmrig"));
    assert!(security_monitor.is_suspicious_process("masscan"));
    assert!(security_monitor.is_suspicious_process("nmap"));

    // Test that normal processes are not detected as suspicious
    assert!(!security_monitor.is_suspicious_process("smoothtaskd"));
    assert!(!security_monitor.is_suspicious_process("systemd"));
    assert!(!security_monitor.is_suspicious_process("init"));

    println!("✅ Suspicious process detection test passed");
}

#[tokio::test]
async fn test_system_process_detection() {
    let config = SecurityMonitorConfig::default();
    let security_monitor = SecurityMonitorImpl::new(config);

    // Test detection of system processes
    assert!(security_monitor.is_system_process("systemd"));
    assert!(security_monitor.is_system_process("kthreadd"));
    assert!(security_monitor.is_system_process("init"));
    assert!(security_monitor.is_system_process("smoothtaskd"));

    // Test that user processes are not detected as system processes
    assert!(!security_monitor.is_system_process("firefox"));
    assert!(!security_monitor.is_system_process("chrome"));

    println!("✅ System process detection test passed");
}

#[tokio::test]
async fn test_suspicious_behavior_patterns() {
    let config = SecurityMonitorConfig::default();
    let security_monitor = SecurityMonitorImpl::new(config);

    // Create test behavior with high values
    let mut behavior = ProcessBehavior {
        pid: 1234,
        child_count: 15,       // Above threshold
        thread_count: 150,     // Above threshold
        open_files_count: 150, // Above threshold
        network_connections_count: 0,
        start_time: None,
        parent_pid: None,
        parent_name: Some("xmrig".to_string()), // Suspicious parent
        cpu_usage: 95.0,                        // Above threshold
        memory_usage: 85.0,                     // Above threshold
        child_creation_rate: 0.0,
    };

    // Test pattern detection
    let patterns = security_monitor
        .check_suspicious_behavior_patterns(&behavior)
        .await
        .unwrap();

    // Should detect multiple patterns
    assert!(!patterns.is_empty());
    assert!(patterns
        .iter()
        .any(|p| p.pattern_type == "high_child_process_count"));
    assert!(patterns
        .iter()
        .any(|p| p.pattern_type == "high_thread_count"));
    assert!(patterns
        .iter()
        .any(|p| p.pattern_type == "high_open_files_count"));
    assert!(patterns
        .iter()
        .any(|p| p.pattern_type == "suspicious_parent_process"));
    assert!(patterns.iter().any(|p| p.pattern_type == "high_cpu_usage"));
    assert!(patterns
        .iter()
        .any(|p| p.pattern_type == "high_memory_usage"));

    println!("✅ Suspicious behavior patterns test passed");
}

#[tokio::test]
async fn test_resource_anomaly_detection() {
    let config = SecurityMonitorConfig::default();
    let security_monitor = SecurityMonitorImpl::new(config);

    // Create test behavior with anomalous values
    let mut behavior = ProcessBehavior {
        pid: 1234,
        child_count: 25,       // Above threshold
        thread_count: 250,     // Above threshold
        open_files_count: 250, // Above threshold
        network_connections_count: 0,
        start_time: None,
        parent_pid: None,
        parent_name: None,
        cpu_usage: 96.0,          // Above threshold
        memory_usage: 86.0,       // Above threshold
        child_creation_rate: 6.0, // Above threshold
    };

    // Test anomaly detection
    let patterns = security_monitor
        .detect_resource_anomaly_patterns(&behavior)
        .await
        .unwrap();

    // Should detect multiple anomaly patterns
    assert!(!patterns.is_empty());
    assert!(patterns
        .iter()
        .any(|p| p.pattern_type == "anomalous_child_process_count"));
    assert!(patterns
        .iter()
        .any(|p| p.pattern_type == "anomalous_thread_count"));
    assert!(patterns
        .iter()
        .any(|p| p.pattern_type == "anomalous_open_files_count"));
    assert!(patterns
        .iter()
        .any(|p| p.pattern_type == "anomalous_cpu_usage"));
    assert!(patterns
        .iter()
        .any(|p| p.pattern_type == "anomalous_memory_usage"));
    assert!(patterns
        .iter()
        .any(|p| p.pattern_type == "anomalous_child_creation_rate"));

    println!("✅ Resource anomaly detection test passed");
}

#[tokio::test]
async fn test_security_score_calculation() {
    let config = SecurityMonitorConfig::default();
    let security_monitor = SecurityMonitorImpl::new(config);

    // Create security monitor with some events
    let mut security_monitor_state = SecurityMonitor::default();

    // Add some unresolved events
    let event1 = SecurityEvent {
        event_id: "test-event-1".to_string(),
        timestamp: Utc::now(),
        event_type: SecurityEventType::SuspiciousProcess,
        severity: SecurityEventSeverity::Critical,
        status: SecurityEventStatus::New,
        process_name: Some("test_process".to_string()),
        process_id: Some(1234),
        description: "Test critical event".to_string(),
        details: None,
        recommendations: None,
        resolved_time: None,
    };

    let event2 = SecurityEvent {
        event_id: "test-event-2".to_string(),
        timestamp: Utc::now(),
        event_type: SecurityEventType::AnomalousResourceUsage,
        severity: SecurityEventSeverity::High,
        status: SecurityEventStatus::New,
        process_name: Some("test_process".to_string()),
        process_id: Some(5678),
        description: "Test high event".to_string(),
        details: None,
        recommendations: None,
        resolved_time: None,
    };

    security_monitor_state.event_history.push(event1);
    security_monitor_state.event_history.push(event2);

    // Calculate security score
    let score = security_monitor.calculate_security_score(&security_monitor_state);

    // Score should be reduced due to unresolved events
    assert!(score < 100.0);
    assert!(score >= 0.0);

    // With 1 critical and 1 high event, score should be around 70 (100 - 20 - 10)
    assert!(score >= 60.0);
    assert!(score <= 80.0);

    println!("✅ Security score calculation test passed");
}

#[tokio::test]
async fn test_security_status_determination() {
    let config = SecurityMonitorConfig::default();
    let security_monitor = SecurityMonitorImpl::new(config);

    // Test with no events - should be Secure
    let mut security_monitor_state = SecurityMonitor::default();
    let status = security_monitor.determine_overall_status(&security_monitor_state);
    assert_eq!(status, SecurityStatus::Secure);

    // Test with medium severity events - should be Warning
    let event = SecurityEvent {
        event_id: "test-event-1".to_string(),
        timestamp: Utc::now(),
        event_type: SecurityEventType::UnusualProcessActivity,
        severity: SecurityEventSeverity::Medium,
        status: SecurityEventStatus::New,
        process_name: Some("test_process".to_string()),
        process_id: Some(1234),
        description: "Test medium event".to_string(),
        details: None,
        recommendations: None,
        resolved_time: None,
    };

    security_monitor_state.event_history.push(event);
    let status = security_monitor.determine_overall_status(&security_monitor_state);
    assert_eq!(status, SecurityStatus::Warning);

    // Test with high severity events - should be PotentialThreat
    let event = SecurityEvent {
        event_id: "test-event-2".to_string(),
        timestamp: Utc::now(),
        event_type: SecurityEventType::SuspiciousProcess,
        severity: SecurityEventSeverity::High,
        status: SecurityEventStatus::New,
        process_name: Some("test_process".to_string()),
        process_id: Some(5678),
        description: "Test high event".to_string(),
        details: None,
        recommendations: None,
        resolved_time: None,
    };

    security_monitor_state.event_history.push(event);
    let status = security_monitor.determine_overall_status(&security_monitor_state);
    assert_eq!(status, SecurityStatus::PotentialThreat);

    // Test with critical severity events - should be CriticalThreat
    let event = SecurityEvent {
        event_id: "test-event-3".to_string(),
        timestamp: Utc::now(),
        event_type: SecurityEventType::PotentialAttack,
        severity: SecurityEventSeverity::Critical,
        status: SecurityEventStatus::New,
        process_name: Some("test_process".to_string()),
        process_id: Some(9012),
        description: "Test critical event".to_string(),
        details: None,
        recommendations: None,
        resolved_time: None,
    };

    security_monitor_state.event_history.push(event);
    let status = security_monitor.determine_overall_status(&security_monitor_state);
    assert_eq!(status, SecurityStatus::CriticalThreat);

    println!("✅ Security status determination test passed");
}
