// Интеграционные тесты для мониторинга и метрик
//
// Эти тесты проверяют работу системы мониторинга, включая:
// - Prometheus метрики
// - Интеграцию с Grafana
// - Alerting правила
// - Критические метрики здоровья системы

use smoothtask_core::{
    api::ApiServer,
    DaemonStats,
};
use std::sync::Arc;
use tokio::sync::RwLock;

#[tokio::test]
async fn test_prometheus_metrics_endpoint_comprehensive() {
    // Тест: комплексная проверка Prometheus метрик
    let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
    let server = ApiServer::with_daemon_stats("127.0.0.1:0".parse().unwrap(), daemon_stats);

    // Запускаем сервер
    let handle_result = server.start().await;
    assert!(handle_result.is_ok());
    let handle = handle_result.unwrap();

    // Получаем порт
    let port = handle.port();

    // Делаем HTTP запрос к Prometheus метрикам
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/metrics", port);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться успешно");

    assert!(response.status().is_success());

    let body = response.text().await.expect("Ответ должен содержать текст");

    // Проверяем, что ответ содержит основные метрики
    assert!(body.contains("smoothtask_version"));
    assert!(body.contains("smoothtask_health_score"));
    assert!(body.contains("smoothtask_system_memory"));
    assert!(body.contains("smoothtask_processes_total"));
    assert!(body.contains("smoothtask_app_groups_total"));
    assert!(body.contains("smoothtask_daemon_iteration_time"));

    // Проверяем формат Prometheus
    assert!(body.contains("# HELP"));
    assert!(body.contains("# TYPE"));

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_critical_health_metrics() {
    // Тест: проверка критических метрик здоровья
    let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
    let server = ApiServer::with_daemon_stats("127.0.0.1:0".parse().unwrap(), daemon_stats);

    // Запускаем сервер
    let handle_result = server.start().await;
    assert!(handle_result.is_ok());
    let handle = handle_result.unwrap();

    // Получаем порт
    let port = handle.port();

    // Делаем HTTP запрос к метрикам
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/metrics", port);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться успешно");

    assert!(response.status().is_success());

    let body = response.text().await.expect("Ответ должен содержать текст");

    // Проверяем критические метрики здоровья
    assert!(body.contains("smoothtask_health_score"));
    assert!(body.contains("smoothtask_health_critical_issues"));
    assert!(body.contains("smoothtask_health_degraded_issues"));

    // Проверяем метрики PSI (Pressure Stall Information)
    assert!(body.contains("smoothtask_system_psi"));

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_audio_monitoring_metrics() {
    // Тест: проверка метрик аудио мониторинга
    let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
    let server = ApiServer::with_daemon_stats("127.0.0.1:0".parse().unwrap(), daemon_stats);

    // Запускаем сервер
    let handle_result = server.start().await;
    assert!(handle_result.is_ok());
    let handle = handle_result.unwrap();

    // Получаем порт
    let port = handle.port();

    // Делаем HTTP запрос к метрикам
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/metrics", port);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться успешно");

    assert!(response.status().is_success());

    let body = response.text().await.expect("Ответ должен содержать текст");

    // Проверяем метрики аудио
    assert!(body.contains("smoothtask_audio_clients_total"));
    assert!(body.contains("smoothtask_audio_xruns_total"));
    assert!(body.contains("smoothtask_audio_latency_ms"));

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_network_monitoring_metrics() {
    // Тест: проверка метрик сетевого мониторинга
    let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
    let server = ApiServer::with_daemon_stats("127.0.0.1:0".parse().unwrap(), daemon_stats);

    // Запускаем сервер
    let handle_result = server.start().await;
    assert!(handle_result.is_ok());
    let handle = handle_result.unwrap();

    // Получаем порт
    let port = handle.port();

    // Делаем HTTP запрос к метрикам
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/metrics", port);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться успешно");

    assert!(response.status().is_success());

    let body = response.text().await.expect("Ответ должен содержать текст");

    // Проверяем метрики сети
    assert!(body.contains("smoothtask_network_receive_bytes"));
    assert!(body.contains("smoothtask_network_transmit_bytes"));
    assert!(body.contains("smoothtask_network_receive_errors"));
    assert!(body.contains("smoothtask_network_transmit_errors"));

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_daemon_performance_metrics() {
    // Тест: проверка метрик производительности демона
    let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
    let server = ApiServer::with_daemon_stats("127.0.0.1:0".parse().unwrap(), daemon_stats);

    // Запускаем сервер
    let handle_result = server.start().await;
    assert!(handle_result.is_ok());
    let handle = handle_result.unwrap();

    // Получаем порт
    let port = handle.port();

    // Делаем HTTP запрос к метрикам
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/metrics", port);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться успешно");

    assert!(response.status().is_success());

    let body = response.text().await.expect("Ответ должен содержать текст");

    // Проверяем метрики производительности демона
    assert!(body.contains("smoothtask_daemon_iteration_time_seconds"));
    assert!(body.contains("smoothtask_daemon_priority_adjustments_total"));
    assert!(body.contains("smoothtask_daemon_total_applied_adjustments"));
    assert!(body.contains("smoothtask_daemon_iterations_total"));

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_metrics_format_compliance() {
    // Тест: проверка соответствия формату Prometheus
    let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
    let server = ApiServer::with_daemon_stats("127.0.0.1:0".parse().unwrap(), daemon_stats);

    // Запускаем сервер
    let handle_result = server.start().await;
    assert!(handle_result.is_ok());
    let handle = handle_result.unwrap();

    // Получаем порт
    let port = handle.port();

    // Делаем HTTP запрос к метрикам
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/metrics", port);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Запрос должен выполниться успешно");

    assert!(response.status().is_success());

    let body = response.text().await.expect("Ответ должен содержать текст");

    // Проверяем соответствие формату Prometheus
    // Каждая метрика должна иметь HELP и TYPE комментарии
    let lines: Vec<&str> = body.lines().collect();
    let mut metric_count = 0;
    let mut help_count = 0;
    let mut type_count = 0;

    for line in lines {
        if line.starts_with("smoothtask_") && !line.starts_with("#") {
            metric_count += 1;
        } else if line.starts_with("# HELP") {
            help_count += 1;
        } else if line.starts_with("# TYPE") {
            type_count += 1;
        }
    }

    // Каждая метрика должна иметь соответствующие HELP и TYPE
    assert!(help_count > 0, "Должны быть HELP комментарии");
    assert!(type_count > 0, "Должны быть TYPE комментарии");
    assert!(metric_count > 0, "Должны быть метрики");

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}

#[tokio::test]
async fn test_metrics_endpoint_performance() {
    // Тест: проверка производительности endpoints метрик
    let daemon_stats = Arc::new(RwLock::new(DaemonStats::new()));
    let server = ApiServer::with_daemon_stats("127.0.0.1:0".parse().unwrap(), daemon_stats);

    // Запускаем сервер
    let handle_result = server.start().await;
    assert!(handle_result.is_ok());
    let handle = handle_result.unwrap();

    // Получаем порт
    let port = handle.port();

    // Делаем несколько последовательных запросов для проверки производительности
    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}/metrics", port);

    let start_time = std::time::Instant::now();
    
    // Делаем 5 последовательных запросов
    for _ in 0..5 {
        let response = client
            .get(&url)
            .send()
            .await
            .expect("Запрос должен выполниться успешно");

        assert!(response.status().is_success());
        let _ = response.text().await.expect("Ответ должен содержать текст");
    }

    let duration = start_time.elapsed();
    
    // Запросы должны выполняться быстро (менее 1 секунды на 5 запросов)
    assert!(duration.as_secs() < 1, "Метрики должны возвращаться быстро");

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}