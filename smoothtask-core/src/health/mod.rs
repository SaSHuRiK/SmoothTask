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

pub mod container_health;
pub mod diagnostics;
pub mod monitoring;
pub mod notifications;
pub mod security_monitoring;
pub mod threat_intelligence;

pub use container_health::*;
pub use diagnostics::*;
pub use monitoring::*;
pub use notifications::*;
pub use security_monitoring::*;
pub use threat_intelligence::*;

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
    /// Отслеживание ошибок по типам и частоте
    pub error_patterns: HashMap<String, ErrorPattern>,
    /// История критических ошибок
    pub critical_error_history: Vec<EnhancedErrorContext>,
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

/// Паттерн ошибки для отслеживания повторяющихся проблем.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorPattern {
    /// Тип ошибки
    pub error_type: String,
    /// Количество возникновений
    pub occurrence_count: usize,
    /// Время первого возникновения
    pub first_occurrence: DateTime<Utc>,
    /// Время последнего возникновения
    pub last_occurrence: DateTime<Utc>,
    /// Компоненты, в которых возникала ошибка
    pub affected_components: Vec<String>,
    /// Серьезность ошибки
    pub severity: HealthIssueSeverity,
    /// Частота возникновения (в минуту)
    pub frequency_per_minute: f32,
    /// Последний контекст ошибки
    pub last_error_context: Option<EnhancedErrorContext>,
}

impl Default for ErrorPattern {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            error_type: String::new(),
            occurrence_count: 0,
            first_occurrence: now,
            last_occurrence: now,
            affected_components: Vec::new(),
            severity: HealthIssueSeverity::Info,
            frequency_per_minute: 0.0,
            last_error_context: None,
        }
    }
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

/// Расширенная информация об ошибке с контекстом.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnhancedErrorContext {
    /// Время возникновения ошибки
    pub error_timestamp: DateTime<Utc>,
    /// Имя компонента, в котором произошла ошибка
    pub component_name: String,
    /// Тип ошибки
    pub error_type: String,
    /// Основное сообщение об ошибке
    pub error_message: String,
    /// Детализированное описание ошибки
    pub error_details: Option<String>,
    /// Стек вызовов (если доступен)
    pub stack_trace: Option<String>,
    /// Контекст выполнения
    pub execution_context: Option<String>,
    /// Связанные метрики и состояние системы
    pub system_context: Option<String>,
    /// Количество повторений этой ошибки
    pub occurrence_count: usize,
    /// Время первого возникновения
    pub first_occurrence: DateTime<Utc>,
    /// Время последнего возникновения
    pub last_occurrence: DateTime<Utc>,
    /// Рекомендации по восстановлению
    pub recovery_suggestions: Option<String>,
}

