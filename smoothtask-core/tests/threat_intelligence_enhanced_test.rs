// Enhanced tests for threat intelligence functionality
use smoothtask_core::health::threat_intelligence::*;

#[tokio::test]
async fn test_threat_intelligence_with_basic_feeds() {
    // Create threat intelligence with basic feeds
    let config = ThreatIntelligenceConfig::default_with_basic_feeds();
    let threat_intel = ThreatIntelligenceImpl::new(config);

    // Verify basic feeds are configured
    assert_eq!(threat_intel.config.feeds.len(), 5);

    // Check feed names
    let feed_names: Vec<String> = threat_intel.config.feeds.iter()
        .map(|feed| feed.name.clone())
        .collect();

    assert!(feed_names.contains(&"Basic Malware Signatures".to_string()));
    assert!(feed_names.contains(&"Phishing Domains".to_string()));
    assert!(feed_names.contains(&"Botnet C2 Servers".to_string()));
    assert!(feed_names.contains(&"Ransomware Indicators".to_string()));
    assert!(feed_names.contains(&"Cryptojacking Signatures".to_string()));

    // Check feed configurations
    for feed in &threat_intel.config.feeds {
        assert!(feed.enabled);
        assert!(feed.trust_level > 0.8);
        assert!(feed.update_interval.as_secs() > 0);
    }

    println!("✅ Threat intelligence with basic feeds test passed");
}

#[tokio::test]
async fn test_custom_threat_signatures() {
    // Create threat intelligence with default config
    let config = ThreatIntelligenceConfig::default();
    let mut threat_intel = ThreatIntelligenceImpl::new(config);

    // Initialize
    threat_intel.initialize().await.unwrap();

    // Create custom threat signatures
    let mut signature1 = ThreatIntel::default();
    signature1.threat_type = ThreatType::Malware;
    signature1.severity = ThreatSeverity::High;
    signature1.description = "Custom malware signature".to_string();
    signature1.source = "custom".to_string();

    let indicator1 = ThreatIndicator {
        indicator_type: ThreatIndicatorType::ProcessName,
        value: "custom_malware.exe".to_string(),
        confidence: 0.95,
        last_seen: None,
        tags: vec!["custom".to_string(), "malware".to_string()],
    };
    signature1.indicators.push(indicator1);

    let mut signature2 = ThreatIntel::default();
    signature2.threat_type = ThreatType::Phishing;
    signature2.severity = ThreatSeverity::Medium;
    signature2.description = "Custom phishing signature".to_string();
    signature2.source = "custom".to_string();

    let indicator2 = ThreatIndicator {
        indicator_type: ThreatIndicatorType::Domain,
        value: "phishing-example.com".to_string(),
        confidence: 0.85,
        last_seen: None,
        tags: vec!["custom".to_string(), "phishing".to_string()],
    };
    signature2.indicators.push(indicator2);

    // Add custom signatures
    threat_intel
        .add_custom_threat_signatures(vec![signature1, signature2])
        .await
        .unwrap();

    // Verify signatures were added
    let stats = threat_intel.get_threat_stats().await.unwrap();
    assert_eq!(stats.total_threats, 2);

    // Test finding threats by indicator
    let found_threats = threat_intel
        .find_threats_by_indicator(ThreatIndicatorType::ProcessName, "custom_malware.exe")
        .await
        .unwrap();

    assert_eq!(found_threats.len(), 1);
    assert_eq!(found_threats[0].threat_type, ThreatType::Malware);

    let found_threats = threat_intel
        .find_threats_by_indicator(ThreatIndicatorType::Domain, "phishing-example.com")
        .await
        .unwrap();

    assert_eq!(found_threats.len(), 1);
    assert_eq!(found_threats[0].threat_type, ThreatType::Phishing);

    println!("✅ Custom threat signatures test passed");
}

#[tokio::test]
async fn test_custom_threat_indicators() {
    // Create threat intelligence with default config
    let config = ThreatIntelligenceConfig::default();
    let mut threat_intel = ThreatIntelligenceImpl::new(config);

    // Initialize
    threat_intel.initialize().await.unwrap();

    // Create custom threat indicators
    let indicators = vec![
        ThreatIndicator {
            indicator_type: ThreatIndicatorType::Ip,
            value: "192.168.1.100".to_string(),
            confidence: 0.90,
            last_seen: None,
            tags: vec!["malicious".to_string(), "botnet".to_string()],
        },
        ThreatIndicator {
            indicator_type: ThreatIndicatorType::Hash,
            value: "abc123def456".to_string(),
            confidence: 0.85,
            last_seen: None,
            tags: vec!["malware".to_string()],
        },
        ThreatIndicator {
            indicator_type: ThreatIndicatorType::Url,
            value: "http://malicious-site.com".to_string(),
            confidence: 0.95,
            last_seen: None,
            tags: vec!["phishing".to_string()],
        },
    ];

    // Add custom indicators
    threat_intel
        .add_custom_threat_indicators(indicators)
        .await
        .unwrap();

    // Verify indicators were added
    let stats = threat_intel.get_threat_stats().await.unwrap();
    assert_eq!(stats.total_threats, 3);

    // Test finding threats by indicator
    let is_known = threat_intel
        .is_known_threat(ThreatIndicatorType::Ip, "192.168.1.100")
        .await
        .unwrap();
    assert!(is_known);

    let is_known = threat_intel
        .is_known_threat(ThreatIndicatorType::Hash, "abc123def456")
        .await
        .unwrap();
    assert!(is_known);

    let is_known = threat_intel
        .is_known_threat(ThreatIndicatorType::Url, "http://malicious-site.com")
        .await
        .unwrap();
    assert!(is_known);

    // Test unknown indicator
    let is_unknown = threat_intel
        .is_known_threat(ThreatIndicatorType::Ip, "192.168.1.200")
        .await
        .unwrap();
    assert!(!is_unknown);

    println!("✅ Custom threat indicators test passed");
}

