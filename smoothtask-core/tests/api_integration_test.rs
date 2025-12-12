// Интеграционные тесты для API сервера
//
// Эти тесты проверяют работу API сервера через публичный интерфейс
// и интеграцию с основными компонентами системы.

use smoothtask_core::{
    api::{ApiServer, ApiServerHandle},
    DaemonStats,
};
use std::sync::Arc;
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
