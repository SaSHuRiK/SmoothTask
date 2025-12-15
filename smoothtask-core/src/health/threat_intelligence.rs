//! Модуль интеграции с внешними источниками угроз (Threat Intelligence).
//!
//! Этот модуль предоставляет функциональность для загрузки, кэширования и использования
//! данных об угрозах из внешних источников для улучшения обнаружения безопасности.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Тип угрозы.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatType {
    /// Вредоносное ПО
    #[serde(rename = "malware")]
    Malware,
    /// Фишинг
    #[serde(rename = "phishing")]
    Phishing,
    /// Ботнет
    #[serde(rename = "botnet")]
    Botnet,
    /// Эксплойт
    #[serde(rename = "exploit")]
    Exploit,
    /// Руткит
    #[serde(rename = "rootkit")]
    Rootkit,
    /// Шпионское ПО
    #[serde(rename = "spyware")]
    Spyware,
    /// Рекламное ПО
    #[serde(rename = "adware")]
    Adware,
    /// Майнинг
    #[serde(rename = "mining")]
    Mining,
    /// Неизвестный тип
    #[serde(rename = "unknown")]
    Unknown,
}

impl Default for ThreatType {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for ThreatType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreatType::Malware => write!(f, "malware"),
            ThreatType::Phishing => write!(f, "phishing"),
            ThreatType::Botnet => write!(f, "botnet"),
            ThreatType::Exploit => write!(f, "exploit"),
            ThreatType::Rootkit => write!(f, "rootkit"),
            ThreatType::Spyware => write!(f, "spyware"),
            ThreatType::Adware => write!(f, "adware"),
            ThreatType::Mining => write!(f, "mining"),
            ThreatType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Уровень опасности угрозы.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatSeverity {
    /// Низкий уровень опасности
    #[serde(rename = "low")]
    Low,
    /// Средний уровень опасности
    #[serde(rename = "medium")]
    Medium,
    /// Высокий уровень опасности
    #[serde(rename = "high")]
    High,
    /// Критический уровень опасности
    #[serde(rename = "critical")]
    Critical,
}

impl Default for ThreatSeverity {
    fn default() -> Self {
        Self::Medium
    }
}

impl std::fmt::Display for ThreatSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreatSeverity::Low => write!(f, "low"),
            ThreatSeverity::Medium => write!(f, "medium"),
            ThreatSeverity::High => write!(f, "high"),
            ThreatSeverity::Critical => write!(f, "critical"),
        }
    }
}

/// Информация об угрозе.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatIntel {
    /// Уникальный идентификатор угрозы
    pub threat_id: String,
    /// Тип угрозы
    pub threat_type: ThreatType,
    /// Уровень опасности
    pub severity: ThreatSeverity,
    /// Описание угрозы
    pub description: String,
    /// Индикаторы компрометации (IoCs)
    pub indicators: Vec<ThreatIndicator>,
    /// Время добавления угрозы
    pub added_time: DateTime<Utc>,
    /// Время последнего обновления
    pub last_updated: DateTime<Utc>,
    /// Источник угрозы
    pub source: String,
    /// Ссылка на дополнительную информацию
    pub reference: Option<String>,
}

impl Default for ThreatIntel {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            threat_id: uuid::Uuid::new_v4().to_string(),
            threat_type: ThreatType::Unknown,
            severity: ThreatSeverity::Medium,
            description: String::new(),
            indicators: Vec::new(),
            added_time: now,
            last_updated: now,
            source: String::new(),
            reference: None,
        }
    }
}

/// Тип индикатора угрозы.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatIndicatorType {
    /// IP адрес
    #[serde(rename = "ip")]
    Ip,
    /// Доменное имя
    #[serde(rename = "domain")]
    Domain,
    /// URL
    #[serde(rename = "url")]
    Url,
    /// Хэш файла (MD5, SHA1, SHA256)
    #[serde(rename = "hash")]
    Hash,
    /// Имя процесса
    #[serde(rename = "process_name")]
    ProcessName,
    /// Путь к файлу
    #[serde(rename = "file_path")]
    FilePath,
    /// Имя пользователя
    #[serde(rename = "username")]
    Username,
    /// Идентификатор пользователя
    #[serde(rename = "user_id")]
    UserId,
    /// Регистрационный ключ
    #[serde(rename = "registry_key")]
    RegistryKey,
    /// Неизвестный тип
    #[serde(rename = "unknown")]
    Unknown,
}

impl Default for ThreatIndicatorType {
    fn default() -> Self {
        Self::Unknown
    }
}

impl std::fmt::Display for ThreatIndicatorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreatIndicatorType::Ip => write!(f, "ip"),
            ThreatIndicatorType::Domain => write!(f, "domain"),
            ThreatIndicatorType::Url => write!(f, "url"),
            ThreatIndicatorType::Hash => write!(f, "hash"),
            ThreatIndicatorType::ProcessName => write!(f, "process_name"),
            ThreatIndicatorType::FilePath => write!(f, "file_path"),
            ThreatIndicatorType::Username => write!(f, "username"),
            ThreatIndicatorType::UserId => write!(f, "user_id"),
            ThreatIndicatorType::RegistryKey => write!(f, "registry_key"),
            ThreatIndicatorType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Индикатор угрозы (Indicator of Compromise - IoC).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatIndicator {
    /// Тип индикатора
    pub indicator_type: ThreatIndicatorType,
    /// Значение индикатора
    pub value: String,
    /// Уровень уверенности (0.0 - 1.0)
    pub confidence: f32,
    /// Время последнего обнаружения
    pub last_seen: Option<DateTime<Utc>>,
    /// Дополнительные метки
    pub tags: Vec<String>,
}

