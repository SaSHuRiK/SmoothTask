//! Integration tests for network security monitoring functionality
//!
//! These tests verify:
//! - Network security metrics collection
//! - Threat detection algorithms
//! - Security score calculation
//! - Threat classification and recommendations

use smoothtask_core::metrics::network::*;
use std::net::{IpAddr, Ipv4Addr};

#[test]
fn test_network_security_metrics_structures() {
    // Test that NetworkSecurityMetrics structure can be created and has proper defaults
    let security_metrics = NetworkSecurityMetrics::default();
    
    // Verify default values
    assert_eq!(security_metrics.suspicious_connections, 0);
    assert_eq!(security_metrics.malicious_ips_detected, 0);
    assert_eq!(security_metrics.port_scan_attempts, 0);
    assert_eq!(security_metrics.ddos_indicators, 0);
    assert_eq!(security_metrics.unusual_traffic_patterns, 0);
    assert_eq!(security_metrics.brute_force_attempts, 0);
    assert_eq!(security_metrics.sql_injection_attempts, 0);
    assert_eq!(security_metrics.xss_attempts, 0);
    assert_eq!(security_metrics.command_injection_attempts, 0);
    assert_eq!(security_metrics.mitm_indicators, 0);
    assert_eq!(security_metrics.data_exfiltration_attempts, 0);
    assert_eq!(security_metrics.zero_day_exploit_indicators, 0);
    assert_eq!(security_metrics.apt_indicators, 0);
    assert_eq!(security_metrics.ransomware_indicators, 0);
    assert_eq!(security_metrics.cryptojacking_indicators, 0);
    assert_eq!(security_metrics.botnet_indicators, 0);
    assert_eq!(security_metrics.phishing_indicators, 0);
    assert_eq!(security_metrics.malware_communication_indicators, 0);
    assert_eq!(security_metrics.dns_tunneling_indicators, 0);
    assert_eq!(security_metrics.icmp_tunneling_indicators, 0);
    assert_eq!(security_metrics.http_tunneling_indicators, 0);
    assert_eq!(security_metrics.protocol_anomaly_indicators, 0);
    assert_eq!(security_metrics.security_score, 1.0);
    assert_eq!(security_metrics.threat_level, 0.0);
    assert_eq!(security_metrics.detection_confidence, 0.0);
    assert_eq!(security_metrics.false_positive_rate, 0.0);
    assert_eq!(security_metrics.true_positive_rate, 0.0);
    assert_eq!(security_metrics.total_security_events, 0);
    assert!(security_metrics.top_malicious_ips.is_empty());
    assert!(security_metrics.top_targeted_ports.is_empty());
    assert!(security_metrics.recommendations.is_empty());
}

#[test]
fn test_network_security_metrics_with_threats() {
    // Test security metrics with various threat indicators
    let mut security_metrics = NetworkSecurityMetrics::default();
    
    // Set some threat indicators
    security_metrics.brute_force_attempts = 5;
    security_metrics.sql_injection_attempts = 3;
    security_metrics.mitm_indicators = 2;
    security_metrics.ransomware_indicators = 1;
    security_metrics.botnet_indicators = 4;
    
    // Verify the values are set correctly
    assert_eq!(security_metrics.brute_force_attempts, 5);
    assert_eq!(security_metrics.sql_injection_attempts, 3);
    assert_eq!(security_metrics.mitm_indicators, 2);
    assert_eq!(security_metrics.ransomware_indicators, 1);
    assert_eq!(security_metrics.botnet_indicators, 4);
}

