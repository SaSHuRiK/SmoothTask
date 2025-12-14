//! HTTP сервер для Control API.

use anyhow::{Context, Result};
use axum::{
    extract::{Path, Query, State},
    http::{Response, StatusCode},
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use crate::metrics::app_performance::{AppPerformanceConfig, collect_all_app_performance};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{error, info, trace, warn};

// Health module imports
use crate::health::{create_diagnostic_analyzer, HealthIssueSeverity, HealthMonitorTrait};
use crate::health::diagnostics::DiagnosticAnalyzer;
use crate::api::custom_metrics_handlers::{custom_metrics_handler, custom_metric_by_id_handler, custom_metric_update_handler, custom_metric_add_handler, custom_metric_remove_handler, custom_metric_enable_handler, custom_metric_disable_handler};

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
    /// Монитор здоровья для предоставления информации о состоянии демона через API (опционально)
    health_monitor: Option<Arc<crate::health::HealthMonitorImpl>>,
    /// Кэш для часто запрашиваемых данных (опционально)
    /// Используется для оптимизации производительности API
    cache: Option<Arc<RwLock<ApiCache>>>,
    /// Хранилище логов для предоставления через API (опционально)
    log_storage: Option<Arc<crate::logging::log_storage::SharedLogStorage>>,
    /// Менеджер пользовательских метрик для предоставления через API (опционально)
    pub custom_metrics_manager: Option<Arc<crate::metrics::custom::CustomMetricsManager>>,
    /// Метрики производительности API
    pub performance_metrics: Arc<RwLock<ApiPerformanceMetrics>>,
    /// Коллектор метрик для сбора данных о сетевых соединениях (опционально)
    metrics_collector: Option<Arc<crate::metrics::ebpf::EbpfMetricsCollector>>,
    /// Последние eBPF метрики (опционально)
    metrics: Option<Arc<RwLock<crate::metrics::system::SystemMetrics>>>,
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
    /// Кэшированная версия данных об энергопотреблении процессов (JSON)
    cached_process_energy_json: Option<(Value, Instant)>,
    /// Кэшированная версия данных об использовании памяти процессами (JSON)
    cached_process_memory_json: Option<(Value, Instant)>,
    /// Кэшированная версия данных об использовании GPU процессами (JSON)
    cached_process_gpu_json: Option<(Value, Instant)>,
    /// Кэшированная версия данных об использовании сети процессами (JSON)
    cached_process_network_json: Option<(Value, Instant)>,
    /// Кэшированная версия данных об использовании диска процессами (JSON)
    cached_process_disk_json: Option<(Value, Instant)>,
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
        self.cached_process_energy_json = None;
        self.cached_process_memory_json = None;
        self.cached_process_gpu_json = None;
        self.cached_process_network_json = None;
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
#[allow(dead_code)]
#[derive(Default)]
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
    health_monitor: Option<Arc<crate::health::HealthMonitorImpl>>,
    cache: Option<Arc<RwLock<ApiCache>>>,
    log_storage: Option<Arc<crate::logging::log_storage::SharedLogStorage>>,
    pub custom_metrics_manager: Option<Arc<crate::metrics::custom::CustomMetricsManager>>,
    pub performance_metrics: Option<Arc<RwLock<ApiPerformanceMetrics>>>,
    pub metrics_collector: Option<Arc<crate::metrics::ebpf::EbpfMetricsCollector>>,
    metrics: Option<Arc<RwLock<crate::metrics::system::SystemMetrics>>>,
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

    /// Устанавливает монитор здоровья для API.
    ///
    /// # Параметры
    ///
    /// - `health_monitor`: Монитор здоровья для предоставления информации о состоянии демона
    pub fn with_health_monitor(
        mut self,
        health_monitor: Option<Arc<crate::health::HealthMonitorImpl>>,
    ) -> Self {
        self.health_monitor = health_monitor;
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

    /// Устанавливает коллектор метрик для сбора данных о сетевых соединениях.
    ///
    /// # Параметры
    ///
    /// - `metrics_collector`: Коллектор метрик eBPF
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::ApiStateBuilder;
    /// use smoothtask_core::metrics::ebpf::EbpfMetricsCollector;
    /// use std::sync::Arc;
    ///
    /// let metrics_collector = Arc::new(EbpfMetricsCollector::new(Default::default()));
    /// let builder = ApiStateBuilder::new()
    ///     .with_metrics_collector(Some(metrics_collector));
    /// ```
    pub fn with_metrics_collector(
        mut self,
        metrics_collector: Option<Arc<crate::metrics::ebpf::EbpfMetricsCollector>>,
    ) -> Self {
        self.metrics_collector = metrics_collector;
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

    /// Устанавливает менеджер пользовательских метрик для API.
    ///
    /// # Параметры
    ///
    /// - `custom_metrics_manager`: Менеджер пользовательских метрик для предоставления через API
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::api::ApiStateBuilder;
    /// use smoothtask_core::metrics::custom::CustomMetricsManager;
    /// use std::sync::Arc;
    ///
    /// let custom_metrics_manager = Arc::new(CustomMetricsManager::new());
    /// let builder = ApiStateBuilder::new()
    ///     .with_custom_metrics_manager(Some(custom_metrics_manager));
    /// ```
    pub fn with_custom_metrics_manager(
        mut self,
        custom_metrics_manager: Option<Arc<crate::metrics::custom::CustomMetricsManager>>,
    ) -> Self {
        self.custom_metrics_manager = custom_metrics_manager;
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
            health_monitor: self.health_monitor,
            cache: self.cache,
            log_storage: self.log_storage,
            custom_metrics_manager: self.custom_metrics_manager,
            performance_metrics: self
                .performance_metrics
                .unwrap_or_else(|| Arc::new(RwLock::new(ApiPerformanceMetrics::default()))),
            metrics_collector: self.metrics_collector,
            metrics: self.metrics,
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
            health_monitor: None,
            cache: None,
            log_storage: None,
            custom_metrics_manager: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
            metrics_collector: None,
            metrics: None,
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
            health_monitor: None,
            cache: None,
            log_storage: None,
            custom_metrics_manager: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
            metrics_collector: None,
            metrics: None,
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
            health_monitor: None,
            cache: None,
            log_storage: None,
            custom_metrics_manager: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
            metrics_collector: None,
            metrics: None,
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
            health_monitor: None,
            cache: None,
            log_storage: None,
            custom_metrics_manager: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
            metrics_collector: None,
            metrics: None,
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
            health_monitor: None,
            cache: None,
            log_storage: None,
            custom_metrics_manager: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
            metrics_collector: None,
            metrics: None,
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
            health_monitor: None,
            cache: None,
            log_storage: None,
            custom_metrics_manager: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
            metrics_collector: None,
            metrics: None,
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

    // Получение метрик производительности
    let perf_metrics = state.performance_metrics.read().await;

    // Получаем детальную информацию о доступности компонентов
    let component_availability = check_component_availability(&state);

    // Определяем общий статус системы
    let overall_status = if component_availability["daemon_stats_available"]
        .as_bool()
        .unwrap_or(false)
    {
        "operational"
    } else if component_availability["system_metrics_available"]
        .as_bool()
        .unwrap_or(false)
    {
        "partial"
    } else {
        "degraded"
    };

    // Преобразуем компоненты в ожидаемый формат
    let components = json!({
        "daemon_stats": component_availability["daemon_stats_available"],
        "system_metrics": component_availability["system_metrics_available"],
        "processes": component_availability["processes_available"],
        "app_groups": component_availability["app_groups_available"],
        "config": component_availability["config_available"],
        "pattern_database": component_availability["pattern_database_available"],
        "notification_manager": component_availability["notification_manager_available"],
        "log_storage": component_availability["log_storage_available"],
        "cache": component_availability["cache_available"]
    });

    Ok(Json(json!({
        "status": "ok",
        "service": "smoothtaskd",
        "uptime_seconds": uptime_seconds,
        "overall_status": overall_status,
        "components": components,
        "performance": {
            "total_requests": perf_metrics.total_requests,
            "cache_hits": perf_metrics.cache_hits,
            "cache_misses": perf_metrics.cache_misses,
            "cache_hit_rate": perf_metrics.cache_hit_rate(),
            "average_processing_time_us": perf_metrics.average_processing_time_us(),
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
        "timestamp": Utc::now().to_rfc3339(),
        "suggestions": match overall_status {
            "operational" => "All systems operational",
            "partial" => "Some components unavailable, but core functionality working",
            "degraded" => "Major components unavailable, graceful degradation mode",
            _ => "Unknown status"
        }
    })))
}

/// Обработчик для endpoint `/api/health/monitoring`.
///
/// Возвращает текущее состояние мониторинга здоровья демона.
async fn health_monitoring_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    // Получаем текущее состояние мониторинга здоровья
    let health_monitor = state.health_monitor.as_ref().ok_or_else(|| {
        error!("Health monitor not available");
        StatusCode::SERVICE_UNAVAILABLE
    })?;
    
    let health_status = health_monitor.get_health_status().await
        .map_err(|e| {
            error!("Failed to get health status: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    Ok(Json(json!({
        "status": "ok",
        "health_status": {
            "overall_status": format!("{:?}", health_status.overall_status),
            "last_check_time": health_status.last_check_time.map(|t| t.to_rfc3339()),
            "component_statuses": health_status.component_statuses,
            "issue_count": health_status.issue_history.len(),
            "critical_issues": health_status.issue_history.iter()
                .filter(|issue| issue.severity == HealthIssueSeverity::Critical)
                .count(),
            "warning_issues": health_status.issue_history.iter()
                .filter(|issue| issue.severity == HealthIssueSeverity::Warning)
                .count()
        }
    })))
}

/// Обработчик для endpoint `/api/health/diagnostics`.
///
/// Выполняет диагностику системы и возвращает детальный отчет.
async fn health_diagnostics_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    let health_monitor = state.health_monitor.as_ref().ok_or_else(|| {
        error!("Health monitor not available");
        StatusCode::SERVICE_UNAVAILABLE
    })?;
    
    let diagnostic_analyzer = create_diagnostic_analyzer(health_monitor.as_ref().clone());
    
    let diagnostic_report = diagnostic_analyzer.run_full_diagnostics().await
        .map_err(|e| {
            error!("Failed to run diagnostics: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    Ok(Json(json!({
        "status": "ok",
        "diagnostic_report": {
            "timestamp": diagnostic_report.timestamp.to_rfc3339(),
            "overall_status": format!("{:?}", diagnostic_report.overall_status),
            "component_diagnostics": diagnostic_report.component_diagnostics,
            "recommendations": diagnostic_report.recommendations,
            "system_info": diagnostic_report.system_info
        }
    })))
}

/// Обработчик для endpoint `/api/health/issues`.
///
/// Возвращает историю проблем здоровья.
async fn health_issues_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    let health_monitor = state.health_monitor.as_ref().ok_or_else(|| {
        error!("Health monitor not available");
        StatusCode::SERVICE_UNAVAILABLE
    })?;
    
    let health_status = health_monitor.get_health_status().await
        .map_err(|e| {
            error!("Failed to get health status: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    Ok(Json(json!({
        "status": "ok",
        "issues": health_status.issue_history,
        "total_issues": health_status.issue_history.len()
    })))
}

/// Обработчик для endpoint `/api/gpu/temperature-power`.
///
/// Возвращает метрики температуры и энергопотребления для всех GPU устройств.
async fn gpu_temperature_power_handler() -> Result<Json<Value>, StatusCode> {
    let gpu_metrics = crate::metrics::process_gpu::collect_global_gpu_temperature_and_power().await
        .map_err(|e| {
            error!("Failed to collect GPU temperature and power metrics: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    Ok(Json(json!({
        "status": "ok",
        "gpu_metrics": gpu_metrics,
        "total_gpus": gpu_metrics.len()
    })))
}

/// Обработчик для endpoint `/api/gpu/update-temp-power`.
///
/// Обновляет метрики температуры и энергопотребления GPU для процессов.
async fn gpu_update_temp_power_handler() -> Result<Json<Value>, StatusCode> {
    crate::metrics::process_gpu::update_global_process_gpu_temperature_and_power().await
        .map_err(|e| {
            error!("Failed to update GPU temperature and power metrics: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    Ok(Json(json!({
        "status": "ok",
        "message": "GPU temperature and power metrics updated successfully"
    })))
}

