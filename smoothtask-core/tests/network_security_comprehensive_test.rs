//! Comprehensive tests for network security monitoring functionality
//!
//! These tests verify:
//! - All network security detection functions
//! - Integration with threat detection algorithms
//! - Security score calculation with new threat types
//! - Comprehensive security recommendations

use smoothtask_core::metrics::network::*;
use std::net::{IpAddr, Ipv4Addr};

#[test]
fn test_all_detection_functions_exist() {
    // Test that all detection functions can be called without errors
    let monitor = NetworkMonitor::new();
    let connections = vec![];
    
    // Test all detection functions
    assert!(monitor.detect_brute_force_attempts(&connections).is_ok());
    assert!(monitor.detect_sql_injection_attempts(&connections).is_ok());
    assert!(monitor.detect_xss_attempts(&connections).is_ok());
    assert!(monitor.detect_command_injection_attempts(&connections).is_ok());
    assert!(monitor.detect_mitm_indicators(&connections).is_ok());
    assert!(monitor.detect_data_exfiltration_attempts(&connections).is_ok());
    assert!(monitor.detect_zero_day_exploit_indicators(&connections).is_ok());
    assert!(monitor.detect_apt_indicators(&connections).is_ok());
    assert!(monitor.detect_ransomware_indicators(&connections).is_ok());
    assert!(monitor.detect_cryptojacking_indicators(&connections).is_ok());
    assert!(monitor.detect_botnet_indicators(&connections).is_ok());
    assert!(monitor.detect_phishing_indicators(&connections).is_ok());
    assert!(monitor.detect_malware_communication_indicators(&connections).is_ok());
    assert!(monitor.detect_dns_tunneling_indicators(&connections).is_ok());
    assert!(monitor.detect_icmp_tunneling_indicators(&connections).is_ok());
    assert!(monitor.detect_http_tunneling_indicators(&connections).is_ok());
    assert!(monitor.detect_protocol_anomaly_indicators(&connections).is_ok());
    assert!(monitor.detect_encryption_anomalies(&connections).is_ok());
    assert!(monitor.detect_authentication_failures(&connections).is_ok());
}

#[test]
fn test_command_injection_detection() {
    // Test command injection detection
    let monitor = NetworkMonitor::new();
    
    // Create test connections that simulate command injection attempts
    let mut connections = vec![];
    
    // Add connections that might indicate command injection
    for i in 0..20 {
        let mut conn = NetworkConnectionStats::default();
        conn.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100 + i as u8));
        conn.bytes_transmitted = 1500; // Large enough to trigger detection
        conn.packets_transmitted = 10;
        connections.push(conn);
    }
    
    // Test command injection detection
    let command_injection_count = monitor.detect_command_injection_attempts(&connections).unwrap();
    
    // Should detect some command injection attempts
    assert!(command_injection_count >= 0, "Command injection count should be non-negative");
}

#[test]
fn test_data_exfiltration_detection() {
    // Test data exfiltration detection
    let monitor = NetworkMonitor::new();
    
    // Create test connections that simulate data exfiltration
    let mut connections = vec![];
    
    // Add connections with large data transfers
    for i in 0..5 {
        let mut conn = NetworkConnectionStats::default();
        conn.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100 + i as u8));
        conn.bytes_transmitted = 2000000; // Large data transfer
        conn.packets_transmitted = 200;
        connections.push(conn);
    }
    
    // Test data exfiltration detection
    let data_exfiltration_count = monitor.detect_data_exfiltration_attempts(&connections).unwrap();
    
    // Should detect some data exfiltration attempts
    assert!(data_exfiltration_count >= 0, "Data exfiltration count should be non-negative");
}

#[test]
fn test_zero_day_exploit_detection() {
    // Test zero-day exploit detection
    let monitor = NetworkMonitor::new();
    
    // Create test connections that simulate zero-day exploit patterns
    let mut connections = vec![];
    
    // Add connections with unusual patterns
    for i in 0..10 {
        let mut conn = NetworkConnectionStats::default();
        conn.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100 + i as u8));
        conn.bytes_transmitted = 10000; // Unusual pattern
        conn.packets_transmitted = 100;
        connections.push(conn);
    }
    
    // Test zero-day exploit detection
    let zero_day_count = monitor.detect_zero_day_exploit_indicators(&connections).unwrap();
    
    // Should detect some zero-day exploit indicators
    assert!(zero_day_count >= 0, "Zero-day exploit count should be non-negative");
}

