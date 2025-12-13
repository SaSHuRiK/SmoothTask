//! Модуль мониторинга здоровья демона SmoothTask.
//!
//! Этот модуль предоставляет комплексную систему мониторинга здоровья демона,
//! включая проверку состояния компонентов, диагностику проблем и уведомления
//! о критических состояниях.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
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
}

/// Реализация HealthMonitorTrait.
#[derive(Debug, Clone)]
pub struct HealthMonitorImpl {
    health_state: Arc<tokio::sync::RwLock<HealthMonitor>>,
    config: HealthMonitorConfig,
}

#[async_trait::async_trait]
impl HealthMonitorTrait for HealthMonitorImpl {
    async fn check_health(&self) -> Result<HealthMonitor> {
        let mut health_monitor = self.health_state.read().await.clone();
        
        // Обновляем время последней проверки
        health_monitor.last_check_time = Some(Utc::now());
        
        // Выполняем проверку компонентов
        health_monitor = self.check_components(health_monitor).await?;
        
        // Определяем общий статус здоровья
        health_monitor.overall_status = self.determine_overall_status(&health_monitor);
        
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
        
        if let Some(issue) = state.issue_history.iter_mut().find(|i| i.issue_id == issue_id) {
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
}

impl HealthMonitorImpl {
    /// Создать новый HealthMonitorImpl.
    pub fn new(config: HealthMonitorConfig) -> Self {
        Self {
            health_state: Arc::new(tokio::sync::RwLock::new(HealthMonitor::default())),
            config,
        }
    }
    
    /// Создать новый HealthMonitorImpl с конфигурацией по умолчанию.
    pub fn new_default() -> Self {
        Self::new(HealthMonitorConfig::default())
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
                Ok(mut entries) => {
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
                        let used_percent = 100.0 * (mem_total - mem_available) as f32 / mem_total as f32;
                        
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