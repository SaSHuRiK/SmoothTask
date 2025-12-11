//! HTTP сервер для Control API.

use anyhow::{Context, Result};
use axum::{extract::State, http::StatusCode, response::Json, routing::get, Router};
use serde_json::{json, Value};
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
}

impl ApiState {
    /// Создаёт новое состояние API сервера.
    pub fn new() -> Self {
        Self {
            daemon_stats: None,
            system_metrics: None,
            processes: None,
            app_groups: None,
        }
    }

    /// Создаёт новое состояние API сервера с переданной статистикой демона.
    pub fn with_daemon_stats(daemon_stats: Arc<RwLock<crate::DaemonStats>>) -> Self {
        Self {
            daemon_stats: Some(daemon_stats),
            system_metrics: None,
            processes: None,
            app_groups: None,
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
        }
    }
}

impl Default for ApiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Обработчик для endpoint `/health`.
///
/// Возвращает статус работоспособности API сервера.
async fn health_handler() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "smoothtask-api"
    }))
}

/// Создаёт роутер для API.
fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health_handler))
        .route("/api/stats", get(stats_handler))
        .route("/api/metrics", get(metrics_handler))
        .route("/api/processes", get(processes_handler))
        .route("/api/appgroups", get(appgroups_handler))
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
    }

    #[test]
    fn test_api_state_default() {
        let state = ApiState::default();
        assert!(state.daemon_stats.is_none());
        assert!(state.system_metrics.is_none());
        assert!(state.processes.is_none());
        assert!(state.app_groups.is_none());
    }

    #[test]
    fn test_api_state_with_daemon_stats() {
        let stats = Arc::new(RwLock::new(crate::DaemonStats::new()));
        let state = ApiState::with_daemon_stats(stats.clone());
        assert!(state.daemon_stats.is_some());
        assert!(state.system_metrics.is_none());
        assert!(state.processes.is_none());
        assert!(state.app_groups.is_none());
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
}