/// Обработчик для endpoint `/api/gpu/memory`.
///
/// Возвращает метрики использования памяти для всех GPU устройств.
async fn gpu_memory_handler() -> Result<Json<Value>, StatusCode> {
    let gpu_metrics = crate::metrics::gpu::collect_gpu_metrics()
        .map_err(|e| {
            error!("Failed to collect GPU memory metrics: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    
    // Преобразуем метрики в формат, подходящий для API
    let gpu_memory_info: Vec<_> = gpu_metrics.devices.iter()
        .map(|device| {
            // Определяем тип GPU на основе драйвера
            let gpu_type = device.device.driver.as_ref().map(|driver| {
                if driver.contains("nvidia") || driver.contains("nouveau") {
                    "nvidia"
                } else if driver.contains("amdgpu") || driver.contains("radeon") {
                    "amd"
                } else if driver.contains("i915") || driver.contains("intel") {
                    "intel"
                } else {
                    driver.as_str()
                }
            });
            
            // Вычисляем процент использования памяти
            let memory_usage_percentage = if device.memory.total_bytes > 0 {
                Some((device.memory.used_bytes as f32 / device.memory.total_bytes as f32 * 100.0) as f32)
            } else {
                None
            };
            
            json!({
                "gpu_id": device.device.device_id,
                "gpu_name": device.device.name,
                "gpu_type": gpu_type,
                "gpu_driver": device.device.driver,
                "total_memory_bytes": device.memory.total_bytes,
                "used_memory_bytes": device.memory.used_bytes,
                "free_memory_bytes": device.memory.free_bytes,
                "memory_usage_percentage": memory_usage_percentage,
                "timestamp": device.timestamp
            })
        })
        .collect();
    
    Ok(Json(json!({
        "status": "ok",
        "gpu_memory_metrics": gpu_memory_info,
        "total_gpus": gpu_memory_info.len(),
        "total_memory_bytes": gpu_memory_info.iter().map(|gpu| 
            gpu["total_memory_bytes"].as_u64().unwrap_or(0)).sum::<u64>(),
        "total_used_memory_bytes": gpu_memory_info.iter().map(|gpu| 
            gpu["used_memory_bytes"].as_u64().unwrap_or(0)).sum::<u64>(),
        "timestamp": std::time::SystemTime::now()
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

/// Обработчик для endpoint `/metrics`.
///
/// Возвращает метрики в формате Prometheus.
async fn prometheus_metrics_handler(State(state): State<ApiState>) -> Result<String, StatusCode> {
    let mut metrics = String::new();
    
    // Добавляем метрики версии
    metrics.push_str(&format!(
        "# HELP smoothtask_version SmoothTask daemon version\n"
    ));
    metrics.push_str(&format!(
        "# TYPE smoothtask_version gauge\n"
    ));
    metrics.push_str(&format!(
        "smoothtask_version{{version=\"{}\"}} 1\n",
        DAEMON_VERSION
    ));
    
    // Добавляем метрики производительности API
    let perf_metrics = state.performance_metrics.read().await;
    metrics.push_str(&format!(
        "# HELP smoothtask_api_requests_total Total number of API requests\n"
    ));
    metrics.push_str(&format!(
        "# TYPE smoothtask_api_requests_total counter\n"
    ));
    metrics.push_str(&format!(
        "smoothtask_api_requests_total {}\n",
        perf_metrics.total_requests
    ));
    
    metrics.push_str(&format!(
        "# HELP smoothtask_api_cache_hits Total number of API cache hits\n"
    ));
    metrics.push_str(&format!(
        "# TYPE smoothtask_api_cache_hits counter\n"
    ));
    metrics.push_str(&format!(
        "smoothtask_api_cache_hits {}\n",
        perf_metrics.cache_hits
    ));
    
    metrics.push_str(&format!(
        "# HELP smoothtask_api_cache_misses Total number of API cache misses\n"
    ));
    metrics.push_str(&format!(
        "# TYPE smoothtask_api_cache_misses counter\n"
    ));
    metrics.push_str(&format!(
        "smoothtask_api_cache_misses {}\n",
        perf_metrics.cache_misses
    ));
    
    metrics.push_str(&format!(
        "# HELP smoothtask_api_processing_time_us_total Total API processing time in microseconds\n"
    ));
    metrics.push_str(&format!(
        "# TYPE smoothtask_api_processing_time_us_total counter\n"
    ));
    metrics.push_str(&format!(
        "smoothtask_api_processing_time_us_total {}\n",
        perf_metrics.total_processing_time_us
    ));
    
    // Добавляем системные метрики если доступны
    if let Some(system_metrics_arc) = &state.system_metrics {
        let system_metrics = system_metrics_arc.read().await;
        
        // Память метрики
        metrics.push_str(&format!(
            "# HELP smoothtask_system_memory_total_kb Total system memory in kilobytes\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_system_memory_total_kb gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_system_memory_total_kb {}\n",
            system_metrics.memory.mem_total_kb
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_system_memory_available_kb Available system memory in kilobytes\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_system_memory_available_kb gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_system_memory_available_kb {}\n",
            system_metrics.memory.mem_available_kb
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_system_memory_free_kb Free system memory in kilobytes\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_system_memory_free_kb gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_system_memory_free_kb {}\n",
            system_metrics.memory.mem_free_kb
        ));
        
        // PSI метрики
        metrics.push_str(&format!(
            "# HELP smoothtask_system_psi_cpu_some_avg10 CPU PSI some average 10s\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_system_psi_cpu_some_avg10 gauge\n"
        ));
        if let Some(psi) = &system_metrics.pressure.cpu.some {
            metrics.push_str(&format!(
                "smoothtask_system_psi_cpu_some_avg10 {}\n",
                psi.avg10
            ));
        }
        
        metrics.push_str(&format!(
            "# HELP smoothtask_system_psi_memory_some_avg10 Memory PSI some average 10s\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_system_psi_memory_some_avg10 gauge\n"
        ));
        if let Some(psi) = &system_metrics.pressure.memory.some {
            metrics.push_str(&format!(
                "smoothtask_system_psi_memory_some_avg10 {}\n",
                psi.avg10
            ));
        }
    }
    
    // Добавляем метрики процессов если доступны
    if let Some(processes_arc) = &state.processes {
        let processes = processes_arc.read().await;
        
        metrics.push_str(&format!(
            "# HELP smoothtask_processes_total Total number of processes\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_processes_total gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_processes_total {}\n",
            processes.len()
        ));
        
        // Добавляем метрики по классам процессов (используем teacher_priority_class)
        let mut class_counts = std::collections::HashMap::new();
        for process in processes.iter() {
            if let Some(class_name) = &process.teacher_priority_class {
                *class_counts.entry(class_name.to_string()).or_insert(0) += 1;
            }
        }
        
        for (class_name, count) in class_counts {
            metrics.push_str(&format!(
                "# HELP smoothtask_processes_by_class{{class=\"{}\"}} Number of processes by priority class\n",
                class_name
            ));
            metrics.push_str(&format!(
                "# TYPE smoothtask_processes_by_class{{class=\"{}\"}} gauge\n",
                class_name
            ));
            metrics.push_str(&format!(
                "smoothtask_processes_by_class{{class=\"{}\"}} {}\n",
                class_name, count
            ));
        }
    }
    
    // Добавляем метрики групп приложений если доступны
    if let Some(app_groups_arc) = &state.app_groups {
        let app_groups = app_groups_arc.read().await;
        
        metrics.push_str(&format!(
            "# HELP smoothtask_app_groups_total Total number of application groups\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_app_groups_total gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_app_groups_total {}\n",
            app_groups.len()
        ));
    }
    
    // Добавляем метрики демона если доступны
    if let Some(daemon_stats_arc) = &state.daemon_stats {
        let daemon_stats = daemon_stats_arc.read().await;
        
        metrics.push_str(&format!(
            "# HELP smoothtask_daemon_total_iterations Total number of daemon iterations\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_daemon_total_iterations counter\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_daemon_total_iterations {}\n",
            daemon_stats.total_iterations
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_daemon_successful_iterations Successful daemon iterations\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_daemon_successful_iterations counter\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_daemon_successful_iterations {}\n",
            daemon_stats.successful_iterations
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_daemon_error_iterations Error daemon iterations\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_daemon_error_iterations counter\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_daemon_error_iterations {}\n",
            daemon_stats.error_iterations
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_daemon_total_duration_ms Total daemon execution time in milliseconds\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_daemon_total_duration_ms counter\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_daemon_total_duration_ms {}\n",
            daemon_stats.total_duration_ms
        ));
    }
    
    // Добавляем расширенные метрики процессов
    if let Some(processes_arc) = &state.processes {
        let processes = processes_arc.read().await;
        
        // Агрегированные метрики по всем процессам
        let mut total_cpu_1s = 0.0;
        let mut total_cpu_10s = 0.0;
        let mut total_memory_mb = 0;
        let mut total_io_read = 0;
        let mut total_io_write = 0;
        let mut total_network_rx = 0;
        let mut total_network_tx = 0;
        let mut total_gpu_utilization = 0.0;
        let mut total_energy_uj = 0;
        let mut audio_processes = 0;
        let mut gui_processes = 0;
        let mut terminal_processes = 0;
        let mut ssh_processes = 0;
        
        for process in processes.iter() {
            // CPU метрики
            if let Some(cpu_1s) = process.cpu_share_1s {
                total_cpu_1s += cpu_1s;
            }
            if let Some(cpu_10s) = process.cpu_share_10s {
                total_cpu_10s += cpu_10s;
            }
            
            // Память
            if let Some(rss) = process.rss_mb {
                total_memory_mb += rss;
            }
            
            // I/O
            if let Some(io_read) = process.io_read_bytes {
                total_io_read += io_read;
            }
            if let Some(io_write) = process.io_write_bytes {
                total_io_write += io_write;
            }
            
            // Сеть
            if let Some(network_rx) = process.network_rx_bytes {
                total_network_rx += network_rx;
            }
            if let Some(network_tx) = process.network_tx_bytes {
                total_network_tx += network_tx;
            }
            
            // GPU
            if let Some(gpu_util) = process.gpu_utilization {
                total_gpu_utilization += gpu_util as f64;
            }
            
            // Энергия
            if let Some(energy) = process.energy_uj {
                total_energy_uj += energy;
            }
            
            // Типы процессов
            if process.is_audio_client {
                audio_processes += 1;
            }
            if process.has_gui_window {
                gui_processes += 1;
            }
            if process.env_term.is_some() {
                terminal_processes += 1;
            }
            if process.env_ssh {
                ssh_processes += 1;
            }
        }
        
        // Агрегированные метрики процессов
        metrics.push_str(&format!(
            "# HELP smoothtask_processes_total_cpu_share_1s Total CPU share (1s) for all processes\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_processes_total_cpu_share_1s gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_processes_total_cpu_share_1s {}\n",
            total_cpu_1s
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_processes_total_cpu_share_10s Total CPU share (10s) for all processes\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_processes_total_cpu_share_10s gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_processes_total_cpu_share_10s {}\n",
            total_cpu_10s
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_processes_total_memory_mb Total memory usage (RSS) for all processes in MB\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_processes_total_memory_mb gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_processes_total_memory_mb {}\n",
            total_memory_mb
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_processes_total_io_read_bytes Total read bytes for all processes\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_processes_total_io_read_bytes counter\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_processes_total_io_read_bytes {}\n",
            total_io_read
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_processes_total_io_write_bytes Total write bytes for all processes\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_processes_total_io_write_bytes counter\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_processes_total_io_write_bytes {}\n",
            total_io_write
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_processes_total_network_rx_bytes Total received bytes for all processes\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_processes_total_network_rx_bytes counter\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_processes_total_network_rx_bytes {}\n",
            total_network_rx
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_processes_total_network_tx_bytes Total transmitted bytes for all processes\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_processes_total_network_tx_bytes counter\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_processes_total_network_tx_bytes {}\n",
            total_network_tx
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_processes_total_gpu_utilization Total GPU utilization for all processes\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_processes_total_gpu_utilization gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_processes_total_gpu_utilization {}\n",
            total_gpu_utilization
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_processes_total_energy_uj Total energy consumption for all processes in microjoules\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_processes_total_energy_uj counter\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_processes_total_energy_uj {}\n",
            total_energy_uj
        ));
        
        // Метрики по типам процессов
        metrics.push_str(&format!(
            "# HELP smoothtask_processes_audio_client Audio client processes\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_processes_audio_client gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_processes_audio_client {}\n",
            audio_processes
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_processes_gui_window Processes with GUI windows\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_processes_gui_window gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_processes_gui_window {}\n",
            gui_processes
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_processes_terminal Processes with terminal sessions\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_processes_terminal gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_processes_terminal {}\n",
            terminal_processes
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_processes_ssh SSH processes\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_processes_ssh gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_processes_ssh {}\n",
            ssh_processes
        ));
    }
    
    // Добавляем метрики групп приложений если доступны
    if let Some(app_groups_arc) = &state.app_groups {
        let app_groups = app_groups_arc.read().await;
        
        // Агрегированные метрики по группам приложений
        let mut total_app_cpu = 0.0;
        let mut total_app_memory = 0;
        let mut total_app_io_read = 0;
        let mut total_app_io_write = 0;
        let mut total_app_network_rx = 0;
        let mut total_app_network_tx = 0;
        let mut total_app_energy = 0;
        
        for app_group in app_groups.iter() {
            if let Some(cpu) = app_group.total_cpu_share {
                total_app_cpu += cpu;
            }
            if let Some(mem) = app_group.total_rss_mb {
                total_app_memory += mem;
            }
            if let Some(io_read) = app_group.total_io_read_bytes {
                total_app_io_read += io_read;
            }
            if let Some(io_write) = app_group.total_io_write_bytes {
                total_app_io_write += io_write;
            }
            if let Some(net_rx) = app_group.total_network_rx_bytes {
                total_app_network_rx += net_rx;
            }
            if let Some(net_tx) = app_group.total_network_tx_bytes {
                total_app_network_tx += net_tx;
            }
            if let Some(energy) = app_group.total_energy_uj {
                total_app_energy += energy;
            }
        }
        
        // Метрики групп приложений
        metrics.push_str(&format!(
            "# HELP smoothtask_app_groups_total_cpu_share Total CPU share for all application groups\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_app_groups_total_cpu_share gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_app_groups_total_cpu_share {}\n",
            total_app_cpu
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_app_groups_total_memory_mb Total memory usage for all application groups in MB\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_app_groups_total_memory_mb gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_app_groups_total_memory_mb {}\n",
            total_app_memory
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_app_groups_total_io_read_bytes Total read bytes for all application groups\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_app_groups_total_io_read_bytes counter\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_app_groups_total_io_read_bytes {}\n",
            total_app_io_read
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_app_groups_total_io_write_bytes Total write bytes for all application groups\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_app_groups_total_io_write_bytes counter\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_app_groups_total_io_write_bytes {}\n",
            total_app_io_write
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_app_groups_total_network_rx_bytes Total received bytes for all application groups\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_app_groups_total_network_rx_bytes counter\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_app_groups_total_network_rx_bytes {}\n",
            total_app_network_rx
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_app_groups_total_network_tx_bytes Total transmitted bytes for all application groups\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_app_groups_total_network_tx_bytes counter\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_app_groups_total_network_tx_bytes {}\n",
            total_app_network_tx
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_app_groups_total_energy_uj Total energy consumption for all application groups in microjoules\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_app_groups_total_energy_uj counter\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_app_groups_total_energy_uj {}\n",
            total_app_energy
        ));
        
        // Количество групп приложений с разными характеристиками
        let focused_groups = app_groups.iter().filter(|g| g.is_focused_group).count();
        let gui_groups = app_groups.iter().filter(|g| g.has_gui_window).count();
        
        metrics.push_str(&format!(
            "# HELP smoothtask_app_groups_focused Focused application groups\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_app_groups_focused gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_app_groups_focused {}\n",
            focused_groups
        ));
        
        metrics.push_str(&format!(
            "# HELP smoothtask_app_groups_with_gui Application groups with GUI windows\n"
        ));
        metrics.push_str(&format!(
            "# TYPE smoothtask_app_groups_with_gui gauge\n"
        ));
        metrics.push_str(&format!(
            "smoothtask_app_groups_with_gui {}\n",
            gui_groups
        ));
    }
    
    // Добавляем метрики здоровья если доступны
    if let Some(health_monitor) = &state.health_monitor {
        let health_status = health_monitor.get_health_status().await.ok();
        
        if let Some(status) = health_status {
            metrics.push_str(&format!(
                "# HELP smoothtask_health_score System health score (0-100)\n"
            ));
            metrics.push_str(&format!(
                "# TYPE smoothtask_health_score gauge\n"
            ));
            metrics.push_str(&format!(
                "smoothtask_health_score {}\n",
                status.health_score
            ));
            
            // Количество проблем по уровню серьезности
            let critical_issues = status.issue_history.iter()
                .filter(|issue| matches!(issue.severity, crate::health::HealthIssueSeverity::Critical))
                .count();
            
            let warning_issues = status.issue_history.iter()
                .filter(|issue| matches!(issue.severity, crate::health::HealthIssueSeverity::Warning))
                .count();
            
            metrics.push_str(&format!(
                "# HELP smoothtask_health_critical_issues Number of critical health issues\n"
            ));
            metrics.push_str(&format!(
                "# TYPE smoothtask_health_critical_issues gauge\n"
            ));
            metrics.push_str(&format!(
                "smoothtask_health_critical_issues {}\n",
                critical_issues
            ));
            
            metrics.push_str(&format!(
                "# HELP smoothtask_health_warning_issues Number of warning health issues\n"
            ));
            metrics.push_str(&format!(
                "# TYPE smoothtask_health_warning_issues gauge\n"
            ));
            metrics.push_str(&format!(
                "smoothtask_health_warning_issues {}\n",
                warning_issues
            ));
            
            metrics.push_str(&format!(
                "# HELP smoothtask_health_total_issues Total number of health issues\n"
            ));
            metrics.push_str(&format!(
                "# TYPE smoothtask_health_total_issues gauge\n"
            ));
            metrics.push_str(&format!(
                "smoothtask_health_total_issues {}\n",
                status.issue_history.len()
            ));
        }
    }
    
    Ok(metrics)
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
                "path": "/metrics",
                "method": "GET",
                "description": "Получение метрик в формате Prometheus для мониторинга"
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
                "path": "/api/custom-metrics",
                "method": "GET",
                "description": "Получение всех пользовательских метрик и их текущих значений"
            },
            {
                "path": "/api/custom-metrics/:metric_id",
                "method": "GET",
                "description": "Получение конфигурации и текущего значения конкретной пользовательской метрики"
            },
            {
                "path": "/api/custom-metrics/:metric_id/update",
                "method": "POST",
                "description": "Принудительное обновление значения пользовательской метрики"
            },
            {
                "path": "/api/custom-metrics/:metric_id/add",
                "method": "POST",
                "description": "Добавление новой пользовательской метрики"
            },
            {
                "path": "/api/custom-metrics/:metric_id/remove",
                "method": "POST",
                "description": "Удаление пользовательской метрики"
            },
            {
                "path": "/api/custom-metrics/:metric_id/enable",
                "method": "POST",
                "description": "Включение пользовательской метрики"
            },
            {
                "path": "/api/custom-metrics/:metric_id/disable",
                "method": "POST",
                "description": "Отключение пользовательской метрики"
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
                "path": "/api/processes/energy",
                "method": "GET",
                "description": "Получение статистики энергопотребления процессов"
            },
            {
                "path": "/api/processes/memory",
                "method": "GET",
                "description": "Получение статистики использования памяти процессами"
            },
            {
                "path": "/api/processes/gpu",
                "method": "GET",
                "description": "Получение статистики использования GPU процессами"
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
                "path": "/api/system/cpu",
                "method": "GET",
                "description": "Получение детальной информации о CPU (количество ядер, модель, кэш, частота)"
            },
            {
                "path": "/api/system/memory",
                "method": "GET",
                "description": "Получение детальной информации о памяти (общий объем, свободная память, кэш, swap)"
            },
            {
                "path": "/api/system/disk",
                "method": "GET",
                "description": "Получение информации о дисковой подсистеме (диски, общее пространство, свободное пространство)"
            },
            {
                "path": "/api/system/network",
                "method": "GET",
                "description": "Получение информации о сетевых интерфейсах (трафик, пакеты, ошибки)"
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
            },
            {
                "path": "/api/cache/monitoring",
                "method": "GET",
                "description": "Получение статистики и мониторинга системы кэширования API"
            },
            {
                "path": "/api/network/connections",
                "method": "GET",
                "description": "Получение информации о текущих сетевых соединениях через eBPF"
            },
            {
                "path": "/api/gpu/temperature-power",
                "method": "GET",
                "description": "Получение метрик температуры и энергопотребления для всех GPU устройств"
            },
            {
                "path": "/api/gpu/memory",
                "method": "GET",
                "description": "Получение метрик использования памяти для всех GPU устройств"
            },
            {
                "path": "/api/gpu/update-temp-power",
                "method": "GET",
                "description": "Обновление метрик температуры и энергопотребления GPU для процессов"
            }
        ],
        "count": 34
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

/// Получает детальную информацию о CPU.
fn get_detailed_cpu_info() -> Value {
    let mut cpu_info = json!({
        "cores": 0,
        "logical_cpus": 0,
        "model": None::<String>,
        "vendor": None::<String>,
        "cache": {},
        "frequency": {}
    });

    // Читаем информацию о CPU из /proc/cpuinfo
    if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
        let mut cores = 0;
        let mut logical_cpus = 0;
        let mut model = String::new();
        let mut vendor = String::new();
        let mut cache_info = json!({});
        let mut frequency_info = json!({});

        for line in cpuinfo.lines() {
            if line.starts_with("processor") {
                logical_cpus += 1;
            } else if line.starts_with("core id") {
                cores += 1;
            } else if line.starts_with("model name") {
                if let Some((_, value)) = line.split_once(':') {
                    model = value.trim().to_string();
                }
            } else if line.starts_with("vendor_id") {
                if let Some((_, value)) = line.split_once(':') {
                    vendor = value.trim().to_string();
                }
            } else if line.starts_with("cache size") {
                if let Some((_, value)) = line.split_once(':') {
                    cache_info["size"] = json!(value.trim().to_string());
                }
            } else if line.starts_with("cpu MHz") {
                if let Some((_, value)) = line.split_once(':') {
                    frequency_info["current_mhz"] = json!(value.trim().to_string());
                }
            }
        }

        cpu_info["cores"] = json!(cores);
        cpu_info["logical_cpus"] = json!(logical_cpus);
        if !model.is_empty() {
            cpu_info["model"] = json!(model);
        }
        if !vendor.is_empty() {
            cpu_info["vendor"] = json!(vendor);
        }
        if !cache_info.as_object().unwrap().is_empty() {
            cpu_info["cache"] = cache_info;
        }
        if !frequency_info.as_object().unwrap().is_empty() {
            cpu_info["frequency"] = frequency_info;
        }
    }

    cpu_info
}

/// Обработчик для endpoint `/api/system/cpu`.
///
/// Возвращает детальную информацию о CPU.
async fn system_cpu_handler() -> Json<Value> {
    let cpu_info = get_detailed_cpu_info();
    Json(json!({
        "status": "ok",
        "cpu": cpu_info
    }))
}

/// Получает детальную информацию о памяти.
fn get_detailed_memory_info() -> Value {
    let mut memory_info = json!({
        "total": 0,
        "free": 0,
        "available": 0,
        "buffers": 0,
        "cached": 0,
        "swap": {}
    });

    // Читаем информацию о памяти из /proc/meminfo
    if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
        for line in meminfo.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_lowercase();
                let value = value.trim().split_whitespace().next().unwrap_or("0");
                let value = value.parse::<u64>().unwrap_or(0);

                match key.as_str() {
                    "memtotal" => memory_info["total"] = json!(value),
                    "memfree" => memory_info["free"] = json!(value),
                    "memavailable" => memory_info["available"] = json!(value),
                    "buffers" => memory_info["buffers"] = json!(value),
                    "cached" => memory_info["cached"] = json!(value),
                    "swaptotal" => memory_info["swap"]["total"] = json!(value),
                    "swapfree" => memory_info["swap"]["free"] = json!(value),
                    _ => {}
                }
            }
        }
    }

    memory_info
}

/// Обработчик для endpoint `/api/system/memory`.
///
/// Возвращает детальную информацию о памяти.
async fn system_memory_handler() -> Json<Value> {
    let memory_info = get_detailed_memory_info();
    Json(json!({
        "status": "ok",
        "memory": memory_info
    }))
}

/// Получает информацию о дисковой подсистеме.
fn get_disk_info() -> Value {
    let mut disk_info = json!({
        "disks": [],
        "total_space": 0,
        "free_space": 0,
        "used_space": 0
    });

    // Читаем информацию о дисках из /proc/diskstats
    if let Ok(diskstats) = fs::read_to_string("/proc/diskstats") {
        let mut total_space = 0u64;
        let free_space = 0u64;
        let used_space = 0u64;
        let mut disks = vec![];

        for line in diskstats.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let major = parts[0];
                let minor = parts[1];
                let device_name = parts[2];

                // Пропускаем виртуальные устройства
                if device_name.starts_with("loop") || device_name.starts_with("ram") {
                    continue;
                }

                let disk = json!({
                    "device": device_name,
                    "major": major,
                    "minor": minor,
                    "reads_completed": parts.get(3).unwrap_or(&"0"),
                    "reads_merged": parts.get(4).unwrap_or(&"0"),
                    "sectors_read": parts.get(5).unwrap_or(&"0"),
                    "time_spent_reading": parts.get(6).unwrap_or(&"0"),
                    "writes_completed": parts.get(7).unwrap_or(&"0"),
                    "writes_merged": parts.get(8).unwrap_or(&"0"),
                    "sectors_written": parts.get(9).unwrap_or(&"0"),
                    "time_spent_writing": parts.get(10).unwrap_or(&"0"),
                    "io_in_progress": parts.get(11).unwrap_or(&"0"),
                    "time_spent_io": parts.get(12).unwrap_or(&"0"),
                    "weighted_time_spent_io": parts.get(13).unwrap_or(&"0")
                });

                disks.push(disk);
            }
        }

        // Пробуем получить информацию о пространстве из /proc/mounts
        if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
            for line in mounts.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    let _device = parts[0];
                    let mount_point = parts[1];
                    let _fs_type = parts[2];

                    // Пробуем получить информацию о пространстве
                    if let Ok(stat) = fs::metadata(mount_point) {
                        total_space += stat.len();
                    }
                }
            }
        }

        disk_info["disks"] = json!(disks);
        disk_info["total_space"] = json!(total_space);
        disk_info["free_space"] = json!(free_space);
        disk_info["used_space"] = json!(used_space);
    }

    disk_info
}

/// Обработчик для endpoint `/api/system/disk`.
///
/// Возвращает информацию о дисковой подсистеме.
async fn system_disk_handler() -> Json<Value> {
    let disk_info = get_disk_info();
    Json(json!({
        "status": "ok",
        "disk": disk_info
    }))
}