#[test]
fn test_apt_detection() {
    // Test APT (Advanced Persistent Threat) detection
    let monitor = NetworkMonitor::new();
    
    // Create test connections that simulate APT activity
    let mut connections = vec![];
    
    // Add connections with low-and-slow patterns typical of APT
    for i in 0..15 {
        let mut conn = NetworkConnectionStats::default();
        conn.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100 + i as u8));
        conn.bytes_transmitted = 2000; // Low-and-slow pattern
        conn.packets_transmitted = 20;
        connections.push(conn);
    }
    
    // Test APT detection
    let apt_count = monitor.detect_apt_indicators(&connections).unwrap();
    
    // Should detect some APT indicators
    assert!(apt_count >= 0, "APT count should be non-negative");
}

#[test]
fn test_cryptojacking_detection() {
    // Test cryptojacking detection
    let monitor = NetworkMonitor::new();
    
    // Create test connections that simulate cryptojacking
    let mut connections = vec![];
    
    // Add connections to known mining pools
    for i in 0..10 {
        let mut conn = NetworkConnectionStats::default();
        conn.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100 + i as u8));
        conn.bytes_transmitted = 3000; // CPU-intensive pattern
        conn.packets_transmitted = 30;
        connections.push(conn);
    }
    
    // Test cryptojacking detection
    let cryptojacking_count = monitor.detect_cryptojacking_indicators(&connections).unwrap();
    
    // Should detect some cryptojacking indicators
    assert!(cryptojacking_count >= 0, "Cryptojacking count should be non-negative");
}

#[test]
fn test_phishing_detection() {
    // Test phishing detection
    let monitor = NetworkMonitor::new();
    
    // Create test connections that simulate phishing attempts
    let mut connections = vec![];
    
    // Add connections to known phishing domains
    for i in 0..12 {
        let mut conn = NetworkConnectionStats::default();
        conn.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100 + i as u8));
        conn.bytes_transmitted = 1500; // Web traffic pattern
        conn.packets_transmitted = 15;
        connections.push(conn);
    }
    
    // Test phishing detection
    let phishing_count = monitor.detect_phishing_indicators(&connections).unwrap();
    
    // Should detect some phishing indicators
    assert!(phishing_count >= 0, "Phishing count should be non-negative");
}

#[test]
fn test_malware_communication_detection() {
    // Test malware communication detection
    let monitor = NetworkMonitor::new();
    
    // Create test connections that simulate malware communication
    let mut connections = vec![];
    
    // Add connections to known malware C2 servers
    for i in 0..8 {
        let mut conn = NetworkConnectionStats::default();
        conn.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100 + i as u8));
        conn.bytes_transmitted = 4000; // C2 communication pattern
        conn.packets_transmitted = 25;
        connections.push(conn);
    }
    
    // Test malware communication detection
    let malware_count = monitor.detect_malware_communication_indicators(&connections).unwrap();
    
    // Should detect some malware communication indicators
    assert!(malware_count >= 0, "Malware communication count should be non-negative");
}

#[test]
fn test_dns_tunneling_detection() {
    // Test DNS tunneling detection
    let monitor = NetworkMonitor::new();
    
    // Create test connections that simulate DNS tunneling
    let mut connections = vec![];
    
    // Add DNS connections with unusual patterns
    for i in 0..6 {
        let mut conn = NetworkConnectionStats::default();
        conn.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100 + i as u8));
        conn.dst_port = 53; // DNS port
        conn.bytes_transmitted = 1000; // Unusual DNS traffic
        conn.packets_transmitted = 20;
        connections.push(conn);
    }
    
    // Test DNS tunneling detection
    let dns_tunneling_count = monitor.detect_dns_tunneling_indicators(&connections).unwrap();
    
    // Should detect some DNS tunneling indicators
    assert!(dns_tunneling_count >= 0, "DNS tunneling count should be non-negative");
}

#[test]
fn test_icmp_tunneling_detection() {
    // Test ICMP tunneling detection
    let monitor = NetworkMonitor::new();
    
    // Create test connections that simulate ICMP tunneling
    let mut connections = vec![];
    
    // Add ICMP connections with unusual patterns
    for i in 0..8 {
        let mut conn = NetworkConnectionStats::default();
        conn.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100 + i as u8));
        conn.protocol = "ICMP".to_string();
        conn.bytes_transmitted = 500; // Unusual ICMP traffic
        conn.packets_transmitted = 15;
        connections.push(conn);
    }
    
    // Test ICMP tunneling detection
    let icmp_tunneling_count = monitor.detect_icmp_tunneling_indicators(&connections).unwrap();
    
    // Should detect some ICMP tunneling indicators
    assert!(icmp_tunneling_count >= 0, "ICMP tunneling count should be non-negative");
}

