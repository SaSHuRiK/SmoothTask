// Интеграционные тесты для мониторинга и метрик
//
// Эти тесты проверяют работу системы мониторинга, включая:
// - Prometheus метрики
// - Интеграцию с Grafana
// - Alerting правила
// - Критические метрики здоровья системы

use smoothtask_core::{
    api::ApiServer,
    metrics::custom::{CustomMetricConfig, CustomMetricType, CustomMetricsManager},
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

#[tokio::test]
async fn test_prometheus_custom_metrics_integration() {
    // Тест: проверка интеграции пользовательских метрик в Prometheus
    use smoothtask_core::api::ApiStateBuilder;

    // Создаём менеджер пользовательских метрик с тестовыми метриками
    let custom_metrics_manager = Arc::new(CustomMetricsManager::new());

    // Добавляем тестовые метрики
    let test_configs = vec![
        CustomMetricConfig {
            id: "test_integer_metric".to_string(),
            name: "Test Integer Metric".to_string(),
            metric_type: CustomMetricType::Integer,
            source: smoothtask_core::metrics::custom::CustomMetricSource::Static {
                value: "42".to_string(),
            },
            update_interval_secs: 60,
            enabled: true,
        },
        CustomMetricConfig {
            id: "test_float_metric".to_string(),
            name: "Test Float Metric".to_string(),
            metric_type: CustomMetricType::Float,
            source: smoothtask_core::metrics::custom::CustomMetricSource::Static {
                value: "3.14".to_string(),
            },
            update_interval_secs: 60,
            enabled: true,
        },
        CustomMetricConfig {
            id: "test_boolean_metric".to_string(),
            name: "Test Boolean Metric".to_string(),
            metric_type: CustomMetricType::Boolean,
            source: smoothtask_core::metrics::custom::CustomMetricSource::Static {
                value: "true".to_string(),
            },
            update_interval_secs: 60,
            enabled: true,
        },
        CustomMetricConfig {
            id: "test_string_metric".to_string(),
            name: "Test String Metric".to_string(),
            metric_type: CustomMetricType::String,
            source: smoothtask_core::metrics::custom::CustomMetricSource::Static {
                value: "hello".to_string(),
            },
            update_interval_secs: 60,
            enabled: true,
        },
    ];

    // Инициализируем метрики
    for config in test_configs {
        custom_metrics_manager.add_metric(config).await.ok();
    }

    // Создаём состояние API с менеджером пользовательских метрик
    let state = ApiStateBuilder::new()
        .with_daemon_stats(Some(Arc::new(RwLock::new(DaemonStats::new()))))
        .with_custom_metrics_manager(Some(Arc::clone(&custom_metrics_manager)))
        .build();

    // Создаём сервер с кастомным состоянием
    let server = ApiServer::with_state("127.0.0.1:0".parse().unwrap(), state);

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

    // Проверяем, что пользовательские метрики присутствуют в Prometheus формате
    assert!(
        body.contains("smoothtask_custom_metric"),
        "Должны быть пользовательские метрики"
    );
    assert!(
        body.contains("smoothtask_custom_metrics_total"),
        "Должна быть метрика общего количества"
    );

    // Проверяем, что каждая тестовая метрика присутствует
    assert!(
        body.contains("test_integer_metric"),
        "Должна быть integer метрика"
    );
    assert!(
        body.contains("test_float_metric"),
        "Должна быть float метрика"
    );
    assert!(
        body.contains("test_boolean_metric"),
        "Должна быть boolean метрика"
    );
    assert!(
        body.contains("test_string_metric"),
        "Должна быть string метрика"
    );

    // Проверяем формат Prometheus для пользовательских метрик
    assert!(
        body.contains("# HELP smoothtask_custom_metric"),
        "Должны быть HELP комментарии для пользовательских метрик"
    );
    assert!(
        body.contains("# TYPE smoothtask_custom_metric"),
        "Должны быть TYPE комментарии для пользовательских метрик"
    );

    // Проверяем метрики статуса
    assert!(
        body.contains("smoothtask_custom_metric_status"),
        "Должны быть метрики статуса"
    );
    assert!(
        body.contains("smoothtask_custom_metric_last_update"),
        "Должны быть метрики времени последнего обновления"
    );

    // Останавливаем сервер
    let _ = handle.shutdown().await;
}
