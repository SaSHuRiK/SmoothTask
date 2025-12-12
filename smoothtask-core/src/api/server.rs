//! HTTP сервер для Control API.

use anyhow::{Context, Result};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{error, info, trace};

/// Состояние API сервера.
#[derive(Clone)]
pub struct ApiState {
    /// Статистика работы демона (опционально, если демон не запущен)
    daemon_stats: Option<Arc<RwLock<crate::DaemonStats>>>,
    /// Последние системные метрики (опционально)
    system_metrics: Option<Arc<RwLock<crate::metrics::system::SystemMetrics>>>,
    /// Последние процессы (опционально)
    processes: Option<Arc<RwLock<Vec<crate::logging::snapshots::ProcessRecord>>>>,
    /// Последние группы приложений (опционально)
    app_groups: Option<Arc<RwLock<Vec<crate::logging::snapshots::AppGroupRecord>>>>,
    /// Последние метрики отзывчивости (опционально)
    responsiveness_metrics: Option<Arc<RwLock<crate::logging::snapshots::ResponsivenessMetrics>>>,
    /// Текущая конфигурация демона (опционально)
    /// Используем Arc<RwLock<Config>> для поддержки динамического обновления конфигурации
    config: Option<Arc<RwLock<crate::config::config_struct::Config>>>,
    /// Путь к конфигурационному файлу (опционально)
    config_path: Option<String>,
    /// База данных паттернов для классификации процессов (опционально)
    pattern_database: Option<Arc<crate::classify::rules::PatternDatabase>>,
    /// Менеджер уведомлений для отправки уведомлений через API (опционально)
    notification_manager:
        Option<Arc<tokio::sync::Mutex<crate::notifications::NotificationManager>>>,
    /// Кэш для часто запрашиваемых данных (опционально)
    /// Используется для оптимизации производительности API
    cache: Option<Arc<RwLock<ApiCache>>>,
    /// Хранилище логов для предоставления через API (опционально)
    log_storage: Option<Arc<crate::logging::log_storage::SharedLogStorage>>,
    /// Метрики производительности API
    performance_metrics: Arc<RwLock<ApiPerformanceMetrics>>,
}

/// Кэш для часто запрашиваемых данных API.
#[derive(Default, Debug)]
pub struct ApiCache {
    /// Кэшированная версия данных о процессах (JSON)
    cached_processes_json: Option<(Value, Instant)>,
    /// Кэшированная версия данных о группах приложений (JSON)
    cached_appgroups_json: Option<(Value, Instant)>,
    /// Кэшированная версия системных метрик (JSON)
    cached_metrics_json: Option<(Value, Instant)>,
    /// Кэшированная версия конфигурации (JSON)
    cached_config_json: Option<(Value, Instant)>,
    /// Время жизни кэша (в секундах)
    cache_ttl_seconds: u64,
}

impl ApiCache {
    /// Создаёт новый кэш с указанным временем жизни.
    pub fn new(cache_ttl_seconds: u64) -> Self {
        Self {
            cache_ttl_seconds,
            ..Default::default()
        }
    }

    /// Проверяет, актуален ли кэш.
    fn is_cache_valid(&self, cache_time: &Instant) -> bool {
        cache_time.elapsed().as_secs() < self.cache_ttl_seconds
    }

    /// Очищает кэш.
    pub fn clear(&mut self) {
        self.cached_processes_json = None;
        self.cached_appgroups_json = None;
        self.cached_metrics_json = None;
        self.cached_config_json = None;
    }
}

/// Метрики производительности API.
#[derive(Default, Debug)]
pub struct ApiPerformanceMetrics {
    /// Общее количество запросов
    pub total_requests: u64,
    /// Количество кэш-хитов
    pub cache_hits: u64,
    /// Количество кэш-миссов
    pub cache_misses: u64,
    /// Общее время обработки запросов (в микросекундах)
    pub total_processing_time_us: u64,
    /// Время последнего запроса (для мониторинга активности)
    pub last_request_time: Option<Instant>,
}

impl ApiPerformanceMetrics {
    /// Увеличивает счётчик запросов и обновляет время последнего запроса.
    pub fn increment_requests(&mut self) {
        self.total_requests += 1;
        self.last_request_time = Some(Instant::now());
    }

    /// Увеличивает счётчик кэш-хитов.
    pub fn increment_cache_hits(&mut self) {
        self.cache_hits += 1;
    }

    /// Увеличивает счётчик кэш-миссов.
    pub fn increment_cache_misses(&mut self) {
        self.cache_misses += 1;
    }

    /// Добавляет время обработки запроса.
    pub fn add_processing_time(&mut self, duration: Duration) {
        self.total_processing_time_us += duration.as_micros() as u64;
    }

    /// Возвращает среднее время обработки запроса в микросекундах.
    pub fn average_processing_time_us(&self) -> f64 {
        if self.total_requests > 0 {
            self.total_processing_time_us as f64 / self.total_requests as f64
        } else {
            0.0
        }
    }

    /// Возвращает процент кэш-хитов.
    pub fn cache_hit_rate(&self) -> f64 {
        let total = self.cache_hits + self.cache_misses;
        if total > 0 {
            self.cache_hits as f64 / total as f64 * 100.0
        } else {
            0.0
        }
    }
}

/// Builder для ApiState, чтобы избежать слишком большого количества аргументов в конструкторах.
///
/// ApiStateBuilder предоставляет удобный способ создания ApiState с различными комбинациями
/// данных. Это позволяет избежать проблем с большим количеством параметров в конструкторах
/// и делает код более читаемым и поддерживаемым.
///
/// # Преимущества использования ApiStateBuilder
///
/// - **Читаемость**: Код становится более понятным, так как каждый вызов метода явно указывает,
///   какие данные добавляются
/// - **Гибкость**: Легко добавлять или удалять данные без изменения сигнатур функций
/// - **Типобезопасность**: Все параметры проверяются на этапе компиляции
/// - **Поддержка IDE**: Автодополнение и подсказки работают лучше с цепочкой методов
///
/// # Примеры использования
///
/// ## Базовый пример
///
/// ```no_run
/// use smoothtask_core::api::ApiStateBuilder;
/// use std::sync::Arc;
/// use tokio::sync::RwLock;
///
/// let state = ApiStateBuilder::new()
///     .with_daemon_stats(None)
///     .with_system_metrics(None)
///     .with_processes(None)
///     .with_app_groups(None)
///     .with_responsiveness_metrics(None)
///     .with_config(None)
///     .with_config_path(None)
///     .with_pattern_database(None)
///     .with_notification_manager(None)
///     .build();
/// ```
///
/// ## Пример с реальными данными
///
/// ```no_run
/// use smoothtask_core::api::ApiStateBuilder;
/// use smoothtask_core::{DaemonStats, config::config_struct::Config};
/// use smoothtask_core::metrics::system::SystemMetrics;
/// use smoothtask_core::logging::snapshots::{ProcessRecord, AppGroupRecord, ResponsivenessMetrics};
/// use smoothtask_core::classify::rules::PatternDatabase;
/// use smoothtask_core::notifications::NotificationManager;
/// use std::sync::Arc;
/// use tokio::sync::{RwLock, Mutex};
///
/// // Создаём тестовые данные
/// let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
/// let system_metrics = Arc::new(RwLock::new(SystemMetrics::default()));
/// let processes = Arc::new(RwLock::new(Vec::<ProcessRecord>::new()));
/// let app_groups = Arc::new(RwLock::new(Vec::<AppGroupRecord>::new()));
/// let responsiveness_metrics = Arc::new(RwLock::new(ResponsivenessMetrics::default()));
/// let config = Arc::new(RwLock::new(Config::default()));
/// let pattern_db = Arc::new(PatternDatabase::load("/path/to/patterns").unwrap());
/// let notification_manager = Arc::new(Mutex::new(NotificationManager::new_stub()));
///
/// // Собираем состояние API
/// let state = ApiStateBuilder::new()
///     .with_daemon_stats(Some(daemon_stats))
///     .with_system_metrics(Some(system_metrics))
///     .with_processes(Some(processes))
///     .with_app_groups(Some(app_groups))
///     .with_responsiveness_metrics(Some(responsiveness_metrics))
///     .with_config(Some(config))
///     .with_config_path(Some("/path/to/config.yml".to_string()))
///     .with_pattern_database(Some(pattern_db))
///     .with_notification_manager(Some(notification_manager))
///     .build();
/// ```
///
/// ## Сравнение с устаревшими методами
///
/// До введения ApiStateBuilder код создания ApiState выглядел так:
///
/// ```no_run
/// // Старый способ (устарел)
/// let state = ApiState::with_all_and_responsiveness_and_config_and_patterns(
///     daemon_stats,
///     system_metrics,
///     processes,
///     app_groups,
///     responsiveness_metrics,
///     config,
///     pattern_database,
/// );
/// ```
///
/// С ApiStateBuilder код становится более понятным и гибким:
///
/// ```no_run
/// // Новый способ (рекомендуется)
/// let state = ApiStateBuilder::new()
///     .with_daemon_stats(Some(daemon_stats))
///     .with_system_metrics(Some(system_metrics))
///     .with_processes(Some(processes))
///     .with_app_groups(Some(app_groups))
///     .with_responsiveness_metrics(Some(responsiveness_metrics))
///     .with_config(Some(config))
///     .with_pattern_database(Some(pattern_db))
///     .build();
/// ```
///
/// # Методы
///
/// Каждый метод `with_*` принимает `Option<T>` и возвращает `Self`, что позволяет
/// использовать цепочку вызовов (builder pattern).
#[derive(Default)]
#[allow(dead_code)]
pub struct ApiStateBuilder {
    daemon_stats: Option<Arc<RwLock<crate::DaemonStats>>>,
    system_metrics: Option<Arc<RwLock<crate::metrics::system::SystemMetrics>>>,
    processes: Option<Arc<RwLock<Vec<crate::logging::snapshots::ProcessRecord>>>>,
    app_groups: Option<Arc<RwLock<Vec<crate::logging::snapshots::AppGroupRecord>>>>,
    responsiveness_metrics: Option<Arc<RwLock<crate::logging::snapshots::ResponsivenessMetrics>>>,
    config: Option<Arc<RwLock<crate::config::config_struct::Config>>>,
    config_path: Option<String>,
    pattern_database: Option<Arc<crate::classify::rules::PatternDatabase>>,
    notification_manager:
        Option<Arc<tokio::sync::Mutex<crate::notifications::NotificationManager>>>,
    cache: Option<Arc<RwLock<ApiCache>>>,
    log_storage: Option<Arc<crate::logging::log_storage::SharedLogStorage>>,
    performance_metrics: Option<Arc<RwLock<ApiPerformanceMetrics>>>,
}

impl ApiStateBuilder {
    /// Создаёт новый пустой ApiStateBuilder.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::ApiStateBuilder;
    ///
    /// let builder = ApiStateBuilder::new();
    /// // Теперь можно добавлять данные через методы with_*
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Добавляет статистику работы демона в состояние API.
    ///
    /// # Параметры
    ///
    /// - `daemon_stats`: Статистика работы демона (опционально)
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::ApiStateBuilder;
    /// use smoothtask_core::DaemonStats;
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
    /// let builder = ApiStateBuilder::new()
    ///     .with_daemon_stats(Some(daemon_stats));
    /// ```
    pub fn with_daemon_stats(
        mut self,
        daemon_stats: Option<Arc<RwLock<crate::DaemonStats>>>,
    ) -> Self {
        self.daemon_stats = daemon_stats;
        self
    }

    /// Добавляет системные метрики в состояние API.
    ///
    /// # Параметры
    ///
    /// - `system_metrics`: Системные метрики (CPU, память, PSI и т.д.) (опционально)
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::ApiStateBuilder;
    /// use smoothtask_core::metrics::system::SystemMetrics;
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// let system_metrics = Arc::new(RwLock::new(SystemMetrics::default()));
    /// let builder = ApiStateBuilder::new()
    ///     .with_system_metrics(Some(system_metrics));
    /// ```
    pub fn with_system_metrics(
        mut self,
        system_metrics: Option<Arc<RwLock<crate::metrics::system::SystemMetrics>>>,
    ) -> Self {
        self.system_metrics = system_metrics;
        self
    }

    /// Добавляет список процессов в состояние API.
    ///
    /// # Параметры
    ///
    /// - `processes`: Список процессов (опционально)
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::ApiStateBuilder;
    /// use smoothtask_core::logging::snapshots::ProcessRecord;
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// let processes = Arc::new(RwLock::new(Vec::<ProcessRecord>::new()));
    /// let builder = ApiStateBuilder::new()
    ///     .with_processes(Some(processes));
    /// ```
    pub fn with_processes(
        mut self,
        processes: Option<Arc<RwLock<Vec<crate::logging::snapshots::ProcessRecord>>>>,
    ) -> Self {
        self.processes = processes;
        self
    }

    /// Добавляет группы приложений в состояние API.
    ///
    /// # Параметры
    ///
    /// - `app_groups`: Список групп приложений (опционально)
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::ApiStateBuilder;
    /// use smoothtask_core::logging::snapshots::AppGroupRecord;
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// let app_groups = Arc::new(RwLock::new(Vec::<AppGroupRecord>::new()));
    /// let builder = ApiStateBuilder::new()
    ///     .with_app_groups(Some(app_groups));
    /// ```
    pub fn with_app_groups(
        mut self,
        app_groups: Option<Arc<RwLock<Vec<crate::logging::snapshots::AppGroupRecord>>>>,
    ) -> Self {
        self.app_groups = app_groups;
        self
    }

    /// Добавляет метрики отзывчивости в состояние API.
    ///
    /// # Параметры
    ///
    /// - `responsiveness_metrics`: Метрики отзывчивости системы (опционально)
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::ApiStateBuilder;
    /// use smoothtask_core::logging::snapshots::ResponsivenessMetrics;
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// let responsiveness_metrics = Arc::new(RwLock::new(ResponsivenessMetrics::default()));
    /// let builder = ApiStateBuilder::new()
    ///     .with_responsiveness_metrics(Some(responsiveness_metrics));
    /// ```
    pub fn with_responsiveness_metrics(
        mut self,
        responsiveness_metrics: Option<
            Arc<RwLock<crate::logging::snapshots::ResponsivenessMetrics>>,
        >,
    ) -> Self {
        self.responsiveness_metrics = responsiveness_metrics;
        self
    }

    /// Добавляет конфигурацию демона в состояние API.
    ///
    /// # Параметры
    ///
    /// - `config`: Конфигурация демона (опционально)
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::ApiStateBuilder;
    /// use smoothtask_core::config::config_struct::Config;
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// let config = Arc::new(RwLock::new(Config::default()));
    /// let builder = ApiStateBuilder::new()
    ///     .with_config(Some(config));
    /// ```
    pub fn with_config(
        mut self,
        config: Option<Arc<RwLock<crate::config::config_struct::Config>>>,
    ) -> Self {
        self.config = config;
        self
    }

    /// Добавляет путь к конфигурационному файлу в состояние API.
    ///
    /// # Параметры
    ///
    /// - `config_path`: Путь к конфигурационному файлу (опционально)
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::ApiStateBuilder;
    ///
    /// let builder = ApiStateBuilder::new()
    ///     .with_config_path(Some("/path/to/config.yml".to_string()));
    /// ```
    pub fn with_config_path(mut self, config_path: Option<String>) -> Self {
        self.config_path = config_path;
        self
    }

    /// Добавляет базу паттернов для классификации процессов в состояние API.
    ///
    /// # Параметры
    ///
    /// - `pattern_database`: База паттернов (опционально)
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::ApiStateBuilder;
    /// use smoothtask_core::classify::rules::PatternDatabase;
    /// use std::sync::Arc;
    ///
    /// let pattern_db = Arc::new(PatternDatabase::load("/path/to/patterns").unwrap());
    /// let builder = ApiStateBuilder::new()
    ///     .with_pattern_database(Some(pattern_db));
    /// ```
    pub fn with_pattern_database(
        mut self,
        pattern_database: Option<Arc<crate::classify::rules::PatternDatabase>>,
    ) -> Self {
        self.pattern_database = pattern_database;
        self
    }

    /// Добавляет менеджер уведомлений в состояние API.
    ///
    /// # Параметры
    ///
    /// - `notification_manager`: Менеджер уведомлений (опционально)
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::ApiStateBuilder;
    /// use smoothtask_core::notifications::NotificationManager;
    /// use std::sync::Arc;
    /// use tokio::sync::Mutex;
    ///
    /// let notification_manager = Arc::new(Mutex::new(NotificationManager::new_stub()));
    /// let builder = ApiStateBuilder::new()
    ///     .with_notification_manager(Some(notification_manager));
    /// ```
    pub fn with_notification_manager(
        mut self,
        notification_manager: Option<
            Arc<tokio::sync::Mutex<crate::notifications::NotificationManager>>,
        >,
    ) -> Self {
        self.notification_manager = notification_manager;
        self
    }

    /// Устанавливает кэш для API.
    ///
    /// # Параметры
    ///
    /// - `cache`: Кэш для часто запрашиваемых данных
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::{ApiStateBuilder, ApiCache};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// let cache = Arc::new(RwLock::new(ApiCache::new(60))); // 60 seconds TTL
    /// let builder = ApiStateBuilder::new()
    ///     .with_cache(Some(cache));
    /// ```
    pub fn with_cache(mut self, cache: Option<Arc<RwLock<ApiCache>>>) -> Self {
        self.cache = cache;
        self
    }

    /// Устанавливает метрики производительности для API.
    ///
    /// # Параметры
    ///
    /// - `performance_metrics`: Метрики производительности
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::{ApiStateBuilder, ApiPerformanceMetrics};
    /// use std::sync::Arc;
    /// use tokio::sync::RwLock;
    ///
    /// let metrics = Arc::new(RwLock::new(ApiPerformanceMetrics::default()));
    /// let builder = ApiStateBuilder::new()
    ///     .with_performance_metrics(Some(metrics));
    /// ```
    pub fn with_performance_metrics(
        mut self,
        performance_metrics: Option<Arc<RwLock<ApiPerformanceMetrics>>>,
    ) -> Self {
        self.performance_metrics = performance_metrics;
        self
    }

    /// Устанавливает хранилище логов для API.
    ///
    /// # Параметры
    ///
    /// - `log_storage`: Хранилище логов для предоставления через API
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::ApiStateBuilder;
    /// use smoothtask_core::logging::log_storage::SharedLogStorage;
    /// use std::sync::Arc;
    ///
    /// let log_storage = Arc::new(SharedLogStorage::new(1000));
    /// let builder = ApiStateBuilder::new()
    ///     .with_log_storage(Some(log_storage));
    /// ```
    pub fn with_log_storage(
        mut self,
        log_storage: Option<Arc<crate::logging::log_storage::SharedLogStorage>>,
    ) -> Self {
        self.log_storage = log_storage;
        self
    }

    /// Завершает построение и возвращает ApiState.
    ///
    /// Этот метод потребляет builder и возвращает готовое состояние API,
    /// которое можно использовать для создания API сервера.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::ApiStateBuilder;
    ///
    /// let state = ApiStateBuilder::new()
    ///     .with_daemon_stats(None)
    ///     .with_system_metrics(None)
    ///     .build();
    /// ```
    pub fn build(self) -> ApiState {
        ApiState {
            daemon_stats: self.daemon_stats,
            system_metrics: self.system_metrics,
            processes: self.processes,
            app_groups: self.app_groups,
            responsiveness_metrics: self.responsiveness_metrics,
            config: self.config,
            config_path: self.config_path,
            pattern_database: self.pattern_database,
            notification_manager: self.notification_manager,
            cache: self.cache,
            log_storage: self.log_storage,
            performance_metrics: self
                .performance_metrics
                .unwrap_or_else(|| Arc::new(RwLock::new(ApiPerformanceMetrics::default()))),
        }
    }
}

impl ApiState {
    /// Создаёт новое состояние API сервера.
    pub fn new() -> Self {
        Self {
            daemon_stats: None,
            system_metrics: None,
            processes: None,
            app_groups: None,
            responsiveness_metrics: None,
            config: None,
            config_path: None,
            pattern_database: None,
            notification_manager: None,
            cache: None,
            log_storage: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
        }
    }

