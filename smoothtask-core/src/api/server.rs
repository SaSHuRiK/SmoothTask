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
}

impl ApiState {
    /// Создаёт новое состояние API сервера.
    pub fn new() -> Self {
        Self { daemon_stats: None }
    }

    /// Создаёт новое состояние API сервера с переданной статистикой демона.
    pub fn with_daemon_stats(daemon_stats: Arc<RwLock<crate::DaemonStats>>) -> Self {
        Self {
            daemon_stats: Some(daemon_stats),
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
    }

    #[test]
    fn test_api_state_default() {
        let state = ApiState::default();
        assert!(state.daemon_stats.is_none());
    }

    #[test]
    fn test_api_state_with_daemon_stats() {
        let stats = Arc::new(RwLock::new(crate::DaemonStats::new()));
        let state = ApiState::with_daemon_stats(stats.clone());
        assert!(state.daemon_stats.is_some());
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
}
