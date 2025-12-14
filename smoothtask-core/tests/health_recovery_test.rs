// Integration test for health recovery functionality
use smoothtask_core::health::*;
use chrono::Utc;

#[tokio::test]
async fn test_health_recovery_basic() {
    // Create health monitor with default config
    let health_monitor = create_default_health_monitor();
    
    // Test basic functionality
    let health_status = health_monitor.get_health_status().await.unwrap();
    assert_eq!(health_status.overall_status, HealthStatus::Unknown);
    assert!(health_status.health_score >= 0.0);
    assert!(health_status.health_score <= 100.0);
    
    // Test health check
    let health_status = health_monitor.check_health().await.unwrap();
    assert_ne!(health_status.overall_status, HealthStatus::Unknown);
    assert!(!health_status.component_statuses.is_empty());
    
    // Test recovery stats
    let recovery_stats = health_monitor.get_recovery_stats().await.unwrap();
    assert_eq!(recovery_stats.total_recovery_attempts, 0);
    assert_eq!(recovery_stats.successful_recoveries, 0);
    assert_eq!(recovery_stats.failed_recoveries, 0);
    
    // Test auto-recovery flags
    assert!(health_status.auto_recovery_flags.auto_recovery_enabled);
    assert!(health_status.auto_recovery_flags.component_auto_recovery_enabled);
    
    println!("✅ Basic health recovery test passed");
}

#[tokio::test]
async fn test_health_issue_management() {
    let health_monitor = create_default_health_monitor();
    
    // Add a health issue
    let issue = HealthIssue {
        issue_id: "test-issue-1".to_string(),
        timestamp: Utc::now(),
        issue_type: HealthIssueType::ComponentFailure,
        severity: HealthIssueSeverity::Warning,
        component: Some("test_component".to_string()),
        description: "Test warning issue".to_string(),
        error_details: Some("Test error details".to_string()),
        status: HealthIssueStatus::Open,
        resolved_time: None,
    };
    
    health_monitor.add_health_issue(issue).await.unwrap();
    
    // Verify issue was added
    let health_status = health_monitor.get_health_status().await.unwrap();
    assert_eq!(health_status.issue_history.len(), 1);
    assert_eq!(health_status.issue_history[0].issue_id, "test-issue-1");
    
    // Resolve the issue
    health_monitor.resolve_health_issue("test-issue-1").await.unwrap();
    
    // Verify issue was resolved
    let health_status = health_monitor.get_health_status().await.unwrap();
    assert_eq!(health_status.issue_history.len(), 1);
    assert_eq!(health_status.issue_history[0].status, HealthIssueStatus::Resolved);
    assert!(health_status.issue_history[0].resolved_time.is_some());
    
    println!("✅ Health issue management test passed");
}

#[tokio::test]
async fn test_auto_recovery_flags_update() {
    let health_monitor = create_default_health_monitor();
    
    // Update auto-recovery flags
    let mut new_flags = AutoRecoveryFlags::default();
    new_flags.auto_recovery_enabled = false;
    new_flags.component_auto_recovery_enabled = false;
    
    health_monitor.update_auto_recovery_flags(new_flags).await.unwrap();
    
    // Verify flags were updated
    let health_status = health_monitor.get_health_status().await.unwrap();
    assert!(!health_status.auto_recovery_flags.auto_recovery_enabled);
    assert!(!health_status.auto_recovery_flags.component_auto_recovery_enabled);
    
    println!("✅ Auto-recovery flags update test passed");
}

#[tokio::test]
async fn test_recovery_stats_management() {
    let health_monitor = create_default_health_monitor();
    
    // Test initial stats
    let recovery_stats = health_monitor.get_recovery_stats().await.unwrap();
    assert_eq!(recovery_stats.total_recovery_attempts, 0);
    assert_eq!(recovery_stats.successful_recoveries, 0);
    assert_eq!(recovery_stats.failed_recoveries, 0);
    
    // Clear stats (should work even if empty)
    health_monitor.clear_recovery_stats().await.unwrap();
    
    // Verify stats are still empty
    let cleared_stats = health_monitor.get_recovery_stats().await.unwrap();
    assert_eq!(cleared_stats.total_recovery_attempts, 0);
    assert_eq!(cleared_stats.successful_recoveries, 0);
    assert_eq!(cleared_stats.failed_recoveries, 0);
    
    println!("✅ Recovery stats management test passed");
}

#[tokio::test]
async fn test_health_scoring() {
    let health_monitor = create_default_health_monitor();
    
    // Perform health check to calculate score
    let health_status = health_monitor.check_health().await.unwrap();
    
    // Verify health score is calculated
    assert!(health_status.health_score >= 0.0);
    assert!(health_status.health_score <= 100.0);
    
    // Verify health score history is populated
    assert!(!health_status.health_score_history.is_empty());
    
    // Verify the latest health score entry matches current score
    let latest_score = health_status.health_score_history.last().unwrap();
    assert_eq!(latest_score.score, health_status.health_score);
    
    println!("✅ Health scoring test passed");
}