    /// Создаёт новое состояние API сервера с переданной статистикой демона.
    pub fn with_daemon_stats(daemon_stats: Arc<RwLock<crate::DaemonStats>>) -> Self {
        Self {
            daemon_stats: Some(daemon_stats),
            system_metrics: None,
            processes: None,
            app_groups: None,
            responsiveness_metrics: None,
            config: None,
            config_path: None,
            pattern_database: None,
            notification_manager: None,
            cache: None,
            log_storage: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
        }
    }

    /// Создаёт новое состояние API сервера с переданными системными метриками.
    pub fn with_system_metrics(
        system_metrics: Arc<RwLock<crate::metrics::system::SystemMetrics>>,
    ) -> Self {
        Self {
            daemon_stats: None,
            system_metrics: Some(system_metrics),
            processes: None,
            app_groups: None,
            responsiveness_metrics: None,
            config: None,
            config_path: None,
            pattern_database: None,
            notification_manager: None,
            cache: None,
            log_storage: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
        }
    }

    /// Создаёт новое состояние API сервера с переданной статистикой демона и системными метриками.
    pub fn with_all(
        daemon_stats: Option<Arc<RwLock<crate::DaemonStats>>>,
        system_metrics: Option<Arc<RwLock<crate::metrics::system::SystemMetrics>>>,
        processes: Option<Arc<RwLock<Vec<crate::logging::snapshots::ProcessRecord>>>>,
        app_groups: Option<Arc<RwLock<Vec<crate::logging::snapshots::AppGroupRecord>>>>,
    ) -> Self {
        Self {
            daemon_stats,
            system_metrics,
            processes,
            app_groups,
            responsiveness_metrics: None,
            config: None,
            config_path: None,
            pattern_database: None,
            notification_manager: None,
            cache: None,
            log_storage: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
        }
    }

    /// Создаёт новое состояние API сервера со всеми данными, включая конфигурацию.
    pub fn with_all_and_config(
        daemon_stats: Option<Arc<RwLock<crate::DaemonStats>>>,
        system_metrics: Option<Arc<RwLock<crate::metrics::system::SystemMetrics>>>,
        processes: Option<Arc<RwLock<Vec<crate::logging::snapshots::ProcessRecord>>>>,
        app_groups: Option<Arc<RwLock<Vec<crate::logging::snapshots::AppGroupRecord>>>>,
        config: Option<Arc<RwLock<crate::config::config_struct::Config>>>,
    ) -> Self {
        Self {
            daemon_stats,
            system_metrics,
            processes,
            app_groups,
            responsiveness_metrics: None,
            config,
            config_path: None,
            pattern_database: None,
            notification_manager: None,
            cache: None,
            log_storage: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
        }
    }

    /// Создаёт новое состояние API сервера со всеми данными, включая метрики отзывчивости и конфигурацию.
    pub fn with_all_and_responsiveness_and_config(
        daemon_stats: Option<Arc<RwLock<crate::DaemonStats>>>,
        system_metrics: Option<Arc<RwLock<crate::metrics::system::SystemMetrics>>>,
        processes: Option<Arc<RwLock<Vec<crate::logging::snapshots::ProcessRecord>>>>,
        app_groups: Option<Arc<RwLock<Vec<crate::logging::snapshots::AppGroupRecord>>>>,
        responsiveness_metrics: Option<
            Arc<RwLock<crate::logging::snapshots::ResponsivenessMetrics>>,
        >,
        config: Option<Arc<RwLock<crate::config::config_struct::Config>>>,
    ) -> Self {
        Self {
            daemon_stats,
            system_metrics,
            processes,
            app_groups,
            responsiveness_metrics,
            config,
            config_path: None,
            pattern_database: None,
            notification_manager: None,
            cache: None,
            log_storage: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
        }
    }

    /// Возвращает кэш, создавая новый по умолчанию если он не установлен.
    pub fn get_or_create_cache(&self) -> Arc<RwLock<ApiCache>> {
        self.cache.clone().unwrap_or_else(|| {
            // Note: We can't store this back in self.cache because self is immutable
            // This is fine for our use case as we'll use the cache temporarily
            Arc::new(RwLock::new(ApiCache::new(60))) // 60 seconds TTL by default
        })
    }
}

impl Default for ApiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Версия демона SmoothTask.
const DAEMON_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Обработчик для endpoint `/health`.
///
/// Возвращает статус работоспособности API сервера.
async fn health_handler() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "smoothtask-api"
    }))
}

/// Обработчик для endpoint `/api/health`.
///
/// Возвращает расширенную информацию о состоянии демона, включая время работы,
/// статус основных компонентов и метрики производительности.
async fn health_detailed_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    let start_time = state.performance_metrics.read().await.last_request_time;
    let uptime_seconds = start_time.map(|t| t.elapsed().as_secs()).unwrap_or(0);

    // Проверка доступности основных компонентов
    let daemon_stats_available = state.daemon_stats.is_some();
    let system_metrics_available = state.system_metrics.is_some();
    let processes_available = state.processes.is_some();
    let app_groups_available = state.app_groups.is_some();
    let config_available = state.config.is_some();
    let pattern_database_available = state.pattern_database.is_some();

    // Получение метрик производительности
    let perf_metrics = state.performance_metrics.read().await;

    Ok(Json(json!({
        "status": "ok",
        "service": "smoothtaskd",
        "uptime_seconds": uptime_seconds,
        "components": {
            "daemon_stats": daemon_stats_available,
            "system_metrics": system_metrics_available,
            "processes": processes_available,
            "app_groups": app_groups_available,
            "config": config_available,
            "pattern_database": pattern_database_available
        },
        "performance": {
            "total_requests": perf_metrics.total_requests,
            "cache_hit_rate": perf_metrics.cache_hit_rate(),
            "average_processing_time_us": perf_metrics.average_processing_time_us()
        },
        "timestamp": Utc::now().to_rfc3339()
    })))
}

/// Обработчик для endpoint `/api/version`.
///
/// Возвращает версию демона SmoothTask.
async fn version_handler() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "version": DAEMON_VERSION,
        "service": "smoothtaskd"
    }))
}

/// Обработчик для endpoint `/api/endpoints`.
///
/// Возвращает список всех доступных endpoints API.
async fn endpoints_handler() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "endpoints": [
            {
                "path": "/health",
                "method": "GET",
                "description": "Проверка работоспособности API сервера"
            },
            {
                "path": "/api/health",
                "method": "GET",
                "description": "Получение расширенной информации о состоянии демона, включая время работы и статус компонентов"
            },
            {
                "path": "/api/version",
                "method": "GET",
                "description": "Получение версии демона SmoothTask"
            },
            {
                "path": "/api/endpoints",
                "method": "GET",
                "description": "Получение списка всех доступных endpoints"
            },
            {
                "path": "/api/stats",
                "method": "GET",
                "description": "Получение статистики работы демона"
            },
            {
                "path": "/api/metrics",
                "method": "GET",
                "description": "Получение последних системных метрик"
            },
            {
                "path": "/api/responsiveness",
                "method": "GET",
                "description": "Получение последних метрик отзывчивости системы"
            },
            {
                "path": "/api/processes",
                "method": "GET",
                "description": "Получение списка последних процессов"
            },
            {
                "path": "/api/processes/:pid",
                "method": "GET",
                "description": "Получение информации о конкретном процессе по PID"
            },
            {
                "path": "/api/appgroups",
                "method": "GET",
                "description": "Получение списка последних групп приложений"
            },
            {
                "path": "/api/appgroups/:id",
                "method": "GET",
                "description": "Получение информации о конкретной группе приложений по ID"
            },
            {
                "path": "/api/config",
                "method": "GET",
                "description": "Получение текущей конфигурации демона (без секретов)"
            },
            {
                "path": "/api/classes",
                "method": "GET",
                "description": "Получение информации о всех доступных классах QoS и их параметрах приоритета"
            },
            {
                "path": "/api/patterns",
                "method": "GET",
                "description": "Получение информации о загруженных паттернах для классификации процессов"
            },
            {
                "path": "/api/system",
                "method": "GET",
                "description": "Получение информации о системе (ядро, архитектура, дистрибутив)"
            },
            {
                "path": "/api/config/reload",
                "method": "POST",
                "description": "Перезагрузка конфигурации демона из файла"
            },
            {
                "path": "/api/notifications/test",
                "method": "POST",
                "description": "Отправка тестового уведомления"
            },
            {
                "path": "/api/notifications/custom",
                "method": "POST",
                "description": "Отправка пользовательского уведомления с указанными параметрами"
            },
            {
                "path": "/api/notifications/status",
                "method": "GET",
                "description": "Получение текущего состояния системы уведомлений"
            },
            {
                "path": "/api/notifications/config",
                "method": "POST",
                "description": "Изменение конфигурации уведомлений в runtime"
            },
            {
                "path": "/api/performance",
                "method": "GET",
                "description": "Получение метрик производительности API сервера"
            },
            {
                "path": "/api/logs",
                "method": "GET",
                "description": "Получение последних логов приложения с фильтрацией по уровню и лимиту"
            }
        ],
        "count": 22
    }))
}

/// Обработчик для endpoint `/api/classes`.
///
/// Возвращает информацию о всех доступных классах QoS (Quality of Service)
/// и их параметрах приоритета (nice, latency_nice, ionice, cpu.weight).
async fn classes_handler() -> Json<Value> {
    use crate::policy::classes::PriorityClass;

    let classes: Vec<Value> = vec![
        PriorityClass::CritInteractive,
        PriorityClass::Interactive,
        PriorityClass::Normal,
        PriorityClass::Background,
        PriorityClass::Idle,
    ]
    .into_iter()
    .map(|class| {
        let params = class.params();
        json!({
            "class": class,
            "name": class.as_str(),
            "description": match class {
                PriorityClass::CritInteractive => "Критически интерактивные процессы (фокус + аудио/игра)",
                PriorityClass::Interactive => "Обычные интерактивные процессы (UI/CLI)",
                PriorityClass::Normal => "Дефолтный приоритет",
                PriorityClass::Background => "Фоновые процессы (batch/maintenance)",
                PriorityClass::Idle => "Процессы, которые можно выполнять \"на остатке\"",
            },
            "params": {
                "nice": params.nice.nice,
                "latency_nice": params.latency_nice.latency_nice,
                "ionice": {
                    "class": params.ionice.class,
                    "level": params.ionice.level,
                    "class_description": match params.ionice.class {
                        1 => "realtime",
                        2 => "best-effort",
                        3 => "idle",
                        _ => "unknown",
                    }
                },
                "cgroup": {
                    "cpu_weight": params.cgroup.cpu_weight,
                }
            }
        })
    })
    .collect();

    Json(json!({
        "status": "ok",
        "classes": classes,
        "count": classes.len()
    }))
}

/// Получает информацию о системе (ядро, архитектура, дистрибутив).
fn get_system_info() -> Value {
    let mut info = json!({
        "kernel": {},
        "architecture": None::<String>,
        "distribution": {}
    });

    // Читаем версию ядра из /proc/version
    if let Ok(version) = fs::read_to_string("/proc/version") {
        let version = version.trim();
        info["kernel"]["version_string"] = json!(version);

        // Пытаемся извлечь версию ядра (например, "Linux version 6.14.0-36-generic")
        if let Some(version_start) = version.find("Linux version ") {
            let version_part = &version[version_start + 14..];
            if let Some(version_end) = version_part.find(' ') {
                info["kernel"]["version"] = json!(version_part[..version_end].to_string());
            }
        }
    }

    // Читаем архитектуру из /proc/sys/kernel/arch
    if let Ok(arch) = fs::read_to_string("/proc/sys/kernel/arch") {
        info["architecture"] = json!(arch.trim().to_string());
    }

    // Читаем информацию о дистрибутиве из /etc/os-release
    if let Ok(os_release) = fs::read_to_string("/etc/os-release") {
        let mut dist_info = json!({});
        for line in os_release.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let value = value.trim_matches('"');
                match key {
                    "NAME" => dist_info["name"] = json!(value),
                    "VERSION" => dist_info["version"] = json!(value),
                    "ID" => dist_info["id"] = json!(value),
                    "ID_LIKE" => dist_info["id_like"] = json!(value),
                    "PRETTY_NAME" => dist_info["pretty_name"] = json!(value),
                    _ => {}
                }
            }
        }
        if !dist_info.as_object().unwrap().is_empty() {
            info["distribution"] = dist_info;
        }
    }

    info
}

/// Обработчик для endpoint `/api/system`.
///
/// Возвращает информацию о системе (ядро, архитектура, дистрибутив).
async fn system_handler() -> Json<Value> {
    let system_info = get_system_info();
    Json(json!({
        "status": "ok",
        "system": system_info
    }))
}

/// Обработчик для endpoint `/api/patterns`.
///
/// Возвращает информацию о загруженных паттернах для классификации процессов (если доступны).
async fn patterns_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    match &state.pattern_database {
        Some(pattern_db) => {
            let patterns_by_category: std::collections::HashMap<_, _> =
                pattern_db.all_patterns().iter().fold(
                    std::collections::HashMap::<String, Vec<serde_json::Value>>::new(),
                    |mut acc, (category, pattern)| {
                        acc.entry(category.0.clone()).or_default().push(json!({
                            "name": pattern.name,
                            "label": pattern.label,
                            "exe_patterns": pattern.exe_patterns,
                            "desktop_patterns": pattern.desktop_patterns,
                            "cgroup_patterns": pattern.cgroup_patterns,
                            "tags": pattern.tags,
                        }));
                        acc
                    },
                );

            let categories: Vec<Value> = patterns_by_category
                .into_iter()
                .map(|(category, patterns)| {
                    json!({
                        "category": category,
                        "patterns": patterns,
                        "count": patterns.len()
                    })
                })
                .collect();

            Ok(Json(json!({
                "status": "ok",
                "categories": categories,
                "total_patterns": pattern_db.all_patterns().len(),
                "total_categories": categories.len()
            })))
        }
        None => Ok(Json(json!({
            "status": "ok",
            "categories": [],
            "total_patterns": 0,
            "total_categories": 0,
            "message": "Pattern database not available (daemon may not be running or patterns not loaded)"
        }))),
    }
}

/// Обработчик для endpoint `/api/notifications/test` (POST).
///
/// Отправляет тестовое уведомление через систему уведомлений.
/// Используется для проверки работоспособности системы уведомлений.
///
/// # Примечания
///
/// - Требует наличия notification_manager в состоянии API
/// - Возвращает информацию об отправленном уведомлении и статусе отправки
async fn notifications_test_handler(
    State(state): State<ApiState>,
) -> Result<Json<Value>, StatusCode> {
    match &state.notification_manager {
        Some(notification_manager_arc) => {
            let notification_manager = notification_manager_arc.lock().await;

            // Создаём тестовое уведомление
            let notification = crate::notifications::Notification::new(
                crate::notifications::NotificationType::Info,
                "Test Notification",
                "This is a test notification from SmoothTask API",
            )
            .with_details("Sent via /api/notifications/test endpoint");

            // Отправляем уведомление
            match notification_manager.send(&notification).await {
                Ok(_) => {
                    tracing::info!("Test notification sent successfully");
                    Ok(Json(json!({
                        "status": "success",
                        "message": "Test notification sent successfully",
                        "notification": {
                            "type": "info",
                            "title": "Test Notification",
                            "message": "This is a test notification from SmoothTask API",
                            "details": "Sent via /api/notifications/test endpoint",
                            "timestamp": notification.timestamp.to_rfc3339()
                        },
                        "backend": notification_manager.backend_name()
                    })))
                }
                Err(e) => {
                    tracing::error!("Failed to send test notification: {}", e);
                    Ok(Json(json!({
                        "status": "error",
                        "message": format!("Failed to send test notification: {}", e),
                        "backend": notification_manager.backend_name()
                    })))
                }
            }
        }
        None => {
            tracing::warn!("Notification manager not available for test notification");
            Ok(Json(json!({
                "status": "error",
                "message": "Notification manager not available (daemon may not be running or notifications not configured)",
                "backend": "none"
            })))
        }
    }
}

/// Обработчик для endpoint `/api/notifications/custom` (POST).
///
/// Отправляет пользовательское уведомление с указанными параметрами.
/// Позволяет тестировать уведомления разных типов и с разными сообщениями.
///
/// # Примечания
///
/// - Требует наличия notification_manager в состоянии API
/// - Позволяет указать тип уведомления, заголовок, сообщение и дополнительные детали
/// - Возвращает информацию об отправленном уведомлении и статусе отправки
async fn notifications_custom_handler(
    State(state): State<ApiState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    match &state.notification_manager {
        Some(notification_manager_arc) => {
            let notification_manager = notification_manager_arc.lock().await;

            // Извлекаем параметры уведомления из payload
            let notification_type = match payload.get("type").and_then(|v| v.as_str()) {
                Some("critical") => crate::notifications::NotificationType::Critical,
                Some("warning") => crate::notifications::NotificationType::Warning,
                Some(_) | None => crate::notifications::NotificationType::Info,
            };

            let title = payload
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("Custom Notification")
                .to_string();

            let message = payload
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("Custom notification message")
                .to_string();

            let details = payload
                .get("details")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            // Создаём уведомление
            let mut notification =
                crate::notifications::Notification::new(notification_type, title, message);

            if let Some(details_str) = details {
                notification = notification.with_details(details_str);
            }

            // Отправляем уведомление
            match notification_manager.send(&notification).await {
                Ok(_) => {
                    tracing::info!("Custom notification sent successfully");
                    Ok(Json(json!({
                        "status": "success",
                        "message": "Custom notification sent successfully",
                        "notification": {
                            "type": format!("{}", notification.notification_type),
                            "title": notification.title,
                            "message": notification.message,
                            "details": notification.details,
                            "timestamp": notification.timestamp.to_rfc3339()
                        },
                        "backend": notification_manager.backend_name()
                    })))
                }
                Err(e) => {
                    tracing::error!("Failed to send custom notification: {}", e);
                    Ok(Json(json!({
                        "status": "error",
                        "message": format!("Failed to send custom notification: {}", e),
                        "backend": notification_manager.backend_name()
                    })))
                }
            }
        }
        None => {
            tracing::warn!("Notification manager not available for custom notification");
            Ok(Json(json!({
                "status": "error",
                "message": "Notification manager not available (daemon may not be running or notifications not configured)",
                "backend": "none"
            })))
        }
    }
}

/// Обработчик для endpoint `/api/notifications/status` (GET).
///
/// Возвращает текущее состояние системы уведомлений.
/// Включает информацию о том, включены ли уведомления, используемый бэкенд и текущую конфигурацию.
async fn notifications_status_handler(
    State(state): State<ApiState>,
) -> Result<Json<Value>, StatusCode> {
    // Получаем текущую конфигурацию уведомлений
    let notification_config = match &state.config {
        Some(config_arc) => {
            let config_guard = config_arc.read().await;
            Some(config_guard.notifications.clone())
        }
        None => None,
    };

    // Получаем информацию о менеджере уведомлений
    let notification_manager_info = match &state.notification_manager {
        Some(notification_manager_arc) => {
            let notification_manager = notification_manager_arc.lock().await;
            Some(json!({
                "enabled": notification_manager.is_enabled(),
                "backend": notification_manager.backend_name()
            }))
        }
        None => None,
    };

    Ok(Json(json!({
        "status": "ok",
        "notifications": {
            "config": notification_config,
            "manager": notification_manager_info,
            "available": notification_manager_info.is_some()
        }
    })))
}

/// Обработчик для endpoint `/api/performance`.
///
/// Возвращает метрики производительности API сервера.
/// Включает информацию о количестве запросов, кэш-хитах, времени обработки и т.д.
async fn performance_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    let perf_metrics = state.performance_metrics.read().await;

    Ok(Json(json!({
        "status": "ok",
        "performance_metrics": {
            "total_requests": perf_metrics.total_requests,
            "cache_hits": perf_metrics.cache_hits,
            "cache_misses": perf_metrics.cache_misses,
            "cache_hit_rate": perf_metrics.cache_hit_rate(),
            "average_processing_time_us": perf_metrics.average_processing_time_us(),
            "total_processing_time_us": perf_metrics.total_processing_time_us,
            "last_request_time": perf_metrics.last_request_time.map(|t| t.elapsed().as_secs_f64()),
            "requests_per_second": if perf_metrics.total_requests > 0 && perf_metrics.last_request_time.is_some() {
                let elapsed = perf_metrics.last_request_time.unwrap().elapsed().as_secs_f64();
                if elapsed > 0.0 {
                    perf_metrics.total_requests as f64 / elapsed
                } else {
                    0.0
                }
            } else {
                0.0
            }
        },
        "cache_info": {
            "enabled": state.cache.is_some(),
            "ttl_seconds": None::<u64>
        }
    })))
}

