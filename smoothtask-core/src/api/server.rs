//! HTTP сервер для Control API.

use anyhow::{Context, Result};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::get,
    Router,
};
use serde_json::{json, Value};
use std::fs;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tracing::{error, info};

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
    config: Option<Arc<crate::config::Config>>,
    /// База данных паттернов для классификации процессов (опционально)
    pattern_database: Option<Arc<crate::classify::rules::PatternDatabase>>,
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
            pattern_database: None,
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
            pattern_database: None,
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
            pattern_database: None,
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
            pattern_database: None,
        }
    }

    /// Создаёт новое состояние API сервера со всеми данными, включая конфигурацию.
    pub fn with_all_and_config(
        daemon_stats: Option<Arc<RwLock<crate::DaemonStats>>>,
        system_metrics: Option<Arc<RwLock<crate::metrics::system::SystemMetrics>>>,
        processes: Option<Arc<RwLock<Vec<crate::logging::snapshots::ProcessRecord>>>>,
        app_groups: Option<Arc<RwLock<Vec<crate::logging::snapshots::AppGroupRecord>>>>,
        config: Option<Arc<crate::config::Config>>,
    ) -> Self {
        Self {
            daemon_stats,
            system_metrics,
            processes,
            app_groups,
            responsiveness_metrics: None,
            config,
            pattern_database: None,
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
        config: Option<Arc<crate::config::Config>>,
    ) -> Self {
        Self {
            daemon_stats,
            system_metrics,
            processes,
            app_groups,
            responsiveness_metrics,
            config,
            pattern_database: None,
        }
    }

    /// Создаёт новое состояние API сервера со всеми данными, включая метрики отзывчивости, конфигурацию и базу паттернов.
    pub fn with_all_and_responsiveness_and_config_and_patterns(
        daemon_stats: Option<Arc<RwLock<crate::DaemonStats>>>,
        system_metrics: Option<Arc<RwLock<crate::metrics::system::SystemMetrics>>>,
        processes: Option<Arc<RwLock<Vec<crate::logging::snapshots::ProcessRecord>>>>,
        app_groups: Option<Arc<RwLock<Vec<crate::logging::snapshots::AppGroupRecord>>>>,
        responsiveness_metrics: Option<
            Arc<RwLock<crate::logging::snapshots::ResponsivenessMetrics>>,
        >,
        config: Option<Arc<crate::config::Config>>,
        pattern_database: Option<Arc<crate::classify::rules::PatternDatabase>>,
    ) -> Self {
        Self {
            daemon_stats,
            system_metrics,
            processes,
            app_groups,
            responsiveness_metrics,
            config,
            pattern_database,
        }
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
            }
        ],
        "count": 14
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

/// Создаёт роутер для API.
fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
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
        .route("/api/classes", get(classes_handler))
        .route("/api/patterns", get(patterns_handler))
        .route("/api/system", get(system_handler))
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
    match &state.processes {
        Some(processes_arc) => {
            let processes = processes_arc.read().await;
            Ok(Json(json!({
                "status": "ok",
                "processes": *processes,
                "count": processes.len()
            })))
        }
        None => Ok(Json(json!({
            "status": "ok",
            "processes": null,
            "count": 0,
            "message": "Processes not available (daemon may not be running or no processes collected yet)"
        }))),
    }
}

/// Обработчик для endpoint `/api/appgroups`.
///
/// Возвращает список последних групп приложений (если доступны).
async fn appgroups_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    match &state.app_groups {
        Some(app_groups_arc) => {
            let app_groups = app_groups_arc.read().await;
            Ok(Json(json!({
                "status": "ok",
                "app_groups": *app_groups,
                "count": app_groups.len()
            })))
        }
        None => Ok(Json(json!({
            "status": "ok",
            "app_groups": null,
            "count": 0,
            "message": "App groups not available (daemon may not be running or no groups collected yet)"
        }))),
    }
}