#[tokio::test]
async fn test_threat_feed_configurations() {
    // Test different threat feed configurations
    
    // JSON feed
    let json_feed = ThreatFeedConfig {
        feed_id: uuid::Uuid::new_v4().to_string(),
        name: "JSON Threat Feed".to_string(),
        url: "https://example.com/threats.json".to_string(),
        format: ThreatFeedFormat::Json,
        update_interval: Duration::from_secs(3600),
        enabled: true,
        trust_level: 0.90,
        api_key: Some("test-api-key".to_string()),
        headers: Some(vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
        ]),
    };

    assert_eq!(json_feed.format, ThreatFeedFormat::Json);
    assert_eq!(json_feed.trust_level, 0.90);
    assert!(json_feed.api_key.is_some());
    assert!(json_feed.headers.is_some());

    // CSV feed
    let csv_feed = ThreatFeedConfig {
        feed_id: uuid::Uuid::new_v4().to_string(),
        name: "CSV Threat Feed".to_string(),
        url: "https://example.com/threats.csv".to_string(),
        format: ThreatFeedFormat::Csv,
        update_interval: Duration::from_secs(7200),
        enabled: true,
        trust_level: 0.85,
        api_key: None,
        headers: None,
    };

    assert_eq!(csv_feed.format, ThreatFeedFormat::Csv);
    assert_eq!(csv_feed.trust_level, 0.85);

    // TXT feed
    let txt_feed = ThreatFeedConfig {
        feed_id: uuid::Uuid::new_v4().to_string(),
        name: "TXT Threat Feed".to_string(),
        url: "https://example.com/threats.txt".to_string(),
        format: ThreatFeedFormat::Txt,
        update_interval: Duration::from_secs(14400),
        enabled: true,
        trust_level: 0.80,
        api_key: None,
        headers: None,
    };

    assert_eq!(txt_feed.format, ThreatFeedFormat::Txt);
    assert_eq!(txt_feed.trust_level, 0.80);

    println!("✅ Threat feed configurations test passed");
}

#[tokio::test]
async fn test_threat_types_and_indicators() {
    // Test all threat types
    let threat_types = vec![
        ThreatType::Malware,
        ThreatType::Phishing,
        ThreatType::Botnet,
        ThreatType::Exploit,
        ThreatType::Rootkit,
        ThreatType::Spyware,
        ThreatType::Adware,
        ThreatType::Mining,
        ThreatType::Unknown,
    ];

    for threat_type in threat_types {
        assert!(!threat_type.to_string().is_empty());
    }

    // Test all indicator types
    let indicator_types = vec![
        ThreatIndicatorType::Ip,
        ThreatIndicatorType::Domain,
        ThreatIndicatorType::Url,
        ThreatIndicatorType::Hash,
        ThreatIndicatorType::ProcessName,
        ThreatIndicatorType::FilePath,
        ThreatIndicatorType::Username,
        ThreatIndicatorType::UserId,
        ThreatIndicatorType::RegistryKey,
        ThreatIndicatorType::Unknown,
    ];

    for indicator_type in indicator_types {
        assert!(!indicator_type.to_string().is_empty());
    }

    println!("✅ Threat types and indicators test passed");
}

#[tokio::test]
async fn test_threat_stats_with_custom_data() {
    // Create threat intelligence with default config
    let config = ThreatIntelligenceConfig::default();
    let mut threat_intel = ThreatIntelligenceImpl::new(config);

    // Initialize
    threat_intel.initialize().await.unwrap();

    // Add various types of threats
    let mut malware_threat = ThreatIntel::default();
    malware_threat.threat_type = ThreatType::Malware;
    malware_threat.severity = ThreatSeverity::High;
    malware_threat.source = "test".to_string();

    let mut phishing_threat = ThreatIntel::default();
    phishing_threat.threat_type = ThreatType::Phishing;
    phishing_threat.severity = ThreatSeverity::Medium;
    phishing_threat.source = "test".to_string();

    let mut botnet_threat = ThreatIntel::default();
    botnet_threat.threat_type = ThreatType::Botnet;
    botnet_threat.severity = ThreatSeverity::Critical;
    botnet_threat.source = "test".to_string();

    threat_intel
        .add_custom_threat_signatures(vec![malware_threat, phishing_threat, botnet_threat])
        .await
        .unwrap();

    // Get stats
    let stats = threat_intel.get_threat_stats().await.unwrap();

    // Verify stats
    assert_eq!(stats.total_threats, 3);
    assert_eq!(stats.threats_by_type.len(), 3);
    assert_eq!(stats.threats_by_type[&ThreatType::Malware], 1);
    assert_eq!(stats.threats_by_type[&ThreatType::Phishing], 1);
    assert_eq!(stats.threats_by_type[&ThreatType::Botnet], 1);

    assert_eq!(stats.threats_by_severity.len(), 3);
    assert_eq!(stats.threats_by_severity[&ThreatSeverity::High], 1);
    assert_eq!(stats.threats_by_severity[&ThreatSeverity::Medium], 1);
    assert_eq!(stats.threats_by_severity[&ThreatSeverity::Critical], 1);

    println!("✅ Threat stats with custom data test passed");
}