impl Default for ThreatIndicator {
    fn default() -> Self {
        Self {
            indicator_type: ThreatIndicatorType::Unknown,
            value: String::new(),
            confidence: 0.8,
            last_seen: None,
            tags: Vec::new(),
        }
    }
}

/// Конфигурация источника угроз.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatFeedConfig {
    /// Уникальный идентификатор источника
    pub feed_id: String,
    /// Название источника
    pub name: String,
    /// URL источника
    pub url: String,
    /// Формат данных
    pub format: ThreatFeedFormat,
    /// Интервал обновления
    pub update_interval: Duration,
    /// Включен ли источник
    pub enabled: bool,
    /// Уровень доверия к источнику (0.0 - 1.0)
    pub trust_level: f32,
    /// API ключ (если требуется)
    pub api_key: Option<String>,
    /// Заголовки HTTP (если требуются)
    pub headers: Option<Vec<(String, String)>>,
}

impl Default for ThreatFeedConfig {
    fn default() -> Self {
        Self {
            feed_id: uuid::Uuid::new_v4().to_string(),
            name: String::new(),
            url: String::new(),
            format: ThreatFeedFormat::Json,
            update_interval: Duration::from_secs(3600), // 1 час
            enabled: true,
            trust_level: 0.9,
            api_key: None,
            headers: None,
        }
    }
}

/// Формат данных источника угроз.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatFeedFormat {
    /// JSON формат
    #[serde(rename = "json")]
    Json,
    /// CSV формат
    #[serde(rename = "csv")]
    Csv,
    /// TXT формат
    #[serde(rename = "txt")]
    Txt,
    /// STIX формат
    #[serde(rename = "stix")]
    Stix,
    /// Неизвестный формат
    #[serde(rename = "unknown")]
    Unknown,
}

impl Default for ThreatFeedFormat {
    fn default() -> Self {
        Self::Json
    }
}

impl std::fmt::Display for ThreatFeedFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreatFeedFormat::Json => write!(f, "json"),
            ThreatFeedFormat::Csv => write!(f, "csv"),
            ThreatFeedFormat::Txt => write!(f, "txt"),
            ThreatFeedFormat::Stix => write!(f, "stix"),
            ThreatFeedFormat::Unknown => write!(f, "unknown"),
        }
    }
}

/// Конфигурация системы угроз.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatIntelligenceConfig {
    /// Включена ли система угроз
    pub enabled: bool,
    /// Интервал обновления всех источников
    pub global_update_interval: Duration,
    /// Максимальное количество хранимых угроз
    pub max_threats: usize,
    /// Путь к кэшу угроз
    pub cache_path: PathBuf,
    /// Время жизни кэша
    pub cache_ttl: Duration,
    /// Источники угроз
    pub feeds: Vec<ThreatFeedConfig>,
    /// Настройки интеграции с мониторингом безопасности
    pub security_integration: ThreatSecurityIntegration,
}

impl Default for ThreatIntelligenceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            global_update_interval: Duration::from_secs(3600), // 1 час
            max_threats: 10000,
            cache_path: PathBuf::from("/var/cache/smoothtask/threat_intel"),
            cache_ttl: Duration::from_secs(86400), // 24 часа
            feeds: Vec::new(),
            security_integration: ThreatSecurityIntegration::default(),
        }
    }
}

/// Настройки интеграции с мониторингом безопасности.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatSecurityIntegration {
    /// Включена ли интеграция с мониторингом безопасности
    pub enabled: bool,
    /// Минимальный уровень опасности для создания событий безопасности
    pub min_security_event_severity: crate::health::security_monitoring::SecurityEventSeverity,
    /// Создавать события безопасности для обнаруженных угроз
    pub create_security_events: bool,
    /// Автоматически блокировать известные угрозы
    pub auto_block_threats: bool,
}

impl Default for ThreatSecurityIntegration {
    fn default() -> Self {
        Self {
            enabled: true,
            min_security_event_severity:
                crate::health::security_monitoring::SecurityEventSeverity::High,
            create_security_events: true,
            auto_block_threats: false,
        }
    }
}

/// Основная структура для управления угрозами.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ThreatIntelligenceManager {
    /// Время последнего обновления
    pub last_update_time: Option<DateTime<Utc>>,
    /// Общее количество угроз
    pub total_threats: usize,
    /// Количество активных угроз
    pub active_threats: usize,
    /// Статус системы угроз
    pub status: ThreatIntelligenceStatus,
    /// Конфигурация системы угроз
    pub config: ThreatIntelligenceConfig,
    /// История обновлений
    pub update_history: Vec<ThreatUpdateHistory>,
}

/// Статус системы угроз.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatIntelligenceStatus {
    /// Система не инициализирована
    #[serde(rename = "not_initialized")]
    NotInitialized,
    /// Система инициализирована
    #[serde(rename = "initialized")]
    Initialized,
    /// Обновление в процессе
    #[serde(rename = "updating")]
    Updating,
    /// Система готова
    #[serde(rename = "ready")]
    Ready,
    /// Ошибка
    #[serde(rename = "error")]
    Error,
}

impl Default for ThreatIntelligenceStatus {
    fn default() -> Self {
        Self::NotInitialized
    }
}

/// История обновлений угроз.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatUpdateHistory {
    /// Время обновления
    pub timestamp: DateTime<Utc>,
    /// Количество добавленных угроз
    pub threats_added: usize,
    /// Количество обновленных угроз
    pub threats_updated: usize,
    /// Количество удаленных угроз
    pub threats_removed: usize,
    /// Общее количество угроз после обновления
    pub total_threats_after: usize,
    /// Статус обновления
    pub status: ThreatUpdateStatus,
    /// Сообщение об ошибке (если есть)
    pub error_message: Option<String>,
}