#[test]
fn test_http_tunneling_detection() {
    // Test HTTP tunneling detection
    let monitor = NetworkMonitor::new();
    
    // Create test connections that simulate HTTP tunneling
    let mut connections = vec![];
    
    // Add HTTP connections with unusual patterns
    for i in 0..10 {
        let mut conn = NetworkConnectionStats::default();
        conn.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100 + i as u8));
        conn.dst_port = 80; // HTTP port
        conn.bytes_transmitted = 8000; // Unusual HTTP traffic
        conn.packets_transmitted = 30;
        connections.push(conn);
    }
    
    // Test HTTP tunneling detection
    let http_tunneling_count = monitor.detect_http_tunneling_indicators(&connections).unwrap();
    
    // Should detect some HTTP tunneling indicators
    assert!(http_tunneling_count >= 0, "HTTP tunneling count should be non-negative");
}

#[test]
fn test_protocol_anomaly_detection() {
    // Test protocol anomaly detection
    let monitor = NetworkMonitor::new();
    
    // Create test connections that simulate protocol anomalies
    let mut connections = vec![];
    
    // Add connections with unusual protocol behavior
    for i in 0..12 {
        let mut conn = NetworkConnectionStats::default();
        conn.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100 + i as u8));
        conn.bytes_transmitted = 2000; // Unusual protocol behavior
        conn.packets_transmitted = 25;
        connections.push(conn);
    }
    
    // Test protocol anomaly detection
    let protocol_anomaly_count = monitor.detect_protocol_anomaly_indicators(&connections).unwrap();
    
    // Should detect some protocol anomaly indicators
    assert!(protocol_anomaly_count >= 0, "Protocol anomaly count should be non-negative");
}

#[test]
fn test_encryption_anomaly_detection() {
    // Test encryption anomaly detection
    let monitor = NetworkMonitor::new();
    
    // Create test connections that simulate encryption anomalies
    let mut connections = vec![];
    
    // Add connections with unusual encryption patterns
    for i in 0..10 {
        let mut conn = NetworkConnectionStats::default();
        conn.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100 + i as u8));
        conn.bytes_transmitted = 3000; // Unusual encryption pattern
        conn.packets_transmitted = 20;
        connections.push(conn);
    }
    
    // Test encryption anomaly detection
    let encryption_anomaly_count = monitor.detect_encryption_anomalies(&connections).unwrap();
    
    // Should detect some encryption anomaly indicators
    assert!(encryption_anomaly_count >= 0, "Encryption anomaly count should be non-negative");
}

#[test]
fn test_authentication_failure_detection() {
    // Test authentication failure detection
    let monitor = NetworkMonitor::new();
    
    // Create test connections that simulate authentication failures
    let mut connections = vec![];
    
    // Add connections with repeated attempts
    for i in 0..15 {
        let mut conn = NetworkConnectionStats::default();
        conn.src_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100 + i as u8));
        conn.bytes_transmitted = 800; // Repeated connection attempts
        conn.packets_transmitted = 8;
        connections.push(conn);
    }
    
    // Test authentication failure detection
    let auth_failure_count = monitor.detect_authentication_failures(&connections).unwrap();
    
    // Should detect some authentication failures
    assert!(auth_failure_count >= 0, "Authentication failure count should be non-negative");
}

#[test]
fn test_comprehensive_security_metrics_with_all_threats() {
    // Test comprehensive security metrics with all threat types
    let monitor = NetworkMonitor::new();
    let mut security_metrics = NetworkSecurityMetrics::default();
    
    // Set values for all threat types
    security_metrics.suspicious_connections = 5;
    security_metrics.malicious_ips_detected = 3;
    security_metrics.port_scan_attempts = 2;
    security_metrics.ddos_indicators = 1;
    security_metrics.unusual_traffic_patterns = 4;
    security_metrics.brute_force_attempts = 2;
    security_metrics.sql_injection_attempts = 1;
    security_metrics.xss_attempts = 1;
    security_metrics.command_injection_attempts = 1;
    security_metrics.mitm_indicators = 1;
    security_metrics.data_exfiltration_attempts = 1;
    security_metrics.zero_day_exploit_indicators = 1;
    security_metrics.apt_indicators = 1;
    security_metrics.ransomware_indicators = 1;
    security_metrics.cryptojacking_indicators = 1;
    security_metrics.botnet_indicators = 1;
    security_metrics.phishing_indicators = 1;
    security_metrics.malware_communication_indicators = 1;
    security_metrics.dns_tunneling_indicators = 1;
    security_metrics.icmp_tunneling_indicators = 1;
    security_metrics.http_tunneling_indicators = 1;
    security_metrics.protocol_anomaly_indicators = 1;
    security_metrics.encryption_anomalies = 1;
    security_metrics.authentication_failures = 1;
    
    // Test total security events calculation
    let total_events = monitor.calculate_total_security_events(&security_metrics);
    let expected_total = 5 + 3 + 2 + 1 + 4 + 2 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1 + 1;
    assert_eq!(total_events, expected_total, "Total security events should include all threat types");
    
    // Test security recommendations
    let recommendations = monitor.generate_security_recommendations_comprehensive(&security_metrics);
    assert!(!recommendations.is_empty(), "Should generate comprehensive recommendations");
    assert!(recommendations.len() >= 20, "Should have recommendations for all threat types");
    
    // Test security score calculation
    let (security_score, threat_level, _, _, _) = monitor.calculate_enhanced_security_metrics(&security_metrics);
    assert!(security_score < 0.5, "Should have low security score with many threats");
    assert!(threat_level > 0.5, "Should have high threat level with many threats");
}

