//! Модуль мониторинга безопасности SmoothTask.
//!
//! Этот модуль предоставляет систему мониторинга безопасности для обнаружения
//! подозрительных процессов, аномальной активности и потенциальных угроз безопасности.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// Тип события безопасности.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityEventType {
    /// Подозрительный процесс
    #[serde(rename = "suspicious_process")]
    SuspiciousProcess,
    /// Необычная активность процесса
    #[serde(rename = "unusual_process_activity")]
    UnusualProcessActivity,
    /// Подозрительное сетевое соединение
    #[serde(rename = "suspicious_network_connection")]
    SuspiciousNetworkConnection,
    /// Аномальное использование ресурсов
    #[serde(rename = "anomalous_resource_usage")]
    AnomalousResourceUsage,
    /// Подозрительная активность файловой системы
    #[serde(rename = "suspicious_filesystem_activity")]
    SuspiciousFilesystemActivity,
    /// Потенциальная атака
    #[serde(rename = "potential_attack")]
    PotentialAttack,
    /// Неизвестный тип события
    #[serde(rename = "unknown")]
    Unknown,
}

impl Default for SecurityEventType {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for SecurityEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityEventType::SuspiciousProcess => write!(f, "suspicious_process"),
            SecurityEventType::UnusualProcessActivity => write!(f, "unusual_process_activity"),
            SecurityEventType::SuspiciousNetworkConnection => {
                write!(f, "suspicious_network_connection")
            }
            SecurityEventType::AnomalousResourceUsage => {
                write!(f, "anomalous_resource_usage")
            }
            SecurityEventType::SuspiciousFilesystemActivity => {
                write!(f, "suspicious_filesystem_activity")
            }
            SecurityEventType::PotentialAttack => write!(f, "potential_attack"),
            SecurityEventType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Уровень серьезности события безопасности.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityEventSeverity {
    /// Информационное событие
    #[serde(rename = "info")]
    Info,
    /// Низкий уровень угрозы
    #[serde(rename = "low")]
    Low,
    /// Средний уровень угрозы
    #[serde(rename = "medium")]
    Medium,
    /// Высокий уровень угрозы
    #[serde(rename = "high")]
    High,
    /// Критический уровень угрозы
    #[serde(rename = "critical")]
    Critical,
}

impl Default for SecurityEventSeverity {
    fn default() -> Self {
        Self::Info
    }
}

impl std::fmt::Display for SecurityEventSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityEventSeverity::Info => write!(f, "info"),
            SecurityEventSeverity::Low => write!(f, "low"),
            SecurityEventSeverity::Medium => write!(f, "medium"),
            SecurityEventSeverity::High => write!(f, "high"),
            SecurityEventSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// Статус события безопасности.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityEventStatus {
    /// Новое событие
    #[serde(rename = "new")]
    New,
    /// В процессе анализа
    #[serde(rename = "analyzing")]
    Analyzing,
    /// Проанализировано
    #[serde(rename = "analyzed")]
    Analyzed,
    /// Ложное срабатывание
    #[serde(rename = "false_positive")]
    FalsePositive,
    /// Подтвержденная угроза
    #[serde(rename = "confirmed_threat")]
    ConfirmedThreat,
    /// Игнорируется
    #[serde(rename = "ignored")]
    Ignored,
}

impl Default for SecurityEventStatus {
    fn default() -> Self {
        Self::New
    }
}

/// Информация о событии безопасности.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Уникальный идентификатор события
    pub event_id: String,
    /// Время возникновения события
    pub timestamp: DateTime<Utc>,
    /// Тип события
    pub event_type: SecurityEventType,
    /// Серьезность события
    pub severity: SecurityEventSeverity,
    /// Статус события
    pub status: SecurityEventStatus,
    /// Имя процесса (если применимо)
    pub process_name: Option<String>,
    /// Идентификатор процесса (если применимо)
    pub process_id: Option<i32>,
    /// Описание события
    pub description: String,
    /// Детали события
    pub details: Option<String>,
    /// Рекомендации по действиям
    pub recommendations: Option<String>,
    /// Время разрешения события (если решено)
    pub resolved_time: Option<DateTime<Utc>>,
}

impl Default for SecurityEvent {
    fn default() -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type: SecurityEventType::Unknown,
            severity: SecurityEventSeverity::Info,
            status: SecurityEventStatus::New,
            process_name: None,
            process_id: None,
            description: String::new(),
            details: None,
            recommendations: None,
            resolved_time: None,
        }
    }
}

/// Конфигурация мониторинга безопасности.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityMonitorConfig {
    /// Включить мониторинг безопасности
    pub enabled: bool,
    /// Интервал проверки безопасности
    pub check_interval: Duration,
    /// Максимальное количество хранимых событий
    pub max_event_history: usize,
    /// Пороги для обнаружения аномалий
    pub anomaly_thresholds: AnomalyThresholds,
    /// Список доверенных процессов
    pub trusted_processes: Vec<String>,
    /// Список подозрительных процессов
    pub suspicious_processes: Vec<String>,
    /// Настройки уведомлений
    pub notification_settings: SecurityNotificationSettings,
}

impl Default for SecurityMonitorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval: Duration::from_secs(300), // 5 минут
            max_event_history: 1000,
            anomaly_thresholds: AnomalyThresholds::default(),
            trusted_processes: vec![
                "smoothtaskd".to_string(),
                "systemd".to_string(),
                "init".to_string(),
                "kthreadd".to_string(),
            ],
            suspicious_processes: vec![
                "bitcoin".to_string(),
                "minerd".to_string(),
                "cryptonight".to_string(),
                "xmrig".to_string(),
                "masscan".to_string(),
                "nmap".to_string(),
                "hydra".to_string(),
                "metasploit".to_string(),
                "john".to_string(),
                "hashcat".to_string(),
            ],
            notification_settings: SecurityNotificationSettings::default(),
        }
    }
}

/// Пороги для обнаружения аномалий.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnomalyThresholds {
    /// Максимальное количество новых процессов в минуту
    pub max_new_processes_per_minute: usize,
    /// Максимальное использование CPU для одного процесса (в процентах)
    pub max_cpu_usage_percent: f32,
    /// Максимальное использование памяти для одного процесса (в процентах)
    pub max_memory_usage_percent: f32,
    /// Максимальное количество сетевых соединений для одного процесса
    pub max_network_connections: usize,
    /// Максимальное количество открытых файлов для одного процесса
    pub max_open_files: usize,
    /// Максимальное количество потоков для одного процесса
    pub max_threads: usize,
}

impl Default for AnomalyThresholds {
    fn default() -> Self {
        Self {
            max_new_processes_per_minute: 50,
            max_cpu_usage_percent: 90.0,
            max_memory_usage_percent: 80.0,
            max_network_connections: 100,
            max_open_files: 1000,
            max_threads: 500,
        }
    }
}

/// Настройки уведомлений о событиях безопасности.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityNotificationSettings {
    /// Включить уведомления о критических событиях
    pub enable_critical_notifications: bool,
    /// Включить уведомления о событиях высокого уровня
    pub enable_high_notifications: bool,
    /// Включить уведомления о событиях среднего уровня
    pub enable_medium_notifications: bool,
    /// Максимальная частота уведомлений (в секундах)
    pub max_notification_frequency_seconds: u64,
}

impl Default for SecurityNotificationSettings {
    fn default() -> Self {
        Self {
            enable_critical_notifications: true,
            enable_high_notifications: true,
            enable_medium_notifications: false,
            max_notification_frequency_seconds: 300, // 5 минут
        }
    }
}

