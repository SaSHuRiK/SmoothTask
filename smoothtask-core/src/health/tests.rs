//! Тесты для модуля мониторинга здоровья.

#[cfg(test)]
mod tests {
    use super::super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_health_monitor_creation() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);
        
        let health_status = health_monitor.get_health_status().await.unwrap();
        assert_eq!(health_status.overall_status, HealthStatus::Unknown);
        assert!(health_status.issue_history.is_empty());
    }
    
    #[tokio::test]
    async fn test_health_check() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);
        
        let health_status = health_monitor.check_health().await.unwrap();
        assert_ne!(health_status.overall_status, HealthStatus::Unknown);
        assert!(!health_status.component_statuses.is_empty());
    }
    
    #[tokio::test]
    async fn test_add_health_issue() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);
        
        let issue = HealthIssue {
            issue_id: "test-issue-1".to_string(),
            timestamp: Utc::now(),
            issue_type: HealthIssueType::ComponentFailure,
            severity: HealthIssueSeverity::Critical,
            component: Some("test_component".to_string()),
            description: "Test critical issue".to_string(),
            error_details: Some("Test error details".to_string()),
            status: HealthIssueStatus::Open,
            resolved_time: None,
        };
        
        health_monitor.add_health_issue(issue.clone()).await.unwrap();
        
        let health_status = health_monitor.get_health_status().await.unwrap();
        assert_eq!(health_status.issue_history.len(), 1);
        assert_eq!(health_status.issue_history[0].issue_id, "test-issue-1");
    }
    
    #[tokio::test]
    async fn test_resolve_health_issue() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);
        
        let issue = HealthIssue {
            issue_id: "test-issue-2".to_string(),
            timestamp: Utc::now(),
            issue_type: HealthIssueType::PerformanceIssue,
            severity: HealthIssueSeverity::Warning,
            component: Some("test_component".to_string()),
            description: "Test warning issue".to_string(),
            error_details: Some("Test warning details".to_string()),
            status: HealthIssueStatus::Open,
            resolved_time: None,
        };
        
        health_monitor.add_health_issue(issue).await.unwrap();
        health_monitor.resolve_health_issue("test-issue-2").await.unwrap();
        
        let health_status = health_monitor.get_health_status().await.unwrap();
        assert_eq!(health_status.issue_history.len(), 1);
        assert_eq!(health_status.issue_history[0].status, HealthIssueStatus::Resolved);
        assert!(health_status.issue_history[0].resolved_time.is_some());
    }
    
    #[tokio::test]
    async fn test_issue_history_limit() {
        let mut config = HealthMonitorConfig::default();
        config.max_issue_history = 2;
        
        let health_monitor = create_health_monitor(config);
        
        // Добавляем 3 проблемы
        for i in 0..3 {
            let issue = HealthIssue {
                issue_id: format!("test-issue-{}", i),
                timestamp: Utc::now(),
                issue_type: HealthIssueType::ComponentFailure,
                severity: HealthIssueSeverity::Error,
                component: Some("test_component".to_string()),
                description: format!("Test issue {}", i),
                error_details: Some(format!("Test error details {}", i)),
                status: HealthIssueStatus::Open,
                resolved_time: None,
            };
            
            health_monitor.add_health_issue(issue).await.unwrap();
        }
        
        let health_status = health_monitor.get_health_status().await.unwrap();
        // Должно быть только 2 проблемы (максимальное количество)
        assert_eq!(health_status.issue_history.len(), 2);
        // Самая старая проблема должна быть удалена
        assert_eq!(health_status.issue_history[0].issue_id, "test-issue-1");
        assert_eq!(health_status.issue_history[1].issue_id, "test-issue-2");
    }
    
    #[tokio::test]
    async fn test_diagnostic_analyzer() {
        let health_monitor = create_default_health_monitor();
        let diagnostic_analyzer = create_diagnostic_analyzer(health_monitor);
        
        let report = diagnostic_analyzer.run_full_diagnostics().await.unwrap();
        assert!(!report.component_diagnostics.is_empty());
        assert!(!report.recommendations.is_empty());
    }
    
    #[tokio::test]
    async fn test_health_monitoring_service() {
        let health_monitor = create_default_health_monitor();
        let monitoring_service = create_health_monitoring_service(health_monitor);
        
        // Запускаем мониторинг
        monitoring_service.start_monitoring().await.unwrap();
        
        // Ждем немного, чтобы мониторинг успел выполнить проверку
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        // Получаем состояние мониторинга
        let state = monitoring_service.get_monitoring_state().await.unwrap();
        assert_ne!(state.current_health.overall_status, HealthStatus::Unknown);
        
        // Останавливаем мониторинг
        monitoring_service.stop_monitoring().await.unwrap();
    }
    
    #[tokio::test]
    async fn test_health_notification_service() {
        let notification_service = create_default_health_notification_service();
        
        let issue = HealthIssue {
            issue_id: "test-notification-1".to_string(),
            timestamp: Utc::now(),
            issue_type: HealthIssueType::CriticalIssue,
            severity: HealthIssueSeverity::Critical,
            component: Some("test_component".to_string()),
            description: "Test critical notification".to_string(),
            error_details: Some("Test critical details".to_string()),
            status: HealthIssueStatus::Open,
            resolved_time: None,
        };
        
        // Отправляем уведомление
        notification_service.send_health_notification(&issue).await.unwrap();
        
        // Проверяем, что уведомление было отправлено
        // В реальной реализации здесь можно проверить логи
    }
}