impl Default for ThreatUpdateHistory {
    fn default() -> Self {
        Self {
            timestamp: Utc::now(),
            threats_added: 0,
            threats_updated: 0,
            threats_removed: 0,
            total_threats_after: 0,
            status: ThreatUpdateStatus::Success,
            error_message: None,
        }
    }
}

/// Статус обновления угроз.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThreatUpdateStatus {
    /// Успешно
    #[serde(rename = "success")]
    Success,
    /// Частично успешно
    #[serde(rename = "partial_success")]
    PartialSuccess,
    /// Ошибка
    #[serde(rename = "error")]
    Error,
    /// Прервано
    #[serde(rename = "cancelled")]
    Cancelled,
}

impl Default for ThreatUpdateStatus {
    fn default() -> Self {
        Self::Success
    }
}

/// Статистика угроз.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThreatStats {
    /// Общее количество угроз
    pub total_threats: usize,
    /// Количество угроз по типам
    pub threats_by_type: HashMap<ThreatType, usize>,
    /// Количество угроз по уровню опасности
    pub threats_by_severity: HashMap<ThreatSeverity, usize>,
    /// Количество индикаторов по типам
    pub indicators_by_type: HashMap<ThreatIndicatorType, usize>,
    /// Время последнего обновления
    pub last_update_time: Option<DateTime<Utc>>,
    /// Количество источников
    pub active_feeds: usize,
}

impl Default for ThreatStats {
    fn default() -> Self {
        Self {
            total_threats: 0,
            threats_by_type: HashMap::new(),
            threats_by_severity: HashMap::new(),
            indicators_by_type: HashMap::new(),
            last_update_time: None,
            active_feeds: 0,
        }
    }
}

/// Интерфейс для управления угрозами.
#[async_trait::async_trait]
pub trait ThreatIntelligenceTrait: Send + Sync {
    /// Инициализировать систему угроз.
    async fn initialize(&self) -> Result<()>;

    /// Обновить угрозы из всех источников.
    async fn update_threats(&self) -> Result<ThreatUpdateHistory>;

    /// Обновить угрозы из конкретного источника.
    async fn update_threats_from_feed(&self, feed_id: &str) -> Result<ThreatUpdateHistory>;

    /// Получить текущее состояние системы угроз.
    async fn get_threat_intelligence_status(&self) -> Result<ThreatIntelligenceManager>;

    /// Поиск угроз по индикатору.
    async fn find_threats_by_indicator(
        &self,
        indicator_type: ThreatIndicatorType,
        value: &str,
    ) -> Result<Vec<ThreatIntel>>;

    /// Поиск угроз по типу.
    async fn find_threats_by_type(&self, threat_type: ThreatType) -> Result<Vec<ThreatIntel>>;

    /// Поиск угроз по уровню опасности.
    async fn find_threats_by_severity(&self, severity: ThreatSeverity) -> Result<Vec<ThreatIntel>>;

    /// Проверка, является ли индикатор известной угрозой.
    async fn is_known_threat(
        &self,
        indicator_type: ThreatIndicatorType,
        value: &str,
    ) -> Result<bool>;

    /// Получить все активные угрозы.
    async fn get_all_active_threats(&self) -> Result<Vec<ThreatIntel>>;

    /// Очистить кэш угроз.
    async fn clear_threat_cache(&self) -> Result<()>;

    /// Получить статистику угроз.
    async fn get_threat_stats(&self) -> Result<ThreatStats>;

    /// Интегрировать с системой мониторинга безопасности.
    async fn integrate_with_security_monitoring(
        &self,
        security_monitor: Arc<dyn crate::health::security_monitoring::SecurityMonitorTrait>,
    ) -> Result<()>;
}

/// Реализация ThreatIntelligenceTrait.
#[derive(Clone)]
pub struct ThreatIntelligenceImpl {
    threat_state: Arc<RwLock<ThreatIntelligenceManager>>,
    threat_database: Arc<RwLock<HashMap<String, ThreatIntel>>>,
    config: ThreatIntelligenceConfig,
    security_monitor: Option<Arc<dyn crate::health::security_monitoring::SecurityMonitorTrait>>,
}

#[async_trait::async_trait]
impl ThreatIntelligenceTrait for ThreatIntelligenceImpl {
    async fn initialize(&self) -> Result<()> {
        info!("Initializing Threat Intelligence system");

        let mut state = self.threat_state.write().await;
        state.status = ThreatIntelligenceStatus::Initializing;

        // Создаем директорию для кэша, если она не существует
        if let Err(e) = std::fs::create_dir_all(&self.config.cache_path) {
            error!(
                "Failed to create threat intelligence cache directory: {}",
                e
            );
            state.status = ThreatIntelligenceStatus::Error;
            return Err(anyhow::anyhow!("Failed to create cache directory: {}", e));
        }

        // Загружаем кэшированные угрозы, если они есть
        self.load_cached_threats().await?;

        state.status = ThreatIntelligenceStatus::Initialized;
        state.last_update_time = Some(Utc::now());

        info!("Threat Intelligence system initialized successfully");
        Ok(())
    }

