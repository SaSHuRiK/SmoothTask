//! Модуль обнаружения угроз SmoothTask.
//!
//! Этот модуль предоставляет расширенную систему обнаружения угроз безопасности
//! с использованием ML-инспирированных алгоритмов и поведенческого анализа.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Тип обнаруженной угрозы.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum ThreatType {
    /// Неизвестная угроза
    #[serde(rename = "unknown")]
    Unknown,
    /// Поведенческая аномалия
    #[serde(rename = "behavioral_anomaly")]
    BehavioralAnomaly,
    /// Сетевая аномалия
    #[serde(rename = "network_anomaly")]
    NetworkAnomaly,
    /// Аномалия файловой системы
    #[serde(rename = "filesystem_anomaly")]
    FilesystemAnomaly,
    /// Аномалия использования ресурсов
    #[serde(rename = "resource_anomaly")]
    ResourceAnomaly,
    /// Потенциальная атака
    #[serde(rename = "potential_attack")]
    PotentialAttack,
    /// Атака методом перебора
    #[serde(rename = "brute_force_attack")]
    BruteForceAttack,
    /// SQL инъекция
    #[serde(rename = "sql_injection")]
    SqlInjection,
    /// XSS атака
    #[serde(rename = "xss_attack")]
    XssAttack,
    /// Атака "человек посередине"
    #[serde(rename = "mitm_attack")]
    MitmAttack,
    /// Активность программ-вымогателей
    #[serde(rename = "ransomware_activity")]
    RansomwareActivity,
    /// Активность ботнета
    #[serde(rename = "botnet_activity")]
    BotnetActivity,
    /// Утечка данных
    #[serde(rename = "data_exfiltration")]
    DataExfiltration,
    /// Криптоджекинг
    #[serde(rename = "cryptojacking")]
    Cryptojacking,
    /// Фишинг
    #[serde(rename = "phishing_activity")]
    PhishingActivity,
    /// Вредоносное ПО
    #[serde(rename = "malware_communication")]
    MalwareCommunication,
    /// DNS туннелирование
    #[serde(rename = "dns_tunneling")]
    DnsTunneling,
    /// ICMP туннелирование
    #[serde(rename = "icmp_tunneling")]
    IcmpTunneling,
    /// HTTP туннелирование
    #[serde(rename = "http_tunneling")]
    HttpTunneling,
    /// Аномалия протокола
    #[serde(rename = "protocol_anomaly")]
    ProtocolAnomaly,
    /// Аномалия шифрования
    #[serde(rename = "encryption_anomaly")]
    EncryptionAnomaly,
    /// Сбой аутентификации
    #[serde(rename = "authentication_failure")]
    AuthenticationFailure,
    /// Эксплуатация уязвимости нулевого дня
    #[serde(rename = "zero_day_exploit")]
    ZeroDayExploit,
    /// Продвинутая постоянная угроза
    #[serde(rename = "apt_activity")]
    AptActivity,
}

impl Default for ThreatType {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for ThreatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreatType::Unknown => write!(f, "unknown"),
            ThreatType::BehavioralAnomaly => write!(f, "behavioral_anomaly"),
            ThreatType::NetworkAnomaly => write!(f, "network_anomaly"),
            ThreatType::FilesystemAnomaly => write!(f, "filesystem_anomaly"),
            ThreatType::ResourceAnomaly => write!(f, "resource_anomaly"),
            ThreatType::PotentialAttack => write!(f, "potential_attack"),
            ThreatType::BruteForceAttack => write!(f, "brute_force_attack"),
            ThreatType::SqlInjection => write!(f, "sql_injection"),
            ThreatType::XssAttack => write!(f, "xss_attack"),
            ThreatType::MitmAttack => write!(f, "mitm_attack"),
            ThreatType::RansomwareActivity => write!(f, "ransomware_activity"),
            ThreatType::BotnetActivity => write!(f, "botnet_activity"),
            ThreatType::DataExfiltration => write!(f, "data_exfiltration"),
            ThreatType::Cryptojacking => write!(f, "cryptojacking"),
            ThreatType::PhishingActivity => write!(f, "phishing_activity"),
            ThreatType::MalwareCommunication => write!(f, "malware_communication"),
            ThreatType::DnsTunneling => write!(f, "dns_tunneling"),
            ThreatType::IcmpTunneling => write!(f, "icmp_tunneling"),
            ThreatType::HttpTunneling => write!(f, "http_tunneling"),
            ThreatType::ProtocolAnomaly => write!(f, "protocol_anomaly"),
            ThreatType::EncryptionAnomaly => write!(f, "encryption_anomaly"),
            ThreatType::AuthenticationFailure => write!(f, "authentication_failure"),
            ThreatType::ZeroDayExploit => write!(f, "zero_day_exploit"),
            ThreatType::AptActivity => write!(f, "apt_activity"),
        }
    }
}

/// Уровень серьезности обнаруженной угрозы.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatDetectionSeverity {
    /// Информационный уровень
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

impl Default for ThreatDetectionSeverity {
    fn default() -> Self {
        Self::Info
    }
}

impl std::fmt::Display for ThreatDetectionSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreatDetectionSeverity::Info => write!(f, "info"),
            ThreatDetectionSeverity::Low => write!(f, "low"),
            ThreatDetectionSeverity::Medium => write!(f, "medium"),
            ThreatDetectionSeverity::High => write!(f, "high"),
            ThreatDetectionSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// Статус обнаруженной угрозы.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatStatus {
    /// Новая угроза
    #[serde(rename = "new")]
    New,
    /// В процессе анализа
    #[serde(rename = "analyzing")]
    Analyzing,
    /// Проанализирована
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

impl Default for ThreatStatus {
    fn default() -> Self {
        Self::New
    }
}

/// Информация об обнаруженной угрозе.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatDetection {
    /// Уникальный идентификатор угрозы
    pub threat_id: String,
    /// Время обнаружения угрозы
    pub timestamp: DateTime<Utc>,
    /// Тип угрозы
    pub threat_type: ThreatType,
    /// Серьезность угрозы
    pub severity: ThreatDetectionSeverity,
    /// Статус угрозы
    pub status: ThreatStatus,
    /// Имя процесса (если применимо)
    pub process_name: Option<String>,
    /// Идентификатор процесса (если применимо)
    pub process_id: Option<i32>,
    /// Описание угрозы
    pub description: String,
    /// Детали угрозы
    pub details: Option<String>,
    /// Уровень уверенности (0.0-100.0)
    pub confidence_score: f32,
    /// Рекомендации по действиям
    pub recommendations: Option<String>,
    /// Время разрешения угрозы (если решена)
    pub resolved_time: Option<DateTime<Utc>>,
}

