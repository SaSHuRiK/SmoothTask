//! Утилиты для валидации входных данных API.

use anyhow::Result;
use axum::{
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use tracing::error;

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
            "error" | "warn" | "info" | "debug" | "trace" => {},
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
    format!("Validation error for field '{}': {} ({})", field, error_type, details)
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
        Some("Please check server logs for details")
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
        Some(&format!("The {} component is required but not available", component)),
        Some(suggestion)
    );
    
    Json(error_response)
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
            Some("must be between 1-100 characters")
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
            Some("Check database configuration")
        );
        
        assert_eq!(response["status"], "error");
        assert_eq!(response["error"], "internal_error");
        assert_eq!(response["message"], "Internal server error");
        assert!(response["details"].is_string());
        assert!(response["suggestion"].is_string());
    }

    #[test]
    fn test_handle_missing_component_error() {
        let response = handle_missing_component_error(
            "database",
            "Please configure database connection"
        );
        
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
}