impl Default for EnhancedErrorContext {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            error_timestamp: now,
            component_name: String::new(),
            error_type: String::new(),
            error_message: String::new(),
            error_details: None,
            stack_trace: None,
            execution_context: None,
            system_context: None,
            occurrence_count: 1,
            first_occurrence: now,
            last_occurrence: now,
            recovery_suggestions: None,
        }
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
    /// Расширенный контекст ошибки
    pub enhanced_error_context: Option<EnhancedErrorContext>,
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
            enhanced_error_context: None,
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
    /// Информационное сообщение
    #[serde(rename = "info")]
    Info,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
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

    /// Добавить расширенную проблему здоровья с контекстом ошибки.
    async fn add_enhanced_health_issue(
        &self,
        component_name: &str,
        error_type: &str,
        error_message: &str,
        error_details: Option<String>,
        stack_trace: Option<String>,
        execution_context: Option<String>,
        system_context: Option<String>,
        severity: HealthIssueSeverity,
    ) -> Result<()>;

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

    /// Выполнить улучшенное автоматическое восстановление.
    async fn enhanced_auto_recovery(
        &self,
        component_name: &str,
        error_context: EnhancedErrorContext,
    ) -> Result<RecoveryAttempt>;

    /// Проанализировать паттерны ошибок.
    async fn analyze_error_patterns(&self) -> Result<Vec<ErrorPatternAnalysis>>;

    /// Улучшенный анализ паттернов ошибок с классификацией и трендами.
    async fn enhanced_error_pattern_analysis(&self) -> Result<Vec<EnhancedErrorPatternAnalysis>>;

    /// Улучшенное автоматическое восстановление с расширенным контекстом и анализом.
    async fn enhanced_auto_recovery_with_analysis(
        &self,
        component_name: &str,
        error_context: EnhancedErrorContext,
    ) -> Result<RecoveryAttempt>;

    /// Создать расширенный контекст ошибки.
    fn create_enhanced_error_context(
        &self,
        component_name: &str,
        error_type: &str,
        error_message: &str,
        error_details: Option<String>,
        stack_trace: Option<String>,
        execution_context: Option<String>,
        system_context: Option<String>,
    ) -> EnhancedErrorContext;

    /// Получить рекомендации по восстановлению.
    fn get_recovery_suggestions(&self, error_type: &str, component_name: &str) -> String;
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

        // Выполняем улучшенное автоматическое восстановление
        self.perform_enhanced_auto_recovery(&mut health_monitor)
            .await?;

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

    /// Добавить расширенную проблему здоровья с контекстом ошибки.
    async fn add_enhanced_health_issue(
        &self,
        component_name: &str,
        error_type: &str,
        error_message: &str,
        error_details: Option<String>,
        stack_trace: Option<String>,
        execution_context: Option<String>,
        system_context: Option<String>,
        severity: HealthIssueSeverity,
    ) -> Result<()> {
        self.add_enhanced_health_issue(
            component_name,
            error_type,
            error_message,
            error_details,
            stack_trace,
            execution_context,
            system_context,
            severity,
        )
        .await
    }

    /// Выполнить улучшенное автоматическое восстановление.
    async fn enhanced_auto_recovery(
        &self,
        component_name: &str,
        error_context: EnhancedErrorContext,
    ) -> Result<RecoveryAttempt> {
        self.enhanced_auto_recovery(component_name, error_context)
            .await
    }

    /// Проанализировать паттерны ошибок.
    async fn analyze_error_patterns(&self) -> Result<Vec<ErrorPatternAnalysis>> {
        self.analyze_error_patterns().await
    }

    /// Улучшенный анализ паттернов ошибок с классификацией и трендами.
    async fn enhanced_error_pattern_analysis(&self) -> Result<Vec<EnhancedErrorPatternAnalysis>> {
        self.enhanced_error_pattern_analysis().await
    }

    /// Улучшенное автоматическое восстановление с расширенным контекстом и анализом.
    async fn enhanced_auto_recovery_with_analysis(
        &self,
        component_name: &str,
        error_context: EnhancedErrorContext,
    ) -> Result<RecoveryAttempt> {
        self.enhanced_auto_recovery_with_analysis(component_name, error_context)
            .await
    }

    /// Создать расширенный контекст ошибки.
    fn create_enhanced_error_context(
        &self,
        component_name: &str,
        error_type: &str,
        error_message: &str,
        error_details: Option<String>,
        stack_trace: Option<String>,
        execution_context: Option<String>,
        system_context: Option<String>,
    ) -> EnhancedErrorContext {
        self.create_enhanced_error_context(
            component_name,
            error_type,
            error_message,
            error_details,
            stack_trace,
            execution_context,
            system_context,
        )
    }

    /// Получить рекомендации по восстановлению.
    fn get_recovery_suggestions(&self, error_type: &str, component_name: &str) -> String {
        self.get_recovery_suggestions(error_type, component_name)
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

            recovery_attempts.push(recovery_attempt.clone());

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
                    enhanced_error_context: None,
                    status: HealthIssueStatus::Resolved,
                    resolved_time: Some(Utc::now()),
                };

                self.add_health_issue(issue).await.ok();
            }
        }

        Ok(())
    }

    /// Выполнить улучшенное автоматическое восстановление компонентов.
    async fn perform_enhanced_auto_recovery(
        &self,
        health_monitor: &mut HealthMonitor,
    ) -> Result<()> {
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
            info!(
                "Attempting enhanced auto-recovery for component: {}",
                component_name
            );

            // Получаем текущий статус компонента
            let component_status = health_monitor
                .component_statuses
                .get(&component_name)
                .expect("Component should exist");

            // Создаем расширенный контекст ошибки
            let error_context = self.create_enhanced_error_context(
                &component_name,
                "component_failure",
                &format!("Component {} is unhealthy", component_name),
                component_status.error_details.clone(),
                None,
                None,
                None,
            );

            // Выполняем улучшенное восстановление
            let recovery_result = self
                .enhanced_auto_recovery(&component_name, error_context)
                .await;

            // Обновляем статистику восстановления
            let mut stats = self.recovery_stats.write().await;
            stats.total_recovery_attempts += 1;

            let recovery_attempt = recovery_result?;

            if recovery_attempt.status == RecoveryStatus::Success {
                stats.successful_recoveries += 1;
                stats.last_recovered_component = Some(component_name.clone());
            } else {
                stats.failed_recoveries += 1;
            }

            stats.last_recovery_attempt_time = Some(Utc::now());
            stats.recovery_history.push(recovery_attempt.clone());

            recovery_attempts.push(recovery_attempt.clone());

            // Если восстановление успешно, обновляем статус компонента
            if recovery_attempt.status == RecoveryStatus::Success {
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
                    description: format!("Enhanced auto-recovery attempt: {}", attempt.status),
                    error_details: Some(attempt.recovery_details.unwrap_or_default()),
                    enhanced_error_context: None,
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

        // Проверяем безопасность системы
        health_monitor
            .component_statuses
            .insert("security".to_string(), self.check_security().await?);

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

    /// Проверка безопасности системы.
    async fn check_security(&self) -> Result<ComponentStatus> {
        // Создаем временный SecurityMonitor для проверки
        let security_config = SecurityMonitorConfig::default();
        let security_monitor = SecurityMonitorImpl::new(security_config);

        // Выполняем проверку безопасности
        match security_monitor.check_security().await {
            Ok(security_status) => {
                let (status, message, error_details) = match security_status.overall_status {
                    SecurityStatus::Secure => (
                        ComponentHealthStatus::Healthy,
                        "Security is healthy".to_string(),
                        None,
                    ),
                    SecurityStatus::Warning => (
                        ComponentHealthStatus::Warning,
                        "Security warnings detected".to_string(),
                        Some(format!(
                            "Security score: {:.1}, events: {}",
                            security_status.security_score,
                            security_status.event_history.len()
                        )),
                    ),
                    SecurityStatus::PotentialThreat | SecurityStatus::CriticalThreat => (
                        ComponentHealthStatus::Unhealthy,
                        "Security threats detected".to_string(),
                        Some(format!(
                            "Security score: {:.1}, critical events: {}",
                            security_status.security_score,
                            security_status.event_history.len()
                        )),
                    ),
                    SecurityStatus::Unknown => (
                        ComponentHealthStatus::Unknown,
                        "Security status unknown".to_string(),
                        None,
                    ),
                };

                Ok(ComponentStatus {
                    status,
                    last_check_time: Some(Utc::now()),
                    message: Some(message),
                    error_details,
                    consecutive_errors: 0,
                })
            }
            Err(e) => {
                error!("Failed to check security: {}", e);
                Ok(ComponentStatus {
                    status: ComponentHealthStatus::Unhealthy,
                    last_check_time: Some(Utc::now()),
                    message: Some("Security check failed".to_string()),
                    error_details: Some(format!("Error: {}", e)),
                    consecutive_errors: 1,
                })
            }
        }
    }
}

impl HealthMonitorImpl {
    /// Создать расширенный контекст ошибки.
    pub fn create_enhanced_error_context(
        &self,
        component_name: &str,
        error_type: &str,
        error_message: &str,
        error_details: Option<String>,
        stack_trace: Option<String>,
        execution_context: Option<String>,
        system_context: Option<String>,
    ) -> EnhancedErrorContext {
        EnhancedErrorContext {
            error_timestamp: Utc::now(),
            component_name: component_name.to_string(),
            error_type: error_type.to_string(),
            error_message: error_message.to_string(),
            error_details,
            stack_trace,
            execution_context,
            system_context,
            occurrence_count: 1,
            first_occurrence: Utc::now(),
            last_occurrence: Utc::now(),
            recovery_suggestions: None,
        }
    }

    /// Обновить отслеживание паттернов ошибок.
    pub async fn update_error_patterns(
        &self,
        error_type: &str,
        component_name: &str,
        severity: HealthIssueSeverity,
        error_context: EnhancedErrorContext,
    ) -> Result<()> {
        let mut state = self.health_state.write().await;

        // Обновляем или создаем паттерн ошибки
        let pattern = state
            .error_patterns
            .entry(error_type.to_string())
            .or_insert_with(|| ErrorPattern {
                error_type: error_type.to_string(),
                occurrence_count: 0,
                first_occurrence: Utc::now(),
                last_occurrence: Utc::now(),
                affected_components: Vec::new(),
                severity,
                frequency_per_minute: 0.0,
                last_error_context: None,
            });

        // Обновляем паттерн
        pattern.occurrence_count += 1;
        pattern.last_occurrence = Utc::now();

        if !pattern
            .affected_components
            .contains(&component_name.to_string())
        {
            pattern.affected_components.push(component_name.to_string());
        }

        // Обновляем серьезность, если текущая ошибка более серьезная
        if severity > pattern.severity {
            pattern.severity = severity;
        }

        // Обновляем контекст ошибки
        pattern.last_error_context = Some(error_context.clone());

        // Если ошибка критическая, добавляем в историю критических ошибок
        if severity == HealthIssueSeverity::Critical {
            state.critical_error_history.push(error_context);
            // Ограничиваем историю критических ошибок
            if state.critical_error_history.len() > 50 {
                state.critical_error_history.remove(0);
            }
        }

        Ok(())
    }

    /// Получить рекомендации по восстановлению на основе типа ошибки.
    pub fn get_recovery_suggestions(&self, error_type: &str, component_name: &str) -> String {
        match error_type {
            "resource_exhaustion" => {
                format!(
                    "Resource exhaustion detected in {}. Recommendations: \n1. Check system resource usage\n2. Increase resource limits if possible\n3. Optimize {} operations\n4. Consider load balancing or scaling\n5. Review memory/CPU usage patterns",
                    component_name, component_name
                )
            }
            "connection_failed" => {
                format!(
                    "Connection failure in {}. Recommendations: \n1. Check network connectivity\n2. Verify service availability\n3. Review {} configuration\n4. Check firewall and security settings\n5. Test network latency and bandwidth",
                    component_name, component_name
                )
            }
            "configuration_error" => {
                format!(
                    "Configuration error in {}. Recommendations: \n1. Validate configuration files\n2. Check {} configuration parameters\n3. Review recent configuration changes\n4. Restore from backup if needed\n5. Verify configuration syntax and schema",
                    component_name, component_name
                )
            }
            "permission_denied" => {
                format!(
                    "Permission denied in {}. Recommendations: \n1. Check file/directory permissions\n2. Verify {} has proper access rights\n3. Review security policies\n4. Consider running with appropriate privileges\n5. Audit permission requirements for the component",
                    component_name, component_name
                )
            }
            "timeout" => {
                format!(
                    "Timeout occurred in {}. Recommendations: \n1. Check system load and performance\n2. Review {} timeout settings\n3. Optimize operations to complete faster\n4. Consider increasing timeout values\n5. Analyze operation complexity and dependencies",
                    component_name, component_name
                )
            }
            "database_error" => {
                format!(
                    "Database error in {}. Recommendations: \n1. Check database connectivity\n2. Verify database service status\n3. Review {} database queries\n4. Check database resource usage\n5. Optimize database schema and indexes",
                    component_name, component_name
                )
            }
            "network_error" => {
                format!(
                    "Network error in {}. Recommendations: \n1. Check network interface status\n2. Verify network configuration\n3. Review {} network dependencies\n4. Test network connectivity and DNS\n5. Analyze network traffic patterns",
                    component_name, component_name
                )
            }
            "security_error" => {
                format!(
                    "Security error in {}. Recommendations: \n1. Review security policies\n2. Check authentication and authorization\n3. Audit {} security configuration\n4. Verify security certificates and keys\n5. Analyze security event logs",
                    component_name, component_name
                )
            }
            _ => {
                format!(
                    "Error detected in {}. General recommendations: \n1. Check system logs for details\n2. Review {} configuration\n3. Verify dependencies and requirements\n4. Consider restarting the component\n5. Collect diagnostic information for analysis",
                    component_name, component_name
                )
            }
        }
    }

    /// Улучшенное автоматическое восстановление с расширенным контекстом и анализом.
    pub async fn enhanced_auto_recovery_with_analysis(
        &self,
        component_name: &str,
        error_context: EnhancedErrorContext,
    ) -> Result<RecoveryAttempt> {
        let mut recovery_attempt = RecoveryAttempt {
            timestamp: Utc::now(),
            component: component_name.to_string(),
            recovery_type: RecoveryType::Restart,
            status: RecoveryStatus::InProgress,
            issue_description: error_context.error_message.clone(),
            recovery_details: Some("Enhanced auto-recovery with analysis initiated".to_string()),
        };

        // Анализируем контекст ошибки для определения лучшей стратегии восстановления
        let recovery_strategy = self.determine_recovery_strategy(&error_context);
        recovery_attempt.recovery_type = recovery_strategy;

        // Получаем рекомендации по восстановлению
        let suggestions = self.get_recovery_suggestions(&error_context.error_type, component_name);

        // Обновляем контекст ошибки с рекомендациями
        let mut updated_context = error_context;
        updated_context.recovery_suggestions = Some(suggestions.clone());

        // Выполняем восстановление в зависимости от стратегии
        let recovery_result = match recovery_strategy {
            RecoveryType::Restart => self.perform_restart_recovery(component_name).await,
            RecoveryType::Reset => self.perform_reset_recovery(component_name).await,
            RecoveryType::Cleanup => self.perform_cleanup_recovery(component_name).await,
            RecoveryType::Restore => self.perform_restore_recovery(component_name).await,
            RecoveryType::Reconfigure => self.perform_reconfigure_recovery(component_name).await,
            RecoveryType::Unknown => self.perform_generic_recovery(component_name).await,
        };

        // Обновляем статус попытки восстановления
        recovery_attempt.status = if recovery_result.is_ok() {
            RecoveryStatus::Success
        } else {
            RecoveryStatus::Failed
        };

        recovery_attempt.recovery_details = Some(format!(
            "Enhanced recovery ({}): {}. Recommendations: {}",
            recovery_strategy,
            if recovery_result.is_ok() {
                "success"
            } else {
                "failed"
            },
            suggestions
        ));

        // Обновляем статистику восстановления
        let mut stats = self.recovery_stats.write().await;
        stats.total_recovery_attempts += 1;

        if recovery_result.is_ok() {
            stats.successful_recoveries += 1;
            stats.last_recovered_component = Some(component_name.to_string());
        } else {
            stats.failed_recoveries += 1;
        }

        stats.last_recovery_attempt_time = Some(Utc::now());
        stats.recovery_history.push(recovery_attempt.clone());

        Ok(recovery_attempt)
    }

    /// Определить стратегию восстановления на основе контекста ошибки.
    fn determine_recovery_strategy(&self, error_context: &EnhancedErrorContext) -> RecoveryType {
        // Анализируем тип ошибки и контекст для определения лучшей стратегии
        match error_context.error_type.as_str() {
            "configuration_error" => RecoveryType::Reconfigure,
            "resource_exhaustion" => RecoveryType::Cleanup,
            "permission_denied" => RecoveryType::Reset,
            "database_error" => RecoveryType::Restore,
            "network_error" => RecoveryType::Reset,
            "security_error" => RecoveryType::Reconfigure,
            _ => {
                // Для других типов ошибок используем стратегию на основе серьезности
                if error_context.occurrence_count > 3 {
                    RecoveryType::Restore // Частые ошибки могут требовать восстановления
                } else {
                    RecoveryType::Restart // Стандартный перезапуск для большинства случаев
                }
            }
        }
    }

    /// Выполнить восстановление с перезапуском.
    async fn perform_restart_recovery(&self, component_name: &str) -> Result<()> {
        info!(
            "Performing restart recovery for component: {}",
            component_name
        );
        // В реальной реализации здесь будет логика перезапуска компонента
        tokio::time::sleep(Duration::from_millis(150)).await;
        Ok(())
    }

    /// Выполнить восстановление со сбросом.
    async fn perform_reset_recovery(&self, component_name: &str) -> Result<()> {
        info!(
            "Performing reset recovery for component: {}",
            component_name
        );
        // В реальной реализации здесь будет логика сброса компонента
        tokio::time::sleep(Duration::from_millis(200)).await;
        Ok(())
    }

    /// Выполнить восстановление с очисткой.
    async fn perform_cleanup_recovery(&self, component_name: &str) -> Result<()> {
        info!(
            "Performing cleanup recovery for component: {}",
            component_name
        );
        // В реальной реализации здесь будет логика очистки ресурсов
        tokio::time::sleep(Duration::from_millis(250)).await;
        Ok(())
    }

    /// Выполнить восстановление из резервной копии.
    async fn perform_restore_recovery(&self, component_name: &str) -> Result<()> {
        info!(
            "Performing restore recovery for component: {}",
            component_name
        );
        // В реальной реализации здесь будет логика восстановления из резервной копии
        tokio::time::sleep(Duration::from_millis(300)).await;
        Ok(())
    }

    /// Выполнить восстановление с переконфигурацией.
    async fn perform_reconfigure_recovery(&self, component_name: &str) -> Result<()> {
        info!(
            "Performing reconfigure recovery for component: {}",
            component_name
        );
        // В реальной реализации здесь будет логика переконфигурации
        tokio::time::sleep(Duration::from_millis(180)).await;
        Ok(())
    }

    /// Выполнить универсальное восстановление.
    async fn perform_generic_recovery(&self, component_name: &str) -> Result<()> {
        info!(
            "Performing generic recovery for component: {}",
            component_name
        );
        // Универсальная логика восстановления
        tokio::time::sleep(Duration::from_millis(120)).await;
        Ok(())
    }

    /// Улучшенный анализ паттернов ошибок с классификацией и трендами.
    pub async fn enhanced_error_pattern_analysis(
        &self,
    ) -> Result<Vec<EnhancedErrorPatternAnalysis>> {
        let state = self.health_state.read().await;
        let mut analyses = Vec::new();

        for (error_type, pattern) in &state.error_patterns {
            let frequency = if pattern.occurrence_count > 1 {
                let duration = pattern
                    .last_occurrence
                    .signed_duration_since(pattern.first_occurrence);
                if duration.num_minutes() > 0 {
                    pattern.occurrence_count as f32 / duration.num_minutes() as f32
                } else {
                    pattern.occurrence_count as f32
                }
            } else {
                0.0
            };

            let severity_level = match pattern.severity {
                HealthIssueSeverity::Critical => "Critical",
                HealthIssueSeverity::Error => "Error",
                HealthIssueSeverity::Warning => "Warning",
                _ => "Info",
            };

            // Классифицируем ошибку по частоте и серьезности
            let error_classification = self.classify_error_by_pattern(pattern);
            let trend_analysis = self.analyze_error_trend(pattern);

            analyses.push(EnhancedErrorPatternAnalysis {
                error_type: error_type.clone(),
                occurrence_count: pattern.occurrence_count,
                frequency_per_minute: frequency,
                affected_components: pattern.affected_components.clone(),
                severity: severity_level.to_string(),
                first_occurrence: pattern.first_occurrence,
                last_occurrence: pattern.last_occurrence,
                is_recurring: pattern.occurrence_count > 1,
                needs_attention: pattern.severity as usize >= HealthIssueSeverity::Warning as usize
                    && frequency > 0.1,
                error_classification,
                trend_analysis,
                recovery_priority: self.determine_recovery_priority(pattern),
            });
        }

        Ok(analyses)
    }

    /// Классифицировать ошибку по паттерну.
    fn classify_error_by_pattern(&self, pattern: &ErrorPattern) -> String {
        let frequency = if pattern.occurrence_count > 1 {
            let duration = pattern
                .last_occurrence
                .signed_duration_since(pattern.first_occurrence);
            if duration.num_minutes() > 0 {
                pattern.occurrence_count as f32 / duration.num_minutes() as f32
            } else {
                pattern.occurrence_count as f32
            }
        } else {
            0.0
        };

        match pattern.severity {
            HealthIssueSeverity::Critical => {
                if frequency > 0.5 {
                    "Critical Recurring Error".to_string()
                } else {
                    "Critical Isolated Error".to_string()
                }
            }
            HealthIssueSeverity::Error => {
                if frequency > 0.3 {
                    "Frequent Error Pattern".to_string()
                } else if pattern.occurrence_count > 3 {
                    "Persistent Error Pattern".to_string()
                } else {
                    "Isolated Error".to_string()
                }
            }
            HealthIssueSeverity::Warning => {
                if frequency > 0.5 {
                    "Frequent Warning Pattern".to_string()
                } else {
                    "Occasional Warning".to_string()
                }
            }
            _ => {
                if frequency > 1.0 {
                    "High Frequency Info Pattern".to_string()
                } else {
                    "Informational Pattern".to_string()
                }
            }
        }
    }

    /// Проанализировать тренд ошибки.
    fn analyze_error_trend(&self, pattern: &ErrorPattern) -> String {
        let duration = pattern
            .last_occurrence
            .signed_duration_since(pattern.first_occurrence);
        let hours_since_first = duration.num_hours();

        if hours_since_first < 1 {
            "Recent Rapid Occurrence".to_string()
        } else if hours_since_first < 24 {
            "Recent Pattern".to_string()
        } else if hours_since_first < 168 {
            "Ongoing Pattern".to_string()
        } else {
            "Long-term Pattern".to_string()
        }
    }

    /// Определить приоритет восстановления.
    fn determine_recovery_priority(&self, pattern: &ErrorPattern) -> String {
        let frequency = if pattern.occurrence_count > 1 {
            let duration = pattern
                .last_occurrence
                .signed_duration_since(pattern.first_occurrence);
            if duration.num_minutes() > 0 {
                pattern.occurrence_count as f32 / duration.num_minutes() as f32
            } else {
                pattern.occurrence_count as f32
            }
        } else {
            0.0
        };

        match pattern.severity {
            HealthIssueSeverity::Critical => "High Priority",
            HealthIssueSeverity::Error => {
                if frequency > 0.3 || pattern.occurrence_count > 5 {
                    "High Priority"
                } else {
                    "Medium Priority"
                }
            }
            HealthIssueSeverity::Warning => {
                if frequency > 0.5 || pattern.occurrence_count > 10 {
                    "Medium Priority"
                } else {
                    "Low Priority"
                }
            }
            _ => "Low Priority",
        }
        .to_string()
    }

    /// Улучшенное автоматическое восстановление с расширенным контекстом.
    pub async fn enhanced_auto_recovery(
        &self,
        component_name: &str,
        error_context: EnhancedErrorContext,
    ) -> Result<RecoveryAttempt> {
        let mut recovery_attempt = RecoveryAttempt {
            timestamp: Utc::now(),
            component: component_name.to_string(),
            recovery_type: RecoveryType::Restart,
            status: RecoveryStatus::InProgress,
            issue_description: error_context.error_message.clone(),
            recovery_details: Some("Enhanced auto-recovery initiated".to_string()),
        };

        // Получаем рекомендации по восстановлению
        let suggestions = self.get_recovery_suggestions(&error_context.error_type, component_name);

        // Обновляем контекст ошибки с рекомендациями
        let mut updated_context = error_context;
        updated_context.recovery_suggestions = Some(suggestions.clone());

        // Выполняем восстановление в зависимости от типа компонента
        let recovery_result = match component_name {
            "system_metrics" => self.recover_system_metrics().await,
            "process_monitoring" => self.recover_process_monitoring().await,
            "resource_usage" => self.recover_resource_usage().await,
            _ => self.generic_component_recovery(component_name).await,
        };

        // Обновляем статус попытки восстановления
        recovery_attempt.status = if recovery_result.is_ok() {
            RecoveryStatus::Success
        } else {
            RecoveryStatus::Failed
        };

        recovery_attempt.recovery_details = Some(format!(
            "Enhanced recovery: {}. Recommendations: {}",
            if recovery_result.is_ok() {
                "success"
            } else {
                "failed"
            },
            suggestions
        ));

        // Обновляем статистику восстановления
        let mut stats = self.recovery_stats.write().await;
        stats.total_recovery_attempts += 1;

        if recovery_result.is_ok() {
            stats.successful_recoveries += 1;
            stats.last_recovered_component = Some(component_name.to_string());
        } else {
            stats.failed_recoveries += 1;
        }

        stats.last_recovery_attempt_time = Some(Utc::now());
        stats.recovery_history.push(recovery_attempt.clone());

        Ok(recovery_attempt)
    }

    /// Анализ паттернов ошибок для выявления повторяющихся проблем.
    pub async fn analyze_error_patterns(&self) -> Result<Vec<ErrorPatternAnalysis>> {
        let state = self.health_state.read().await;
        let mut analyses = Vec::new();

        for (error_type, pattern) in &state.error_patterns {
            let frequency = if pattern.occurrence_count > 1 {
                let duration = pattern
                    .last_occurrence
                    .signed_duration_since(pattern.first_occurrence);
                if duration.num_minutes() > 0 {
                    pattern.occurrence_count as f32 / duration.num_minutes() as f32
                } else {
                    pattern.occurrence_count as f32
                }
            } else {
                0.0
            };

            let severity_level = match pattern.severity {
                HealthIssueSeverity::Critical => "Critical",
                HealthIssueSeverity::Error => "Error",
                HealthIssueSeverity::Warning => "Warning",
                _ => "Info",
            };

            analyses.push(ErrorPatternAnalysis {
                error_type: error_type.clone(),
                occurrence_count: pattern.occurrence_count,
                frequency_per_minute: frequency,
                affected_components: pattern.affected_components.clone(),
                severity: severity_level.to_string(),
                first_occurrence: pattern.first_occurrence,
                last_occurrence: pattern.last_occurrence,
                is_recurring: pattern.occurrence_count > 1,
                needs_attention: pattern.severity as usize >= HealthIssueSeverity::Warning as usize
                    && frequency > 0.1,
            });
        }

        Ok(analyses)
    }

    /// Добавить расширенную информацию об ошибке в проблему здоровья.
    pub async fn add_enhanced_health_issue(
        &self,
        component_name: &str,
        error_type: &str,
        error_message: &str,
        error_details: Option<String>,
        stack_trace: Option<String>,
        execution_context: Option<String>,
        system_context: Option<String>,
        severity: HealthIssueSeverity,
    ) -> Result<()> {
        // Создаем расширенный контекст ошибки
        let error_context = self.create_enhanced_error_context(
            component_name,
            error_type,
            error_message,
            error_details.clone(),
            stack_trace,
            execution_context,
            system_context,
        );

        // Создаем проблему здоровья с расширенным контекстом
        let health_issue = HealthIssue {
            issue_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            issue_type: match severity {
                HealthIssueSeverity::Critical => HealthIssueType::CriticalIssue,
                HealthIssueSeverity::Error => HealthIssueType::ComponentFailure,
                HealthIssueSeverity::Warning => HealthIssueType::PerformanceIssue,
                _ => HealthIssueType::Info,
            },
            severity,
            component: Some(component_name.to_string()),
            description: error_message.to_string(),
            error_details,
            enhanced_error_context: Some(error_context.clone()),
            status: HealthIssueStatus::Open,
            resolved_time: None,
        };

        // Обновляем паттерны ошибок
        self.update_error_patterns(error_type, component_name, severity, error_context)
            .await?;

        // Добавляем проблему в историю
        let mut state = self.health_state.write().await;

        if state.issue_history.len() >= state.config.max_issue_history {
            state.issue_history.remove(0);
        }

        state.issue_history.push(health_issue);

        // Обновляем статус компонента
        if let Some(component_status) = state.component_statuses.get_mut(component_name) {
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

            component_status.error_details = Some(error_message.to_string());
            component_status.last_check_time = Some(Utc::now());
        }

        Ok(())
    }
}

/// Структура для анализа паттернов ошибок.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorPatternAnalysis {
    /// Тип ошибки
    pub error_type: String,
    /// Количество возникновений
    pub occurrence_count: usize,
    /// Частота возникновения (в минуту)
    pub frequency_per_minute: f32,
    /// Затронутые компоненты
    pub affected_components: Vec<String>,
    /// Серьезность
    pub severity: String,
    /// Время первого возникновения
    pub first_occurrence: DateTime<Utc>,
    /// Время последнего возникновения
    pub last_occurrence: DateTime<Utc>,
    /// Является ли повторяющейся
    pub is_recurring: bool,
    /// Требует внимания
    pub needs_attention: bool,
}