    async fn update_threats(&self) -> Result<ThreatUpdateHistory> {
        info!("Starting threat intelligence update from all feeds");

        let mut state = self.threat_state.write().await;
        state.status = ThreatIntelligenceStatus::Updating;

        let mut total_added = 0;
        let mut total_updated = 0;
        let mut total_removed = 0;
        let mut success_count = 0;
        let mut error_count = 0;

        // Обновляем угрозы из каждого источника
        for feed in &self.config.feeds {
            if !feed.enabled {
                debug!("Skipping disabled feed: {}", feed.name);
                continue;
            }

            match self.update_threats_from_feed(&feed.feed_id).await {
                Ok(history) => {
                    total_added += history.threats_added;
                    total_updated += history.threats_updated;
                    total_removed += history.threats_removed;
                    success_count += 1;
                }
                Err(e) => {
                    error!("Failed to update threats from feed {}: {}", feed.name, e);
                    error_count += 1;
                }
            }
        }

        // Обновляем статистику
        let mut stats = self.get_threat_stats().await?;
        state.total_threats = stats.total_threats;
        state.active_threats = stats.total_threats;

        // Создаем запись в истории обновлений
        let update_status = if error_count == 0 {
            ThreatUpdateStatus::Success
        } else if success_count > 0 {
            ThreatUpdateStatus::PartialSuccess
        } else {
            ThreatUpdateStatus::Error
        };

        let history = ThreatUpdateHistory {
            timestamp: Utc::now(),
            threats_added: total_added,
            threats_updated: total_updated,
            threats_removed: total_removed,
            total_threats_after: stats.total_threats,
            status: update_status,
            error_message: if error_count > 0 {
                Some(format!("{} feeds failed to update", error_count))
            } else {
                None
            },
        };

        state.update_history.push(history.clone());
        state.last_update_time = Some(Utc::now());
        state.status = ThreatIntelligenceStatus::Ready;

        info!(
            "Threat intelligence update completed: {} threats added, {} updated, {} removed",
            total_added, total_updated, total_removed
        );

        Ok(history)
    }

    async fn update_threats_from_feed(&self, feed_id: &str) -> Result<ThreatUpdateHistory> {
        // Находим конфигурацию источника
        let feed_config = self
            .config
            .feeds
            .iter()
            .find(|feed| feed.feed_id == feed_id)
            .ok_or_else(|| anyhow::anyhow!("Feed not found: {}", feed_id))?;

        info!("Updating threats from feed: {}", feed_config.name);

        // Загружаем данные из источника
        let feed_data = self.fetch_feed_data(feed_config).await?;

        // Парсим данные
        let threats = self.parse_feed_data(feed_config, &feed_data).await?;

        // Обновляем базу данных угроз
        let history = self.update_threat_database(threats).await?;

        info!(
            "Successfully updated threats from feed {}: {} added, {} updated, {} removed",
            feed_config.name,
            history.threats_added,
            history.threats_updated,
            history.threats_removed
        );

        Ok(history)
    }

    async fn get_threat_intelligence_status(&self) -> Result<ThreatIntelligenceManager> {
        Ok(self.threat_state.read().await.clone())
    }

    async fn find_threats_by_indicator(
        &self,
        indicator_type: ThreatIndicatorType,
        value: &str,
    ) -> Result<Vec<ThreatIntel>> {
        let database = self.threat_database.read().await;
        let mut results = Vec::new();

        for threat in database.values() {
            for indicator in &threat.indicators {
                if indicator.indicator_type == indicator_type && indicator.value == value {
                    results.push(threat.clone());
                    break;
                }
            }
        }

        Ok(results)
    }

    async fn find_threats_by_type(&self, threat_type: ThreatType) -> Result<Vec<ThreatIntel>> {
        let database = self.threat_database.read().await;
        let results = database
            .values()
            .filter(|threat| threat.threat_type == threat_type)
            .cloned()
            .collect();

        Ok(results)
    }

    async fn find_threats_by_severity(&self, severity: ThreatSeverity) -> Result<Vec<ThreatIntel>> {
        let database = self.threat_database.read().await;
        let results = database
            .values()
            .filter(|threat| threat.severity == severity)
            .cloned()
            .collect();

        Ok(results)
    }

    async fn is_known_threat(
        &self,
        indicator_type: ThreatIndicatorType,
        value: &str,
    ) -> Result<bool> {
        let results = self
            .find_threats_by_indicator(indicator_type, value)
            .await?;
        Ok(!results.is_empty())
    }

    async fn get_all_active_threats(&self) -> Result<Vec<ThreatIntel>> {
        let database = self.threat_database.read().await;
        let results = database.values().cloned().collect();

        Ok(results)
    }

    async fn clear_threat_cache(&self) -> Result<()> {
        info!("Clearing threat intelligence cache");

        let mut database = self.threat_database.write().await;
        database.clear();

        let mut state = self.threat_state.write().await;
        state.total_threats = 0;
        state.active_threats = 0;
        state.last_update_time = None;

        // Очищаем файлы кэша
        if let Err(e) = std::fs::remove_dir_all(&self.config.cache_path) {
            warn!("Failed to remove cache directory: {}", e);
        }

        // Создаем директорию заново
        std::fs::create_dir_all(&self.config.cache_path)?;

        info!("Threat intelligence cache cleared successfully");
        Ok(())
    }

    async fn get_threat_stats(&self) -> Result<ThreatStats> {
        let database = self.threat_database.read().await;
        let mut stats = ThreatStats::default();

        stats.total_threats = database.len();
        stats.last_update_time = self.threat_state.read().await.last_update_time;
        stats.active_feeds = self.config.feeds.iter().filter(|feed| feed.enabled).count();

        // Подсчитываем угрозы по типам
        for threat in database.values() {
            *stats.threats_by_type.entry(threat.threat_type).or_insert(0) += 1;
            *stats
                .threats_by_severity
                .entry(threat.severity)
                .or_insert(0) += 1;

            for indicator in &threat.indicators {
                *stats
                    .indicators_by_type
                    .entry(indicator.indicator_type)
                    .or_insert(0) += 1;
            }
        }

        Ok(stats)
    }

    async fn integrate_with_security_monitoring(
        &self,
        security_monitor: Arc<dyn crate::health::security_monitoring::SecurityMonitorTrait>,
    ) -> Result<()> {
        info!("Integrating threat intelligence with security monitoring");
        self.security_monitor = Some(security_monitor);
        Ok(())
    }
}