impl Default for ThreatDetection {
    fn default() -> Self {
        Self {
            threat_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            threat_type: ThreatType::Unknown,
            severity: ThreatDetectionSeverity::Info,
            status: ThreatStatus::New,
            process_name: None,
            process_id: None,
            description: String::new(),
            details: None,
            confidence_score: 0.0,
            recommendations: None,
            resolved_time: None,
        }
    }
}

/// Конфигурация системы обнаружения угроз.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatDetectionConfig {
    /// Включить обнаружение угроз
    pub enabled: bool,
    /// Интервал проверки угроз
    pub check_interval: std::time::Duration,
    /// Максимальное количество хранимых угроз
    pub max_threat_history: usize,
    /// Пороги для обнаружения аномалий
    pub anomaly_thresholds: ThreatAnomalyThresholds,
    /// Настройки ML-алгоритмов
    pub ml_settings: MLSettings,
    /// Список доверенных процессов
    pub trusted_processes: Vec<String>,
    /// Список подозрительных процессов
    pub suspicious_processes: Vec<String>,
    /// Настройки уведомлений
    pub notification_settings: ThreatNotificationSettings,
}

impl Default for ThreatDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_interval: std::time::Duration::from_secs(60), // 1 минута
            max_threat_history: 1000,
            anomaly_thresholds: ThreatAnomalyThresholds::default(),
            ml_settings: MLSettings::default(),
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
            notification_settings: ThreatNotificationSettings::default(),
        }
    }
}

/// Пороги для обнаружения аномалий угроз.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatAnomalyThresholds {
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
    /// Максимальная частота создания дочерних процессов (в минуту)
    pub max_child_creation_rate: f32,
}

impl Default for ThreatAnomalyThresholds {
    fn default() -> Self {
        Self {
            max_new_processes_per_minute: 30,
            max_cpu_usage_percent: 85.0,
            max_memory_usage_percent: 75.0,
            max_network_connections: 50,
            max_open_files: 500,
            max_threads: 200,
            max_child_creation_rate: 3.0,
        }
    }
}

/// Настройки ML-алгоритмов для обнаружения угроз.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MLSettings {
    /// Включить ML-базированное обнаружение
    pub enabled: bool,
    /// Минимальный уровень уверенности для ML-обнаружения
    pub min_confidence_threshold: f32,
    /// Веса для различных типов угроз
    pub threat_type_weights: HashMap<String, f32>,
    /// Настройки обучения модели
    pub training_settings: MLTrainingSettings,
}

impl Default for MLSettings {
    fn default() -> Self {
        let mut threat_type_weights = HashMap::new();
        threat_type_weights.insert("behavioral_anomaly".to_string(), 1.0);
        threat_type_weights.insert("network_anomaly".to_string(), 1.2);
        threat_type_weights.insert("filesystem_anomaly".to_string(), 1.1);
        threat_type_weights.insert("resource_anomaly".to_string(), 1.0);
        threat_type_weights.insert("potential_attack".to_string(), 1.5);

        Self {
            enabled: true,
            min_confidence_threshold: 70.0,
            threat_type_weights,
            training_settings: MLTrainingSettings::default(),
        }
    }
}

/// Настройки обучения ML-модели.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MLTrainingSettings {
    /// Включить автоматическое обучение
    pub auto_training_enabled: bool,
    /// Интервал автоматического обучения
    pub auto_training_interval: std::time::Duration,
    /// Размер обучающего набора данных
    pub training_dataset_size: usize,
    /// Количество эпох обучения
    pub training_epochs: usize,
    /// Размер батча для обучения
    pub batch_size: usize,
    /// Скорость обучения
    pub learning_rate: f32,
}

impl Default for MLTrainingSettings {
    fn default() -> Self {
        Self {
            auto_training_enabled: false,
            auto_training_interval: std::time::Duration::from_secs(3600), // 1 час
            training_dataset_size: 1000,
            training_epochs: 10,
            batch_size: 32,
            learning_rate: 0.01,
        }
    }
}

/// Настройки уведомлений об угрозах.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatNotificationSettings {
    /// Включить уведомления о критических угрозах
    pub enable_critical_notifications: bool,
    /// Включить уведомления об угрозах высокого уровня
    pub enable_high_notifications: bool,
    /// Включить уведомления об угрозах среднего уровня
    pub enable_medium_notifications: bool,
    /// Максимальная частота уведомлений (в секундах)
    pub max_notification_frequency_seconds: u64,
}

impl Default for ThreatNotificationSettings {
    fn default() -> Self {
        Self {
            enable_critical_notifications: true,
            enable_high_notifications: true,
            enable_medium_notifications: false,
            max_notification_frequency_seconds: 300, // 5 минут
        }
    }
}

/// Основная структура для системы обнаружения угроз.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ThreatDetectionSystem {
    /// Время последней проверки угроз
    pub last_check_time: Option<DateTime<Utc>>,
    /// Общий статус угроз
    pub overall_status: ThreatSystemStatus,
    /// История обнаруженных угроз
    pub threat_history: Vec<ThreatDetection>,
    /// Конфигурация системы обнаружения угроз
    pub config: ThreatDetectionConfig,
    /// Текущий балл безопасности (0-100)
    pub security_score: f32,
    /// История баллов безопасности для анализа трендов
    pub security_score_history: Vec<ThreatSecurityScoreEntry>,
    /// ML-модель для обнаружения угроз
    pub ml_model: Option<MLThreatDetectionModel>,
}

/// Статус системы угроз.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatSystemStatus {
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

impl Default for ThreatSystemStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Запись балла безопасности с временной меткой.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatSecurityScoreEntry {
    /// Время записи балла
    pub timestamp: DateTime<Utc>,
    /// Балл безопасности (0-100)
    pub score: f32,
    /// Состояние безопасности в это время
    pub status: ThreatSystemStatus,
}

impl Default for ThreatSecurityScoreEntry {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            score: 100.0,
            status: ThreatSystemStatus::Secure,
        }
    }
}

/// ML-модель для обнаружения угроз.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MLThreatDetectionModel {
    /// Версия модели
    pub version: String,
    /// Время обучения модели
    pub training_time: DateTime<Utc>,
    /// Точность модели
    pub accuracy: f32,
    /// Параметры модели
    pub parameters: HashMap<String, f32>,
    /// Статистика модели
    pub statistics: MLModelStatistics,
}

impl Default for MLThreatDetectionModel {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            training_time: Utc::now(),
            accuracy: 0.0,
            parameters: HashMap::new(),
            statistics: MLModelStatistics::default(),
        }
    }
}

/// Статистика ML-модели.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MLModelStatistics {
    /// Количество обучающих образцов
    pub training_samples: usize,
    /// Количество тестовых образцов
    pub test_samples: usize,
    /// Точность на обучающем наборе
    pub training_accuracy: f32,
    /// Точность на тестовом наборе
    pub test_accuracy: f32,
    /// Время обучения (в секундах)
    pub training_time_seconds: f32,
    /// Количество эпох
    pub epochs: usize,
}