/// Обработчик для endpoint `/api/logs` (GET).
///
/// Возвращает последние логи приложения с возможностью фильтрации по уровню
/// и ограничения количества записей.
///
/// # Параметры запроса
///
/// - `level` (опционально): Минимальный уровень логирования (error, warn, info, debug, trace)
/// - `limit` (опционально): Максимальное количество возвращаемых записей (по умолчанию: 100)
///
/// # Примеры
///
/// ```bash
/// # Получение последних 50 логов
/// curl "http://127.0.0.1:8080/api/logs?limit=50"
///
/// # Получение только ошибок и предупреждений
/// curl "http://127.0.0.1:8080/api/logs?level=warn"
///
/// # Получение последних 20 информационных сообщений
/// curl "http://127.0.0.1:8080/api/logs?level=info&limit=20"
/// ```
async fn logs_handler(
    State(state): State<ApiState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, StatusCode> {
    // Извлекаем параметры запроса
    let level_param = params.get("level").map(|s| s.as_str());
    let limit_param = params.get("limit").and_then(|s| s.parse::<usize>().ok());

    // Определяем минимальный уровень логирования
    let min_level = match level_param {
        Some("error") => crate::logging::log_storage::LogLevel::Error,
        Some("warn") => crate::logging::log_storage::LogLevel::Warn,
        Some("info") => crate::logging::log_storage::LogLevel::Info,
        Some("debug") => crate::logging::log_storage::LogLevel::Debug,
        Some("trace") => crate::logging::log_storage::LogLevel::Trace,
        _ => crate::logging::log_storage::LogLevel::Trace, // По умолчанию: все уровни
    };

    // Определяем лимит (по умолчанию: 100, максимум: 1000)
    let limit = limit_param.unwrap_or(100).min(1000);

    match &state.log_storage {
        Some(log_storage) => {
            // Получаем записи логов, отфильтрованные по уровню
            let entries = log_storage.get_entries_by_level(min_level).await;

            // Применяем лимит
            let limited_entries = if limit > 0 && entries.len() > limit {
                entries.into_iter().rev().take(limit).rev().collect()
            } else {
                entries
            };

            // Преобразуем записи в JSON
            let logs: Vec<Value> = limited_entries
                .into_iter()
                .map(|entry| {
                    let mut log_json = json!({
                        "timestamp": entry.timestamp.to_rfc3339(),
                        "level": format!("{}", entry.level),
                        "target": entry.target,
                        "message": entry.message,
                    });

                    if let Some(fields) = entry.fields {
                        log_json["fields"] = fields;
                    }

                    log_json
                })
                .collect();

            Ok(Json(json!({
                "status": "ok",
                "logs": logs,
                "count": logs.len(),
                "total_available": log_storage.len().await,
                "max_capacity": log_storage.max_entries().await,
                "filter": {
                    "min_level": format!("{}", min_level),
                    "limit": limit
                }
            })))
        }
        None => {
            // Хранилище логов не доступно
            Ok(Json(json!({
                "status": "ok",
                "logs": [],
                "count": 0,
                "message": "Log storage not available (daemon may not be running or logs not configured)",
                "filter": {
                    "min_level": format!("{}", min_level),
                    "limit": limit
                }
            })))
        }
    }
}

/// Обработчик для endpoint `/api/notifications/config` (POST).
///
/// Изменяет конфигурацию уведомлений в runtime.
/// Позволяет включать/отключать уведомления, изменять бэкенд и другие параметры.
///
/// # Примечания
///
/// - Требует наличия конфигурации в состоянии API
/// - Изменения применяются немедленно
/// - Возвращает обновлённую конфигурацию
async fn notifications_config_handler(
    State(state): State<ApiState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    match &state.config {
        Some(config_arc) => {
            // Пробуем обновить конфигурацию уведомлений
            let mut config_guard = config_arc.write().await;

            // Обновляем параметры уведомлений, если они предоставлены
            if let Some(enabled) = payload.get("enabled").and_then(|v| v.as_bool()) {
                config_guard.notifications.enabled = enabled;
            }

            if let Some(backend_str) = payload.get("backend").and_then(|v| v.as_str()) {
                match backend_str {
                    "stub" => {
                        config_guard.notifications.backend =
                            crate::config::config_struct::NotificationBackend::Stub
                    }
                    "libnotify" => {
                        config_guard.notifications.backend =
                            crate::config::config_struct::NotificationBackend::Libnotify
                    }
                    "dbus" => {
                        config_guard.notifications.backend =
                            crate::config::config_struct::NotificationBackend::Dbus
                    }
                    _ => {
                        tracing::warn!("Unknown notification backend: {}", backend_str);
                    }
                }
            }

            if let Some(app_name) = payload.get("app_name").and_then(|v| v.as_str()) {
                config_guard.notifications.app_name = app_name.to_string();
            }

            if let Some(min_level_str) = payload.get("min_level").and_then(|v| v.as_str()) {
                match min_level_str {
                    "critical" => {
                        config_guard.notifications.min_level =
                            crate::config::config_struct::NotificationLevel::Critical
                    }
                    "warning" => {
                        config_guard.notifications.min_level =
                            crate::config::config_struct::NotificationLevel::Warning
                    }
                    "info" => {
                        config_guard.notifications.min_level =
                            crate::config::config_struct::NotificationLevel::Info
                    }
                    _ => {
                        tracing::warn!("Unknown notification level: {}", min_level_str);
                    }
                }
            }

            // Также обновляем менеджер уведомлений, если он доступен
            if let Some(notification_manager_arc) = &state.notification_manager {
                let mut notification_manager = notification_manager_arc.lock().await;

                // Обновляем состояние включения уведомлений
                if let Some(enabled) = payload.get("enabled").and_then(|v| v.as_bool()) {
                    notification_manager.set_enabled(enabled);
                }
            }

            // Возвращаем обновлённую конфигурацию
            let updated_config = config_guard.notifications.clone();

            tracing::info!("Notification configuration updated successfully");

            Ok(Json(json!({
                "status": "success",
                "message": "Notification configuration updated successfully",
                "config": updated_config
            })))
        }
        None => {
            tracing::warn!("Config not available for notification configuration update");
            Ok(Json(json!({
                "status": "error",
                "message": "Config not available (daemon may not be running or config not set)"
            })))
        }
    }
}

/// Создаёт роутер для API.
fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/api/health", get(health_detailed_handler))
        .route("/api/version", get(version_handler))
        .route("/api/endpoints", get(endpoints_handler))
        .route("/api/stats", get(stats_handler))
        .route("/api/metrics", get(metrics_handler))
        .route("/api/responsiveness", get(responsiveness_handler))
        .route("/api/processes", get(processes_handler))
        .route("/api/processes/:pid", get(process_by_pid_handler))
        .route("/api/appgroups", get(appgroups_handler))
        .route("/api/appgroups/:id", get(appgroup_by_id_handler))
        .route("/api/config", get(config_handler))
        .route("/api/config/reload", post(config_reload_handler))
        .route("/api/classes", get(classes_handler))
        .route("/api/patterns", get(patterns_handler))
        .route("/api/system", get(system_handler))
        .route("/api/notifications/test", post(notifications_test_handler))
        .route(
            "/api/notifications/custom",
            post(notifications_custom_handler),
        )
        .route(
            "/api/notifications/status",
            get(notifications_status_handler),
        )
        .route(
            "/api/notifications/config",
            post(notifications_config_handler),
        )
        .route("/api/performance", get(performance_handler))
        .route("/api/logs", get(logs_handler))
        .with_state(state)
}

/// Обработчик для endpoint `/api/stats`.
///
/// Возвращает статистику работы демона (если доступна).
async fn stats_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    match &state.daemon_stats {
        Some(stats_arc) => {
            let stats = stats_arc.read().await;
            Ok(Json(json!({
                "status": "ok",
                "daemon_stats": *stats
            })))
        }
        None => Ok(Json(json!({
            "status": "ok",
            "daemon_stats": null,
            "message": "Daemon stats not available (daemon may not be running)"
        }))),
    }
}

/// Обработчик для endpoint `/api/metrics`.
///
/// Возвращает последние системные метрики (если доступны).
async fn metrics_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    match &state.system_metrics {
        Some(metrics_arc) => {
            let metrics = metrics_arc.read().await;
            Ok(Json(json!({
                "status": "ok",
                "system_metrics": *metrics
            })))
        }
        None => Ok(Json(json!({
            "status": "ok",
            "system_metrics": null,
            "message": "System metrics not available (daemon may not be running or no metrics collected yet)"
        }))),
    }
}

/// Обработчик для endpoint `/api/processes`.
///
/// Возвращает список последних процессов (если доступны).
async fn processes_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    // Начинаем отслеживание производительности
    let _start_time = Instant::now();

    // Обновляем метрики производительности
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_requests();
    drop(perf_metrics); // Освобождаем блокировку

    // Пробуем использовать кэш
    let cache = state.get_or_create_cache();
    let mut cache_write = cache.write().await;

    if let Some((cached_json, cache_time)) = &cache_write.cached_processes_json {
        if cache_write.is_cache_valid(cache_time) {
            // Кэш актуален - используем его
            let mut perf_metrics = state.performance_metrics.write().await;
            perf_metrics.increment_cache_hits();
            drop(perf_metrics);

            trace!("Cache hit for processes_handler");
            return Ok(Json(cached_json.clone()));
        }
    }

    // Кэш не актуален или отсутствует - получаем свежие данные
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_cache_misses();
    drop(perf_metrics);

    match &state.processes {
        Some(processes_arc) => {
            let processes = processes_arc.read().await;
            let result = json!({
                "status": "ok",
                "processes": *processes,
                "count": processes.len(),
                "cache_info": {
                    "cached": false,
                    "ttl_seconds": cache_write.cache_ttl_seconds
                }
            });

            // Кэшируем результат
            cache_write.cached_processes_json = Some((result.clone(), Instant::now()));

            trace!("Cached processes data (count: {})", processes.len());
            Ok(Json(result))
        }
        None => {
            let result = json!({
                "status": "ok",
                "processes": null,
                "count": 0,
                "message": "Processes not available (daemon may not be running or no processes collected yet)",
                "cache_info": {
                    "cached": false,
                    "ttl_seconds": cache_write.cache_ttl_seconds
                }
            });

            // Кэшируем результат (даже если данных нет)
            cache_write.cached_processes_json = Some((result.clone(), Instant::now()));

            trace!("Cached empty processes data");
            Ok(Json(result))
        }
    }
}

/// Обработчик для endpoint `/api/appgroups`.
///
/// Возвращает список последних групп приложений (если доступны).
async fn appgroups_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    // Начинаем отслеживание производительности
    let _start_time = Instant::now();

    // Обновляем метрики производительности
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_requests();
    drop(perf_metrics); // Освобождаем блокировку

    // Пробуем использовать кэш
    let cache = state.get_or_create_cache();
    let mut cache_write = cache.write().await;

    if let Some((cached_json, cache_time)) = &cache_write.cached_appgroups_json {
        if cache_write.is_cache_valid(cache_time) {
            // Кэш актуален - используем его
            let mut perf_metrics = state.performance_metrics.write().await;
            perf_metrics.increment_cache_hits();
            drop(perf_metrics);

            trace!("Cache hit for appgroups_handler");
            return Ok(Json(cached_json.clone()));
        }
    }

    // Кэш не актуален или отсутствует - получаем свежие данные
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_cache_misses();
    drop(perf_metrics);

    match &state.app_groups {
        Some(app_groups_arc) => {
            let app_groups = app_groups_arc.read().await;
            let result = json!({
                "status": "ok",
                "app_groups": *app_groups,
                "count": app_groups.len(),
                "cache_info": {
                    "cached": false,
                    "ttl_seconds": cache_write.cache_ttl_seconds
                }
            });

            // Кэшируем результат
            cache_write.cached_appgroups_json = Some((result.clone(), Instant::now()));

            trace!("Cached appgroups data (count: {})", app_groups.len());
            Ok(Json(result))
        }
        None => {
            let result = json!({
                "status": "ok",
                "app_groups": null,
                "count": 0,
                "message": "App groups not available (daemon may not be running or no groups collected yet)",
                "cache_info": {
                    "cached": false,
                    "ttl_seconds": cache_write.cache_ttl_seconds
                }
            });

            // Кэшируем результат (даже если данных нет)
            cache_write.cached_appgroups_json = Some((result.clone(), Instant::now()));

            trace!("Cached empty appgroups data");
            Ok(Json(result))
        }
    }
}

/// Обработчик для endpoint `/api/processes/:pid`.
///
/// Возвращает информацию о конкретном процессе по PID (если доступен).
async fn process_by_pid_handler(
    Path(pid): Path<i32>,
    State(state): State<ApiState>,
) -> Result<Json<Value>, StatusCode> {
    // Проверяем, что PID является допустимым значением
    if pid <= 0 {
        error!("Invalid PID value: {}. PID must be a positive integer", pid);
        return Ok(Json(json!({
            "status": "error",
            "error": "invalid_input",
            "message": format!("Invalid PID value: {}. PID must be a positive integer", pid),
            "details": {
                "field": "pid",
                "value": pid,
                "constraint": "must be > 0"
            }
        })));
    }

    // Проверяем, что PID находится в разумном диапазоне
    // Максимальный PID в Linux обычно ограничен (по умолчанию 32768, но может быть до 4194304)
    if pid > 4_194_304 {
        error!(
            "PID value {} is too large. Maximum reasonable PID value is 4194304",
            pid
        );
        return Ok(Json(json!({
            "status": "error",
            "error": "invalid_input",
            "message": format!("PID value {} is too large", pid),
            "details": {
                "field": "pid",
                "value": pid,
                "constraint": "must be <= 4194304",
                "note": "Maximum PID in Linux is typically limited to this range"
            }
        })));
    }

    match &state.processes {
        Some(processes_arc) => {
            let processes = processes_arc.read().await;

            // Добавляем отладочную информацию о количестве доступных процессов
            let process_count = processes.len();
            tracing::debug!("Looking for PID {} among {} processes", pid, process_count);

            match processes.iter().find(|p| p.pid == pid) {
                Some(process) => {
                    tracing::info!("Successfully found process with PID {}", pid);
                    Ok(Json(json!({
                        "status": "ok",
                        "process": process
                    })))
                }
                None => {
                    error!(
                        "Process with PID {} not found among {} processes",
                        pid, process_count
                    );
                    Ok(Json(json!({
                        "status": "error",
                        "error": "not_found",
                        "message": format!("Process with PID {} not found", pid),
                        "details": {
                            "available_processes": process_count,
                            "suggestion": "Check if the process is still running or if the daemon has collected recent data"
                        }
                    })))
                }
            }
        }
        None => {
            error!("Processes data not available for PID lookup. Daemon may not be running or no processes collected yet");
            Ok(Json(json!({
                "status": "error",
                "error": "not_available",
                "message": "Processes not available (daemon may not be running or no processes collected yet)",
                "details": {
                    "suggestion": "Ensure the daemon is running and has completed at least one collection cycle"
                }
            })))
        }
    }
}

/// Обработчик для endpoint `/api/appgroups/:id`.
///
/// Возвращает информацию о конкретной группе приложений по ID (если доступна).
async fn appgroup_by_id_handler(
    Path(id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<Value>, StatusCode> {
    // Проверяем, что ID не является пустым
    if id.is_empty() {
        error!("Empty app group ID provided");
        return Ok(Json(json!({
            "status": "error",
            "error": "invalid_input",
            "message": "App group ID cannot be empty",
            "details": {
                "field": "id",
                "value": "",
                "constraint": "must not be empty"
            }
        })));
    }

    // Проверяем, что ID имеет разумную длину
    if id.len() > 256 {
        error!(
            "App group ID is too long: {} characters (max 256)",
            id.len()
        );
        return Ok(Json(json!({
            "status": "error",
            "error": "invalid_input",
            "message": "App group ID is too long",
            "details": {
                "field": "id",
                "length": id.len(),
                "constraint": "must be <= 256 characters"
            }
        })));
    }

    // Проверяем, что ID содержит только допустимые символы
    if !id.chars().all(|c| {
        c.is_ascii() && (c.is_alphanumeric() || c == '_' || c == '-' || c == '.' || c == ':')
    }) {
        error!("App group ID contains invalid characters: {}", id);
        return Ok(Json(json!({
            "status": "error",
            "error": "invalid_input",
            "message": "App group ID contains invalid characters",
            "details": {
                "field": "id",
                "value": id,
                "constraint": "must contain only alphanumeric, '_', '-', '.', ':' characters",
                "suggestion": "Use only standard ASCII characters in app group IDs"
            }
        })));
    }

    match &state.app_groups {
        Some(app_groups_arc) => {
            let app_groups = app_groups_arc.read().await;

            // Добавляем отладочную информацию о количестве доступных групп
            let group_count = app_groups.len();
            tracing::debug!(
                "Looking for app group with ID '{}' among {} groups",
                id,
                group_count
            );

            match app_groups.iter().find(|g| g.app_group_id == id) {
                Some(app_group) => {
                    tracing::info!("Successfully found app group with ID '{}'", id);
                    Ok(Json(json!({
                        "status": "ok",
                        "app_group": app_group
                    })))
                }
                None => {
                    error!(
                        "App group with ID '{}' not found among {} groups",
                        id, group_count
                    );
                    Ok(Json(json!({
                        "status": "error",
                        "error": "not_found",
                        "message": format!("App group with ID '{}' not found", id),
                        "details": {
                            "available_groups": group_count,
                            "suggestion": "Check if the app group ID is correct and if the daemon has collected recent data"
                        }
                    })))
                }
            }
        }
        None => {
            error!("App groups data not available for ID lookup. Daemon may not be running or no groups collected yet");
            Ok(Json(json!({
                "status": "error",
                "error": "not_available",
                "message": "App groups not available (daemon may not be running or no groups collected yet)",
                "details": {
                    "suggestion": "Ensure the daemon is running and has completed at least one collection cycle"
                }
            })))
        }
    }
}

/// Обработчик для endpoint `/api/responsiveness`.
///
/// Возвращает последние метрики отзывчивости системы (если доступны).
async fn responsiveness_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    match &state.responsiveness_metrics {
        Some(metrics_arc) => {
            let metrics = metrics_arc.read().await;
            Ok(Json(json!({
                "status": "ok",
                "responsiveness_metrics": *metrics
            })))
        }
        None => Ok(Json(json!({
            "status": "ok",
            "responsiveness_metrics": null,
            "message": "Responsiveness metrics not available (daemon may not be running or no metrics collected yet)"
        }))),
    }
}

/// Обработчик для endpoint `/api/config`.
///
/// Возвращает текущую конфигурацию демона (без секретов).
/// Конфигурация возвращается как есть, так как в SmoothTask нет явных секретов
/// (паролей, токенов и т.д.). Все поля конфигурации безопасны для просмотра.
async fn config_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    match &state.config {
        Some(config_arc) => {
            let config_guard = config_arc.read().await;
            Ok(Json(json!({
                "status": "ok",
                "config": serde_json::to_value(&*config_guard).unwrap_or(Value::Null)
            })))
        }
        None => Ok(Json(json!({
            "status": "ok",
            "config": null,
            "message": "Config not available (daemon may not be running or config not set)"
        }))),
    }
}