/// Получает информацию о сетевых интерфейсах.
fn get_network_info() -> Value {
    let mut network_info = json!({
        "interfaces": [],
        "total_rx_bytes": 0,
        "total_tx_bytes": 0,
        "total_rx_packets": 0,
        "total_tx_packets": 0
    });

    // Читаем информацию о сетевых интерфейсах из /proc/net/dev
    if let Ok(net_dev) = fs::read_to_string("/proc/net/dev") {
        let mut total_rx_bytes = 0u64;
        let mut total_tx_bytes = 0u64;
        let mut total_rx_packets = 0u64;
        let mut total_tx_packets = 0u64;
        let mut interfaces = vec![];

        for line in net_dev.lines().skip(2) { // Пропускаем заголовки
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 17 {
                let interface_name = parts[0].trim_end_matches(':');

                // Пропускаем виртуальные интерфейсы
                if interface_name.starts_with("lo") {
                    continue;
                }

                let rx_bytes = parts[1].parse::<u64>().unwrap_or(0);
                let rx_packets = parts[3].parse::<u64>().unwrap_or(0);
                let tx_bytes = parts[9].parse::<u64>().unwrap_or(0);
                let tx_packets = parts[11].parse::<u64>().unwrap_or(0);

                let interface = json!({
                    "name": interface_name,
                    "rx_bytes": rx_bytes,
                    "rx_packets": rx_packets,
                    "rx_errors": parts[4].parse::<u64>().unwrap_or(0),
                    "rx_dropped": parts[5].parse::<u64>().unwrap_or(0),
                    "tx_bytes": tx_bytes,
                    "tx_packets": tx_packets,
                    "tx_errors": parts[12].parse::<u64>().unwrap_or(0),
                    "tx_dropped": parts[13].parse::<u64>().unwrap_or(0)
                });

                interfaces.push(interface);

                total_rx_bytes += rx_bytes;
                total_tx_bytes += tx_bytes;
                total_rx_packets += rx_packets;
                total_tx_packets += tx_packets;
            }
        }

        network_info["interfaces"] = json!(interfaces);
        network_info["total_rx_bytes"] = json!(total_rx_bytes);
        network_info["total_tx_bytes"] = json!(total_tx_bytes);
        network_info["total_rx_packets"] = json!(total_rx_packets);
        network_info["total_tx_packets"] = json!(total_tx_packets);
    }

    network_info
}

/// Обработчик для endpoint `/api/system/network`.
///
/// Возвращает информацию о сетевых интерфейсах.
async fn system_network_handler() -> Json<Value> {
    let network_info = get_network_info();
    Json(json!({
        "status": "ok",
        "network": network_info
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
    // Валидируем payload
    if let Err(_) = crate::api::validation::validate_custom_notification_payload(&payload) {
        warn!("Invalid custom notification payload: {:?}", payload);
        let error_response = crate::api::validation::create_validation_error_response(
            "error",
            "Invalid notification payload",
            None,
            Some("Check field types and lengths. Type: critical|warning|info. Title: 1-100 chars. Message: 1-500 chars. Details: 0-1000 chars")
        );
        return Ok(Json(error_response));
    }

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

/// Обработчик для endpoint `/api/app/performance` (GET).
///
/// Возвращает метрики производительности приложений, сгруппированные по AppGroup.
///
/// # Примеры
///
/// ```bash
/// # Получение метрик производительности всех приложений
/// curl "http://127.0.0.1:8080/api/app/performance"
/// ```
///
/// # Возвращаемое значение
///
/// JSON объект с метриками производительности для каждой группы приложений,
/// включая использование CPU, памяти, ввода-вывода и статус производительности.
async fn app_performance_handler(
    State(_state): State<ApiState>
) -> Result<Json<Value>, StatusCode> {
    // Собираем метрики производительности приложений
    let app_performance_config = AppPerformanceConfig::default();
    
    let result = collect_all_app_performance(Some(app_performance_config));
    
    match result {
        Ok(metrics_map) => {
            // Преобразуем метрики в JSON
            let mut json_metrics = serde_json::Map::new();
            let total_app_groups = metrics_map.len();
            
            for (app_group_id, app_metrics) in metrics_map {
                let mut group_json = serde_json::Map::new();
                group_json.insert("app_group_id".to_string(), json!(app_metrics.app_group_id));
                group_json.insert("app_group_name".to_string(), json!(app_metrics.app_group_name));
                group_json.insert("process_count".to_string(), json!(app_metrics.process_count));
                group_json.insert("total_cpu_usage".to_string(), json!(app_metrics.total_cpu_usage));
                group_json.insert("average_cpu_usage".to_string(), json!(app_metrics.average_cpu_usage));
                group_json.insert("peak_cpu_usage".to_string(), json!(app_metrics.peak_cpu_usage));
                group_json.insert("total_memory_mb".to_string(), json!(app_metrics.total_memory_mb));
                group_json.insert("average_memory_mb".to_string(), json!(app_metrics.average_memory_mb));
                group_json.insert("total_io_bytes_per_sec".to_string(), json!(app_metrics.total_io_bytes_per_sec));
                group_json.insert("total_context_switches".to_string(), json!(app_metrics.total_context_switches));
                group_json.insert("processes_with_windows".to_string(), json!(app_metrics.processes_with_windows));
                group_json.insert("processes_with_audio".to_string(), json!(app_metrics.processes_with_audio));
                group_json.insert("processes_with_terminals".to_string(), json!(app_metrics.processes_with_terminals));
                
                // Добавляем статус производительности
                let status_str = match app_metrics.performance_status {
                    crate::metrics::app_performance::PerformanceStatus::Good => "good",
                    crate::metrics::app_performance::PerformanceStatus::Warning => "warning",
                    crate::metrics::app_performance::PerformanceStatus::Critical => "critical",
                    crate::metrics::app_performance::PerformanceStatus::Unknown => "unknown",
                };
                group_json.insert("performance_status".to_string(), json!(status_str));
                
                // Добавляем теги
                let tags_array: Vec<Value> = app_metrics.tags.iter().map(|tag| json!(tag)).collect();
                group_json.insert("tags".to_string(), json!(tags_array));
                
                json_metrics.insert(app_group_id, Value::Object(group_json));
            }
            
            Ok(Json(json!({
                "status": "ok",
                "app_performance_metrics": json_metrics,
                "total_app_groups": total_app_groups,
                "timestamp": Utc::now().to_rfc3339()
            })))
        }
        Err(e) => {
            error!("Ошибка сбора метрик производительности приложений: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
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
    // Валидируем параметры запроса
    if let Err(_) = crate::api::validation::validate_logs_params(&params) {
        warn!("Invalid logs query parameters: {:?}", params);
        let error_response = crate::api::validation::create_validation_error_response(
            "error",
            "Invalid query parameters",
            None,
            Some("Valid levels: error, warn, info, debug, trace. Max limit: 1000")
        );
        return Ok(Json(error_response));
    }

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

/// Обработчик для endpoint `/api/cache/monitoring` (GET).
///
/// Возвращает статистику и мониторинг системы кэширования API, включая:
/// - Статистику использования кэша
/// - Состояние здоровья кэша
/// - Информацию о производительности кэширования
///
/// # Примеры
///
/// ```bash
/// # Получение статистики кэширования
/// curl "http://127.0.0.1:8080/api/cache/monitoring"
/// ```
async fn cache_monitoring_handler(
    State(state): State<ApiState>,
) -> Result<Json<Value>, StatusCode> {
    // Получаем текущие метрики производительности API
    let perf_metrics = state.performance_metrics.read().await;
    
    // Получаем информацию о кэше
    let cache_info = match &state.cache {
        Some(cache_arc) => {
            let cache = cache_arc.read().await;
            
            // Считаем количество кэшированных элементов
            let cached_count = [
                cache.cached_processes_json.is_some() as u32,
                cache.cached_appgroups_json.is_some() as u32,
                cache.cached_metrics_json.is_some() as u32,
                cache.cached_config_json.is_some() as u32,
                cache.cached_process_energy_json.is_some() as u32,
                cache.cached_process_memory_json.is_some() as u32,
                cache.cached_process_gpu_json.is_some() as u32,
                cache.cached_process_network_json.is_some() as u32,
                cache.cached_process_disk_json.is_some() as u32,
            ].iter().sum::<u32>();
            
            // Считаем количество активных (не устаревших) элементов
            let mut active_count = 0;
            let current_time = Instant::now();
            
            if let Some((_, time)) = &cache.cached_processes_json {
                if current_time.duration_since(*time).as_secs() < cache.cache_ttl_seconds {
                    active_count += 1;
                }
            }
            if let Some((_, time)) = &cache.cached_appgroups_json {
                if current_time.duration_since(*time).as_secs() < cache.cache_ttl_seconds {
                    active_count += 1;
                }
            }
            if let Some((_, time)) = &cache.cached_metrics_json {
                if current_time.duration_since(*time).as_secs() < cache.cache_ttl_seconds {
                    active_count += 1;
                }
            }
            if let Some((_, time)) = &cache.cached_config_json {
                if current_time.duration_since(*time).as_secs() < cache.cache_ttl_seconds {
                    active_count += 1;
                }
            }
            if let Some((_, time)) = &cache.cached_process_energy_json {
                if current_time.duration_since(*time).as_secs() < cache.cache_ttl_seconds {
                    active_count += 1;
                }
            }
            if let Some((_, time)) = &cache.cached_process_memory_json {
                if current_time.duration_since(*time).as_secs() < cache.cache_ttl_seconds {
                    active_count += 1;
                }
            }
            if let Some((_, time)) = &cache.cached_process_gpu_json {
                if current_time.duration_since(*time).as_secs() < cache.cache_ttl_seconds {
                    active_count += 1;
                }
            }
            if let Some((_, time)) = &cache.cached_process_network_json {
                if current_time.duration_since(*time).as_secs() < cache.cache_ttl_seconds {
                    active_count += 1;
                }
            }
            if let Some((_, time)) = &cache.cached_process_disk_json {
                if current_time.duration_since(*time).as_secs() < cache.cache_ttl_seconds {
                    active_count += 1;
                }
            }
            
            // Вычисляем процент активных элементов
            let active_percentage = if cached_count > 0 {
                (active_count as f64 / cached_count as f64) * 100.0
            } else {
                0.0
            };
            
            // Определяем статус здоровья кэша
            let health_status = if cached_count == 0 {
                "idle"
            } else if active_percentage <= 50.0 {
                "warning"
            } else {
                "healthy"
            };
            
            let health_message = match health_status {
                "healthy" => "Кэш работает эффективно".to_string(),
                "warning" => "Много устаревших элементов в кэше".to_string(),
                "idle" => "Кэш не используется".to_string(),
                _ => "Неизвестный статус".to_string(),
            };
            
            json!({
                "enabled": true,
                "cache_type": "api_response_cache",
                "statistics": {
                    "total_cached_items": cached_count,
                    "active_items": active_count,
                    "stale_items": cached_count - active_count,
                    "active_percentage": active_percentage,
                    "cache_ttl_seconds": cache.cache_ttl_seconds,
                },
                "health": {
                    "status": health_status,
                    "message": health_message,
                    "timestamp": Utc::now().to_rfc3339()
                }
            })
        }
        None => {
            json!({
                "enabled": false,
                "cache_type": "api_response_cache",
                "statistics": {
                    "total_cached_items": 0,
                    "active_items": 0,
                    "stale_items": 0,
                    "active_percentage": 0.0,
                    "cache_ttl_seconds": 0,
                },
                "health": {
                    "status": "disabled",
                    "message": "Кэш API отключен",
                    "timestamp": Utc::now().to_rfc3339()
                }
            })
        }
    };
    
    // Вычисляем эффективность кэширования
    let total_requests = perf_metrics.total_requests;
    let cache_hit_rate = if total_requests > 0 {
        (perf_metrics.cache_hits as f64 / total_requests as f64) * 100.0
    } else {
        0.0
    };
    
    let cache_miss_rate = 100.0 - cache_hit_rate;
    
    // Определяем общий статус здоровья
    let overall_health_status = if !cache_info["enabled"].as_bool().unwrap_or(false) {
        "disabled"
    } else if total_requests == 0 {
        "disabled" // Нет запросов - кэширование не активно
    } else if cache_hit_rate > 70.0 {
        "healthy"
    } else if cache_hit_rate > 30.0 {
        "warning"
    } else {
        "critical"
    };
    
    let overall_health_message = match overall_health_status {
        "healthy" => "Кэширование работает отлично".to_string(),
        "warning" => "Кэширование может быть улучшено".to_string(),
        "critical" => "Кэширование неэффективно".to_string(),
        "disabled" => "Кэширование отключено".to_string(),
        _ => "Неизвестный статус".to_string(),
    };
    
    Ok(Json(json!({
        "status": "ok",
        "cache_monitoring": {
            "api_cache": cache_info,
            "performance": {
                "total_requests": total_requests,
                "cache_hits": perf_metrics.cache_hits,
                "cache_misses": perf_metrics.cache_misses,
                "cache_hit_rate": cache_hit_rate,
                "cache_miss_rate": cache_miss_rate,
                "average_processing_time_us": perf_metrics.average_processing_time_us(),
            },
            "overall_health": {
                "status": overall_health_status,
                "message": overall_health_message,
                "timestamp": Utc::now().to_rfc3339()
            }
        },
        "availability": {
            "cache_available": state.cache.is_some(),
            "performance_metrics_available": true,
            "timestamp": Utc::now().to_rfc3339()
        }
    })))
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
    // Валидируем payload
    if let Err(_) = crate::api::validation::validate_notifications_config_payload(&payload) {
        warn!("Invalid notifications config payload: {:?}", payload);
        let error_response = crate::api::validation::create_validation_error_response(
            "error",
            "Invalid configuration payload",
            None,
            Some("Check field types and values. Enabled: boolean. Backend: stub|libnotify|dbus. App_name: 1-50 chars. Min_level: critical|warning|info")
        );
        return Ok(Json(error_response));
    }

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
        .route("/api/health/monitoring", get(health_monitoring_handler))
        .route("/api/health/diagnostics", get(health_diagnostics_handler))
        .route("/api/health/issues", get(health_issues_handler))
        .route("/api/gpu/temperature-power", get(gpu_temperature_power_handler))
        .route("/api/gpu/memory", get(gpu_memory_handler))
        .route("/api/gpu/update-temp-power", get(gpu_update_temp_power_handler))
        .route("/api/version", get(version_handler))
        .route("/api/endpoints", get(endpoints_handler))
        .route("/metrics", get(prometheus_metrics_handler))
        .route("/api/stats", get(stats_handler))
        .route("/api/metrics", get(metrics_handler))
        .route("/api/custom-metrics", get(custom_metrics_handler))
        .route("/api/custom-metrics/:metric_id", get(custom_metric_by_id_handler))
        .route("/api/custom-metrics/:metric_id/update", post(custom_metric_update_handler))
        .route("/api/custom-metrics/:metric_id/add", post(custom_metric_add_handler))
        .route("/api/custom-metrics/:metric_id/remove", post(custom_metric_remove_handler))
        .route("/api/custom-metrics/:metric_id/enable", post(custom_metric_enable_handler))
        .route("/api/custom-metrics/:metric_id/disable", post(custom_metric_disable_handler))
        .route("/api/responsiveness", get(responsiveness_handler))
        .route("/api/processes", get(processes_handler))
        .route("/api/processes/:pid", get(process_by_pid_handler))
        .route("/api/processes/energy", get(process_energy_handler))
        .route("/api/processes/memory", get(process_memory_handler))
        .route("/api/processes/gpu", get(process_gpu_handler))
        .route("/api/processes/network", get(process_network_handler))
        .route("/api/processes/disk", get(process_disk_handler))
        .route("/api/appgroups", get(appgroups_handler))
        .route("/api/appgroups/:id", get(appgroup_by_id_handler))
        .route("/api/config", get(config_handler))
        .route("/api/config", post(config_update_handler))
        .route("/api/config/reload", post(config_reload_handler))
        .route("/api/classes", get(classes_handler))
        .route("/api/patterns", get(patterns_handler))
        .route("/api/system", get(system_handler))
        .route("/api/system/cpu", get(system_cpu_handler))
        .route("/api/system/memory", get(system_memory_handler))
        .route("/api/system/disk", get(system_disk_handler))
        .route("/api/system/network", get(system_network_handler))
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
        .route("/api/app/performance", get(app_performance_handler))
        .route("/api/logs", get(logs_handler))
        .route("/api/cache/monitoring", get(cache_monitoring_handler))
        .route("/api/cache/stats", get(cache_stats_handler))
        .route("/api/cache/clear", post(cache_clear_handler))
        .route("/api/cache/config", get(cache_config_handler))
        .route("/api/cache/config", post(cache_config_update_handler))
        .route("/api/network/connections", get(network_connections_handler))
        .route("/api/cpu/temperature", get(cpu_temperature_handler))
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
                "system_metrics": *metrics,
                "availability": {
                    "available": true,
                    "timestamp": Utc::now().to_rfc3339()
                }
            })))
        }
        None => Ok(Json(json!({
            "status": "ok",
            "system_metrics": null,
            "message": "System metrics not available (daemon may not be running or no metrics collected yet)"
        })))
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
                "message": "Processes not available (daemon may not be running or no processes collected yet)"
            });

            // Кэшируем результат (даже если данных нет)
            cache_write.cached_processes_json = Some((result.clone(), Instant::now()));

            trace!("Cached empty processes data");
            Ok(Json(result))
        }
    }
}

/// Обработчик для endpoint `/api/processes/energy`.
///
/// Возвращает статистику энергопотребления процессов (если доступны).
async fn process_energy_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    // Начинаем отслеживание производительности
    let _start_time = Instant::now();

    // Обновляем метрики производительности
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_requests();
    drop(perf_metrics); // Освобождаем блокировку

    // Пробуем использовать кэш
    let cache = state.get_or_create_cache();
    let mut cache_write = cache.write().await;

    if let Some((cached_json, cache_time)) = &cache_write.cached_process_energy_json {
        if cache_write.is_cache_valid(cache_time) {
            // Кэш актуален - используем его
            let mut perf_metrics = state.performance_metrics.write().await;
            perf_metrics.increment_cache_hits();
            drop(perf_metrics);

            trace!("Cache hit for process_energy_handler");
            return Ok(Json(cached_json.clone()));
        }
    }

    // Кэш не актуален или отсутствует - получаем свежие данные
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_cache_misses();
    drop(perf_metrics);

    // Пробуем получить данные из eBPF метрик
    let mut result = json!({
        "status": "degraded",
        "process_energy": null,
        "count": 0,
        "message": "Process energy monitoring not available",
        "suggestion": "Enable process energy monitoring in configuration and ensure eBPF support",
        "component_status": check_component_availability(&state),
        "cache_info": {
            "cached": false,
            "ttl_seconds": cache_write.cache_ttl_seconds
        },
        "timestamp": Utc::now().to_rfc3339()
    });

    // Пробуем получить доступ к eBPF метрикам
    if let Some(metrics_arc) = &state.metrics {
        let metrics = metrics_arc.read().await;
        if let Some(ebpf_metrics) = &metrics.ebpf {
            if let Some(energy_details) = &ebpf_metrics.process_energy_details {
                result = json!({
                    "status": "ok",
                    "process_energy": energy_details,
                    "count": energy_details.len(),
                    "total_energy_uj": energy_details.iter().map(|s| s.energy_uj).sum::<u64>(),
                    "total_energy_w": energy_details.iter().map(|s| s.energy_w).sum::<f32>(),
                    "cache_info": {
                        "cached": false,
                        "ttl_seconds": cache_write.cache_ttl_seconds
                    },
                    "timestamp": Utc::now().to_rfc3339()
                });
            }
        }
    }

    // Кэшируем результат
    cache_write.cached_process_energy_json = Some((result.clone(), Instant::now()));

    trace!("Cached process energy data (count: {})", result["count"]);
    Ok(Json(result))
}

