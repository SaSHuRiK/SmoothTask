//! HTTP API для просмотра состояния демона SmoothTask.
//!
//! Модуль предоставляет REST API для мониторинга работы демона,
//! просмотра метрик, процессов и AppGroup.

mod server;

pub use server::{ApiServer, ApiServerHandle};
