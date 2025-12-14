//! Модуль мониторинга здоровья демона SmoothTask.
//!
//! Этот модуль предоставляет комплексную систему мониторинга здоровья демона,
//! включая проверку состояния компонентов, диагностику проблем и уведомления
//! о критических состояниях.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

pub mod diagnostics;
pub mod monitoring;
pub mod notifications;

pub use diagnostics::*;
pub use monitoring::*;
pub use notifications::*;

#[cfg(test)]
pub mod tests;

/// Основная структура для мониторинга здоровья демона.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HealthMonitor {
    /// Время последней проверки здоровья
    pub last_check_time: Option<DateTime<Utc>>,
    /// Общий статус здоровья системы
    pub overall_status: HealthStatus,
    /// Состояние отдельных компонентов
    pub component_statuses: HashMap<String, ComponentStatus>,
    /// История проблем и предупреждений
    pub issue_history: Vec<HealthIssue>,
    /// Конфигурация мониторинга здоровья
    pub config: HealthMonitorConfig,
    /// Текущий балл здоровья системы (0-100)
    pub health_score: f32,
    /// История баллов здоровья для анализа трендов
    pub health_score_history: Vec<HealthScoreEntry>,
    /// Флаги автоматического восстановления
    pub auto_recovery_flags: AutoRecoveryFlags,
}

/// Конфигурация мониторинга здоровья.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthMonitorConfig {
    /// Интервал проверки здоровья
    pub check_interval: Duration,
    /// Максимальное количество хранимых проблем в истории
    pub max_issue_history: usize,
    /// Пороги для определения критических состояний
    pub critical_thresholds: CriticalThresholds,
    /// Настройки уведомлений
    pub notification_settings: NotificationSettings,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(60),
            max_issue_history: 100,
            critical_thresholds: CriticalThresholds::default(),
            notification_settings: NotificationSettings::default(),
        }
    }
}

/// Пороги для определения критических состояний.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriticalThresholds {
    /// Максимальное время бездействия демона (в секундах)
    pub max_idle_time_seconds: u64,
    /// Максимальное количество последовательных ошибок
    pub max_consecutive_errors: usize,
    /// Максимальное использование памяти (в процентах)
    pub max_memory_usage_percent: f32,
    /// Максимальное использование CPU (в процентах)
    pub max_cpu_usage_percent: f32,
    /// Максимальная температура CPU (в градусах Цельсия)
    pub max_cpu_temperature_c: f32,
    /// Максимальная температура GPU (в градусах Цельсия)
    pub max_gpu_temperature_c: f32,
}

impl Default for CriticalThresholds {
    fn default() -> Self {
        Self {
            max_idle_time_seconds: 300, // 5 минут
            max_consecutive_errors: 10,
            max_memory_usage_percent: 90.0,
            max_cpu_usage_percent: 95.0,
            max_cpu_temperature_c: 90.0,
            max_gpu_temperature_c: 95.0,
        }
    }
}

/// Настройки уведомлений о проблемах здоровья.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NotificationSettings {
    /// Включить уведомления о критических проблемах
    pub enable_critical_notifications: bool,
    /// Включить уведомления о предупреждениях
    pub enable_warning_notifications: bool,
    /// Максимальная частота уведомлений (в секундах)
    pub max_notification_frequency_seconds: u64,
}

/// Запись балла здоровья с временной меткой.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthScoreEntry {
    /// Время записи балла
    pub timestamp: DateTime<Utc>,
    /// Балл здоровья (0-100)
    pub score: f32,
    /// Состояние системы в это время
    pub status: HealthStatus,
}

impl Default for HealthScoreEntry {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            score: 100.0,
            status: HealthStatus::Healthy,
        }
    }
}

/// Флаги автоматического восстановления.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoRecoveryFlags {
    /// Включено автоматическое восстановление
    pub auto_recovery_enabled: bool,
    /// Включено автоматическое восстановление компонентов
    pub component_auto_recovery_enabled: bool,
    /// Включено автоматическое восстановление ресурсов
    pub resource_auto_recovery_enabled: bool,
    /// Включено автоматическое восстановление конфигурации
    pub config_auto_recovery_enabled: bool,
    /// Максимальное количество попыток восстановления
    pub max_recovery_attempts: usize,
    /// Время ожидания между попытками восстановления (в секундах)
    pub recovery_attempt_interval_seconds: u64,
    /// Список компонентов, для которых разрешено автоматическое восстановление
    pub allowed_recovery_components: Vec<String>,
    /// Список компонентов, для которых запрещено автоматическое восстановление
    pub blocked_recovery_components: Vec<String>,
}

