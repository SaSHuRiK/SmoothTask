//! Модуль диагностики проблем здоровья.
//!
//! Этот модуль предоставляет функции для диагностики и анализа проблем
//! здоровья демона SmoothTask.

use super::*;
use anyhow::Result;
use std::collections::HashMap;

/// Структура для хранения результатов диагностики.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DiagnosticReport {
    /// Время создания отчета
    pub timestamp: DateTime<Utc>,
    /// Общий статус диагностики
    pub overall_status: HealthStatus,
    /// Детальные результаты диагностики компонентов
    pub component_diagnostics: HashMap<String, ComponentDiagnostic>,
    /// Рекомендации по устранению проблем
    pub recommendations: Vec<String>,
    /// Детальная информация о системе
    pub system_info: SystemDiagnosticInfo,
}

/// Диагностическая информация о компоненте.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentDiagnostic {
    /// Состояние компонента
    pub status: ComponentHealthStatus,
    /// Детальное описание проблемы
    pub description: String,
    /// Возможные причины
    pub possible_causes: Vec<String>,
    /// Рекомендации по устранению
    pub recommendations: Vec<String>,
    /// Детальная техническая информация
    pub technical_details: Option<String>,
}

impl Default for ComponentDiagnostic {
    fn default() -> Self {
        Self {
            status: ComponentHealthStatus::Unknown,
            description: String::new(),
            possible_causes: Vec::new(),
            recommendations: Vec::new(),
            technical_details: None,
        }
    }
}

/// Информация о системе для диагностики.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SystemDiagnosticInfo {
    /// Версия демона
    pub daemon_version: Option<String>,
    /// Версия ОС
    pub os_version: Option<String>,
    /// Версия ядра
    pub kernel_version: Option<String>,
    /// Информация о CPU
    pub cpu_info: Option<String>,
    /// Информация о памяти
    pub memory_info: Option<String>,
    /// Информация о диске
    pub disk_info: Option<String>,
    /// Информация о GPU
    pub gpu_info: Option<String>,
    /// Конфигурация демона
    pub daemon_config: Option<String>,
}

/// Интерфейс для диагностики проблем здоровья.
#[async_trait::async_trait]
pub trait DiagnosticAnalyzer: Send + Sync {
    /// Выполнить полную диагностику системы.
    async fn run_full_diagnostics(&self) -> Result<DiagnosticReport>;

    /// Выполнить диагностику конкретного компонента.
    async fn diagnose_component(&self, component_name: &str) -> Result<ComponentDiagnostic>;

    /// Проанализировать историю проблем.
    async fn analyze_issue_history(&self, issues: &[HealthIssue]) -> Result<DiagnosticReport>;

    /// Собрать информацию о системе.
    async fn collect_system_info(&self) -> Result<SystemDiagnosticInfo>;
}

/// Реализация DiagnosticAnalyzer.
#[derive(Debug, Clone)]
pub struct DiagnosticAnalyzerImpl {
    health_monitor: HealthMonitorImpl,
}

#[async_trait::async_trait]
impl DiagnosticAnalyzer for DiagnosticAnalyzerImpl {
    async fn run_full_diagnostics(&self) -> Result<DiagnosticReport> {
        let health_status = self.health_monitor.get_health_status().await?;
        let system_info = self.collect_system_info().await?;

        let mut component_diagnostics = HashMap::new();
        let mut recommendations = Vec::new();

        // Анализируем каждый компонент
        for (component_name, _component_status) in &health_status.component_statuses {
            let diagnostic = self.diagnose_component(component_name).await?;

            // Собираем рекомендации перед вставкой
            if !diagnostic.recommendations.is_empty() {
                recommendations.extend(diagnostic.recommendations.clone());
            }

            component_diagnostics.insert(component_name.clone(), diagnostic);
        }

        Ok(DiagnosticReport {
            timestamp: Utc::now(),
            overall_status: health_status.overall_status,
            component_diagnostics,
            recommendations,
            system_info,
        })
    }