/// Улучшенная структура для анализа паттернов ошибок с классификацией и трендами.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnhancedErrorPatternAnalysis {
    /// Тип ошибки
    pub error_type: String,
    /// Количество возникновений
    pub occurrence_count: usize,
    /// Частота возникновения (в минуту)
    pub frequency_per_minute: f32,
    /// Затронутые компоненты
    pub affected_components: Vec<String>,
    /// Серьезность
    pub severity: String,
    /// Время первого возникновения
    pub first_occurrence: DateTime<Utc>,
    /// Время последнего возникновения
    pub last_occurrence: DateTime<Utc>,
    /// Является ли повторяющейся
    pub is_recurring: bool,
    /// Требует внимания
    pub needs_attention: bool,
    /// Классификация ошибки
    pub error_classification: String,
    /// Анализ тренда
    pub trend_analysis: String,
    /// Приоритет восстановления
    pub recovery_priority: String,
}

/// Вспомогательная функция для создания HealthMonitor.
pub fn create_health_monitor(config: HealthMonitorConfig) -> HealthMonitorImpl {
    HealthMonitorImpl::new(config)
}

/// Вспомогательная функция для создания HealthMonitor с конфигурацией по умолчанию.
pub fn create_default_health_monitor() -> HealthMonitorImpl {
    HealthMonitorImpl::new_default()
}