impl Default for AutoRecoveryFlags {
    fn default() -> Self {
        Self {
            auto_recovery_enabled: true,
            component_auto_recovery_enabled: true,
            resource_auto_recovery_enabled: true,
            config_auto_recovery_enabled: false,
            max_recovery_attempts: 3,
            recovery_attempt_interval_seconds: 60,
            allowed_recovery_components: vec![
                "system_metrics".to_string(),
                "process_monitoring".to_string(),
                "resource_usage".to_string(),
            ],
            blocked_recovery_components: vec!["configuration".to_string()],
        }
    }
}

/// Статистика автоматического восстановления.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutoRecoveryStats {
    /// Общее количество попыток восстановления
    pub total_recovery_attempts: usize,
    /// Количество успешных восстановлений
    pub successful_recoveries: usize,
    /// Количество неудачных восстановлений
    pub failed_recoveries: usize,
    /// Время последней попытки восстановления
    pub last_recovery_attempt_time: Option<DateTime<Utc>>,
    /// Последний восстановленный компонент
    pub last_recovered_component: Option<String>,
    /// История восстановлений
    pub recovery_history: Vec<RecoveryAttempt>,
}

impl Default for AutoRecoveryStats {
    fn default() -> Self {
        Self {
            total_recovery_attempts: 0,
            successful_recoveries: 0,
            failed_recoveries: 0,
            last_recovery_attempt_time: None,
            last_recovered_component: None,
            recovery_history: Vec::new(),
        }
    }
}

/// Запись о попытке восстановления.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecoveryAttempt {
    /// Время попытки восстановления
    pub timestamp: DateTime<Utc>,
    /// Компонент, для которого выполнялось восстановление
    pub component: String,
    /// Тип восстановления
    pub recovery_type: RecoveryType,
    /// Статус попытки
    pub status: RecoveryStatus,
    /// Описание проблемы
    pub issue_description: String,
    /// Детали восстановления
    pub recovery_details: Option<String>,
}

impl Default for RecoveryAttempt {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            component: String::new(),
            recovery_type: RecoveryType::Restart,
            status: RecoveryStatus::Pending,
            issue_description: String::new(),
            recovery_details: None,
        }
    }
}

/// Тип восстановления.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryType {
    /// Перезапуск компонента
    #[serde(rename = "restart")]
    Restart,
    /// Сброс конфигурации
    #[serde(rename = "reset")]
    Reset,
    /// Очистка ресурсов
    #[serde(rename = "cleanup")]
    Cleanup,
    /// Восстановление из резервной копии
    #[serde(rename = "restore")]
    Restore,
    /// Автоматическая настройка
    #[serde(rename = "reconfigure")]
    Reconfigure,
    /// Неизвестный тип
    #[serde(rename = "unknown")]
    Unknown,
}

impl Default for RecoveryType {
    fn default() -> Self {
        Self::Restart
    }
}

impl std::fmt::Display for RecoveryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecoveryType::Restart => write!(f, "restart"),
            RecoveryType::Reset => write!(f, "reset"),
            RecoveryType::Cleanup => write!(f, "cleanup"),
            RecoveryType::Restore => write!(f, "restore"),
            RecoveryType::Reconfigure => write!(f, "reconfigure"),
            RecoveryType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Статус восстановления.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryStatus {
    /// В ожидании
    #[serde(rename = "pending")]
    Pending,
    /// В процессе
    #[serde(rename = "in_progress")]
    InProgress,
    /// Успешно
    #[serde(rename = "success")]
    Success,
    /// Неудачно
    #[serde(rename = "failed")]
    Failed,
    /// Отменено
    #[serde(rename = "cancelled")]
    Cancelled,
}

impl Default for RecoveryStatus {
    fn default() -> Self {
        Self::Pending
    }
}

impl std::fmt::Display for RecoveryStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RecoveryStatus::Pending => write!(f, "pending"),
            RecoveryStatus::InProgress => write!(f, "in_progress"),
            RecoveryStatus::Success => write!(f, "success"),
            RecoveryStatus::Failed => write!(f, "failed"),
            RecoveryStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enable_critical_notifications: true,
            enable_warning_notifications: false,
            max_notification_frequency_seconds: 300, // 5 минут
        }
    }
}

/// Статус здоровья системы.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Система работает нормально
    #[serde(rename = "healthy")]
    Healthy,
    /// Есть предупреждения, но система работает
    #[serde(rename = "warning")]
    Warning,
    /// Критические проблемы, система работает в режиме деградации
    #[serde(rename = "degraded")]
    Degraded,
    /// Критическая ошибка, система не может продолжать работу
    #[serde(rename = "critical")]
    Critical,
    /// Состояние неизвестно
    #[serde(rename = "unknown")]
    Unknown,
}