/// Основная структура для мониторинга безопасности.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SecurityMonitor {
    /// Время последней проверки безопасности
    pub last_check_time: Option<DateTime<Utc>>,
    /// Общий статус безопасности
    pub overall_status: SecurityStatus,
    /// История событий безопасности
    pub event_history: Vec<SecurityEvent>,
    /// Конфигурация мониторинга безопасности
    pub config: SecurityMonitorConfig,
    /// Текущий балл безопасности (0-100)
    pub security_score: f32,
    /// История баллов безопасности для анализа трендов
    pub security_score_history: Vec<SecurityScoreEntry>,
}

/// Статус безопасности системы.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecurityStatus {
    /// Система безопасна
    #[serde(rename = "secure")]
    Secure,
    /// Есть предупреждения о безопасности
    #[serde(rename = "warning")]
    Warning,
    /// Потенциальные угрозы обнаружены
    #[serde(rename = "potential_threat")]
    PotentialThreat,
    /// Критические угрозы обнаружены
    #[serde(rename = "critical_threat")]
    CriticalThreat,
    /// Состояние безопасности неизвестно
    #[serde(rename = "unknown")]
    Unknown,
}

impl Default for SecurityStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Запись балла безопасности с временной меткой.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityScoreEntry {
    /// Время записи балла
    pub timestamp: DateTime<Utc>,
    /// Балл безопасности (0-100)
    pub score: f32,
    /// Состояние безопасности в это время
    pub status: SecurityStatus,
}

impl Default for SecurityScoreEntry {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            score: 100.0,
            status: SecurityStatus::Secure,
        }
    }
}

/// Интерфейс для мониторинга безопасности.
#[async_trait::async_trait]
pub trait SecurityMonitorTrait: Send + Sync {
    /// Выполнить проверку безопасности.
    async fn check_security(&self) -> Result<SecurityMonitor>;

    /// Обновить состояние безопасности.
    async fn update_security_status(&self, security_monitor: SecurityMonitor) -> Result<()>;

    /// Получить текущее состояние безопасности.
    async fn get_security_status(&self) -> Result<SecurityMonitor>;

    /// Добавить событие безопасности.
    async fn add_security_event(&self, event: SecurityEvent) -> Result<()>;

    /// Разрешить событие безопасности.
    async fn resolve_security_event(&self, event_id: &str) -> Result<()>;

    /// Очистить историю событий.
    async fn clear_event_history(&self) -> Result<()>;

    /// Пометить событие как ложное срабатывание.
    async fn mark_event_as_false_positive(&self, event_id: &str) -> Result<()>;

    /// Получить статистику событий безопасности.
    async fn get_security_stats(&self) -> Result<SecurityStats>;

    /// Очистить статистику событий безопасности.
    async fn clear_security_stats(&self) -> Result<()>;
}

/// Статистика событий безопасности.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityStats {
    /// Общее количество событий
    pub total_events: usize,
    /// Количество критических событий
    pub critical_events: usize,
    /// Количество событий высокого уровня
    pub high_events: usize,
    /// Количество событий среднего уровня
    pub medium_events: usize,
    /// Количество событий низкого уровня
    pub low_events: usize,
    /// Количество подтвержденных угроз
    pub confirmed_threats: usize,
    /// Количество ложных срабатываний
    pub false_positives: usize,
    /// Время последнего события
    pub last_event_time: Option<DateTime<Utc>>,
}

impl Default for SecurityStats {
    fn default() -> Self {
        Self {
            total_events: 0,
            critical_events: 0,
            high_events: 0,
            medium_events: 0,
            low_events: 0,
            confirmed_threats: 0,
            false_positives: 0,
            last_event_time: None,
        }
    }
}

/// Реализация SecurityMonitorTrait.
#[derive(Clone)]
pub struct SecurityMonitorImpl {
    security_state: Arc<tokio::sync::RwLock<SecurityMonitor>>,
    config: SecurityMonitorConfig,
    stats: Arc<tokio::sync::RwLock<SecurityStats>>,
    notifier: Option<Arc<dyn crate::notifications::Notifier>>,
}

#[async_trait::async_trait]
impl SecurityMonitorTrait for SecurityMonitorImpl {
    async fn check_security(&self) -> Result<SecurityMonitor> {
        let mut security_monitor = self.security_state.read().await.clone();

        // Обновляем время последней проверки
        security_monitor.last_check_time = Some(Utc::now());

        // Выполняем проверку безопасности
        security_monitor = self.perform_security_checks(security_monitor).await?;

        // Определяем общий статус безопасности
        security_monitor.overall_status = self.determine_overall_status(&security_monitor);

        // Рассчитываем балл безопасности
        self.update_security_score_history(&mut security_monitor);

        Ok(security_monitor)
    }

    async fn update_security_status(&self, security_monitor: SecurityMonitor) -> Result<()> {
        let mut state = self.security_state.write().await;
        *state = security_monitor;
        Ok(())
    }

    async fn get_security_status(&self) -> Result<SecurityMonitor> {
        Ok(self.security_state.read().await.clone())
    }

    async fn add_security_event(&self, event: SecurityEvent) -> Result<()> {
        let mut state = self.security_state.write().await;

        // Проверяем максимальное количество событий в истории
        if state.event_history.len() >= state.config.max_event_history {
            state.event_history.remove(0); // Удаляем самое старое событие
        }

        // Проверяем, нужно ли отправлять уведомление для этого события
        let should_notify = self.should_send_notification_for_event(&event);

        state.event_history.push(event.clone());

        // Обновляем статистику
        let mut stats = self.stats.write().await;
        stats.total_events += 1;
        stats.last_event_time = Some(Utc::now());

        // Отправляем уведомление, если нужно
        if should_notify {
            self.send_security_notification(&event).await?;
        }

        Ok(())
    }

    async fn resolve_security_event(&self, event_id: &str) -> Result<()> {
        let mut state = self.security_state.write().await;

        if let Some(event) = state
            .event_history
            .iter_mut()
            .find(|e| e.event_id == event_id)
        {
            event.status = SecurityEventStatus::Analyzed;
            event.resolved_time = Some(Utc::now());
        }

        Ok(())
    }

    async fn clear_event_history(&self) -> Result<()> {
        let mut state = self.security_state.write().await;
        state.event_history.clear();
        Ok(())
    }

    async fn mark_event_as_false_positive(&self, event_id: &str) -> Result<()> {
        let mut state = self.security_state.write().await;

        if let Some(event) = state
            .event_history
            .iter_mut()
            .find(|e| e.event_id == event_id)
        {
            event.status = SecurityEventStatus::FalsePositive;

            // Обновляем статистику
            let mut stats = self.stats.write().await;
            stats.false_positives += 1;
        }

        Ok(())
    }

    async fn get_security_stats(&self) -> Result<SecurityStats> {
        Ok(self.stats.read().await.clone())
    }

    async fn clear_security_stats(&self) -> Result<()> {
        let mut stats = self.stats.write().await;
        *stats = SecurityStats::default();
        Ok(())
    }
}

impl SecurityMonitorImpl {
    /// Создать новый SecurityMonitorImpl.
    pub fn new(config: SecurityMonitorConfig) -> Self {
        Self {
            security_state: Arc::new(tokio::sync::RwLock::new(SecurityMonitor::default())),
            config,
            stats: Arc::new(tokio::sync::RwLock::new(SecurityStats::default())),
            notifier: None,
        }
    }

