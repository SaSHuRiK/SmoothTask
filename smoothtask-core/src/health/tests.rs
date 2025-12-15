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
            enhanced_error_context: None,
            status: HealthIssueStatus::Open,
            resolved_time: None,
        };

        health_monitor
            .add_health_issue(issue.clone())
            .await
            .unwrap();

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
            enhanced_error_context: None,
            status: HealthIssueStatus::Open,
            resolved_time: None,
        };

        health_monitor.add_health_issue(issue).await.unwrap();
        health_monitor
            .resolve_health_issue("test-issue-2")
            .await
            .unwrap();

        let health_status = health_monitor.get_health_status().await.unwrap();
        assert_eq!(health_status.issue_history.len(), 1);
        assert_eq!(
            health_status.issue_history[0].status,
            HealthIssueStatus::Resolved
        );
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
                enhanced_error_context: None,
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
            enhanced_error_context: None,
            status: HealthIssueStatus::Open,
            resolved_time: None,
        };

        // Отправляем уведомление
        notification_service
            .send_health_notification(&issue)
            .await
            .unwrap();

        // Проверяем, что уведомление было отправлено
        // В реальной реализации здесь можно проверить логи
    }

    #[tokio::test]
    async fn test_health_score_calculation() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Получаем текущее состояние здоровья
        let health_status = health_monitor.check_health().await.unwrap();

        // Проверяем, что балл здоровья рассчитан
        assert!(health_status.health_score >= 0.0);
        assert!(health_status.health_score <= 100.0);

        // Проверяем, что история баллов здоровья не пуста
        assert!(!health_status.health_score_history.is_empty());
    }

    #[tokio::test]
    async fn test_auto_recovery_flags() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Получаем текущие флаги автоматического восстановления
        let health_status = health_monitor.get_health_status().await.unwrap();

        // Проверяем, что флаги автоматического восстановления установлены по умолчанию
        assert!(health_status.auto_recovery_flags.auto_recovery_enabled);
        assert!(
            health_status
                .auto_recovery_flags
                .component_auto_recovery_enabled
        );
        assert!(
            health_status
                .auto_recovery_flags
                .resource_auto_recovery_enabled
        );
        assert!(
            !health_status
                .auto_recovery_flags
                .config_auto_recovery_enabled
        );
    }

    #[tokio::test]
    async fn test_recovery_stats() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Получаем статистику восстановления
        let recovery_stats = health_monitor.get_recovery_stats().await.unwrap();

        // Проверяем, что статистика инициализирована
        assert_eq!(recovery_stats.total_recovery_attempts, 0);
        assert_eq!(recovery_stats.successful_recoveries, 0);
        assert_eq!(recovery_stats.failed_recoveries, 0);
        assert!(recovery_stats.recovery_history.is_empty());
    }

    #[tokio::test]
    async fn test_update_auto_recovery_flags() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Создаем новые флаги автоматического восстановления
        let mut new_flags = AutoRecoveryFlags::default();
        new_flags.auto_recovery_enabled = false;
        new_flags.component_auto_recovery_enabled = false;

        // Обновляем флаги
        health_monitor
            .update_auto_recovery_flags(new_flags.clone())
            .await
            .unwrap();

        // Проверяем, что флаги обновлены
        let health_status = health_monitor.get_health_status().await.unwrap();
        assert!(!health_status.auto_recovery_flags.auto_recovery_enabled);
        assert!(
            !health_status
                .auto_recovery_flags
                .component_auto_recovery_enabled
        );
    }

    #[tokio::test]
    async fn test_clear_recovery_stats() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Очищаем статистику восстановления
        health_monitor.clear_recovery_stats().await.unwrap();

        // Проверяем, что статистика очищена
        let recovery_stats = health_monitor.get_recovery_stats().await.unwrap();
        assert_eq!(recovery_stats.total_recovery_attempts, 0);
        assert_eq!(recovery_stats.successful_recoveries, 0);
        assert_eq!(recovery_stats.failed_recoveries, 0);
        assert!(recovery_stats.recovery_history.is_empty());
    }

    #[tokio::test]
    async fn test_component_recovery_simulation() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Создаем проблему с компонентом
        let issue = HealthIssue {
            issue_id: "test-recovery-issue".to_string(),
            timestamp: Utc::now(),
            issue_type: HealthIssueType::ComponentFailure,
            severity: HealthIssueSeverity::Critical,
            component: Some("system_metrics".to_string()),
            description: "System metrics component failed".to_string(),
            error_details: Some("Component not responding".to_string()),
            enhanced_error_context: None,
            status: HealthIssueStatus::Open,
            resolved_time: None,
        };

        // Добавляем проблему
        health_monitor.add_health_issue(issue).await.unwrap();

        // Выполняем проверку здоровья (это должно запустить автоматическое восстановление)
        let _health_status = health_monitor.check_health().await.unwrap();

        // Проверяем, что автоматическое восстановление было выполнено
        let recovery_stats = health_monitor.get_recovery_stats().await.unwrap();
        assert!(recovery_stats.total_recovery_attempts > 0);
    }

    #[tokio::test]
    async fn test_security_integration() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Выполняем проверку здоровья
        let health_status = health_monitor.check_health().await.unwrap();

        // Проверяем, что компонент безопасности присутствует
        assert!(health_status.component_statuses.contains_key("security"));

        // Проверяем, что статус безопасности установлен
        let security_status = health_status.component_statuses.get("security").unwrap();
        assert_ne!(security_status.status, ComponentHealthStatus::Unknown);
    }

    #[tokio::test]
    async fn test_security_component_status() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Выполняем проверку здоровья
        let health_status = health_monitor.check_health().await.unwrap();

        // Получаем статус компонента безопасности
        let security_status = health_status.component_statuses.get("security").unwrap();

        // Проверяем, что статус безопасности установлен корректно
        match security_status.status {
            ComponentHealthStatus::Healthy => {
                assert!(security_status.message.as_ref().unwrap().contains("healthy"));
            }
            ComponentHealthStatus::Warning => {
                assert!(security_status.message.as_ref().unwrap().contains("warnings"));
            }
            ComponentHealthStatus::Unhealthy => {
                assert!(security_status.message.as_ref().unwrap().contains("threats"));
            }
            ComponentHealthStatus::Unknown => {
                assert!(security_status.message.as_ref().unwrap().contains("unknown"));
            }
            ComponentHealthStatus::Disabled => {
                panic!("Security component should not be disabled");
            }
        }

        // Проверяем, что время последней проверки установлено
        assert!(security_status.last_check_time.is_some());
    }

    #[tokio::test]
    async fn test_security_with_health_monitor() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Выполняем проверку здоровья
        let health_status = health_monitor.check_health().await.unwrap();

        // Проверяем, что компонент безопасности присутствует
        assert!(health_status.component_statuses.contains_key("security"));

        // Проверяем, что статус безопасности установлен
        let security_status = health_status.component_statuses.get("security").unwrap();
        assert_ne!(security_status.status, ComponentHealthStatus::Unknown);

        // Проверяем, что время последней проверки установлено
        assert!(security_status.last_check_time.is_some());

        // Проверяем, что сообщение о состоянии установлено
        assert!(security_status.message.is_some());
    }

    #[tokio::test]
    async fn test_enhanced_error_context_creation() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Создаем расширенный контекст ошибки
        let error_context = health_monitor.create_enhanced_error_context(
            "test_component",
            "resource_exhaustion",
            "Test resource exhaustion error",
            Some("Detailed error information".to_string()),
            Some("Stack trace information".to_string()),
            Some("Execution context details".to_string()),
            Some("System context information".to_string()),
        );

        // Проверяем, что контекст создан корректно
        assert_eq!(error_context.component_name, "test_component");
        assert_eq!(error_context.error_type, "resource_exhaustion");
        assert_eq!(error_context.error_message, "Test resource exhaustion error");
        assert_eq!(error_context.error_details, Some("Detailed error information".to_string()));
        assert_eq!(error_context.stack_trace, Some("Stack trace information".to_string()));
        assert_eq!(error_context.execution_context, Some("Execution context details".to_string()));
        assert_eq!(error_context.system_context, Some("System context information".to_string()));
        assert_eq!(error_context.occurrence_count, 1);
    }

    #[tokio::test]
    async fn test_enhanced_recovery_suggestions() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Тестируем различные типы ошибок
        let resource_suggestions = health_monitor.get_recovery_suggestions("resource_exhaustion", "test_component");
        assert!(resource_suggestions.contains("Resource exhaustion detected"));
        assert!(resource_suggestions.contains("Check system resource usage"));

        let connection_suggestions = health_monitor.get_recovery_suggestions("connection_failed", "test_component");
        assert!(connection_suggestions.contains("Connection failure"));
        assert!(connection_suggestions.contains("Check network connectivity"));

        let database_suggestions = health_monitor.get_recovery_suggestions("database_error", "test_component");
        assert!(database_suggestions.contains("Database error"));
        assert!(database_suggestions.contains("Check database connectivity"));

        let security_suggestions = health_monitor.get_recovery_suggestions("security_error", "test_component");
        assert!(security_suggestions.contains("Security error"));
        assert!(security_suggestions.contains("Review security policies"));

        let general_suggestions = health_monitor.get_recovery_suggestions("unknown_error", "test_component");
        assert!(general_suggestions.contains("Error detected"));
        assert!(general_suggestions.contains("Check system logs"));
    }

    #[tokio::test]
    async fn test_enhanced_auto_recovery_with_analysis() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Создаем расширенный контекст ошибки
        let error_context = health_monitor.create_enhanced_error_context(
            "system_metrics",
            "resource_exhaustion",
            "System metrics resource exhaustion",
            Some("Memory and CPU resources exhausted".to_string()),
            None,
            None,
            None,
        );

        // Выполняем улучшенное автоматическое восстановление
        let recovery_attempt = health_monitor
            .enhanced_auto_recovery_with_analysis("system_metrics", error_context)
            .await
            .unwrap();

        // Проверяем результат восстановления
        assert_eq!(recovery_attempt.component, "system_metrics");
        assert_eq!(recovery_attempt.status, RecoveryStatus::Success);
        assert!(recovery_attempt.recovery_details.unwrap().contains("Enhanced recovery"));
        assert!(recovery_attempt.recovery_details.unwrap().contains("success"));

        // Проверяем, что статистика восстановления обновлена
        let recovery_stats = health_monitor.get_recovery_stats().await.unwrap();
        assert_eq!(recovery_stats.total_recovery_attempts, 1);
        assert_eq!(recovery_stats.successful_recoveries, 1);
        assert_eq!(recovery_stats.failed_recoveries, 0);
    }

    #[tokio::test]
    async fn test_recovery_strategy_determination() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Тестируем различные типы ошибок и стратегии восстановления
        let resource_context = health_monitor.create_enhanced_error_context(
            "test_component",
            "resource_exhaustion",
            "Resource exhaustion",
            None,
            None,
            None,
            None,
        );
        let resource_strategy = health_monitor.determine_recovery_strategy(&resource_context);
        assert_eq!(resource_strategy, RecoveryType::Cleanup);

        let config_context = health_monitor.create_enhanced_error_context(
            "test_component",
            "configuration_error",
            "Configuration error",
            None,
            None,
            None,
            None,
        );
        let config_strategy = health_monitor.determine_recovery_strategy(&config_context);
        assert_eq!(config_strategy, RecoveryType::Reconfigure);

        let permission_context = health_monitor.create_enhanced_error_context(
            "test_component",
            "permission_denied",
            "Permission denied",
            None,
            None,
            None,
            None,
        );
        let permission_strategy = health_monitor.determine_recovery_strategy(&permission_context);
        assert_eq!(permission_strategy, RecoveryType::Reset);

        // Тестируем стратегию на основе частоты ошибок
        let mut frequent_context = health_monitor.create_enhanced_error_context(
            "test_component",
            "unknown_error",
            "Frequent error",
            None,
            None,
            None,
            None,
        );
        frequent_context.occurrence_count = 5;
        let frequent_strategy = health_monitor.determine_recovery_strategy(&frequent_context);
        assert_eq!(frequent_strategy, RecoveryType::Restore);

        let mut infrequent_context = health_monitor.create_enhanced_error_context(
            "test_component",
            "unknown_error",
            "Infrequent error",
            None,
            None,
            None,
            None,
        );
        infrequent_context.occurrence_count = 1;
        let infrequent_strategy = health_monitor.determine_recovery_strategy(&infrequent_context);
        assert_eq!(infrequent_strategy, RecoveryType::Restart);
    }

    #[tokio::test]
    async fn test_error_pattern_analysis() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Добавляем несколько ошибок для анализа паттернов
        let error_context1 = health_monitor.create_enhanced_error_context(
            "test_component",
            "resource_exhaustion",
            "Resource exhaustion error 1",
            None,
            None,
            None,
            None,
        );

        let error_context2 = health_monitor.create_enhanced_error_context(
            "test_component",
            "resource_exhaustion",
            "Resource exhaustion error 2",
            None,
            None,
            None,
            None,
        );

        // Обновляем паттерны ошибок
        health_monitor
            .update_error_patterns(
                "resource_exhaustion",
                "test_component",
                HealthIssueSeverity::Error,
                error_context1,
            )
            .await
            .unwrap();

        health_monitor
            .update_error_patterns(
                "resource_exhaustion",
                "test_component",
                HealthIssueSeverity::Error,
                error_context2,
            )
            .await
            .unwrap();

        // Выполняем анализ паттернов ошибок
        let patterns = health_monitor.analyze_error_patterns().await.unwrap();
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].error_type, "resource_exhaustion");
        assert_eq!(patterns[0].occurrence_count, 2);
        assert!(patterns[0].is_recurring);

        // Выполняем улучшенный анализ паттернов ошибок
        let enhanced_patterns = health_monitor.enhanced_error_pattern_analysis().await.unwrap();
        assert_eq!(enhanced_patterns.len(), 1);
        assert_eq!(enhanced_patterns[0].error_type, "resource_exhaustion");
        assert_eq!(enhanced_patterns[0].occurrence_count, 2);
        assert!(enhanced_patterns[0].is_recurring);
        assert!(!enhanced_patterns[0].error_classification.is_empty());
        assert!(!enhanced_patterns[0].trend_analysis.is_empty());
        assert!(!enhanced_patterns[0].recovery_priority.is_empty());
    }

    #[tokio::test]
    async fn test_error_classification() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Создаем паттерн для критической ошибки
        let mut critical_pattern = ErrorPattern {
            error_type: "critical_error".to_string(),
            occurrence_count: 3,
            first_occurrence: Utc::now() - chrono::Duration::minutes(10),
            last_occurrence: Utc::now(),
            affected_components: vec!["test_component".to_string()],
            severity: HealthIssueSeverity::Critical,
            frequency_per_minute: 0.5,
            last_error_context: None,
        };

        let classification = health_monitor.classify_error_by_pattern(&critical_pattern);
        assert_eq!(classification, "Critical Recurring Error");

        // Создаем паттерн для частой ошибки
        let mut frequent_pattern = ErrorPattern {
            error_type: "frequent_error".to_string(),
            occurrence_count: 5,
            first_occurrence: Utc::now() - chrono::Duration::minutes(15),
            last_occurrence: Utc::now(),
            affected_components: vec!["test_component".to_string()],
            severity: HealthIssueSeverity::Error,
            frequency_per_minute: 0.4,
            last_error_context: None,
        };

        let classification = health_monitor.classify_error_by_pattern(&frequent_pattern);
        assert_eq!(classification, "Frequent Error Pattern");

        // Создаем паттерн для изолированной ошибки
        let mut isolated_pattern = ErrorPattern {
            error_type: "isolated_error".to_string(),
            occurrence_count: 1,
            first_occurrence: Utc::now(),
            last_occurrence: Utc::now(),
            affected_components: vec!["test_component".to_string()],
            severity: HealthIssueSeverity::Error,
            frequency_per_minute: 0.0,
            last_error_context: None,
        };

        let classification = health_monitor.classify_error_by_pattern(&isolated_pattern);
        assert_eq!(classification, "Isolated Error");
    }

    #[tokio::test]
    async fn test_error_trend_analysis() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Тестируем различные тренды ошибок
        let mut recent_pattern = ErrorPattern {
            error_type: "recent_error".to_string(),
            occurrence_count: 3,
            first_occurrence: Utc::now() - chrono::Duration::minutes(30),
            last_occurrence: Utc::now(),
            affected_components: vec!["test_component".to_string()],
            severity: HealthIssueSeverity::Error,
            frequency_per_minute: 0.1,
            last_error_context: None,
        };

        let trend = health_monitor.analyze_error_trend(&recent_pattern);
        assert_eq!(trend, "Recent Pattern");

        let mut rapid_pattern = ErrorPattern {
            error_type: "rapid_error".to_string(),
            occurrence_count: 5,
            first_occurrence: Utc::now() - chrono::Duration::minutes(10),
            last_occurrence: Utc::now(),
            affected_components: vec!["test_component".to_string()],
            severity: HealthIssueSeverity::Error,
            frequency_per_minute: 0.5,
            last_error_context: None,
        };

        let trend = health_monitor.analyze_error_trend(&rapid_pattern);
        assert_eq!(trend, "Recent Rapid Occurrence");

        let mut long_term_pattern = ErrorPattern {
            error_type: "long_term_error".to_string(),
            occurrence_count: 20,
            first_occurrence: Utc::now() - chrono::Duration::days(10),
            last_occurrence: Utc::now(),
            affected_components: vec!["test_component".to_string()],
            severity: HealthIssueSeverity::Warning,
            frequency_per_minute: 0.01,
            last_error_context: None,
        };

        let trend = health_monitor.analyze_error_trend(&long_term_pattern);
        assert_eq!(trend, "Long-term Pattern");
    }

    #[tokio::test]
    async fn test_recovery_priority_determination() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Тестируем определение приоритета восстановления
        let mut critical_pattern = ErrorPattern {
            error_type: "critical_error".to_string(),
            occurrence_count: 2,
            first_occurrence: Utc::now() - chrono::Duration::minutes(5),
            last_occurrence: Utc::now(),
            affected_components: vec!["test_component".to_string()],
            severity: HealthIssueSeverity::Critical,
            frequency_per_minute: 0.4,
            last_error_context: None,
        };

        let priority = health_monitor.determine_recovery_priority(&critical_pattern);
        assert_eq!(priority, "High Priority");

        let mut frequent_error_pattern = ErrorPattern {
            error_type: "frequent_error".to_string(),
            occurrence_count: 8,
            first_occurrence: Utc::now() - chrono::Duration::minutes(20),
            last_occurrence: Utc::now(),
            affected_components: vec!["test_component".to_string()],
            severity: HealthIssueSeverity::Error,
            frequency_per_minute: 0.5,
            last_error_context: None,
        };

        let priority = health_monitor.determine_recovery_priority(&frequent_error_pattern);
        assert_eq!(priority, "High Priority");

        let mut medium_error_pattern = ErrorPattern {
            error_type: "medium_error".to_string(),
            occurrence_count: 3,
            first_occurrence: Utc::now() - chrono::Duration::minutes(30),
            last_occurrence: Utc::now(),
            affected_components: vec!["test_component".to_string()],
            severity: HealthIssueSeverity::Error,
            frequency_per_minute: 0.1,
            last_error_context: None,
        };

        let priority = health_monitor.determine_recovery_priority(&medium_error_pattern);
        assert_eq!(priority, "Medium Priority");

        let mut warning_pattern = ErrorPattern {
            error_type: "warning_pattern".to_string(),
            occurrence_count: 15,
            first_occurrence: Utc::now() - chrono::Duration::hours(2),
            last_occurrence: Utc::now(),
            affected_components: vec!["test_component".to_string()],
            severity: HealthIssueSeverity::Warning,
            frequency_per_minute: 0.6,
            last_error_context: None,
        };

        let priority = health_monitor.determine_recovery_priority(&warning_pattern);
        assert_eq!(priority, "Medium Priority");

        let mut info_pattern = ErrorPattern {
            error_type: "info_pattern".to_string(),
            occurrence_count: 5,
            first_occurrence: Utc::now() - chrono::Duration::hours(1),
            last_occurrence: Utc::now(),
            affected_components: vec!["test_component".to_string()],
            severity: HealthIssueSeverity::Info,
            frequency_per_minute: 0.1,
            last_error_context: None,
        };

        let priority = health_monitor.determine_recovery_priority(&info_pattern);
        assert_eq!(priority, "Low Priority");
    }

    #[tokio::test]
    async fn test_enhanced_error_recovery_integration() {
        let config = HealthMonitorConfig::default();
        let health_monitor = create_health_monitor(config);

        // Создаем несколько ошибок разных типов
        let resource_context = health_monitor.create_enhanced_error_context(
            "resource_manager",
            "resource_exhaustion",
            "Resource exhaustion in manager",
            Some("CPU and memory exhausted".to_string()),
            None,
            None,
            None,
        );

        let config_context = health_monitor.create_enhanced_error_context(
            "config_service",
            "configuration_error",
            "Configuration error in service",
            Some("Invalid configuration parameters".to_string()),
            None,
            None,
            None,
        );

        // Выполняем восстановление для разных типов ошибок
        let resource_recovery = health_monitor
            .enhanced_auto_recovery_with_analysis("resource_manager", resource_context)
            .await
            .unwrap();

        let config_recovery = health_monitor
            .enhanced_auto_recovery_with_analysis("config_service", config_context)
            .await
            .unwrap();

        // Проверяем, что восстановление выполнено успешно
        assert_eq!(resource_recovery.status, RecoveryStatus::Success);
        assert_eq!(config_recovery.status, RecoveryStatus::Success);

        // Проверяем, что стратегии восстановления выбраны правильно
        assert_eq!(resource_recovery.recovery_type, RecoveryType::Cleanup);
        assert_eq!(config_recovery.recovery_type, RecoveryType::Reconfigure);

        // Проверяем статистику восстановления
        let recovery_stats = health_monitor.get_recovery_stats().await.unwrap();
        assert_eq!(recovery_stats.total_recovery_attempts, 2);
        assert_eq!(recovery_stats.successful_recoveries, 2);
        assert_eq!(recovery_stats.failed_recoveries, 0);
        assert_eq!(recovery_stats.recovery_history.len(), 2);
    }
}