    async fn diagnose_component(&self, component_name: &str) -> Result<ComponentDiagnostic> {
        let health_status = self.health_monitor.get_health_status().await?;

        if let Some(component_status) = health_status.component_statuses.get(component_name) {
            match component_status.status {
                ComponentHealthStatus::Healthy => Ok(ComponentDiagnostic {
                    status: ComponentHealthStatus::Healthy,
                    description: format!("Component {} is healthy", component_name),
                    possible_causes: Vec::new(),
                    recommendations: Vec::new(),
                    technical_details: None,
                }),
                ComponentHealthStatus::Warning => {
                    let mut diagnostic = ComponentDiagnostic::default();
                    diagnostic.status = ComponentHealthStatus::Warning;
                    diagnostic.description = format!("Component {} has warnings", component_name);

                    if let Some(error_details) = &component_status.error_details {
                        diagnostic.possible_causes.push(error_details.clone());
                        diagnostic
                            .recommendations
                            .push(format!("Investigate the warning: {}", error_details));
                    }

                    Ok(diagnostic)
                }
                ComponentHealthStatus::Unhealthy => {
                    let mut diagnostic = ComponentDiagnostic::default();
                    diagnostic.status = ComponentHealthStatus::Unhealthy;
                    diagnostic.description = format!("Component {} is unhealthy", component_name);

                    if let Some(error_details) = &component_status.error_details {
                        diagnostic.possible_causes.push(error_details.clone());
                        diagnostic
                            .recommendations
                            .push(format!("Fix the issue: {}", error_details));
                        diagnostic
                            .recommendations
                            .push(format!("Restart the {} component", component_name));
                    }

                    Ok(diagnostic)
                }
                _ => Ok(ComponentDiagnostic::default()),
            }
        } else {
            Err(anyhow::anyhow!("Component {} not found", component_name))
        }
    }

    async fn analyze_issue_history(&self, issues: &[HealthIssue]) -> Result<DiagnosticReport> {
        let mut report = DiagnosticReport::default();
        report.timestamp = Utc::now();

        // Анализируем историю проблем
        let mut critical_count = 0;
        let mut error_count = 0;
        let mut warning_count = 0;

        let mut component_issues = HashMap::new();

        for issue in issues {
            match issue.severity {
                HealthIssueSeverity::Critical => critical_count += 1,
                HealthIssueSeverity::Error => error_count += 1,
                HealthIssueSeverity::Warning => warning_count += 1,
                _ => {}
            }

            if let Some(component) = &issue.component {
                let entry = component_issues.entry(component.clone()).or_insert(0);
                *entry += 1;
            }
        }

        // Определяем общий статус
        if critical_count > 0 {
            report.overall_status = HealthStatus::Critical;
            report.recommendations.push(format!(
                "Found {} critical issues that need immediate attention",
                critical_count
            ));
        } else if error_count > 0 {
            report.overall_status = HealthStatus::Degraded;
            report.recommendations.push(format!(
                "Found {} errors that need to be fixed",
                error_count
            ));
        } else if warning_count > 0 {
            report.overall_status = HealthStatus::Warning;
            report.recommendations.push(format!(
                "Found {} warnings that should be investigated",
                warning_count
            ));
        } else {
            report.overall_status = HealthStatus::Healthy;
        }

        // Добавляем информацию о компонентах с проблемами
        for (component, count) in component_issues {
            let mut diagnostic = ComponentDiagnostic::default();
            diagnostic.status = ComponentHealthStatus::Unhealthy;
            diagnostic.description = format!("Component {} had {} issues", component, count);
            diagnostic
                .recommendations
                .push(format!("Investigate and fix issues with {}", component));

            report.component_diagnostics.insert(component, diagnostic);
        }

        Ok(report)
    }

    async fn collect_system_info(&self) -> Result<SystemDiagnosticInfo> {
        let mut system_info = SystemDiagnosticInfo::default();

        // Собираем информацию о системе
        // В реальной реализации здесь будет сбор реальной информации
        system_info.daemon_version = Some("1.0.0".to_string());
        system_info.os_version = Some("Linux".to_string());
        system_info.kernel_version = Some("5.15.0".to_string());
        system_info.cpu_info = Some("Intel/AMD CPU".to_string());
        system_info.memory_info = Some("16GB RAM".to_string());
        system_info.disk_info = Some("500GB SSD".to_string());

        Ok(system_info)
    }
}

impl DiagnosticAnalyzerImpl {
    /// Создать новый DiagnosticAnalyzerImpl.
    pub fn new(health_monitor: HealthMonitorImpl) -> Self {
        Self { health_monitor }
    }

    /// Создать новый DiagnosticAnalyzerImpl с HealthMonitor по умолчанию.
    pub fn new_default() -> Self {
        Self::new(create_default_health_monitor())
    }
}

/// Вспомогательная функция для создания DiagnosticAnalyzer.
pub fn create_diagnostic_analyzer(health_monitor: HealthMonitorImpl) -> DiagnosticAnalyzerImpl {
    DiagnosticAnalyzerImpl::new(health_monitor)
}

/// Вспомогательная функция для создания DiagnosticAnalyzer с HealthMonitor по умолчанию.
pub fn create_default_diagnostic_analyzer() -> DiagnosticAnalyzerImpl {
    DiagnosticAnalyzerImpl::new_default()
}
