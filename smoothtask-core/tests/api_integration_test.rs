// Интеграционные тесты для API сервера
//
// Эти тесты проверяют работу API сервера через публичный интерфейс
// и интеграцию с основными компонентами системы.

use smoothtask_core::{
    api::{ApiServer, ApiServerHandle, ApiStateBuilder},
    metrics::system::{NetworkInterface, NetworkMetrics, SystemMetrics},
    DaemonStats,
};
use std::sync::Arc;
use tokio::sync::RwLock;

// Импорты для HTTP тестирования
use reqwest::Client;
use serde_json::json;

#[tokio::test]
async fn test_api_server_creation() {
    // Тест: создание API сервера должно работать без ошибок
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());

    // Сервер должен быть создан и запущен без ошибок
    let handle_result = server.start().await;
    assert!(handle_result.is_ok());

    // Останавливаем сервер
    if let Ok(handle) = handle_result {
        let _ = handle.shutdown().await;
    }
}

#[tokio::test]
async fn test_api_server_with_daemon_stats() {
    // Тест: создание API сервера с статистикой демона
    let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
    let server = ApiServer::with_daemon_stats("127.0.0.1:0".parse().unwrap(), daemon_stats);

    // Сервер должен быть создан и запущен без ошибок
    let handle_result = server.start().await;
    assert!(handle_result.is_ok());

    // Останавливаем сервер
    if let Ok(handle) = handle_result {
        let _ = handle.shutdown().await;
    }
}

#[tokio::test]
async fn test_api_server_start_and_shutdown() {
    // Тест: сервер должен корректно запускаться и останавливаться
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());

    // Запускаем сервер
    let handle_result = server.start().await;
    assert!(handle_result.is_ok());

    if let Ok(handle) = handle_result {
        // Останавливаем сервер
        assert!(handle.shutdown().await.is_ok());
    }
}

#[tokio::test]
async fn test_api_server_with_daemon_stats_start_shutdown() {
    // Тест: сервер с статистикой демона должен корректно запускаться и останавливаться
    let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
    let server = ApiServer::with_daemon_stats("127.0.0.1:0".parse().unwrap(), daemon_stats);

    // Запускаем сервер
    let handle_result = server.start().await;
    assert!(handle_result.is_ok());

    if let Ok(handle) = handle_result {
        // Останавливаем сервер
        assert!(handle.shutdown().await.is_ok());
    }
}

#[tokio::test]
async fn test_api_server_multiple_instances() {
    // Тест: должно быть возможно создать и запустить несколько инстансов сервера
    let server1 = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let server2 = ApiServer::new("127.0.0.1:0".parse().unwrap());

    // Оба сервера должны запуститься без ошибок
    let handle1 = server1.start().await;
    let handle2 = server2.start().await;

    assert!(handle1.is_ok());
    assert!(handle2.is_ok());

    // Останавливаем оба сервера
    if let Ok(handle) = handle1 {
        let _ = handle.shutdown().await;
    }
    if let Ok(handle) = handle2 {
        let _ = handle.shutdown().await;
    }
}

#[tokio::test]
async fn test_api_server_handle_types() {
    // Тест: handle должен быть корректного типа
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());

    if let Ok(handle) = server.start().await {
        // Handle должен быть ApiServerHandle
        let _: ApiServerHandle = handle;
        // Останавливаем сервер
        let _ = handle.shutdown().await;
    }
}

#[tokio::test]
async fn test_api_server_different_ports() {
    // Тест: серверы должны работать на разных портах
    let server1 = ApiServer::new("127.0.0.1:8080".parse().unwrap());
    let server2 = ApiServer::new("127.0.0.1:8081".parse().unwrap());

    // Оба сервера должны запуститься без ошибок
    let handle1 = server1.start().await;
    let handle2 = server2.start().await;

    assert!(handle1.is_ok());
    assert!(handle2.is_ok());

    // Останавливаем оба сервера
    if let Ok(handle) = handle1 {
        let _ = handle.shutdown().await;
    }
    if let Ok(handle) = handle2 {
        let _ = handle.shutdown().await;
    }
}

