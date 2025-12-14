// Интеграционные тесты для критических сценариев
//
// Эти тесты проверяют работу системы в критических сценариях:
// - Обработка ошибок
// - Edge cases
// - Взаимодействие между компонентами
// - Производительность под нагрузкой

use smoothtask_core::{
    api::{ApiServer, ApiServerHandle, ApiStateBuilder},
    config::config_struct::{Config, LoggingConfig},
    metrics::system::{NetworkInterface, NetworkMetrics, SystemMetrics},
    DaemonStats,
};
use std::sync::Arc;
use tokio::sync::RwLock;

// Импорты для HTTP тестирования

#[tokio::test]
async fn test_api_error_handling_invalid_endpoint() {
    // Тест: проверка обработки ошибок для несуществующих endpoint
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/nonexistent", port);

    // Отправляем GET запрос к несуществующему endpoint
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен быть выполнен");

    // Проверяем статус код (должен быть 404 или другой код ошибки)
    assert!(response.status().is_client_error() || response.status().is_server_error());

    // Проверяем ответ
    let body = response.text().await.expect("Ответ должен быть текстом");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем структуру ответа об ошибке
    if !body.is_empty() {
        let json_result = serde_json::from_str::<serde_json::Value>(&body);
        if let Ok(json) = json_result {
            assert_eq!(json["status"].as_str(), Some("error"));
            assert!(json["error"].is_string());
            assert!(json["timestamp"].is_string());
        }
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_error_handling_invalid_pid() {
    // Тест: проверка обработки ошибок для невалидного PID
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/processes/-1", port);

    // Отправляем GET запрос с невалидным PID
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен быть выполнен");

    // Проверяем статус код (должен быть 400 Bad Request или другой код ошибки)
    assert!(response.status().is_client_error() || response.status().is_server_error());

    // Проверяем ответ
    let body = response.text().await.expect("Ответ должен быть текстом");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем структуру ответа об ошибке
    assert_eq!(json["status"].as_str(), Some("error"));
    assert!(json["error"].is_string());
    assert!(json["error"].as_str().unwrap().contains("invalid"));
    assert!(json["timestamp"].is_string());

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_graceful_degradation_no_data() {
    // Тест: проверка graceful degradation при отсутствии данных
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/processes", port);

    // Отправляем GET запрос (данные о процессах не предоставлены)
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен быть выполнен");

    // Проверяем статус код (должен быть 200 OK)
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // Проверяем ответ
    let body = response.text().await.expect("Ответ должен быть текстом");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем структуру ответа (должен быть статус ok, но данные могут быть null)
    if !body.is_empty() {
        let json_result = serde_json::from_str::<serde_json::Value>(&body);
        if let Ok(json) = json_result {
            assert_eq!(json["status"].as_str(), Some("ok"));
            assert!(json["timestamp"].is_string());
            // Данные о процессах должны быть null или пустым массивом
            assert!(json["processes"].is_null() || json["processes"].is_array());
        }
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_concurrent_requests() {
    // Тест: проверка обработки параллельных запросов
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/health", port);

    // Отправляем несколько параллельных запросов
    let mut handles = vec![];
    for _ in 0..10 {
        let client = client.clone();
        let url = url.clone();
        handles.push(tokio::spawn(async move {
            let response = client
                .get(&url)
                .send()
                .await
                .expect("Запрос должен быть выполнен");

            // Проверяем статус код
            assert_eq!(response.status(), reqwest::StatusCode::OK);

            // Проверяем ответ
            let body = response.text().await.expect("Ответ должен быть текстом");
            let json: serde_json::Value =
                serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

            assert_eq!(json["status"].as_str(), Some("ok"));
        }));
    }

    // Ждем завершения всех запросов
    for handle in handles {
        handle
            .await
            .expect("Все запросы должны завершиться успешно");
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_component_interaction() {
    // Тест: проверка взаимодействия между компонентами через API
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();

    // Проверяем, что разные компоненты возвращают согласованные данные

    // 1. Проверяем системные метрики
    let system_url = format!("http://127.0.0.1:{}/api/system", port);
    let system_response = client
        .get(&system_url)
        .send()
        .await
        .expect("Запрос должен быть выполнен");

    // 2. Проверяем статистику
    let stats_url = format!("http://127.0.0.1:{}/api/stats", port);
    let stats_response = client
        .get(&stats_url)
        .send()
        .await
        .expect("Запрос должен быть выполнен");

    // 3. Проверяем health
    let health_url = format!("http://127.0.0.1:{}/api/health", port);
    let health_response = client
        .get(&health_url)
        .send()
        .await
        .expect("Запрос должен быть выполнен");

    // Все запросы должны быть успешными
    assert_eq!(system_response.status(), reqwest::StatusCode::OK);
    assert_eq!(stats_response.status(), reqwest::StatusCode::OK);
    assert_eq!(health_response.status(), reqwest::StatusCode::OK);

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_comprehensive_integration() {
    // Тест: комплексная проверка интеграции всех компонентов
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();

    // Проверяем все основные endpoints
    let endpoints = [
        "/health",
        "/api/health",
        "/api/system",
        "/api/stats",
        "/api/metrics",
        "/api/processes",
        "/api/appgroups",
        "/api/config",
        "/api/version",
        "/api/endpoints",
    ];

    for endpoint in endpoints {
        let url = format!("http://127.0.0.1:{}{}", port, endpoint);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен быть выполнен");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_edge_case_empty_parameters() {
    // Тест: проверка обработки пустых параметров
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();

    // Проверяем endpoint с пустым параметром
    let url = format!("http://127.0.0.1:{}/api/appgroups/", port);
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен быть выполнен");

    // Должен быть 404 или 400
    assert!(
        response.status() == reqwest::StatusCode::NOT_FOUND
            || response.status() == reqwest::StatusCode::BAD_REQUEST
    );

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_edge_case_large_payloads() {
    // Тест: проверка обработки больших payload
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/health", port);

    // Отправляем запрос с большим заголовком
    let large_header_value = "x".repeat(10000);
    let response = client
        .get(&url)
        .header("X-Large-Header", large_header_value)
        .send()
        .await;

    // Запрос может быть успешным или отклонен (зависит от конфигурации сервера)
    if let Ok(resp) = response {
        assert!(
            resp.status() == reqwest::StatusCode::OK
                || resp.status() == reqwest::StatusCode::PAYLOAD_TOO_LARGE
        );
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_error_response_structure() {
    // Тест: проверка структуры ответов об ошибках
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/nonexistent", port);

    // Отправляем запрос к несуществующему endpoint
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен быть выполнен");

    // Проверяем статус код
    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

    // Проверяем структуру ответа об ошибке
    let body = response.text().await.expect("Ответ должен быть текстом");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем обязательные поля
    assert_eq!(json["status"].as_str(), Some("error"));
    assert!(json["error"].is_string());
    assert!(json["timestamp"].is_string());

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_success_response_structure() {
    // Тест: проверка структуры успешных ответов
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/health", port);

    // Отправляем запрос
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен быть выполнен");

    // Проверяем статус код
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // Проверяем структуру ответа
    let body = response.text().await.expect("Ответ должен быть текстом");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем обязательные поля
    assert_eq!(json["status"].as_str(), Some("ok"));
    assert!(json["timestamp"].is_string());

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_version_compatibility() {
    // Тест: проверка совместимости версий API
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/version", port);

    // Отправляем запрос
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен быть выполнен");

    // Проверяем статус код
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // Проверяем структуру ответа
    let body = response.text().await.expect("Ответ должен быть текстом");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем обязательные поля
    if !body.is_empty() {
        let json_result = serde_json::from_str::<serde_json::Value>(&body);
        if let Ok(json) = json_result {
            assert_eq!(json["status"].as_str(), Some("ok"));
            assert!(json["version"].is_string());
            assert!(json["timestamp"].is_string());
        }
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_endpoints_discovery() {
    // Тест: проверка discovery endpoint
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/endpoints", port);

    // Отправляем запрос
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен быть выполнен");

    // Проверяем статус код
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // Проверяем структуру ответа
    let body = response.text().await.expect("Ответ должен быть текстом");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем обязательные поля
    if !body.is_empty() {
        let json_result = serde_json::from_str::<serde_json::Value>(&body);
        if let Ok(json) = json_result {
            assert_eq!(json["status"].as_str(), Some("ok"));
            assert!(json["endpoints"].is_array());
            assert!(json["timestamp"].is_string());
        }
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_system_integration() {
    // Тест: проверка интеграции системных компонентов
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();

    // Проверяем системные endpoints
    let endpoints = [
        "/api/system",
        "/api/stats",
        "/api/metrics",
        "/api/responsiveness",
    ];

    for endpoint in endpoints {
        let url = format!("http://127.0.0.1:{}{}", port, endpoint);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен быть выполнен");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_process_management_integration() {
    // Тест: проверка интеграции управления процессами
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();

    // Проверяем endpoints управления процессами
    let endpoints = [
        "/api/processes",
        "/api/processes/energy",
        "/api/processes/memory",
        "/api/processes/gpu",
        "/api/processes/network",
        "/api/processes/disk",
    ];

    for endpoint in endpoints {
        let url = format!("http://127.0.0.1:{}{}", port, endpoint);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен быть выполнен");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_app_group_integration() {
    // Тест: проверка интеграции групп приложений
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();

    // Проверяем endpoints групп приложений
    let endpoints = ["/api/appgroups", "/api/classes", "/api/patterns"];

    for endpoint in endpoints {
        let url = format!("http://127.0.0.1:{}{}", port, endpoint);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен быть выполнен");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_network_integration() {
    // Тест: проверка интеграции сетевых компонентов
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();

    // Проверяем сетевые endpoints
    let endpoints = [
        "/api/network/connections",
        "/api/gpu/temperature-power",
        "/api/gpu/memory",
        "/api/cpu/temperature",
    ];

    for endpoint in endpoints {
        let url = format!("http://127.0.0.1:{}{}", port, endpoint);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен быть выполнен");

        assert!(
            response.status() == reqwest::StatusCode::OK
                || response.status() == reqwest::StatusCode::SERVICE_UNAVAILABLE
        );
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_configuration_integration() {
    // Тест: проверка интеграции конфигурации
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();

    // Проверяем endpoints конфигурации
    let endpoints = [
        "/api/config",
        "/api/config/reload",
        "/api/notifications/status",
        "/api/notifications/config",
    ];

    for endpoint in endpoints {
        let url = format!("http://127.0.0.1:{}{}", port, endpoint);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен быть выполнен");

        assert!(
            response.status() == reqwest::StatusCode::OK
                || response.status() == reqwest::StatusCode::BAD_REQUEST
                || response.status() == reqwest::StatusCode::SERVICE_UNAVAILABLE
        );
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_logging_integration() {
    // Тест: проверка интеграции логирования
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();

    // Проверяем endpoints логирования
    let endpoints = [
        "/api/logs",
        "/api/cache/monitoring",
        "/api/cache/stats",
        "/api/cache/config",
    ];

    for endpoint in endpoints {
        let url = format!("http://127.0.0.1:{}{}", port, endpoint);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен быть выполнен");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_performance_integration() {
    // Тест: проверка интеграции производительности
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();

    // Проверяем endpoints производительности
    let endpoints = ["/api/performance", "/api/app/performance"];

    for endpoint in endpoints {
        let url = format!("http://127.0.0.1:{}{}", port, endpoint);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен быть выполнен");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_api_comprehensive_integration_final() {
    // Тест: финальная комплексная проверка интеграции всех компонентов
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();

    // Проверяем все основные endpoints
    let endpoints = [
        "/health",
        "/api/health",
        "/api/system",
        "/api/stats",
        "/api/metrics",
        "/api/processes",
        "/api/appgroups",
        "/api/config",
        "/api/version",
        "/api/endpoints",
    ];

    for endpoint in endpoints {
        let url = format!("http://127.0.0.1:{}{}", port, endpoint);
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен быть выполнен");

        assert_eq!(response.status(), reqwest::StatusCode::OK);
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}