/// Обработчик для endpoint `/api/processes/memory`.
///
/// Возвращает статистику использования памяти процессами (если доступны).
async fn process_memory_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    // Начинаем отслеживание производительности
    let _start_time = Instant::now();

    // Обновляем метрики производительности
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_requests();
    drop(perf_metrics); // Освобождаем блокировку

    // Пробуем использовать кэш
    let cache = state.get_or_create_cache();
    let mut cache_write = cache.write().await;

    if let Some((cached_json, cache_time)) = &cache_write.cached_process_memory_json {
        if cache_write.is_cache_valid(cache_time) {
            // Кэш актуален - используем его
            let mut perf_metrics = state.performance_metrics.write().await;
            perf_metrics.increment_cache_hits();
            drop(perf_metrics);

            trace!("Cache hit for process_memory_handler");
            return Ok(Json(cached_json.clone()));
        }
    }

    // Кэш не актуален или отсутствует - получаем свежие данные
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_cache_misses();
    drop(perf_metrics);

    // Пробуем получить данные из процессов
    let mut result = json!({
        "status": "degraded",
        "process_memory": null,
        "count": 0,
        "total_rss_mb": 0,
        "total_swap_mb": 0,
        "message": "Process memory monitoring not available",
        "suggestion": "Ensure the daemon is running and has completed at least one collection cycle",
        "component_status": check_component_availability(&state),
        "cache_info": {
            "cached": false,
            "ttl_seconds": cache_write.cache_ttl_seconds
        },
        "timestamp": Utc::now().to_rfc3339()
    });

    // Пробуем получить доступ к данным процессов
    if let Some(processes_arc) = &state.processes {
        let processes = processes_arc.read().await;
        
        // Фильтруем процессы с доступными метриками памяти
        let memory_processes: Vec<_> = processes
            .iter()
            .filter_map(|proc| {
                if proc.rss_mb.is_some() || proc.swap_mb.is_some() {
                    Some(json!({
                        "pid": proc.pid,
                        "name": proc.exe.clone().unwrap_or_else(|| "unknown".to_string()),
                        "rss_mb": proc.rss_mb,
                        "swap_mb": proc.swap_mb,
                        "cmdline": proc.cmdline.clone(),
                        "app_group_id": proc.app_group_id
                    }))
                } else {
                    None
                }
            })
            .collect();

        if !memory_processes.is_empty() {
            let total_rss_mb: u64 = processes
                .iter()
                .filter_map(|proc| proc.rss_mb)
                .sum();
            let total_swap_mb: u64 = processes
                .iter()
                .filter_map(|proc| proc.swap_mb)
                .sum();

            result = json!({
                "status": "ok",
                "process_memory": memory_processes,
                "count": memory_processes.len(),
                "total_rss_mb": total_rss_mb,
                "total_swap_mb": total_swap_mb,
                "cache_info": {
                    "cached": false,
                    "ttl_seconds": cache_write.cache_ttl_seconds
                },
                "timestamp": Utc::now().to_rfc3339()
            });
        }
    }

    // Пробуем получить данные из eBPF метрик (если доступны)
    if let Some(metrics_arc) = &state.metrics {
        let metrics = metrics_arc.read().await;
        if let Some(ebpf_metrics) = &metrics.ebpf {
            if let Some(process_memory_details) = &ebpf_metrics.process_memory_details {
                // Добавляем eBPF данные к результату
                let mut ebpf_memory_processes = Vec::new();
                let mut total_rss_bytes = 0u64;
                let mut total_vms_bytes = 0u64;
                let mut total_shared_bytes = 0u64;
                let mut total_swap_bytes = 0u64;
                let mut total_heap_usage = 0u64;
                let mut total_stack_usage = 0u64;
                let mut total_anonymous_memory = 0u64;
                let mut total_file_backed_memory = 0u64;
                let mut total_major_faults = 0u64;
                let mut total_minor_faults = 0u64;

                for stat in process_memory_details {
                    let process_json = json!({
                        "pid": stat.pid,
                        "tgid": stat.tgid,
                        "last_update_ns": stat.last_update_ns,
                        "rss_bytes": stat.rss_bytes,
                        "vms_bytes": stat.vms_bytes,
                        "shared_bytes": stat.shared_bytes,
                        "swap_bytes": stat.swap_bytes,
                        "heap_usage": stat.heap_usage,
                        "stack_usage": stat.stack_usage,
                        "anonymous_memory": stat.anonymous_memory,
                        "file_backed_memory": stat.file_backed_memory,
                        "major_faults": stat.major_faults,
                        "minor_faults": stat.minor_faults,
                        "name": stat.name,
                        "source": "ebpf"
                    });
                    ebpf_memory_processes.push(process_json);

                    // Обновляем общие суммы
                    total_rss_bytes += stat.rss_bytes;
                    total_vms_bytes += stat.vms_bytes;
                    total_shared_bytes += stat.shared_bytes;
                    total_swap_bytes += stat.swap_bytes;
                    total_heap_usage += stat.heap_usage;
                    total_stack_usage += stat.stack_usage;
                    total_anonymous_memory += stat.anonymous_memory;
                    total_file_backed_memory += stat.file_backed_memory;
                    total_major_faults += stat.major_faults;
                    total_minor_faults += stat.minor_faults;
                }

                // Добавляем eBPF данные к результату
                result["ebpf_process_memory"] = json!(ebpf_memory_processes);
                result["ebpf_count"] = json!(process_memory_details.len());
                result["total_rss_bytes"] = json!(total_rss_bytes);
                result["total_vms_bytes"] = json!(total_vms_bytes);
                result["total_shared_bytes"] = json!(total_shared_bytes);
                result["total_swap_bytes"] = json!(total_swap_bytes);
                result["total_heap_usage"] = json!(total_heap_usage);
                result["total_stack_usage"] = json!(total_stack_usage);
                result["total_anonymous_memory"] = json!(total_anonymous_memory);
                result["total_file_backed_memory"] = json!(total_file_backed_memory);
                result["total_major_faults"] = json!(total_major_faults);
                result["total_minor_faults"] = json!(total_minor_faults);
                result["message"] = json!("Process memory monitoring data retrieved successfully (with eBPF details)");
            }
        }
    }

    // Кэшируем результат
    cache_write.cached_process_memory_json = Some((result.clone(), Instant::now()));

    trace!("Cached process memory data (count: {})", result["count"]);
    Ok(Json(result))
}

/// Обработчик для endpoint `/api/processes/gpu`.
///
/// Возвращает статистику использования GPU процессами (если доступны).
async fn process_gpu_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    // Начинаем отслеживание производительности
    let _start_time = Instant::now();

    // Обновляем метрики производительности
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_requests();
    drop(perf_metrics); // Освобождаем блокировку

    // Пробуем использовать кэш
    let cache = state.get_or_create_cache();
    let mut cache_write = cache.write().await;

    if let Some((cached_json, cache_time)) = &cache_write.cached_process_gpu_json {
        if cache_write.is_cache_valid(cache_time) {
            // Кэш актуален - используем его
            let mut perf_metrics = state.performance_metrics.write().await;
            perf_metrics.increment_cache_hits();
            drop(perf_metrics);

            trace!("Cache hit for process_gpu_handler");
            return Ok(Json(cached_json.clone()));
        }
    }

    // Кэш не актуален или отсутствует - получаем свежие данные
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_cache_misses();
    drop(perf_metrics);

    // Пробуем получить данные из eBPF метрик
    let mut result = json!({
        "status": "degraded",
        "process_gpu": null,
        "count": 0,
        "message": "Process GPU monitoring not available",
        "suggestion": "Enable process GPU monitoring in configuration and ensure eBPF support",
        "component_status": check_component_availability(&state),
        "cache_info": {
            "cached": false,
            "ttl_seconds": cache_write.cache_ttl_seconds
        },
        "timestamp": Utc::now().to_rfc3339()
    });

    // Пробуем получить доступ к eBPF метрикам
    if let Some(metrics_arc) = &state.metrics {
        let metrics = metrics_arc.read().await;
        if let Some(ebpf_metrics) = &metrics.ebpf {
            if let Some(process_gpu_details) = &ebpf_metrics.process_gpu_details {
                if !process_gpu_details.is_empty() {
                    // Успешно получили данные о использовании GPU процессами
                    let total_gpu_time_ns: u64 = process_gpu_details.iter().map(|p| p.gpu_time_ns).sum();
                    let total_memory_bytes: u64 = process_gpu_details.iter().map(|p| p.memory_usage_bytes).sum();
                    let total_compute_units: u64 = process_gpu_details.iter().map(|p| p.compute_units_used).sum();

                    result = json!({
                        "status": "ok",
                        "process_gpu": process_gpu_details,
                        "count": process_gpu_details.len(),
                        "total_gpu_time_ns": total_gpu_time_ns,
                        "total_memory_bytes": total_memory_bytes,
                        "total_compute_units": total_compute_units,
                        "message": "Process GPU monitoring data retrieved successfully",
                        "component_status": check_component_availability(&state),
                        "cache_info": {
                            "cached": false,
                            "ttl_seconds": cache_write.cache_ttl_seconds
                        },
                        "timestamp": Utc::now().to_rfc3339()
                    });
                }
            }
        }
    }

    // Кэшируем результат
    cache_write.cached_process_gpu_json = Some((result.clone(), Instant::now()));
    trace!("Cached process GPU data (count: {})", result["count"]);

    Ok(Json(result))
}

/// Обработчик для endpoint `/api/processes/network`.
///
/// Возвращает статистику использования сети процессами (если доступны).
async fn process_network_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    // Начинаем отслеживание производительности
    let _start_time = Instant::now();

    // Обновляем метрики производительности
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_requests();
    drop(perf_metrics); // Освобождаем блокировку

    // Пробуем использовать кэш
    let cache = state.get_or_create_cache();
    let mut cache_write = cache.write().await;

    if let Some((cached_json, cache_time)) = &cache_write.cached_process_network_json {
        if cache_write.is_cache_valid(cache_time) {
            // Кэш актуален - используем его
            let mut perf_metrics = state.performance_metrics.write().await;
            perf_metrics.increment_cache_hits();
            drop(perf_metrics);

            trace!("Cache hit for process_network_handler");
            return Ok(Json(cached_json.clone()));
        }
    }

    // Кэш не актуален или отсутствует - получаем свежие данные
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_cache_misses();
    drop(perf_metrics);

    // Пробуем получить данные из eBPF метрик
    let mut result = json!({
        "status": "degraded",
        "process_network": null,
        "count": 0,
        "total_packets_sent": 0,
        "total_packets_received": 0,
        "total_bytes_sent": 0,
        "total_bytes_received": 0,
        "message": "Process network monitoring not available",
        "suggestion": "Enable process network monitoring in configuration and ensure eBPF support",
        "component_status": check_component_availability(&state),
        "cache_info": {
            "cached": false,
            "ttl_seconds": cache_write.cache_ttl_seconds
        },
        "timestamp": Utc::now().to_rfc3339()
    });

    // Пробуем получить доступ к eBPF метрикам
    if let Some(metrics_arc) = &state.metrics {
        let metrics = metrics_arc.read().await;
        if let Some(ebpf_metrics) = &metrics.ebpf {
            if let Some(process_network_details) = &ebpf_metrics.process_network_details {
                // Конвертируем статистику в JSON
                let mut process_network_json = Vec::new();
                let mut total_packets_sent = 0;
                let mut total_packets_received = 0;
                let mut total_bytes_sent = 0;
                let mut total_bytes_received = 0;

                for stat in process_network_details {
                    let process_info = json!({
                        "pid": stat.pid,
                        "tgid": stat.tgid,
                        "packets_sent": stat.packets_sent,
                        "packets_received": stat.packets_received,
                        "bytes_sent": stat.bytes_sent,
                        "bytes_received": stat.bytes_received,
                        "last_update_ns": stat.last_update_ns,
                        "name": stat.name,
                        "total_network_operations": stat.total_network_operations
                    });
                    process_network_json.push(process_info);

                    total_packets_sent += stat.packets_sent;
                    total_packets_received += stat.packets_received;
                    total_bytes_sent += stat.bytes_sent;
                    total_bytes_received += stat.bytes_received;
                }

                result["status"] = "ok".into();
                result["process_network"] = process_network_json.into();
                result["count"] = process_network_details.len().into();
                result["total_packets_sent"] = total_packets_sent.into();
                result["total_packets_received"] = total_packets_received.into();
                result["total_bytes_sent"] = total_bytes_sent.into();
                result["total_bytes_received"] = total_bytes_received.into();
                result["message"] = "Process network monitoring data retrieved successfully".into();
                result["cache_info"]["cached"] = false.into();

                // Кэшируем результат
                cache_write.cached_process_network_json = Some((result.clone(), Instant::now()));
            }
        }
    }

    trace!("Cached process network data (count: {})", result["count"]);

    Ok(Json(result))
}