impl ThreatIntelligenceImpl {
    /// Создать новый ThreatIntelligenceImpl.
    pub fn new(config: ThreatIntelligenceConfig) -> Self {
        Self {
            threat_state: Arc::new(RwLock::new(ThreatIntelligenceManager::default())),
            threat_database: Arc::new(RwLock::new(HashMap::new())),
            config,
            security_monitor: None,
        }
    }

    /// Создать новый ThreatIntelligenceImpl с конфигурацией по умолчанию.
    pub fn new_default() -> Self {
        Self::new(ThreatIntelligenceConfig::default())
    }

    /// Создать новый ThreatIntelligenceImpl с интеграцией безопасности.
    pub fn new_with_security_monitor(
        config: ThreatIntelligenceConfig,
        security_monitor: Arc<dyn crate::health::security_monitoring::SecurityMonitorTrait>,
    ) -> Self {
        Self {
            threat_state: Arc::new(RwLock::new(ThreatIntelligenceManager::default())),
            threat_database: Arc::new(RwLock::new(HashMap::new())),
            config,
            security_monitor: Some(security_monitor),
        }
    }

    /// Загрузить кэшированные угрозы.
    async fn load_cached_threats(&self) -> Result<()> {
        let cache_file = self.config.cache_path.join("threats.json");

        if cache_file.exists() {
            info!("Loading cached threats from {}", cache_file.display());

            let cache_content = std::fs::read_to_string(&cache_file)?;
            let cached_threats: Vec<ThreatIntel> = serde_json::from_str(&cache_content)?;

            let mut database = self.threat_database.write().await;
            for threat in cached_threats {
                database.insert(threat.threat_id.clone(), threat);
            }

            let mut state = self.threat_state.write().await;
            state.total_threats = database.len();
            state.active_threats = database.len();

            info!("Loaded {} threats from cache", database.len());
        } else {
            debug!("No cached threats found at {}", cache_file.display());
        }

        Ok(())
    }

    /// Сохранить угрозы в кэш.
    async fn save_cached_threats(&self) -> Result<()> {
        let cache_file = self.config.cache_path.join("threats.json");

        // Создаем директорию, если она не существует
        if let Some(parent) = cache_file.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let database = self.threat_database.read().await;
        let threats: Vec<ThreatIntel> = database.values().cloned().collect();

        let cache_content = serde_json::to_string_pretty(&threats)?;
        std::fs::write(&cache_file, cache_content)?;

        debug!("Saved {} threats to cache", threats.len());
        Ok(())
    }