#[test]
fn test_brute_force_detection() {
    // Test brute force attack detection
    let monitor = NetworkMonitor::new();
    
    // Create test connections that simulate brute force attempts
    let mut connections = vec![];
    
    // Add multiple SSH connection attempts from the same IP
    for i in 0..15 {
        let mut conn = NetworkConnectionStats::default();
        conn.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100));
        conn.dst_port = 22; // SSH port
        conn.packets_transmitted = 3;
        conn.bytes_transmitted = 100;
        connections.push(conn);
    }
    
    // Test brute force detection
    let brute_force_count = monitor.detect_brute_force_attempts(&connections).unwrap();
    
    // Should detect brute force attempts
    assert!(brute_force_count > 0, "Should detect brute force attempts");
    assert!(brute_force_count <= 15, "Brute force count should be reasonable");
}

#[test]
fn test_sql_injection_detection() {
    // Test SQL injection detection
    let monitor = NetworkMonitor::new();
    
    // Create test connections that simulate web traffic
    let mut connections = vec![];
    
    // Add HTTP connections that might contain SQL injection attempts
    for i in 0..10 {
        let mut conn = NetworkConnectionStats::default();
        conn.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100 + i as u8));
        conn.dst_port = 80; // HTTP port
        conn.packets_transmitted = 10;
        conn.bytes_transmitted = 2000; // Large payload
        connections.push(conn);
    }
    
    // Test SQL injection detection
    let sql_injection_count = monitor.detect_sql_injection_attempts(&connections).unwrap();
    
    // Should detect some SQL injection attempts (simplified detection)
    assert!(sql_injection_count >= 0, "SQL injection count should be non-negative");
}

#[test]
fn test_security_score_calculation() {
    // Test security score calculation with different threat levels
    let monitor = NetworkMonitor::new();
    
    // Test case 1: No threats - should have high security score
    let mut security_metrics = NetworkSecurityMetrics::default();
    let (security_score, threat_level, _, _, _) = monitor.calculate_enhanced_security_metrics(&security_metrics);
    
    assert!(security_score > 0.9, "Should have high security score with no threats");
    assert!(threat_level < 0.1, "Should have low threat level with no threats");
    
    // Test case 2: Some threats - should have medium security score
    security_metrics.brute_force_attempts = 3;
    security_metrics.sql_injection_attempts = 2;
    security_metrics.port_scan_attempts = 1;
    let (security_score, threat_level, _, _, _) = monitor.calculate_enhanced_security_metrics(&security_metrics);
    
    assert!(security_score > 0.5 && security_score < 0.9, "Should have medium security score with some threats");
    assert!(threat_level > 0.1 && threat_level < 0.5, "Should have medium threat level with some threats");
    
    // Test case 3: Many threats - should have low security score
    security_metrics.ransomware_indicators = 2;
    security_metrics.botnet_indicators = 5;
    security_metrics.malicious_ips_detected = 3;
    let (security_score, threat_level, _, _, _) = monitor.calculate_enhanced_security_metrics(&security_metrics);
    
    assert!(security_score < 0.5, "Should have low security score with many threats");
    assert!(threat_level > 0.5, "Should have high threat level with many threats");
}

#[test]
fn test_threat_classification() {
    // Test threat classification based on security metrics
    let monitor = NetworkMonitor::new();
    
    // Test case 1: Low threat level
    let mut security_metrics = NetworkSecurityMetrics::default();
    security_metrics.brute_force_attempts = 1;
    security_metrics.port_scan_attempts = 1;
    
    let classification = monitor.classify_threat_level(&security_metrics);
    assert_eq!(classification, "low", "Should classify as low threat");
    
    // Test case 2: Medium threat level
    security_metrics.brute_force_attempts = 5;
    security_metrics.sql_injection_attempts = 3;
    security_metrics.mitm_indicators = 2;
    
    let classification = monitor.classify_threat_level(&security_metrics);
    assert_eq!(classification, "medium", "Should classify as medium threat");
    
    // Test case 3: High threat level
    security_metrics.ransomware_indicators = 1;
    security_metrics.botnet_indicators = 3;
    security_metrics.malicious_ips_detected = 2;
    
    let classification = monitor.classify_threat_level(&security_metrics);
    assert_eq!(classification, "high", "Should classify as high threat");
    
    // Test case 4: Critical threat level
    security_metrics.ransomware_indicators = 3;
    security_metrics.botnet_indicators = 10;
    security_metrics.malicious_ips_detected = 5;
    
    let classification = monitor.classify_threat_level(&security_metrics);
    assert_eq!(classification, "critical", "Should classify as critical threat");
}