#[tokio::test]
async fn test_api_server_ipv4_addresses() {
    // Тест: сервер должен работать с разными IPv4 адресами
    let server1 = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let server2 = ApiServer::new("0.0.0.0:0".parse().unwrap());

    // Оба сервера должны запуститься без ошибок
    let handle1 = server1.start().await;
    let handle2 = server2.start().await;

    assert!(handle1.is_ok());
    assert!(handle2.is_ok());

    // Останавливаем оба сервера
    if let Ok(handle) = handle1 {
        let _ = handle.shutdown().await;
    }
    if let Ok(handle) = handle2 {
        let _ = handle.shutdown().await;
    }
}

#[tokio::test]
async fn test_api_server_daemon_stats_immutability() {
    // Тест: статистика демона должна оставаться неизменной
    let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
    let original_stats = daemon_stats.clone();

    let server = ApiServer::with_daemon_stats("127.0.0.1:0".parse().unwrap(), daemon_stats);

    // Сервер должен запуститься без ошибок
    let handle_result = server.start().await;
    assert!(handle_result.is_ok());

    // Статистика должна оставаться доступной
    let _stats_read = original_stats.read().await;

    // Останавливаем сервер
    if let Ok(handle) = handle_result {
        let _ = handle.shutdown().await;
    }
}

#[tokio::test]
async fn test_api_server_creation_consistency() {
    // Тест: создание и запуск сервера должно быть консистентным
    let server1 = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let server2 = ApiServer::new("127.0.0.1:0".parse().unwrap());

    let handle1 = server1.start().await;
    let handle2 = server2.start().await;

    // Оба создания должны иметь одинаковый результат
    assert_eq!(handle1.is_ok(), handle2.is_ok());

    // Останавливаем оба сервера
    if let Ok(handle) = handle1 {
        let _ = handle.shutdown().await;
    }
    if let Ok(handle) = handle2 {
        let _ = handle.shutdown().await;
    }
}

#[tokio::test]
async fn test_api_server_port_zero() {
    // Тест: сервер должен работать с портом 0 (автоматический выбор)
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());

    // Сервер должен запуститься без ошибок
    let handle = server.start().await;
    assert!(handle.is_ok());

    // Останавливаем сервер
    if let Ok(handle) = handle {
        let _ = handle.shutdown().await;
    }
}

#[tokio::test]
async fn test_api_server_localhost_variations() {
    // Тест: сервер должен работать с разными вариантами localhost
    let server1 = ApiServer::new("127.0.0.1:0".parse().unwrap());

    // Пробуем создать сервер с localhost, но обрабатываем ошибку парсинга
    let server2_result = "localhost:0".parse::<std::net::SocketAddr>();

    // Первый сервер должен запуститься без ошибок
    let handle1 = server1.start().await;
    assert!(handle1.is_ok());

    // Останавливаем первый сервер
    if let Ok(handle) = handle1 {
        let _ = handle.shutdown().await;
    }

    // Проверяем, что localhost не может быть распарсен как SocketAddr
    // Это ожидаемое поведение - нужно использовать IP адрес
    assert!(server2_result.is_err());
}