impl Default for MLModelStatistics {
    fn default() -> Self {
        Self {
            training_samples: 0,
            test_samples: 0,
            training_accuracy: 0.0,
            test_accuracy: 0.0,
            training_time_seconds: 0.0,
            epochs: 0,
        }
    }
}

/// Интерфейс для системы обнаружения угроз.
#[async_trait::async_trait]
pub trait ThreatDetectionTrait: Send + Sync {
    /// Выполнить проверку угроз.
    async fn check_threats(&self) -> Result<ThreatDetectionSystem>;

    /// Обновить состояние угроз.
    async fn update_threat_status(&self, threat_system: ThreatDetectionSystem) -> Result<()>;

    /// Получить текущее состояние угроз.
    async fn get_threat_status(&self) -> Result<ThreatDetectionSystem>;

    /// Добавить обнаруженную угрозу.
    async fn add_threat_detection(&self, threat: ThreatDetection) -> Result<()>;

    /// Разрешить обнаруженную угрозу.
    async fn resolve_threat_detection(&self, threat_id: &str) -> Result<()>;

    /// Очистить историю угроз.
    async fn clear_threat_history(&self) -> Result<()>;

    /// Пометить угрозу как ложное срабатывание.
    async fn mark_threat_as_false_positive(&self, threat_id: &str) -> Result<()>;

    /// Получить статистику угроз.
    async fn get_threat_stats(&self) -> Result<ThreatDetectionStats>;

    /// Очистить статистику угроз.
    async fn clear_threat_stats(&self) -> Result<()>;

    /// Обновить ML-модель.
    async fn update_ml_model(&self, model: MLThreatDetectionModel) -> Result<()>;

    /// Получить текущую ML-модель.
    async fn get_ml_model(&self) -> Result<Option<MLThreatDetectionModel>>;

    /// Обучить ML-модель.
    async fn train_ml_model(&self) -> Result<MLThreatDetectionModel>;
}

/// Статистика обнаруженных угроз.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatDetectionStats {
    /// Общее количество угроз
    pub total_threats: usize,
    /// Количество критических угроз
    pub critical_threats: usize,
    /// Количество угроз высокого уровня
    pub high_threats: usize,
    /// Количество угроз среднего уровня
    pub medium_threats: usize,
    /// Количество угроз низкого уровня
    pub low_threats: usize,
    /// Количество подтвержденных угроз
    pub confirmed_threats: usize,
    /// Количество ложных срабатываний
    pub false_positives: usize,
    /// Средний уровень уверенности
    pub average_confidence_score: f32,
    /// Время последней угрозы
    pub last_threat_time: Option<DateTime<Utc>>,
}

impl Default for ThreatDetectionStats {
    fn default() -> Self {
        Self {
            total_threats: 0,
            critical_threats: 0,
            high_threats: 0,
            medium_threats: 0,
            low_threats: 0,
            confirmed_threats: 0,
            false_positives: 0,
            average_confidence_score: 0.0,
            last_threat_time: None,
        }
    }
}

/// Реализация ThreatDetectionTrait.
#[derive(Clone)]
pub struct ThreatDetectionSystemImpl {
    threat_state: Arc<tokio::sync::RwLock<ThreatDetectionSystem>>,
    config: ThreatDetectionConfig,
    stats: Arc<tokio::sync::RwLock<ThreatDetectionStats>>,
    notifier: Option<Arc<dyn crate::notifications::Notifier>>,
}

#[async_trait::async_trait]
impl ThreatDetectionTrait for ThreatDetectionSystemImpl {
    async fn check_threats(&self) -> Result<ThreatDetectionSystem> {
        let mut threat_system = self.threat_state.read().await.clone();

        // Обновляем время последней проверки
        threat_system.last_check_time = Some(Utc::now());

        // Выполняем проверку угроз
        threat_system = self.perform_threat_detection(threat_system).await?;

        // Определяем общий статус угроз
        threat_system.overall_status = self.determine_overall_status(&threat_system);

        // Рассчитываем балл безопасности
        self.update_security_score_history(&mut threat_system);

        Ok(threat_system)
    }

    async fn update_threat_status(&self, threat_system: ThreatDetectionSystem) -> Result<()> {
        let mut state = self.threat_state.write().await;
        *state = threat_system;
        Ok(())
    }

    async fn get_threat_status(&self) -> Result<ThreatDetectionSystem> {
        Ok(self.threat_state.read().await.clone())
    }

    async fn add_threat_detection(&self, threat: ThreatDetection) -> Result<()> {
        let mut state = self.threat_state.write().await;

        // Проверяем максимальное количество угроз в истории
        if state.threat_history.len() >= state.config.max_threat_history {
            state.threat_history.remove(0); // Удаляем самую старую угрозу
        }

        // Проверяем, нужно ли отправлять уведомление для этой угрозы
        let should_notify = self.should_send_notification_for_threat(&threat);

        state.threat_history.push(threat.clone());

        // Обновляем статистику
        let mut stats = self.stats.write().await;
        stats.total_threats += 1;
        stats.last_threat_time = Some(Utc::now());
        
        // Обновляем средний уровень уверенности
        let total_confidence = stats.average_confidence_score * (stats.total_threats as f32 - 1.0);
        stats.average_confidence_score = (total_confidence + threat.confidence_score) / stats.total_threats as f32;

        // Отправляем уведомление, если нужно
        if should_notify {
            self.send_threat_notification(&threat).await?;
        }

        Ok(())
    }

    async fn resolve_threat_detection(&self, threat_id: &str) -> Result<()> {
        let mut state = self.threat_state.write().await;

        if let Some(threat) = state
            .threat_history
            .iter_mut()
            .find(|t| t.threat_id == threat_id)
        {
            threat.status = ThreatStatus::Analyzed;
            threat.resolved_time = Some(Utc::now());
        }

        Ok(())
    }

    async fn clear_threat_history(&self) -> Result<()> {
        let mut state = self.threat_state.write().await;
        state.threat_history.clear();
        Ok(())
    }

    async fn mark_threat_as_false_positive(&self, threat_id: &str) -> Result<()> {
        let mut state = self.threat_state.write().await;

        if let Some(threat) = state
            .threat_history
            .iter_mut()
            .find(|t| t.threat_id == threat_id)
        {
            threat.status = ThreatStatus::FalsePositive;

            // Обновляем статистику
            let mut stats = self.stats.write().await;
            stats.false_positives += 1;
        }

        Ok(())
    }

    async fn get_threat_stats(&self) -> Result<ThreatDetectionStats> {
        Ok(self.stats.read().await.clone())
    }

