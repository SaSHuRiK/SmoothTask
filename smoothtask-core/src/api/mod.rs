//! HTTP API для просмотра состояния демона SmoothTask.
//!
//! Модуль предоставляет REST API для мониторинга работы демона,
//! просмотра метрик, процессов и AppGroup.

mod server;
mod custom_metrics_handlers;

pub use server::{ApiServer, ApiServerHandle, ApiStateBuilder};
pub use custom_metrics_handlers::*;