impl Default for HealthStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Статус отдельного компонента.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentStatus {
    /// Состояние компонента
    pub status: ComponentHealthStatus,
    /// Время последней проверки
    pub last_check_time: Option<DateTime<Utc>>,
    /// Сообщение о состоянии
    pub message: Option<String>,
    /// Детали ошибки (если есть)
    pub error_details: Option<String>,
    /// Количество последовательных ошибок
    pub consecutive_errors: usize,
}

impl Default for ComponentStatus {
    fn default() -> Self {
        Self {
            status: ComponentHealthStatus::Unknown,
            last_check_time: None,
            message: None,
            error_details: None,
            consecutive_errors: 0,
        }
    }
}

/// Состояние здоровья компонента.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ComponentHealthStatus {
    /// Компонент работает нормально
    #[serde(rename = "healthy")]
    Healthy,
    /// Компонент работает, но есть предупреждения
    #[serde(rename = "warning")]
    Warning,
    /// Компонент недоступен или работает с ошибками
    #[serde(rename = "unhealthy")]
    Unhealthy,
    /// Компонент отключен
    #[serde(rename = "disabled")]
    Disabled,
    /// Состояние неизвестно
    #[serde(rename = "unknown")]
    Unknown,
}

impl Default for ComponentHealthStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Информация о проблеме здоровья.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthIssue {
    /// Уникальный идентификатор проблемы
    pub issue_id: String,
    /// Время возникновения проблемы
    pub timestamp: DateTime<Utc>,
    /// Тип проблемы
    pub issue_type: HealthIssueType,
    /// Серьезность проблемы
    pub severity: HealthIssueSeverity,
    /// Компонент, связанный с проблемой
    pub component: Option<String>,
    /// Описание проблемы
    pub description: String,
    /// Детали ошибки
    pub error_details: Option<String>,
    /// Статус проблемы
    pub status: HealthIssueStatus,
    /// Время разрешения проблемы (если решена)
    pub resolved_time: Option<DateTime<Utc>>,
}

impl Default for HealthIssue {
    fn default() -> Self {
        Self {
            issue_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            issue_type: HealthIssueType::Unknown,
            severity: HealthIssueSeverity::Info,
            component: None,
            description: String::new(),
            error_details: None,
            status: HealthIssueStatus::Open,
            resolved_time: None,
        }
    }
}

/// Тип проблемы здоровья.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthIssueType {
    /// Проблема с компонентом
    #[serde(rename = "component_failure")]
    ComponentFailure,
    /// Проблема с производительностью
    #[serde(rename = "performance_issue")]
    PerformanceIssue,
    /// Проблема с ресурсами
    #[serde(rename = "resource_issue")]
    ResourceIssue,
    /// Проблема с конфигурацией
    #[serde(rename = "configuration_issue")]
    ConfigurationIssue,
    /// Проблема с зависимостями
    #[serde(rename = "dependency_issue")]
    DependencyIssue,
    /// Критическая проблема
    #[serde(rename = "critical_issue")]
    CriticalIssue,
    /// Неизвестный тип проблемы
    #[serde(rename = "unknown")]
    Unknown,
}

impl Default for HealthIssueType {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for HealthIssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthIssueType::ComponentFailure => write!(f, "component_failure"),
            HealthIssueType::PerformanceIssue => write!(f, "performance_issue"),
            HealthIssueType::ResourceIssue => write!(f, "resource_issue"),
            HealthIssueType::ConfigurationIssue => write!(f, "configuration_issue"),
            HealthIssueType::DependencyIssue => write!(f, "dependency_issue"),
            HealthIssueType::CriticalIssue => write!(f, "critical_issue"),
            HealthIssueType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Серьезность проблемы здоровья.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthIssueSeverity {
    /// Информационное сообщение
    #[serde(rename = "info")]
    Info,
    /// Предупреждение
    #[serde(rename = "warning")]
    Warning,
    /// Ошибка
    #[serde(rename = "error")]
    Error,
    /// Критическая ошибка
    #[serde(rename = "critical")]
    Critical,
}

impl Default for HealthIssueSeverity {
    fn default() -> Self {
        Self::Info
    }
}

impl std::fmt::Display for HealthIssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthIssueSeverity::Info => write!(f, "info"),
            HealthIssueSeverity::Warning => write!(f, "warning"),
            HealthIssueSeverity::Error => write!(f, "error"),
            HealthIssueSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// Статус проблемы здоровья.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthIssueStatus {
    /// Проблема открыта
    #[serde(rename = "open")]
    Open,
    /// Проблема в процессе решения
    #[serde(rename = "in_progress")]
    InProgress,
    /// Проблема решена
    #[serde(rename = "resolved")]
    Resolved,
    /// Проблема игнорируется
    #[serde(rename = "ignored")]
    Ignored,
}