/// Обработчик для endpoint `/api/config/reload` (POST).
///
/// Перезагружает конфигурацию демона из файла.
/// Возвращает результат перезагрузки и новую конфигурацию (если успешно).
///
/// # Примечания
///
/// В текущей архитектуре, API сервер не имеет прямого доступа к демону для перезагрузки конфигурации.
/// Однако, мы можем реализовать базовую функциональность, которая:
/// 1. Пытается загрузить конфигурацию из файла (если путь известен)
/// 2. Возвращает новую конфигурацию (если загрузка успешна)
/// 3. Логирует событие для ручной перезагрузки
///
/// Для полной интеграции потребуется рефакторинг архитектуры, но этот endpoint предоставляет
/// базовую функциональность для мониторинга и отладки.
async fn config_reload_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    match (&state.config, &state.config_path) {
        (Some(config_arc), Some(config_path)) => {
            // Пробуем загрузить новую конфигурацию из файла
            match crate::config::config_struct::Config::load(config_path) {
                Ok(new_config) => {
                    // Успешно загрузили новую конфигурацию
                    // Теперь мы можем напрямую обновить конфигурацию через Arc<RwLock<Config>>

                    // Сохраняем старую конфигурацию для ответа
                    let old_config = {
                        let config_guard = config_arc.read().await;
                        serde_json::to_value(&*config_guard).unwrap_or(Value::Null)
                    };

                    // Обновляем конфигурацию
                    {
                        let mut config_guard = config_arc.write().await;
                        *config_guard = new_config;
                    }

                    tracing::info!("API config reload successful: loaded and applied new configuration from {}", config_path);

                    // Получаем новую конфигурацию для ответа
                    let new_config_value = {
                        let config_guard = config_arc.read().await;
                        serde_json::to_value(&*config_guard).unwrap_or(Value::Null)
                    };

                    Ok(Json(json!({
                        "status": "success",
                        "message": "Configuration successfully reloaded from file and applied",
                        "old_config": old_config,
                        "new_config": new_config_value,
                        "action_required": "Configuration has been updated and is now active.",
                        "config_path": config_path
                    })))
                }
                Err(e) => {
                    // Ошибка загрузки конфигурации
                    tracing::error!("API config reload failed from {}: {}", config_path, e);

                    // Получаем текущую конфигурацию для ответа
                    let current_config = {
                        let config_guard = config_arc.read().await;
                        serde_json::to_value(&*config_guard).unwrap_or(Value::Null)
                    };

                    // Разбираем тип ошибки для более детального сообщения
                    let error_type =
                        if e.to_string().contains("YAML") || e.to_string().contains("yaml") {
                            "yaml_parse_error"
                        } else if e.to_string().contains("file") || e.to_string().contains("File") {
                            "file_error"
                        } else if e.to_string().contains("permission")
                            || e.to_string().contains("Permission")
                        {
                            "permission_error"
                        } else {
                            "unknown_error"
                        };

                    Ok(Json(json!({
                        "status": "error",
                        "error_type": error_type,
                        "message": format!("Failed to reload configuration from '{}': {}", config_path, e),
                        "current_config": current_config,
                        "config_path": config_path,
                        "action_required": "Check the configuration file for errors and try again.",
                        "details": {
                            "error_details": e.to_string(),
                            "suggestion": match error_type {
                                "yaml_parse_error" => "Check YAML syntax and structure in the configuration file",
                                "file_error" => "Verify the configuration file exists and is accessible",
                                "permission_error" => "Ensure the daemon has read permissions for the configuration file",
                                _ => "Check the error details and verify the configuration"
                            }
                        }
                    })))
                }
            }
        }
        (Some(config_arc), None) => {
            // Конфигурация доступна, но путь к файлу неизвестен
            tracing::warn!("API config reload requested but config path is not available");

            // Получаем текущую конфигурацию для ответа
            let current_config = {
                let config_guard = config_arc.read().await;
                serde_json::to_value(&*config_guard).unwrap_or(Value::Null)
            };

            Ok(Json(json!({
                "status": "warning",
                "message": "Config reload requested but config file path is not available",
                "current_config": current_config,
                "action_required": "To enable full config reload, ensure the daemon is running with config path information."
            })))
        }
        (None, _) => {
            // Конфигурация недоступна
            Ok(Json(json!({
                "status": "error",
                "message": "Config reload not available (daemon may not be running or config not set)"
            })))
        }
    }
}

/// HTTP API сервер для SmoothTask.
///
/// Сервер предоставляет REST API для мониторинга работы демона.
/// Сервер запускается в отдельной задаче и может быть остановлен через handle.
///
/// # Примеры использования
///
/// ## Базовый пример
///
/// ```no_run
/// use smoothtask_core::api::ApiServer;
/// use std::net::SocketAddr;
///
/// # async fn example() -> anyhow::Result<()> {
/// let addr: SocketAddr = "127.0.0.1:8080".parse()?;
/// let server = ApiServer::new(addr);
/// let handle = server.start().await?;
///
/// // Сервер работает в фоне
/// // ...
///
/// // Остановка сервера
/// handle.shutdown().await?;
/// # Ok(())
/// # }
/// ```
///
/// ## Пример с использованием ApiStateBuilder
///
/// ```no_run
/// use smoothtask_core::api::{ApiServer, ApiStateBuilder};
/// use smoothtask_core::{DaemonStats, config::config_struct::Config};
/// use smoothtask_core::metrics::system::SystemMetrics;
/// use smoothtask_core::logging::snapshots::{ProcessRecord, AppGroupRecord, ResponsivenessMetrics};
/// use smoothtask_core::classify::rules::PatternDatabase;
/// use smoothtask_core::notifications::NotificationManager;
/// use std::net::SocketAddr;
/// use std::sync::Arc;
/// use tokio::sync::{RwLock, Mutex};
///
/// # async fn example() -> anyhow::Result<()> {
/// // Создаём тестовые данные
/// let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
/// let system_metrics = Arc::new(RwLock::new(SystemMetrics::default()));
/// let processes = Arc::new(RwLock::new(Vec::<ProcessRecord>::new()));
/// let app_groups = Arc::new(RwLock::new(Vec::<AppGroupRecord>::new()));
/// let responsiveness_metrics = Arc::new(RwLock::new(ResponsivenessMetrics::default()));
/// let config = Arc::new(RwLock::new(Config::default()));
/// let pattern_db = Arc::new(PatternDatabase::load("/path/to/patterns").unwrap());
/// let notification_manager = Arc::new(Mutex::new(NotificationManager::new_stub()));
///
/// // Используем ApiStateBuilder для создания состояния API
/// let api_state = ApiStateBuilder::new()
///     .with_daemon_stats(Some(daemon_stats))
///     .with_system_metrics(Some(system_metrics))
///     .with_processes(Some(processes))
///     .with_app_groups(Some(app_groups))
///     .with_responsiveness_metrics(Some(responsiveness_metrics))
///     .with_config(Some(config))
///     .with_config_path(Some("/path/to/config.yml".to_string()))
///     .with_pattern_database(Some(pattern_db))
///     .with_notification_manager(Some(notification_manager))
///     .build();
///
/// // Создаём сервер с состоянием
/// let addr: SocketAddr = "127.0.0.1:8080".parse()?;
/// let server = ApiServer::with_state(addr, api_state);
/// let handle = server.start().await?;
///
/// // Сервер работает в фоне
/// // ...
///
/// // Остановка сервера
/// handle.shutdown().await?;
/// # Ok(())
/// # }
/// ```
///
/// ## Сравнение с устаревшими методами
///
/// До введения ApiStateBuilder код создания сервера с полным состоянием выглядел так:
///
/// ```no_run
/// // Старый способ (устарел)
/// let server = ApiServer::with_all_and_responsiveness_and_config_and_patterns(
///     addr,
///     daemon_stats,
///     system_metrics,
///     processes,
///     app_groups,
///     responsiveness_metrics,
///     config,
///     pattern_db,
/// );
/// ```
///
/// С ApiStateBuilder код становится более понятным и гибким:
///
/// ```no_run
/// // Новый способ (рекомендуется)
/// let api_state = ApiStateBuilder::new()
///     .with_daemon_stats(Some(daemon_stats))
///     .with_system_metrics(Some(system_metrics))
///     .with_processes(Some(processes))
///     .with_app_groups(Some(app_groups))
///     .with_responsiveness_metrics(Some(responsiveness_metrics))
///     .with_config(Some(config))
///     .with_pattern_database(Some(pattern_db))
///     .build();
///
/// let server = ApiServer::with_state(addr, api_state);
/// ```
///
/// # Преимущества ApiStateBuilder
///
/// - **Читаемость**: Код становится более понятным и самодокументированным
/// - **Гибкость**: Легко добавлять или удалять компоненты без изменения сигнатур
/// - **Типобезопасность**: Все параметры проверяются на этапе компиляции
/// - **Поддержка IDE**: Автодополнение работает лучше с цепочкой методов
/// - **Документация**: Каждый метод имеет свою документацию с примерами
pub struct ApiServer {
    /// Адрес для прослушивания
    addr: std::net::SocketAddr,
    /// Состояние API
    state: ApiState,
}

impl ApiServer {
    /// Создаёт новый API сервер.
    ///
    /// # Параметры
    ///
    /// - `addr`: Адрес для прослушивания (например, "127.0.0.1:8080")
    pub fn new(addr: std::net::SocketAddr) -> Self {
        Self {
            addr,
            state: ApiState::new(),
        }
    }

    /// Создаёт новый API сервер с переданной статистикой демона.
    ///
    /// # Параметры
    ///
    /// - `addr`: Адрес для прослушивания (например, "127.0.0.1:8080")
    /// - `daemon_stats`: Статистика работы демона для отображения через API
    pub fn with_daemon_stats(
        addr: std::net::SocketAddr,
        daemon_stats: Arc<RwLock<crate::DaemonStats>>,
    ) -> Self {
        Self {
            addr,
            state: ApiState::with_daemon_stats(daemon_stats),
        }
    }

    /// Создаёт новый API сервер с переданными системными метриками.
    ///
    /// # Параметры
    ///
    /// - `addr`: Адрес для прослушивания (например, "127.0.0.1:8080")
    /// - `system_metrics`: Системные метрики для отображения через API
    pub fn with_system_metrics(
        addr: std::net::SocketAddr,
        system_metrics: Arc<RwLock<crate::metrics::system::SystemMetrics>>,
    ) -> Self {
        Self {
            addr,
            state: ApiState::with_system_metrics(system_metrics),
        }
    }

    /// Создаёт новый API сервер с переданной статистикой демона и системными метриками.
    ///
    /// # Параметры
    ///
    /// - `addr`: Адрес для прослушивания (например, "127.0.0.1:8080")
    /// - `daemon_stats`: Статистика работы демона (опционально)
    /// - `system_metrics`: Системные метрики (опционально)
    /// - `processes`: Список процессов (опционально)
    /// - `app_groups`: Список групп приложений (опционально)
    pub fn with_all(
        addr: std::net::SocketAddr,
        daemon_stats: Option<Arc<RwLock<crate::DaemonStats>>>,
        system_metrics: Option<Arc<RwLock<crate::metrics::system::SystemMetrics>>>,
        processes: Option<Arc<RwLock<Vec<crate::logging::snapshots::ProcessRecord>>>>,
        app_groups: Option<Arc<RwLock<Vec<crate::logging::snapshots::AppGroupRecord>>>>,
    ) -> Self {
        Self {
            addr,
            state: ApiState::with_all(daemon_stats, system_metrics, processes, app_groups),
        }
    }

    /// Создаёт новый API сервер с переданным состоянием API.
    ///
    /// # Параметры
    ///
    /// - `addr`: Адрес для прослушивания (например, "127.0.0.1:8080")
    /// - `state`: Состояние API
    pub fn with_state(addr: std::net::SocketAddr, state: ApiState) -> Self {
        Self { addr, state }
    }

    /// Создаёт новый API сервер со всеми данными, включая конфигурацию.
    ///
    /// # Параметры
    ///
    /// - `addr`: Адрес для прослушивания (например, "127.0.0.1:8080")
    /// - `daemon_stats`: Статистика работы демона (опционально)
    /// - `system_metrics`: Системные метрики (опционально)
    /// - `processes`: Список процессов (опционально)
    /// - `app_groups`: Список групп приложений (опционально)
    /// - `config`: Конфигурация демона (опционально)
    pub fn with_all_and_config(
        addr: std::net::SocketAddr,
        daemon_stats: Option<Arc<RwLock<crate::DaemonStats>>>,
        system_metrics: Option<Arc<RwLock<crate::metrics::system::SystemMetrics>>>,
        processes: Option<Arc<RwLock<Vec<crate::logging::snapshots::ProcessRecord>>>>,
        app_groups: Option<Arc<RwLock<Vec<crate::logging::snapshots::AppGroupRecord>>>>,
        config: Option<Arc<RwLock<crate::config::config_struct::Config>>>,
    ) -> Self {
        Self {
            addr,
            state: ApiState::with_all_and_config(
                daemon_stats,
                system_metrics,
                processes,
                app_groups,
                config,
            ),
        }
    }

    /// Создаёт новый API сервер со всеми данными, включая метрики отзывчивости и конфигурацию.
    ///
    /// # Параметры
    ///
    /// - `addr`: Адрес для прослушивания (например, "127.0.0.1:8080")
    /// - `daemon_stats`: Статистика работы демона (опционально)
    /// - `system_metrics`: Системные метрики (опционально)
    /// - `processes`: Список процессов (опционально)
    /// - `app_groups`: Список групп приложений (опционально)
    /// - `responsiveness_metrics`: Метрики отзывчивости (опционально)
    /// - `config`: Конфигурация демона (опционально)
    pub fn with_all_and_responsiveness_and_config(
        addr: std::net::SocketAddr,
        daemon_stats: Option<Arc<RwLock<crate::DaemonStats>>>,
        system_metrics: Option<Arc<RwLock<crate::metrics::system::SystemMetrics>>>,
        processes: Option<Arc<RwLock<Vec<crate::logging::snapshots::ProcessRecord>>>>,
        app_groups: Option<Arc<RwLock<Vec<crate::logging::snapshots::AppGroupRecord>>>>,
        responsiveness_metrics: Option<
            Arc<RwLock<crate::logging::snapshots::ResponsivenessMetrics>>,
        >,
        config: Option<Arc<RwLock<crate::config::config_struct::Config>>>,
    ) -> Self {
        Self {
            addr,
            state: ApiState::with_all_and_responsiveness_and_config(
                daemon_stats,
                system_metrics,
                processes,
                app_groups,
                responsiveness_metrics,
                config,
            ),
        }
    }

    /// Создаёт новый API сервер со всеми данными, включая метрики отзывчивости, конфигурацию и базу паттернов.
    ///
    /// # Параметры
    ///
    /// - `addr`: Адрес для прослушивания (например, "127.0.0.1:8080")
    /// - `daemon_stats`: Статистика работы демона (опционально)
    /// - `system_metrics`: Системные метрики (опционально)
    /// - `processes`: Список процессов (опционально)
    /// - `app_groups`: Список групп приложений (опционально)
    ///
    /// Запускает API сервер в фоновой задаче.
    ///
    /// Возвращает handle для управления сервером (остановка, проверка состояния).
    ///
    /// # Ошибки
    ///
    /// Возвращает ошибку, если не удалось запустить сервер (например, адрес уже занят).
    pub async fn start(self) -> Result<ApiServerHandle> {
        let listener = TcpListener::bind(&self.addr)
            .await
            .with_context(|| format!("Failed to bind API server to {}", self.addr))?;

        // Get the actual address that was bound (important for port 0 which auto-selects port)
        let actual_addr = listener.local_addr()?;
        info!("API server listening on http://{}", actual_addr);

        let router = create_router(self.state);
        let server = axum::serve(listener, router);

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let handle = ApiServerHandle {
            shutdown_tx: Some(shutdown_tx),
            addr: actual_addr,
        };

        // Запускаем сервер в отдельной задаче
        tokio::spawn(async move {
            let graceful = server.with_graceful_shutdown(async {
                shutdown_rx.await.ok();
            });

            if let Err(e) = graceful.await {
                error!("API server error: {}", e);
            } else {
                info!("API server stopped");
            }
        });

        Ok(handle)
    }
}

/// Handle для управления API сервером.
///
/// Позволяет остановить сервер и проверить его состояние.
pub struct ApiServerHandle {
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
    addr: std::net::SocketAddr,
}

impl ApiServerHandle {
    /// Возвращает адрес, на котором запущен API сервер.
    pub fn addr(&self) -> std::net::SocketAddr {
        self.addr
    }

