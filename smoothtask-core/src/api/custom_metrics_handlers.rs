//! Обработчики API для пользовательских метрик.
//!
//! Этот модуль предоставляет обработчики для работы с пользовательскими метриками
//! через REST API.

use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde_json::{json, Value};
use std::time::Instant;
use chrono::Utc;

use crate::api::server::ApiState;

/// Обработчик для endpoint `/api/custom-metrics`.
///
/// Возвращает все пользовательские метрики и их текущие значения.
pub async fn custom_metrics_handler(State(state): State<ApiState>) -> Result<Json<Value>, StatusCode> {
    // Начинаем отслеживание производительности
    let _start_time = Instant::now();

    // Обновляем метрики производительности
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_requests();
    drop(perf_metrics); // Освобождаем блокировку

    match &state.custom_metrics_manager {
        Some(manager) => {
            let configs = manager.get_all_metrics_config().map_err(|e| {
                tracing::error!("Failed to get custom metrics configs: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            let values = manager.get_all_metrics_values().map_err(|e| {
                tracing::error!("Failed to get custom metrics values: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            // Преобразуем значения в JSON-совместимый формат
            let metrics_json: Vec<Value> = configs.into_iter().map(|(id, config)| {
                let value_json = values.get(&id).map(|value| {
                    json!({
                        "value": match &value.value {
                            crate::metrics::custom::CustomMetricValue::Integer(v) => json!(v),
                            crate::metrics::custom::CustomMetricValue::Float(v) => json!(v),
                            crate::metrics::custom::CustomMetricValue::Boolean(v) => json!(v),
                            crate::metrics::custom::CustomMetricValue::String(v) => json!(v),
                        },
                        "timestamp": value.timestamp,
                        "status": match &value.status {
                            crate::metrics::custom::MetricStatus::Ok => "ok",
                            crate::metrics::custom::MetricStatus::Error { message } => json!({
                                "error": "error",
                                "message": message
                            }),
                            crate::metrics::custom::MetricStatus::Disabled => "disabled",
                        }
                    })
                });

                json!({
                    "id": id,
                    "name": config.name,
                    "description": config.description,
                    "metric_type": match config.metric_type {
                        crate::metrics::custom::CustomMetricType::Integer => "integer",
                        crate::metrics::custom::CustomMetricType::Float => "float",
                        crate::metrics::custom::CustomMetricType::Boolean => "boolean",
                        crate::metrics::custom::CustomMetricType::String => "string",
                    },
                    "source": match config.source {
                        crate::metrics::custom::CustomMetricSource::File { path, format } => json!({
                            "type": "file",
                            "path": path,
                            "format": match format {
                                crate::metrics::custom::FileMetricFormat::PlainNumber => "plain_number",
                                crate::metrics::custom::FileMetricFormat::Json { path: json_path } => json!({
                                    "format": "json",
                                    "path": json_path
                                }),
                                crate::metrics::custom::FileMetricFormat::Regex { pattern } => json!({
                                    "format": "regex",
                                    "pattern": pattern
                                }),
                            }
                        }),
                        crate::metrics::custom::CustomMetricSource::Command { command, args, format } => json!({
                            "type": "command",
                            "command": command,
                            "args": args,
                            "format": match format {
                                crate::metrics::custom::CommandMetricFormat::PlainNumber => "plain_number",
                                crate::metrics::custom::CommandMetricFormat::Json { path } => json!({
                                    "format": "json",
                                    "path": path
                                }),
                                crate::metrics::custom::CommandMetricFormat::Regex { pattern } => json!({
                                    "format": "regex",
                                    "pattern": pattern
                                }),
                            }
                        }),
                        crate::metrics::custom::CustomMetricSource::Http { url, method, headers, body, json_path } => json!({
                            "type": "http",
                            "url": url,
                            "method": method,
                            "headers": headers,
                            "body": body,
                            "json_path": json_path
                        }),
                        crate::metrics::custom::CustomMetricSource::Static { value } => json!({
                            "type": "static",
                            "value": match value {
                                crate::metrics::custom::CustomMetricValue::Integer(v) => json!(v),
                                crate::metrics::custom::CustomMetricValue::Float(v) => json!(v),
                                crate::metrics::custom::CustomMetricValue::Boolean(v) => json!(v),
                                crate::metrics::custom::CustomMetricValue::String(v) => json!(v),
                            }
                        }),
                    },
                    "update_interval_sec": config.update_interval_sec,
                    "enabled": config.enabled,
                    "current_value": value_json
                })
            }).collect();

            Ok(Json(json!({
                "status": "ok",
                "custom_metrics": metrics_json,
                "count": metrics_json.len(),
                "timestamp": Utc::now().to_rfc3339()
            })))
        }
        None => Ok(Json(json!({
            "status": "ok",
            "custom_metrics": [],
            "count": 0,
            "message": "Custom metrics manager not available (daemon may not be running or feature not enabled)"
        })))
    }
}

/// Обработчик для endpoint `/api/custom-metrics/:metric_id`.
///
/// Возвращает конфигурацию и текущее значение конкретной пользовательской метрики.
pub async fn custom_metric_by_id_handler(
    State(state): State<ApiState>,
    Path(metric_id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    // Начинаем отслеживание производительности
    let _start_time = Instant::now();

    // Обновляем метрики производительности
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_requests();
    drop(perf_metrics); // Освобождаем блокировку

    // Проверяем валидность metric_id
    if metric_id.is_empty() || metric_id.len() > 100 {
        return Ok(Json(json!({
            "status": "error",
            "message": "Invalid metric_id: must be 1-100 characters long"
        })));
    }

    match &state.custom_metrics_manager {
        Some(manager) => {
            let config = manager.get_metric_config(&metric_id).map_err(|e| {
                tracing::error!("Failed to get custom metric config for '{}': {}", metric_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            match config {
                Some(config) => {
                    let value = manager.get_metric_value(&metric_id).map_err(|e| {
                        tracing::error!("Failed to get custom metric value for '{}': {}", metric_id, e);
                        StatusCode::INTERNAL_SERVER_ERROR
                    })?;

                    let value_json = value.map(|value| {
                        json!({
                            "value": match value.value {
                                crate::metrics::custom::CustomMetricValue::Integer(v) => json!(v),
                                crate::metrics::custom::CustomMetricValue::Float(v) => json!(v),
                                crate::metrics::custom::CustomMetricValue::Boolean(v) => json!(v),
                                crate::metrics::custom::CustomMetricValue::String(v) => json!(v),
                            },
                            "timestamp": value.timestamp,
                            "status": match value.status {
                                crate::metrics::custom::MetricStatus::Ok => "ok",
                                crate::metrics::custom::MetricStatus::Error { message } => json!({
                                    "error": "error",
                                    "message": message
                                }),
                                crate::metrics::custom::MetricStatus::Disabled => "disabled",
                            }
                        })
                    });

                    Ok(Json(json!({
                        "status": "ok",
                        "metric": {
                            "id": config.id,
                            "name": config.name,
                            "description": config.description,
                            "metric_type": match config.metric_type {
                                crate::metrics::custom::CustomMetricType::Integer => "integer",
                                crate::metrics::custom::CustomMetricType::Float => "float",
                                crate::metrics::custom::CustomMetricType::Boolean => "boolean",
                                crate::metrics::custom::CustomMetricType::String => "string",
                            },
                            "source": match config.source {
                                crate::metrics::custom::CustomMetricSource::File { path, format } => json!({
                                    "type": "file",
                                    "path": path,
                                    "format": match format {
                                        crate::metrics::custom::FileMetricFormat::PlainNumber => "plain_number",
                                        crate::metrics::custom::FileMetricFormat::Json { path: json_path } => json!({
                                            "format": "json",
                                            "path": json_path
                                        }),
                                        crate::metrics::custom::FileMetricFormat::Regex { pattern } => json!({
                                            "format": "regex",
                                            "pattern": pattern
                                        }),
                                    }
                                }),
                                crate::metrics::custom::CustomMetricSource::Command { command, args, format } => json!({
                                    "type": "command",
                                    "command": command,
                                    "args": args,
                                    "format": match format {
                                        crate::metrics::custom::CommandMetricFormat::PlainNumber => "plain_number",
                                        crate::metrics::custom::CommandMetricFormat::Json { path } => json!({
                                            "format": "json",
                                            "path": path
                                        }),
                                        crate::metrics::custom::CommandMetricFormat::Regex { pattern } => json!({
                                            "format": "regex",
                                            "pattern": pattern
                                        }),
                                    }
                                }),
                                crate::metrics::custom::CustomMetricSource::Http { url, method, headers, body, json_path } => json!({
                                    "type": "http",
                                    "url": url,
                                    "method": method,
                                    "headers": headers,
                                    "body": body,
                                    "json_path": json_path
                                }),
                                crate::metrics::custom::CustomMetricSource::Static { value } => json!({
                                    "type": "static",
                                    "value": match value {
                                        crate::metrics::custom::CustomMetricValue::Integer(v) => json!(v),
                                        crate::metrics::custom::CustomMetricValue::Float(v) => json!(v),
                                        crate::metrics::custom::CustomMetricValue::Boolean(v) => json!(v),
                                        crate::metrics::custom::CustomMetricValue::String(v) => json!(v),
                                    }
                                }),
                            },
                            "update_interval_sec": config.update_interval_sec,
                            "enabled": config.enabled,
                            "current_value": value_json
                        },
                        "timestamp": Utc::now().to_rfc3339()
                    })))
                }
                None => Ok(Json(json!({
                    "status": "error",
                    "message": format!("Custom metric '{}' not found", metric_id)
                })))
            }
        }
        None => Ok(Json(json!({
            "status": "error",
            "message": "Custom metrics manager not available (daemon may not be running or feature not enabled)"
        })))
    }
}

/// Обработчик для endpoint `/api/custom-metrics/:metric_id/update` (POST).
///
/// Принудительно обновляет значение конкретной пользовательской метрики.
pub async fn custom_metric_update_handler(
    State(state): State<ApiState>,
    Path(metric_id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    // Начинаем отслеживание производительности
    let _start_time = Instant::now();

    // Обновляем метрики производительности
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_requests();
    drop(perf_metrics); // Освобождаем блокировку

    // Проверяем валидность metric_id
    if metric_id.is_empty() || metric_id.len() > 100 {
        return Ok(Json(json!({
            "status": "error",
            "message": "Invalid metric_id: must be 1-100 characters long"
        })));
    }

    match &state.custom_metrics_manager {
        Some(manager) => {
            // Проверяем, что метрика существует
            let config = manager.get_metric_config(&metric_id).map_err(|e| {
                tracing::error!("Failed to get custom metric config for '{}': {}", metric_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            match config {
                Some(_) => {
                    // Обновляем метрику
                    let result = manager.update_metric_value(&metric_id).await;

                    match result {
                        Ok(_) => {
                            // Получаем обновленное значение
                            let value = manager.get_metric_value(&metric_id).map_err(|e| {
                                tracing::error!("Failed to get updated custom metric value for '{}': {}", metric_id, e);
                                StatusCode::INTERNAL_SERVER_ERROR
                            })?;

                            let value_json = value.map(|value| {
                                json!({
                                    "value": match value.value {
                                        crate::metrics::custom::CustomMetricValue::Integer(v) => json!(v),
                                        crate::metrics::custom::CustomMetricValue::Float(v) => json!(v),
                                        crate::metrics::custom::CustomMetricValue::Boolean(v) => json!(v),
                                        crate::metrics::custom::CustomMetricValue::String(v) => json!(v),
                                    },
                                    "timestamp": value.timestamp,
                                    "status": match value.status {
                                        crate::metrics::custom::MetricStatus::Ok => "ok",
                                        crate::metrics::custom::MetricStatus::Error { message } => json!({
                                            "error": "error",
                                            "message": message
                                        }),
                                        crate::metrics::custom::MetricStatus::Disabled => "disabled",
                                    }
                                })
                            });

                            Ok(Json(json!({
                                "status": "ok",
                                "message": "Custom metric updated successfully",
                                "metric_id": metric_id,
                                "updated_value": value_json,
                                "timestamp": Utc::now().to_rfc3339()
                            })))
                        }
                        Err(e) => Ok(Json(json!({
                            "status": "error",
                            "message": format!("Failed to update custom metric '{}': {}", metric_id, e)
                        })))
                    }
                }
                None => Ok(Json(json!({
                    "status": "error",
                    "message": format!("Custom metric '{}' not found", metric_id)
                })))
            }
        }
        None => Ok(Json(json!({
            "status": "error",
            "message": "Custom metrics manager not available (daemon may not be running or feature not enabled)"
        })))
    }
}

/// Обработчик для endpoint `/api/custom-metrics/:metric_id/add` (POST).
///
/// Добавляет новую пользовательскую метрику.
pub async fn custom_metric_add_handler(
    State(state): State<ApiState>,
    Path(metric_id): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    // Начинаем отслеживание производительности
    let _start_time = Instant::now();

    // Обновляем метрики производительности
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_requests();
    drop(perf_metrics); // Освобождаем блокировку

    // Проверяем валидность metric_id
    if metric_id.is_empty() || metric_id.len() > 100 {
        return Ok(Json(json!({
            "status": "error",
            "message": "Invalid metric_id: must be 1-100 characters long"
        })));
    }

    match &state.custom_metrics_manager {
        Some(manager) => {
            // Пробуем разобрать конфигурацию метрики из JSON
            let config_result: Result<crate::metrics::custom::CustomMetricConfig, _> = 
                serde_json::from_value(payload);

            match config_result {
                Ok(mut config) => {
                    // Убедимся, что ID метрики соответствует указанному в пути
                    config.id = metric_id.clone();

                    // Добавляем метрику
                    let result = manager.add_metric(config);

                    match result {
                        Ok(_) => Ok(Json(json!({
                            "status": "ok",
                            "message": "Custom metric added successfully",
                            "metric_id": metric_id,
                            "timestamp": Utc::now().to_rfc3339()
                        }))),
                        Err(e) => Ok(Json(json!({
                            "status": "error",
                            "message": format!("Failed to add custom metric '{}': {}", metric_id, e)
                        })))
                    }
                }
                Err(e) => Ok(Json(json!({
                    "status": "error",
                    "message": format!("Invalid custom metric configuration: {}", e)
                })))
            }
        }
        None => Ok(Json(json!({
            "status": "error",
            "message": "Custom metrics manager not available (daemon may not be running or feature not enabled)"
        })))
    }
}

/// Обработчик для endpoint `/api/custom-metrics/:metric_id/remove` (POST).
///
/// Удаляет пользовательскую метрику.
pub async fn custom_metric_remove_handler(
    State(state): State<ApiState>,
    Path(metric_id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    // Начинаем отслеживание производительности
    let _start_time = Instant::now();

    // Обновляем метрики производительности
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_requests();
    drop(perf_metrics); // Освобождаем блокировку

    // Проверяем валидность metric_id
    if metric_id.is_empty() || metric_id.len() > 100 {
        return Ok(Json(json!({
            "status": "error",
            "message": "Invalid metric_id: must be 1-100 characters long"
        })));
    }

    match &state.custom_metrics_manager {
        Some(manager) => {
            // Удаляем метрику
            let result = manager.remove_metric(&metric_id);

            match result {
                Ok(_) => Ok(Json(json!({
                    "status": "ok",
                    "message": "Custom metric removed successfully",
                    "metric_id": metric_id,
                    "timestamp": Utc::now().to_rfc3339()
                }))),
                Err(e) => Ok(Json(json!({
                    "status": "error",
                    "message": format!("Failed to remove custom metric '{}': {}", metric_id, e)
                })))
            }
        }
        None => Ok(Json(json!({
            "status": "error",
            "message": "Custom metrics manager not available (daemon may not be running or feature not enabled)"
        })))
    }
}

/// Обработчик для endpoint `/api/custom-metrics/:metric_id/enable` (POST).
///
/// Включает пользовательскую метрику.
pub async fn custom_metric_enable_handler(
    State(state): State<ApiState>,
    Path(metric_id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    // Начинаем отслеживание производительности
    let _start_time = Instant::now();

    // Обновляем метрики производительности
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_requests();
    drop(perf_metrics); // Освобождаем блокировку

    // Проверяем валидность metric_id
    if metric_id.is_empty() || metric_id.len() > 100 {
        return Ok(Json(json!({
            "status": "error",
            "message": "Invalid metric_id: must be 1-100 characters long"
        })));
    }

    match &state.custom_metrics_manager {
        Some(manager) => {
            // Получаем текущую конфигурацию
            let config = manager.get_metric_config(&metric_id).map_err(|e| {
                tracing::error!("Failed to get custom metric config for '{}': {}", metric_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            match config {
                Some(mut config) => {
                    config.enabled = true;
                    
                    // Удаляем старую метрику и добавляем новую с обновленной конфигурацией
                    if let Err(e) = manager.remove_metric(&metric_id) {
                        return Ok(Json(json!({
                            "status": "error",
                            "message": format!("Failed to remove old metric config for '{}': {}", metric_id, e)
                        })));
                    }
                    
                    if let Err(e) = manager.add_metric(config) {
                        return Ok(Json(json!({
                            "status": "error",
                            "message": format!("Failed to add updated metric config for '{}': {}", metric_id, e)
                        })));
                    }

                    Ok(Json(json!({
                        "status": "ok",
                        "message": "Custom metric enabled successfully",
                        "metric_id": metric_id,
                        "timestamp": Utc::now().to_rfc3339()
                    })))
                }
                None => Ok(Json(json!({
                    "status": "error",
                    "message": format!("Custom metric '{}' not found", metric_id)
                })))
            }
        }
        None => Ok(Json(json!({
            "status": "error",
            "message": "Custom metrics manager not available (daemon may not be running or feature not enabled)"
        })))
    }
}

/// Обработчик для endpoint `/api/custom-metrics/:metric_id/disable` (POST).
///
/// Отключает пользовательскую метрику.
pub async fn custom_metric_disable_handler(
    State(state): State<ApiState>,
    Path(metric_id): Path<String>,
) -> Result<Json<Value>, StatusCode> {
    // Начинаем отслеживание производительности
    let _start_time = Instant::now();

    // Обновляем метрики производительности
    let mut perf_metrics = state.performance_metrics.write().await;
    perf_metrics.increment_requests();
    drop(perf_metrics); // Освобождаем блокировку

    // Проверяем валидность metric_id
    if metric_id.is_empty() || metric_id.len() > 100 {
        return Ok(Json(json!({
            "status": "error",
            "message": "Invalid metric_id: must be 1-100 characters long"
        })));
    }

    match &state.custom_metrics_manager {
        Some(manager) => {
            // Получаем текущую конфигурацию
            let config = manager.get_metric_config(&metric_id).map_err(|e| {
                tracing::error!("Failed to get custom metric config for '{}': {}", metric_id, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

            match config {
                Some(mut config) => {
                    config.enabled = false;
                    
                    // Удаляем старую метрику и добавляем новую с обновленной конфигурацией
                    if let Err(e) = manager.remove_metric(&metric_id) {
                        return Ok(Json(json!({
                            "status": "error",
                            "message": format!("Failed to remove old metric config for '{}': {}", metric_id, e)
                        })));
                    }
                    
                    if let Err(e) = manager.add_metric(config) {
                        return Ok(Json(json!({
                            "status": "error",
                            "message": format!("Failed to add updated metric config for '{}': {}", metric_id, e)
                        })));
                    }

                    Ok(Json(json!({
                        "status": "ok",
                        "message": "Custom metric disabled successfully",
                        "metric_id": metric_id,
                        "timestamp": Utc::now().to_rfc3339()
                    })))
                }
                None => Ok(Json(json!({
                    "status": "error",
                    "message": format!("Custom metric '{}' not found", metric_id)
                })))
            }
        }
        None => Ok(Json(json!({
            "status": "error",
            "message": "Custom metrics manager not available (daemon may not be running or feature not enabled)"
        })))
    }
}