impl Default for HealthIssueStatus {
    fn default() -> Self {
        Self::Open
    }
}

/// Основной интерфейс для мониторинга здоровья.
#[async_trait::async_trait]
pub trait HealthMonitorTrait: Send + Sync {
    /// Выполнить проверку здоровья.
    async fn check_health(&self) -> Result<HealthMonitor>;

    /// Обновить состояние здоровья.
    async fn update_health_status(&self, health_monitor: HealthMonitor) -> Result<()>;

    /// Получить текущее состояние здоровья.
    async fn get_health_status(&self) -> Result<HealthMonitor>;

    /// Добавить проблему здоровья.
    async fn add_health_issue(&self, issue: HealthIssue) -> Result<()>;

    /// Разрешить проблему здоровья.
    async fn resolve_health_issue(&self, issue_id: &str) -> Result<()>;

    /// Очистить историю проблем.
    async fn clear_issue_history(&self) -> Result<()>;

    /// Получить статистику автоматического восстановления.
    async fn get_recovery_stats(&self) -> Result<AutoRecoveryStats>;

    /// Очистить статистику автоматического восстановления.
    async fn clear_recovery_stats(&self) -> Result<()>;

    /// Обновить флаги автоматического восстановления.
    async fn update_auto_recovery_flags(&self, flags: AutoRecoveryFlags) -> Result<()>;
}

/// Реализация HealthMonitorTrait.
#[derive(Debug, Clone)]
pub struct HealthMonitorImpl {
    health_state: Arc<tokio::sync::RwLock<HealthMonitor>>,
    config: HealthMonitorConfig,
    recovery_stats: Arc<tokio::sync::RwLock<AutoRecoveryStats>>,
}

#[async_trait::async_trait]
impl HealthMonitorTrait for HealthMonitorImpl {
    async fn check_health(&self) -> Result<HealthMonitor> {
        let mut health_monitor = self.health_state.read().await.clone();

        // Обновляем время последней проверки
        health_monitor.last_check_time = Some(Utc::now());

        // Выполняем проверку компонентов
        health_monitor = self.check_components(health_monitor).await?;

        // Выполняем автоматическое восстановление
        self.perform_auto_recovery(&mut health_monitor).await?;

        // Определяем общий статус здоровья
        health_monitor.overall_status = self.determine_overall_status(&health_monitor);

        // Рассчитываем балл здоровья
        self.update_health_score_history(&mut health_monitor);

        Ok(health_monitor)
    }

    async fn update_health_status(&self, health_monitor: HealthMonitor) -> Result<()> {
        let mut state = self.health_state.write().await;
        *state = health_monitor;
        Ok(())
    }

    async fn get_health_status(&self) -> Result<HealthMonitor> {
        Ok(self.health_state.read().await.clone())
    }

    async fn add_health_issue(&self, issue: HealthIssue) -> Result<()> {
        let mut state = self.health_state.write().await;

        // Проверяем максимальное количество проблем в истории
        if state.issue_history.len() >= state.config.max_issue_history {
            state.issue_history.remove(0); // Удаляем самую старую проблему
        }

        // Сохраняем информацию о проблеме перед тем, как переместить её в историю
        let component = issue.component.clone();
        let severity = issue.severity;
        let description = issue.description.clone();

        state.issue_history.push(issue);

        // Обновляем статус компонента, связанного с проблемой
        if let Some(component_name) = component {
            if let Some(component_status) = state.component_statuses.get_mut(&component_name) {
                match severity {
                    HealthIssueSeverity::Critical | HealthIssueSeverity::Error => {
                        component_status.status = ComponentHealthStatus::Unhealthy;
                        component_status.consecutive_errors += 1;
                    }
                    HealthIssueSeverity::Warning => {
                        if component_status.status == ComponentHealthStatus::Healthy {
                            component_status.status = ComponentHealthStatus::Warning;
                        }
                    }
                    _ => {}
                }

                component_status.error_details = Some(description);
                component_status.last_check_time = Some(Utc::now());
            }
        }

        Ok(())
    }

    async fn resolve_health_issue(&self, issue_id: &str) -> Result<()> {
        let mut state = self.health_state.write().await;

        if let Some(issue) = state
            .issue_history
            .iter_mut()
            .find(|i| i.issue_id == issue_id)
        {
            // Сохраняем компонент перед обновлением статуса
            let component = issue.component.clone();

            issue.status = HealthIssueStatus::Resolved;
            issue.resolved_time = Some(Utc::now());

            // Обновляем статус компонента
            if let Some(component_name) = component {
                if let Some(component_status) = state.component_statuses.get_mut(&component_name) {
                    component_status.status = ComponentHealthStatus::Healthy;
                    component_status.consecutive_errors = 0;
                    component_status.error_details = None;
                }
            }
        }

        Ok(())
    }

