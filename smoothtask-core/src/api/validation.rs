//! Утилиты для валидации входных данных API.

use anyhow::Result;
use axum::{http::StatusCode, response::Json};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Instant;
use tracing::error;

// Импортируем необходимые типы из модуля server
use crate::api::server::ApiPerformanceMetrics;

/// Валидирует параметры запроса для endpoint `/api/logs`.
///
/// # Аргументы
///
/// * `params` - Параметры запроса в виде HashMap
///
/// # Возвращает
///
/// * `Result<(), StatusCode>` - Ok(()) если валидация прошла успешно, или ошибка с кодом статуса
pub fn validate_logs_params(params: &HashMap<String, String>) -> Result<(), StatusCode> {
    // Проверяем уровень логирования
    if let Some(level) = params.get("level") {
        match level.as_str() {
            "error" | "warn" | "info" | "debug" | "trace" => {}
            _ => {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
    }

    // Проверяем лимит
    if let Some(limit) = params.get("limit") {
        if let Err(_) = limit.parse::<usize>() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let limit_value = limit.parse::<usize>().unwrap();
        if limit_value > 1000 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    Ok(())
}

/// Валидирует payload для endpoint `/api/notifications/custom`.
///
/// # Аргументы
///
/// * `payload` - JSON payload
///
/// # Возвращает
///
/// * `Result<(), StatusCode>` - Ok(()) если валидация прошла успешно, или ошибка с кодом статуса
pub fn validate_custom_notification_payload(payload: &Value) -> Result<(), StatusCode> {
    // Проверяем, что payload является объектом
    if !payload.is_object() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Проверяем тип уведомления
    if let Some(notification_type) = payload.get("type") {
        if !notification_type.is_string() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let type_str = notification_type.as_str().unwrap();
        if !matches!(type_str, "critical" | "warning" | "info") {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем заголовок
    if let Some(title) = payload.get("title") {
        if !title.is_string() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let title_str = title.as_str().unwrap();
        if title_str.is_empty() || title_str.len() > 100 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем сообщение
    if let Some(message) = payload.get("message") {
        if !message.is_string() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let message_str = message.as_str().unwrap();
        if message_str.is_empty() || message_str.len() > 500 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем детали
    if let Some(details) = payload.get("details") {
        if !details.is_string() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let details_str = details.as_str().unwrap();
        if details_str.len() > 1000 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    Ok(())
}

/// Валидирует payload для endpoint `/api/notifications/config`.
///
/// # Аргументы
///
/// * `payload` - JSON payload
///
/// # Возвращает
///
/// * `Result<(), StatusCode>` - Ok(()) если валидация прошла успешно, или ошибка с кодом статуса
pub fn validate_notifications_config_payload(payload: &Value) -> Result<(), StatusCode> {
    // Проверяем, что payload является объектом
    if !payload.is_object() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Проверяем enabled
    if let Some(enabled) = payload.get("enabled") {
        if !enabled.is_boolean() {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем backend
    if let Some(backend) = payload.get("backend") {
        if !backend.is_string() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let backend_str = backend.as_str().unwrap();
        if !matches!(backend_str, "stub" | "libnotify" | "dbus") {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем app_name
    if let Some(app_name) = payload.get("app_name") {
        if !app_name.is_string() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let app_name_str = app_name.as_str().unwrap();
        if app_name_str.is_empty() || app_name_str.len() > 50 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем min_level
    if let Some(min_level) = payload.get("min_level") {
        if !min_level.is_string() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let min_level_str = min_level.as_str().unwrap();
        if !matches!(min_level_str, "critical" | "warning" | "info") {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    Ok(())
}

/// Валидирует параметры запроса для endpoint `/api/processes/:pid`.
///
/// # Аргументы
///
/// * `pid` - PID процесса
///
/// # Возвращает
///
/// * `Result<(), StatusCode>` - Ok(()) если валидация прошла успешно, или ошибка с кодом статуса
pub fn validate_process_pid(pid: i32) -> Result<(), StatusCode> {
    // Проверяем, что PID находится в допустимом диапазоне
    if pid <= 0 || pid > 999999 {
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(())
}

/// Валидирует параметры запроса для endpoint `/api/appgroups/:id`.
///
/// # Аргументы
///
/// * `app_group_id` - Идентификатор группы приложений
///
/// # Возвращает
///
/// * `Result<(), StatusCode>` - Ok(()) если валидация прошла успешно, или ошибка с кодом статуса
pub fn validate_app_group_id(app_group_id: &str) -> Result<(), StatusCode> {
    // Проверяем, что ID не пустой и не слишком длинный
    if app_group_id.is_empty() || app_group_id.len() > 100 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Проверяем, что ID содержит только допустимые символы
    if !app_group_id
        .chars()
        .all(|c| c.is_ascii() && !c.is_control())
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(())
}

/// Валидирует параметры запроса для endpoint `/api/custom-metrics/:metric_id`.
///
/// # Аргументы
///
/// * `metric_id` - Идентификатор пользовательской метрики
///
/// # Возвращает
///
/// * `Result<(), StatusCode>` - Ok(()) если валидация прошла успешно, или ошибка с кодом статуса
pub fn validate_custom_metric_id(metric_id: &str) -> Result<(), StatusCode> {
    // Проверяем, что ID не пустой и не слишком длинный
    if metric_id.is_empty() || metric_id.len() > 50 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Проверяем, что ID содержит только допустимые символы
    if !metric_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(())
}

/// Валидирует payload для endpoint `/api/custom-metrics/:metric_id/add`.
///
/// # Аргументы
///
/// * `payload` - JSON payload
///
/// # Возвращает
///
/// * `Result<(), StatusCode>` - Ok(()) если валидация прошла успешно, или ошибка с кодом статуса
pub fn validate_custom_metric_add_payload(payload: &Value) -> Result<(), StatusCode> {
    // Проверяем, что payload является объектом
    if !payload.is_object() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Проверяем обязательные поля
    if !payload.get("name").is_some() || !payload.get("source_type").is_some() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Проверяем name
    if let Some(name) = payload.get("name") {
        if !name.is_string() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let name_str = name.as_str().unwrap();
        if name_str.is_empty() || name_str.len() > 100 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем source_type
    if let Some(source_type) = payload.get("source_type") {
        if !source_type.is_string() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let source_type_str = source_type.as_str().unwrap();
        if !matches!(source_type_str, "file" | "command" | "http" | "static") {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем опциональные поля
    if let Some(interval) = payload.get("interval_seconds") {
        if !interval.is_number() || interval.as_u64().unwrap_or(0) > 3600 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    if let Some(timeout) = payload.get("timeout_seconds") {
        if !timeout.is_number() || timeout.as_u64().unwrap_or(0) > 60 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    Ok(())
}

/// Валидирует payload для endpoint `/api/custom-metrics/:metric_id/update`.
///
/// # Аргументы
///
/// * `payload` - JSON payload
///
/// # Возвращает
///
/// * `Result<(), StatusCode>` - Ok(()) если валидация прошла успешно, или ошибка с кодом статуса
pub fn validate_custom_metric_update_payload(payload: &Value) -> Result<(), StatusCode> {
    // Проверяем, что payload является объектом
    if !payload.is_object() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Проверяем, что хотя бы одно поле для обновления предоставлено
    if !payload.get("name").is_some()
        && !payload.get("source_type").is_some()
        && !payload.get("source_config").is_some()
        && !payload.get("interval_seconds").is_some()
        && !payload.get("timeout_seconds").is_some()
        && !payload.get("enabled").is_some()
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Проверяем name
    if let Some(name) = payload.get("name") {
        if !name.is_string() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let name_str = name.as_str().unwrap();
        if name_str.is_empty() || name_str.len() > 100 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем source_type
    if let Some(source_type) = payload.get("source_type") {
        if !source_type.is_string() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let source_type_str = source_type.as_str().unwrap();
        if !matches!(source_type_str, "file" | "command" | "http" | "static") {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем interval_seconds
    if let Some(interval) = payload.get("interval_seconds") {
        if !interval.is_number() || interval.as_u64().unwrap_or(0) > 3600 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем timeout_seconds
    if let Some(timeout) = payload.get("timeout_seconds") {
        if !timeout.is_number() || timeout.as_u64().unwrap_or(0) > 60 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем enabled
    if let Some(enabled) = payload.get("enabled") {
        if !enabled.is_boolean() {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    Ok(())
}

/// Валидирует параметры запроса для endpoint `/api/cache/config`.
///
/// # Аргументы
///
/// * `payload` - JSON payload
///
/// # Возвращает
///
/// * `Result<(), StatusCode>` - Ok(()) если валидация прошла успешно, или ошибка с кодом статуса
pub fn validate_cache_config_payload(payload: &Value) -> Result<(), StatusCode> {
    // Проверяем, что payload является объектом
    if !payload.is_object() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Проверяем ttl_seconds
    if let Some(ttl) = payload.get("ttl_seconds") {
        if !ttl.is_number() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let ttl_value = ttl.as_u64().unwrap_or(0);
        if ttl_value < 10 || ttl_value > 3600 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем enabled
    if let Some(enabled) = payload.get("enabled") {
        if !enabled.is_boolean() {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    Ok(())
}

/// Валидирует payload для обновления основной конфигурации.
///
/// # Аргументы
///
/// * `payload` - JSON payload
///
/// # Возвращает
///
/// * `Result<(), StatusCode>` - Ok(()) если валидация прошла успешно, или ошибка с кодом статуса
pub fn validate_config_update_payload(payload: &Value) -> Result<(), StatusCode> {
    // Проверяем, что payload является объектом
    if !payload.is_object() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Проверяем polling_interval_ms
    if let Some(interval) = payload.get("polling_interval_ms") {
        if !interval.is_number() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let interval_value = interval.as_u64().unwrap_or(0);
        if interval_value < 100 || interval_value > 60000 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем max_candidates
    if let Some(max_candidates) = payload.get("max_candidates") {
        if !max_candidates.is_number() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let max_candidates_value = max_candidates.as_u64().unwrap_or(0);
        if max_candidates_value < 10 || max_candidates_value > 1000 {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем dry_run_default
    if let Some(dry_run) = payload.get("dry_run_default") {
        if !dry_run.is_boolean() {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем policy_mode
    if let Some(policy_mode) = payload.get("policy_mode") {
        if !policy_mode.is_string() {
            return Err(StatusCode::BAD_REQUEST);
        }

        let policy_mode_str = policy_mode.as_str().unwrap();
        if !matches!(policy_mode_str, "rules-only" | "hybrid") {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    // Проверяем enable_snapshot_logging
    if let Some(enable_snapshot) = payload.get("enable_snapshot_logging") {
        if !enable_snapshot.is_boolean() {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    Ok(())
}

/// Создает детальное сообщение об ошибке валидации.
///
/// # Аргументы
///
/// * `field` - Название поля
/// * `error_type` - Тип ошибки
/// * `details` - Дополнительные детали
///
/// # Возвращает
///
/// * `String` - Детальное сообщение об ошибке
pub fn create_validation_error_message(field: &str, error_type: &str, details: &str) -> String {
    format!(
        "Validation error for field '{}': {} ({})",
        field, error_type, details
    )
}

/// Создает JSON ответ с ошибкой валидации.
///
/// # Аргументы
///
/// * `status` - Код статуса
/// * `message` - Сообщение об ошибке
/// * `field` - Поле, вызвавшее ошибку (опционально)
/// * `details` - Дополнительные детали (опционально)
///
/// # Возвращает
///
/// * `Value` - JSON объект с информацией об ошибке
pub fn create_validation_error_response(
    status: &str,
    message: &str,
    field: Option<&str>,
    details: Option<&str>,
) -> Value {
    let mut error_json = json!({
        "status": status,
        "error": "validation_error",
        "message": message,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    if let Some(field_name) = field {
        error_json["field"] = json!(field_name);
    }

    if let Some(details_str) = details {
        error_json["details"] = json!(details_str);
    }

    error_json
}

/// Создает JSON ответ с ошибкой API.
///
/// # Аргументы
///
/// * `status` - Код статуса
/// * `error_type` - Тип ошибки
/// * `message` - Сообщение об ошибке
/// * `details` - Дополнительные детали (опционально)
/// * `suggestion` - Рекомендация по исправлению (опционально)
///
/// # Возвращает
///
/// * `Value` - JSON объект с информацией об ошибке
pub fn create_api_error_response(
    status: &str,
    error_type: &str,
    message: &str,
    details: Option<&str>,
    suggestion: Option<&str>,
) -> Value {
    let mut error_json = json!({
        "status": status,
        "error": error_type,
        "message": message,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    if let Some(details_str) = details {
        error_json["details"] = json!(details_str);
    }

    if let Some(suggestion_str) = suggestion {
        error_json["suggestion"] = json!(suggestion_str);
    }

    error_json
}

/// Централизованный обработчик ошибок для API.
///
/// # Аргументы
///
/// * `error` - Ошибка anyhow
/// * `context` - Контекст ошибки
/// * `_status_code` - Код статуса HTTP (используется для документации)
///
/// # Возвращает
///
/// * `Result<Json<Value>, StatusCode>` - JSON ответ с ошибкой или оригинальная ошибка
pub fn handle_api_error(
    error: anyhow::Error,
    context: &str,
    _status_code: StatusCode,
) -> Result<Json<Value>, StatusCode> {
    error!("API error in {}: {}", context, error);

    let error_response = create_api_error_response(
        "error",
        "internal_error",
        "Internal server error",
        Some(&format!("Error in {}: {}", context, error)),
        Some("Please check server logs for details"),
    );

    Ok(Json(error_response))
}

/// Централизованный обработчик ошибок для отсутствующих компонентов.
///
/// # Аргументы
///
/// * `component` - Название отсутствующего компонента
/// * `suggestion` - Рекомендация по исправлению
///
/// # Возвращает
///
/// * `Json<Value>` - JSON ответ с информацией об ошибке
pub fn handle_missing_component_error(component: &str, suggestion: &str) -> Json<Value> {
    let error_response = create_api_error_response(
        "error",
        "component_unavailable",
        &format!("Component '{}' is not available", component),
        Some(&format!(
            "The {} component is required but not available",
            component
        )),
        Some(suggestion),
    );

    Json(error_response)
}

/// Создает детальный JSON ответ с ошибкой валидации, включая информацию о поле и ограничениях.
///
/// # Аргументы
///
/// * `field` - Название поля, вызвавшего ошибку
/// * `error_type` - Тип ошибки валидации
/// * `value` - Значение, вызвавшее ошибку (опционально)
/// * `constraints` - Ограничения для поля (опционально)
/// * `suggestion` - Рекомендация по исправлению (опционально)
///
/// # Возвращает
///
/// * `Value` - JSON объект с детальной информацией об ошибке валидации
pub fn create_detailed_validation_error_response(
    field: &str,
    error_type: &str,
    value: Option<&str>,
    constraints: Option<&str>,
    suggestion: Option<&str>,
) -> Value {
    let mut error_json = json!({
        "status": "error",
        "error": "validation_error",
        "field": field,
        "error_type": error_type,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    if let Some(value_str) = value {
        error_json["value"] = json!(value_str);
    }

    if let Some(constraints_str) = constraints {
        error_json["constraints"] = json!(constraints_str);
    }

    if let Some(suggestion_str) = suggestion {
        error_json["suggestion"] = json!(suggestion_str);
    }

    error_json
}

/// Создает JSON ответ с ошибкой аутентификации/авторизации.
///
/// # Аргументы
///
/// * `error_type` - Тип ошибки аутентификации
/// * `message` - Сообщение об ошибке
/// * `details` - Дополнительные детали (опционально)
///
/// # Возвращает
///
/// * `Value` - JSON объект с информацией об ошибке аутентификации
pub fn create_auth_error_response(error_type: &str, message: &str, details: Option<&str>) -> Value {
    let mut error_json = json!({
        "status": "error",
        "error": error_type,
        "message": message,
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    if let Some(details_str) = details {
        error_json["details"] = json!(details_str);
    }

    error_json
}

/// Создает JSON ответ с ошибкой ограничения скорости.
///
/// # Аргументы
///
/// * `retry_after_seconds` - Время до повторной попытки в секундах
/// * `limit` - Максимальное количество запросов
/// * `remaining` - Оставшееся количество запросов
///
/// # Возвращает
///
/// * `Value` - JSON объект с информацией об ошибке ограничения скорости
pub fn create_rate_limit_error_response(
    retry_after_seconds: u64,
    limit: u64,
    remaining: u64,
) -> Value {
    json!({
        "status": "error",
        "error": "rate_limit_exceeded",
        "message": "Rate limit exceeded",
        "retry_after_seconds": retry_after_seconds,
        "limit": limit,
        "remaining": remaining,
        "timestamp": chrono::Utc::now().to_rfc3339()
    })
}

/// Централизованный обработчик ошибок для API с улучшенным логированием и метриками.
///
/// # Аргументы
///
/// * `error` - Ошибка anyhow
/// * `context` - Контекст ошибки
/// * `perf_metrics` - Метрики производительности для обновления (опционально)
///
/// # Возвращает
///
/// * `Result<Json<Value>, StatusCode>` - JSON ответ с ошибкой или оригинальная ошибка
pub fn handle_api_error_with_metrics(
    error: anyhow::Error,
    context: &str,
    _status_code: StatusCode,
    perf_metrics: Option<&mut ApiPerformanceMetrics>,
) -> Result<Json<Value>, StatusCode> {
    error!("API error in {}: {}", context, error);

    // Обновляем метрики ошибок, если предоставлены
    if let Some(metrics) = perf_metrics {
        metrics.total_requests += 1;
        metrics.last_request_time = Some(Instant::now());
    }

    let error_response = create_api_error_response(
        "error",
        "internal_error",
        "Internal server error",
        Some(&format!("Error in {}: {}", context, error)),
        Some("Please check server logs for details"),
    );

    Ok(Json(error_response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_logs_params_valid() {
        let mut params = HashMap::new();
        params.insert("level".to_string(), "info".to_string());
        params.insert("limit".to_string(), "50".to_string());

        let result = validate_logs_params(&params);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_logs_params_invalid_level() {
        let mut params = HashMap::new();
        params.insert("level".to_string(), "invalid".to_string());

        let result = validate_logs_params(&params);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_logs_params_invalid_limit() {
        let mut params = HashMap::new();
        params.insert("limit".to_string(), "invalid".to_string());

        let result = validate_logs_params(&params);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_logs_params_limit_too_high() {
        let mut params = HashMap::new();
        params.insert("limit".to_string(), "1500".to_string());

        let result = validate_logs_params(&params);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_custom_notification_payload_valid() {
        let payload = json!({
            "type": "info",
            "title": "Test",
            "message": "Test message",
            "details": "Test details"
        });

        let result = validate_custom_notification_payload(&payload);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_custom_notification_payload_invalid_type() {
        let payload = json!({
            "type": "invalid"
        });

        let result = validate_custom_notification_payload(&payload);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_custom_notification_payload_empty_title() {
        let payload = json!({
            "title": ""
        });

        let result = validate_custom_notification_payload(&payload);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_notifications_config_payload_valid() {
        let payload = json!({
            "enabled": true,
            "backend": "stub",
            "app_name": "SmoothTask",
            "min_level": "info"
        });

        let result = validate_notifications_config_payload(&payload);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_notifications_config_payload_invalid_backend() {
        let payload = json!({
            "backend": "invalid"
        });

        let result = validate_notifications_config_payload(&payload);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_create_validation_error_message() {
        let message = create_validation_error_message("title", "invalid_length", "too short");
        assert!(message.contains("title"));
        assert!(message.contains("invalid_length"));
        assert!(message.contains("too short"));
    }

    #[test]
    fn test_create_validation_error_response() {
        let response = create_validation_error_response(
            "error",
            "Invalid input",
            Some("title"),
            Some("must be between 1-100 characters"),
        );

        assert_eq!(response["status"], "error");
        assert_eq!(response["error"], "validation_error");
        assert_eq!(response["field"], "title");
        assert!(response["details"].is_string());
    }

    #[test]
    fn test_create_api_error_response() {
        let response = create_api_error_response(
            "error",
            "internal_error",
            "Internal server error",
            Some("Database connection failed"),
            Some("Check database configuration"),
        );

        assert_eq!(response["status"], "error");
        assert_eq!(response["error"], "internal_error");
        assert_eq!(response["message"], "Internal server error");
        assert!(response["details"].is_string());
        assert!(response["suggestion"].is_string());
    }

    #[test]
    fn test_handle_missing_component_error() {
        let response =
            handle_missing_component_error("database", "Please configure database connection");

        assert_eq!(response["status"], "error");
        assert_eq!(response["error"], "component_unavailable");
        assert!(response["message"].as_str().unwrap().contains("database"));
    }

    #[test]
    fn test_handle_api_error() {
        let error = anyhow::anyhow!("Test error");
        let result = handle_api_error(error, "test_context", StatusCode::INTERNAL_SERVER_ERROR);

        assert!(result.is_ok());
        let json_response = result.unwrap();
        assert_eq!(json_response["status"], "error");
        assert_eq!(json_response["error"], "internal_error");
    }

    #[test]
    fn test_validate_process_pid_valid() {
        let result = validate_process_pid(12345);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_process_pid_invalid() {
        let result = validate_process_pid(-1);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));

        let result = validate_process_pid(1000000);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_app_group_id_valid() {
        let result = validate_app_group_id("test-group-123");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_app_group_id_invalid() {
        let result = validate_app_group_id("");
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));

        let result = validate_app_group_id(&"a".repeat(101));
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));

        let result = validate_app_group_id("test\x00group");
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_custom_metric_id_valid() {
        let result = validate_custom_metric_id("test_metric_123");
        assert!(result.is_ok());

        let result = validate_custom_metric_id("test-metric-456");
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_custom_metric_id_invalid() {
        let result = validate_custom_metric_id("");
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));

        let result = validate_custom_metric_id(&"a".repeat(51));
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));

        let result = validate_custom_metric_id("test metric");
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_custom_metric_add_payload_valid() {
        let payload = json!({
            "name": "test_metric",
            "source_type": "file",
            "source_config": {"path": "/tmp/test"},
            "interval_seconds": 60,
            "timeout_seconds": 10
        });

        let result = validate_custom_metric_add_payload(&payload);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_custom_metric_add_payload_missing_required() {
        let payload = json!({
            "name": "test_metric"
        });

        let result = validate_custom_metric_add_payload(&payload);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_custom_metric_add_payload_invalid_name() {
        let payload = json!({
            "name": "",
            "source_type": "file"
        });

        let result = validate_custom_metric_add_payload(&payload);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_custom_metric_add_payload_invalid_source_type() {
        let payload = json!({
            "name": "test_metric",
            "source_type": "invalid"
        });

        let result = validate_custom_metric_add_payload(&payload);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_custom_metric_update_payload_valid() {
        let payload = json!({
            "name": "updated_metric",
            "enabled": true
        });

        let result = validate_custom_metric_update_payload(&payload);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_custom_metric_update_payload_no_fields() {
        let payload = json!({});

        let result = validate_custom_metric_update_payload(&payload);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_custom_metric_update_payload_invalid_interval() {
        let payload = json!({
            "interval_seconds": 4000
        });

        let result = validate_custom_metric_update_payload(&payload);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_cache_config_payload_valid() {
        let payload = json!({
            "ttl_seconds": 60,
            "enabled": true
        });

        let result = validate_cache_config_payload(&payload);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_cache_config_payload_invalid_ttl() {
        let payload = json!({
            "ttl_seconds": 5
        });

        let result = validate_cache_config_payload(&payload);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));

        let payload = json!({
            "ttl_seconds": 4000
        });

        let result = validate_cache_config_payload(&payload);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_cache_config_payload_invalid_enabled() {
        let payload = json!({
            "enabled": "true"
        });

        let result = validate_cache_config_payload(&payload);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_create_detailed_validation_error_response() {
        let response = create_detailed_validation_error_response(
            "username",
            "invalid_length",
            Some("ab"),
            Some("must be 3-20 characters"),
            Some("Please provide a username between 3-20 characters"),
        );

        assert_eq!(response["status"], "error");
        assert_eq!(response["error"], "validation_error");
        assert_eq!(response["field"], "username");
        assert_eq!(response["error_type"], "invalid_length");
        assert_eq!(response["value"], "ab");
        assert!(response["constraints"].is_string());
        assert!(response["suggestion"].is_string());
    }

    #[test]
    fn test_create_auth_error_response() {
        let response = create_auth_error_response(
            "unauthorized",
            "Authentication required",
            Some("Please provide valid credentials"),
        );

        assert_eq!(response["status"], "error");
        assert_eq!(response["error"], "unauthorized");
        assert_eq!(response["message"], "Authentication required");
        assert!(response["details"].is_string());
    }

    #[test]
    fn test_create_rate_limit_error_response() {
        let response = create_rate_limit_error_response(60, 100, 0);

        assert_eq!(response["status"], "error");
        assert_eq!(response["error"], "rate_limit_exceeded");
        assert_eq!(response["retry_after_seconds"], 60);
        assert_eq!(response["limit"], 100);
        assert_eq!(response["remaining"], 0);
    }

    #[test]
    fn test_validate_config_update_payload_valid() {
        let payload = json!({
            "polling_interval_ms": 1000,
            "max_candidates": 100,
            "dry_run_default": true,
            "policy_mode": "hybrid",
            "enable_snapshot_logging": false
        });

        let result = validate_config_update_payload(&payload);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_update_payload_partial() {
        let payload = json!({
            "polling_interval_ms": 500
        });

        let result = validate_config_update_payload(&payload);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_config_update_payload_invalid_interval() {
        let payload = json!({
            "polling_interval_ms": 50
        });

        let result = validate_config_update_payload(&payload);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_config_update_payload_invalid_max_candidates() {
        let payload = json!({
            "max_candidates": 5
        });

        let result = validate_config_update_payload(&payload);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_config_update_payload_invalid_policy_mode() {
        let payload = json!({
            "policy_mode": "invalid"
        });

        let result = validate_config_update_payload(&payload);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }

    #[test]
    fn test_validate_config_update_payload_valid_policy_modes() {
        // Тест валидных policy_mode значений
        let payload1 = json!({
            "policy_mode": "rules-only"
        });
        let result1 = validate_config_update_payload(&payload1);
        assert!(result1.is_ok());

        let payload2 = json!({
            "policy_mode": "hybrid"
        });
        let result2 = validate_config_update_payload(&payload2);
        assert!(result2.is_ok());
    }

    #[test]
    fn test_validate_config_update_payload_invalid_type() {
        let payload = json!({
            "polling_interval_ms": "not_a_number"
        });

        let result = validate_config_update_payload(&payload);
        assert!(matches!(result, Err(StatusCode::BAD_REQUEST)));
    }
}