    async fn clear_threat_stats(&self) -> Result<()> {
        let mut stats = self.stats.write().await;
        *stats = ThreatDetectionStats::default();
        Ok(())
    }

    async fn update_ml_model(&self, model: MLThreatDetectionModel) -> Result<()> {
        let mut state = self.threat_state.write().await;
        state.ml_model = Some(model);
        Ok(())
    }

    async fn get_ml_model(&self) -> Result<Option<MLThreatDetectionModel>> {
        Ok(self.threat_state.read().await.ml_model.clone())
    }

    async fn train_ml_model(&self) -> Result<MLThreatDetectionModel> {
        // В реальной реализации здесь будет обучение ML-модели
        // Для примера возвращаем модель по умолчанию
        Ok(MLThreatDetectionModel::default())
    }
}

impl ThreatDetectionSystemImpl {
    /// Создать новый ThreatDetectionSystemImpl.
    pub fn new(config: ThreatDetectionConfig) -> Self {
        Self {
            threat_state: Arc::new(tokio::sync::RwLock::new(ThreatDetectionSystem::default())),
            config,
            stats: Arc::new(tokio::sync::RwLock::new(ThreatDetectionStats::default())),
            notifier: None,
        }
    }

    /// Создать новый ThreatDetectionSystemImpl с конфигурацией по умолчанию.
    pub fn new_default() -> Self {
        Self::new(ThreatDetectionConfig::default())
    }

    /// Создать новый ThreatDetectionSystemImpl с уведомителем.
    pub fn new_with_notifier(
        config: ThreatDetectionConfig,
        notifier: Arc<dyn crate::notifications::Notifier>,
    ) -> Self {
        Self {
            threat_state: Arc::new(tokio::sync::RwLock::new(ThreatDetectionSystem::default())),
            config,
            stats: Arc::new(tokio::sync::RwLock::new(ThreatDetectionStats::default())),
            notifier: Some(notifier),
        }
    }

    /// Выполнить обнаружение угроз.
    async fn perform_threat_detection(
        &self,
        mut threat_system: ThreatDetectionSystem,
    ) -> Result<ThreatDetectionSystem> {
        // Выполняем базовое обнаружение угроз
        self.perform_basic_threat_detection(&mut threat_system).await?;

        // Выполняем ML-базированное обнаружение угроз
        if self.config.ml_settings.enabled {
            self.perform_ml_threat_detection(&mut threat_system).await?;
        }

        // Выполняем продвинутый анализ угроз
        self.perform_advanced_threat_analysis(&mut threat_system).await?;

        Ok(threat_system)
    }

    /// Выполнить базовое обнаружение угроз.
    async fn perform_basic_threat_detection(
        &self,
        _threat_system: &mut ThreatDetectionSystem,
    ) -> Result<()> {
        // В реальной реализации здесь будет базовое обнаружение угроз
        // Для примера возвращаем Ok
        Ok(())
    }

    /// Выполнить ML-базированное обнаружение угроз.
    async fn perform_ml_threat_detection(
        &self,
        threat_system: &mut ThreatDetectionSystem,
    ) -> Result<()> {
        // Выполняем обнаружение сетевых аномалий
        self.detect_network_anomalies(threat_system).await?;

        // Выполняем обнаружение аномалий файловой системы
        self.detect_filesystem_anomalies(threat_system).await?;

        // Выполняем обнаружение аномалий использования ресурсов
        self.detect_resource_anomalies(threat_system).await?;

        // Выполняем обнаружение поведенческих аномалий
        self.detect_behavioral_anomalies(threat_system).await?;

        Ok(())
    }

    /// Выполнить продвинутый анализ угроз.
    async fn perform_advanced_threat_analysis(
        &self,
        threat_system: &mut ThreatDetectionSystem,
    ) -> Result<()> {
        // Выполняем анализ шаблонов безопасности
        self.analyze_security_patterns(threat_system).await?;

        // Выполняем корреляционный анализ угроз
        self.perform_threat_correlation(threat_system).await?;

        Ok(())
    }

    /// Обнаружение сетевых аномалий.
    async fn detect_network_anomalies(
        &self,
        _threat_system: &mut ThreatDetectionSystem,
    ) -> Result<()> {
        // В реальной реализации здесь будет обнаружение сетевых аномалий
        // Для примера добавляем тестовую угрозу
        let network_threat = ThreatDetection {
            threat_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            threat_type: ThreatType::NetworkAnomaly,
            severity: ThreatDetectionSeverity::High,
            status: ThreatStatus::New,
            process_name: Some("network_process".to_string()),
            process_id: Some(1234),
            description: "Suspicious network activity detected".to_string(),
            details: Some("Multiple connections to suspicious ports detected".to_string()),
            confidence_score: 85.0,
            recommendations: Some("Investigate network connections and consider blocking suspicious traffic".to_string()),
            resolved_time: None,
        };

        self.add_threat_detection(network_threat).await?;

        Ok(())
    }

    /// Обнаружение аномалий файловой системы.
    async fn detect_filesystem_anomalies(
        &self,
        _threat_system: &mut ThreatDetectionSystem,
    ) -> Result<()> {
        // В реальной реализации здесь будет обнаружение аномалий файловой системы
        // Для примера добавляем тестовую угрозу
        let filesystem_threat = ThreatDetection {
            threat_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            threat_type: ThreatType::FilesystemAnomaly,
            severity: ThreatDetectionSeverity::Medium,
            status: ThreatStatus::New,
            process_name: Some("filesystem_process".to_string()),
            process_id: Some(5678),
            description: "Suspicious filesystem activity detected".to_string(),
            details: Some("Unauthorized access to system files detected".to_string()),
            confidence_score: 75.0,
            recommendations: Some("Investigate filesystem access patterns and review permissions".to_string()),
            resolved_time: None,
        };

        self.add_threat_detection(filesystem_threat).await?;

        Ok(())
    }

    /// Обнаружение аномалий использования ресурсов.
    async fn detect_resource_anomalies(
        &self,
        _threat_system: &mut ThreatDetectionSystem,
    ) -> Result<()> {
        // В реальной реализации здесь будет обнаружение аномалий использования ресурсов
        // Для примера добавляем тестовую угрозу
        let resource_threat = ThreatDetection {
            threat_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            threat_type: ThreatType::ResourceAnomaly,
            severity: ThreatDetectionSeverity::High,
            status: ThreatStatus::New,
            process_name: Some("resource_process".to_string()),
            process_id: Some(9012),
            description: "Anomalous resource usage detected".to_string(),
            details: Some("Process using excessive CPU and memory resources".to_string()),
            confidence_score: 90.0,
            recommendations: Some("Investigate resource usage and consider terminating suspicious process".to_string()),
            resolved_time: None,
        };

        self.add_threat_detection(resource_threat).await?;

        Ok(())
    }