    async fn clear_issue_history(&self) -> Result<()> {
        let mut state = self.health_state.write().await;
        state.issue_history.clear();
        Ok(())
    }

    /// Получить статистику автоматического восстановления.
    async fn get_recovery_stats(&self) -> Result<AutoRecoveryStats> {
        Ok(self.recovery_stats.read().await.clone())
    }

    /// Очистить статистику автоматического восстановления.
    async fn clear_recovery_stats(&self) -> Result<()> {
        let mut stats = self.recovery_stats.write().await;
        *stats = AutoRecoveryStats::default();
        Ok(())
    }

    /// Обновить флаги автоматического восстановления.
    async fn update_auto_recovery_flags(&self, flags: AutoRecoveryFlags) -> Result<()> {
        let mut state = self.health_state.write().await;
        state.auto_recovery_flags = flags;
        Ok(())
    }
}

impl HealthMonitorImpl {
    /// Создать новый HealthMonitorImpl.
    pub fn new(config: HealthMonitorConfig) -> Self {
        Self {
            health_state: Arc::new(tokio::sync::RwLock::new(HealthMonitor::default())),
            config,
            recovery_stats: Arc::new(tokio::sync::RwLock::new(AutoRecoveryStats::default())),
        }
    }

    /// Создать новый HealthMonitorImpl с конфигурацией по умолчанию.
    pub fn new_default() -> Self {
        Self::new(HealthMonitorConfig::default())
    }

    /// Выполнить автоматическое восстановление компонентов.
    async fn perform_auto_recovery(&self, health_monitor: &mut HealthMonitor) -> Result<()> {
        // Проверяем, включено ли автоматическое восстановление
        if !health_monitor.auto_recovery_flags.auto_recovery_enabled {
            debug!("Auto-recovery is disabled");
            return Ok(());
        }

        let mut recovery_attempts = Vec::new();

        // Собираем список компонентов, которые нужно восстановить
        let components_to_recover: Vec<String> = health_monitor
            .component_statuses
            .iter()
            .filter(|(_, component_status)| {
                component_status.status == ComponentHealthStatus::Unhealthy
            })
            .filter(|(component_name, _)| self.is_recovery_allowed(component_name, health_monitor))
            .map(|(component_name, _)| component_name.clone())
            .collect();

        // Выполняем восстановление для каждого компонента
        for component_name in components_to_recover {
            info!("Attempting auto-recovery for component: {}", component_name);

            // Получаем текущий статус компонента
            let component_status = health_monitor
                .component_statuses
                .get(&component_name)
                .expect("Component should exist");

            // Выполняем восстановление
            let recovery_result = self
                .attempt_component_recovery(&component_name, component_status)
                .await;

            // Обновляем статистику восстановления
            let mut stats = self.recovery_stats.write().await;
            stats.total_recovery_attempts += 1;

            let recovery_attempt = RecoveryAttempt {
                timestamp: Utc::now(),
                component: component_name.clone(),
                recovery_type: RecoveryType::Restart,
                status: if recovery_result.is_ok() {
                    RecoveryStatus::Success
                } else {
                    RecoveryStatus::Failed
                },
                issue_description: component_status.error_details.clone().unwrap_or_default(),
                recovery_details: Some(format!(
                    "Auto-recovery attempt: {}",
                    if recovery_result.is_ok() {
                        "success"
                    } else {
                        "failed"
                    }
                )),
            };

            if recovery_result.is_ok() {
                stats.successful_recoveries += 1;
                stats.last_recovered_component = Some(component_name.clone());
            } else {
                stats.failed_recoveries += 1;
            }

            stats.last_recovery_attempt_time = Some(Utc::now());
            stats.recovery_history.push(recovery_attempt.clone());

            recovery_attempts.push(recovery_attempt);

            // Если восстановление успешно, обновляем статус компонента
            if recovery_result.is_ok() {
                if let Some(component_status) =
                    health_monitor.component_statuses.get_mut(&component_name)
                {
                    component_status.status = ComponentHealthStatus::Healthy;
                    component_status.consecutive_errors = 0;
                    component_status.error_details = None;
                    component_status.message = Some("Component recovered successfully".to_string());
                }
            }
        }

        // Если были попытки восстановления, обновляем историю проблем
        if !recovery_attempts.is_empty() {
            for attempt in recovery_attempts {
                let issue = HealthIssue {
                    issue_id: uuid::Uuid::new_v4().to_string(),
                    timestamp: attempt.timestamp,
                    issue_type: HealthIssueType::ComponentFailure,
                    severity: if attempt.status == RecoveryStatus::Success {
                        HealthIssueSeverity::Info
                    } else {
                        HealthIssueSeverity::Warning
                    },
                    component: Some(attempt.component.clone()),
                    description: format!("Auto-recovery attempt: {}", attempt.status),
                    error_details: Some(attempt.recovery_details.unwrap_or_default()),
                    status: HealthIssueStatus::Resolved,
                    resolved_time: Some(Utc::now()),
                };

                self.add_health_issue(issue).await.ok();
            }
        }

        Ok(())
    }