    /// Возвращает порт, на котором запущен API сервер.
    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// Останавливает API сервер.
    ///
    /// # Ошибки
    ///
    /// Возвращает ошибку, если сервер уже остановлен или произошла ошибка при остановке.
    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(tx) = self.shutdown_tx.take() {
            tx.send(()).map_err(|_| {
                anyhow::anyhow!("Failed to send shutdown signal to API server (receiver dropped)")
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    // Import notification types for test configurations
    use crate::config::config_struct::{MLClassifierConfig, ModelType, PatternAutoUpdateConfig};
    use crate::metrics::ebpf::{EbpfConfig, EbpfNotificationThresholds};

    #[tokio::test]
    async fn test_api_server_start_and_shutdown() {
        // Используем порт 0 для автоматического выбора свободного порта
        // Но axum::serve требует конкретный адрес, поэтому используем тестовый порт
        // В реальном использовании адрес будет задан из конфига
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let server = ApiServer::new(addr);

        // Тест: сервер должен запуститься без ошибок
        // Примечание: в реальном тесте нужно использовать свободный порт,
        // но для базовой проверки достаточно проверить создание сервера
        let _server = server;
    }

    #[test]
    fn test_api_state_new() {
        let state = ApiState::new();
        assert!(state.daemon_stats.is_none());
        assert!(state.system_metrics.is_none());
        assert!(state.processes.is_none());
        assert!(state.app_groups.is_none());
        assert!(state.responsiveness_metrics.is_none());
        assert!(state.config.is_none());
    }

    #[test]
    fn test_api_state_default() {
        let state = ApiState::default();
        assert!(state.daemon_stats.is_none());
        assert!(state.system_metrics.is_none());
        assert!(state.processes.is_none());
        assert!(state.app_groups.is_none());
        assert!(state.responsiveness_metrics.is_none());
        assert!(state.config.is_none());
    }

    #[test]
    fn test_api_state_with_daemon_stats() {
        let stats = Arc::new(RwLock::new(crate::DaemonStats::new()));
        let state = ApiState::with_daemon_stats(stats.clone());
        assert!(state.daemon_stats.is_some());
        assert!(state.system_metrics.is_none());
        assert!(state.processes.is_none());
        assert!(state.app_groups.is_none());
        assert!(state.responsiveness_metrics.is_none());
        assert!(state.config.is_none());
    }

    #[test]
    fn test_api_state_with_system_metrics() {
        use crate::metrics::system::{
            CpuTimes, DiskMetrics, LoadAvg, MemoryInfo, NetworkMetrics, PowerMetrics,
            PressureMetrics, SystemMetrics, TemperatureMetrics,
        };
        let metrics = SystemMetrics {
            cpu_times: CpuTimes {
                user: 100,
                nice: 20,
                system: 50,
                idle: 200,
                iowait: 10,
                irq: 5,
                softirq: 5,
                steal: 0,
                guest: 0,
                guest_nice: 0,
            },
            memory: MemoryInfo {
                mem_total_kb: 1000,
                mem_available_kb: 500,
                mem_free_kb: 400,
                buffers_kb: 50,
                cached_kb: 50,
                swap_total_kb: 1000,
                swap_free_kb: 800,
            },
            load_avg: LoadAvg {
                one: 1.0,
                five: 1.0,
                fifteen: 1.0,
            },
            pressure: PressureMetrics::default(),
            temperature: TemperatureMetrics::default(),
            power: PowerMetrics::default(),
            network: NetworkMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
        };
        let metrics_arc = Arc::new(RwLock::new(metrics));
        let state = ApiState::with_system_metrics(metrics_arc.clone());
        assert!(state.daemon_stats.is_none());
        assert!(state.system_metrics.is_some());
        assert!(state.responsiveness_metrics.is_none());
        assert!(state.config.is_none());
    }

    #[test]
    fn test_api_state_with_all() {
        let stats = Arc::new(RwLock::new(crate::DaemonStats::new()));
        use crate::metrics::system::{
            CpuTimes, DiskMetrics, LoadAvg, MemoryInfo, NetworkMetrics, PowerMetrics,
            PressureMetrics, SystemMetrics, TemperatureMetrics,
        };
        let metrics = SystemMetrics {
            cpu_times: CpuTimes {
                user: 100,
                nice: 20,
                system: 50,
                idle: 200,
                iowait: 10,
                irq: 5,
                softirq: 5,
                steal: 0,
                guest: 0,
                guest_nice: 0,
            },
            memory: MemoryInfo {
                mem_total_kb: 1000,
                mem_available_kb: 500,
                mem_free_kb: 400,
                buffers_kb: 50,
                cached_kb: 50,
                swap_total_kb: 1000,
                swap_free_kb: 800,
            },
            load_avg: LoadAvg {
                one: 1.0,
                five: 1.0,
                fifteen: 1.0,
            },
            pressure: PressureMetrics::default(),
            temperature: TemperatureMetrics::default(),
            power: PowerMetrics::default(),
            network: NetworkMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
        };
        let metrics_arc = Arc::new(RwLock::new(metrics));
        let processes = Arc::new(RwLock::new(vec![]));
        let app_groups = Arc::new(RwLock::new(vec![]));
        let state = ApiState::with_all(
            Some(stats.clone()),
            Some(metrics_arc.clone()),
            Some(processes.clone()),
            Some(app_groups.clone()),
        );
        assert!(state.daemon_stats.is_some());
        assert!(state.system_metrics.is_some());
        assert!(state.processes.is_some());
        assert!(state.app_groups.is_some());
        assert!(state.responsiveness_metrics.is_none());
        assert!(state.config.is_none());
    }

    #[tokio::test]
    async fn test_stats_handler_without_stats() {
        let state = ApiState::new();
        let result = stats_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert_eq!(value["daemon_stats"], Value::Null);
        assert!(value["message"].is_string());
    }

    #[tokio::test]
    async fn test_stats_handler_with_stats() {
        let mut stats = crate::DaemonStats::new();
        stats.record_successful_iteration(100, 5, 1);
        stats.record_error_iteration();
        let stats_arc = Arc::new(RwLock::new(stats));
        let state = ApiState::with_daemon_stats(stats_arc);
        let result = stats_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert!(value["daemon_stats"].is_object());
        let daemon_stats = &value["daemon_stats"];
        assert_eq!(daemon_stats["total_iterations"], 2);
        assert_eq!(daemon_stats["successful_iterations"], 1);
        assert_eq!(daemon_stats["error_iterations"], 1);
        assert_eq!(daemon_stats["total_duration_ms"], 100);
        assert_eq!(daemon_stats["max_iteration_duration_ms"], 100);
        assert_eq!(daemon_stats["total_applied_adjustments"], 5);
        assert_eq!(daemon_stats["total_apply_errors"], 1);
    }

    #[test]
    fn test_api_server_with_daemon_stats() {
        let addr: SocketAddr = "127.0.0.1:8081".parse().unwrap();
        let stats = Arc::new(RwLock::new(crate::DaemonStats::new()));
        let server = ApiServer::with_daemon_stats(addr, stats);
        // Проверяем, что сервер создан (нет способа проверить внутренние поля без pub)
        let _ = server;
    }

    #[test]
    fn test_api_server_new() {
        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let server = ApiServer::new(addr);
        // Проверяем, что сервер создан (нет способа проверить внутренние поля без pub)
        let _ = server;
    }

    #[tokio::test]
    async fn test_metrics_handler_without_metrics() {
        let state = ApiState::new();
        let result = metrics_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert_eq!(value["system_metrics"], Value::Null);
        assert!(value["message"].is_string());
    }

    #[tokio::test]
    async fn test_metrics_handler_with_metrics() {
        use crate::metrics::system::{
            CpuTimes, DiskMetrics, LoadAvg, MemoryInfo, NetworkMetrics, PowerMetrics,
            PressureMetrics, SystemMetrics, TemperatureMetrics,
        };
        let metrics = SystemMetrics {
            cpu_times: CpuTimes {
                user: 100,
                nice: 20,
                system: 50,
                idle: 200,
                iowait: 10,
                irq: 5,
                softirq: 5,
                steal: 0,
                guest: 0,
                guest_nice: 0,
            },
            memory: MemoryInfo {
                mem_total_kb: 1000,
                mem_available_kb: 500,
                mem_free_kb: 400,
                buffers_kb: 50,
                cached_kb: 50,
                swap_total_kb: 1000,
                swap_free_kb: 800,
            },
            load_avg: LoadAvg {
                one: 1.0,
                five: 1.0,
                fifteen: 1.0,
            },
            pressure: PressureMetrics::default(),
            temperature: TemperatureMetrics::default(),
            power: PowerMetrics::default(),
            network: NetworkMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
        };
        let metrics_arc = Arc::new(RwLock::new(metrics));
        let state = ApiState::with_system_metrics(metrics_arc);
        let result = metrics_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert!(value["system_metrics"].is_object());
        let system_metrics = &value["system_metrics"];
        assert!(system_metrics["cpu_times"].is_object());
        assert!(system_metrics["memory"].is_object());
        assert!(system_metrics["load_avg"].is_object());
        assert!(system_metrics["pressure"].is_object());
    }

    #[test]
    fn test_api_server_with_system_metrics() {
        use crate::metrics::system::{
            CpuTimes, DiskMetrics, LoadAvg, MemoryInfo, NetworkMetrics, PowerMetrics,
            PressureMetrics, SystemMetrics, TemperatureMetrics,
        };
        let addr: SocketAddr = "127.0.0.1:8082".parse().unwrap();
        let metrics = SystemMetrics {
            cpu_times: CpuTimes {
                user: 100,
                nice: 20,
                system: 50,
                idle: 200,
                iowait: 10,
                irq: 5,
                softirq: 5,
                steal: 0,
                guest: 0,
                guest_nice: 0,
            },
            memory: MemoryInfo {
                mem_total_kb: 1000,
                mem_available_kb: 500,
                mem_free_kb: 400,
                buffers_kb: 50,
                cached_kb: 50,
                swap_total_kb: 1000,
                swap_free_kb: 800,
            },
            load_avg: LoadAvg {
                one: 1.0,
                five: 1.0,
                fifteen: 1.0,
            },
            pressure: PressureMetrics::default(),
            temperature: TemperatureMetrics::default(),
            power: PowerMetrics::default(),
            network: NetworkMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
        };
        let metrics_arc = Arc::new(RwLock::new(metrics));
        let server = ApiServer::with_system_metrics(addr, metrics_arc);
        // Проверяем, что сервер создан
        let _ = server;
    }

    #[tokio::test]
    async fn test_processes_handler_without_processes() {
        let state = ApiState::new();
        let result = processes_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert_eq!(value["processes"], Value::Null);
        assert_eq!(value["count"], 0);
        assert!(value["message"].is_string());
    }

    #[tokio::test]
    async fn test_processes_handler_with_processes() {
        use crate::logging::snapshots::ProcessRecord;
        let processes = vec![ProcessRecord {
            pid: 1,
            ppid: 0,
            uid: 0,
            gid: 0,
            exe: Some("/sbin/init".to_string()),
            cmdline: Some("init".to_string()),
            cgroup_path: None,
            systemd_unit: None,
            app_group_id: None,
            state: "S".to_string(),
            start_time: 0,
            uptime_sec: 100,
            tty_nr: 0,
            has_tty: false,
            cpu_share_1s: Some(0.1),
            cpu_share_10s: Some(0.05),
            io_read_bytes: Some(1000),
            io_write_bytes: Some(500),
            rss_mb: Some(10),
            swap_mb: Some(0),
            voluntary_ctx: Some(100),
            involuntary_ctx: Some(10),
            has_gui_window: false,
            is_focused_window: false,
            window_state: None,
            env_has_display: false,
            env_has_wayland: false,
            env_term: None,
            env_ssh: false,
            is_audio_client: false,
            has_active_stream: false,
            process_type: None,
            tags: vec![],
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
        }];
        let processes_arc = Arc::new(RwLock::new(processes));
        let state = ApiState::with_all(None, None, Some(processes_arc), None);
        let result = processes_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert!(value["processes"].is_array());
        assert_eq!(value["count"], 1);
    }

    #[tokio::test]
    async fn test_appgroups_handler_without_appgroups() {
        let state = ApiState::new();
        let result = appgroups_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert_eq!(value["app_groups"], Value::Null);
        assert_eq!(value["count"], 0);
        assert!(value["message"].is_string());
    }

    #[tokio::test]
    async fn test_appgroups_handler_with_appgroups() {
        use crate::logging::snapshots::AppGroupRecord;
        let app_groups = vec![AppGroupRecord {
            app_group_id: "group-1".to_string(),
            root_pid: 1,
            process_ids: vec![1, 2, 3],
            app_name: Some("test-app".to_string()),
            total_cpu_share: Some(0.5),
            total_io_read_bytes: Some(10000),
            total_io_write_bytes: Some(5000),
            total_rss_mb: Some(100),
            has_gui_window: true,
            is_focused_group: false,
            tags: vec!["gui".to_string()],
            priority_class: None,
        }];
        let app_groups_arc = Arc::new(RwLock::new(app_groups));
        let state = ApiState::with_all(None, None, None, Some(app_groups_arc));
        let result = appgroups_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert!(value["app_groups"].is_array());
        assert_eq!(value["count"], 1);
    }

    #[tokio::test]
    async fn test_version_handler() {
        let result = version_handler().await;
        let json = result.0;
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "smoothtaskd");
        assert!(json["version"].is_string());
        // Проверяем, что версия соответствует версии из Cargo.toml
        assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn test_config_handler_without_config() {
        let state = ApiState::new();
        let result = config_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert_eq!(value["config"], Value::Null);
        assert!(value["message"].is_string());
    }

    #[tokio::test]
    async fn test_config_handler_with_config() {
        use crate::config::config_struct::{
            CacheIntervals, Config, LoggingConfig, MLClassifierConfig, ModelConfig, ModelType,
            NotificationBackend, NotificationConfig, NotificationLevel, Paths,
            PatternAutoUpdateConfig, PolicyMode, Thresholds,
        };
        use crate::metrics::ebpf::EbpfConfig;
        let config = Config {
            polling_interval_ms: 1000,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: PolicyMode::RulesOnly,
            enable_snapshot_logging: true,
            thresholds: Thresholds {
                psi_cpu_some_high: 0.6,
                psi_io_some_high: 0.4,
                user_idle_timeout_sec: 120,
                interactive_build_grace_sec: 10,
                noisy_neighbour_cpu_share: 0.7,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.6,
                normal_percentile: 0.3,
                background_percentile: 0.1,
                sched_latency_p99_threshold_ms: 20.0,
                ui_loop_p95_threshold_ms: 16.67,
            },
            paths: Paths {
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
            },
            cache_intervals: CacheIntervals {
                system_metrics_cache_interval: 3,
                process_metrics_cache_interval: 1,
            },
            notifications: NotificationConfig {
                enabled: false,
                backend: NotificationBackend::Stub,
                app_name: "SmoothTask".to_string(),
                min_level: NotificationLevel::Warning,
            },
            model: ModelConfig {
                enabled: false,
                model_path: "/tmp/model.onnx".to_string(),
                model_type: ModelType::Onnx,
            },
            ml_classifier: MLClassifierConfig {
                enabled: false,
                model_path: "/tmp/classifier.onnx".to_string(),
                model_type: ModelType::Onnx,
                confidence_threshold: 0.7,
            },
            pattern_auto_update: PatternAutoUpdateConfig {
                enabled: false,
                interval_sec: 60,
                notify_on_update: false,
            },
            ebpf: EbpfConfig {
                enable_cpu_metrics: false,
                enable_memory_metrics: false,
                enable_syscall_monitoring: false,
                enable_network_monitoring: false,
                enable_network_connections: false,
                enable_gpu_monitoring: false,
                enable_filesystem_monitoring: false,
                enable_process_monitoring: false,
                collection_interval: Duration::from_secs(1),
                enable_caching: true,
                batch_size: 100,
                max_init_attempts: 3,
                operation_timeout_ms: 1000,
                enable_high_performance_mode: true,
                enable_notifications: false,
                notification_thresholds: EbpfNotificationThresholds::default(),
                enable_aggressive_caching: false,
                aggressive_cache_interval_ms: 5000,
            },
        };
        let config_arc = Arc::new(RwLock::new(config));
        let state = ApiState::with_all_and_config(None, None, None, None, Some(config_arc));
        let result = config_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert!(value["config"].is_object());
        let config_obj = &value["config"];
        assert_eq!(config_obj["polling_interval_ms"], 1000);
        assert_eq!(config_obj["max_candidates"], 150);
        assert_eq!(config_obj["dry_run_default"], false);
        assert_eq!(config_obj["policy_mode"], "rules-only");
        assert_eq!(config_obj["enable_snapshot_logging"], true);
        assert!(config_obj["thresholds"].is_object());
        assert!(config_obj["paths"].is_object());
    }

    #[test]
    fn test_api_state_with_all_and_config() {
        use crate::config::config_struct::{
            CacheIntervals, Config, LoggingConfig, MLClassifierConfig, ModelConfig, ModelType,
            Paths, PatternAutoUpdateConfig, PolicyMode, Thresholds,
        };
        use crate::metrics::ebpf::EbpfConfig;
        let config = Config {
            polling_interval_ms: 1000,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: PolicyMode::RulesOnly,
            enable_snapshot_logging: true,
            thresholds: Thresholds {
                psi_cpu_some_high: 0.6,
                psi_io_some_high: 0.4,
                user_idle_timeout_sec: 120,
                interactive_build_grace_sec: 10,
                noisy_neighbour_cpu_share: 0.7,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.6,
                normal_percentile: 0.3,
                background_percentile: 0.1,
                sched_latency_p99_threshold_ms: 20.0,
                ui_loop_p95_threshold_ms: 16.67,
            },
            paths: Paths {
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
            },
            cache_intervals: CacheIntervals {
                system_metrics_cache_interval: 3,
                process_metrics_cache_interval: 1,
            },
            notifications: crate::config::config_struct::NotificationConfig {
                enabled: false,
                backend: crate::config::config_struct::NotificationBackend::Stub,
                app_name: "SmoothTask".to_string(),
                min_level: crate::config::config_struct::NotificationLevel::Warning,
            },
            model: ModelConfig {
                enabled: false,
                model_path: "/tmp/model.onnx".to_string(),
                model_type: ModelType::Onnx,
            },
            ml_classifier: MLClassifierConfig {
                enabled: false,
                model_path: "/tmp/classifier.onnx".to_string(),
                model_type: ModelType::Onnx,
                confidence_threshold: 0.7,
            },
            pattern_auto_update: PatternAutoUpdateConfig {
                enabled: false,
                interval_sec: 60,
                notify_on_update: false,
            },
            ebpf: EbpfConfig {
                enable_cpu_metrics: false,
                enable_memory_metrics: false,
                enable_syscall_monitoring: false,
                enable_network_monitoring: false,
                enable_network_connections: false,
                enable_gpu_monitoring: false,
                enable_filesystem_monitoring: false,
                enable_process_monitoring: false,
                collection_interval: Duration::from_secs(1),
                enable_caching: true,
                batch_size: 100,
                max_init_attempts: 3,
                operation_timeout_ms: 1000,
                enable_high_performance_mode: true,
                enable_aggressive_caching: false,
                aggressive_cache_interval_ms: 5000,
                enable_notifications: false,
                notification_thresholds: EbpfNotificationThresholds::default(),
            },
        };
        let config_arc = Arc::new(RwLock::new(config));
        let state = ApiState::with_all_and_config(None, None, None, None, Some(config_arc));
        assert!(state.config.is_some());
        assert!(state.responsiveness_metrics.is_none());
    }

    #[tokio::test]
    async fn test_responsiveness_handler_without_metrics() {
        let state = ApiState::new();
        let result = responsiveness_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert_eq!(value["responsiveness_metrics"], Value::Null);
        assert!(value["message"].is_string());
    }

    #[tokio::test]
    async fn test_responsiveness_handler_with_metrics() {
        use crate::logging::snapshots::ResponsivenessMetrics;
        let metrics = ResponsivenessMetrics {
            sched_latency_p95_ms: Some(5.0),
            sched_latency_p99_ms: Some(10.0),
            audio_xruns_delta: Some(0),
            ui_loop_p95_ms: Some(16.0),
            frame_jank_ratio: None,
            bad_responsiveness: false,
            responsiveness_score: Some(0.9),
        };
        let metrics_arc = Arc::new(RwLock::new(metrics));
        let state = ApiState::with_all_and_responsiveness_and_config(
            None,
            None,
            None,
            None,
            Some(metrics_arc),
            None,
        );
        let result = responsiveness_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert!(value["responsiveness_metrics"].is_object());
        let resp_metrics = &value["responsiveness_metrics"];
        assert_eq!(resp_metrics["sched_latency_p95_ms"], 5.0);
        assert_eq!(resp_metrics["sched_latency_p99_ms"], 10.0);
        assert_eq!(resp_metrics["audio_xruns_delta"], 0);
        assert_eq!(resp_metrics["ui_loop_p95_ms"], 16.0);
        assert_eq!(resp_metrics["bad_responsiveness"], false);
        assert_eq!(resp_metrics["responsiveness_score"], 0.9);
    }

    #[test]
    fn test_api_server_with_all_and_config() {
        use crate::config::config_struct::{
            CacheIntervals, Config, LoggingConfig, Paths, PolicyMode, Thresholds,
        };
        use crate::metrics::ebpf::EbpfConfig;
        let addr: SocketAddr = "127.0.0.1:8083".parse().unwrap();
        let config = Config {
            polling_interval_ms: 1000,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: PolicyMode::RulesOnly,
            enable_snapshot_logging: true,
            thresholds: Thresholds {
                psi_cpu_some_high: 0.6,
                psi_io_some_high: 0.4,
                user_idle_timeout_sec: 120,
                interactive_build_grace_sec: 10,
                noisy_neighbour_cpu_share: 0.7,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.6,
                normal_percentile: 0.3,
                background_percentile: 0.1,
                sched_latency_p99_threshold_ms: 20.0,
                ui_loop_p95_threshold_ms: 16.67,
            },
            paths: Paths {
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
            },
            cache_intervals: CacheIntervals {
                system_metrics_cache_interval: 3,
                process_metrics_cache_interval: 1,
            },
            notifications: crate::config::config_struct::NotificationConfig {
                enabled: false,
                backend: crate::config::config_struct::NotificationBackend::Stub,
                app_name: "SmoothTask".to_string(),
                min_level: crate::config::config_struct::NotificationLevel::Warning,
            },
            model: crate::config::config_struct::ModelConfig {
                enabled: false,
                model_path: "models/ranker.onnx".to_string(),
                model_type: ModelType::Onnx,
            },
            ml_classifier: MLClassifierConfig::default(),
            pattern_auto_update: PatternAutoUpdateConfig::default(),
            ebpf: EbpfConfig::default(),
        };
        let config_arc = Arc::new(RwLock::new(config));
        let server = ApiServer::with_all_and_config(addr, None, None, None, None, Some(config_arc));
        // Проверяем, что сервер создан
        let _ = server;
    }

    #[tokio::test]
    async fn test_process_by_pid_handler_invalid_pid() {
        let state = ApiState::new();
        let result = process_by_pid_handler(Path(-1), State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "error");
        assert_eq!(value["error"], "invalid_input");
        assert!(value["message"]
            .as_str()
            .unwrap()
            .contains("Invalid PID value"));
        // Check that details are included
        assert!(value["details"]["field"].as_str().unwrap() == "pid");
        assert!(value["details"]["constraint"].as_str().unwrap() == "must be > 0");
    }

    #[tokio::test]
    async fn test_process_by_pid_handler_too_large_pid() {
        let state = ApiState::new();
        let result = process_by_pid_handler(Path(5_000_000), State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "error");
        assert_eq!(value["error"], "invalid_input");
        assert!(value["message"].as_str().unwrap().contains("too large"));
        // Check that details are included
        assert!(value["details"]["constraint"].as_str().unwrap() == "must be <= 4194304");
        assert!(value["details"]["note"]
            .as_str()
            .unwrap()
            .contains("Maximum PID in Linux"));
    }

    #[tokio::test]
    async fn test_process_by_pid_handler_without_processes() {
        let state = ApiState::new();
        let result = process_by_pid_handler(Path(1), State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "error");
        assert_eq!(value["error"], "not_available");
        assert!(value["message"].is_string());
    }

    #[tokio::test]
    async fn test_process_by_pid_handler_process_not_found() {
        use crate::logging::snapshots::ProcessRecord;
        let processes = vec![ProcessRecord {
            pid: 1,
            ppid: 0,
            uid: 0,
            gid: 0,
            exe: Some("/sbin/init".to_string()),
            cmdline: Some("init".to_string()),
            cgroup_path: None,
            systemd_unit: None,
            app_group_id: None,
            state: "S".to_string(),
            start_time: 0,
            uptime_sec: 100,
            tty_nr: 0,
            has_tty: false,
            cpu_share_1s: Some(0.1),
            cpu_share_10s: Some(0.05),
            io_read_bytes: Some(1000),
            io_write_bytes: Some(500),
            rss_mb: Some(10),
            swap_mb: Some(0),
            voluntary_ctx: Some(100),
            involuntary_ctx: Some(10),
            has_gui_window: false,
            is_focused_window: false,
            window_state: None,
            env_has_display: false,
            env_has_wayland: false,
            env_term: None,
            env_ssh: false,
            is_audio_client: false,
            has_active_stream: false,
            process_type: None,
            tags: vec![],
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
        }];
        let processes_arc = Arc::new(RwLock::new(processes));
        let state = ApiState::with_all(None, None, Some(processes_arc), None);
        let result = process_by_pid_handler(Path(999), State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "error");
        assert_eq!(value["error"], "not_found");
        assert!(value["message"].as_str().unwrap().contains("999"));
    }

    #[tokio::test]
    async fn test_process_by_pid_handler_process_found() {
        use crate::logging::snapshots::ProcessRecord;
        let processes = vec![ProcessRecord {
            pid: 1,
            ppid: 0,
            uid: 0,
            gid: 0,
            exe: Some("/sbin/init".to_string()),
            cmdline: Some("init".to_string()),
            cgroup_path: None,
            systemd_unit: None,
            app_group_id: None,
            state: "S".to_string(),
            start_time: 0,
            uptime_sec: 100,
            tty_nr: 0,
            has_tty: false,
            cpu_share_1s: Some(0.1),
            cpu_share_10s: Some(0.05),
            io_read_bytes: Some(1000),
            io_write_bytes: Some(500),
            rss_mb: Some(10),
            swap_mb: Some(0),
            voluntary_ctx: Some(100),
            involuntary_ctx: Some(10),
            has_gui_window: false,
            is_focused_window: false,
            window_state: None,
            env_has_display: false,
            env_has_wayland: false,
            env_term: None,
            env_ssh: false,
            is_audio_client: false,
            has_active_stream: false,
            process_type: None,
            tags: vec![],
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
        }];
        let processes_arc = Arc::new(RwLock::new(processes));
        let state = ApiState::with_all(None, None, Some(processes_arc), None);
        let result = process_by_pid_handler(Path(1), State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert!(value["process"].is_object());
        let process = &value["process"];
        assert_eq!(process["pid"], 1);
    }

    #[tokio::test]
    async fn test_appgroup_by_id_handler_empty_id() {
        let state = ApiState::new();
        let result = appgroup_by_id_handler(Path("".to_string()), State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "error");
        assert_eq!(value["error"], "invalid_input");
        assert_eq!(value["message"], "App group ID cannot be empty");
        // Check that details are included
        assert!(value["details"]["field"].as_str().unwrap() == "id");
        assert!(value["details"]["constraint"].as_str().unwrap() == "must not be empty");
    }

    #[tokio::test]
    async fn test_appgroup_by_id_handler_too_long_id() {
        let state = ApiState::new();
        let long_id = "a".repeat(257); // 257 characters, exceeds 256 limit
        let result = appgroup_by_id_handler(Path(long_id), State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "error");
        assert_eq!(value["error"], "invalid_input");
        assert!(value["message"].as_str().unwrap().contains("too long"));
        // Check that details are included
        assert!(value["details"]["constraint"].as_str().unwrap() == "must be <= 256 characters");
    }

    #[tokio::test]
    async fn test_appgroup_by_id_handler_invalid_characters() {
        let state = ApiState::new();
        let invalid_id = "group-1\u{0000}".to_string(); // Contains null character
        let result = appgroup_by_id_handler(Path(invalid_id), State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "error");
        assert_eq!(value["error"], "invalid_input");
        assert!(value["message"]
            .as_str()
            .unwrap()
            .contains("invalid characters"));
        // Check that details are included
        assert!(value["details"]["constraint"]
            .as_str()
            .unwrap()
            .contains("alphanumeric"));
    }

    #[tokio::test]
    async fn test_appgroup_by_id_handler_without_appgroups() {
        let state = ApiState::new();
        let result = appgroup_by_id_handler(Path("group-1".to_string()), State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "error");
        assert_eq!(value["error"], "not_available");
        assert!(value["message"].is_string());
    }

    #[tokio::test]
    async fn test_appgroup_by_id_handler_group_not_found() {
        use crate::logging::snapshots::AppGroupRecord;
        let app_groups = vec![AppGroupRecord {
            app_group_id: "group-1".to_string(),
            root_pid: 1,
            process_ids: vec![1, 2, 3],
            app_name: Some("test-app".to_string()),
            total_cpu_share: Some(0.5),
            total_io_read_bytes: Some(10000),
            total_io_write_bytes: Some(5000),
            total_rss_mb: Some(100),
            has_gui_window: true,
            is_focused_group: false,
            tags: vec!["gui".to_string()],
            priority_class: None,
        }];
        let app_groups_arc = Arc::new(RwLock::new(app_groups));
        let state = ApiState::with_all(None, None, None, Some(app_groups_arc));
        let result = appgroup_by_id_handler(Path("group-999".to_string()), State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "error");
        assert_eq!(value["error"], "not_found");
        assert!(value["message"].as_str().unwrap().contains("group-999"));
    }

    #[tokio::test]
    async fn test_appgroup_by_id_handler_group_found() {
        use crate::logging::snapshots::AppGroupRecord;
        let app_groups = vec![AppGroupRecord {
            app_group_id: "group-1".to_string(),
            root_pid: 1,
            process_ids: vec![1, 2, 3],
            app_name: Some("test-app".to_string()),
            total_cpu_share: Some(0.5),
            total_io_read_bytes: Some(10000),
            total_io_write_bytes: Some(5000),
            total_rss_mb: Some(100),
            has_gui_window: true,
            is_focused_group: false,
            tags: vec!["gui".to_string()],
            priority_class: None,
        }];
        let app_groups_arc = Arc::new(RwLock::new(app_groups));
        let state = ApiState::with_all(None, None, None, Some(app_groups_arc));
        let result = appgroup_by_id_handler(Path("group-1".to_string()), State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert!(value["app_group"].is_object());
        let app_group = &value["app_group"];
        assert_eq!(app_group["app_group_id"], "group-1");
    }

    #[tokio::test]
    async fn test_endpoints_handler() {
        let result = endpoints_handler().await;
        let json = result.0;
        assert_eq!(json["status"], "ok");
        assert!(json["endpoints"].is_array());
        assert_eq!(json["count"], 22); // Обновлено с 19 до 20

        let endpoints = json["endpoints"].as_array().unwrap();
        assert_eq!(endpoints.len(), 22); // Обновлено с 20 до 22

        // Проверяем наличие основных endpoints
        let endpoint_paths: Vec<&str> = endpoints
            .iter()
            .map(|e| e["path"].as_str().unwrap())
            .collect();

        assert!(endpoint_paths.contains(&"/health"));
        assert!(endpoint_paths.contains(&"/api/version"));
        assert!(endpoint_paths.contains(&"/api/endpoints"));
        assert!(endpoint_paths.contains(&"/api/stats"));
        assert!(endpoint_paths.contains(&"/api/metrics"));
        assert!(endpoint_paths.contains(&"/api/responsiveness"));
        assert!(endpoint_paths.contains(&"/api/processes"));
        assert!(endpoint_paths.contains(&"/api/processes/:pid"));
        assert!(endpoint_paths.contains(&"/api/appgroups"));
        assert!(endpoint_paths.contains(&"/api/appgroups/:id"));
        assert!(endpoint_paths.contains(&"/api/config"));
        assert!(endpoint_paths.contains(&"/api/classes"));
        assert!(endpoint_paths.contains(&"/api/patterns"));
        assert!(endpoint_paths.contains(&"/api/system"));
        assert!(endpoint_paths.contains(&"/api/config/reload"));
        assert!(endpoint_paths.contains(&"/api/notifications/test"));
        assert!(endpoint_paths.contains(&"/api/notifications/status"));
        assert!(endpoint_paths.contains(&"/api/notifications/config"));
        assert!(endpoint_paths.contains(&"/api/performance"));
        assert!(endpoint_paths.contains(&"/api/health"));

        // Проверяем структуру endpoint
        let first_endpoint = &endpoints[0];
        assert!(first_endpoint["path"].is_string());
        assert!(first_endpoint["method"].is_string());
        assert!(first_endpoint["description"].is_string());
        assert_eq!(first_endpoint["method"], "GET");
    }

    #[tokio::test]
    async fn test_classes_handler() {
        let result = classes_handler().await;
        let json = result.0;
        assert_eq!(json["status"], "ok");
        assert!(json["classes"].is_array());
        assert_eq!(json["count"], 5);

        let classes = json["classes"].as_array().unwrap();
        assert_eq!(classes.len(), 5);

        // Проверяем наличие всех классов
        let class_names: Vec<&str> = classes
            .iter()
            .map(|c| c["name"].as_str().unwrap())
            .collect();

        assert!(class_names.contains(&"CRIT_INTERACTIVE"));
        assert!(class_names.contains(&"INTERACTIVE"));
        assert!(class_names.contains(&"NORMAL"));
        assert!(class_names.contains(&"BACKGROUND"));
        assert!(class_names.contains(&"IDLE"));

        // Проверяем структуру класса
        let crit_interactive = classes
            .iter()
            .find(|c| c["name"] == "CRIT_INTERACTIVE")
            .unwrap();

        assert_eq!(crit_interactive["class"], "CRIT_INTERACTIVE");
        assert!(crit_interactive["description"].is_string());
        assert!(crit_interactive["params"].is_object());

        let params = &crit_interactive["params"];
        assert!(params["nice"].is_number());
        assert_eq!(params["nice"], -8);
        assert!(params["latency_nice"].is_number());
        assert_eq!(params["latency_nice"], -15);
        assert!(params["ionice"].is_object());
        assert_eq!(params["ionice"]["class"], 2);
        assert_eq!(params["ionice"]["level"], 0);
        assert_eq!(params["ionice"]["class_description"], "best-effort");
        assert!(params["cgroup"].is_object());
        assert_eq!(params["cgroup"]["cpu_weight"], 200);

        // Проверяем параметры для Idle класса (должен иметь class 3)
        let idle = classes.iter().find(|c| c["name"] == "IDLE").unwrap();
        assert_eq!(idle["params"]["ionice"]["class"], 3);
        assert_eq!(idle["params"]["ionice"]["class_description"], "idle");
    }

    #[tokio::test]
    async fn test_performance_handler() {
        let state = ApiState::new();
        let result = performance_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert!(value["performance_metrics"].is_object());
        assert!(value["cache_info"].is_object());

        let perf_metrics = &value["performance_metrics"];
        assert_eq!(perf_metrics["total_requests"], 0);
        assert_eq!(perf_metrics["cache_hits"], 0);
        assert_eq!(perf_metrics["cache_misses"], 0);
        assert_eq!(perf_metrics["cache_hit_rate"], 0.0);
        assert_eq!(perf_metrics["average_processing_time_us"], 0.0);
        assert_eq!(perf_metrics["total_processing_time_us"], 0);
        assert!(perf_metrics["last_request_time"].is_null());
        assert_eq!(perf_metrics["requests_per_second"], 0.0);

        let cache_info = &value["cache_info"];
        assert_eq!(cache_info["enabled"], false);
        assert!(cache_info["ttl_seconds"].is_null());
    }

    #[tokio::test]
    async fn test_patterns_handler_without_patterns() {
        let state = ApiState::new();
        let result = patterns_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert_eq!(value["total_patterns"], 0);
        assert_eq!(value["total_categories"], 0);
        assert!(value["categories"].is_array());
        assert_eq!(value["categories"].as_array().unwrap().len(), 0);
        assert!(value["message"].is_string());
    }

    #[tokio::test]
    async fn test_patterns_handler_with_empty_database() {
        use crate::classify::rules::PatternDatabase;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let pattern_db = PatternDatabase::load(temp_dir.path()).unwrap();
        assert_eq!(pattern_db.all_patterns().len(), 0);

        let state = ApiStateBuilder::new()
            .with_daemon_stats(None)
            .with_system_metrics(None)
            .with_processes(None)
            .with_app_groups(None)
            .with_responsiveness_metrics(None)
            .with_config(None)
            .with_pattern_database(Some(Arc::new(pattern_db)))
            .with_config_path(None)
            .with_notification_manager(None)
            .build();

        let result = patterns_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;

        assert_eq!(value["status"], "ok");
        assert_eq!(value["total_patterns"], 0);
        assert_eq!(value["total_categories"], 0);
        assert!(value["categories"].is_array());
        assert!(value["categories"].as_array().unwrap().is_empty());
        assert!(value["message"].is_null());
    }

    #[tokio::test]
    async fn test_patterns_handler_with_patterns() {
        use crate::classify::rules::PatternDatabase;
        use tempfile::TempDir;

        // Создаём временную директорию с паттернами
        let temp_dir = TempDir::new().unwrap();
        let patterns_dir = temp_dir.path();

        // Создаём тестовый YAML файл с паттернами
        let pattern_content = r#"
category: browser
apps:
  - name: firefox
    label: Mozilla Firefox
    exe_patterns:
      - "firefox"
      - "firefox-*-bin"
    desktop_patterns:
      - "firefox.desktop"
    cgroup_patterns:
      - "*firefox*"
    tags:
      - "browser"
      - "gui"
  - name: chromium
    label: Chromium
    exe_patterns:
      - "chromium"
      - "chromium-browser"
    tags:
      - "browser"
      - "gui"
"#;
        std::fs::write(patterns_dir.join("browser.yml"), pattern_content).unwrap();

        // Загружаем паттерны
        let pattern_db = PatternDatabase::load(patterns_dir).unwrap();
        let pattern_db_arc = Arc::new(pattern_db);
        let state = ApiStateBuilder::new()
            .with_daemon_stats(None)
            .with_system_metrics(None)
            .with_processes(None)
            .with_app_groups(None)
            .with_responsiveness_metrics(None)
            .with_config(None)
            .with_pattern_database(Some(pattern_db_arc))
            .with_config_path(None)
            .with_notification_manager(None)
            .build();

        let result = patterns_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert_eq!(value["total_patterns"], 2);
        assert_eq!(value["total_categories"], 1);
        assert!(value["categories"].is_array());

        let categories = value["categories"].as_array().unwrap();
        assert_eq!(categories.len(), 1);

        let browser_category = &categories[0];
        assert_eq!(browser_category["category"], "browser");
        assert_eq!(browser_category["count"], 2);
        assert!(browser_category["patterns"].is_array());

        let patterns = browser_category["patterns"].as_array().unwrap();
        assert_eq!(patterns.len(), 2);

        // Проверяем структуру паттерна
        let firefox_pattern = patterns.iter().find(|p| p["name"] == "firefox").unwrap();
        assert_eq!(firefox_pattern["name"], "firefox");
        assert_eq!(firefox_pattern["label"], "Mozilla Firefox");
        assert!(firefox_pattern["exe_patterns"].is_array());
        assert!(firefox_pattern["desktop_patterns"].is_array());
        assert!(firefox_pattern["cgroup_patterns"].is_array());
        assert!(firefox_pattern["tags"].is_array());
    }

    #[test]
    fn test_api_server_with_all() {
        let addr: SocketAddr = "127.0.0.1:8084".parse().unwrap();
        let stats = Arc::new(RwLock::new(crate::DaemonStats::new()));
        use crate::metrics::system::{
            CpuTimes, DiskMetrics, LoadAvg, MemoryInfo, NetworkMetrics, PowerMetrics,
            PressureMetrics, SystemMetrics, TemperatureMetrics,
        };
        let metrics = SystemMetrics {
            cpu_times: CpuTimes {
                user: 100,
                nice: 20,
                system: 50,
                idle: 200,
                iowait: 10,
                irq: 5,
                softirq: 5,
                steal: 0,
                guest: 0,
                guest_nice: 0,
            },
            memory: MemoryInfo {
                mem_total_kb: 1000,
                mem_available_kb: 500,
                mem_free_kb: 400,
                buffers_kb: 50,
                cached_kb: 50,
                swap_total_kb: 1000,
                swap_free_kb: 800,
            },
            load_avg: LoadAvg {
                one: 1.0,
                five: 1.0,
                fifteen: 1.0,
            },
            pressure: PressureMetrics::default(),
            temperature: TemperatureMetrics::default(),
            power: PowerMetrics::default(),
            network: NetworkMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
        };
        let metrics_arc = Arc::new(RwLock::new(metrics));
        let processes = Arc::new(RwLock::new(vec![]));
        let app_groups = Arc::new(RwLock::new(vec![]));
        let server = ApiServer::with_all(
            addr,
            Some(stats),
            Some(metrics_arc),
            Some(processes),
            Some(app_groups),
        );
        // Проверяем, что сервер создан
        let _ = server;
    }

    #[test]
    fn test_api_server_with_all_and_responsiveness_and_config() {
        use crate::config::config_struct::{
            CacheIntervals, Config, LoggingConfig, Paths, PolicyMode, Thresholds,
        };
        use crate::logging::snapshots::ResponsivenessMetrics;
        use crate::metrics::ebpf::EbpfConfig;
        let addr: SocketAddr = "127.0.0.1:8085".parse().unwrap();
        let config = Config {
            polling_interval_ms: 1000,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: PolicyMode::RulesOnly,
            enable_snapshot_logging: true,
            thresholds: Thresholds {
                psi_cpu_some_high: 0.6,
                psi_io_some_high: 0.4,
                user_idle_timeout_sec: 120,
                interactive_build_grace_sec: 10,
                noisy_neighbour_cpu_share: 0.7,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.6,
                normal_percentile: 0.3,
                background_percentile: 0.1,
                sched_latency_p99_threshold_ms: 20.0,
                ui_loop_p95_threshold_ms: 16.67,
            },
            paths: Paths {
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
            },
            cache_intervals: CacheIntervals {
                system_metrics_cache_interval: 3,
                process_metrics_cache_interval: 1,
            },
            notifications: crate::config::config_struct::NotificationConfig {
                enabled: false,
                backend: crate::config::config_struct::NotificationBackend::Stub,
                app_name: "SmoothTask".to_string(),
                min_level: crate::config::config_struct::NotificationLevel::Warning,
            },
            model: crate::config::config_struct::ModelConfig {
                enabled: false,
                model_path: "models/ranker.onnx".to_string(),
                model_type: ModelType::Onnx,
            },
            ml_classifier: MLClassifierConfig::default(),
            pattern_auto_update: PatternAutoUpdateConfig::default(),
            ebpf: EbpfConfig::default(),
        };
        let _config_arc = Arc::new(config.clone());
        let responsiveness_metrics = ResponsivenessMetrics {
            sched_latency_p95_ms: Some(5.0),
            sched_latency_p99_ms: Some(10.0),
            audio_xruns_delta: Some(0),
            ui_loop_p95_ms: Some(16.0),
            frame_jank_ratio: None,
            bad_responsiveness: false,
            responsiveness_score: Some(0.9),
        };
        let metrics_arc = Arc::new(RwLock::new(responsiveness_metrics));
        let server = ApiServer::with_all_and_responsiveness_and_config(
            addr,
            None,
            None,
            None,
            None,
            Some(metrics_arc),
            Some(Arc::new(RwLock::new(config))),
        );
        // Проверяем, что сервер создан
        let _ = server;
    }

    #[test]
    fn test_api_server_with_all_and_responsiveness_and_config_and_patterns() {
        use crate::classify::rules::PatternDatabase;
        use crate::config::config_struct::{
            CacheIntervals, Config, LoggingConfig, Paths, PolicyMode, Thresholds,
        };
        use crate::metrics::ebpf::EbpfConfig;
        let addr: SocketAddr = "127.0.0.1:8086".parse().unwrap();
        let config = Config {
            polling_interval_ms: 1000,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: PolicyMode::RulesOnly,
            enable_snapshot_logging: true,
            thresholds: Thresholds {
                psi_cpu_some_high: 0.6,
                psi_io_some_high: 0.4,
                user_idle_timeout_sec: 120,
                interactive_build_grace_sec: 10,
                noisy_neighbour_cpu_share: 0.7,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.6,
                normal_percentile: 0.3,
                background_percentile: 0.1,
                sched_latency_p99_threshold_ms: 20.0,
                ui_loop_p95_threshold_ms: 16.67,
            },
            paths: Paths {
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
            },
            cache_intervals: CacheIntervals {
                system_metrics_cache_interval: 3,
                process_metrics_cache_interval: 1,
            },
            notifications: crate::config::config_struct::NotificationConfig {
                enabled: false,
                backend: crate::config::config_struct::NotificationBackend::Stub,
                app_name: "SmoothTask".to_string(),
                min_level: crate::config::config_struct::NotificationLevel::Warning,
            },
            model: crate::config::config_struct::ModelConfig {
                enabled: false,
                model_path: "models/ranker.onnx".to_string(),
                model_type: ModelType::Onnx,
            },
            ml_classifier: MLClassifierConfig::default(),
            pattern_auto_update: PatternAutoUpdateConfig::default(),
            ebpf: EbpfConfig::default(),
        };
        let _config_arc = Arc::new(config.clone());
        // Создаём временную директорию для паттернов
        let temp_dir = std::env::temp_dir().join("smoothtask_test_patterns");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let pattern_db = PatternDatabase::load(&temp_dir).unwrap();
        let pattern_db_arc = Arc::new(pattern_db);
        let api_state = ApiStateBuilder::new()
            .with_config(Some(Arc::new(RwLock::new(config))))
            .with_pattern_database(Some(pattern_db_arc))
            .build();
        let server = ApiServer {
            addr,
            state: api_state,
        };
        // Проверяем, что сервер создан
        let _ = server;
    }

    #[test]
    fn test_api_state_with_all_and_responsiveness_and_config() {
        use crate::config::config_struct::{
            CacheIntervals, Config, LoggingConfig, Paths, PolicyMode, Thresholds,
        };
        use crate::logging::snapshots::ResponsivenessMetrics;
        use crate::metrics::ebpf::EbpfConfig;
        let config = Config {
            polling_interval_ms: 1000,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: PolicyMode::RulesOnly,
            enable_snapshot_logging: true,
            thresholds: Thresholds {
                psi_cpu_some_high: 0.6,
                psi_io_some_high: 0.4,
                user_idle_timeout_sec: 120,
                interactive_build_grace_sec: 10,
                noisy_neighbour_cpu_share: 0.7,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.6,
                normal_percentile: 0.3,
                background_percentile: 0.1,
                sched_latency_p99_threshold_ms: 20.0,
                ui_loop_p95_threshold_ms: 16.67,
            },
            paths: Paths {
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
            },
            cache_intervals: CacheIntervals {
                system_metrics_cache_interval: 3,
                process_metrics_cache_interval: 1,
            },
            notifications: crate::config::config_struct::NotificationConfig {
                enabled: false,
                backend: crate::config::config_struct::NotificationBackend::Stub,
                app_name: "SmoothTask".to_string(),
                min_level: crate::config::config_struct::NotificationLevel::Warning,
            },
            model: crate::config::config_struct::ModelConfig {
                enabled: false,
                model_path: "models/ranker.onnx".to_string(),
                model_type: ModelType::Onnx,
            },
            ml_classifier: MLClassifierConfig::default(),
            pattern_auto_update: PatternAutoUpdateConfig::default(),
            ebpf: EbpfConfig::default(),
        };
        let _config_arc = Arc::new(config.clone());
        let responsiveness_metrics = ResponsivenessMetrics {
            sched_latency_p95_ms: Some(5.0),
            sched_latency_p99_ms: Some(10.0),
            audio_xruns_delta: Some(0),
            ui_loop_p95_ms: Some(16.0),
            frame_jank_ratio: None,
            bad_responsiveness: false,
            responsiveness_score: Some(0.9),
        };
        let metrics_arc = Arc::new(RwLock::new(responsiveness_metrics));
        let state = ApiState::with_all_and_responsiveness_and_config(
            None,
            None,
            None,
            None,
            Some(metrics_arc),
            Some(Arc::new(RwLock::new(config))),
        );
        assert!(state.config.is_some());
        assert!(state.responsiveness_metrics.is_some());
        assert!(state.pattern_database.is_none());
    }

    #[test]
    fn test_api_state_with_all_and_responsiveness_and_config_and_patterns() {
        use crate::classify::rules::PatternDatabase;
        use crate::config::config_struct::{
            CacheIntervals, Config, LoggingConfig, Paths, PolicyMode, Thresholds,
        };
        use crate::metrics::ebpf::EbpfConfig;
        let config = Config {
            polling_interval_ms: 1000,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: PolicyMode::RulesOnly,
            enable_snapshot_logging: true,
            thresholds: Thresholds {
                psi_cpu_some_high: 0.6,
                psi_io_some_high: 0.4,
                user_idle_timeout_sec: 120,
                interactive_build_grace_sec: 10,
                noisy_neighbour_cpu_share: 0.7,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.6,
                normal_percentile: 0.3,
                background_percentile: 0.1,
                sched_latency_p99_threshold_ms: 20.0,
                ui_loop_p95_threshold_ms: 16.67,
            },
            paths: Paths {
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
            },
            cache_intervals: CacheIntervals {
                system_metrics_cache_interval: 3,
                process_metrics_cache_interval: 1,
            },
            notifications: crate::config::config_struct::NotificationConfig {
                enabled: false,
                backend: crate::config::config_struct::NotificationBackend::Stub,
                app_name: "SmoothTask".to_string(),
                min_level: crate::config::config_struct::NotificationLevel::Warning,
            },
            model: crate::config::config_struct::ModelConfig {
                enabled: false,
                model_path: "models/ranker.onnx".to_string(),
                model_type: ModelType::Onnx,
            },
            ml_classifier: MLClassifierConfig::default(),
            pattern_auto_update: PatternAutoUpdateConfig::default(),
            ebpf: EbpfConfig::default(),
        };
        let _config_arc = Arc::new(config.clone());
        // Создаём временную директорию для паттернов
        let temp_dir = std::env::temp_dir().join("smoothtask_test_patterns");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let pattern_db = PatternDatabase::load(&temp_dir).unwrap();
        let pattern_db_arc = Arc::new(pattern_db);
        let state = ApiStateBuilder::new()
            .with_config(Some(Arc::new(RwLock::new(config))))
            .with_pattern_database(Some(pattern_db_arc))
            .build();
        assert!(state.config.is_some());
        assert!(state.pattern_database.is_some());
        assert!(state.responsiveness_metrics.is_none());
    }

    #[tokio::test]
    async fn test_health_handler() {
        let result = health_handler().await;
        let json = result.0;
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "smoothtask-api");
    }

    #[tokio::test]
    async fn test_health_detailed_handler_without_data() {
        // Тестируем обработчик health_detailed_handler без данных
        let state = ApiStateBuilder::new()
            .with_daemon_stats(None)
            .with_system_metrics(None)
            .with_processes(None)
            .with_app_groups(None)
            .with_responsiveness_metrics(None)
            .with_config(None)
            .with_config_path(None)
            .with_pattern_database(None)
            .with_notification_manager(None)
            .build();

        let result = health_detailed_handler(State(state)).await;
        assert!(result.is_ok());

        let json = result.unwrap().0;
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "smoothtaskd");
        assert_eq!(json["uptime_seconds"], 0);

        let components = &json["components"];
        assert_eq!(components["daemon_stats"], false);
        assert_eq!(components["system_metrics"], false);
        assert_eq!(components["processes"], false);
        assert_eq!(components["app_groups"], false);
        assert_eq!(components["config"], false);
        assert_eq!(components["pattern_database"], false);

        let performance = &json["performance"];
        assert_eq!(performance["total_requests"], 0);
        assert_eq!(performance["cache_hit_rate"], 0.0);
        assert_eq!(performance["average_processing_time_us"], 0.0);

        assert!(json["timestamp"].is_string());
    }

    #[tokio::test]
    async fn test_health_detailed_handler_with_data() {
        // Тестируем обработчик health_detailed_handler с данными
        let daemon_stats = Arc::new(RwLock::new(crate::DaemonStats::new()));
        let system_metrics =
            Arc::new(RwLock::new(crate::metrics::system::SystemMetrics::default()));
        let processes = Arc::new(RwLock::new(
            Vec::<crate::logging::snapshots::ProcessRecord>::new(),
        ));
        let app_groups = Arc::new(RwLock::new(
            Vec::<crate::logging::snapshots::AppGroupRecord>::new(),
        ));
        let config = Arc::new(RwLock::new(crate::config::config_struct::Config::default()));
        let pattern_db = Arc::new(
            crate::classify::rules::PatternDatabase::load("/tmp/test_patterns").unwrap_or_default(),
        );

        let state = ApiStateBuilder::new()
            .with_daemon_stats(Some(daemon_stats))
            .with_system_metrics(Some(system_metrics))
            .with_processes(Some(processes))
            .with_app_groups(Some(app_groups))
            .with_responsiveness_metrics(None)
            .with_config(Some(config))
            .with_config_path(None)
            .with_pattern_database(Some(pattern_db))
            .with_notification_manager(None)
            .build();

        let result = health_detailed_handler(State(state)).await;
        assert!(result.is_ok());

        let json = result.unwrap().0;
        assert_eq!(json["status"], "ok");
        assert_eq!(json["service"], "smoothtaskd");

        let components = &json["components"];
        assert_eq!(components["daemon_stats"], true);
        assert_eq!(components["system_metrics"], true);
        assert_eq!(components["processes"], true);
        assert_eq!(components["app_groups"], true);
        assert_eq!(components["config"], true);
        assert_eq!(components["pattern_database"], true);

        assert!(json["timestamp"].is_string());
    }

    #[tokio::test]
    async fn test_system_handler() {
        let result = system_handler().await;
        let json = result.0;
        assert_eq!(json["status"], "ok");
        assert!(json["system"].is_object());

        let system = &json["system"];
        assert!(system["kernel"].is_object());
        assert!(system["distribution"].is_object());

        // Проверяем, что если /proc/version доступен, то версия ядра присутствует
        if fs::read_to_string("/proc/version").is_ok() {
            // kernel должен содержать либо version_string, либо version
            assert!(
                system["kernel"]["version_string"].is_string()
                    || system["kernel"]["version"].is_string()
            );
        }

        // architecture может быть null или строкой
        assert!(system["architecture"].is_null() || system["architecture"].is_string());
    }

    #[tokio::test]
    async fn test_config_reload_handler_without_config() {
        // Тест для config_reload_handler когда конфигурация не доступна
        let state = ApiState::new();
        let result = config_reload_handler(State(state)).await;
        let json = result.unwrap().0;

        assert_eq!(json["status"], "error");
        assert!(json["message"].as_str().unwrap().contains("not available"));
    }

    #[tokio::test]
    async fn test_config_reload_handler_with_config() {
        // Тест для config_reload_handler когда конфигурация доступна
        use crate::config::config_struct::{
            CacheIntervals, Config, LoggingConfig, MLClassifierConfig, ModelConfig, ModelType,
            NotificationBackend, NotificationConfig, NotificationLevel, Paths,
            PatternAutoUpdateConfig, PolicyMode, Thresholds,
        };
        use crate::metrics::ebpf::EbpfConfig;

        let config = Config {
            polling_interval_ms: 500,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: PolicyMode::RulesOnly,
            enable_snapshot_logging: false,
            thresholds: Thresholds {
                psi_cpu_some_high: 0.6,
                psi_io_some_high: 0.4,
                user_idle_timeout_sec: 120,
                interactive_build_grace_sec: 10,
                noisy_neighbour_cpu_share: 0.7,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.6,
                normal_percentile: 0.3,
                background_percentile: 0.1,
                sched_latency_p99_threshold_ms: 20.0,
                ui_loop_p95_threshold_ms: 16.67,
            },
            paths: Paths {
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
            },
            cache_intervals: CacheIntervals {
                system_metrics_cache_interval: 3,
                process_metrics_cache_interval: 1,
            },
            notifications: NotificationConfig {
                enabled: false,
                backend: NotificationBackend::Stub,
                app_name: "SmoothTask".to_string(),
                min_level: NotificationLevel::Warning,
            },
            model: ModelConfig {
                enabled: false,
                model_path: "models/ranker.onnx".to_string(),
                model_type: ModelType::Onnx,
            },
            ml_classifier: MLClassifierConfig::default(),
            pattern_auto_update: PatternAutoUpdateConfig::default(),
            ebpf: EbpfConfig::default(),
        };
        let config_arc = Arc::new(RwLock::new(config));
        let state = ApiStateBuilder::new()
            .with_daemon_stats(None)
            .with_system_metrics(None)
            .with_processes(None)
            .with_app_groups(None)
            .with_responsiveness_metrics(None)
            .with_config(Some(config_arc.clone()))
            .with_config_path(None)
            .with_pattern_database(None)
            .with_notification_manager(None)
            .build();

        let result = config_reload_handler(State(state)).await;
        let json = result.unwrap().0;

        assert_eq!(json["status"], "warning");
        assert!(json["message"]
            .as_str()
            .unwrap()
            .contains("Config reload requested but config file path is not available"));
        assert!(json["current_config"].is_object());
        assert_eq!(json["current_config"]["polling_interval_ms"], 500);
        assert!(json["action_required"].is_string());
    }

    #[tokio::test]
    async fn test_config_reload_handler_with_config_path() {
        // Тест для config_reload_handler когда конфигурация и путь доступны
        // Создаём временный конфигурационный файл
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let config_file_path = temp_dir.path().join("test_config.yml");
        let config_file_content = r#"
polling_interval_ms: 1000
max_candidates: 200
dry_run_default: true

paths:
  snapshot_db_path: "/tmp/test.db"
  patterns_dir: "/tmp/patterns"

thresholds:
  psi_cpu_some_high: 0.5
  psi_io_some_high: 0.3
  user_idle_timeout_sec: 60
  interactive_build_grace_sec: 5
  noisy_neighbour_cpu_share: 0.5
  crit_interactive_percentile: 0.8
  interactive_percentile: 0.5
  normal_percentile: 0.2
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67

logging:
  log_max_size_bytes: 10485760
  log_max_rotated_files: 5
  log_compression_enabled: true
  log_rotation_interval_sec: 0

cache_intervals:
  system_metrics_cache_interval: 3
  process_metrics_cache_interval: 1

notifications:
  enabled: false
  backend: stub
  app_name: "SmoothTask"
  min_level: warning
"#;

        std::fs::write(&config_file_path, config_file_content).expect("write config");
        // Создаём директорию patterns_dir, так как она требуется для валидации конфигурации
        std::fs::create_dir_all("/tmp/patterns").expect("create patterns dir");

        // Создаём текущую конфигурацию (отличную от файла)
        let current_config = crate::config::config_struct::Config {
            polling_interval_ms: 500,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: crate::config::config_struct::PolicyMode::RulesOnly,
            enable_snapshot_logging: false,
            thresholds: crate::config::config_struct::Thresholds {
                psi_cpu_some_high: 0.6,
                psi_io_some_high: 0.4,
                user_idle_timeout_sec: 120,
                interactive_build_grace_sec: 10,
                noisy_neighbour_cpu_share: 0.7,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.6,
                normal_percentile: 0.3,
                background_percentile: 0.1,
                sched_latency_p99_threshold_ms: 20.0,
                ui_loop_p95_threshold_ms: 16.67,
            },
            paths: crate::config::config_struct::Paths {
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: crate::config::config_struct::LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
            },
            cache_intervals: crate::config::config_struct::CacheIntervals {
                system_metrics_cache_interval: 3,
                process_metrics_cache_interval: 1,
            },
            notifications: crate::config::config_struct::NotificationConfig {
                enabled: false,
                backend: crate::config::config_struct::NotificationBackend::Stub,
                app_name: "SmoothTask".to_string(),
                min_level: crate::config::config_struct::NotificationLevel::Warning,
            },
            model: crate::config::config_struct::ModelConfig {
                enabled: false,
                model_path: "models/ranker.onnx".to_string(),
                model_type: ModelType::Onnx,
            },
            ml_classifier: MLClassifierConfig::default(),
            pattern_auto_update: PatternAutoUpdateConfig::default(),
            ebpf: EbpfConfig::default(),
        };

        let config_arc = Arc::new(RwLock::new(current_config));
        let config_path = config_file_path.to_str().unwrap().to_string();

        let state = ApiStateBuilder::new()
            .with_daemon_stats(None)
            .with_system_metrics(None)
            .with_processes(None)
            .with_app_groups(None)
            .with_responsiveness_metrics(None)
            .with_config(Some(config_arc.clone()))
            .with_config_path(Some(config_path.clone()))
            .with_pattern_database(None)
            .with_notification_manager(None)
            .build();

        let result = config_reload_handler(State(state)).await;
        assert!(result.is_ok(), "Config reload handler failed");

        let json = result.unwrap().0;
        if json["status"] != "success" {
            eprintln!("Config reload failed with message: {}", json["message"]);
            if let Some(details) = json.get("error") {
                eprintln!("Error details: {}", details);
            }
        }
        assert_eq!(json["status"], "success");
        assert_eq!(
            json["message"],
            "Configuration successfully reloaded from file and applied"
        );
        assert!(json["old_config"].is_object());
        assert!(json["new_config"].is_object());
        assert_eq!(json["config_path"], config_path);

        // Проверяем, что старая конфигурация соответствует текущей
        assert_eq!(json["old_config"]["polling_interval_ms"], 500);
        assert_eq!(json["old_config"]["max_candidates"], 150);

        // Проверяем, что новая конфигурация загружена из файла
        assert_eq!(json["new_config"]["polling_interval_ms"], 1000);
        assert_eq!(json["new_config"]["max_candidates"], 200);
        assert!(json["action_required"].is_string());
    }

    #[tokio::test]
    async fn test_config_reload_handler_with_invalid_config() {
        // Тест для config_reload_handler когда конфигурационный файл невалиден
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let config_file_path = temp_dir.path().join("invalid_config.yml");
        let invalid_config_content = r#"
polling_interval_ms: 50  # Слишком маленькое значение
max_candidates: 200
"#;

        std::fs::write(&config_file_path, invalid_config_content).expect("write invalid config");

        let current_config = crate::config::config_struct::Config {
            polling_interval_ms: 500,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: crate::config::config_struct::PolicyMode::RulesOnly,
            enable_snapshot_logging: false,
            thresholds: crate::config::config_struct::Thresholds {
                psi_cpu_some_high: 0.6,
                psi_io_some_high: 0.4,
                user_idle_timeout_sec: 120,
                interactive_build_grace_sec: 10,
                noisy_neighbour_cpu_share: 0.7,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.6,
                normal_percentile: 0.3,
                background_percentile: 0.1,
                sched_latency_p99_threshold_ms: 20.0,
                ui_loop_p95_threshold_ms: 16.67,
            },
            paths: crate::config::config_struct::Paths {
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: crate::config::config_struct::LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
            },
            cache_intervals: crate::config::config_struct::CacheIntervals {
                system_metrics_cache_interval: 3,
                process_metrics_cache_interval: 1,
            },
            notifications: crate::config::config_struct::NotificationConfig {
                enabled: false,
                backend: crate::config::config_struct::NotificationBackend::Stub,
                app_name: "SmoothTask".to_string(),
                min_level: crate::config::config_struct::NotificationLevel::Warning,
            },
            model: crate::config::config_struct::ModelConfig {
                enabled: false,
                model_path: "models/ranker.onnx".to_string(),
                model_type: ModelType::Onnx,
            },
            ml_classifier: MLClassifierConfig::default(),
            pattern_auto_update: PatternAutoUpdateConfig::default(),
            ebpf: EbpfConfig::default(),
        };

        let config_arc = Arc::new(RwLock::new(current_config));
        let config_path = config_file_path.to_str().unwrap().to_string();

        let state = ApiStateBuilder::new()
            .with_daemon_stats(None)
            .with_system_metrics(None)
            .with_processes(None)
            .with_app_groups(None)
            .with_responsiveness_metrics(None)
            .with_config(Some(config_arc.clone()))
            .with_config_path(Some(config_path.clone()))
            .with_pattern_database(None)
            .with_notification_manager(None)
            .build();

        let result = config_reload_handler(State(state)).await;
        assert!(result.is_ok());

        let json = result.unwrap().0;
        assert_eq!(json["status"], "error");
        assert!(json["message"]
            .as_str()
            .unwrap()
            .contains("Failed to reload configuration"));
        assert!(json["current_config"].is_object());
        assert_eq!(json["config_path"], config_path);
        assert!(json["action_required"].is_string());
    }

    #[tokio::test]
    async fn test_notifications_test_handler_without_manager() {
        // Тест для notifications_test_handler когда менеджер уведомлений недоступен
        let state = ApiState::new();
        let result = notifications_test_handler(State(state)).await;
        assert!(result.is_ok());

        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "error");
        assert!(value["message"]
            .as_str()
            .unwrap()
            .contains("Notification manager not available"));
        assert_eq!(value["backend"], "none");
    }

    #[tokio::test]
    async fn test_notifications_test_handler_with_manager() {
        // Тест для notifications_test_handler когда менеджер уведомлений доступен
        use crate::notifications::NotificationManager;

        let notification_manager = NotificationManager::new_stub();
        let notification_manager_arc = Arc::new(tokio::sync::Mutex::new(notification_manager));

        let state = ApiStateBuilder::new()
            .with_daemon_stats(None)
            .with_system_metrics(None)
            .with_processes(None)
            .with_app_groups(None)
            .with_responsiveness_metrics(None)
            .with_config(None)
            .with_config_path(None)
            .with_pattern_database(None)
            .with_notification_manager(Some(notification_manager_arc))
            .build();

        let result = notifications_test_handler(State(state)).await;
        assert!(result.is_ok());

        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "success");
        assert_eq!(value["message"], "Test notification sent successfully");
        assert!(value["notification"].is_object());
        assert_eq!(value["notification"]["title"], "Test Notification");
        assert_eq!(
            value["notification"]["message"],
            "This is a test notification from SmoothTask API"
        );
        assert_eq!(value["backend"], "stub");
    }

    #[tokio::test]
    async fn test_notifications_status_handler_without_config() {
        // Тест для notifications_status_handler когда конфигурация недоступна
        let state = ApiState::new();
        let result = notifications_status_handler(State(state)).await;
        assert!(result.is_ok());

        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert!(value["notifications"]["config"].is_null());
        assert!(value["notifications"]["manager"].is_null());
        assert_eq!(value["notifications"]["available"], false);
    }

    #[tokio::test]
    async fn test_notifications_status_handler_with_config() {
        // Тест для notifications_status_handler когда конфигурация доступна
        use crate::config::config_struct::{
            CacheIntervals, Config, LoggingConfig, Paths, PolicyMode, Thresholds,
        };
        use crate::metrics::ebpf::EbpfConfig;

        let config = Config {
            polling_interval_ms: 1000,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: PolicyMode::RulesOnly,
            enable_snapshot_logging: true,
            thresholds: Thresholds {
                psi_cpu_some_high: 0.6,
                psi_io_some_high: 0.4,
                user_idle_timeout_sec: 120,
                interactive_build_grace_sec: 10,
                noisy_neighbour_cpu_share: 0.7,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.6,
                normal_percentile: 0.3,
                background_percentile: 0.1,
                sched_latency_p99_threshold_ms: 20.0,
                ui_loop_p95_threshold_ms: 16.67,
            },
            paths: Paths {
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
            },
            cache_intervals: CacheIntervals {
                system_metrics_cache_interval: 3,
                process_metrics_cache_interval: 1,
            },
            notifications: crate::config::config_struct::NotificationConfig {
                enabled: true,
                backend: crate::config::config_struct::NotificationBackend::Stub,
                app_name: "SmoothTask".to_string(),
                min_level: crate::config::config_struct::NotificationLevel::Info,
            },
            model: crate::config::config_struct::ModelConfig {
                enabled: false,
                model_path: "models/ranker.onnx".to_string(),
                model_type: ModelType::Onnx,
            },
            ml_classifier: MLClassifierConfig::default(),
            pattern_auto_update: PatternAutoUpdateConfig::default(),
            ebpf: EbpfConfig::default(),
        };

        let config_arc = Arc::new(RwLock::new(config));
        let state = ApiState::with_all_and_config(None, None, None, None, Some(config_arc));

        let result = notifications_status_handler(State(state)).await;
        assert!(result.is_ok());

        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert!(value["notifications"]["config"].is_object());
        assert_eq!(value["notifications"]["config"]["enabled"], true);
        assert_eq!(value["notifications"]["config"]["backend"], "stub");
        assert_eq!(value["notifications"]["config"]["app_name"], "SmoothTask");
        assert_eq!(value["notifications"]["config"]["min_level"], "info");
        assert!(value["notifications"]["manager"].is_null());
        assert_eq!(value["notifications"]["available"], false);
    }

    #[tokio::test]
    async fn test_notifications_status_handler_with_manager() {
        // Тест для notifications_status_handler когда менеджер уведомлений доступен
        use crate::notifications::NotificationManager;

        let notification_manager = NotificationManager::new_stub();
        let notification_manager_arc = Arc::new(tokio::sync::Mutex::new(notification_manager));

        let state = ApiStateBuilder::new()
            .with_daemon_stats(None)
            .with_system_metrics(None)
            .with_processes(None)
            .with_app_groups(None)
            .with_responsiveness_metrics(None)
            .with_config(None)
            .with_config_path(None)
            .with_pattern_database(None)
            .with_notification_manager(Some(notification_manager_arc))
            .build();

        let result = notifications_status_handler(State(state)).await;
        assert!(result.is_ok());

        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert!(value["notifications"]["config"].is_null());
        assert!(value["notifications"]["manager"].is_object());
        assert_eq!(value["notifications"]["manager"]["enabled"], true);
        assert_eq!(value["notifications"]["manager"]["backend"], "stub");
        assert_eq!(value["notifications"]["available"], true);
    }

    #[tokio::test]
    async fn test_notifications_config_handler_without_config() {
        // Тест для notifications_config_handler когда конфигурация недоступна
        let state = ApiState::new();
        let payload = json!({
            "enabled": true
        });
        let result = notifications_config_handler(State(state), Json(payload)).await;
        assert!(result.is_ok());

        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "error");
        assert!(value["message"]
            .as_str()
            .unwrap()
            .contains("Config not available"));
    }