    /// Создать новый SecurityMonitorImpl с конфигурацией по умолчанию.
    pub fn new_default() -> Self {
        Self::new(SecurityMonitorConfig::default())
    }

    /// Создать новый SecurityMonitorImpl с уведомителем.
    pub fn new_with_notifier(
        config: SecurityMonitorConfig,
        notifier: Arc<dyn crate::notifications::Notifier>,
    ) -> Self {
        Self {
            security_state: Arc::new(tokio::sync::RwLock::new(SecurityMonitor::default())),
            config,
            stats: Arc::new(tokio::sync::RwLock::new(SecurityStats::default())),
            notifier: Some(notifier),
        }
    }

    /// Выполнить проверку безопасности.
    async fn perform_security_checks(
        &self,
        mut security_monitor: SecurityMonitor,
    ) -> Result<SecurityMonitor> {
        // Проверяем подозрительные процессы
        self.check_suspicious_processes(&mut security_monitor)
            .await?;

        // Проверяем подозрительные паттерны поведения процессов
        self.check_suspicious_behavior(&mut security_monitor)
            .await?;

        // Проверяем аномальное использование ресурсов
        self.check_anomalous_resource_usage(&mut security_monitor)
            .await?;

        // Проверяем подозрительные сетевые соединения
        self.check_suspicious_network_connections(&mut security_monitor)
            .await?;

        // Проверяем подозрительную активность файловой системы
        self.check_suspicious_filesystem_activity(&mut security_monitor)
            .await?;

        Ok(security_monitor)
    }

    /// Проверка подозрительных процессов.
    async fn check_suspicious_processes(
        &self,
        _security_monitor: &mut SecurityMonitor,
    ) -> Result<()> {
        // Получаем список всех процессов
        let processes = self.get_all_processes().await?;

        for process in processes {
            // Проверяем, является ли процесс подозрительным
            if self.is_suspicious_process(&process.name) {
                let event = SecurityEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                    event_type: SecurityEventType::SuspiciousProcess,
                    severity: SecurityEventSeverity::High,
                    status: SecurityEventStatus::New,
                    process_name: Some(process.name.clone()),
                    process_id: Some(process.pid),
                    description: format!(
                        "Suspicious process detected: {} (PID: {})",
                        process.name, process.pid
                    ),
                    details: Some(format!(
                        "Process path: {}",
                        process.exe_path.unwrap_or_default()
                    )),
                    recommendations: Some(
                        "Investigate this process and consider terminating it if it's malicious"
                            .to_string(),
                    ),
                    resolved_time: None,
                };

                self.add_security_event(event).await?;
            }
        }