    /// Проверка, разрешено ли восстановление для компонента.
    fn is_recovery_allowed(&self, component_name: &str, health_monitor: &HealthMonitor) -> bool {
        // Проверяем общие флаги восстановления
        if !health_monitor
            .auto_recovery_flags
            .component_auto_recovery_enabled
        {
            return false;
        }

        // Проверяем, не заблокирован ли компонент
        if health_monitor
            .auto_recovery_flags
            .blocked_recovery_components
            .contains(&component_name.to_string())
        {
            debug!("Recovery blocked for component: {}", component_name);
            return false;
        }

        // Проверяем, разрешено ли восстановление для этого компонента
        if !health_monitor
            .auto_recovery_flags
            .allowed_recovery_components
            .is_empty()
        {
            if !health_monitor
                .auto_recovery_flags
                .allowed_recovery_components
                .contains(&component_name.to_string())
            {
                debug!("Recovery not allowed for component: {}", component_name);
                return false;
            }
        }

        true
    }

    /// Попытка восстановления компонента.
    async fn attempt_component_recovery(
        &self,
        component_name: &str,
        component_status: &ComponentStatus,
    ) -> Result<()> {
        info!(
            "Attempting recovery for component: {} (status: {:?})",
            component_name, component_status.status
        );

        // В реальной реализации здесь будут конкретные действия по восстановлению
        // Например: перезапуск службы, сброс кэша, восстановление конфигурации и т.д.

        // Логируем детали проблемы, если они есть
        if let Some(error_details) = &component_status.error_details {
            warn!("Component error details: {}", error_details);
        }

        match component_name {
            "system_metrics" => {
                // Восстановление системных метрик
                self.recover_system_metrics().await?;
            }
            "process_monitoring" => {
                // Восстановление мониторинга процессов
                self.recover_process_monitoring().await?;
            }
            "resource_usage" => {
                // Восстановление использования ресурсов
                self.recover_resource_usage().await?;
            }
            _ => {
                // Универсальное восстановление для других компонентов
                self.generic_component_recovery(component_name).await?;
            }
        }

        info!("Successfully recovered component: {}", component_name);
        Ok(())
    }