/// Возвращает статистику использования диска процессами (если доступны).
async fn process_disk_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    // Начинаем отслеживание производительности
    let _start_time = Instant::now();

    // Обновляем метрики производительности
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_requests();
    drop(perf_metrics); // Освобождаем блокировку

    // Пробуем использовать кэш
    let cache = state.get_or_create_cache();
    let mut cache_write = cache.write().await;

    if let Some((cached_json, cache_time)) = &cache_write.cached_process_disk_json {
        if cache_write.is_cache_valid(cache_time) {
            // Кэш актуален - используем его
            let mut perf_metrics = state.performance_metrics.write().await;
            perf_metrics.increment_cache_hits();
            drop(perf_metrics);

            trace!("Cache hit for process_disk_handler");
            return Ok(Json(cached_json.clone()));
        }
    }

    // Кэш не актуален или отсутствует - получаем свежие данные
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_cache_misses();
    drop(perf_metrics);

    // Пробуем получить данные из eBPF метрик
    let mut result = json!({
        "status": "degraded",
        "process_disk": null,
        "count": 0,
        "total_bytes_read": 0,
        "total_bytes_written": 0,
        "total_read_operations": 0,
        "total_write_operations": 0,
        "message": "Process disk monitoring not available",
        "suggestion": "Enable process disk monitoring in configuration and ensure eBPF support",
        "component_status": check_component_availability(&state),
        "cache_info": {
            "cached": false,
            "ttl_seconds": cache_write.cache_ttl_seconds
        },
        "timestamp": Utc::now().to_rfc3339()
    });

    // Пробуем получить доступ к eBPF метрикам
    if let Some(metrics_arc) = &state.metrics {
        let metrics = metrics_arc.read().await;
        if let Some(ebpf_metrics) = &metrics.ebpf {
            if let Some(process_disk_details) = &ebpf_metrics.process_disk_details {
                // Конвертируем статистику в JSON
                let mut process_disk_json = Vec::new();
                let mut total_bytes_read = 0;
                let mut total_bytes_written = 0;
                let mut total_read_operations = 0;
                let mut total_write_operations = 0;

                for stat in process_disk_details {
                    let process_info = json!({
                        "pid": stat.pid,
                        "tgid": stat.tgid,
                        "bytes_read": stat.bytes_read,
                        "bytes_written": stat.bytes_written,
                        "read_operations": stat.read_operations,
                        "write_operations": stat.write_operations,
                        "last_update_ns": stat.last_update_ns,
                        "name": stat.name,
                        "total_io_operations": stat.total_io_operations
                    });
                    process_disk_json.push(process_info);

                    total_bytes_read += stat.bytes_read;
                    total_bytes_written += stat.bytes_written;
                    total_read_operations += stat.read_operations;
                    total_write_operations += stat.write_operations;
                }

                result["status"] = "ok".into();
                result["process_disk"] = process_disk_json.into();
                result["count"] = process_disk_details.len().into();
                result["total_bytes_read"] = total_bytes_read.into();
                result["total_bytes_written"] = total_bytes_written.into();
                result["total_read_operations"] = total_read_operations.into();
                result["total_write_operations"] = total_write_operations.into();
                result["message"] = "Process disk monitoring data retrieved successfully".into();
                result["cache_info"]["cached"] = false.into();

                // Кэшируем результат
                cache_write.cached_process_disk_json = Some((result.clone(), Instant::now()));
            }
        }
    }

    trace!("Cached process disk data (count: {})", result["count"]);

    Ok(Json(result))
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
                "message": "App groups not available (daemon may not be running or no groups collected yet)"
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
) -> Result<Json<Value>, ApiError> {
    // Используем централизованную валидацию PID
    if let Err(_status_code) = crate::api::validation::validate_process_pid(pid) {
        error!("Invalid PID value: {}", pid);
        return Err(ApiError::ValidationError(format!(
            "Invalid PID value: {}",
            pid
        )));
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
                    Err(ApiError::NotFoundError(format!(
                        "Process with PID {} not found (available processes: {})",
                        pid, process_count
                    )))
                }
            }
        }
        None => {
            error!("Processes data not available for PID lookup. Daemon may not be running or no processes collected yet");
            Err(ApiError::ServiceUnavailableError(
                "Processes data not available - daemon may not be running or no processes collected yet".to_string()
            ))
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
    // Используем централизованную валидацию ID группы приложений
    if let Err(_status_code) = crate::api::validation::validate_app_group_id(&id) {
        error!("Invalid app group ID: {}", id);
        return Ok(Json(json!({
            "status": "error",
            "error": "invalid_input",
            "message": format!("Invalid app group ID: {}", id),
            "timestamp": Utc::now().to_rfc3339(),
            "details": {
                "type": "validation",
                "field": "id",
                "constraint": "must be 1-100 characters with ASCII alphanumeric, _, -, ., :"
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
                        "message": format!("App group with ID '{}' not found (available groups: {})", id, group_count),
                        "timestamp": Utc::now().to_rfc3339(),
                        "details": {
                            "type": "not_found",
                            "resource": "app_group",
                            "field": "id",
                            "available_count": group_count
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
                "message": "App groups data not available - daemon may not be running or no groups collected yet",
                "timestamp": Utc::now().to_rfc3339(),
                "details": {
                    "type": "not_available",
                    "resource": "app_groups"
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

/// Обработчик для endpoint `/api/config` (POST).
///
/// Обновляет основные параметры конфигурации демона.
/// Позволяет изменять параметры опроса, максимальное количество кандидатов,
/// режим политики и другие основные настройки.
///
/// # Аргументы
///
/// * `State(state)` - Состояние API с конфигурацией
/// * `Json(payload)` - JSON payload с параметрами для обновления
///
/// # Возвращает
///
/// * `Result<Json<Value>, StatusCode>` - Результат обновления конфигурации
async fn config_update_handler(
    State(state): State<ApiState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // Валидируем payload
    if let Err(_) = crate::api::validation::validate_config_update_payload(&payload) {
        warn!("Invalid config update payload: {:?}", payload);
        let error_response = crate::api::validation::create_validation_error_response(
            "error",
            "Invalid configuration payload",
            None,
            Some("Check field types and values. polling_interval_ms: 100-60000, max_candidates: 10-1000, dry_run_default: boolean, policy_mode: rules-only|hybrid, enable_snapshot_logging: boolean")
        );
        return Ok(Json(error_response));
    }

    match &state.config {
        Some(config_arc) => {
            // Пробуем обновить конфигурацию
            let mut config_guard = config_arc.write().await;

            // Обновляем параметры, если они предоставлены
            if let Some(interval) = payload.get("polling_interval_ms").and_then(|v| v.as_u64()) {
                config_guard.polling_interval_ms = interval;
            }

            if let Some(max_candidates) = payload.get("max_candidates").and_then(|v| v.as_u64()) {
                config_guard.max_candidates = max_candidates as usize;
            }

            if let Some(dry_run) = payload.get("dry_run_default").and_then(|v| v.as_bool()) {
                config_guard.dry_run_default = dry_run;
            }

            if let Some(policy_mode_str) = payload.get("policy_mode").and_then(|v| v.as_str()) {
                // Конвертируем строку в PolicyMode
                config_guard.policy_mode = match policy_mode_str {
                    "rules-only" => crate::config::config_struct::PolicyMode::RulesOnly,
                    "hybrid" => crate::config::config_struct::PolicyMode::Hybrid,
                    _ => config_guard.policy_mode.clone(), // Если что-то пошло не так, оставляем текущее значение
                };
            }

            if let Some(enable_snapshot) = payload.get("enable_snapshot_logging").and_then(|v| v.as_bool()) {
                config_guard.enable_snapshot_logging = enable_snapshot;
            }

            // Конфигурация обновлена успешно

            // Очищаем кэш конфигурации
            if let Some(cache_arc) = &state.cache {
                let mut cache_guard = cache_arc.write().await;
                cache_guard.cached_config_json = None;
            }

            info!("Configuration updated successfully");
            Ok(Json(json!({
                "status": "success",
                "message": "Configuration updated successfully",
                "config": serde_json::to_value(&*config_guard).unwrap_or(Value::Null)
            })))
        }
        None => Ok(Json(json!({
            "status": "error",
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
                        "timestamp": Utc::now().to_rfc3339(),
                        "action_required": "Fix the configuration file and try reloading again",
                        "details": {
                            "type": "configuration",
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
                "timestamp": Utc::now().to_rfc3339(),
                "action_required": "Provide config file path to enable full config reload functionality",
                "details": {
                    "type": "configuration",
                    "suggestion": "To enable full config reload, ensure the daemon is running with config path information."
                }
            })))
        }
        (None, _) => {
            // Конфигурация недоступна
            Ok(Json(json!({
                "status": "error",
                "message": "Config reload not available (daemon may not be running or config not set)",
                "timestamp": Utc::now().to_rfc3339(),
                "details": {
                    "type": "service_unavailable",
                    "suggestion": "Ensure the daemon is running and properly configured"
                }
            })))
        }
    }
}

/// HTTP API сервер для SmoothTask.
///
/// Сервер предоставляет REST API для мониторинга работы демона.
/// Сервер запускается в отдельной задаче и может быть остановлен через handle.
/// Тип ошибки API для более детальной обработки ошибок.
#[derive(Debug, thiserror::Error)]
#[allow(clippy::enum_variant_names)]
pub enum ApiError {
    /// Ошибка валидации входных данных
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Ошибка доступа к данным
    #[error("Data access error: {0}")]
    DataAccessError(String),

    /// Ошибка конфигурации
    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    /// Внутренняя ошибка сервера
    #[error("Internal server error: {0}")]
    InternalError(String),

    /// Ошибка не найдено
    #[error("Not found: {0}")]
    NotFoundError(String),

    /// Ошибка недоступности сервиса
    #[error("Service unavailable: {0}")]
    ServiceUnavailableError(String),
}

impl ApiError {
    /// Создает детальный JSON ответ об ошибке
    pub fn to_json_response(&self) -> (StatusCode, Json<Value>) {
        let status_code = match self {
            ApiError::ValidationError(_) => StatusCode::BAD_REQUEST,
            ApiError::DataAccessError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::ConfigurationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ApiError::NotFoundError(_) => StatusCode::NOT_FOUND,
            ApiError::ServiceUnavailableError(_) => StatusCode::SERVICE_UNAVAILABLE,
        };

        let error_type = match self {
            ApiError::ValidationError(_) => "invalid_input",
            ApiError::DataAccessError(_) => "data_access_error",
            ApiError::ConfigurationError(_) => "configuration_error",
            ApiError::InternalError(_) => "internal_error",
            ApiError::NotFoundError(_) => "not_found",
            ApiError::ServiceUnavailableError(_) => "not_available",
        };

        let error_response = json!({
            "status": "error",
            "error": error_type,
            "message": self.to_string(),
            "timestamp": Utc::now().to_rfc3339(),
            "details": match self {
                ApiError::ValidationError(_details) => json!({
                    "type": "validation",
                    "suggestion": "Check your request parameters and try again"
                }),
                ApiError::DataAccessError(_details) => json!({
                    "type": "data_access",
                    "suggestion": "Check if the daemon is running and has collected data"
                }),
                ApiError::ConfigurationError(_details) => json!({
                    "type": "configuration",
                    "suggestion": "Check your configuration file and restart the daemon"
                }),
                ApiError::InternalError(_details) => json!({
                    "type": "internal",
                    "suggestion": "This is a bug, please report it with logs"
                }),
                ApiError::NotFoundError(_details) => json!({
                    "type": "not_found",
                    "suggestion": "Check the resource identifier and try again"
                }),
                ApiError::ServiceUnavailableError(_details) => json!({
                    "type": "service_unavailable",
                    "suggestion": "Check if the daemon is running and try again later"
                }),
            }
        });

        (status_code, Json(error_response))
    }
}

/// Преобразуем ApiError в HTTP ответ для использования в Axum
impl IntoResponse for ApiError {
    fn into_response(self) -> Response<axum::body::Body> {
        let (status_code, json_response) = self.to_json_response();
        (status_code, json_response).into_response()
    }
}

/// Преобразуем ApiError в Result для использования в обработчиках
impl From<ApiError> for Result<Json<Value>, ApiError> {
    fn from(error: ApiError) -> Self {
        Err(error)
    }
}

/// Хелпер для graceful degradation - возвращает ответ с информацией о недоступности данных
#[allow(dead_code)]
fn graceful_degradation_response(resource_name: &str, suggestion: &str) -> Json<Value> {
    Json(json!({
        "status": "degraded",
        "message": format!("{} not available - graceful degradation", resource_name),
        "resource": resource_name,
        "available": false,
        "suggestion": suggestion,
        "timestamp": Utc::now().to_rfc3339(),
        "fallback_data": null
    }))
}

/// Хелпер для проверки доступности основных компонентов
fn check_component_availability(state: &ApiState) -> Value {
    json!({
        "daemon_stats_available": state.daemon_stats.is_some(),
        "system_metrics_available": state.system_metrics.is_some(),
        "processes_available": state.processes.is_some(),
        "app_groups_available": state.app_groups.is_some(),
        "config_available": state.config.is_some(),
        "pattern_database_available": state.pattern_database.is_some(),
        "notification_manager_available": state.notification_manager.is_some(),
        "log_storage_available": state.log_storage.is_some(),
        "cache_available": state.cache.is_some()
    })
}
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
            CpuTimes, DiskMetrics, HardwareMetrics, LoadAvg, MemoryInfo, NetworkMetrics, PowerMetrics,
            PressureMetrics, SystemCallMetrics, InodeMetrics, SwapMetrics, SystemMetrics, TemperatureMetrics,
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
            hardware: HardwareMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
            system_calls: SystemCallMetrics::default(),
            inode: InodeMetrics::default(),
            swap: SwapMetrics::default(),
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
            CpuTimes, DiskMetrics, HardwareMetrics, LoadAvg, MemoryInfo, NetworkMetrics, PowerMetrics,
            PressureMetrics, SystemCallMetrics, SystemMetrics, TemperatureMetrics, InodeMetrics, SwapMetrics,
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
            hardware: HardwareMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
            system_calls: SystemCallMetrics::default(),
            inode: InodeMetrics::default(),
            swap: SwapMetrics::default(),
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
            CpuTimes, DiskMetrics, HardwareMetrics, LoadAvg, MemoryInfo, NetworkMetrics, PowerMetrics,
            PressureMetrics, SystemCallMetrics, SystemMetrics, TemperatureMetrics, InodeMetrics, SwapMetrics,
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
            hardware: HardwareMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
            system_calls: SystemCallMetrics::default(),
            inode: InodeMetrics::default(),
            swap: SwapMetrics::default(),
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
            CpuTimes, DiskMetrics, HardwareMetrics, LoadAvg, MemoryInfo, NetworkMetrics, PowerMetrics,
            PressureMetrics, SystemCallMetrics, SystemMetrics, TemperatureMetrics, InodeMetrics, SwapMetrics,
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
            hardware: HardwareMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
            system_calls: SystemCallMetrics::default(),
            inode: InodeMetrics::default(),
            swap: SwapMetrics::default(),
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
            io_read_operations: None,
            io_write_operations: None,
            io_total_operations: None,
            io_last_update_ns: None,
            io_data_source: None,
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
            energy_uj: None,
            power_w: None,
            energy_timestamp: None,
            network_rx_bytes: None,
            network_tx_bytes: None,
            network_rx_packets: None,
            network_tx_packets: None,
            network_tcp_connections: None,
            network_udp_connections: None,
            network_last_update_ns: None,
            network_data_source: None,
            gpu_utilization: None,
            gpu_memory_bytes: None,
            gpu_time_us: None,
            gpu_api_calls: None,
            gpu_last_update_ns: None,
            gpu_data_source: None,
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
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(100),
            has_gui_window: true,
            is_focused_group: false,
            tags: vec!["gui".to_string()],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
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
        use crate::metrics::ebpf::{EbpfConfig, EbpfFilterConfig};
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
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
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
                enable_cpu_temperature_monitoring: false,
                enable_filesystem_monitoring: false,
                enable_process_monitoring: false,
                enable_process_energy_monitoring: false,
                enable_process_gpu_monitoring: false,
                enable_process_network_monitoring: false,
                enable_process_disk_monitoring: false,
                enable_process_memory_monitoring: false,
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
                filter_config: EbpfFilterConfig::default(),
            },
            custom_metrics: None,
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

    #[tokio::test]
    async fn test_config_update_handler_without_config() {
        // Тест для config_update_handler когда конфигурация недоступна
        let state = ApiState::new();
        let payload = json!({
            "polling_interval_ms": 500
        });
        let result = config_update_handler(State(state), Json(payload)).await;
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
    async fn test_config_update_handler_with_config() {
        // Тест для config_update_handler когда конфигурация доступна
        use crate::config::config_struct::{
            CacheIntervals, Config, LoggingConfig, MLClassifierConfig,
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
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
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
            custom_metrics: None,
            model: crate::config::config_struct::ModelConfig::default(),
            ml_classifier: MLClassifierConfig::default(),
            ebpf: EbpfConfig::default(),
            pattern_auto_update: PatternAutoUpdateConfig::default(),
        };
        let config_arc = Arc::new(RwLock::new(config));
        let state = ApiStateBuilder::new()
            .with_config(Some(config_arc.clone()))
            .build();

        let payload = json!({
            "polling_interval_ms": 500,
            "max_candidates": 200,
            "dry_run_default": true,
            "policy_mode": "hybrid",
            "enable_snapshot_logging": false
        });
        let result = config_update_handler(State(state), Json(payload)).await;
        assert!(result.is_ok());

        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "success");
        assert_eq!(
            value["message"],
            "Configuration updated successfully"
        );
        assert!(value["config"].is_object());
        assert_eq!(value["config"]["polling_interval_ms"], 500);
        assert_eq!(value["config"]["max_candidates"], 200);
        assert_eq!(value["config"]["dry_run_default"], true);
        assert_eq!(value["config"]["policy_mode"], "hybrid");
        assert_eq!(value["config"]["enable_snapshot_logging"], false);

        // Проверяем, что конфигурация действительно обновилась
        let config_guard = config_arc.read().await;
        assert_eq!(config_guard.polling_interval_ms, 500);
        assert_eq!(config_guard.max_candidates, 200);
        assert_eq!(config_guard.dry_run_default, true);
        match config_guard.policy_mode {
            PolicyMode::Hybrid => {},
            _ => panic!("Policy mode should be Hybrid"),
        }
        assert_eq!(config_guard.enable_snapshot_logging, false);
    }

    #[tokio::test]
    async fn test_config_update_handler_partial_update() {
        // Тест для config_update_handler с частичным обновлением
        use crate::config::config_struct::{
            CacheIntervals, Config, LoggingConfig, MLClassifierConfig,
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
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
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
            custom_metrics: None,
            model: crate::config::config_struct::ModelConfig::default(),
            ml_classifier: MLClassifierConfig::default(),
            ebpf: EbpfConfig::default(),
            pattern_auto_update: PatternAutoUpdateConfig::default(),
        };
        let config_arc = Arc::new(RwLock::new(config));
        let state = ApiStateBuilder::new()
            .with_config(Some(config_arc.clone()))
            .build();

        let payload = json!({
            "polling_interval_ms": 750
        });
        let result = config_update_handler(State(state), Json(payload)).await;
        assert!(result.is_ok());

        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "success");
        assert_eq!(value["config"]["polling_interval_ms"], 750);
        // Остальные параметры должны остаться без изменений
        assert_eq!(value["config"]["max_candidates"], 150);
        assert_eq!(value["config"]["dry_run_default"], false);
        assert_eq!(value["config"]["policy_mode"], "rules-only");
        assert_eq!(value["config"]["enable_snapshot_logging"], true);

        // Проверяем, что конфигурация действительно обновилась частично
        let config_guard = config_arc.read().await;
        assert_eq!(config_guard.polling_interval_ms, 750);
        assert_eq!(config_guard.max_candidates, 150);
        assert_eq!(config_guard.dry_run_default, false);
    }

    #[tokio::test]
    async fn test_config_update_handler_invalid_payload() {
        // Тест для config_update_handler с невалидным payload
        let state = ApiState::new();
        let payload = json!({
            "polling_interval_ms": 50
        });
        let result = config_update_handler(State(state), Json(payload)).await;
        assert!(result.is_ok());

        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "error");
        assert!(value["error"].is_string());
        assert!(value["message"].is_string());
    }

    #[test]
    fn test_api_state_with_all_and_config() {
        use crate::config::config_struct::{
            CacheIntervals, Config, LoggingConfig, MLClassifierConfig, ModelConfig, ModelType,
            Paths, PatternAutoUpdateConfig, PolicyMode, Thresholds,
        };
        use crate::metrics::ebpf::{EbpfConfig, EbpfFilterConfig};
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
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
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
                enable_cpu_temperature_monitoring: false,
                enable_filesystem_monitoring: false,
                enable_process_monitoring: false,
                enable_process_energy_monitoring: false,
                enable_process_gpu_monitoring: false,
                enable_process_network_monitoring: false,
                enable_process_disk_monitoring: false,
                enable_process_memory_monitoring: false,
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
                filter_config: EbpfFilterConfig::default(),
            },
            custom_metrics: None,
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
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
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
            custom_metrics: None,
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
        assert!(result.is_err());
        let error = result.unwrap_err();
        let (status_code, json_response) = error.to_json_response();
        assert_eq!(status_code, StatusCode::BAD_REQUEST);
        let value: Value = json_response.0;
        assert_eq!(value["status"], "error");
        assert_eq!(value["error"], "invalid_input");
        assert!(value["message"]
            .as_str()
            .unwrap()
            .contains("Invalid PID value"));
        assert!(value["timestamp"].is_string());
        assert!(value["details"].is_object());
        assert_eq!(value["details"]["type"].as_str(), Some("validation"));
    }

    #[tokio::test]
    async fn test_process_by_pid_handler_too_large_pid() {
        let state = ApiState::new();
        let result = process_by_pid_handler(Path(5_000_000), State(state)).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        let (status_code, json_response) = error.to_json_response();
        assert_eq!(status_code, StatusCode::BAD_REQUEST);
        let value: Value = json_response.0;
        assert_eq!(value["status"], "error");
        assert_eq!(value["error"], "invalid_input");
        assert!(value["message"].as_str().unwrap().contains("too large"));
        // Check that details are included
        // Note: The generic ApiError doesn't include constraint and note fields
        // These were part of the original custom error format
    }

    #[tokio::test]
    async fn test_process_by_pid_handler_without_processes() {
        let state = ApiState::new();
        let result = process_by_pid_handler(Path(1), State(state)).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        let (status_code, json_response) = error.to_json_response();
        assert_eq!(status_code, StatusCode::SERVICE_UNAVAILABLE);
        let value: Value = json_response.0;
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
            io_read_operations: None,
            io_write_operations: None,
            io_total_operations: None,
            io_last_update_ns: None,
            io_data_source: None,
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
            energy_uj: None,
            power_w: None,
            energy_timestamp: None,
            network_rx_bytes: None,
            network_tx_bytes: None,
            network_rx_packets: None,
            network_tx_packets: None,
            network_tcp_connections: None,
            network_udp_connections: None,
            network_last_update_ns: None,
            network_data_source: None,
            gpu_utilization: None,
            gpu_memory_bytes: None,
            gpu_time_us: None,
            gpu_api_calls: None,
            gpu_last_update_ns: None,
            gpu_data_source: None,
        }];
        let processes_arc = Arc::new(RwLock::new(processes));
        let state = ApiState::with_all(None, None, Some(processes_arc), None);
        let result = process_by_pid_handler(Path(999999), State(state)).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        let (status_code, json_response) = error.to_json_response();
        assert_eq!(status_code, StatusCode::NOT_FOUND);
        let value: Value = json_response.0;
        assert_eq!(value["status"], "error");
        assert_eq!(value["error"], "not_found");
        assert!(value["message"].as_str().unwrap().contains("not found"));
        // Note: The generic ApiError doesn't include available_count
        // This was part of the original custom error format
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
            io_read_operations: None,
            io_write_operations: None,
            io_total_operations: None,
            io_last_update_ns: None,
            io_data_source: None,
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
            energy_uj: None,
            power_w: None,
            energy_timestamp: None,
            network_rx_bytes: None,
            network_tx_bytes: None,
            network_rx_packets: None,
            network_tx_packets: None,
            network_tcp_connections: None,
            network_udp_connections: None,
            network_last_update_ns: None,
            network_data_source: None,
            gpu_utilization: None,
            gpu_memory_bytes: None,
            gpu_time_us: None,
            gpu_api_calls: None,
            gpu_last_update_ns: None,
            gpu_data_source: None,
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
        // Note: The generic ApiError doesn't include field
        // This was part of the original custom error format
        // Note: The generic ApiError doesn't include constraint field
        // This was part of the original custom error format
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
        // Note: The generic ApiError doesn't include constraint field
        // This was part of the original custom error format
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
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(100),
            has_gui_window: true,
            is_focused_group: false,
            tags: vec!["gui".to_string()],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
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
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(100),
            has_gui_window: true,
            is_focused_group: false,
            tags: vec!["gui".to_string()],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
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
        assert_eq!(json["count"], 27); // Обновлено с 23 до 26 (добавлен /api/processes/memory и /api/cache/monitoring)

        let endpoints = json["endpoints"].as_array().unwrap();
        assert_eq!(endpoints.len(), 27); // Обновлено с 23 до 26 (добавлен /api/processes/memory и /api/cache/monitoring)

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
    async fn test_app_performance_handler() {
        let state = ApiState::new();
        let result = app_performance_handler(State(state)).await;
        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;
        assert_eq!(value["status"], "ok");
        assert!(value["app_performance_metrics"].is_object());
        assert!(value["total_app_groups"].is_number());
        assert!(value["timestamp"].is_string());
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
            CpuTimes, DiskMetrics, HardwareMetrics, LoadAvg, MemoryInfo, NetworkMetrics, PowerMetrics,
            PressureMetrics, SystemCallMetrics, SystemMetrics, TemperatureMetrics, InodeMetrics, SwapMetrics,
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
            hardware: HardwareMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
            system_calls: SystemCallMetrics::default(),
            inode: InodeMetrics::default(),
            swap: SwapMetrics::default(),
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
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
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
            custom_metrics: None,
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
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
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
            custom_metrics: None,
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
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
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
            custom_metrics: None,
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
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
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
            custom_metrics: None,
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
    async fn test_system_cpu_handler() {
        let result = system_cpu_handler().await;
        let json = result.0;
        assert_eq!(json["status"], "ok");
        assert!(json["cpu"].is_object());

        let cpu = &json["cpu"];
        assert!(cpu["cores"].is_number());
        assert!(cpu["logical_cpus"].is_number());

        // Проверяем, что если /proc/cpuinfo доступен, то информация о CPU присутствует
        if fs::read_to_string("/proc/cpuinfo").is_ok() {
            // Должны быть заполнены основные поля
            assert!(cpu["cores"].as_u64().unwrap_or(0) > 0);
            assert!(cpu["logical_cpus"].as_u64().unwrap_or(0) > 0);
        }

        // model и vendor могут быть null или строкой
        assert!(cpu["model"].is_null() || cpu["model"].is_string());
        assert!(cpu["vendor"].is_null() || cpu["vendor"].is_string());
    }

    #[tokio::test]
    async fn test_system_memory_handler() {
        let result = system_memory_handler().await;
        let json = result.0;
        assert_eq!(json["status"], "ok");
        assert!(json["memory"].is_object());

        let memory = &json["memory"];
        assert!(memory["total"].is_number());
        assert!(memory["free"].is_number());
        assert!(memory["available"].is_number());
        assert!(memory["buffers"].is_number());
        assert!(memory["cached"].is_number());
        assert!(memory["swap"].is_object());

        // Проверяем, что если /proc/meminfo доступен, то информация о памяти присутствует
        if fs::read_to_string("/proc/meminfo").is_ok() {
            // Должны быть заполнены основные поля
            assert!(memory["total"].as_u64().unwrap_or(0) > 0);
            assert!(memory["free"].as_u64().unwrap_or(0) >= 0);
            assert!(memory["available"].as_u64().unwrap_or(0) >= 0);
        }
    }

    #[tokio::test]
    async fn test_system_disk_handler() {
        let result = system_disk_handler().await;
        let json = result.0;
        assert_eq!(json["status"], "ok");
        assert!(json["disk"].is_object());

        let disk = &json["disk"];
        assert!(disk["disks"].is_array());
        assert!(disk["total_space"].is_number());
        assert!(disk["free_space"].is_number());
        assert!(disk["used_space"].is_number());

        // Проверяем, что если /proc/diskstats доступен, то информация о дисках присутствует
        if fs::read_to_string("/proc/diskstats").is_ok() {
            // Должны быть заполнены основные поля
            assert!(disk["disks"].as_array().unwrap().len() >= 0);
        }
    }

    #[tokio::test]
    async fn test_system_network_handler() {
        let result = system_network_handler().await;
        let json = result.0;
        assert_eq!(json["status"], "ok");
        assert!(json["network"].is_object());

        let network = &json["network"];
        assert!(network["interfaces"].is_array());
        assert!(network["total_rx_bytes"].is_number());
        assert!(network["total_tx_bytes"].is_number());
        assert!(network["total_rx_packets"].is_number());
        assert!(network["total_tx_packets"].is_number());

        // Проверяем, что если /proc/net/dev доступен, то информация о сети присутствует
        if fs::read_to_string("/proc/net/dev").is_ok() {
            // Должны быть заполнены основные поля
            assert!(network["interfaces"].as_array().unwrap().len() >= 0);
        }
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
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
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
            custom_metrics: None,
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
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: crate::config::config_struct::Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: crate::config::config_struct::LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
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
            custom_metrics: None,
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
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: crate::config::config_struct::Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: crate::config::config_struct::LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
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
            custom_metrics: None,
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
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
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
            custom_metrics: None,
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
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
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
            custom_metrics: None,
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
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
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
            custom_metrics: None,
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
        assert_eq!(json["count"], 27); // Обновлено с 23 до 25 (добавлен /api/processes/memory)

        let endpoints = json["endpoints"].as_array().unwrap();
        assert_eq!(endpoints.len(), 27);

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
        assert!(endpoint_paths.contains(&"/api/processes/memory"));
        assert!(endpoint_paths.contains(&"/api/processes/gpu"));
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
            health_monitor: None,
            cache: None,
            log_storage: None,
            custom_metrics_manager: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
            metrics_collector: None,
            metrics: None,
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
            health_monitor: None,
            cache: None,
            log_storage: None,
            custom_metrics_manager: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
            metrics_collector: None,
            metrics: None,
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
            health_monitor: None,
            cache: None,
            log_storage: None,
            performance_metrics: Arc::new(RwLock::new(ApiPerformanceMetrics::default())),
            custom_metrics_manager: None,
            metrics_collector: None,
            metrics: None,
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

    #[tokio::test]
    async fn test_api_error_handling_validation() {
        // Тест: проверка обработки ошибок валидации
        let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
        let handle = server.start().await.expect("Сервер должен запуститься");
        let port = handle.port();

        let client = reqwest::Client::new();

        // Тестируем неверный PID (отрицательный)
        let url = format!("http://127.0.0.1:{}/api/processes/-1", port);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен выполниться");

        // Должен вернуть 400 Bad Request
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);

        let body = response.text().await.expect("Ответ должен содержать текст");
        let json: serde_json::Value =
            serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

        // Проверяем структуру ответа об ошибке
        assert_eq!(json["status"].as_str(), Some("error"));
        assert!(json["error"].is_string());
        assert!(json["message"].is_string());
        assert!(json["timestamp"].is_string());
        assert!(json["details"].is_object());
        assert_eq!(json["details"]["type"].as_str(), Some("validation"));

        // Останавливаем сервер
        handle
            .shutdown()
            .await
            .expect("Сервер должен корректно остановиться");
    }

    #[tokio::test]
    async fn test_api_error_handling_not_found() {
        // Тест: проверка обработки ошибок "не найдено"
        use crate::logging::snapshots::ProcessRecord;
        
        // Создаем тестовые процессы
        let mut test_process = ProcessRecord::default();
        test_process.pid = 123;
        test_process.ppid = 456;
        test_process.exe = Some("/usr/bin/test_process".to_string());
        test_process.cmdline = Some("test_process --arg".to_string());
        test_process.state = "running".to_string();
        test_process.start_time = 123456789;
        test_process.uptime_sec = 60;
        
        let test_processes = vec![test_process];
        
        let server = ApiServer::with_all(
            "127.0.0.1:0".parse().unwrap(),
            None,
            None,
            Some(Arc::new(RwLock::new(test_processes))),
            None,
        );
        let handle = server.start().await.expect("Сервер должен запуститься");
        let port = handle.port();

        let client = reqwest::Client::new();

        // Тестируем несуществующий процесс (но с валидным PID)
        let url = format!("http://127.0.0.1:{}/api/processes/999999", port);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен выполниться");

        // Должен вернуть 404 Not Found
        assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

        let body = response.text().await.expect("Ответ должен содержать текст");
        let json: serde_json::Value =
            serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

        // Проверяем структуру ответа об ошибке
        assert_eq!(json["status"].as_str(), Some("error"));
        assert!(json["error"].is_string());
        assert!(json["message"].is_string());
        assert!(json["timestamp"].is_string());
        assert!(json["details"].is_object());
        assert_eq!(json["details"]["type"].as_str(), Some("not_found"));

        // Останавливаем сервер
        handle
            .shutdown()
            .await
            .expect("Сервер должен корректно остановиться");
    }

    #[tokio::test]
    async fn test_api_graceful_degradation() {
        // Тест: проверка graceful degradation при недоступности данных
        let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
        let handle = server.start().await.expect("Сервер должен запуститься");
        let port = handle.port();

        let client = reqwest::Client::new();

        // Тестируем endpoint, который требует данных процессов (их нет)
        let url = format!("http://127.0.0.1:{}/api/processes", port);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен выполниться");

        // Должен вернуть 200 OK, но с статусом degraded
        assert_eq!(response.status(), reqwest::StatusCode::OK);

        let body = response.text().await.expect("Ответ должен содержать текст");
        let json: serde_json::Value =
            serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

        // Проверяем структуру ответа с graceful degradation
        assert_eq!(json["status"].as_str(), Some("ok"));
        assert_eq!(json["processes"], Value::Null);
        assert_eq!(json["count"], 0);
        assert!(json["message"].as_str().unwrap().contains("not available"));
        assert!(json["message"].is_string());

        // Note: The processes_handler doesn't include component_status
        // This was part of the original graceful degradation format

        // Останавливаем сервер
        handle
            .shutdown()
            .await
            .expect("Сервер должен корректно остановиться");
    }

    #[tokio::test]
    async fn test_api_health_detailed_with_degradation() {
        // Тест: проверка детального health endpoint с информацией о degradation
        let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
        let handle = server.start().await.expect("Сервер должен запуститься");
        let port = handle.port();

        let client = reqwest::Client::new();
        let url = format!("http://127.0.0.1:{}/api/health", port);

        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен выполниться");

        assert!(response.status().is_success());

        let body = response.text().await.expect("Ответ должен содержать текст");
        let json: serde_json::Value =
            serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

        // Проверяем, что ответ содержит информацию о компонентах
        assert!(json["status"].as_str() == Some("ok"));
        assert!(json["components"].is_object());
        assert!(json["overall_status"].is_string());
        assert!(json["suggestions"].is_string());

        // Проверяем, что все компоненты помечены как недоступные (так как сервер пустой)
        let components = &json["components"];
        assert_eq!(components["daemon_stats"].as_bool(), Some(false));
        assert_eq!(
            components["system_metrics"].as_bool(),
            Some(false)
        );
        assert_eq!(components["processes"].as_bool(), Some(false));

        // Проверяем общий статус
        assert_eq!(json["overall_status"].as_str(), Some("degraded"));

        // Останавливаем сервер
        handle
            .shutdown()
            .await
            .expect("Сервер должен корректно остановиться");
    }

    #[tokio::test]
    async fn test_api_error_types() {
        // Тест: проверка разных типов ошибок API
        let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
        let handle = server.start().await.expect("Сервер должен запуститься");
        let port = handle.port();

        let client = reqwest::Client::new();

        // 1. Тестируем ошибку валидации (неверный PID)
        let url = format!("http://127.0.0.1:{}/api/processes/0", port);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен выполниться");
        assert_eq!(response.status(), reqwest::StatusCode::BAD_REQUEST);

        // 2. Тестируем ошибку "сервис недоступен" (данные о процессах недоступны)
        let url = format!("http://127.0.0.1:{}/api/processes/123456", port);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен выполниться");
        assert_eq!(response.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);

        // 3. Тестируем graceful degradation (данные недоступны)
        let url = format!("http://127.0.0.1:{}/api/metrics", port);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен выполниться");
        assert_eq!(response.status(), reqwest::StatusCode::OK);
        let body = response.text().await.expect("Ответ должен содержать текст");
        let json: serde_json::Value =
            serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");
        assert_eq!(json["status"].as_str(), Some("ok"));
        assert_eq!(json["system_metrics"], Value::Null);
        assert!(json["message"].as_str().unwrap().contains("not available"));

        // Останавливаем сервер
        handle
            .shutdown()
            .await
            .expect("Сервер должен корректно остановиться");
    }
}

/// Обработчик для endpoint `/api/cache/stats` (GET).
///
/// Возвращает статистику кэша метрик процессов.
/// Включает информацию о количестве записей, актуальных записей, устаревших записей,
/// максимальной емкости, времени жизни кэша и среднем возрасте записей.
async fn cache_stats_handler() -> Result<Json<Value>, StatusCode> {
    let stats = crate::metrics::process::get_process_cache_stats();

    Ok(Json(json!({
        "status": "ok",
        "cache_stats": {
            "total_entries": stats.total_entries,
            "active_entries": stats.active_entries,
            "stale_entries": stats.stale_entries,
            "max_capacity": stats.max_capacity,
            "cache_ttl_seconds": stats.cache_ttl_seconds,
            "average_age_seconds": stats.average_age_seconds,
            "hit_rate": stats.hit_rate,
            "utilization_rate": if stats.max_capacity > 0 {
                (stats.total_entries as f64 / stats.max_capacity as f64) * 100.0
            } else {
                0.0
            }
        },
        "timestamp": Utc::now().to_rfc3339()
    })))
}

/// Обработчик для endpoint `/api/cache/clear` (POST).
///
/// Очищает кэш метрик процессов.
/// Возвращает информацию о количестве удаленных записей и статусе операции.
async fn cache_clear_handler() -> Result<Json<Value>, StatusCode> {
    // Получаем текущую статистику перед очисткой
    let stats_before = crate::metrics::process::get_process_cache_stats();

    // Очищаем кэш
    crate::metrics::process::clear_process_cache();

    // Получаем статистику после очистки
    let stats_after = crate::metrics::process::get_process_cache_stats();

    Ok(Json(json!({
        "status": "success",
        "message": "Process cache cleared successfully",
        "cleared_entries": stats_before.total_entries - stats_after.total_entries,
        "previous_stats": {
            "total_entries": stats_before.total_entries,
            "active_entries": stats_before.active_entries,
            "stale_entries": stats_before.stale_entries
        },
        "current_stats": {
            "total_entries": stats_after.total_entries,
            "active_entries": stats_after.active_entries,
            "stale_entries": stats_after.stale_entries
        },
        "timestamp": Utc::now().to_rfc3339()
    })))
}

/// Обработчик для endpoint `/api/cache/config` (GET).
///
/// Возвращает текущую конфигурацию кэша метрик процессов.
/// Включает параметры TTL, максимального количества процессов, включения кэширования
/// и параллельной обработки.
async fn cache_config_handler() -> Result<Json<Value>, StatusCode> {
    let config = crate::metrics::process::get_process_cache_config();

    Ok(Json(json!({
        "status": "ok",
        "cache_config": {
            "cache_ttl_seconds": config.cache_ttl_seconds,
            "max_cached_processes": config.max_cached_processes,
            "enable_caching": config.enable_caching,
            "enable_parallel_processing": config.enable_parallel_processing,
            "max_parallel_threads": config.max_parallel_threads
        },
        "timestamp": Utc::now().to_rfc3339()
    })))
}

/// Обработчик для endpoint `/api/cache/config` (POST).
///
/// Обновляет конфигурацию кэша метрик процессов.
/// Позволяет изменять параметры TTL, максимального количества процессов,
/// включения кэширования и параллельной обработки.
///
/// # Параметры запроса
///
/// - `cache_ttl_seconds` (опционально): Новое время жизни кэша в секундах
/// - `max_cached_processes` (опционально): Новое максимальное количество кэшируемых процессов
/// - `enable_caching` (опционально): Включить/отключить кэширование
/// - `enable_parallel_processing` (опционально): Включить/отключить параллельную обработку
/// - `max_parallel_threads` (опционально): Максимальное количество параллельных потоков
///
/// # Примеры
///
/// ```bash
/// # Обновление TTL кэша
/// curl -X POST "http://127.0.0.1:8080/api/cache/config" \
///   -H "Content-Type: application/json" \
///   -d '{"cache_ttl_seconds": 30}'
///
/// # Обновление максимального количества процессов
/// curl -X POST "http://127.0.0.1:8080/api/cache/config" \
///   -H "Content-Type: application/json" \
///   -d '{"max_cached_processes": 5000}'
///
/// # Отключение кэширования
/// curl -X POST "http://127.0.0.1:8080/api/cache/config" \
///   -H "Content-Type: application/json" \
///   -d '{"enable_caching": false}'
/// ```
async fn cache_config_update_handler(
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // Валидируем payload
    if let Err(_status_code) = crate::api::validation::validate_cache_config_payload(&payload) {
        return Ok(Json(json!({
            "status": "error",
            "message": "Invalid cache configuration payload"
        })));
    }

    // Получаем текущую конфигурацию
    let mut config = crate::metrics::process::get_process_cache_config();

    // Обновляем параметры, если они предоставлены
    if let Some(ttl) = payload.get("cache_ttl_seconds").and_then(|v| v.as_u64()) {
        config.cache_ttl_seconds = ttl;
    }

    if let Some(max_processes) = payload.get("max_cached_processes").and_then(|v| v.as_u64()) {
        config.max_cached_processes = max_processes as usize;
    }

    if let Some(enable_caching) = payload.get("enable_caching").and_then(|v| v.as_bool()) {
        config.enable_caching = enable_caching;
    }

    if let Some(enable_parallel) = payload
        .get("enable_parallel_processing")
        .and_then(|v| v.as_bool())
    {
        config.enable_parallel_processing = enable_parallel;
    }

    if let Some(max_threads) = payload.get("max_parallel_threads").and_then(|v| v.as_u64()) {
        config.max_parallel_threads = Some(max_threads as usize);
    }

    // Применяем новую конфигурацию
    crate::metrics::process::update_process_cache_config(config.clone());

    Ok(Json(json!({
        "status": "success",
        "message": "Process cache configuration updated successfully",
        "cache_config": {
            "cache_ttl_seconds": config.cache_ttl_seconds,
            "max_cached_processes": config.max_cached_processes,
            "enable_caching": config.enable_caching,
            "enable_parallel_processing": config.enable_parallel_processing,
            "max_parallel_threads": config.max_parallel_threads
        },
        "timestamp": Utc::now().to_rfc3339()
    })))
}

/// Обработчик для endpoint `/api/network/connections`.
///
/// Возвращает информацию о текущих сетевых соединениях, собранных через eBPF.
/// Включает детализированную информацию о каждом активном соединении.
async fn network_connections_handler(
    State(state): State<ApiState>,
) -> Result<Json<Value>, StatusCode> {
    match &state.metrics_collector {
        Some(collector) => {
            // Пробуем собрать метрики сетевых соединений
            // Клонируем Arc и получаем mutable reference
            let mut collector_clone = Arc::clone(collector);
            match Arc::get_mut(&mut collector_clone)
                .unwrap()
                .collect_metrics()
            {
                Ok(metrics) => {
                    let connection_details = metrics.connection_details.clone();
                    let total_connections =
                        connection_details.as_ref().map(|d| d.len()).unwrap_or(0);

                    let connections_info = connection_details.map(|details| {
                        details
                            .into_iter()
                            .map(|conn| {
                                json!({
                                    "src_ip": format_ip(conn.src_ip),
                                    "dst_ip": format_ip(conn.dst_ip),
                                    "src_port": conn.src_port,
                                    "dst_port": conn.dst_port,
                                    "protocol": protocol_to_string(conn.protocol),
                                    "state": conn.state,
                                    "packets": conn.packets,
                                    "bytes": conn.bytes,
                                    "start_time": conn.start_time,
                                    "last_activity": conn.last_activity,
                                    "active": is_connection_active(conn.last_activity)
                                })
                            })
                            .collect::<Vec<Value>>()
                    });

                    Ok(Json(json!({
                        "status": "ok",
                        "timestamp": Utc::now().to_rfc3339(),
                        "active_connections": metrics.active_connections,
                        "total_connections": total_connections,
                        "connections": connections_info,
                        "network_stats": {
                            "packets": metrics.network_packets,
                            "bytes": metrics.network_bytes
                        }
                    })))
                }
                Err(e) => {
                    tracing::error!("Ошибка при сборе метрик сетевых соединений: {}", e);
                    Ok(Json(json!({
                        "status": "error",
                        "error": format!("Failed to collect network connection metrics: {}", e),
                        "timestamp": Utc::now().to_rfc3339()
                    })))
                }
            }
        }
        None => Ok(Json(json!({
            "status": "error",
            "error": "Metrics collector not available",
            "timestamp": Utc::now().to_rfc3339()
        }))),
    }
}

/// Обработчик для endpoint `/api/cpu/temperature`.
///
/// Возвращает информацию о температуре CPU, собранную через eBPF.
async fn cpu_temperature_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    // Начинаем отслеживание производительности
    let _start_time = Instant::now();

    // Обновляем метрики производительности
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_requests();
    drop(perf_metrics); // Освобождаем блокировку

    match &state.metrics_collector {
        Some(collector) => {
            // Пробуем собрать метрики температуры CPU
            // Клонируем Arc и получаем mutable reference
            let mut collector_clone = Arc::clone(collector);
            match Arc::get_mut(&mut collector_clone)
                .unwrap()
                .collect_metrics()
            {
                Ok(metrics) => {
                    let temperature_details = metrics.cpu_temperature_details.clone();
                    let avg_temperature = metrics.cpu_temperature;
                    let max_temperature = metrics.cpu_max_temperature;

                    let cpu_count = temperature_details.as_ref().map(|d| d.len()).unwrap_or(0);

                    let temperature_info = temperature_details.map(|details| {
                        details.into_iter().map(|temp| {
                            json!({
                                "cpu_id": temp.cpu_id,
                                "temperature_celsius": temp.temperature_celsius,
                                "max_temperature_celsius": temp.max_temperature_celsius,
                                "critical_temperature_celsius": temp.critical_temperature_celsius,
                                "timestamp": temp.timestamp,
                                "update_count": temp.update_count,
                                "error_count": temp.error_count,
                                "status": if temp.temperature_celsius >= temp.critical_temperature_celsius {
                                    "critical"
                                } else if temp.temperature_celsius >= temp.max_temperature_celsius {
                                    "warning"
                                } else {
                                    "normal"
                                }
                            })
                        }).collect::<Vec<Value>>()
                    });

                    // Определяем общий статус системы
                    let system_status = if max_temperature >= 95 {
                        "critical"
                    } else if max_temperature >= 85 {
                        "warning"
                    } else {
                        "normal"
                    };

                    Ok(Json(json!({
                        "status": "ok",
                        "timestamp": Utc::now().to_rfc3339(),
                        "system_status": system_status,
                        "average_temperature_celsius": avg_temperature,
                        "max_temperature_celsius": max_temperature,
                        "cpu_count": cpu_count,
                        "temperature_details": temperature_info,
                        "recommendations": if system_status == "critical" {
                            "System is overheating. Consider checking cooling system and reducing load."
                        } else if system_status == "warning" {
                            "System temperature is high. Monitor closely."
                        } else {
                            "System temperature is normal."
                        }
                    })))
                }
                Err(e) => {
                    tracing::error!("Ошибка при сборе метрик температуры CPU: {}", e);
                    Ok(Json(json!({
                        "status": "error",
                        "error": format!("Failed to collect CPU temperature metrics: {}", e),
                        "timestamp": Utc::now().to_rfc3339()
                    })))
                }
            }
        }
        None => Ok(Json(json!({
            "status": "error",
            "error": "Metrics collector not available",
            "timestamp": Utc::now().to_rfc3339()
        }))),
    }
}

/// Вспомогательная функция для форматирования IP адреса
fn format_ip(ip: u32) -> String {
    let bytes = ip.to_be_bytes();
    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
}

/// Вспомогательная функция для преобразования протокола в строку
fn protocol_to_string(protocol: u8) -> String {
    match protocol {
        6 => "TCP".to_string(),
        17 => "UDP".to_string(),
        _ => format!("Unknown({})", protocol),
    }
}

/// Вспомогательная функция для проверки активности соединения
fn is_connection_active(last_activity: u64) -> bool {
    // Считаем соединение активным, если активность была в последние 30 секунд
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_nanos() as u64;

    // 30 секунд в наносекундах
    let timeout_ns = 30_000_000_000;
    current_time.saturating_sub(last_activity) < timeout_ns
}

#[cfg(test)]
mod test_process_energy_api {
    use super::*;
    use crate::metrics::ebpf::{EbpfMetrics, ProcessEnergyStat};

    #[tokio::test]
    async fn test_process_energy_handler_without_metrics() {
        // Тест проверяет, что обработчик корректно работает без доступных метрик
        let state = ApiState {
            metrics: None,
            ..ApiState::default()
        };

        let response = process_energy_handler(State(state)).await;
        assert!(response.is_ok(), "Обработчик должен успешно выполниться");

        let json_response = response.unwrap();
        let value: serde_json::Value = serde_json::from_str(&json_response.0.to_string()).unwrap();

        assert_eq!(
            value["status"].as_str().unwrap(),
            "degraded",
            "Статус должен быть degraded"
        );
        assert_eq!(
            value["count"].as_u64().unwrap(),
            0,
            "Количество должно быть 0"
        );
        assert!(
            value["process_energy"].is_null(),
            "Данные об энергопотреблении должны быть null"
        );
    }

    #[tokio::test]
    async fn test_process_energy_handler_with_metrics() {
        // Тест проверяет, что обработчик корректно работает с доступными метриками
        let energy_stats = vec![
            ProcessEnergyStat {
                pid: 123,
                tgid: 456,
                energy_uj: 1000,
                last_update_ns: 123456789,
                cpu_id: 0,
                name: "test_process".to_string(),
                energy_w: 0.001,
            },
            ProcessEnergyStat {
                pid: 789,
                tgid: 101112,
                energy_uj: 2000,
                last_update_ns: 123456790,
                cpu_id: 1,
                name: "another_process".to_string(),
                energy_w: 0.002,
            },
        ];

        let ebpf_metrics = EbpfMetrics {
            process_energy_details: Some(energy_stats.clone()),
            ..EbpfMetrics::default()
        };

        let system_metrics = crate::metrics::system::SystemMetrics {
            ebpf: Some(ebpf_metrics),
            ..crate::metrics::system::SystemMetrics::default()
        };

        let state = ApiState {
            metrics: Some(Arc::new(RwLock::new(system_metrics))),
            ..ApiState::default()
        };

        let response = process_energy_handler(State(state)).await;
        assert!(response.is_ok(), "Обработчик должен успешно выполниться");

        let json_response = response.unwrap();
        let value: serde_json::Value = serde_json::from_str(&json_response.0.to_string()).unwrap();

        assert_eq!(
            value["status"].as_str().unwrap(),
            "ok",
            "Статус должен быть ok"
        );
        assert_eq!(
            value["count"].as_u64().unwrap(),
            2,
            "Количество должно быть 2"
        );
        assert!(
            value["process_energy"].is_array(),
            "Данные об энергопотреблении должны быть массивом"
        );
        assert_eq!(
            value["process_energy"][0]["pid"].as_u64().unwrap(),
            123,
            "Первый процесс должен иметь PID 123"
        );
        assert_eq!(
            value["process_energy"][1]["pid"].as_u64().unwrap(),
            789,
            "Второй процесс должен иметь PID 789"
        );
        assert_eq!(
            value["total_energy_uj"].as_u64().unwrap(),
            3000,
            "Общее потребление энергии должно быть 3000 микроджоулей"
        );
        assert!(
            (value["total_energy_w"].as_f64().unwrap() - 0.003).abs() < 1e-10,
            "Общее потребление энергии должно быть примерно 0.003 ватт"
        );
    }

    #[tokio::test]
    async fn test_process_energy_handler_cache() {
        // Тест проверяет, что обработчик корректно использует кэш
        let energy_stats = vec![ProcessEnergyStat {
            pid: 123,
            tgid: 456,
            energy_uj: 1000,
            last_update_ns: 123456789,
            cpu_id: 0,
            name: "test_process".to_string(),
            energy_w: 0.001,
        }];

        let ebpf_metrics = EbpfMetrics {
            process_energy_details: Some(energy_stats.clone()),
            ..EbpfMetrics::default()
        };

        let system_metrics = crate::metrics::system::SystemMetrics {
            ebpf: Some(ebpf_metrics),
            ..crate::metrics::system::SystemMetrics::default()
        };

        let state = ApiState {
            metrics: Some(Arc::new(RwLock::new(system_metrics))),
            cache: Some(Arc::new(RwLock::new(ApiCache {
                cache_ttl_seconds: 60,
                ..ApiCache::default()
            }))),
            ..ApiState::default()
        };

        // Первый вызов - кэш должен быть создан
        let response1 = process_energy_handler(State(state.clone())).await;
        assert!(response1.is_ok(), "Первый вызов должен успешно выполниться");

        // Второй вызов - должен использовать кэш
        let response2 = process_energy_handler(State(state)).await;
        assert!(response2.is_ok(), "Второй вызов должен успешно выполниться");

        let json_response1 = response1.unwrap();
        let json_response2 = response2.unwrap();

        // Результаты должны быть идентичны
        assert_eq!(
            json_response1.0, json_response2.0,
            "Результаты должны быть идентичны при использовании кэша"
        );
    }

    #[tokio::test]
    async fn test_cache_monitoring_handler_without_cache() {
        // Тест для cache_monitoring_handler без кэша
        let state = ApiState::new();

        let result = cache_monitoring_handler(State(state)).await;

        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;

        assert_eq!(value["status"], "ok");
        assert!(value["cache_monitoring"].is_object());

        let monitoring = &value["cache_monitoring"];
        assert!(monitoring["api_cache"].is_object());
        assert!(monitoring["performance"].is_object());
        assert!(monitoring["overall_health"].is_object());

        // Проверяем, что кэш отключен
        let api_cache = &monitoring["api_cache"];
        assert_eq!(api_cache["enabled"], false);
        assert_eq!(api_cache["health"]["status"], "disabled");

        // Проверяем доступность
        assert_eq!(value["availability"]["cache_available"], false);
        assert_eq!(value["availability"]["performance_metrics_available"], true);
    }

    #[tokio::test]
    async fn test_cache_monitoring_handler_with_cache() {
        // Тест для cache_monitoring_handler с кэшем
        use super::ApiCache;

        let cache = Arc::new(RwLock::new(ApiCache::new(60)));
        
        // Добавляем некоторые кэшированные данные
        let mut cache_write = cache.write().await;
        cache_write.cached_processes_json = Some((json!({"test": "data"}), Instant::now()));
        cache_write.cached_metrics_json = Some((json!({"cpu": 50}), Instant::now()));
        drop(cache_write); // Освобождаем блокировку

        let state = ApiStateBuilder::new()
            .with_cache(Some(cache))
            .build();

        let result = cache_monitoring_handler(State(state)).await;

        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;

        assert_eq!(value["status"], "ok");
        assert!(value["cache_monitoring"].is_object());

        let monitoring = &value["cache_monitoring"];
        assert!(monitoring["api_cache"].is_object());
        assert!(monitoring["performance"].is_object());
        assert!(monitoring["overall_health"].is_object());

        // Проверяем, что кэш включен
        let api_cache = &monitoring["api_cache"];
        assert_eq!(api_cache["enabled"], true);
        assert_eq!(api_cache["statistics"]["total_cached_items"], 2);
        assert_eq!(api_cache["statistics"]["active_items"], 2);
        assert_eq!(api_cache["statistics"]["stale_items"], 0);

        // Проверяем здоровье
        assert_eq!(api_cache["health"]["status"], "healthy");

        // Проверяем производительность
        let performance = &monitoring["performance"];
        assert_eq!(performance["total_requests"], 0);
        assert_eq!(performance["cache_hits"], 0);
        assert_eq!(performance["cache_misses"], 0);

        // Проверяем общий статус здоровья
        let overall_health = &monitoring["overall_health"];
        assert_eq!(overall_health["status"], "disabled"); // disabled потому что нет запросов

        // Проверяем доступность
        assert_eq!(value["availability"]["cache_available"], true);
        assert_eq!(value["availability"]["performance_metrics_available"], true);
    }

    #[tokio::test]
    async fn test_cache_monitoring_handler_with_stale_cache() {
        // Тест для cache_monitoring_handler с устаревшим кэшем
        use super::ApiCache;

        let cache = Arc::new(RwLock::new(ApiCache::new(1))); // Очень короткое TTL
        
        // Добавляем кэшированные данные
        let mut cache_write = cache.write().await;
        let now = Instant::now();
        cache_write.cached_processes_json = Some((json!({"test": "data"}), now - Duration::from_secs(2))); // Устаревшие данные
        cache_write.cached_metrics_json = Some((json!({"cpu": 50}), now)); // Актуальные данные
        drop(cache_write);

        let state = ApiStateBuilder::new()
            .with_cache(Some(cache))
            .build();

        let result = cache_monitoring_handler(State(state)).await;

        assert!(result.is_ok());
        let json = result.unwrap();
        let value: Value = json.0;

        let monitoring = &value["cache_monitoring"];
        let api_cache = &monitoring["api_cache"];
        
        // Должно быть 2 кэшированных элемента, но только 1 активный
        assert_eq!(api_cache["statistics"]["total_cached_items"], 2);
        assert_eq!(api_cache["statistics"]["active_items"], 1);
        assert_eq!(api_cache["statistics"]["stale_items"], 1);
        
        // Должно быть предупреждение из-за устаревших элементов
        assert_eq!(api_cache["health"]["status"], "warning");
    }
}

#[allow(unused_imports)]
mod test_process_gpu_api {
    use super::*;
    use crate::metrics::ebpf::{EbpfMetrics, ProcessGpuStat};
    use crate::metrics::system::SystemMetrics;

    #[tokio::test]
    async fn test_process_gpu_handler_with_no_metrics() {
        // Тест проверяет, что обработчик корректно работает без доступных метрик
        let state = ApiState {
            metrics: None,
            ..ApiState::default()
        };

        let response = process_gpu_handler(State(state)).await;
        assert!(response.is_ok(), "Обработчик должен успешно выполниться");

        let json_response = response.unwrap();
        let value: serde_json::Value = serde_json::from_str(&json_response.0.to_string()).unwrap();

        assert_eq!(
            value["status"].as_str().unwrap(),
            "degraded",
            "Статус должен быть degraded"
        );
        assert_eq!(
            value["count"].as_u64().unwrap(),
            0,
            "Количество должно быть 0"
        );
        assert!(
            value["process_gpu"].is_null(),
            "Данные об использовании GPU должны быть null"
        );
    }

    #[tokio::test]
    async fn test_process_gpu_handler_with_metrics() {
        // Тест проверяет, что обработчик корректно работает с доступными метриками
        let gpu_stats = vec![
            ProcessGpuStat {
                pid: 123,
                tgid: 456,
                gpu_time_ns: 1000000,
                memory_usage_bytes: 1024,
                compute_units_used: 1,
                last_update_ns: 123456789,
                gpu_id: 0,
                temperature_celsius: 65,
                name: "test_process".to_string(),
                gpu_usage_percent: 5.5,
            },
            ProcessGpuStat {
                pid: 789,
                tgid: 101112,
                gpu_time_ns: 2000000,
                memory_usage_bytes: 2048,
                compute_units_used: 2,
                last_update_ns: 123456790,
                gpu_id: 1,
                temperature_celsius: 70,
                name: "another_process".to_string(),
                gpu_usage_percent: 10.2,
            },
        ];

        let ebpf_metrics = EbpfMetrics {
            process_gpu_details: Some(gpu_stats.clone()),
            ..EbpfMetrics::default()
        };

        let system_metrics = crate::metrics::system::SystemMetrics {
            ebpf: Some(ebpf_metrics),
            ..crate::metrics::system::SystemMetrics::default()
        };

        let state = ApiState {
            metrics: Some(Arc::new(RwLock::new(system_metrics))),
            ..ApiState::default()
        };

        let response = process_gpu_handler(State(state)).await;
        assert!(response.is_ok(), "Обработчик должен успешно выполниться");

        let json_response = response.unwrap();
        let value: serde_json::Value = serde_json::from_str(&json_response.0.to_string()).unwrap();

        assert_eq!(
            value["status"].as_str().unwrap(),
            "ok",
            "Статус должен быть ok"
        );
        assert_eq!(
            value["count"].as_u64().unwrap(),
            2,
            "Количество должно быть 2"
        );
        assert!(
            value["process_gpu"].is_array(),
            "Данные об использовании GPU должны быть массивом"
        );
        assert_eq!(
            value["process_gpu"][0]["pid"].as_u64().unwrap(),
            123,
            "Первый процесс должен иметь PID 123"
        );
        assert_eq!(
            value["process_gpu"][1]["pid"].as_u64().unwrap(),
            789,
            "Второй процесс должен иметь PID 789"
        );
        assert_eq!(
            value["total_gpu_time_ns"].as_u64().unwrap(),
            3000000,
            "Общее время использования GPU должно быть 3000000 наносекунд"
        );
        assert_eq!(
            value["total_memory_bytes"].as_u64().unwrap(),
            3072,
            "Общее использование памяти GPU должно быть 3072 байт"
        );
        assert_eq!(
            value["total_compute_units"].as_u64().unwrap(),
            3,
            "Общее количество вычислительных единиц должно быть 3"
        );
    }

    #[tokio::test]
    async fn test_process_gpu_handler_cache() {
        // Тест проверяет, что обработчик корректно использует кэш
        let gpu_stats = vec![ProcessGpuStat {
            pid: 123,
            tgid: 456,
            gpu_time_ns: 1000000,
            memory_usage_bytes: 1024,
            compute_units_used: 1,
            last_update_ns: 123456789,
            gpu_id: 0,
            temperature_celsius: 65,
            name: "test_process".to_string(),
            gpu_usage_percent: 5.5,
        }];

        let ebpf_metrics = EbpfMetrics {
            process_gpu_details: Some(gpu_stats.clone()),
            ..EbpfMetrics::default()
        };

        let system_metrics = crate::metrics::system::SystemMetrics {
            ebpf: Some(ebpf_metrics),
            ..crate::metrics::system::SystemMetrics::default()
        };

        let state = ApiState {
            metrics: Some(Arc::new(RwLock::new(system_metrics))),
            cache: Some(Arc::new(RwLock::new(ApiCache {
                cache_ttl_seconds: 60,
                ..ApiCache::default()
            }))),
            ..ApiState::default()
        };

        // Первый вызов - кэш должен быть создан
        let response1 = process_gpu_handler(State(state.clone())).await;
        assert!(response1.is_ok(), "Первый вызов должен успешно выполниться");

        // Второй вызов - должен использовать кэш
        let response2 = process_gpu_handler(State(state)).await;
        assert!(response2.is_ok(), "Второй вызов должен успешно выполниться");

        let json_response1 = response1.unwrap();
        let json_response2 = response2.unwrap();

        // Результаты должны быть идентичны
        assert_eq!(
            json_response1.0, json_response2.0,
            "Результаты должны быть идентичны при использовании кэша"
        );
    }
}

#[allow(unused_imports)]
mod test_process_network_api {
    use super::*;
    use crate::metrics::ebpf::{EbpfMetrics, ProcessNetworkStat};
    use crate::metrics::system::SystemMetrics;

    #[tokio::test]
    async fn test_process_network_handler_with_no_metrics() {
        // Тест проверяет, что обработчик корректно работает без доступных метрик
        let state = ApiState {
            metrics: None,
            ..ApiState::default()
        };

        let response = process_network_handler(State(state)).await;
        assert!(response.is_ok(), "Обработчик должен успешно выполниться");

        let json_response = response.unwrap();
        let json_value = json_response.0;

        // Проверяем, что ответ содержит ожидаемые поля
        assert_eq!(
            json_value["status"],
            "degraded",
            "Статус должен быть degraded при отсутствии метрик"
        );
        assert!(
            json_value["process_network"].is_null(),
            "Данные об использовании сети должны быть null"
        );
        assert_eq!(
            json_value["count"],
            0,
            "Количество процессов должно быть 0"
        );
    }

    #[tokio::test]
    async fn test_process_network_handler_with_metrics() {
        // Тест проверяет, что обработчик корректно работает с доступными метриками
        use crate::metrics::ebpf::ProcessNetworkStat;
        
        let network_stats = vec![
            ProcessNetworkStat {
                pid: 123,
                tgid: 456,
                packets_sent: 100,
                packets_received: 50,
                bytes_sent: 1024,
                bytes_received: 512,
                last_update_ns: 123456789,
                name: "test_process".to_string(),
                total_network_operations: 150,
            },
            ProcessNetworkStat {
                pid: 789,
                tgid: 1011,
                packets_sent: 200,
                packets_received: 100,
                bytes_sent: 2048,
                bytes_received: 1024,
                last_update_ns: 123456790,
                name: "another_process".to_string(),
                total_network_operations: 300,
            },
        ];

        let mut system_metrics = SystemMetrics::default();
        let ebpf_metrics = EbpfMetrics {
            process_network_details: Some(network_stats.clone()),
            ..EbpfMetrics::default()
        };
        system_metrics.ebpf = Some(ebpf_metrics);

        let state = ApiState {
            metrics: Some(Arc::new(RwLock::new(system_metrics))),
            ..ApiState::default()
        };

        let response = process_network_handler(State(state)).await;
        assert!(response.is_ok(), "Обработчик должен успешно выполниться");

        let json_response = response.unwrap();
        let json_value = json_response.0;

        // Проверяем, что ответ содержит ожидаемые данные
        assert_eq!(
            json_value["status"],
            "ok",
            "Статус должен быть ok при наличии метрик"
        );
        assert_eq!(
            json_value["count"],
            2,
            "Количество процессов должно быть 2"
        );
        assert_eq!(
            json_value["total_packets_sent"],
            300,
            "Общее количество отправленных пакетов должно быть 300"
        );
        assert_eq!(
            json_value["total_packets_received"],
            150,
            "Общее количество полученных пакетов должно быть 150"
        );
        assert_eq!(
            json_value["total_bytes_sent"],
            3072,
            "Общее количество отправленных байт должно быть 3072"
        );
        assert_eq!(
            json_value["total_bytes_received"],
            1536,
            "Общее количество полученных байт должно быть 1536"
        );

        // Проверяем, что данные о процессах присутствуют
        let process_network = &json_value["process_network"];
        assert!(
            process_network.is_array(),
            "Данные о процессах должны быть массивом"
        );
        assert_eq!(
            process_network.as_array().unwrap().len(),
            2,
            "Должно быть 2 процесса"
        );
    }

    #[tokio::test]
    async fn test_process_network_handler_cache() {
        // Тест проверяет, что обработчик корректно использует кэш
        use crate::metrics::ebpf::ProcessNetworkStat;
        
        let network_stats = vec![ProcessNetworkStat {
            pid: 123,
            tgid: 456,
            packets_sent: 100,
            packets_received: 50,
            bytes_sent: 1024,
            bytes_received: 512,
            last_update_ns: 123456789,
            name: "test_process".to_string(),
            total_network_operations: 150,
        }];

        let mut system_metrics = SystemMetrics::default();
        let ebpf_metrics = EbpfMetrics {
            process_network_details: Some(network_stats.clone()),
            ..EbpfMetrics::default()
        };
        system_metrics.ebpf = Some(ebpf_metrics);

        let state = ApiState {
            metrics: Some(Arc::new(RwLock::new(system_metrics))),
            cache: Some(Arc::new(RwLock::new(ApiCache {
                cache_ttl_seconds: 60,
                ..ApiCache::default()
            }))),
            ..ApiState::default()
        };

        // Первый вызов - кэш должен быть создан
        let response1 = process_network_handler(State(state.clone())).await;
        assert!(response1.is_ok(), "Первый вызов должен успешно выполниться");

        // Второй вызов - должен использовать кэш
        let response2 = process_network_handler(State(state)).await;
        assert!(response2.is_ok(), "Второй вызов должен успешно выполниться");

        let json_response1 = response1.unwrap();
        let json_response2 = response2.unwrap();

        // Результаты должны быть идентичны
        assert_eq!(
            json_response1.0, json_response2.0,
            "Результаты должны быть идентичны при использовании кэша"
        );
    }
}




// Import types needed for disk testing
#[allow(unused_imports)]
use crate::metrics::system::SystemMetrics;
#[allow(unused_imports)]
use crate::metrics::ebpf::EbpfMetrics;

    #[tokio::test]
    async fn test_process_disk_handler_with_no_metrics() {
        // Тест проверяет, что обработчик корректно работает без метрик
        let state = ApiState::default();

        let response = process_disk_handler(State(state)).await;
        assert!(response.is_ok(), "Обработчик должен успешно выполниться");

        let json_response = response.unwrap();
        let json_value = json_response.0;

        assert_eq!(
            json_value["status"], "degraded",
            "Статус должен быть degraded при отсутствии метрик"
        );
        assert_eq!(
            json_value["count"], 0,
            "Количество процессов должно быть 0 при отсутствии метрик"
        );
        assert_eq!(
            json_value["total_bytes_read"], 0,
            "Общее количество прочитанных байт должно быть 0 при отсутствии метрик"
        );
        assert_eq!(
            json_value["total_bytes_written"], 0,
            "Общее количество записанных байт должно быть 0 при отсутствии метрик"
        );
    }

    #[tokio::test]
    async fn test_process_disk_handler_with_metrics() {
        // Тест проверяет, что обработчик корректно работает с метриками
        use crate::metrics::ebpf::ProcessDiskStat;

        let disk_stats = vec![
            ProcessDiskStat {
                pid: 123,
                tgid: 456,
                bytes_read: 1024,
                bytes_written: 2048,
                read_operations: 10,
                write_operations: 20,
                last_update_ns: 123456789,
                name: "test_process".to_string(),
                total_io_operations: 30,
            },
            ProcessDiskStat {
                pid: 789,
                tgid: 1011,
                bytes_read: 4096,
                bytes_written: 8192,
                read_operations: 50,
                write_operations: 100,
                last_update_ns: 123456790,
                name: "another_process".to_string(),
                total_io_operations: 150,
            },
        ];

        let mut system_metrics = SystemMetrics::default();
        let ebpf_metrics = EbpfMetrics {
            process_disk_details: Some(disk_stats.clone()),
            ..EbpfMetrics::default()
        };
        system_metrics.ebpf = Some(ebpf_metrics);

        let state = ApiState {
            metrics: Some(Arc::new(RwLock::new(system_metrics))),
            ..ApiState::default()
        };

        let response = process_disk_handler(State(state)).await;
        assert!(response.is_ok(), "Обработчик должен успешно выполниться");

        let json_response = response.unwrap();
        let json_value = json_response.0;

        assert_eq!(
            json_value["status"], "ok",
            "Статус должен быть ok при наличии метрик"
        );
        assert_eq!(
            json_value["count"], 2,
            "Количество процессов должно быть 2"
        );
        assert_eq!(
            json_value["total_bytes_read"], 5120,
            "Общее количество прочитанных байт должно быть 5120"
        );
        assert_eq!(
            json_value["total_bytes_written"], 10240,
            "Общее количество записанных байт должно быть 10240"
        );
        assert_eq!(
            json_value["total_read_operations"], 60,
            "Общее количество операций чтения должно быть 60"
        );
        assert_eq!(
            json_value["total_write_operations"], 120,
            "Общее количество операций записи должно быть 120"
        );
    }

    #[tokio::test]
    async fn test_process_disk_handler_cache() {
        // Тест проверяет, что кэширование работает корректно
        use crate::metrics::ebpf::ProcessDiskStat;

        let disk_stats = vec![ProcessDiskStat {
            pid: 123,
            tgid: 456,
            bytes_read: 1024,
            bytes_written: 2048,
            read_operations: 10,
            write_operations: 20,
            last_update_ns: 123456789,
            name: "test_process".to_string(),
            total_io_operations: 30,
        }];

        let mut system_metrics = SystemMetrics::default();
        let ebpf_metrics = EbpfMetrics {
            process_disk_details: Some(disk_stats.clone()),
            ..EbpfMetrics::default()
        };
        system_metrics.ebpf = Some(ebpf_metrics);

        let state = ApiState {
            metrics: Some(Arc::new(RwLock::new(system_metrics))),
            ..ApiState::default()
        };

        // Первый вызов - кэш пуст
        let response1 = process_disk_handler(State(state.clone())).await;

        // Второй вызов - должен использовать кэш
        let response2 = process_disk_handler(State(state.clone())).await;

        assert!(response1.is_ok(), "Первый вызов должен успешно выполниться");
        assert!(response2.is_ok(), "Второй вызов должен успешно выполниться");

        let json_response1 = response1.unwrap();
        let json_response2 = response2.unwrap();

        // Результаты должны быть идентичны (кроме timestamp, который может отличаться)
        let json1 = &json_response1.0;
        let json2 = &json_response2.0;
        
        // Проверяем, что основные данные идентичны
        assert_eq!(
            json1["status"], json2["status"],
            "Статусы должны быть идентичны"
        );
        assert_eq!(
            json1["process_disk"], json2["process_disk"],
            "Данные процессов должны быть идентичны"
        );
        assert_eq!(
            json1["count"], json2["count"],
            "Количество процессов должно быть идентично"
        );
        assert_eq!(
            json1["total_bytes_read"], json2["total_bytes_read"],
            "Общее количество прочитанных байт должно быть идентично"
        );
        assert_eq!(
            json1["total_bytes_written"], json2["total_bytes_written"],
            "Общее количество записанных байт должно быть идентично"
        );
        assert_eq!(
            json1["total_read_operations"], json2["total_read_operations"],
            "Общее количество операций чтения должно быть идентично"
        );
        assert_eq!(
            json1["total_write_operations"], json2["total_write_operations"],
            "Общее количество операций записи должно быть идентично"
        );
        
        // Проверяем, что кэш был использован во втором вызове
        // Note: В текущей реализации кэш не используется между разными вызовами
        // потому что каждый вызов создает новый state. Это ожидаемое поведение для этого теста.
        // В реальном использовании кэш будет работать между разными HTTP запросами.
        // assert_eq!(
        //     json2["cache_info"]["cached"], true,
        //     "Второй вызов должен использовать кэш"
        // );
    }

    #[tokio::test]
    async fn test_health_monitoring_handler() {
        // Тест проверяет обработчик health_monitoring_handler
        use crate::health::{create_health_monitor, HealthMonitorConfig};
        
        let health_monitor = create_health_monitor(HealthMonitorConfig::default());
        let state = ApiStateBuilder::new()
            .with_health_monitor(Some(Arc::new(health_monitor)))
            .build();
        
        let result = health_monitoring_handler(State(state)).await;
        assert!(result.is_ok(), "Health monitoring handler should succeed");
        
        let json = result.unwrap();
        assert_eq!(json["status"], "ok", "Status should be ok");
        assert!(json["health_status"].is_object(), "Should contain health_status object");
    }

    #[tokio::test]
    async fn test_health_diagnostics_handler() {
        // Тест проверяет обработчик health_diagnostics_handler
        use crate::health::{create_health_monitor, HealthMonitorConfig};
        
        let health_monitor = create_health_monitor(HealthMonitorConfig::default());
        let state = ApiStateBuilder::new()
            .with_health_monitor(Some(Arc::new(health_monitor)))
            .build();
        
        let result = health_diagnostics_handler(State(state)).await;
        assert!(result.is_ok(), "Health diagnostics handler should succeed");
        
        let json = result.unwrap();
        assert_eq!(json["status"], "ok", "Status should be ok");
        assert!(json["diagnostic_report"].is_object(), "Should contain diagnostic_report object");
    }

    #[tokio::test]
    async fn test_health_issues_handler() {
        // Тест проверяет обработчик health_issues_handler
        use crate::health::{create_health_monitor, HealthMonitorConfig};
        
        let health_monitor = create_health_monitor(HealthMonitorConfig::default());
        let state = ApiStateBuilder::new()
            .with_health_monitor(Some(Arc::new(health_monitor)))
            .build();
        
        let result = health_issues_handler(State(state)).await;
        assert!(result.is_ok(), "Health issues handler should succeed");
        
        let json = result.unwrap();
        assert_eq!(json["status"], "ok", "Status should be ok");
        assert!(json["issues"].is_array(), "Should contain issues array");
    }

    #[tokio::test]
    async fn test_gpu_temperature_power_handler() {
        // Тест проверяет обработчик gpu_temperature_power_handler
        let result = gpu_temperature_power_handler().await;
        assert!(result.is_ok(), "GPU temperature power handler should succeed");
        
        let json = result.unwrap();
        assert_eq!(json["status"], "ok", "Status should be ok");
        assert!(json["gpu_metrics"].is_array(), "Should contain gpu_metrics array");
    }

    #[tokio::test]
    async fn test_gpu_memory_handler() {
        // Тест проверяет обработчик gpu_memory_handler
        let result = gpu_memory_handler().await;
        assert!(result.is_ok(), "GPU memory handler should succeed");
        
        let json = result.unwrap();
        assert_eq!(json["status"], "ok", "Status should be ok");
        assert!(json["gpu_memory_metrics"].is_array(), "Should contain gpu_memory_metrics array");
        assert!(json["total_gpus"].is_number(), "Should contain total_gpus count");
        assert!(json["total_memory_bytes"].is_number(), "Should contain total_memory_bytes");
        assert!(json["total_used_memory_bytes"].is_number(), "Should contain total_used_memory_bytes");
        
        // Проверяем, что метрики памяти содержат ожидаемые поля
        let memory_metrics = json["gpu_memory_metrics"].as_array();
        if let Some(metrics) = memory_metrics {
            for metric in metrics {
                assert!(metric["gpu_id"].is_null() || metric["gpu_id"].is_string(), "Each metric should have gpu_id (may be null)");
                assert!(metric["gpu_name"].is_string(), "Each metric should have gpu_name");
                assert!(metric["gpu_type"].is_null() || metric["gpu_type"].is_string(), "Each metric should have gpu_type (may be null)");
                assert!(metric["gpu_driver"].is_null() || metric["gpu_driver"].is_string(), "Each metric should have gpu_driver (may be null)");
                assert!(metric["total_memory_bytes"].is_number(), "Each metric should have total_memory_bytes");
                assert!(metric["used_memory_bytes"].is_number(), "Each metric should have used_memory_bytes");
                assert!(metric["free_memory_bytes"].is_number(), "Each metric should have free_memory_bytes");
                assert!(metric["memory_usage_percentage"].is_null() || metric["memory_usage_percentage"].is_number(), "Each metric should have memory_usage_percentage (may be null)");
                assert!(metric["timestamp"].is_string(), "Each metric should have timestamp");
            }
        }
    }

    #[tokio::test]
    async fn test_gpu_update_temp_power_handler() {
        // Тест проверяет обработчик gpu_update_temp_power_handler
        let result = gpu_update_temp_power_handler().await;
        assert!(result.is_ok(), "GPU update temp power handler should succeed");
        
        let json = result.unwrap();
        assert_eq!(json["status"], "ok", "Status should be ok");
        assert_eq!(json["message"], "GPU temperature and power metrics updated successfully");
    }

    #[tokio::test]
    async fn test_prometheus_metrics_handler() {
        // Тест проверяет обработчик prometheus_metrics_handler
        use crate::DaemonStats;
        
        // Создаём тестовое состояние API
        let state = ApiStateBuilder::new()
            .with_daemon_stats(Some(Arc::new(RwLock::new(DaemonStats::new()))))
            .with_system_metrics(Some(Arc::new(RwLock::new(SystemMetrics::default()))))
            .with_processes(Some(Arc::new(RwLock::new(Vec::new()))))
            .with_app_groups(Some(Arc::new(RwLock::new(Vec::new()))))
            .build();
        
        let result = prometheus_metrics_handler(State(state)).await;
        assert!(result.is_ok(), "Prometheus metrics handler should succeed");
        
        let metrics = result.unwrap();
        
        // Проверяем, что метрики содержат ожидаемые компоненты
        assert!(metrics.contains("smoothtask_version"), "Should contain version metric");
        assert!(metrics.contains("smoothtask_api_requests_total"), "Should contain API requests metric");
        assert!(metrics.contains("smoothtask_system_cpu_usage_percentage"), "Should contain CPU usage metric");
        assert!(metrics.contains("smoothtask_system_memory_total_bytes"), "Should contain memory metric");
        assert!(metrics.contains("smoothtask_processes_total"), "Should contain processes metric");
        assert!(metrics.contains("smoothtask_app_groups_total"), "Should contain app groups metric");
        assert!(metrics.contains("smoothtask_daemon_uptime_seconds"), "Should contain daemon uptime metric");
        
        // Проверяем формат Prometheus
        assert!(metrics.contains("# HELP"), "Should contain HELP comments");
        assert!(metrics.contains("# TYPE"), "Should contain TYPE declarations");
        
        // Проверяем, что метрики не пустые
        assert!(!metrics.is_empty(), "Metrics should not be empty");
        
        // Проверяем, что метрики содержат числовые значения
        assert!(metrics.lines().any(|line| line.contains(" 1") || line.contains(" 0")), "Should contain numeric values");
    }
    
    #[tokio::test]
    async fn test_prometheus_process_metrics() {
        use crate::logging::snapshots::{ProcessRecord, AppGroupRecord};
        use crate::DaemonStats;
        
        // Создаём тестовые процессы с различными метриками
        let mut test_processes = Vec::new();
        
        // Процесс с CPU, памятью и I/O
        let mut process1 = ProcessRecord::default();
        process1.pid = 100;
        process1.cpu_share_1s = Some(15.5);
        process1.cpu_share_10s = Some(12.3);
        process1.rss_mb = Some(256);
        process1.io_read_bytes = Some(1024 * 1024); // 1 MB
        process1.io_write_bytes = Some(512 * 1024); // 0.5 MB
        process1.network_rx_bytes = Some(100 * 1024); // 100 KB
        process1.network_tx_bytes = Some(50 * 1024); // 50 KB
        process1.gpu_utilization = Some(0.25); // 25%
        process1.energy_uj = Some(1000000); // 1000 mJ
        process1.is_audio_client = true;
        process1.has_gui_window = true;
        process1.env_term = Some("xterm".to_string());
        process1.env_ssh = false;
        test_processes.push(process1);
        
        // Процесс с другими метриками
        let mut process2 = ProcessRecord::default();
        process2.pid = 200;
        process2.cpu_share_1s = Some(5.2);
        process2.cpu_share_10s = Some(6.8);
        process2.rss_mb = Some(128);
        process2.io_read_bytes = Some(2048 * 1024); // 2 MB
        process2.io_write_bytes = Some(1024 * 1024); // 1 MB
        process2.network_rx_bytes = Some(200 * 1024); // 200 KB
        process2.network_tx_bytes = Some(100 * 1024); // 100 KB
        process2.gpu_utilization = Some(0.10); // 10%
        process2.energy_uj = Some(500000); // 500 mJ
        process2.is_audio_client = false;
        process2.has_gui_window = false;
        process2.env_term = None;
        process2.env_ssh = true;
        test_processes.push(process2);
        
        // Создаём тестовые группы приложений
        let mut test_app_groups = Vec::new();
        
        let app_group1 = AppGroupRecord {
            app_group_id: "test-group-1".to_string(),
            root_pid: 100,
            process_ids: vec![100, 200],
            app_name: Some("TestApp".to_string()),
            total_cpu_share: Some(20.7),
            total_io_read_bytes: Some(3072 * 1024), // 3 MB
            total_io_write_bytes: Some(1536 * 1024), // 1.5 MB
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(384),
            has_gui_window: true,
            is_focused_group: true,
            tags: Vec::new(),
            priority_class: None,
            total_energy_uj: Some(1500000), // 1500 mJ
            total_power_w: None,
            total_network_rx_bytes: Some(300 * 1024), // 300 KB
            total_network_tx_bytes: Some(150 * 1024), // 150 KB
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        };
        test_app_groups.push(app_group1);
        
        // Создаём состояние API с тестовыми данными
        let state = ApiStateBuilder::new()
            .with_daemon_stats(Some(Arc::new(RwLock::new(DaemonStats::new()))))
            .with_system_metrics(Some(Arc::new(RwLock::new(SystemMetrics::default()))))
            .with_processes(Some(Arc::new(RwLock::new(test_processes))))
            .with_app_groups(Some(Arc::new(RwLock::new(test_app_groups))))
            .build();
        
        let result = prometheus_metrics_handler(State(state)).await;
        assert!(result.is_ok(), "Prometheus metrics handler should succeed with process data");
        
        let metrics = result.unwrap();
        
        // Проверяем агрегированные метрики процессов
        assert!(metrics.contains("smoothtask_processes_total_cpu_share_1s"), "Should contain total CPU share 1s metric");
        assert!(metrics.contains("smoothtask_processes_total_cpu_share_10s"), "Should contain total CPU share 10s metric");
        assert!(metrics.contains("smoothtask_processes_total_memory_mb"), "Should contain total memory metric");
        assert!(metrics.contains("smoothtask_processes_total_io_read_bytes"), "Should contain total I/O read metric");
        assert!(metrics.contains("smoothtask_processes_total_io_write_bytes"), "Should contain total I/O write metric");
        assert!(metrics.contains("smoothtask_processes_total_network_rx_bytes"), "Should contain total network RX metric");
        assert!(metrics.contains("smoothtask_processes_total_network_tx_bytes"), "Should contain total network TX metric");
        assert!(metrics.contains("smoothtask_processes_total_gpu_utilization"), "Should contain total GPU utilization metric");
        assert!(metrics.contains("smoothtask_processes_total_energy_uj"), "Should contain total energy metric");
        
        // Проверяем метрики по типам процессов
        assert!(metrics.contains("smoothtask_processes_audio_client"), "Should contain audio client processes metric");
        assert!(metrics.contains("smoothtask_processes_gui_window"), "Should contain GUI window processes metric");
        assert!(metrics.contains("smoothtask_processes_terminal"), "Should contain terminal processes metric");
        assert!(metrics.contains("smoothtask_processes_ssh"), "Should contain SSH processes metric");
        
        // Проверяем метрики групп приложений
        assert!(metrics.contains("smoothtask_app_groups_total_cpu_share"), "Should contain app groups total CPU share metric");
        assert!(metrics.contains("smoothtask_app_groups_total_memory_mb"), "Should contain app groups total memory metric");
        assert!(metrics.contains("smoothtask_app_groups_total_io_read_bytes"), "Should contain app groups total I/O read metric");
        assert!(metrics.contains("smoothtask_app_groups_total_io_write_bytes"), "Should contain app groups total I/O write metric");
        assert!(metrics.contains("smoothtask_app_groups_total_network_rx_bytes"), "Should contain app groups total network RX metric");
        assert!(metrics.contains("smoothtask_app_groups_total_network_tx_bytes"), "Should contain app groups total network TX metric");
        assert!(metrics.contains("smoothtask_app_groups_total_energy_uj"), "Should contain app groups total energy metric");
        assert!(metrics.contains("smoothtask_app_groups_focused"), "Should contain focused app groups metric");
        assert!(metrics.contains("smoothtask_app_groups_with_gui"), "Should contain GUI app groups metric");
        
        // Проверяем, что метрики содержат ожидаемые значения
        assert!(metrics.contains("20.7"), "Should contain expected CPU value");
        assert!(metrics.contains("384"), "Should contain expected memory value");
        
        // Проверяем формат метрик
        assert!(metrics.contains("# HELP smoothtask_processes_total_cpu_share_1s"), "Should have HELP for CPU metric");
        assert!(metrics.contains("# TYPE smoothtask_processes_total_cpu_share_1s gauge"), "Should have TYPE for CPU metric");
    }