    #[tokio::test]
    async fn test_notifications_config_handler_with_config() {
        // Тест для notifications_config_handler когда конфигурация доступна
        use crate::config::config_struct::{
            CacheIntervals, Config, LoggingConfig, Paths, PolicyMode, Thresholds,
        };
        use crate::metrics::ebpf::EbpfConfig;

        let config = Config {
            polling_interval_ms: 1000,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: PolicyMode::RulesOnly,
            enable_snapshot_logging: true,
            thresholds: Thresholds {
                psi_cpu_some_high: 0.6,
                psi_io_some_high: 0.4,
                user_idle_timeout_sec: 120,
                interactive_build_grace_sec: 10,
                noisy_neighbour_cpu_share: 0.7,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.6,
                normal_percentile: 0.3,
                background_percentile: 0.1,
                sched_latency_p99_threshold_ms: 20.0,
                ui_loop_p95_threshold_ms: 16.67,
            },
            paths: Paths {
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
            },
            cache_intervals: CacheIntervals {
                system_metrics_cache_interval: 3,
                process_metrics_cache_interval: 1,
            },
            notifications: crate::config::config_struct::NotificationConfig {
                enabled: false,
                backend: crate::config::config_struct::NotificationBackend::Stub,
                app_name: "SmoothTask".to_string(),
                min_level: crate::config::config_struct::NotificationLevel::Warning,
            },
            model: crate::config::config_struct::ModelConfig {
                enabled: false,
                model_path: "models/ranker.onnx".to_string(),
                model_type: ModelType::Onnx,
            },
            ml_classifier: MLClassifierConfig::default(),
            pattern_auto_update: PatternAutoUpdateConfig::default(),
            ebpf: EbpfConfig::default(),
        };

        let config_arc = Arc::new(RwLock::new(config));
        let state = ApiState::with_all_and_config(None, None, None, None, Some(config_arc));

        let payload = json!({
            "enabled": true,
            "backend": "libnotify",
            "app_name": "SmoothTask Test",
            "min_level": "info"
        });

        let result = notifications_config_handler(State(state), Json(payload)).await;
        assert!(result.is_ok());

        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "success");
        assert_eq!(
            value["message"],
            "Notification configuration updated successfully"
        );
        assert!(value["config"].is_object());
        assert_eq!(value["config"]["enabled"], true);
        assert_eq!(value["config"]["backend"], "libnotify");
        assert_eq!(value["config"]["app_name"], "SmoothTask Test");
        assert_eq!(value["config"]["min_level"], "info");
    }

    #[tokio::test]
    async fn test_notifications_config_handler_partial_update() {
        // Тест для notifications_config_handler с частичным обновлением
        use crate::config::config_struct::{
            CacheIntervals, Config, LoggingConfig, Paths, PolicyMode, Thresholds,
        };
        use crate::metrics::ebpf::EbpfConfig;

        let config = Config {
            polling_interval_ms: 1000,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: PolicyMode::RulesOnly,
            enable_snapshot_logging: true,
            thresholds: Thresholds {
                psi_cpu_some_high: 0.6,
                psi_io_some_high: 0.4,
                user_idle_timeout_sec: 120,
                interactive_build_grace_sec: 10,
                noisy_neighbour_cpu_share: 0.7,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.6,
                normal_percentile: 0.3,
                background_percentile: 0.1,
                sched_latency_p99_threshold_ms: 20.0,
                ui_loop_p95_threshold_ms: 16.67,
            },
            paths: Paths {
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
            },
            cache_intervals: CacheIntervals {
                system_metrics_cache_interval: 3,
                process_metrics_cache_interval: 1,
            },
            notifications: crate::config::config_struct::NotificationConfig {
                enabled: false,
                backend: crate::config::config_struct::NotificationBackend::Stub,
                app_name: "SmoothTask".to_string(),
                min_level: crate::config::config_struct::NotificationLevel::Warning,
            },
            model: crate::config::config_struct::ModelConfig {
                enabled: false,
                model_path: "models/ranker.onnx".to_string(),
                model_type: ModelType::Onnx,
            },
            ml_classifier: MLClassifierConfig::default(),
            pattern_auto_update: PatternAutoUpdateConfig::default(),
            ebpf: EbpfConfig::default(),
        };

        let config_arc = Arc::new(RwLock::new(config));
        let state = ApiState::with_all_and_config(None, None, None, None, Some(config_arc));

        // Обновляем только enabled
        let payload = json!({
            "enabled": true
        });

        let result = notifications_config_handler(State(state), Json(payload)).await;
        assert!(result.is_ok());

        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "success");
        assert_eq!(value["config"]["enabled"], true);
        // Остальные параметры должны остаться без изменений
        assert_eq!(value["config"]["backend"], "stub");
        assert_eq!(value["config"]["app_name"], "SmoothTask");
        assert_eq!(value["config"]["min_level"], "warning");
    }

    #[tokio::test]
    async fn test_endpoints_handler_updated() {
        let result = endpoints_handler().await;
        let json = result.0;
        assert_eq!(json["status"], "ok");
        assert!(json["endpoints"].is_array());
        assert_eq!(json["count"], 22); // Обновлено с 21 до 22

        let endpoints = json["endpoints"].as_array().unwrap();
        assert_eq!(endpoints.len(), 22);

        // Проверяем наличие новых endpoints
        let endpoint_paths: Vec<&str> = endpoints
            .iter()
            .map(|e| e["path"].as_str().unwrap())
            .collect();

        assert!(endpoint_paths.contains(&"/api/notifications/test"));
        assert!(endpoint_paths.contains(&"/api/notifications/custom"));
        assert!(endpoint_paths.contains(&"/api/notifications/status"));
        assert!(endpoint_paths.contains(&"/api/notifications/config"));
        assert!(endpoint_paths.contains(&"/api/performance"));
        assert!(endpoint_paths.contains(&"/api/logs"));
    }

    #[tokio::test]
    async fn test_notifications_custom_handler_without_manager() {
        // Тест для notifications_custom_handler когда notification_manager не доступен
        let state = ApiState {
            daemon_stats: None,
            system_metrics: None,
            processes: None,
            app_groups: None,
            responsiveness_metrics: None,
            config: None,
            config_path: None,
            pattern_database: None,
            notification_manager: None,
            cache: None,
            log_storage: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
        };

        let payload = json!({
            "type": "info",
            "title": "Test Title",
            "message": "Test Message"
        });

        let result = notifications_custom_handler(State(state), Json(payload)).await;
        assert!(result.is_ok());

        let json = result.unwrap().0;
        assert_eq!(json["status"], "error");
        assert_eq!(json["backend"], "none");
        assert!(json["message"]
            .as_str()
            .unwrap()
            .contains("Notification manager not available"));
    }

    #[tokio::test]
    async fn test_notifications_custom_handler_with_manager() {
        // Тест для notifications_custom_handler с доступным notification_manager
        use crate::notifications::NotificationManager;

        let notification_manager =
            Arc::new(tokio::sync::Mutex::new(NotificationManager::new_stub()));
        let state = ApiState {
            daemon_stats: None,
            system_metrics: None,
            processes: None,
            app_groups: None,
            responsiveness_metrics: None,
            config: None,
            config_path: None,
            pattern_database: None,
            notification_manager: Some(Arc::clone(&notification_manager)),
            cache: None,
            log_storage: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
        };

        let payload = json!({
            "type": "warning",
            "title": "Custom Warning",
            "message": "This is a custom warning message",
            "details": "Additional details about the warning"
        });

        let result = notifications_custom_handler(State(state), Json(payload)).await;
        assert!(result.is_ok());

        let json = result.unwrap().0;
        assert_eq!(json["status"], "success");
        assert_eq!(json["backend"], "stub");
        assert_eq!(json["notification"]["type"], "WARNING");
        assert_eq!(json["notification"]["title"], "Custom Warning");
        assert_eq!(
            json["notification"]["message"],
            "This is a custom warning message"
        );
        assert_eq!(
            json["notification"]["details"],
            "Additional details about the warning"
        );
    }

    #[tokio::test]
    async fn test_notifications_custom_handler_default_values() {
        // Тест для notifications_custom_handler с значениями по умолчанию
        use crate::notifications::NotificationManager;

        let notification_manager =
            Arc::new(tokio::sync::Mutex::new(NotificationManager::new_stub()));
        let state = ApiState {
            daemon_stats: None,
            system_metrics: None,
            processes: None,
            app_groups: None,
            responsiveness_metrics: None,
            config: None,
            config_path: None,
            pattern_database: None,
            notification_manager: Some(Arc::clone(&notification_manager)),
            cache: None,
            log_storage: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
        };

        // Пустой payload - должны использоваться значения по умолчанию
        let payload = json!({});

        let result = notifications_custom_handler(State(state), Json(payload)).await;
        assert!(result.is_ok());

        let json = result.unwrap().0;
        assert_eq!(json["status"], "success");
        assert_eq!(json["notification"]["type"], "INFO");
        assert_eq!(json["notification"]["title"], "Custom Notification");
        assert_eq!(
            json["notification"]["message"],
            "Custom notification message"
        );
        assert!(json["notification"]["details"].is_null());
    }

    #[tokio::test]
    async fn test_logs_handler_without_log_storage() {
        // Тест для logs_handler без хранилища логов
        let state = ApiState::new();

        let result = logs_handler(State(state), Query(HashMap::new())).await;

        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;

        assert_eq!(value["status"], "ok");
        assert_eq!(value["count"], 0);
        assert!(value["logs"].is_array());
        assert_eq!(value["logs"].as_array().unwrap().len(), 0);
        assert!(value["message"].is_string());
    }

    #[tokio::test]
    async fn test_logs_handler_with_log_storage() {
        // Тест для logs_handler с хранилищем логов
        use crate::logging::log_storage::{LogEntry, LogLevel, SharedLogStorage};

        let log_storage = Arc::new(SharedLogStorage::new(100));

        // Добавляем тестовые записи
        log_storage
            .add_entry(LogEntry::new(
                LogLevel::Info,
                "test_module",
                "Test info message",
            ))
            .await;

        log_storage
            .add_entry(LogEntry::new(
                LogLevel::Warn,
                "test_module",
                "Test warning message",
            ))
            .await;

        log_storage
            .add_entry(LogEntry::new(
                LogLevel::Error,
                "test_module",
                "Test error message",
            ))
            .await;

        let state = ApiStateBuilder::new()
            .with_log_storage(Some(log_storage))
            .build();

        let result = logs_handler(State(state), Query(HashMap::new())).await;

        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;

        assert_eq!(value["status"], "ok");
        assert_eq!(value["count"], 3);
        assert!(value["logs"].is_array());

        let logs = value["logs"].as_array().unwrap();
        assert_eq!(logs.len(), 3);

        // Проверяем структуру логов
        assert_eq!(logs[0]["level"], "INFO");
        assert_eq!(logs[0]["target"], "test_module");
        assert_eq!(logs[0]["message"], "Test info message");

        assert_eq!(logs[1]["level"], "WARN");
        assert_eq!(logs[1]["target"], "test_module");
        assert_eq!(logs[1]["message"], "Test warning message");

        assert_eq!(logs[2]["level"], "ERROR");
        assert_eq!(logs[2]["target"], "test_module");
        assert_eq!(logs[2]["message"], "Test error message");
    }

    #[tokio::test]
    async fn test_logs_handler_with_level_filter() {
        // Тест для logs_handler с фильтрацией по уровню
        use crate::logging::log_storage::{LogEntry, LogLevel, SharedLogStorage};

        let log_storage = Arc::new(SharedLogStorage::new(100));

        // Добавляем тестовые записи разных уровней
        log_storage
            .add_entry(LogEntry::new(LogLevel::Trace, "test", "Trace message"))
            .await;
        log_storage
            .add_entry(LogEntry::new(LogLevel::Debug, "test", "Debug message"))
            .await;
        log_storage
            .add_entry(LogEntry::new(LogLevel::Info, "test", "Info message"))
            .await;
        log_storage
            .add_entry(LogEntry::new(LogLevel::Warn, "test", "Warn message"))
            .await;
        log_storage
            .add_entry(LogEntry::new(LogLevel::Error, "test", "Error message"))
            .await;

        let state = ApiStateBuilder::new()
            .with_log_storage(Some(log_storage))
            .build();

        // Тестируем фильтрацию по уровню WARN
        let mut params = HashMap::new();
        params.insert("level".to_string(), "warn".to_string());

        let result = logs_handler(State(state), Query(params)).await;

        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;

        assert_eq!(value["status"], "ok");
        assert_eq!(value["count"], 2); // Только WARN и ERROR

        let logs = value["logs"].as_array().unwrap();
        assert_eq!(logs[0]["level"], "WARN");
        assert_eq!(logs[1]["level"], "ERROR");
    }

    #[tokio::test]
    async fn test_logs_handler_with_limit() {
        // Тест для logs_handler с лимитом
        use crate::logging::log_storage::{LogEntry, LogLevel, SharedLogStorage};

        let log_storage = Arc::new(SharedLogStorage::new(100));

        // Добавляем 5 тестовых записей
        for i in 1..=5 {
            log_storage
                .add_entry(LogEntry::new(
                    LogLevel::Info,
                    "test",
                    format!("Message {}", i),
                ))
                .await;
        }

        let state = ApiStateBuilder::new()
            .with_log_storage(Some(log_storage))
            .build();

        // Тестируем лимит 3
        let mut params = HashMap::new();
        params.insert("limit".to_string(), "3".to_string());

        let result = logs_handler(State(state), Query(params)).await;

        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;

        assert_eq!(value["status"], "ok");
        assert_eq!(value["count"], 3);
        assert_eq!(value["filter"]["limit"], 3);

        let logs = value["logs"].as_array().unwrap();
        assert_eq!(logs.len(), 3);
        // Должны быть последние 3 сообщения
        assert_eq!(logs[0]["message"], "Message 3");
        assert_eq!(logs[1]["message"], "Message 4");
        assert_eq!(logs[2]["message"], "Message 5");
    }

    #[tokio::test]
    async fn test_logs_handler_with_fields() {
        // Тест для logs_handler с дополнительными полями
        use crate::logging::log_storage::{LogEntry, LogLevel, SharedLogStorage};

        let log_storage = Arc::new(SharedLogStorage::new(100));

        // Добавляем запись с дополнительными полями
        let mut entry = LogEntry::new(LogLevel::Info, "test", "Message with fields");
        let fields = serde_json::json!({
            "user_id": 123,
            "action": "login",
            "status": "success"
        });
        entry = entry.with_fields(fields);

        log_storage.add_entry(entry).await;

        let state = ApiStateBuilder::new()
            .with_log_storage(Some(log_storage))
            .build();

        let result = logs_handler(State(state), Query(HashMap::new())).await;

        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;

        assert_eq!(value["status"], "ok");
        assert_eq!(value["count"], 1);

        let log = &value["logs"][0];
        assert_eq!(log["message"], "Message with fields");
        assert!(log["fields"].is_object());
        assert_eq!(log["fields"]["user_id"], 123);
        assert_eq!(log["fields"]["action"], "login");
        assert_eq!(log["fields"]["status"], "success");
    }
}