    /// Восстановление системных метрик.
    async fn recover_system_metrics(&self) -> Result<()> {
        info!("Recovering system metrics component");

        // В реальной реализации здесь будут действия по восстановлению
        // Например: очистка кэша, перезапуск сбора метрик и т.д.

        // Имитируем успешное восстановление
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    /// Восстановление мониторинга процессов.
    async fn recover_process_monitoring(&self) -> Result<()> {
        info!("Recovering process monitoring component");

        // В реальной реализации здесь будут действия по восстановлению
        // Например: перезапуск мониторинга процессов, очистка состояния и т.д.

        // Имитируем успешное восстановление
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    /// Восстановление использования ресурсов.
    async fn recover_resource_usage(&self) -> Result<()> {
        info!("Recovering resource usage component");

        // В реальной реализации здесь будут действия по восстановлению
        // Например: очистка кэша ресурсов, сброс счетчиков и т.д.

        // Имитируем успешное восстановление
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    /// Универсальное восстановление компонента.
    async fn generic_component_recovery(&self, component_name: &str) -> Result<()> {
        info!(
            "Performing generic recovery for component: {}",
            component_name
        );

        // В реальной реализации здесь будут универсальные действия по восстановлению

        // Имитируем успешное восстановление
        tokio::time::sleep(Duration::from_millis(100)).await;

        Ok(())
    }

    /// Рассчитать балл здоровья системы.
    fn calculate_health_score(&self, health_monitor: &HealthMonitor) -> f32 {
        // Начинаем с максимального балла
        let mut score = 100.0;

        // Учитываем состояние компонентов
        for (_, component_status) in &health_monitor.component_statuses {
            match component_status.status {
                ComponentHealthStatus::Healthy => {
                    // Здоровый компонент не снижает балл
                }
                ComponentHealthStatus::Warning => {
                    // Предупреждение снижает балл на 5
                    score -= 5.0;
                }
                ComponentHealthStatus::Unhealthy => {
                    // Нездоровый компонент снижает балл на 15
                    score -= 15.0;
                }
                ComponentHealthStatus::Disabled => {
                    // Отключенный компонент снижает балл на 10
                    score -= 10.0;
                }
                ComponentHealthStatus::Unknown => {
                    // Неизвестное состояние снижает балл на 5
                    score -= 5.0;
                }
            }
        }

        // Учитываем историю проблем
        let recent_issues = health_monitor
            .issue_history
            .iter()
            .filter(|issue| issue.status == HealthIssueStatus::Open)
            .count();

        // Каждая открытая проблема снижает балл на 2
        score -= recent_issues as f32 * 2.0;

        // Учитываем последовательные ошибки
        for (_, component_status) in &health_monitor.component_statuses {
            if component_status.consecutive_errors > 0 {
                // Каждая последовательная ошибка снижает балл на 1
                score -= component_status.consecutive_errors as f32;
            }
        }

        // Ограничиваем балл в диапазоне 0-100
        score = score.clamp(0.0, 100.0);

        score
    }

    /// Обновить историю баллов здоровья.
    fn update_health_score_history(&self, health_monitor: &mut HealthMonitor) {
        let score = self.calculate_health_score(health_monitor);
        health_monitor.health_score = score;

        let entry = HealthScoreEntry {
            timestamp: Utc::now(),
            score,
            status: health_monitor.overall_status,
        };

        health_monitor.health_score_history.push(entry);

        // Ограничиваем историю (например, 100 записей)
        if health_monitor.health_score_history.len() > 100 {
            health_monitor.health_score_history.remove(0);
        }
    }

    /// Проверка состояния компонентов.
    async fn check_components(&self, mut health_monitor: HealthMonitor) -> Result<HealthMonitor> {
        // Проверяем основные компоненты
        health_monitor.component_statuses.insert(
            "system_metrics".to_string(),
            self.check_system_metrics().await?,
        );

        health_monitor.component_statuses.insert(
            "process_monitoring".to_string(),
            self.check_process_monitoring().await?,
        );

        health_monitor.component_statuses.insert(
            "resource_usage".to_string(),
            self.check_resource_usage().await?,
        );

        health_monitor.component_statuses.insert(
            "configuration".to_string(),
            self.check_configuration().await?,
        );

        Ok(health_monitor)
    }

    /// Определить общий статус здоровья.
    fn determine_overall_status(&self, health_monitor: &HealthMonitor) -> HealthStatus {
        let mut has_critical = false;
        let mut has_warning = false;

        for component_status in health_monitor.component_statuses.values() {
            match component_status.status {
                ComponentHealthStatus::Unhealthy => {
                    has_critical = true;
                }
                ComponentHealthStatus::Warning => {
                    has_warning = true;
                }
                _ => {}
            }
        }

        if has_critical {
            HealthStatus::Degraded
        } else if has_warning {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        }
    }

    /// Проверка системных метрик.
    async fn check_system_metrics(&self) -> Result<ComponentStatus> {
        // Проверяем доступность системных метрик
        let proc_stat_path = Path::new("/proc/stat");
        let proc_meminfo_path = Path::new("/proc/meminfo");
        let proc_loadavg_path = Path::new("/proc/loadavg");

        let mut status = ComponentHealthStatus::Healthy;
        let mut message = "System metrics are healthy".to_string();
        let mut error_details = None;

        // Проверяем доступность основных файлов /proc
        if !proc_stat_path.exists() {
            status = ComponentHealthStatus::Unhealthy;
            message = "System metrics files not accessible".to_string();
            error_details = Some("/proc/stat not found".to_string());
        }

        if !proc_meminfo_path.exists() {
            status = ComponentHealthStatus::Unhealthy;
            message = "System metrics files not accessible".to_string();
            error_details = Some("/proc/meminfo not found".to_string());
        }

        if !proc_loadavg_path.exists() {
            status = ComponentHealthStatus::Unhealthy;
            message = "System metrics files not accessible".to_string();
            error_details = Some("/proc/loadavg not found".to_string());
        }

        Ok(ComponentStatus {
            status,
            last_check_time: Some(Utc::now()),
            message: Some(message),
            error_details,
            consecutive_errors: 0,
        })
    }

    /// Проверка мониторинга процессов.
    async fn check_process_monitoring(&self) -> Result<ComponentStatus> {
        // Проверяем доступность /proc для мониторинга процессов
        let proc_path = Path::new("/proc");

        let mut status = ComponentHealthStatus::Healthy;
        let mut message = "Process monitoring is healthy".to_string();
        let mut error_details = None;

        if !proc_path.exists() {
            status = ComponentHealthStatus::Unhealthy;
            message = "Process monitoring not available".to_string();
            error_details = Some("/proc not accessible".to_string());
        } else {
            // Проверяем, что можно читать информацию о процессах
            match std::fs::read_dir(proc_path) {
                Ok(entries) => {
                    let count = entries.count();
                    if count < 10 {
                        status = ComponentHealthStatus::Warning;
                        message = "Low number of processes detected".to_string();
                        error_details = Some(format!("Only {} process entries found", count));
                    }
                }
                Err(e) => {
                    status = ComponentHealthStatus::Unhealthy;
                    message = "Cannot read process information".to_string();
                    error_details = Some(format!("Error reading /proc: {}", e));
                }
            }
        }

        Ok(ComponentStatus {
            status,
            last_check_time: Some(Utc::now()),
            message: Some(message),
            error_details,
            consecutive_errors: 0,
        })
    }

    /// Проверка использования ресурсов.
    async fn check_resource_usage(&self) -> Result<ComponentStatus> {
        // Проверяем использование ресурсов
        let mut status = ComponentHealthStatus::Healthy;
        let mut message = "Resource usage is healthy".to_string();
        let mut error_details = None;

        // Проверяем доступность системных метрик
        let proc_meminfo_path = Path::new("/proc/meminfo");

        if proc_meminfo_path.exists() {
            match std::fs::read_to_string(proc_meminfo_path) {
                Ok(contents) => {
                    // Парсим информацию о памяти
                    let mut mem_total = 0u64;
                    let mut mem_available = 0u64;

                    for line in contents.lines() {
                        if line.starts_with("MemTotal:") {
                            if let Some(value) = line.split_whitespace().nth(1) {
                                if let Ok(val) = value.parse::<u64>() {
                                    mem_total = val;
                                }
                            }
                        }
                        if line.starts_with("MemAvailable:") {
                            if let Some(value) = line.split_whitespace().nth(1) {
                                if let Ok(val) = value.parse::<u64>() {
                                    mem_available = val;
                                }
                            }
                        }
                    }

                    if mem_total > 0 {
                        let used_percent =
                            100.0 * (mem_total - mem_available) as f32 / mem_total as f32;

                        if used_percent > self.config.critical_thresholds.max_memory_usage_percent {
                            status = ComponentHealthStatus::Unhealthy;
                            message = "High memory usage detected".to_string();
                            error_details = Some(format!("Memory usage: {:.1}%", used_percent));
                        } else if used_percent > 80.0 {
                            status = ComponentHealthStatus::Warning;
                            message = "Memory usage is high".to_string();
                            error_details = Some(format!("Memory usage: {:.1}%", used_percent));
                        }
                    }
                }
                Err(e) => {
                    status = ComponentHealthStatus::Warning;
                    message = "Cannot read memory information".to_string();
                    error_details = Some(format!("Error reading /proc/meminfo: {}", e));
                }
            }
        } else {
            status = ComponentHealthStatus::Warning;
            message = "Memory information not available".to_string();
            error_details = Some("/proc/meminfo not found".to_string());
        }

        Ok(ComponentStatus {
            status,
            last_check_time: Some(Utc::now()),
            message: Some(message),
            error_details,
            consecutive_errors: 0,
        })
    }

    /// Проверка конфигурации.
    async fn check_configuration(&self) -> Result<ComponentStatus> {
        // Проверяем конфигурацию мониторинга здоровья
        let mut status = ComponentHealthStatus::Healthy;
        let mut message = "Configuration is healthy".to_string();
        let mut error_details = None;

        // Проверяем, что конфигурация валидна
        let config = self.health_state.read().await.config.clone();

        if config.check_interval.as_secs() == 0 {
            status = ComponentHealthStatus::Warning;
            message = "Invalid check interval".to_string();
            error_details = Some("Check interval cannot be zero".to_string());
        }

        if config.max_issue_history == 0 {
            status = ComponentHealthStatus::Warning;
            message = "Invalid issue history size".to_string();
            error_details = Some("Issue history size cannot be zero".to_string());
        }

        Ok(ComponentStatus {
            status,
            last_check_time: Some(Utc::now()),
            message: Some(message),
            error_details,
            consecutive_errors: 0,
        })
    }
}

/// Вспомогательная функция для создания HealthMonitor.
pub fn create_health_monitor(config: HealthMonitorConfig) -> HealthMonitorImpl {
    HealthMonitorImpl::new(config)
}

/// Вспомогательная функция для создания HealthMonitor с конфигурацией по умолчанию.
pub fn create_default_health_monitor() -> HealthMonitorImpl {
    HealthMonitorImpl::new_default()
}