    /// Загрузить данные из источника.
    async fn fetch_feed_data(&self, feed_config: &ThreatFeedConfig) -> Result<String> {
        info!("Fetching data from threat feed: {}", feed_config.name);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let mut request = client.get(&feed_config.url);

        // Добавляем заголовки, если они есть
        if let Some(headers) = &feed_config.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        // Добавляем API ключ, если он есть
        if let Some(api_key) = &feed_config.api_key {
            request = request.header("Authorization", format!("Bearer {}", api_key));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to fetch feed data: HTTP {}",
                response.status()
            ));
        }

        let body = response.text().await?;
        Ok(body)
    }

    /// Парсить данные из источника.
    async fn parse_feed_data(
        &self,
        feed_config: &ThreatFeedConfig,
        data: &str,
    ) -> Result<Vec<ThreatIntel>> {
        info!("Parsing threat feed data from: {}", feed_config.name);

        match feed_config.format {
            ThreatFeedFormat::Json => self.parse_json_feed(feed_config, data).await,
            ThreatFeedFormat::Csv => self.parse_csv_feed(feed_config, data).await,
            ThreatFeedFormat::Txt => self.parse_txt_feed(feed_config, data).await,
            ThreatFeedFormat::Stix => self.parse_stix_feed(feed_config, data).await,
            ThreatFeedFormat::Unknown => {
                warn!(
                    "Unknown feed format for {}, attempting JSON parsing",
                    feed_config.name
                );
                self.parse_json_feed(feed_config, data).await
            }
        }
    }

    /// Парсить JSON формат.
    async fn parse_json_feed(
        &self,
        feed_config: &ThreatFeedConfig,
        data: &str,
    ) -> Result<Vec<ThreatIntel>> {
        // Пытаемся парсить как массив угроз
        if let Ok(threats) = serde_json::from_str::<Vec<ThreatIntel>>(data) {
            return Ok(threats);
        }

        // Пытаемся парсить как объект с массивом угроз
        #[derive(Deserialize)]
        struct FeedWrapper {
            threats: Option<Vec<ThreatIntel>>,
            data: Option<Vec<ThreatIntel>>,
            items: Option<Vec<ThreatIntel>>,
        }

        if let Ok(wrapper) = serde_json::from_str::<FeedWrapper>(data) {
            if let Some(threats) = wrapper.threats {
                return Ok(threats);
            }
            if let Some(data) = wrapper.data {
                return Ok(data);
            }
            if let Some(items) = wrapper.items {
                return Ok(items);
            }
        }

        // Пытаемся парсить как простой список индикаторов
        #[derive(Deserialize)]
        struct SimpleIndicator {
            indicator: Option<String>,
            value: Option<String>,
            type_field: Option<String>,
            description: Option<String>,
        }

        if let Ok(indicators) = serde_json::from_str::<Vec<SimpleIndicator>>(data) {
            let mut threats = Vec::new();

            for indicator in indicators {
                let mut threat = ThreatIntel::default();
                threat.source = feed_config.name.clone();
                threat.last_updated = Utc::now();

                if let Some(indicator_value) = indicator.indicator.or(indicator.value) {
                    let indicator_type = indicator.type_field.unwrap_or_else(|| "ip".to_string());

                    let threat_indicator = ThreatIndicator {
                        indicator_type: self.parse_indicator_type(&indicator_type),
                        value: indicator_value,
                        confidence: feed_config.trust_level,
                        last_seen: None,
                        tags: vec![feed_config.name.clone()],
                    };

                    threat.indicators.push(threat_indicator);
                }

                if let Some(description) = indicator.description {
                    threat.description = description;
                }

                threats.push(threat);
            }

            return Ok(threats);
        }

        Err(anyhow::anyhow!("Failed to parse JSON feed data"))
    }

    /// Парсить CSV формат.
    async fn parse_csv_feed(
        &self,
        feed_config: &ThreatFeedConfig,
        data: &str,
    ) -> Result<Vec<ThreatIntel>> {
        let mut threats = Vec::new();
        let mut rdr = csv::Reader::from_reader(data.as_bytes());

        for result in rdr.records() {
            let record = result?;

            if record.len() >= 2 {
                let mut threat = ThreatIntel::default();
                threat.source = feed_config.name.clone();
                threat.last_updated = Utc::now();

                let indicator_type = if record.len() > 2 {
                    self.parse_indicator_type(&record[2])
                } else {
                    ThreatIndicatorType::Ip // По умолчанию
                };

                let threat_indicator = ThreatIndicator {
                    indicator_type,
                    value: record[0].to_string(),
                    confidence: feed_config.trust_level,
                    last_seen: None,
                    tags: vec![feed_config.name.clone()],
                };

                threat.indicators.push(threat_indicator);

                if record.len() > 1 {
                    threat.description = record[1].to_string();
                }

                threats.push(threat);
            }
        }

        Ok(threats)
    }

    /// Парсить TXT формат.
    async fn parse_txt_feed(
        &self,
        feed_config: &ThreatFeedConfig,
        data: &str,
    ) -> Result<Vec<ThreatIntel>> {
        let mut threats = Vec::new();

        for line in data.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let mut threat = ThreatIntel::default();
            threat.source = feed_config.name.clone();
            threat.last_updated = Utc::now();

            let threat_indicator = ThreatIndicator {
                indicator_type: ThreatIndicatorType::Ip, // По умолчанию для TXT
                value: line.to_string(),
                confidence: feed_config.trust_level,
                last_seen: None,
                tags: vec![feed_config.name.clone()],
            };

            threat.indicators.push(threat_indicator);
            threats.push(threat);
        }

        Ok(threats)
    }

    /// Парсить STIX формат.
    async fn parse_stix_feed(
        &self,
        feed_config: &ThreatFeedConfig,
        data: &str,
    ) -> Result<Vec<ThreatIntel>> {
        // Упрощенный парсинг STIX - в реальной реализации нужно использовать STIX библиотеку
        // Для примера парсим как JSON и пытаемся извлечь основную информацию

        #[derive(Deserialize)]
        struct StixObject {
            id: Option<String>,
            type_field: Option<String>,
            name: Option<String>,
            description: Option<String>,
            pattern: Option<String>,
            indicators: Option<Vec<StixIndicator>>,
        }

        #[derive(Deserialize)]
        struct StixIndicator {
            pattern: Option<String>,
            type_field: Option<String>,
            value: Option<String>,
        }

        if let Ok(stix_objects) = serde_json::from_str::<Vec<StixObject>>(data) {
            let mut threats = Vec::new();

            for stix_obj in stix_objects {
                let mut threat = ThreatIntel::default();
                threat.source = feed_config.name.clone();
                threat.last_updated = Utc::now();

                if let Some(name) = stix_obj.name {
                    threat.description = name;
                }

                if let Some(description) = stix_obj.description {
                    if threat.description.is_empty() {
                        threat.description = description;
                    } else {
                        threat.description = format!("{}: {}", threat.description, description);
                    }
                }

                // Парсим индикаторы
                if let Some(pattern) = stix_obj.pattern {
                    let indicator = self.parse_stix_pattern(&pattern);
                    if let Some(indicator) = indicator {
                        threat.indicators.push(indicator);
                    }
                }

                if let Some(indicators) = stix_obj.indicators {
                    for indicator in indicators {
                        if let Some(pattern) = indicator.pattern {
                            let parsed_indicator = self.parse_stix_pattern(&pattern);
                            if let Some(parsed_indicator) = parsed_indicator {
                                threat.indicators.push(parsed_indicator);
                            }
                        }
                    }
                }

                if !threat.indicators.is_empty() {
                    threats.push(threat);
                }
            }

            return Ok(threats);
        }

        Err(anyhow::anyhow!("Failed to parse STIX feed data"))
    }

    /// Парсить STIX паттерн.
    fn parse_stix_pattern(&self, pattern: &str) -> Option<ThreatIndicator> {
        // Упрощенный парсинг STIX паттернов
        // В реальной реализации нужно использовать полноценный STIX парсер

        if pattern.contains("ipv4-addr") {
            // Извлекаем IP адрес из паттерна
            if let Some(ip_start) = pattern.find('"') {
                if let Some(ip_end) = pattern[ip_start + 1..].find('"') {
                    let ip = &pattern[ip_start + 1..ip_start + 1 + ip_end];
                    return Some(ThreatIndicator {
                        indicator_type: ThreatIndicatorType::Ip,
                        value: ip.to_string(),
                        confidence: 0.9,
                        last_seen: None,
                        tags: vec!["stix".to_string()],
                    });
                }
            }
        } else if pattern.contains("domain-name") {
            // Извлекаем домен из паттерна
            if let Some(domain_start) = pattern.find('"') {
                if let Some(domain_end) = pattern[domain_start + 1..].find('"') {
                    let domain = &pattern[domain_start + 1..domain_start + 1 + domain_end];
                    return Some(ThreatIndicator {
                        indicator_type: ThreatIndicatorType::Domain,
                        value: domain.to_string(),
                        confidence: 0.9,
                        last_seen: None,
                        tags: vec!["stix".to_string()],
                    });
                }
            }
        } else if pattern.contains("file:hashes") {
            // Извлекаем хэш из паттерна
            if let Some(hash_start) = pattern.find("MD5' = '") {
                if let Some(hash_end) = pattern[hash_start + 8..].find("'") {
                    let hash = &pattern[hash_start + 8..hash_start + 8 + hash_end];
                    return Some(ThreatIndicator {
                        indicator_type: ThreatIndicatorType::Hash,
                        value: hash.to_string(),
                        confidence: 0.9,
                        last_seen: None,
                        tags: vec!["stix".to_string(), "md5".to_string()],
                    });
                }
            }
        }

        None
    }

    /// Парсить тип индикатора.
    fn parse_indicator_type(&self, type_str: &str) -> ThreatIndicatorType {
        match type_str.to_lowercase().as_str() {
            "ip" | "ipv4" | "ipv6" | "ip_address" => ThreatIndicatorType::Ip,
            "domain" | "domain_name" => ThreatIndicatorType::Domain,
            "url" => ThreatIndicatorType::Url,
            "hash" | "md5" | "sha1" | "sha256" => ThreatIndicatorType::Hash,
            "process" | "process_name" => ThreatIndicatorType::ProcessName,
            "file" | "file_path" => ThreatIndicatorType::FilePath,
            "user" | "username" => ThreatIndicatorType::Username,
            "uid" | "user_id" => ThreatIndicatorType::UserId,
            "registry" | "registry_key" => ThreatIndicatorType::RegistryKey,
            _ => ThreatIndicatorType::Unknown,
        }
    }

    /// Обновить базу данных угроз.
    async fn update_threat_database(
        &self,
        new_threats: Vec<ThreatIntel>,
    ) -> Result<ThreatUpdateHistory> {
        let mut database = self.threat_database.write().await;
        let existing_keys: HashSet<String> = database.keys().cloned().collect();
        let new_keys: HashSet<String> = new_threats.iter().map(|t| t.threat_id.clone()).collect();

        let mut threats_added = 0;
        let mut threats_updated = 0;
        let mut threats_removed = 0;

        // Обновляем существующие угрозы и добавляем новые
        for threat in new_threats {
            if existing_keys.contains(&threat.threat_id) {
                // Обновляем существующую угрозу
                database.insert(threat.threat_id.clone(), threat);
                threats_updated += 1;
            } else {
                // Добавляем новую угрозу
                database.insert(threat.threat_id.clone(), threat);
                threats_added += 1;
            }
        }

        // Удаляем угрозы, которые больше не присутствуют в новых данных
        // (если они не были обновлены в течение cache_ttl)
        let current_time = Utc::now();
        let mut to_remove = Vec::new();

        for (threat_id, threat) in database.iter() {
            if !new_keys.contains(threat_id) {
                // Проверяем, не устарела ли угроза
                let age = current_time.signed_duration_since(threat.last_updated);
                if age > self.config.cache_ttl {
                    to_remove.push(threat_id.clone());
                }
            }
        }

        for threat_id in to_remove {
            database.remove(&threat_id);
            threats_removed += 1;
        }

        // Ограничиваем количество угроз
        if database.len() > self.config.max_threats {
            // Удаляем самые старые угрозы
            let mut threats: Vec<_> = database.iter().collect();
            threats.sort_by_key(|(_, threat)| threat.last_updated);

            let to_remove_count = database.len() - self.config.max_threats;
            for i in 0..to_remove_count {
                if let Some((threat_id, _)) = threats.get(i) {
                    database.remove(threat_id);
                    threats_removed += 1;
                }
            }
        }

        // Сохраняем в кэш
        self.save_cached_threats().await?;

        let history = ThreatUpdateHistory {
            timestamp: Utc::now(),
            threats_added,
            threats_updated,
            threats_removed,
            total_threats_after: database.len(),
            status: ThreatUpdateStatus::Success,
            error_message: None,
        };

        Ok(history)
    }
}

