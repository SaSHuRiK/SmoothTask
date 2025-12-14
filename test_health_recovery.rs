// Simple test to verify health recovery functionality
use smoothtask_core::health::*;
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing health recovery functionality...");
    
    // Create health monitor with default config
    let health_monitor = create_default_health_monitor();
    
    // Test 1: Check initial health status
    println!("\n1. Testing initial health status...");
    let health_status = health_monitor.get_health_status().await?;
    println!("Initial health status: {:?}", health_status.overall_status);
    println!("Initial health score: {}", health_status.health_score);
    println!("Auto-recovery enabled: {}", health_status.auto_recovery_flags.auto_recovery_enabled);
    
    // Test 2: Check health monitoring
    println!("\n2. Testing health check...");
    let health_status = health_monitor.check_health().await?;
    println!("Health check status: {:?}", health_status.overall_status);
    println!("Health score: {}", health_status.health_score);
    println!("Number of components: {}", health_status.component_statuses.len());
    
    // Test 3: Test recovery statistics
    println!("\n3. Testing recovery statistics...");
    let recovery_stats = health_monitor.get_recovery_stats().await?;
    println!("Total recovery attempts: {}", recovery_stats.total_recovery_attempts);
    println!("Successful recoveries: {}", recovery_stats.successful_recoveries);
    println!("Failed recoveries: {}", recovery_stats.failed_recoveries);
    
    // Test 4: Test adding and resolving health issues
    println!("\n4. Testing health issue management...");
    
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
    
    health_monitor.add_health_issue(issue).await?;
    println!("Added health issue");
    
    let health_status = health_monitor.get_health_status().await?;
    println!("Issue history count: {}", health_status.issue_history.len());
    
    health_monitor.resolve_health_issue("test-issue-1").await?;
    println!("Resolved health issue");
    
    let health_status = health_monitor.get_health_status().await?;
    let resolved_issue = health_status.issue_history.iter().find(|i| i.issue_id == "test-issue-1");
    if let Some(issue) = resolved_issue {
        println!("Issue status: {:?}", issue.status);
        println!("Issue resolved time: {:?}", issue.resolved_time);
    }
    
    // Test 5: Test auto-recovery flags update
    println!("\n5. Testing auto-recovery flags update...");
    
    let mut new_flags = AutoRecoveryFlags::default();
    new_flags.auto_recovery_enabled = false;
    new_flags.component_auto_recovery_enabled = false;
    
    health_monitor.update_auto_recovery_flags(new_flags).await?;
    
    let health_status = health_monitor.get_health_status().await?;
    println!("Updated auto-recovery enabled: {}", health_status.auto_recovery_flags.auto_recovery_enabled);
    println!("Updated component auto-recovery enabled: {}", health_status.auto_recovery_flags.component_auto_recovery_enabled);
    
    // Test 6: Test clearing recovery stats
    println!("\n6. Testing recovery stats clearing...");
    
    // First, let's add some fake recovery attempts
    let mut stats = health_monitor.get_recovery_stats().await?;
    stats.total_recovery_attempts = 5;
    stats.successful_recoveries = 3;
    stats.failed_recoveries = 2;
    
    health_monitor.clear_recovery_stats().await?;
    
    let cleared_stats = health_monitor.get_recovery_stats().await?;
    println!("Cleared total recovery attempts: {}", cleared_stats.total_recovery_attempts);
    println!("Cleared successful recoveries: {}", cleared_stats.successful_recoveries);
    println!("Cleared failed recoveries: {}", cleared_stats.failed_recoveries);
    
    println!("\n✅ All health recovery tests completed successfully!");
    
    Ok(())
}