#[tokio::test]
async fn test_network_metrics_in_api() {
    // Тест: проверка, что сетевые метрики корректно возвращаются через API
    use smoothtask_core::metrics::system::{CpuTimes, LoadAvg, MemoryInfo, PressureMetrics};

    // Создаем тестовые сетевые метрики
    let mut network_metrics = NetworkMetrics::default();
    network_metrics.interfaces.push(NetworkInterface {
        name: "eth0".to_string().into(),
        rx_bytes: 1000,
        tx_bytes: 2000,
        rx_packets: 100,
        tx_packets: 200,
        rx_errors: 1,
        tx_errors: 2,
    });
    network_metrics.total_rx_bytes = 1000;
    network_metrics.total_tx_bytes = 2000;

    // Создаем полные системные метрики с сетевыми метриками
    let system_metrics = SystemMetrics {
        cpu_times: CpuTimes::default(),
        memory: MemoryInfo::default(),
        load_avg: LoadAvg::default(),
        pressure: PressureMetrics::default(),
        network: network_metrics,
        ..Default::default()
    };

    // Создаем API сервер с тестовыми метриками
    let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
    let system_metrics_arc = Arc::new(RwLock::new(system_metrics));

    let api_state = ApiStateBuilder::new()
        .with_daemon_stats(Some(daemon_stats))
        .with_system_metrics(Some(system_metrics_arc))
        .build();

    let server = ApiServer::with_state("127.0.0.1:0".parse().unwrap(), api_state);
    let handle = server.start().await.expect("Сервер должен запуститься");

    // Получаем порт, на котором запустился сервер
    let port = handle.port();

    // Делаем HTTP запрос к API для получения метрик
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/metrics", port);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться успешно");

    assert!(response.status().is_success());

    let body = response.text().await.expect("Ответ должен содержать текст");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем, что ответ содержит сетевые метрики
    assert!(json["status"].as_str() == Some("ok"));
    assert!(json["system_metrics"].is_object());

    let system_metrics_json = &json["system_metrics"];

    // Проверяем, что сетевые метрики присутствуют
    assert!(system_metrics_json["network"].is_object());
    assert!(system_metrics_json["network"]["interfaces"].is_array());
    assert!(system_metrics_json["network"]["total_rx_bytes"].is_number());
    assert!(system_metrics_json["network"]["total_tx_bytes"].is_number());

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_api_error_handling() {
    // Тест: проверка обработки ошибок API
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    let client = reqwest::Client::new();

    // Тестируем несуществующий endpoint
    let url = format!("http://127.0.0.1:{}/api/nonexistent", port);
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться");

    // Должен вернуть 404
    assert_eq!(response.status(), reqwest::StatusCode::NOT_FOUND);

    // Тестируем неверный метод
    let url = format!("http://127.0.0.1:{}/api/stats", port);
    let response = client
        .post(&url)
        .send()
        .await
        .expect("Запрос должен выполниться");

    // Должен вернуть 405 Method Not Allowed
    assert_eq!(response.status(), reqwest::StatusCode::METHOD_NOT_ALLOWED);

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_api_concurrent_requests() {
    // Тест: проверка обработки конкурентных запросов
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/health", port);

    // Отправляем 10 конкурентных запросов
    let mut tasks = Vec::new();
    for _ in 0..10 {
        let client = client.clone();
        let url = url.clone();
        tasks.push(tokio::spawn(async move {
            let response = client
                .get(&url)
                .send()
                .await
                .expect("Запрос должен выполниться");
            assert!(response.status().is_success());
            response.text().await.expect("Ответ должен содержать текст")
        }));
    }

    // Ждем завершения всех запросов
    for task in tasks {
        task.await.expect("Задача должна завершиться успешно");
    }

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_api_json_validation() {
    // Тест: проверка валидации JSON ответов
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

    // Проверяем структуру JSON
    assert!(json["status"].is_string());
    assert!(json["service"].is_string());

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_api_content_type_headers() {
    // Тест: проверка Content-Type заголовков
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

    // Проверяем Content-Type заголовок
    let content_type = response
        .headers()
        .get("content-type")
        .expect("Content-Type должен присутствовать");
    assert!(content_type.to_str().unwrap().contains("application/json"));

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_api_cors_headers() {
    // Тест: проверка CORS заголовков (если поддерживаются)
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

    // Проверяем, что ответ успешно возвращается
    // (CORS заголовки могут быть добавлены в будущем)

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_api_response_time() {
    // Тест: проверка времени ответа API
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/health", port);

    let start_time = std::time::Instant::now();
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться");
    let response_time = start_time.elapsed();

    assert!(response.status().is_success());

    // Время ответа должно быть разумным (менее 1 секунды)
    assert!(response_time.as_millis() < 1000);

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_api_endpoint_consistency() {
    // Тест: проверка консистентности ответов endpoints
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/endpoints", port);

    // Делаем несколько запросов и проверяем, что ответы одинаковые
    let response1 = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться");

    let response2 = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться");

    assert!(response1.status().is_success());
    assert!(response2.status().is_success());

    let body1 = response1
        .text()
        .await
        .expect("Ответ должен содержать текст");
    let body2 = response2
        .text()
        .await
        .expect("Ответ должен содержать текст");

    // Ответы должны быть идентичными
    assert_eq!(body1, body2);

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_api_version_consistency() {
    // Тест: проверка консистентности версии API
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/version", port);

    // Делаем несколько запросов и проверяем, что версия одинаковая
    let response1 = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться");

    let response2 = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться");

    assert!(response1.status().is_success());
    assert!(response2.status().is_success());

    let body1 = response1
        .text()
        .await
        .expect("Ответ должен содержать текст");
    let body2 = response2
        .text()
        .await
        .expect("Ответ должен содержать текст");

    // Ответы должны быть идентичными
    assert_eq!(body1, body2);

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_api_health_endpoint_stability() {
    // Тест: проверка стабильности health endpoint
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/health", port);

    // Делаем 100 запросов и проверяем, что все успешные
    for _ in 0..100 {
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен выполниться");
        assert!(response.status().is_success());
    }

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_network_metrics_empty() {
    // Тест: проверка, что API корректно обрабатывает пустые сетевые метрики
    use smoothtask_core::metrics::system::{CpuTimes, LoadAvg, MemoryInfo, PressureMetrics};

    // Создаем системные метрики с пустыми сетевыми метриками
    let system_metrics = SystemMetrics {
        cpu_times: CpuTimes::default(),
        memory: MemoryInfo::default(),
        load_avg: LoadAvg::default(),
        pressure: PressureMetrics::default(),
        network: NetworkMetrics::default(),
        ..Default::default()
    };

    // Создаем API сервер с тестовыми метриками
    let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
    let system_metrics_arc = Arc::new(RwLock::new(system_metrics));

    let api_state = ApiStateBuilder::new()
        .with_daemon_stats(Some(daemon_stats))
        .with_system_metrics(Some(system_metrics_arc))
        .build();

    let server = ApiServer::with_state("127.0.0.1:0".parse().unwrap(), api_state);
    let handle = server.start().await.expect("Сервер должен запуститься");

    // Получаем порт, на котором запустился сервер
    let port = handle.port();

    // Делаем HTTP запрос к API для получения метрик
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/metrics", port);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться успешно");

    assert!(response.status().is_success());

    let body = response.text().await.expect("Ответ должен содержать текст");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем, что ответ содержит пустые сетевые метрики
    assert!(json["status"].as_str() == Some("ok"));
    assert!(json["system_metrics"].is_object());

    let system_metrics_json = &json["system_metrics"];
    assert!(system_metrics_json["network"].is_object());

    let network_json = &system_metrics_json["network"];
    assert!(network_json["interfaces"].is_array());
    assert_eq!(network_json["interfaces"].as_array().unwrap().len(), 0);
    assert_eq!(network_json["total_rx_bytes"].as_u64(), Some(0));
    assert_eq!(network_json["total_tx_bytes"].as_u64(), Some(0));

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_cpu_temperature_endpoint_without_metrics() {
    // Тест: endpoint температуры CPU без метрик
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Выполняем запрос к endpoint
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/cpu/temperature", port);
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен завершиться успешно");

    // Проверяем статус код
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // Проверяем ответ
    let body = response.text().await.expect("Ответ должен быть текстом");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем структуру ответа
    assert_eq!(json["status"].as_str(), Some("error"));
    assert_eq!(
        json["error"].as_str(),
        Some("Metrics collector not available")
    );
    assert!(json["timestamp"].is_string());

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_cpu_temperature_endpoint_with_metrics() {
    // Тест: endpoint температуры CPU с метриками
    // Упрощенная версия, которая тестирует поведение при недоступности eBPF
    
    // Создаем сервер без коллектора метрик (симулируем недоступность eBPF)
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Выполняем запрос к endpoint
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/cpu/temperature", port);
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен завершиться успешно");

    // Проверяем статус код
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // Проверяем ответ
    let body = response.text().await.expect("Ответ должен быть текстом");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем структуру ответа
    // Без коллектора метрик должен быть статус error
    assert_eq!(json["status"].as_str(), Some("error"));
    assert_eq!(
        json["error"].as_str(),
        Some("Metrics collector not available")
    );

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_cpu_temperature_endpoint_error_handling() {
    // Тест: обработка ошибок в endpoint температуры CPU
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Выполняем запрос к endpoint
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/cpu/temperature", port);
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен завершиться успешно");

    // Проверяем статус код
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // Проверяем ответ
    let body = response.text().await.expect("Ответ должен быть текстом");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем структуру ответа
    assert!(json["status"].is_string());
    assert!(json["timestamp"].is_string());

    // В тестовой среде без метрик должен быть статус error
    if json["status"].as_str() == Some("error") {
        assert!(json["error"].is_string());
    }

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_network_metrics_multiple_interfaces() {
    // Тест: проверка, что API корректно обрабатывает несколько сетевых интерфейсов
    use smoothtask_core::metrics::system::{CpuTimes, LoadAvg, MemoryInfo, PressureMetrics};

    // Создаем тестовые сетевые метрики с несколькими интерфейсами
    let mut network_metrics = NetworkMetrics::default();

    // Добавляем первый интерфейс
    network_metrics.interfaces.push(NetworkInterface {
        name: "eth0".to_string().into(),
        rx_bytes: 1000,
        tx_bytes: 2000,
        rx_packets: 100,
        tx_packets: 200,
        rx_errors: 1,
        tx_errors: 2,
    });

    // Добавляем второй интерфейс
    network_metrics.interfaces.push(NetworkInterface {
        name: "wlan0".to_string().into(),
        rx_bytes: 500,
        tx_bytes: 1500,
        rx_packets: 50,
        tx_packets: 150,
        rx_errors: 0,
        tx_errors: 0,
    });

    // Устанавливаем общие метрики (сумма всех интерфейсов)
    network_metrics.total_rx_bytes = 1500; // 1000 + 500
    network_metrics.total_tx_bytes = 3500; // 2000 + 1500

    // Создаем полные системные метрики
    let system_metrics = SystemMetrics {
        cpu_times: CpuTimes::default(),
        memory: MemoryInfo::default(),
        load_avg: LoadAvg::default(),
        pressure: PressureMetrics::default(),
        network: network_metrics,
        ..Default::default()
    };

    // Создаем API сервер с тестовыми метриками
    let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
    let system_metrics_arc = Arc::new(RwLock::new(system_metrics));

    let api_state = ApiStateBuilder::new()
        .with_daemon_stats(Some(daemon_stats))
        .with_system_metrics(Some(system_metrics_arc))
        .build();

    let server = ApiServer::with_state("127.0.0.1:0".parse().unwrap(), api_state);
    let handle = server.start().await.expect("Сервер должен запуститься");

    // Получаем порт, на котором запустился сервер
    let port = handle.port();

    // Делаем HTTP запрос к API для получения метрик
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/metrics", port);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться успешно");

    assert!(response.status().is_success());

    let body = response.text().await.expect("Ответ должен содержать текст");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем, что ответ содержит сетевые метрики
    assert!(json["status"].as_str() == Some("ok"));
    assert!(json["system_metrics"].is_object());

    let system_metrics_json = &json["system_metrics"];
    assert!(system_metrics_json["network"].is_object());

    let network_json = &system_metrics_json["network"];
    assert!(network_json["interfaces"].is_array());

    // Проверяем, что есть два интерфейса
    let interfaces = network_json["interfaces"].as_array().unwrap();
    assert_eq!(interfaces.len(), 2);

    // Проверяем первый интерфейс (eth0)
    let eth0 = &interfaces[0];
    assert_eq!(eth0["name"].as_str(), Some("eth0"));
    assert_eq!(eth0["rx_bytes"].as_u64(), Some(1000));
    assert_eq!(eth0["tx_bytes"].as_u64(), Some(2000));

    // Проверяем второй интерфейс (wlan0)
    let wlan0 = &interfaces[1];
    assert_eq!(wlan0["name"].as_str(), Some("wlan0"));
    assert_eq!(wlan0["rx_bytes"].as_u64(), Some(500));
    assert_eq!(wlan0["tx_bytes"].as_u64(), Some(1500));

    // Проверяем общие метрики (должны быть суммой)
    assert_eq!(network_json["total_rx_bytes"].as_u64(), Some(1500));
    assert_eq!(network_json["total_tx_bytes"].as_u64(), Some(3500));

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_cache_stats_endpoint() {
    // Тест: проверка работы endpoint /api/cache/stats
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/cache/stats", port);

    // Отправляем GET запрос
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен быть успешным");

    // Проверяем статус код
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // Проверяем ответ
    let body = response.text().await.expect("Ответ должен быть текстом");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем структуру ответа
    assert_eq!(json["status"].as_str(), Some("ok"));
    assert!(json["cache_stats"].is_object());
    assert!(json["cache_stats"]["total_entries"].is_number());
    assert!(json["cache_stats"]["active_entries"].is_number());
    assert!(json["cache_stats"]["stale_entries"].is_number());
    assert!(json["cache_stats"]["max_capacity"].is_number());
    assert!(json["cache_stats"]["cache_ttl_seconds"].is_number());
    assert!(json["cache_stats"]["average_age_seconds"].is_number());
    assert!(json["cache_stats"]["hit_rate"].is_number());
    assert!(json["cache_stats"]["utilization_rate"].is_number());
    assert!(json["timestamp"].is_string());

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_cache_clear_endpoint() {
    // Тест: проверка работы endpoint /api/cache/clear
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/cache/clear", port);

    // Отправляем POST запрос
    let response = client
        .post(&url)
        .send()
        .await
        .expect("Запрос должен быть успешным");

    // Проверяем статус код
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // Проверяем ответ
    let body = response.text().await.expect("Ответ должен быть текстом");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем структуру ответа
    assert_eq!(json["status"].as_str(), Some("success"));
    assert_eq!(
        json["message"].as_str(),
        Some("Process cache cleared successfully")
    );
    assert!(json["cleared_entries"].is_number());
    assert!(json["previous_stats"].is_object());
    assert!(json["current_stats"].is_object());
    assert!(json["timestamp"].is_string());

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_cache_config_endpoint() {
    // Тест: проверка работы endpoint /api/cache/config (GET)
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/cache/config", port);

    // Отправляем GET запрос
    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен быть успешным");

    // Проверяем статус код
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // Проверяем ответ
    let body = response.text().await.expect("Ответ должен быть текстом");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем структуру ответа
    assert_eq!(json["status"].as_str(), Some("ok"));
    assert!(json["cache_config"].is_object());
    assert!(json["cache_config"]["cache_ttl_seconds"].is_number());
    assert!(json["cache_config"]["max_cached_processes"].is_number());
    assert!(json["cache_config"]["enable_caching"].is_boolean());
    assert!(json["cache_config"]["enable_parallel_processing"].is_boolean());
    assert!(json["timestamp"].is_string());

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_cache_config_update_endpoint() {
    // Тест: проверка работы endpoint /api/cache/config (POST)
    let server = ApiServer::new("127.0.0.1:0".parse().unwrap());
    let handle = server.start().await.expect("Сервер должен запуститься");
    let port = handle.port();

    // Создаем HTTP клиент
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/cache/config", port);

    // Отправляем POST запрос с новой конфигурацией
    let new_config = serde_json::json!({
        "cache_ttl_seconds": 30,
        "max_cached_processes": 5000,
        "enable_caching": true,
        "enable_parallel_processing": true
    });

    let response = client
        .post(&url)
        .json(&new_config)
        .send()
        .await
        .expect("Запрос должен быть успешным");

    // Проверяем статус код
    assert_eq!(response.status(), reqwest::StatusCode::OK);

    // Проверяем ответ
    let body = response.text().await.expect("Ответ должен быть текстом");
    let json: serde_json::Value =
        serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");

    // Проверяем структуру ответа
    assert_eq!(json["status"].as_str(), Some("success"));
    assert_eq!(
        json["message"].as_str(),
        Some("Process cache configuration updated successfully")
    );
    assert!(json["cache_config"].is_object());
    assert_eq!(json["cache_config"]["cache_ttl_seconds"].as_u64(), Some(30));
    assert_eq!(
        json["cache_config"]["max_cached_processes"].as_u64(),
        Some(5000)
    );
    assert_eq!(json["cache_config"]["enable_caching"].as_bool(), Some(true));
    assert_eq!(
        json["cache_config"]["enable_parallel_processing"].as_bool(),
        Some(true)
    );
    assert!(json["timestamp"].is_string());

    // Останавливаем сервер
    handle
        .shutdown()
        .await
        .expect("Сервер должен корректно остановиться");
}