/// Вспомогательные структуры для интеграции
/// Информация о процессе для проверки угроз.
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

/// Информация о сетевом соединении для проверки угроз.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkConnection {
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
}

/// Вспомогательная функция для создания ThreatIntelligence.
pub fn create_threat_intelligence(config: ThreatIntelligenceConfig) -> ThreatIntelligenceImpl {
    ThreatIntelligenceImpl::new(config)
}

/// Вспомогательная функция для создания ThreatIntelligence с конфигурацией по умолчанию.
pub fn create_default_threat_intelligence() -> ThreatIntelligenceImpl {
    ThreatIntelligenceImpl::new_default()
}

/// Вспомогательная функция для создания ThreatIntelligence с интеграцией безопасности.
pub fn create_threat_intelligence_with_security_monitor(
    config: ThreatIntelligenceConfig,
    security_monitor: Arc<dyn crate::health::security_monitoring::SecurityMonitorTrait>,
) -> ThreatIntelligenceImpl {
    ThreatIntelligenceImpl::new_with_security_monitor(config, security_monitor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_threat_intelligence_creation() {
        let config = ThreatIntelligenceConfig::default();
        let threat_intel = ThreatIntelligenceImpl::new(config);

        let status = threat_intel.get_threat_intelligence_status().await.unwrap();
        assert_eq!(status.status, ThreatIntelligenceStatus::NotInitialized);
        assert_eq!(status.total_threats, 0);
    }

    #[tokio::test]
    async fn test_threat_intelligence_initialization() {
        let config = ThreatIntelligenceConfig::default();
        let threat_intel = ThreatIntelligenceImpl::new(config);

        threat_intel.initialize().await.unwrap();

        let status = threat_intel.get_threat_intelligence_status().await.unwrap();
        assert_eq!(status.status, ThreatIntelligenceStatus::Initialized);
    }

    #[tokio::test]
    async fn test_add_and_find_threats() {
        let config = ThreatIntelligenceConfig::default();
        let threat_intel = ThreatIntelligenceImpl::new(config);

        threat_intel.initialize().await.unwrap();

        // Создаем тестовую угрозу
        let mut threat = ThreatIntel::default();
        threat.threat_type = ThreatType::Malware;
        threat.severity = ThreatSeverity::High;
        threat.description = "Test malware threat".to_string();
        threat.source = "test_feed".to_string();

        let indicator = ThreatIndicator {
            indicator_type: ThreatIndicatorType::ProcessName,
            value: "test_malware.exe".to_string(),
            confidence: 0.9,
            last_seen: None,
            tags: vec!["test".to_string()],
        };

        threat.indicators.push(indicator);

        // Добавляем угрозу в базу данных
        let mut database = threat_intel.threat_database.write().await;
        database.insert(threat.threat_id.clone(), threat);

        // Проверяем поиск по индикатору
        let found_threats = threat_intel
            .find_threats_by_indicator(ThreatIndicatorType::ProcessName, "test_malware.exe")
            .await
            .unwrap();

        assert_eq!(found_threats.len(), 1);
        assert_eq!(found_threats[0].threat_type, ThreatType::Malware);
    }

    #[tokio::test]
    async fn test_threat_stats() {
        let config = ThreatIntelligenceConfig::default();
        let threat_intel = ThreatIntelligenceImpl::new(config);

        threat_intel.initialize().await.unwrap();

        let stats = threat_intel.get_threat_stats().await.unwrap();
        assert_eq!(stats.total_threats, 0);
        assert_eq!(stats.active_feeds, 0);
    }

    #[tokio::test]
    async fn test_indicator_type_parsing() {
        let config = ThreatIntelligenceConfig::default();
        let threat_intel = ThreatIntelligenceImpl::new(config);

        assert_eq!(
            threat_intel.parse_indicator_type("ip"),
            ThreatIndicatorType::Ip
        );
        assert_eq!(
            threat_intel.parse_indicator_type("domain"),
            ThreatIndicatorType::Domain
        );
        assert_eq!(
            threat_intel.parse_indicator_type("hash"),
            ThreatIndicatorType::Hash
        );
        assert_eq!(
            threat_intel.parse_indicator_type("process"),
            ThreatIndicatorType::ProcessName
        );
        assert_eq!(
            threat_intel.parse_indicator_type("unknown_type"),
            ThreatIndicatorType::Unknown
        );
    }

    #[tokio::test]
    async fn test_known_threat_detection() {
        let config = ThreatIntelligenceConfig::default();
        let threat_intel = ThreatIntelligenceImpl::new(config);

        threat_intel.initialize().await.unwrap();

        // Создаем тестовую угрозу
        let mut threat = ThreatIntel::default();
        threat.source = "test_feed".to_string();

        let indicator = ThreatIndicator {
            indicator_type: ThreatIndicatorType::Ip,
            value: "192.168.1.100".to_string(),
            confidence: 0.9,
            last_seen: None,
            tags: vec!["test".to_string()],
        };

        threat.indicators.push(indicator);

        // Добавляем угрозу в базу данных
        let mut database = threat_intel.threat_database.write().await;
        database.insert(threat.threat_id.clone(), threat);

        // Проверяем обнаружение известной угрозы
        let is_known = threat_intel
            .is_known_threat(ThreatIndicatorType::Ip, "192.168.1.100")
            .await
            .unwrap();

        assert!(is_known);

        // Проверяем, что неизвестный IP не обнаруживается
        let is_unknown = threat_intel
            .is_known_threat(ThreatIndicatorType::Ip, "192.168.1.200")
            .await
            .unwrap();

        assert!(!is_unknown);
    }

    #[tokio::test]
    async fn test_threat_feed_config() {
        let mut config = ThreatFeedConfig::default();
        config.name = "Test Feed".to_string();
        config.url = "https://example.com/feed.json".to_string();
        config.format = ThreatFeedFormat::Json;
        config.update_interval = Duration::from_secs(3600);

        assert_eq!(config.name, "Test Feed");
        assert_eq!(config.url, "https://example.com/feed.json");
        assert_eq!(config.format, ThreatFeedFormat::Json);
        assert_eq!(config.update_interval, Duration::from_secs(3600));
    }

    #[tokio::test]
    async fn test_threat_types() {
        assert_eq!(ThreatType::Malware.to_string(), "malware");
        assert_eq!(ThreatType::Phishing.to_string(), "phishing");
        assert_eq!(ThreatType::Botnet.to_string(), "botnet");
        assert_eq!(ThreatType::Unknown.to_string(), "unknown");
    }

    #[tokio::test]
    async fn test_indicator_types() {
        assert_eq!(ThreatIndicatorType::Ip.to_string(), "ip");
        assert_eq!(ThreatIndicatorType::Domain.to_string(), "domain");
        assert_eq!(ThreatIndicatorType::Hash.to_string(), "hash");
        assert_eq!(ThreatIndicatorType::Unknown.to_string(), "unknown");
    }

    #[tokio::test]
    async fn test_feed_formats() {
        assert_eq!(ThreatFeedFormat::Json.to_string(), "json");
        assert_eq!(ThreatFeedFormat::Csv.to_string(), "csv");
        assert_eq!(ThreatFeedFormat::Stix.to_string(), "stix");
        assert_eq!(ThreatFeedFormat::Unknown.to_string(), "unknown");
    }
}