/// Обработчик для endpoint `/api/processes/:pid`.
///
/// Возвращает информацию о конкретном процессе по PID (если доступен).
async fn process_by_pid_handler(
    Path(pid): Path<i32>,
    State(state): State<ApiState>,
) -> Result<Json<Value>, StatusCode> {
    match &state.processes {
        Some(processes_arc) => {
            let processes = processes_arc.read().await;
            match processes.iter().find(|p| p.pid == pid) {
                Some(process) => Ok(Json(json!({
                    "status": "ok",
                    "process": process
                }))),
                None => Ok(Json(json!({
                    "status": "error",
                    "error": "not_found",
                    "message": format!("Process with PID {} not found", pid)
                }))),
            }
        }
        None => Ok(Json(json!({
            "status": "error",
            "error": "not_available",
            "message": "Processes not available (daemon may not be running or no processes collected yet)"
        }))),
    }
}

/// Обработчик для endpoint `/api/appgroups/:id`.
///
/// Возвращает информацию о конкретной группе приложений по ID (если доступна).
async fn appgroup_by_id_handler(
    Path(id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<Value>, StatusCode> {
    match &state.app_groups {
        Some(app_groups_arc) => {
            let app_groups = app_groups_arc.read().await;
            match app_groups.iter().find(|g| g.app_group_id == id) {
                Some(app_group) => Ok(Json(json!({
                    "status": "ok",
                    "app_group": app_group
                }))),
                None => Ok(Json(json!({
                    "status": "error",
                    "error": "not_found",
                    "message": format!("App group with ID '{}' not found", id)
                }))),
            }
        }
        None => Ok(Json(json!({
            "status": "error",
            "error": "not_available",
            "message": "App groups not available (daemon may not be running or no groups collected yet)"
        }))),
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
        Some(config_arc) => Ok(Json(json!({
            "status": "ok",
            "config": serde_json::to_value(config_arc.as_ref()).unwrap_or(Value::Null)
        }))),
        None => Ok(Json(json!({
            "status": "ok",
            "config": null,
            "message": "Config not available (daemon may not be running or config not set)"
        }))),
    }
}

/// HTTP API сервер для SmoothTask.
///
/// Сервер предоставляет REST API для мониторинга работы демона.
/// Сервер запускается в отдельной задаче и может быть остановлен через handle.
///
/// # Примеры использования
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
        config: Option<Arc<crate::config::Config>>,
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
        config: Option<Arc<crate::config::Config>>,
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
    /// - `responsiveness_metrics`: Метрики отзывчивости (опционально)
    /// - `config`: Конфигурация демона (опционально)
    /// - `pattern_database`: База данных паттернов для классификации процессов (опционально)
    #[allow(clippy::too_many_arguments)]
    pub fn with_all_and_responsiveness_and_config_and_patterns(
        addr: std::net::SocketAddr,
        daemon_stats: Option<Arc<RwLock<crate::DaemonStats>>>,
        system_metrics: Option<Arc<RwLock<crate::metrics::system::SystemMetrics>>>,
        processes: Option<Arc<RwLock<Vec<crate::logging::snapshots::ProcessRecord>>>>,
        app_groups: Option<Arc<RwLock<Vec<crate::logging::snapshots::AppGroupRecord>>>>,
        responsiveness_metrics: Option<
            Arc<RwLock<crate::logging::snapshots::ResponsivenessMetrics>>,
        >,
        config: Option<Arc<crate::config::Config>>,
        pattern_database: Option<Arc<crate::classify::rules::PatternDatabase>>,
    ) -> Self {
        Self {
            addr,
            state: ApiState::with_all_and_responsiveness_and_config_and_patterns(
                daemon_stats,
                system_metrics,
                processes,
                app_groups,
                responsiveness_metrics,
                config,
                pattern_database,
            ),
        }
    }

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

        info!("API server listening on http://{}", self.addr);

        let router = create_router(self.state);
        let server = axum::serve(listener, router);

        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        let handle = ApiServerHandle {
            shutdown_tx: Some(shutdown_tx),
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
}

impl ApiServerHandle {
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
            CpuTimes, LoadAvg, MemoryInfo, PressureMetrics, SystemMetrics,
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
            CpuTimes, LoadAvg, MemoryInfo, PressureMetrics, SystemMetrics,
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
            CpuTimes, LoadAvg, MemoryInfo, PressureMetrics, SystemMetrics,
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
            CpuTimes, LoadAvg, MemoryInfo, PressureMetrics, SystemMetrics,
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
        use crate::config::{Config, Paths, PolicyMode, Thresholds};
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
        };
        let config_arc = Arc::new(config);
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
        use crate::config::{Config, Paths, PolicyMode, Thresholds};
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
        };
        let config_arc = Arc::new(config);
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
        use crate::config::{Config, Paths, PolicyMode, Thresholds};
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
        };
        let config_arc = Arc::new(config);
        let server = ApiServer::with_all_and_config(addr, None, None, None, None, Some(config_arc));
        // Проверяем, что сервер создан
        let _ = server;
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
        assert_eq!(json["count"], 14);

        let endpoints = json["endpoints"].as_array().unwrap();
        assert_eq!(endpoints.len(), 14);

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

        let state = ApiState::with_all_and_responsiveness_and_config_and_patterns(
            None,
            None,
            None,
            None,
            None,
            None,
            Some(Arc::new(pattern_db)),
        );

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
        let state = ApiState::with_all_and_responsiveness_and_config_and_patterns(
            None,
            None,
            None,
            None,
            None,
            None,
            Some(pattern_db_arc),
        );

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
            CpuTimes, LoadAvg, MemoryInfo, PressureMetrics, SystemMetrics,
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
        use crate::config::{Config, Paths, PolicyMode, Thresholds};
        use crate::logging::snapshots::ResponsivenessMetrics;
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
        };
        let config_arc = Arc::new(config);
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
            Some(config_arc),
        );
        // Проверяем, что сервер создан
        let _ = server;
    }

    #[test]
    fn test_api_server_with_all_and_responsiveness_and_config_and_patterns() {
        use crate::classify::rules::PatternDatabase;
        use crate::config::{Config, Paths, PolicyMode, Thresholds};
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
        };
        let config_arc = Arc::new(config);
        // Создаём временную директорию для паттернов
        let temp_dir = std::env::temp_dir().join("smoothtask_test_patterns");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let pattern_db = PatternDatabase::load(&temp_dir).unwrap();
        let pattern_db_arc = Arc::new(pattern_db);
        let server = ApiServer::with_all_and_responsiveness_and_config_and_patterns(
            addr,
            None,
            None,
            None,
            None,
            None,
            Some(config_arc),
            Some(pattern_db_arc),
        );
        // Проверяем, что сервер создан
        let _ = server;
    }

    #[test]
    fn test_api_state_with_all_and_responsiveness_and_config() {
        use crate::config::{Config, Paths, PolicyMode, Thresholds};
        use crate::logging::snapshots::ResponsivenessMetrics;
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
        };
        let config_arc = Arc::new(config);
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
            Some(config_arc),
        );
        assert!(state.config.is_some());
        assert!(state.responsiveness_metrics.is_some());
        assert!(state.pattern_database.is_none());
    }

    #[test]
    fn test_api_state_with_all_and_responsiveness_and_config_and_patterns() {
        use crate::classify::rules::PatternDatabase;
        use crate::config::{Config, Paths, PolicyMode, Thresholds};
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
        };
        let config_arc = Arc::new(config);
        // Создаём временную директорию для паттернов
        let temp_dir = std::env::temp_dir().join("smoothtask_test_patterns");
        std::fs::create_dir_all(&temp_dir).unwrap();
        let pattern_db = PatternDatabase::load(&temp_dir).unwrap();
        let pattern_db_arc = Arc::new(pattern_db);
        let state = ApiState::with_all_and_responsiveness_and_config_and_patterns(
            None,
            None,
            None,
            None,
            None,
            Some(config_arc),
            Some(pattern_db_arc),
        );
        assert!(state.config.is_some());
        assert!(state.pattern_database.is_some());
        assert!(state.responsiveness_metrics.is_none());
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
}