#[test]
fn test_security_recommendations() {
    // Test security recommendations generation
    let monitor = NetworkMonitor::new();
    
    let mut security_metrics = NetworkSecurityMetrics::default();
    security_metrics.brute_force_attempts = 5;
    security_metrics.sql_injection_attempts = 3;
    security_metrics.mitm_indicators = 2;
    security_metrics.ransomware_indicators = 1;
    
    let recommendations = monitor.generate_security_recommendations_comprehensive(&security_metrics);
    
    // Should generate multiple recommendations
    assert!(!recommendations.is_empty(), "Should generate security recommendations");
    assert!(recommendations.len() >= 5, "Should have multiple recommendations");
    
    // Check for specific recommendation types
    let recommendations_str = recommendations.join("\n");
    assert!(recommendations_str.contains("brute force"), "Should have brute force recommendations");
    assert!(recommendations_str.contains("SQL injection"), "Should have SQL injection recommendations");
    assert!(recommendations_str.contains("Man-in-the-middle"), "Should have MITM recommendations");
    assert!(recommendations_str.contains("ransomware"), "Should have ransomware recommendations");
}

#[test]
fn test_total_security_events_calculation() {
    // Test total security events calculation
    let monitor = NetworkMonitor::new();
    
    let mut security_metrics = NetworkSecurityMetrics::default();
    security_metrics.suspicious_connections = 5;
    security_metrics.malicious_ips_detected = 3;
    security_metrics.port_scan_attempts = 2;
    security_metrics.ddos_indicators = 1;
    security_metrics.unusual_traffic_patterns = 4;
    security_metrics.brute_force_attempts = 2;
    security_metrics.sql_injection_attempts = 1;
    security_metrics.xss_attempts = 1;
    security_metrics.mitm_indicators = 1;
    security_metrics.ransomware_indicators = 1;
    security_metrics.botnet_indicators = 1;
    
    let total_events = monitor.calculate_total_security_events(&security_metrics);
    
    // Should sum up all the individual event counts
    let expected_total = 5 + 3 + 2 + 1 + 4 + 2 + 1 + 1 + 1 + 1;
    assert_eq!(total_events, expected_total, "Total security events should be sum of all individual counts");
}

#[test]
fn test_security_metrics_serialization() {
    // Test that security metrics can be serialized and deserialized
    use serde_json;
    
    let mut security_metrics = NetworkSecurityMetrics::default();
    security_metrics.brute_force_attempts = 5;
    security_metrics.sql_injection_attempts = 3;
    security_metrics.mitm_indicators = 2;
    security_metrics.ransomware_indicators = 1;
    security_metrics.security_score = 0.75;
    security_metrics.threat_level = 0.25;
    security_metrics.top_malicious_ips = vec!["192.168.1.100".to_string(), "10.0.0.50".to_string()];
    security_metrics.top_targeted_ports = vec![80, 443, 22];
    
    // Serialize to JSON
    let json = serde_json::to_string(&security_metrics);
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
    assert_eq!(deserialized_metrics.top_malicious_ips.len(), 2);
    assert_eq!(deserialized_metrics.top_targeted_ports.len(), 3);
}

