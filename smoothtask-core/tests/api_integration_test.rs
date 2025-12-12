// Интеграционные тесты для API сервера
//
// Эти тесты проверяют работу API сервера через публичный интерфейс
// и интеграцию с основными компонентами системы.

use smoothtask_core::{
    api::{ApiServer, ApiServerHandle, ApiState},
    metrics::system::{SystemMetrics, NetworkMetrics, NetworkInterface},
    DaemonStats,
};
use std::sync::Arc;
use tokio::sync::RwLock;

// Импорты для HTTP тестирования
use reqwest;
use serde_json;
use tokio::sync::RwLock;

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
    use smoothtask_core::metrics::system::{collect_system_metrics, CpuTimes, LoadAvg, MemoryInfo, PressureMetrics};
    
    // Создаем тестовые сетевые метрики
    let mut network_metrics = NetworkMetrics::default();
    network_metrics.interfaces.push(NetworkInterface {
        name: "eth0".to_string(),
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
    
    let api_state = ApiState::new(
        Some(daemon_stats),
        Some(system_metrics_arc),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    
    let server = ApiServer::with_api_state("127.0.0.1:0".parse().unwrap(), api_state);
    let handle = server.start().await.expect("Сервер должен запуститься");
    
    // Получаем порт, на котором запустился сервер
    let port = handle.port();
    
    // Делаем HTTP запрос к API для получения метрик
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/metrics", port);
    
    let response = client.get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться успешно");
    
    assert!(response.status().is_success());
    
    let body = response.text().await.expect("Ответ должен содержать текст");
    let json: serde_json::Value = serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");
    
    // Проверяем, что ответ содержит сетевые метрики
    assert!(json["status"].as_str() == Some("ok"));
    assert!(json["system_metrics"].is_object());
    
    let system_metrics_json = &json["system_metrics"];
    assert!(system_metrics_json["network"].is_object());
    
    let network_json = &system_metrics_json["network"];
    assert!(network_json["interfaces"].is_array());
    assert!(network_json["total_rx_bytes"].is_number());
    assert!(network_json["total_tx_bytes"].is_number());
    
    // Проверяем, что интерфейсы содержат ожидаемые данные
    let interfaces = network_json["interfaces"].as_array().unwrap();
    assert!(!interfaces.is_empty());
    
    let first_interface = &interfaces[0];
    assert_eq!(first_interface["name"].as_str(), Some("eth0"));
    assert_eq!(first_interface["rx_bytes"].as_u64(), Some(1000));
    assert_eq!(first_interface["tx_bytes"].as_u64(), Some(2000));
    assert_eq!(first_interface["rx_packets"].as_u64(), Some(100));
    assert_eq!(first_interface["tx_packets"].as_u64(), Some(200));
    assert_eq!(first_interface["rx_errors"].as_u64(), Some(1));
    assert_eq!(first_interface["tx_errors"].as_u64(), Some(2));
    
    // Проверяем общие метрики
    assert_eq!(network_json["total_rx_bytes"].as_u64(), Some(1000));
    assert_eq!(network_json["total_tx_bytes"].as_u64(), Some(2000));
    
    // Останавливаем сервер
    handle.shutdown().await.expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_network_metrics_empty() {
    // Тест: проверка, что API корректно обрабатывает пустые сетевые метрики
    use smoothtask_core::metrics::system::{collect_system_metrics, CpuTimes, LoadAvg, MemoryInfo, PressureMetrics};
    
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
    
    let api_state = ApiState::new(
        Some(daemon_stats),
        Some(system_metrics_arc),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    
    let server = ApiServer::with_api_state("127.0.0.1:0".parse().unwrap(), api_state);
    let handle = server.start().await.expect("Сервер должен запуститься");
    
    // Получаем порт, на котором запустился сервер
    let port = handle.port();
    
    // Делаем HTTP запрос к API для получения метрик
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/metrics", port);
    
    let response = client.get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться успешно");
    
    assert!(response.status().is_success());
    
    let body = response.text().await.expect("Ответ должен содержать текст");
    let json: serde_json::Value = serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");
    
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
    handle.shutdown().await.expect("Сервер должен корректно остановиться");
}

#[tokio::test]
async fn test_network_metrics_multiple_interfaces() {
    // Тест: проверка, что API корректно обрабатывает несколько сетевых интерфейсов
    use smoothtask_core::metrics::system::{collect_system_metrics, CpuTimes, LoadAvg, MemoryInfo, PressureMetrics};
    
    // Создаем тестовые сетевые метрики с несколькими интерфейсами
    let mut network_metrics = NetworkMetrics::default();
    
    // Добавляем первый интерфейс
    network_metrics.interfaces.push(NetworkInterface {
        name: "eth0".to_string(),
        rx_bytes: 1000,
        tx_bytes: 2000,
        rx_packets: 100,
        tx_packets: 200,
        rx_errors: 1,
        tx_errors: 2,
    });
    
    // Добавляем второй интерфейс
    network_metrics.interfaces.push(NetworkInterface {
        name: "wlan0".to_string(),
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
    
    let api_state = ApiState::new(
        Some(daemon_stats),
        Some(system_metrics_arc),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
    );
    
    let server = ApiServer::with_api_state("127.0.0.1:0".parse().unwrap(), api_state);
    let handle = server.start().await.expect("Сервер должен запуститься");
    
    // Получаем порт, на котором запустился сервер
    let port = handle.port();
    
    // Делаем HTTP запрос к API для получения метрик
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/api/metrics", port);
    
    let response = client.get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться успешно");
    
    assert!(response.status().is_success());
    
    let body = response.text().await.expect("Ответ должен содержать текст");
    let json: serde_json::Value = serde_json::from_str(&body).expect("Ответ должен быть валидным JSON");
    
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
    handle.shutdown().await.expect("Сервер должен корректно остановиться");
}
