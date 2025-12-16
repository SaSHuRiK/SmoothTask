//! Basic test to verify network security metrics structure and functionality

use smoothtask_core::metrics::network::*;

#[test]
fn test_network_security_metrics_basic_structure() {
    // Test that the basic structure works
    let metrics = NetworkSecurityMetrics::default();
    
    // Verify default values for basic fields
    assert_eq!(metrics.suspicious_connections, 0);
    assert_eq!(metrics.malicious_ips_detected, 0);
    assert_eq!(metrics.port_scan_attempts, 0);
    assert_eq!(metrics.ddos_indicators, 0);
    assert_eq!(metrics.unusual_traffic_patterns, 0);
    assert_eq!(metrics.security_score, 1.0);
    assert_eq!(metrics.threat_level, 0.0);
    
    // Verify default values for new fields
    assert_eq!(metrics.brute_force_attempts, 0);
    assert_eq!(metrics.sql_injection_attempts, 0);
    assert_eq!(metrics.xss_attempts, 0);
    assert_eq!(metrics.mitm_indicators, 0);
    assert_eq!(metrics.ransomware_indicators, 0);
    assert_eq!(metrics.botnet_indicators, 0);
    assert_eq!(metrics.detection_confidence, 0.0);
    assert_eq!(metrics.false_positive_rate, 0.0);
    assert_eq!(metrics.true_positive_rate, 0.0);
    assert_eq!(metrics.total_security_events, 0);
    assert!(metrics.top_malicious_ips.is_empty());
    assert!(metrics.top_targeted_ports.is_empty());
    assert!(metrics.recommendations.is_empty());
}

#[test]
fn test_network_security_metrics_with_values() {
    // Test that we can create metrics with values
    let mut metrics = NetworkSecurityMetrics::default();
    
    // Set some values
    metrics.brute_force_attempts = 5;
    metrics.sql_injection_attempts = 3;
    metrics.mitm_indicators = 2;
    metrics.ransomware_indicators = 1;
    metrics.botnet_indicators = 4;
    
    // Verify the values are set correctly
    assert_eq!(metrics.brute_force_attempts, 5);
    assert_eq!(metrics.sql_injection_attempts, 3);
    assert_eq!(metrics.mitm_indicators, 2);
    assert_eq!(metrics.ransomware_indicators, 1);
    assert_eq!(metrics.botnet_indicators, 4);
}

#[test]
fn test_network_security_metrics_serialization() {
    // Test that metrics can be serialized and deserialized
    use serde_json;
    
    let mut metrics = NetworkSecurityMetrics::default();
    metrics.brute_force_attempts = 5;
    metrics.sql_injection_attempts = 3;
    metrics.mitm_indicators = 2;
    metrics.ransomware_indicators = 1;
    metrics.security_score = 0.75;
    metrics.threat_level = 0.25;
    
    // Serialize to JSON
    let json = serde_json::to_string(&metrics);
    assert!(json.is_ok(), "Should be able to serialize security metrics");
    
    // Deserialize back
    let deserialized: Result<NetworkSecurityMetrics, _> = serde_json::from_str(&json.unwrap());
    assert!(deserialized.is_ok(), "Should be able to deserialize security metrics");
    
    let deserialized_metrics = deserialized.unwrap();
    assert_eq!(deserialized_metrics.brute_force_attempts, 5);
    assert_eq!(deserialized_metrics.sql_injection_attempts, 3);
    assert_eq!(deserialized_metrics.mitm_indicators, 2);
    assert_eq!(deserialized_metrics.ransomware_indicators, 1);
    assert_eq!(deserialized_metrics.security_score, 0.75);
    assert_eq!(deserialized_metrics.threat_level, 0.25);
}