    /// Обнаружение поведенческих аномалий.
    async fn detect_behavioral_anomalies(
        &self,
        _threat_system: &mut ThreatDetectionSystem,
    ) -> Result<()> {
        // В реальной реализации здесь будет обнаружение поведенческих аномалий
        // Для примера добавляем тестовую угрозу
        let behavioral_threat = ThreatDetection {
            threat_id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            threat_type: ThreatType::BehavioralAnomaly,
            severity: ThreatDetectionSeverity::Critical,
            status: ThreatStatus::New,
            process_name: Some("behavioral_process".to_string()),
            process_id: Some(3456),
            description: "Critical behavioral anomaly detected".to_string(),
            details: Some("Process exhibiting suspicious behavior patterns".to_string()),
            confidence_score: 95.0,
            recommendations: Some("Immediately investigate and take action on this critical threat".to_string()),
            resolved_time: None,
        };

        self.add_threat_detection(behavioral_threat).await?;

        Ok(())
    }

    /// Анализ шаблонов безопасности.
    async fn analyze_security_patterns(
        &self,
        _threat_system: &mut ThreatDetectionSystem,
    ) -> Result<()> {
        // В реальной реализации здесь будет анализ шаблонов безопасности
        // Для примера возвращаем Ok
        Ok(())
    }

    /// Корреляционный анализ угроз.
    async fn perform_threat_correlation(
        &self,
        _threat_system: &mut ThreatDetectionSystem,
    ) -> Result<()> {
        // В реальной реализации здесь будет корреляционный анализ угроз
        // Для примера возвращаем Ok
        Ok(())
    }

    /// Проверка, нужно ли отправлять уведомление для этой угрозы.
    fn should_send_notification_for_threat(&self, threat: &ThreatDetection) -> bool {
        // Проверяем, включены ли уведомления в конфигурации
        match threat.severity {
            ThreatDetectionSeverity::Critical => {
                self.config
                    .notification_settings
                    .enable_critical_notifications
            }
            ThreatDetectionSeverity::High => {
                self.config.notification_settings.enable_high_notifications
            }
            ThreatDetectionSeverity::Medium => {
                self.config
                    .notification_settings
                    .enable_medium_notifications
            }
            ThreatDetectionSeverity::Low => false, // Не отправляем уведомления для низкого уровня
            ThreatDetectionSeverity::Info => false, // Не отправляем уведомления для информационного уровня
        }
    }

    /// Отправить уведомление об угрозе.
    async fn send_threat_notification(&self, threat: &ThreatDetection) -> Result<()> {
        if let Some(notifier) = &self.notifier {
            // Преобразуем уровень серьезности угрозы в тип уведомления
            let notification_type = match threat.severity {
                ThreatDetectionSeverity::Critical => crate::notifications::NotificationType::Critical,
                ThreatDetectionSeverity::High => crate::notifications::NotificationType::Critical,
                ThreatDetectionSeverity::Medium => crate::notifications::NotificationType::Warning,
                ThreatDetectionSeverity::Low => crate::notifications::NotificationType::Info,
                ThreatDetectionSeverity::Info => crate::notifications::NotificationType::Info,
            };

            // Создаем уведомление
            let notification = crate::notifications::Notification::new(
                notification_type,
                format!("Threat Detected: {}", threat.threat_type),
                threat.description.clone(),
            )
            .with_details(threat.details.clone().unwrap_or_default());

            // Отправляем уведомление
            notifier.send_notification(&notification).await?;

            tracing::info!("Sent threat notification for threat: {}", threat.threat_id);
        }

        Ok(())
    }

    /// Определить общий статус угроз.
    fn determine_overall_status(&self, threat_system: &ThreatDetectionSystem) -> ThreatSystemStatus {
        let mut has_critical = false;
        let mut has_high = false;
        let mut has_medium = false;

        for threat in &threat_system.threat_history {
            if threat.status == ThreatStatus::New
                || threat.status == ThreatStatus::Analyzing
            {
                match threat.severity {
                    ThreatDetectionSeverity::Critical => has_critical = true,
                    ThreatDetectionSeverity::High => has_high = true,
                    ThreatDetectionSeverity::Medium => has_medium = true,
                    _ => {}
                }
            }
        }

        if has_critical {
            ThreatSystemStatus::CriticalThreat
        } else if has_high {
            ThreatSystemStatus::PotentialThreat
        } else if has_medium {
            ThreatSystemStatus::Warning
        } else {
            ThreatSystemStatus::Secure
        }
    }

    /// Рассчитать балл безопасности.
    fn calculate_security_score(&self, threat_system: &ThreatDetectionSystem) -> f32 {
        // Начинаем с максимального балла
        let mut score = 100.0;

        // Учитываем неразрешенные угрозы
        let unresolved_threats = threat_system
            .threat_history
            .iter()
            .filter(|threat| {
                threat.status == ThreatStatus::New
                    || threat.status == ThreatStatus::Analyzing
            })
            .count();

        // Каждая неразрешенная угроза снижает балл
        for threat in &threat_system.threat_history {
            if threat.status == ThreatStatus::New
                || threat.status == ThreatStatus::Analyzing
            {
                match threat.severity {
                    ThreatDetectionSeverity::Critical => score -= 20.0,
                    ThreatDetectionSeverity::High => score -= 10.0,
                    ThreatDetectionSeverity::Medium => score -= 5.0,
                    ThreatDetectionSeverity::Low => score -= 2.0,
                    ThreatDetectionSeverity::Info => score -= 1.0,
                }
            }
        }

        // Учитываем количество угроз
        score -= unresolved_threats as f32 * 0.5;

        // Ограничиваем балл в диапазоне 0-100
        score = score.clamp(0.0, 100.0);

        score
    }

    /// Обновить историю баллов безопасности.
    fn update_security_score_history(&self, threat_system: &mut ThreatDetectionSystem) {
        let score = self.calculate_security_score(threat_system);
        threat_system.security_score = score;

        let entry = ThreatSecurityScoreEntry {
            timestamp: Utc::now(),
            score,
            status: threat_system.overall_status,
        };

        threat_system.security_score_history.push(entry);

        // Ограничиваем историю (например, 100 записей)
        if threat_system.security_score_history.len() > 100 {
            threat_system.security_score_history.remove(0);
        }
    }
}

/// Вспомогательная функция для создания ThreatDetectionSystem.
pub fn create_threat_detection_system(config: ThreatDetectionConfig) -> ThreatDetectionSystemImpl {
    ThreatDetectionSystemImpl::new(config)
}