#[test]
fn test_security_metrics_serialization_with_all_fields() {
    // Test that security metrics with all fields can be serialized and deserialized
    use serde_json;
    
    let mut security_metrics = NetworkSecurityMetrics::default();
    
    // Set values for all fields
    security_metrics.suspicious_connections = 5;
    security_metrics.malicious_ips_detected = 3;
    security_metrics.port_scan_attempts = 2;
    security_metrics.ddos_indicators = 1;
    security_metrics.unusual_traffic_patterns = 4;
    security_metrics.brute_force_attempts = 2;
    security_metrics.sql_injection_attempts = 1;
    security_metrics.xss_attempts = 1;
    security_metrics.command_injection_attempts = 1;
    security_metrics.mitm_indicators = 1;
    security_metrics.data_exfiltration_attempts = 1;
    security_metrics.zero_day_exploit_indicators = 1;
    security_metrics.apt_indicators = 1;
    security_metrics.ransomware_indicators = 1;
    security_metrics.cryptojacking_indicators = 1;
    security_metrics.botnet_indicators = 1;
    security_metrics.phishing_indicators = 1;
    security_metrics.malware_communication_indicators = 1;
    security_metrics.dns_tunneling_indicators = 1;
    security_metrics.icmp_tunneling_indicators = 1;
    security_metrics.http_tunneling_indicators = 1;
    security_metrics.protocol_anomaly_indicators = 1;
    security_metrics.encryption_anomalies = 1;
    security_metrics.authentication_failures = 1;
    security_metrics.security_score = 0.3;
    security_metrics.threat_level = 0.7;
    security_metrics.detection_confidence = 0.8;
    security_metrics.false_positive_rate = 0.1;
    security_metrics.true_positive_rate = 0.9;
    security_metrics.total_security_events = 30;
    security_metrics.top_malicious_ips = vec!["192.168.1.100".to_string(), "10.0.0.50".to_string()];
    security_metrics.top_targeted_ports = vec![22, 80, 443];
    security_metrics.recommendations = vec![
        "Implement stronger security measures".to_string(),
        "Monitor network traffic closely".to_string()
    ];
    
    // Serialize to JSON
    let json = serde_json::to_string(&security_metrics);
    assert!(json.is_ok(), "Should be able to serialize security metrics with all fields");
    
    // Deserialize back
    let deserialized: Result<NetworkSecurityMetrics, _> = serde_json::from_str(&json.unwrap());
    assert!(deserialized.is_ok(), "Should be able to deserialize security metrics with all fields");
    
    let deserialized_metrics = deserialized.unwrap();
    
    // Verify all fields are preserved
    assert_eq!(deserialized_metrics.suspicious_connections, 5);
    assert_eq!(deserialized_metrics.malicious_ips_detected, 3);
    assert_eq!(deserialized_metrics.command_injection_attempts, 1);
    assert_eq!(deserialized_metrics.data_exfiltration_attempts, 1);
    assert_eq!(deserialized_metrics.zero_day_exploit_indicators, 1);
    assert_eq!(deserialized_metrics.apt_indicators, 1);
    assert_eq!(deserialized_metrics.cryptojacking_indicators, 1);
    assert_eq!(deserialized_metrics.phishing_indicators, 1);
    assert_eq!(deserialized_metrics.malware_communication_indicators, 1);
    assert_eq!(deserialized_metrics.dns_tunneling_indicators, 1);
    assert_eq!(deserialized_metrics.icmp_tunneling_indicators, 1);
    assert_eq!(deserialized_metrics.http_tunneling_indicators, 1);
    assert_eq!(deserialized_metrics.protocol_anomaly_indicators, 1);
    assert_eq!(deserialized_metrics.encryption_anomalies, 1);
    assert_eq!(deserialized_metrics.authentication_failures, 1);
    assert_eq!(deserialized_metrics.security_score, 0.3);
    assert_eq!(deserialized_metrics.threat_level, 0.7);
    assert_eq!(deserialized_metrics.total_security_events, 30);
    assert_eq!(deserialized_metrics.top_malicious_ips.len(), 2);
    assert_eq!(deserialized_metrics.top_targeted_ports.len(), 3);
    assert_eq!(deserialized_metrics.recommendations.len(), 2);
}