        Ok(())
    }

    /// Проверка аномального использования ресурсов.
    async fn check_anomalous_resource_usage(
        &self,
        _security_monitor: &mut SecurityMonitor,
    ) -> Result<()> {
        // Получаем информацию о процессах с высоким использованием ресурсов
        let high_resource_processes = self.get_high_resource_processes().await?;

        // Анализируем процессы на предмет аномального поведения
        for process in high_resource_processes {
            // Проверяем, является ли процесс известным системным процессом
            let is_system_process = self.is_system_process(&process.name);

            // Определяем уровень серьезности на основе типа процесса и использования ресурсов
            let (severity, description, recommendations) = if is_system_process {
                // Системные процессы с высоким использованием ресурсов
                if process.cpu_usage > 95.0 || process.memory_usage > 90.0 {
                    (SecurityEventSeverity::High,
                     format!("Critical resource usage by system process: {} (CPU: {:.1}%, Memory: {:.1}%)", 
                         process.name, process.cpu_usage, process.memory_usage),
                     "Investigate immediately - this may indicate a system issue or resource exhaustion attack".to_string())
                } else {
                    (SecurityEventSeverity::Medium,
                     format!("High resource usage by system process: {} (CPU: {:.1}%, Memory: {:.1}%)", 
                         process.name, process.cpu_usage, process.memory_usage),
                     "Monitor this system process - high resource usage may be legitimate but should be investigated".to_string())
                }
            } else {
                // Пользовательские процессы с высоким использованием ресурсов
                if process.cpu_usage > 95.0 || process.memory_usage > 90.0 {
                    (SecurityEventSeverity::High,
                     format!("Critical resource usage by user process: {} (CPU: {:.1}%, Memory: {:.1}%)", 
                         process.name, process.cpu_usage, process.memory_usage),
                     "Investigate immediately - this may indicate malicious activity or resource abuse".to_string())
                } else {
                    (SecurityEventSeverity::Medium,
                     format!("High resource usage by user process: {} (CPU: {:.1}%, Memory: {:.1}%)", 
                         process.name, process.cpu_usage, process.memory_usage),
                     "Monitor this user process - high resource usage may be legitimate but should be investigated".to_string())
                }
            };

            // Анализируем поведение процесса для более точной классификации
            let behavior = self.analyze_process_behavior(process.pid).await?;
            let anomaly_patterns = self.detect_resource_anomaly_patterns(&behavior).await?;

            // Если обнаружены паттерны аномалий, повышаем серьезность
            let final_severity = if !anomaly_patterns.is_empty() {
                match severity {
                    SecurityEventSeverity::High => SecurityEventSeverity::Critical,
                    SecurityEventSeverity::Medium => SecurityEventSeverity::High,
                    _ => SecurityEventSeverity::High,
                }
            } else {
                severity
            };

            // Добавляем информацию о паттернах в детали
            let mut details = format!("Process path: {}", process.exe_path.unwrap_or_default());
            if !anomaly_patterns.is_empty() {
                details.push_str("\nAnomaly patterns detected:");
                for pattern in &anomaly_patterns {
                    details.push_str(&format!(
                        "\n- {}: {} (threshold: {}, current: {})",
                        pattern.pattern_type,
                        pattern.description,
                        pattern.threshold,
                        pattern.current_value
                    ));
                }
            }

            let event = SecurityEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                timestamp: Utc::now(),
                event_type: SecurityEventType::AnomalousResourceUsage,
                severity: final_severity,
                status: SecurityEventStatus::New,
                process_name: Some(process.name.clone()),
                process_id: Some(process.pid),
                description,
                details: Some(details),
                recommendations: Some(recommendations),
                resolved_time: None,
            };

            self.add_security_event(event).await?;
        }

        Ok(())
    }

    /// Проверка, является ли процесс системным.
    fn is_system_process(&self, process_name: &str) -> bool {
        let system_processes = [
            "systemd",
            "init",
            "kthreadd",
            "ksoftirqd",
            "kworker",
            "rcu_sched",
            "rcu_bh",
            "migration",
            "watchdog",
            "idle",
            "smoothtaskd",
            "dbus",
            "polkitd",
            "rsyslogd",
            "cron",
            "sshd",
            "networkd",
            "udevd",
            "thermald",
            "bluetoothd",
        ];

        system_processes.contains(&process_name)
    }

    /// Обнаружение паттернов аномалий в использовании ресурсов.
    async fn detect_resource_anomaly_patterns(
        &self,
        behavior: &ProcessBehavior,
    ) -> Result<Vec<SuspiciousBehaviorPattern>> {
        let mut patterns = Vec::new();

        // Паттерн 1: Аномально высокое количество дочерних процессов
        if behavior.child_count > 20 {
            patterns.push(SuspiciousBehaviorPattern {
                pattern_type: "anomalous_child_process_count".to_string(),
                description: "Process has anomalously high number of child processes".to_string(),
                severity: SecurityEventSeverity::High,
                threshold: 20.0,
                current_value: behavior.child_count as f32,
            });
        }

        // Паттерн 2: Аномально высокое количество потоков
        if behavior.thread_count > 200 {
            patterns.push(SuspiciousBehaviorPattern {
                pattern_type: "anomalous_thread_count".to_string(),
                description: "Process has anomalously high number of threads".to_string(),
                severity: SecurityEventSeverity::High,
                threshold: 200.0,
                current_value: behavior.thread_count as f32,
            });
        }

        // Паттерн 3: Аномально высокое количество открытых файлов
        if behavior.open_files_count > 200 {
            patterns.push(SuspiciousBehaviorPattern {
                pattern_type: "anomalous_open_files_count".to_string(),
                description: "Process has anomalously high number of open files".to_string(),
                severity: SecurityEventSeverity::Medium,
                threshold: 200.0,
                current_value: behavior.open_files_count as f32,
            });
        }

        // Паттерн 4: Аномально высокое использование CPU для типа процесса
        if behavior.cpu_usage > 95.0
            && !behavior.device_name.to_lowercase().contains("render")
            && !behavior.device_name.to_lowercase().contains("gpu")
        {
            patterns.push(SuspiciousBehaviorPattern {
                pattern_type: "anomalous_cpu_usage".to_string(),
                description: "Process has anomalously high CPU usage for its type".to_string(),
                severity: SecurityEventSeverity::High,
                threshold: 95.0,
                current_value: behavior.cpu_usage,
            });
        }

        // Паттерн 5: Аномально высокое использование памяти для типа процесса
        if behavior.memory_usage > 85.0
            && !behavior.device_name.to_lowercase().contains("database")
            && !behavior.device_name.to_lowercase().contains("java")
        {
            patterns.push(SuspiciousBehaviorPattern {
                pattern_type: "anomalous_memory_usage".to_string(),
                description: "Process has anomalously high memory usage for its type".to_string(),
                severity: SecurityEventSeverity::High,
                threshold: 85.0,
                current_value: behavior.memory_usage,
            });
        }

        // Паттерн 6: Аномально высокая частота создания дочерних процессов
        if behavior.child_creation_rate > 5.0 {
            // более 5 процессов в минуту
            patterns.push(SuspiciousBehaviorPattern {
                pattern_type: "anomalous_child_creation_rate".to_string(),
                description: "Process has anomalously high child process creation rate".to_string(),
                severity: SecurityEventSeverity::Critical,
                threshold: 5.0,
                current_value: behavior.child_creation_rate,
            });
        }

        Ok(patterns)
    }

    /// Проверка подозрительных сетевых соединений.
    async fn check_suspicious_network_connections(
        &self,
        _security_monitor: &mut SecurityMonitor,
    ) -> Result<()> {
        // Получаем информацию о сетевых соединениях
        let network_connections = self.get_network_connections().await?;

        for connection in network_connections {
            // Проверяем подозрительные соединения (например, с известными вредоносными IP)
            if self.is_suspicious_connection(&connection) {
                let event = SecurityEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                    event_type: SecurityEventType::SuspiciousNetworkConnection,
                    severity: SecurityEventSeverity::High,
                    status: SecurityEventStatus::New,
                    process_name: connection.process_name,
                    process_id: connection.process_id,
                    description: format!("Suspicious network connection detected: {}:{}", 
                        connection.remote_address, connection.remote_port),
                    details: Some(format!("Local address: {}:{}, Protocol: {}", 
                        connection.local_address, connection.local_port, connection.protocol)),
                    recommendations: Some("Investigate this network connection and consider blocking it if it's malicious".to_string()),
                    resolved_time: None,
                };

                self.add_security_event(event).await?;
            }
        }

        Ok(())
    }

    /// Проверка, нужно ли отправлять уведомление для этого события.
    fn should_send_notification_for_event(&self, event: &SecurityEvent) -> bool {
        // Проверяем, включены ли уведомления в конфигурации
        match event.severity {
            SecurityEventSeverity::Critical => {
                self.config
                    .notification_settings
                    .enable_critical_notifications
            }
            SecurityEventSeverity::High => {
                self.config.notification_settings.enable_high_notifications
            }
            SecurityEventSeverity::Medium => {
                self.config
                    .notification_settings
                    .enable_medium_notifications
            }
            SecurityEventSeverity::Low => false, // Не отправляем уведомления для низкого уровня
            SecurityEventSeverity::Info => false, // Не отправляем уведомления для информационного уровня
        }
    }

    /// Отправить уведомление о событии безопасности.
    async fn send_security_notification(&self, event: &SecurityEvent) -> Result<()> {
        if let Some(notifier) = &self.notifier {
            // Преобразуем уровень серьезности события безопасности в тип уведомления
            let notification_type = match event.severity {
                SecurityEventSeverity::Critical => crate::notifications::NotificationType::Critical,
                SecurityEventSeverity::High => crate::notifications::NotificationType::Critical,
                SecurityEventSeverity::Medium => crate::notifications::NotificationType::Warning,
                SecurityEventSeverity::Low => crate::notifications::NotificationType::Info,
                SecurityEventSeverity::Info => crate::notifications::NotificationType::Info,
            };

            // Создаем уведомление
            let notification = crate::notifications::Notification::new(
                notification_type,
                format!("Security Event: {}", event.event_type),
                event.description.clone(),
            )
            .with_details(event.details.clone().unwrap_or_default());

            // Отправляем уведомление
            notifier.send_notification(&notification).await?;

            tracing::info!("Sent security notification for event: {}", event.event_id);
        }

        Ok(())
    }

    /// Проверка подозрительной активности файловой системы.
    async fn check_suspicious_filesystem_activity(
        &self,
        _security_monitor: &mut SecurityMonitor,
    ) -> Result<()> {
        // Получаем информацию о недавней активности файловой системы
        let filesystem_activity = self.get_filesystem_activity().await?;

        for activity in filesystem_activity {
            // Проверяем подозрительную активность (например, доступ к системным файлам)
            if self.is_suspicious_filesystem_activity(&activity) {
                let event = SecurityEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                    event_type: SecurityEventType::SuspiciousFilesystemActivity,
                    severity: SecurityEventSeverity::Medium,
                    status: SecurityEventStatus::New,
                    process_name: activity.process_name.clone(),
                    process_id: activity.process_id,
                    description: format!(
                        "Suspicious filesystem activity detected: {}",
                        activity.path
                    ),
                    details: Some(format!(
                        "Operation: {}, Process: {}",
                        activity.operation,
                        activity.process_name.unwrap_or_default()
                    )),
                    recommendations: Some(
                        "Investigate this filesystem activity and monitor the process".to_string(),
                    ),
                    resolved_time: None,
                };

                self.add_security_event(event).await?;
            }
        }

        Ok(())
    }

    /// Определить общий статус безопасности.
    fn determine_overall_status(&self, security_monitor: &SecurityMonitor) -> SecurityStatus {
        let mut has_critical = false;
        let mut has_high = false;
        let mut has_medium = false;

        for event in &security_monitor.event_history {
            if event.status == SecurityEventStatus::New
                || event.status == SecurityEventStatus::Analyzing
            {
                match event.severity {
                    SecurityEventSeverity::Critical => has_critical = true,
                    SecurityEventSeverity::High => has_high = true,
                    SecurityEventSeverity::Medium => has_medium = true,
                    _ => {}
                }
            }
        }

        if has_critical {
            SecurityStatus::CriticalThreat
        } else if has_high {
            SecurityStatus::PotentialThreat
        } else if has_medium {
            SecurityStatus::Warning
        } else {
            SecurityStatus::Secure
        }
    }

    /// Рассчитать балл безопасности.
    fn calculate_security_score(&self, security_monitor: &SecurityMonitor) -> f32 {
        // Начинаем с максимального балла
        let mut score = 100.0;

        // Учитываем неразрешенные события
        let unresolved_events = security_monitor
            .event_history
            .iter()
            .filter(|event| {
                event.status == SecurityEventStatus::New
                    || event.status == SecurityEventStatus::Analyzing
            })
            .count();

        // Каждое неразрешенное событие снижает балл
        for event in &security_monitor.event_history {
            if event.status == SecurityEventStatus::New
                || event.status == SecurityEventStatus::Analyzing
            {
                match event.severity {
                    SecurityEventSeverity::Critical => score -= 20.0,
                    SecurityEventSeverity::High => score -= 10.0,
                    SecurityEventSeverity::Medium => score -= 5.0,
                    SecurityEventSeverity::Low => score -= 2.0,
                    SecurityEventSeverity::Info => score -= 1.0,
                }
            }
        }

        // Учитываем количество событий
        score -= unresolved_events as f32 * 0.5;

        // Ограничиваем балл в диапазоне 0-100
        score = score.clamp(0.0, 100.0);

        score
    }

    /// Обновить историю баллов безопасности.
    fn update_security_score_history(&self, security_monitor: &mut SecurityMonitor) {
        let score = self.calculate_security_score(security_monitor);
        security_monitor.security_score = score;

        let entry = SecurityScoreEntry {
            timestamp: Utc::now(),
            score,
            status: security_monitor.overall_status,
        };

        security_monitor.security_score_history.push(entry);

        // Ограничиваем историю (например, 100 записей)
        if security_monitor.security_score_history.len() > 100 {
            security_monitor.security_score_history.remove(0);
        }
    }

    /// Получить все процессы.
    async fn get_all_processes(&self) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();

        // Чтение информации о процессах из /proc
        let proc_dir = Path::new("/proc");
        if !proc_dir.exists() {
            return Ok(processes);
        }

        for entry in proc_dir.read_dir()? {
            let entry = entry?;
            let pid_str = entry.file_name().to_string_lossy().to_string();

            if let Ok(pid) = pid_str.parse::<i32>() {
                let process_info = self.get_process_info(pid).await?;
                if let Some(info) = process_info {
                    processes.push(info);
                }
            }
        }

        Ok(processes)
    }

    /// Получить информацию о процессе.
    async fn get_process_info(&self, pid: i32) -> Result<Option<ProcessInfo>> {
        let proc_path = Path::new("/proc").join(pid.to_string());
        if !proc_path.exists() {
            return Ok(None);
        }

        let mut process_info = ProcessInfo {
            pid,
            name: String::new(),
            exe_path: None,
            cpu_usage: 0.0,
            memory_usage: 0.0,
        };

        // Чтение статуса процесса
        let status_path = proc_path.join("status");
        if let Ok(status_content) = std::fs::read_to_string(&status_path) {
            for line in status_content.lines() {
                if line.starts_with("Name:") {
                    if let Some(name) = line.split(':').nth(1) {
                        process_info.name = name.trim().to_string();
                    }
                }
            }
        }

        // Чтение пути к исполняемому файлу
        let exe_path = proc_path.join("exe");
        if let Ok(exe_link) = std::fs::read_link(&exe_path) {
            process_info.exe_path = Some(exe_link.to_string_lossy().to_string());
        }

        Ok(Some(process_info))
    }

    /// Получить процессы с высоким использованием ресурсов.
    async fn get_high_resource_processes(&self) -> Result<Vec<ProcessInfo>> {
        let high_resource_processes = Vec::new();

        // В реальной реализации здесь будет анализ использования ресурсов
        // Для примера возвращаем пустой вектор

        Ok(high_resource_processes)
    }

    /// Получить сетевые соединения.
    async fn get_network_connections(&self) -> Result<Vec<NetworkConnection>> {
        let connections = Vec::new();

        // В реальной реализации здесь будет анализ сетевых соединений
        // Для примера возвращаем пустой вектор

        Ok(connections)
    }

    /// Получить активность файловой системы.
    async fn get_filesystem_activity(&self) -> Result<Vec<FilesystemActivity>> {
        let activities = Vec::new();

        // В реальной реализации здесь будет анализ активности файловой системы
        // Для примера возвращаем пустой вектор

        Ok(activities)
    }

    /// Проверка, является ли процесс подозрительным.
    fn is_suspicious_process(&self, process_name: &str) -> bool {
        self.config
            .suspicious_processes
            .contains(&process_name.to_lowercase())
    }

    /// Проверка, является ли сетевое соединение подозрительным.
    fn is_suspicious_connection(&self, _connection: &NetworkConnection) -> bool {
        // В реальной реализации здесь будет анализ соединений
        false
    }

    /// Проверка, является ли активность файловой системы подозрительной.
    fn is_suspicious_filesystem_activity(&self, _activity: &FilesystemActivity) -> bool {
        // В реальной реализации здесь будет анализ активности
        false
    }

    /// Анализ поведения процесса.
    async fn analyze_process_behavior(&self, pid: i32) -> Result<ProcessBehavior> {
        let mut behavior = ProcessBehavior {
            pid,
            child_count: 0,
            thread_count: 0,
            open_files_count: 0,
            network_connections_count: 0,
            start_time: None,
            parent_pid: None,
            parent_name: None,
            cpu_usage: 0.0,
            memory_usage: 0.0,
            device_name: String::new(),
            child_creation_rate: 0.0,
        };

        // Получаем информацию о процессе
        let proc_path = Path::new("/proc").join(pid.to_string());
        if !proc_path.exists() {
            return Ok(behavior);
        }

        // Чтение статуса процесса
        let status_path = proc_path.join("status");
        if let Ok(status_content) = std::fs::read_to_string(&status_path) {
            for line in status_content.lines() {
                if line.starts_with("Threads:") {
                    if let Some(thread_count) = line.split(':').nth(1) {
                        behavior.thread_count = thread_count.trim().parse().unwrap_or(0);
                    }
                }
            }
        }

        // Чтение информации о родительском процессе
        behavior.parent_pid = self.get_parent_pid(pid).await?;
        if let Some(parent_pid) = behavior.parent_pid {
            behavior.parent_name = self.get_process_name(parent_pid).await?;
        }

        // Чтение времени создания процесса
        behavior.start_time = self.get_process_start_time(pid).await?;

        // Подсчет дочерних процессов
        behavior.child_count = self.count_child_processes(pid).await?;

        // Подсчет открытых файлов
        behavior.open_files_count = self.count_open_files(pid).await?;

        // Подсчет сетевых соединений
        behavior.network_connections_count = self.count_network_connections(pid).await?;

        // Получение использования ресурсов
        let process_info = self.get_process_info(pid).await?;
        if let Some(info) = process_info {
            behavior.cpu_usage = info.cpu_usage;
            behavior.memory_usage = info.memory_usage;
            // Устанавливаем device_name на основе имени процесса
            behavior.device_name = info.name.clone();
        }

        Ok(behavior)
    }

    /// Получение идентификатора родительского процесса.
    async fn get_parent_pid(&self, pid: i32) -> Result<Option<i32>> {
        let proc_path = Path::new("/proc").join(pid.to_string());
        let status_path = proc_path.join("status");

        if let Ok(status_content) = std::fs::read_to_string(&status_path) {
            for line in status_content.lines() {
                if line.starts_with("PPid:") {
                    if let Some(ppid) = line.split(':').nth(1) {
                        return Ok(Some(ppid.trim().parse().unwrap_or(0)));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Получение имени процесса.
    async fn get_process_name(&self, pid: i32) -> Result<Option<String>> {
        let proc_path = Path::new("/proc").join(pid.to_string());
        let status_path = proc_path.join("status");

        if let Ok(status_content) = std::fs::read_to_string(&status_path) {
            for line in status_content.lines() {
                if line.starts_with("Name:") {
                    if let Some(name) = line.split(':').nth(1) {
                        return Ok(Some(name.trim().to_string()));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Получение времени создания процесса.
    async fn get_process_start_time(&self, pid: i32) -> Result<Option<DateTime<Utc>>> {
        let proc_path = Path::new("/proc").join(pid.to_string());
        let stat_path = proc_path.join("stat");

        if let Ok(stat_content) = std::fs::read_to_string(&stat_path) {
            // Парсинг времени создания из /proc/[pid]/stat
            // Формат: pid (comm) state ppid ... starttime ...
            let parts: Vec<&str> = stat_content.split_whitespace().collect();
            if parts.len() >= 22 {
                if let Ok(start_time_clock_ticks) = parts[21].parse::<i64>() {
                    // Конвертация clock ticks в DateTime
                    // Используем стандартное значение 100 clock ticks per second для Linux
                    let clock_ticks_per_second = 100;
                    let boot_time = self.get_system_boot_time().await?;
                    let duration_since_boot = Duration::from_secs_f64(
                        start_time_clock_ticks as f64 / clock_ticks_per_second as f64,
                    );
                    let start_time = boot_time + duration_since_boot;
                    return Ok(Some(start_time));
                }
            }
        }

        Ok(None)
    }

    /// Получение времени загрузки системы.
    async fn get_system_boot_time(&self) -> Result<DateTime<Utc>> {
        let proc_stat_path = Path::new("/proc/stat");
        if let Ok(stat_content) = std::fs::read_to_string(proc_stat_path) {
            for line in stat_content.lines() {
                if line.starts_with("btime") {
                    if let Some(btime) = line.split_whitespace().nth(1) {
                        if let Ok(btime_secs) = btime.parse::<i64>() {
                            return Ok(DateTime::<Utc>::from_timestamp(btime_secs, 0)
                                .unwrap_or(Utc::now()));
                        }
                    }
                }
            }
        }

        Ok(Utc::now())
    }

    /// Подсчет дочерних процессов.
    async fn count_child_processes(&self, parent_pid: i32) -> Result<usize> {
        let mut count = 0;
        let proc_dir = Path::new("/proc");

        if !proc_dir.exists() {
            return Ok(count);
        }

        for entry in proc_dir.read_dir()? {
            let entry = entry?;
            let pid_str = entry.file_name().to_string_lossy().to_string();

            if let Ok(pid) = pid_str.parse::<i32>() {
                if let Ok(Some(ppid)) = self.get_parent_pid(pid).await {
                    if ppid == parent_pid {
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    /// Подсчет открытых файлов.
    async fn count_open_files(&self, pid: i32) -> Result<usize> {
        let fd_path = Path::new("/proc").join(pid.to_string()).join("fd");
        if !fd_path.exists() {
            return Ok(0);
        }

        let mut count = 0;
        for entry in fd_path.read_dir()? {
            let _ = entry?;
            count += 1;
        }

        Ok(count)
    }

    /// Подсчет сетевых соединений.
    async fn count_network_connections(&self, pid: i32) -> Result<usize> {
        // В реальной реализации здесь будет анализ /proc/[pid]/fd и /proc/net/tcp
        // Для примера возвращаем 0
        Ok(0)
    }

    /// Проверка подозрительных паттернов поведения.
    async fn check_suspicious_behavior_patterns(
        &self,
        behavior: &ProcessBehavior,
    ) -> Result<Vec<SuspiciousBehaviorPattern>> {
        let mut patterns = Vec::new();

        // Паттерн 1: Слишком много дочерних процессов
        if behavior.child_count > 10 {
            patterns.push(SuspiciousBehaviorPattern {
                pattern_type: "high_child_process_count".to_string(),
                description: "Process has unusually high number of child processes".to_string(),
                severity: SecurityEventSeverity::Medium,
                threshold: 10.0,
                current_value: behavior.child_count as f32,
            });
        }

        // Паттерн 2: Слишком много потоков
        if behavior.thread_count > 100 {
            patterns.push(SuspiciousBehaviorPattern {
                pattern_type: "high_thread_count".to_string(),
                description: "Process has unusually high number of threads".to_string(),
                severity: SecurityEventSeverity::Medium,
                threshold: 100.0,
                current_value: behavior.thread_count as f32,
            });
        }

        // Паттерн 3: Слишком много открытых файлов
        if behavior.open_files_count > 100 {
            patterns.push(SuspiciousBehaviorPattern {
                pattern_type: "high_open_files_count".to_string(),
                description: "Process has unusually high number of open files".to_string(),
                severity: SecurityEventSeverity::Low,
                threshold: 100.0,
                current_value: behavior.open_files_count as f32,
            });
        }

        // Паттерн 4: Подозрительный родительский процесс
        if let Some(parent_name) = &behavior.parent_name {
            if self.is_suspicious_process(parent_name) {
                patterns.push(SuspiciousBehaviorPattern {
                    pattern_type: "suspicious_parent_process".to_string(),
                    description: format!("Process has suspicious parent: {}", parent_name),
                    severity: SecurityEventSeverity::High,
                    threshold: 0.0,
                    current_value: 1.0,
                });
            }
        }

        // Паттерн 5: Высокое использование CPU
        if behavior.cpu_usage > 90.0 {
            patterns.push(SuspiciousBehaviorPattern {
                pattern_type: "high_cpu_usage".to_string(),
                description: "Process has unusually high CPU usage".to_string(),
                severity: SecurityEventSeverity::Medium,
                threshold: 90.0,
                current_value: behavior.cpu_usage,
            });
        }

        // Паттерн 6: Высокое использование памяти
        if behavior.memory_usage > 80.0 {
            patterns.push(SuspiciousBehaviorPattern {
                pattern_type: "high_memory_usage".to_string(),
                description: "Process has unusually high memory usage".to_string(),
                severity: SecurityEventSeverity::Medium,
                threshold: 80.0,
                current_value: behavior.memory_usage,
            });
        }

        Ok(patterns)
    }

    /// Проверка подозрительных паттернов поведения процессов.
    async fn check_suspicious_behavior(
        &self,
        _security_monitor: &mut SecurityMonitor,
    ) -> Result<()> {
        // Получаем список всех процессов
        let processes = self.get_all_processes().await?;

        for process in processes {
            // Анализируем поведение процесса
            let behavior = self.analyze_process_behavior(process.pid).await?;

            // Проверяем подозрительные паттерны
            let patterns = self.check_suspicious_behavior_patterns(&behavior).await?;

            for pattern in patterns {
                let event = SecurityEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    timestamp: Utc::now(),
                    event_type: SecurityEventType::UnusualProcessActivity,
                    severity: pattern.severity,
                    status: SecurityEventStatus::New,
                    process_name: Some(process.name.clone()),
                    process_id: Some(process.pid),
                    description: format!(
                        "Suspicious behavior pattern detected: {}",
                        pattern.pattern_type
                    ),
                    details: Some(format!(
                        "Pattern: {}\nDescription: {}\nThreshold: {}\nCurrent: {}",
                        pattern.pattern_type,
                        pattern.description,
                        pattern.threshold,
                        pattern.current_value
                    )),
                    recommendations: Some(
                        "Investigate this process behavior and monitor for potential threats"
                            .to_string(),
                    ),
                    resolved_time: None,
                };

                self.add_security_event(event).await?;
            }
        }

        Ok(())
    }
}

/// Информация о процессе.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessInfo {
    /// Идентификатор процесса
    pub pid: i32,
    /// Имя процесса
    pub name: String,
    /// Путь к исполняемому файлу
    pub exe_path: Option<String>,
    /// Использование CPU (в процентах)
    pub cpu_usage: f32,
    /// Использование памяти (в процентах)
    pub memory_usage: f32,
}

/// Информация о сетевом соединении.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkConnection {
    /// Идентификатор процесса
    pub process_id: Option<i32>,
    /// Имя процесса
    pub process_name: Option<String>,
    /// Локальный адрес
    pub local_address: String,
    /// Локальный порт
    pub local_port: u16,
    /// Удаленный адрес
    pub remote_address: String,
    /// Удаленный порт
    pub remote_port: u16,
    /// Протокол
    pub protocol: String,
    /// Состояние соединения
    pub state: String,
}

/// Информация об активности файловой системы.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilesystemActivity {
    /// Идентификатор процесса
    pub process_id: Option<i32>,
    /// Имя процесса
    pub process_name: Option<String>,
    /// Путь к файлу
    pub path: String,
    /// Тип операции
    pub operation: String,
    /// Время операции
    pub timestamp: DateTime<Utc>,
}

/// Информация о поведении процесса.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessBehavior {
    /// Идентификатор процесса
    pub pid: i32,
    /// Количество дочерних процессов
    pub child_count: usize,
    /// Количество потоков
    pub thread_count: usize,
    /// Количество открытых файлов
    pub open_files_count: usize,
    /// Количество сетевых соединений
    pub network_connections_count: usize,
    /// Время создания процесса
    pub start_time: Option<DateTime<Utc>>,
    /// Родительский процесс
    pub parent_pid: Option<i32>,
    /// Родительское имя процесса
    pub parent_name: Option<String>,
    /// Использование CPU
    pub cpu_usage: f32,
    /// Использование памяти
    pub memory_usage: f32,
    /// Имя устройства (если применимо)
    pub device_name: String,
    /// Частота создания дочерних процессов (в минуту)
    pub child_creation_rate: f32,
}

/// Паттерны подозрительного поведения.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuspiciousBehaviorPattern {
    /// Тип паттерна
    pub pattern_type: String,
    /// Описание паттерна
    pub description: String,
    /// Уровень серьезности
    pub severity: SecurityEventSeverity,
    /// Пороговое значение
    pub threshold: f32,
    /// Текущее значение
    pub current_value: f32,
}

/// Вспомогательная функция для создания SecurityMonitor.
pub fn create_security_monitor(config: SecurityMonitorConfig) -> SecurityMonitorImpl {
    SecurityMonitorImpl::new(config)
}

/// Вспомогательная функция для создания SecurityMonitor с конфигурацией по умолчанию.
pub fn create_default_security_monitor() -> SecurityMonitorImpl {
    SecurityMonitorImpl::new_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_security_monitor_creation() {
        let config = SecurityMonitorConfig::default();
        let monitor = SecurityMonitorImpl::new(config);

        let status = monitor.get_security_status().await.unwrap();
        assert_eq!(status.overall_status, SecurityStatus::Unknown);
        assert_eq!(status.event_history.len(), 0);
    }

    #[tokio::test]
    async fn test_add_security_event() {
        let config = SecurityMonitorConfig::default();
        let monitor = SecurityMonitorImpl::new(config);

        let event = SecurityEvent {
            event_id: "test-event-1".to_string(),
            timestamp: Utc::now(),
            event_type: SecurityEventType::SuspiciousProcess,
            severity: SecurityEventSeverity::High,
            status: SecurityEventStatus::New,
            process_name: Some("test_process".to_string()),
            process_id: Some(1234),
            description: "Test suspicious process".to_string(),
            details: Some("Test details".to_string()),
            recommendations: Some("Test recommendations".to_string()),
            resolved_time: None,
        };

        monitor.add_security_event(event).await.unwrap();

        let status = monitor.get_security_status().await.unwrap();
        assert_eq!(status.event_history.len(), 1);
        assert_eq!(status.event_history[0].event_id, "test-event-1");
    }

    #[tokio::test]
    async fn test_resolve_security_event() {
        let config = SecurityMonitorConfig::default();
        let monitor = SecurityMonitorImpl::new(config);

        let event = SecurityEvent {
            event_id: "test-event-2".to_string(),
            timestamp: Utc::now(),
            event_type: SecurityEventType::SuspiciousProcess,
            severity: SecurityEventSeverity::High,
            status: SecurityEventStatus::New,
            process_name: Some("test_process".to_string()),
            process_id: Some(1234),
            description: "Test suspicious process".to_string(),
            details: Some("Test details".to_string()),
            recommendations: Some("Test recommendations".to_string()),
            resolved_time: None,
        };

        monitor.add_security_event(event).await.unwrap();
        monitor
            .resolve_security_event("test-event-2")
            .await
            .unwrap();

        let status = monitor.get_security_status().await.unwrap();
        assert_eq!(status.event_history.len(), 1);
        assert_eq!(
            status.event_history[0].status,
            SecurityEventStatus::Analyzed
        );
    }

    #[tokio::test]
    async fn test_mark_event_as_false_positive() {
        let config = SecurityMonitorConfig::default();
        let monitor = SecurityMonitorImpl::new(config);

        let event = SecurityEvent {
            event_id: "test-event-3".to_string(),
            timestamp: Utc::now(),
            event_type: SecurityEventType::SuspiciousProcess,
            severity: SecurityEventSeverity::High,
            status: SecurityEventStatus::New,
            process_name: Some("test_process".to_string()),
            process_id: Some(1234),
            description: "Test suspicious process".to_string(),
            details: Some("Test details".to_string()),
            recommendations: Some("Test recommendations".to_string()),
            resolved_time: None,
        };

        monitor.add_security_event(event).await.unwrap();
        monitor
            .mark_event_as_false_positive("test-event-3")
            .await
            .unwrap();

        let status = monitor.get_security_status().await.unwrap();
        assert_eq!(status.event_history.len(), 1);
        assert_eq!(
            status.event_history[0].status,
            SecurityEventStatus::FalsePositive
        );
    }

    #[tokio::test]
    async fn test_security_stats() {
        let config = SecurityMonitorConfig::default();
        let monitor = SecurityMonitorImpl::new(config);

        let stats = monitor.get_security_stats().await.unwrap();
        assert_eq!(stats.total_events, 0);
        assert_eq!(stats.critical_events, 0);
        assert_eq!(stats.high_events, 0);
        assert_eq!(stats.medium_events, 0);
        assert_eq!(stats.low_events, 0);
        assert_eq!(stats.confirmed_threats, 0);
        assert_eq!(stats.false_positives, 0);
    }

    #[tokio::test]
    async fn test_is_suspicious_process() {
        let config = SecurityMonitorConfig::default();
        let monitor = SecurityMonitorImpl::new(config);

        // Проверяем, что известные подозрительные процессы обнаруживаются
        assert!(monitor.is_suspicious_process("bitcoin"));
        assert!(monitor.is_suspicious_process("minerd"));
        assert!(monitor.is_suspicious_process("xmrig"));

        // Проверяем, что обычные процессы не обнаруживаются
        assert!(!monitor.is_suspicious_process("smoothtaskd"));
        assert!(!monitor.is_suspicious_process("systemd"));
    }

    #[tokio::test]
    async fn test_analyze_process_behavior() {
        let config = SecurityMonitorConfig::default();
        let monitor = SecurityMonitorImpl::new(config);

        // Тестируем анализ поведения процесса (используем текущий PID)
        let current_pid = std::process::id() as i32;
        let behavior = monitor.analyze_process_behavior(current_pid).await.unwrap();

        // Проверяем, что поведение содержит основную информацию
        assert_eq!(behavior.pid, current_pid);
        assert!(behavior.thread_count > 0);
        assert!(behavior.open_files_count > 0);
    }

    #[tokio::test]
    async fn test_check_suspicious_behavior_patterns() {
        let config = SecurityMonitorConfig::default();
        let monitor = SecurityMonitorImpl::new(config);

        // Создаем тестовое поведение с высокими значениями
        let mut behavior = ProcessBehavior {
            pid: 1234,
            child_count: 15,       // Выше порога
            thread_count: 150,     // Выше порога
            open_files_count: 150, // Выше порога
            network_connections_count: 0,
            start_time: None,
            parent_pid: None,
            parent_name: Some("xmrig".to_string()), // Подозрительный родитель
            cpu_usage: 95.0,                        // Выше порога
            memory_usage: 85.0,                     // Выше порога
            child_creation_rate: 0.0,
            device_name: String::new(),
        };

        // Проверяем обнаружение паттернов
        let patterns = monitor
            .check_suspicious_behavior_patterns(&behavior)
            .await
            .unwrap();

        // Должны быть обнаружены несколько паттернов
        assert!(!patterns.is_empty());
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == "high_child_process_count"));
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == "high_thread_count"));
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == "high_open_files_count"));
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == "suspicious_parent_process"));
        assert!(patterns.iter().any(|p| p.pattern_type == "high_cpu_usage"));
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == "high_memory_usage"));
    }

    #[tokio::test]
    async fn test_check_suspicious_behavior() {
        let config = SecurityMonitorConfig::default();
        let monitor = SecurityMonitorImpl::new(config);

        // Создаем тестовый SecurityMonitor
        let mut security_monitor = SecurityMonitor::default();

        // Выполняем проверку подозрительного поведения
        let result = monitor
            .check_suspicious_behavior(&mut security_monitor)
            .await;

        // Проверяем, что проверка завершилась успешно
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_resource_anomaly_detection() {
        let config = SecurityMonitorConfig::default();
        let monitor = SecurityMonitorImpl::new(config);

        // Создаем тестовое поведение с аномальными значениями
        let mut behavior = ProcessBehavior {
            pid: 1234,
            child_count: 25,       // Выше порога
            thread_count: 250,     // Выше порога
            open_files_count: 250, // Выше порога
            network_connections_count: 0,
            start_time: None,
            parent_pid: None,
            parent_name: None,
            cpu_usage: 96.0,          // Выше порога
            memory_usage: 86.0,       // Выше порога
            child_creation_rate: 6.0, // Выше порога
            device_name: String::new(),
        };

        // Проверяем обнаружение паттернов аномалий
        let patterns = monitor
            .detect_resource_anomaly_patterns(&behavior)
            .await
            .unwrap();

        // Должны быть обнаружены несколько паттернов
        assert!(!patterns.is_empty());
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == "anomalous_child_process_count"));
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == "anomalous_thread_count"));
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == "anomalous_open_files_count"));
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == "anomalous_cpu_usage"));
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == "anomalous_memory_usage"));
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == "anomalous_child_creation_rate"));
    }

    #[tokio::test]
    async fn test_system_process_detection() {
        let config = SecurityMonitorConfig::default();
        let monitor = SecurityMonitorImpl::new(config);

        // Проверяем обнаружение системных процессов
        assert!(monitor.is_system_process("systemd"));
        assert!(monitor.is_system_process("kthreadd"));
        assert!(monitor.is_system_process("smoothtaskd"));

        // Проверяем, что пользовательские процессы не обнаруживаются как системные
        assert!(!monitor.is_system_process("firefox"));
        assert!(!monitor.is_system_process("chrome"));
        assert!(!monitor.is_system_process("python"));
    }

    #[tokio::test]
    async fn test_notification_integration() {
        use crate::notifications::{NotificationType, StubNotifier};
        use std::sync::Arc;

        let config = SecurityMonitorConfig::default();
        let notifier = Arc::new(StubNotifier::default());
        let monitor = SecurityMonitorImpl::new_with_notifier(config, notifier);

        // Создаем тестовое событие безопасности
        let event = SecurityEvent {
            event_id: "test-event-1".to_string(),
            timestamp: Utc::now(),
            event_type: SecurityEventType::SuspiciousProcess,
            severity: SecurityEventSeverity::High,
            status: SecurityEventStatus::New,
            process_name: Some("test_process".to_string()),
            process_id: Some(1234),
            description: "Test suspicious process for notification".to_string(),
            details: Some("Test details for notification".to_string()),
            recommendations: Some("Test recommendations".to_string()),
            resolved_time: None,
        };

        // Проверяем, что уведомление должно быть отправлено для высокого уровня серьезности
        assert!(monitor.should_send_notification_for_event(&event));

        // Добавляем событие (должно отправить уведомление)
        let result = monitor.add_security_event(event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_notification_thresholds() {
        let config = SecurityMonitorConfig::default();
        let monitor = SecurityMonitorImpl::new(config);

        // Проверяем пороги уведомлений
        let critical_event = SecurityEvent {
            event_id: "test-critical".to_string(),
            timestamp: Utc::now(),
            event_type: SecurityEventType::PotentialAttack,
            severity: SecurityEventSeverity::Critical,
            status: SecurityEventStatus::New,
            process_name: None,
            process_id: None,
            description: "Critical test event".to_string(),
            details: None,
            recommendations: None,
            resolved_time: None,
        };

        let medium_event = SecurityEvent {
            event_id: "test-medium".to_string(),
            timestamp: Utc::now(),
            event_type: SecurityEventType::AnomalousResourceUsage,
            severity: SecurityEventSeverity::Medium,
            status: SecurityEventStatus::New,
            process_name: None,
            process_id: None,
            description: "Medium test event".to_string(),
            details: None,
            recommendations: None,
            resolved_time: None,
        };

        let low_event = SecurityEvent {
            event_id: "test-low".to_string(),
            timestamp: Utc::now(),
            event_type: SecurityEventType::UnusualProcessActivity,
            severity: SecurityEventSeverity::Low,
            status: SecurityEventStatus::New,
            process_name: None,
            process_id: None,
            description: "Low test event".to_string(),
            details: None,
            recommendations: None,
            resolved_time: None,
        };

        // Проверяем пороги уведомлений
        assert!(monitor.should_send_notification_for_event(&critical_event));
        assert!(monitor.should_send_notification_for_event(&medium_event));
        assert!(!monitor.should_send_notification_for_event(&low_event));
    }
}

// Реэкспорт основных типов для удобства использования
// (удалено для избежания конфликтов)