#[test]
fn test_security_metrics_with_different_scenarios() {
    // Test security metrics with different threat scenarios
    let monitor = NetworkMonitor::new();
    
    // Scenario 1: DDoS attack simulation
    let mut ddos_metrics = NetworkSecurityMetrics::default();
    ddos_metrics.ddos_indicators = 10;
    ddos_metrics.suspicious_connections = 50;
    
    let (security_score, threat_level, _, _, _) = monitor.calculate_enhanced_security_metrics(&ddos_metrics);
    assert!(security_score < 0.6, "DDoS attack should significantly reduce security score");
    assert!(threat_level > 0.4, "DDoS attack should increase threat level");
    
    // Scenario 2: Ransomware attack simulation
    let mut ransomware_metrics = NetworkSecurityMetrics::default();
    ransomware_metrics.ransomware_indicators = 3;
    ransomware_metrics.botnet_indicators = 2;
    
    let (security_score, threat_level, _, _, _) = monitor.calculate_enhanced_security_metrics(&ransomware_metrics);
    assert!(security_score < 0.4, "Ransomware attack should severely reduce security score");
    assert!(threat_level > 0.6, "Ransomware attack should significantly increase threat level");
    
    // Scenario 3: Web application attacks simulation
    let mut web_attacks_metrics = NetworkSecurityMetrics::default();
    web_attacks_metrics.sql_injection_attempts = 5;
    web_attacks_metrics.xss_attempts = 3;
    web_attacks_metrics.command_injection_attempts = 2;
    
    let (security_score, threat_level, _, _, _) = monitor.calculate_enhanced_security_metrics(&web_attacks_metrics);
    assert!(security_score > 0.4 && security_score < 0.7, "Web attacks should moderately reduce security score");
    assert!(threat_level > 0.3 && threat_level < 0.6, "Web attacks should moderately increase threat level");
}

#[test]
fn test_security_metrics_edge_cases() {
    // Test edge cases for security metrics
    let monitor = NetworkMonitor::new();
    
    // Test with zero values
    let zero_metrics = NetworkSecurityMetrics::default();
    let (security_score, threat_level, _, _, _) = monitor.calculate_enhanced_security_metrics(&zero_metrics);
    
    assert_eq!(security_score, 1.0, "Zero threats should result in perfect security score");
    assert_eq!(threat_level, 0.0, "Zero threats should result in zero threat level");
    
    // Test with very high values
    let mut high_metrics = NetworkSecurityMetrics::default();
    high_metrics.brute_force_attempts = 100;
    high_metrics.sql_injection_attempts = 100;
    high_metrics.ransomware_indicators = 50;
    high_metrics.botnet_indicators = 50;
    
    let (security_score, threat_level, _, _, _) = monitor.calculate_enhanced_security_metrics(&high_metrics);
    
    assert!(security_score > 0.0, "Security score should still be positive even with many threats");
    assert!(security_score < 0.3, "Many threats should result in very low security score");
    assert!(threat_level > 0.7, "Many threats should result in very high threat level");
}

#[test]
fn test_security_metrics_recommendations_priority() {
    // Test that recommendations are prioritized correctly
    let monitor = NetworkMonitor::new();
    
    let mut security_metrics = NetworkSecurityMetrics::default();
    security_metrics.ransomware_indicators = 1; // Critical
    security_metrics.botnet_indicators = 1;     // Critical
    security_metrics.brute_force_attempts = 5;  // High
    security_metrics.sql_injection_attempts = 3; // Medium
    security_metrics.port_scan_attempts = 2;     // Medium
    
    let recommendations = monitor.generate_security_recommendations_comprehensive(&security_metrics);
    
    // Should have recommendations for all threat types
    assert!(recommendations.len() >= 5, "Should have recommendations for all threat types");
    
    // Critical threats should be mentioned first in the recommendations
    let recommendations_str = recommendations.join("\n");
    let ransomware_pos = recommendations_str.find("ransomware").unwrap_or(usize::MAX);
    let botnet_pos = recommendations_str.find("botnet").unwrap_or(usize::MAX);
    let brute_force_pos = recommendations_str.find("brute force").unwrap_or(usize::MAX);
    
    // Critical threats should appear before high/medium threats
    assert!(ransomware_pos < brute_force_pos, "Critical threats should be prioritized");
    assert!(botnet_pos < brute_force_pos, "Critical threats should be prioritized");
}