/// Вспомогательная функция для создания ThreatDetectionSystem с конфигурацией по умолчанию.
pub fn create_default_threat_detection_system() -> ThreatDetectionSystemImpl {
    ThreatDetectionSystemImpl::new_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_threat_detection_system_creation() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        let status = system.get_threat_status().await.unwrap();
        assert_eq!(status.overall_status, ThreatSystemStatus::Unknown);
        assert_eq!(status.threat_history.len(), 0);
    }

    #[tokio::test]
    async fn test_add_threat_detection() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        let threat = ThreatDetection {
            threat_id: "test-threat-1".to_string(),
            timestamp: Utc::now(),
            threat_type: ThreatType::BehavioralAnomaly,
            severity: ThreatDetectionSeverity::High,
            status: ThreatStatus::New,
            process_name: Some("test_process".to_string()),
            process_id: Some(1234),
            description: "Test behavioral anomaly".to_string(),
            details: Some("Test details".to_string()),
            confidence_score: 85.0,
            recommendations: Some("Test recommendations".to_string()),
            resolved_time: None,
        };

        system.add_threat_detection(threat).await.unwrap();

        let status = system.get_threat_status().await.unwrap();
        assert_eq!(status.threat_history.len(), 1);
        assert_eq!(status.threat_history[0].threat_id, "test-threat-1");
    }

    #[tokio::test]
    async fn test_resolve_threat_detection() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        let threat = ThreatDetection {
            threat_id: "test-threat-2".to_string(),
            timestamp: Utc::now(),
            threat_type: ThreatType::NetworkAnomaly,
            severity: ThreatDetectionSeverity::High,
            status: ThreatStatus::New,
            process_name: Some("test_process".to_string()),
            process_id: Some(1234),
            description: "Test network anomaly".to_string(),
            details: Some("Test details".to_string()),
            confidence_score: 90.0,
            recommendations: Some("Test recommendations".to_string()),
            resolved_time: None,
        };

        system.add_threat_detection(threat).await.unwrap();
        system
            .resolve_threat_detection("test-threat-2")
            .await
            .unwrap();

        let status = system.get_threat_status().await.unwrap();
        assert_eq!(status.threat_history.len(), 1);
        assert_eq!(
            status.threat_history[0].status,
            ThreatStatus::Analyzed
        );
    }

    #[tokio::test]
    async fn test_mark_threat_as_false_positive() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        let threat = ThreatDetection {
            threat_id: "test-threat-3".to_string(),
            timestamp: Utc::now(),
            threat_type: ThreatType::FilesystemAnomaly,
            severity: ThreatDetectionSeverity::Medium,
            status: ThreatStatus::New,
            process_name: Some("test_process".to_string()),
            process_id: Some(1234),
            description: "Test filesystem anomaly".to_string(),
            details: Some("Test details".to_string()),
            confidence_score: 75.0,
            recommendations: Some("Test recommendations".to_string()),
            resolved_time: None,
        };

        system.add_threat_detection(threat).await.unwrap();
        system
            .mark_threat_as_false_positive("test-threat-3")
            .await
            .unwrap();

        let status = system.get_threat_status().await.unwrap();
        assert_eq!(status.threat_history.len(), 1);
        assert_eq!(
            status.threat_history[0].status,
            ThreatStatus::FalsePositive
        );
    }

    #[tokio::test]
    async fn test_threat_stats() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        let stats = system.get_threat_stats().await.unwrap();
        assert_eq!(stats.total_threats, 0);
        assert_eq!(stats.critical_threats, 0);
        assert_eq!(stats.high_threats, 0);
        assert_eq!(stats.medium_threats, 0);
        assert_eq!(stats.low_threats, 0);
        assert_eq!(stats.confirmed_threats, 0);
        assert_eq!(stats.false_positives, 0);
        assert_eq!(stats.average_confidence_score, 0.0);
    }

    #[tokio::test]
    async fn test_threat_type_display() {
        // Проверяем отображение типов угроз
        assert_eq!(format!("{}", ThreatType::BehavioralAnomaly), "behavioral_anomaly");
        assert_eq!(format!("{}", ThreatType::NetworkAnomaly), "network_anomaly");
        assert_eq!(format!("{}", ThreatType::FilesystemAnomaly), "filesystem_anomaly");
        assert_eq!(format!("{}", ThreatType::ResourceAnomaly), "resource_anomaly");
        assert_eq!(format!("{}", ThreatType::PotentialAttack), "potential_attack");
        assert_eq!(format!("{}", ThreatType::BruteForceAttack), "brute_force_attack");
        assert_eq!(format!("{}", ThreatType::SqlInjection), "sql_injection");
        assert_eq!(format!("{}", ThreatType::XssAttack), "xss_attack");
        assert_eq!(format!("{}", ThreatType::MitmAttack), "mitm_attack");
        assert_eq!(format!("{}", ThreatType::RansomwareActivity), "ransomware_activity");
        assert_eq!(format!("{}", ThreatType::BotnetActivity), "botnet_activity");
        assert_eq!(format!("{}", ThreatType::DataExfiltration), "data_exfiltration");
        assert_eq!(format!("{}", ThreatType::Cryptojacking), "cryptojacking");
        assert_eq!(format!("{}", ThreatType::PhishingActivity), "phishing_activity");
        assert_eq!(format!("{}", ThreatType::MalwareCommunication), "malware_communication");
        assert_eq!(format!("{}", ThreatType::DnsTunneling), "dns_tunneling");
        assert_eq!(format!("{}", ThreatType::IcmpTunneling), "icmp_tunneling");
        assert_eq!(format!("{}", ThreatType::HttpTunneling), "http_tunneling");
        assert_eq!(format!("{}", ThreatType::ProtocolAnomaly), "protocol_anomaly");
        assert_eq!(format!("{}", ThreatType::EncryptionAnomaly), "encryption_anomaly");
        assert_eq!(format!("{}", ThreatType::AuthenticationFailure), "authentication_failure");
        assert_eq!(format!("{}", ThreatType::ZeroDayExploit), "zero_day_exploit");
        assert_eq!(format!("{}", ThreatType::AptActivity), "apt_activity");
    }

    #[tokio::test]
    async fn test_security_score_calculation() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        // Создаем тестовый ThreatDetectionSystem с угрозами
        let mut threat_system = ThreatDetectionSystem::default();
        
        // Добавляем несколько угроз
        threat_system.threat_history.push(ThreatDetection {
            threat_id: "test-1".to_string(),
            timestamp: Utc::now(),
            threat_type: ThreatType::BehavioralAnomaly,
            severity: ThreatDetectionSeverity::Critical,
            status: ThreatStatus::New,
            process_name: None,
            process_id: None,
            description: "Critical threat".to_string(),
            details: None,
            confidence_score: 95.0,
            recommendations: None,
            resolved_time: None,
        });
        
        threat_system.threat_history.push(ThreatDetection {
            threat_id: "test-2".to_string(),
            timestamp: Utc::now(),
            threat_type: ThreatType::NetworkAnomaly,
            severity: ThreatDetectionSeverity::High,
            status: ThreatStatus::New,
            process_name: None,
            process_id: None,
            description: "High threat".to_string(),
            details: None,
            confidence_score: 85.0,
            recommendations: None,
            resolved_time: None,
        });
        
        threat_system.threat_history.push(ThreatDetection {
            threat_id: "test-3".to_string(),
            timestamp: Utc::now(),
            threat_type: ThreatType::FilesystemAnomaly,
            severity: ThreatDetectionSeverity::Medium,
            status: ThreatStatus::New,
            process_name: None,
            process_id: None,
            description: "Medium threat".to_string(),
            details: None,
            confidence_score: 75.0,
            recommendations: None,
            resolved_time: None,
        });

        // Рассчитываем балл безопасности
        let score = system.calculate_security_score(&threat_system);
        
        // Проверяем, что балл рассчитан корректно
        // Ожидаем: 100 - 20 (critical) - 10 (high) - 5 (medium) - 1.5 (3 threats * 0.5) = 63.5
        let expected_score = 100.0 - 20.0 - 10.0 - 5.0 - 1.5;
        assert_eq!(score, expected_score);
    }

    #[tokio::test]
    async fn test_notification_thresholds() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        // Проверяем пороги уведомлений
        let critical_threat = ThreatDetection {
            threat_id: "test-critical".to_string(),
            timestamp: Utc::now(),
            threat_type: ThreatType::PotentialAttack,
            severity: ThreatDetectionSeverity::Critical,
            status: ThreatStatus::New,
            process_name: None,
            process_id: None,
            description: "Critical test threat".to_string(),
            details: None,
            confidence_score: 95.0,
            recommendations: None,
            resolved_time: None,
        };

        let medium_threat = ThreatDetection {
            threat_id: "test-medium".to_string(),
            timestamp: Utc::now(),
            threat_type: ThreatType::ResourceAnomaly,
            severity: ThreatDetectionSeverity::Medium,
            status: ThreatStatus::New,
            process_name: None,
            process_id: None,
            description: "Medium test threat".to_string(),
            details: None,
            confidence_score: 70.0,
            recommendations: None,
            resolved_time: None,
        };

        let low_threat = ThreatDetection {
            threat_id: "test-low".to_string(),
            timestamp: Utc::now(),
            threat_type: ThreatType::BehavioralAnomaly,
            severity: ThreatDetectionSeverity::Low,
            status: ThreatStatus::New,
            process_name: None,
            process_id: None,
            description: "Low test threat".to_string(),
            details: None,
            confidence_score: 50.0,
            recommendations: None,
            resolved_time: None,
        };

        // Проверяем пороги уведомлений
        assert!(system.should_send_notification_for_threat(&critical_threat));
        assert!(system.should_send_notification_for_threat(&medium_threat));
        assert!(!system.should_send_notification_for_threat(&low_threat));
    }

    #[tokio::test]
    async fn test_ml_model_management() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        // Проверяем, что модель отсутствует изначально
        let model = system.get_ml_model().await.unwrap();
        assert!(model.is_none());

        // Создаем тестовую модель
        let test_model = MLThreatDetectionModel {
            version: "test-1.0".to_string(),
            training_time: Utc::now(),
            accuracy: 95.0,
            parameters: HashMap::new(),
            statistics: MLModelStatistics::default(),
        };

        // Обновляем модель
        system.update_ml_model(test_model.clone()).await.unwrap();

        // Проверяем, что модель обновлена
        let updated_model = system.get_ml_model().await.unwrap();
        assert!(updated_model.is_some());
        let model = updated_model.unwrap();
        assert_eq!(model.version, "test-1.0");
        assert_eq!(model.accuracy, 95.0);
    }

    #[tokio::test]
    async fn test_ml_model_training() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        // Выполняем обучение модели (должно завершиться успешно)
        let result = system.train_ml_model().await;
        assert!(result.is_ok());

        let model = result.unwrap();
        assert_eq!(model.version, "1.0");
        assert!(model.accuracy >= 0.0);
    }

    #[tokio::test]
    async fn test_comprehensive_threat_detection() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        // Создаем тестовый ThreatDetectionSystem
        let threat_system = ThreatDetectionSystem::default();

        // Выполняем проверку угроз (должно завершиться успешно)
        let result = system.check_threats().await;
        assert!(result.is_ok());

        let updated_system = result.unwrap();
        
        // Проверяем, что статус угроз определен
        assert_ne!(updated_system.overall_status, ThreatSystemStatus::Unknown);
         
        // Проверяем, что балл безопасности рассчитан
        assert!(updated_system.security_score >= 0.0 && updated_system.security_score <= 100.0);
    }

    #[tokio::test]
    async fn test_network_anomaly_detection_integration() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        // Создаем тестовый ThreatDetectionSystem
        let mut threat_system = ThreatDetectionSystem::default();

        // Выполняем обнаружение сетевых аномалий
        let result = system.detect_network_anomalies(&mut threat_system).await;
        assert!(result.is_ok());

        // Проверяем, что угроза добавлена
        let status = system.get_threat_status().await.unwrap();
        assert!(!status.threat_history.is_empty());
        
        // Проверяем, что обнаружены сетевые угрозы
        let network_threats: Vec<&ThreatDetection> = status
            .threat_history
            .iter()
            .filter(|t| t.threat_type == ThreatType::NetworkAnomaly)
            .collect();
        assert!(!network_threats.is_empty(), "Should detect network anomalies");
        
        // Проверяем свойства сетевой угрозы
        let network_threat = network_threats[0];
        assert_eq!(network_threat.severity, ThreatDetectionSeverity::High);
        assert!(network_threat.confidence_score > 70.0);
        assert!(network_threat.description.contains("network"));
    }

    #[tokio::test]
    async fn test_filesystem_anomaly_detection_integration() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        // Создаем тестовый ThreatDetectionSystem
        let mut threat_system = ThreatDetectionSystem::default();

        // Выполняем обнаружение аномалий файловой системы
        let result = system.detect_filesystem_anomalies(&mut threat_system).await;
        assert!(result.is_ok());

        // Проверяем, что угроза добавлена
        let status = system.get_threat_status().await.unwrap();
        assert!(!status.threat_history.is_empty());
        
        // Проверяем, что обнаружены аномалии файловой системы
        let filesystem_threats: Vec<&ThreatDetection> = status
            .threat_history
            .iter()
            .filter(|t| t.threat_type == ThreatType::FilesystemAnomaly)
            .collect();
        assert!(!filesystem_threats.is_empty(), "Should detect filesystem anomalies");
        
        // Проверяем свойства угрозы файловой системы
        let filesystem_threat = filesystem_threats[0];
        assert_eq!(filesystem_threat.severity, ThreatDetectionSeverity::Medium);
        assert!(filesystem_threat.confidence_score > 50.0);
        assert!(filesystem_threat.description.contains("filesystem"));
    }

    #[tokio::test]
    async fn test_resource_anomaly_detection_integration() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        // Создаем тестовый ThreatDetectionSystem
        let mut threat_system = ThreatDetectionSystem::default();

        // Выполняем обнаружение аномалий использования ресурсов
        let result = system.detect_resource_anomalies(&mut threat_system).await;
        assert!(result.is_ok());

        // Проверяем, что угроза добавлена
        let status = system.get_threat_status().await.unwrap();
        assert!(!status.threat_history.is_empty());
        
        // Проверяем, что обнаружены аномалии использования ресурсов
        let resource_threats: Vec<&ThreatDetection> = status
            .threat_history
            .iter()
            .filter(|t| t.threat_type == ThreatType::ResourceAnomaly)
            .collect();
        assert!(!resource_threats.is_empty(), "Should detect resource anomalies");
        
        // Проверяем свойства угрозы использования ресурсов
        let resource_threat = resource_threats[0];
        assert_eq!(resource_threat.severity, ThreatDetectionSeverity::High);
        assert!(resource_threat.confidence_score > 80.0);
        assert!(resource_threat.description.contains("resource"));
    }

    #[tokio::test]
    async fn test_behavioral_anomaly_detection_integration() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        // Создаем тестовый ThreatDetectionSystem
        let mut threat_system = ThreatDetectionSystem::default();

        // Выполняем обнаружение поведенческих аномалий
        let result = system.detect_behavioral_anomalies(&mut threat_system).await;
        assert!(result.is_ok());

        // Проверяем, что угроза добавлена
        let status = system.get_threat_status().await.unwrap();
        assert!(!status.threat_history.is_empty());
        
        // Проверяем, что обнаружены поведенческие аномалии
        let behavioral_threats: Vec<&ThreatDetection> = status
            .threat_history
            .iter()
            .filter(|t| t.threat_type == ThreatType::BehavioralAnomaly)
            .collect();
        assert!(!behavioral_threats.is_empty(), "Should detect behavioral anomalies");
        
        // Проверяем свойства поведенческой угрозы
        let behavioral_threat = behavioral_threats[0];
        assert_eq!(behavioral_threat.severity, ThreatDetectionSeverity::Critical);
        assert!(behavioral_threat.confidence_score > 90.0);
        assert!(behavioral_threat.description.contains("behavioral"));
    }

    #[tokio::test]
    async fn test_ml_threat_detection_integration() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        // Создаем тестовый ThreatDetectionSystem
        let mut threat_system = ThreatDetectionSystem::default();

        // Выполняем ML-базированное обнаружение угроз
        let result = system.perform_ml_threat_detection(&mut threat_system).await;
        assert!(result.is_ok());

        // Проверяем, что угрозы обнаружены
        let status = system.get_threat_status().await.unwrap();
        assert!(!status.threat_history.is_empty());
        
        // Проверяем, что обнаружены различные типы угроз
        let threat_types: Vec<ThreatType> = status
            .threat_history
            .iter()
            .map(|t| t.threat_type)
            .collect();
        
        assert!(threat_types.contains(&ThreatType::NetworkAnomaly));
        assert!(threat_types.contains(&ThreatType::FilesystemAnomaly));
        assert!(threat_types.contains(&ThreatType::ResourceAnomaly));
        assert!(threat_types.contains(&ThreatType::BehavioralAnomaly));
    }

    #[tokio::test]
    async fn test_advanced_threat_analysis_integration() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        // Создаем тестовый ThreatDetectionSystem
        let mut threat_system = ThreatDetectionSystem::default();

        // Выполняем продвинутый анализ угроз
        let result = system.perform_advanced_threat_analysis(&mut threat_system).await;
        assert!(result.is_ok());

        // Проверяем, что анализ завершился успешно
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_comprehensive_threat_detection_with_all_modules() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        // Создаем тестовый ThreatDetectionSystem
        let threat_system = ThreatDetectionSystem::default();

        // Выполняем полную проверку угроз
        let result = system.check_threats().await;
        assert!(result.is_ok());

        let updated_system = result.unwrap();
        
        // Проверяем, что обнаружены угрозы
        assert!(!updated_system.threat_history.is_empty());
        
        // Проверяем, что обнаружены различные типы угроз
        let threat_types: Vec<ThreatType> = updated_system
            .threat_history
            .iter()
            .map(|t| t.threat_type)
            .collect();
        
        assert!(threat_types.contains(&ThreatType::NetworkAnomaly));
        assert!(threat_types.contains(&ThreatType::FilesystemAnomaly));
        assert!(threat_types.contains(&ThreatType::ResourceAnomaly));
        assert!(threat_types.contains(&ThreatType::BehavioralAnomaly));
        
        // Проверяем, что статус угроз определен
        assert_ne!(updated_system.overall_status, ThreatSystemStatus::Unknown);
         
        // Проверяем, что балл безопасности рассчитан
        assert!(updated_system.security_score >= 0.0 && updated_system.security_score <= 100.0);
        
        // Проверяем, что балл безопасности снижен из-за обнаруженных угроз
        assert!(updated_system.security_score < 100.0, "Security score should be reduced due to detected threats");
    }

    #[tokio::test]
    async fn test_threat_detection_with_ml_disabled() {
        let mut config = ThreatDetectionConfig::default();
        config.ml_settings.enabled = false;
        
        let system = ThreatDetectionSystemImpl::new(config);

        // Создаем тестовый ThreatDetectionSystem
        let threat_system = ThreatDetectionSystem::default();

        // Выполняем проверку угроз с отключенным ML
        let result = system.check_threats().await;
        assert!(result.is_ok());

        let updated_system = result.unwrap();
        
        // Проверяем, что базовое обнаружение угроз все равно работает
        assert_ne!(updated_system.overall_status, ThreatSystemStatus::Unknown);
        assert!(updated_system.security_score >= 0.0 && updated_system.security_score <= 100.0);
    }

    #[tokio::test]
    async fn test_threat_detection_statistics() {
        let config = ThreatDetectionConfig::default();
        let system = ThreatDetectionSystemImpl::new(config);

        // Создаем тестовый ThreatDetectionSystem
        let threat_system = ThreatDetectionSystem::default();

        // Выполняем проверку угроз
        let result = system.check_threats().await;
        assert!(result.is_ok());

        // Получаем статистику угроз
        let stats = system.get_threat_stats().await.unwrap();
        
        // Проверяем, что статистика обновлена
        assert!(stats.total_threats > 0);
        assert!(stats.average_confidence_score > 0.0);
        assert!(stats.last_threat_time.is_some());
        
        // Проверяем, что статистика содержит информацию о различных уровнях угроз
        assert!(stats.critical_threats > 0 || stats.high_threats > 0 || stats.medium_threats > 0);
    }
}