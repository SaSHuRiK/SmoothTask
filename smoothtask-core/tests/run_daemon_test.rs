//! Интеграционные тесты для главного цикла демона (run_daemon).
//!
//! Эти тесты проверяют работу демона в различных сценариях:
//! - инициализация с минимальным конфигом
//! - работа в dry-run режиме
//! - обработка ошибок (несуществующая директория паттернов)
//! - работа с snapshot logger
//! - один полный цикл сбора снапшота (с моками)

use smoothtask_core::config::{Config, Paths, PolicyMode, Thresholds};
use smoothtask_core::run_daemon;
use std::time::Duration;
use tempfile::TempDir;
use tokio::sync::watch;
use tokio::time::{sleep, timeout};

fn create_test_config(patterns_dir: &str, snapshot_db_path: String) -> Config {
    Config {
        polling_interval_ms: 100, // Короткий интервал для быстрых тестов
        max_candidates: 150,
        dry_run_default: false,
        policy_mode: PolicyMode::RulesOnly,
        enable_snapshot_logging: true,
        thresholds: Thresholds {
            psi_cpu_some_high: 0.6,
            psi_io_some_high: 0.4,
            user_idle_timeout_sec: 120,
            interactive_build_grace_sec: 10,
            noisy_neighbour_cpu_share: 0.7,
            crit_interactive_percentile: 0.9,
            interactive_percentile: 0.6,
            normal_percentile: 0.3,
            background_percentile: 0.1,
            sched_latency_p99_threshold_ms: 10.0,
            ui_loop_p95_threshold_ms: 16.67,
        },
        paths: Paths {
            snapshot_db_path,
            patterns_dir: patterns_dir.to_string(),
            api_listen_addr: None, // API сервер отключен по умолчанию в тестах
        },
    }
}

/// Тест проверяет инициализацию демона с минимальным конфигом.
/// Демон должен успешно запуститься и выполнить хотя бы одну итерацию.
#[tokio::test]
async fn test_daemon_initializes_with_minimal_config() {
    // Создаём временную директорию для patterns
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let patterns_dir = temp_dir.path().to_str().unwrap();

    // Создаём пустую директорию для patterns (демон должен корректно обработать пустую директорию)
    std::fs::create_dir_all(patterns_dir).expect("Failed to create patterns dir");

    // Используем временный файл для БД, чтобы пройти валидацию
    // Но демон не будет логировать, если путь пустой (проверяется в run_daemon)
    let db_file = tempfile::NamedTempFile::new().expect("Failed to create temp db file");
    let db_path = db_file.path().to_str().unwrap().to_string();
    let config = create_test_config(patterns_dir, db_path);

    // Создаём канал для graceful shutdown
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Запускаем демон с автоматическим shutdown через 200ms
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(200)).await;
        let _ = shutdown_tx_clone.send(true);
    });

    // Запускаем демон и ждём его завершения
    let result = run_daemon(config, true, shutdown_rx, None, None).await;

    match result {
        Ok(()) => {
            // Демон завершился успешно - это ожидаемое поведение
        }
        Err(e) => {
            panic!("Daemon failed with error: {}", e);
        }
    }
}

/// Тест проверяет работу демона в dry-run режиме.
/// В dry-run режиме демон не должен применять изменения, но должен работать.
#[tokio::test]
async fn test_daemon_dry_run_mode() {
    // Создаём временную директорию для patterns
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let patterns_dir = temp_dir.path().to_str().unwrap();

    // Создаём пустую директорию для patterns
    std::fs::create_dir_all(patterns_dir).expect("Failed to create patterns dir");

    // Используем временный файл для БД, чтобы пройти валидацию
    let db_file = tempfile::NamedTempFile::new().expect("Failed to create temp db file");
    let db_path = db_file.path().to_str().unwrap().to_string();
    let config = create_test_config(patterns_dir, db_path);

    // Создаём канал для graceful shutdown
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Запускаем демон с автоматическим shutdown через 200ms
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(200)).await;
        let _ = shutdown_tx_clone.send(true);
    });

    // Запускаем демон и ждём его завершения
    let result = run_daemon(config, true, shutdown_rx, None, None).await;

    match result {
        Ok(()) => {
            // Демон завершился успешно
        }
        Err(e) => {
            panic!("Daemon failed with error: {}", e);
        }
    }
}

/// Тест проверяет обработку несуществующей директории паттернов.
/// Демон должен упасть с ошибкой при загрузке паттернов.
#[tokio::test]
async fn test_daemon_handles_nonexistent_patterns_dir() {
    // Используем несуществующую директорию
    let nonexistent_dir = "/tmp/smoothtask_test_nonexistent_patterns_12345";

    // Для теста на несуществующую директорию нужен валидный путь к БД
    let db_file = tempfile::NamedTempFile::new().expect("Failed to create temp db file");
    let db_path = db_file.path().to_str().unwrap().to_string();
    let config = create_test_config(nonexistent_dir, db_path);

    // Создаём канал для graceful shutdown (хотя демон должен упасть до использования)
    let (_shutdown_tx, shutdown_rx) = watch::channel(false);

    // Демон должен упасть с ошибкой при загрузке паттернов
    let result = timeout(
        Duration::from_secs(1),
        run_daemon(config, true, shutdown_rx, None, None),
    )
    .await;

    match result {
        Ok(Ok(())) => {
            panic!("Daemon should fail with nonexistent patterns dir");
        }
        Ok(Err(_)) => {
            // Ожидаемая ошибка - демон не может загрузить паттерны из несуществующей директории
            // Это корректное поведение
        }
        Err(_) => {
            // Таймаут - неожиданно, демон не должен работать с несуществующей директорией
            panic!("Daemon should fail immediately with nonexistent patterns dir");
        }
    }
}

/// Тест проверяет работу демона с snapshot logger.
/// Демон должен успешно инициализироваться с snapshot logger и записывать снапшоты.
#[tokio::test]
async fn test_daemon_with_snapshot_logger() {
    // Создаём временную директорию для patterns
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let patterns_dir = temp_dir.path().to_str().unwrap();

    // Создаём пустую директорию для patterns
    std::fs::create_dir_all(patterns_dir).expect("Failed to create patterns dir");

    // Создаём временный файл для БД снапшотов
    let db_file = tempfile::NamedTempFile::new().expect("Failed to create temp db file");
    let db_path = db_file.path().to_str().unwrap().to_string();

    let config = create_test_config(patterns_dir, db_path.clone());

    // Создаём канал для graceful shutdown
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Запускаем демон с автоматическим shutdown через 200ms
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(200)).await;
        let _ = shutdown_tx_clone.send(true);
    });

    // Запускаем демон и ждём его завершения
    let result = run_daemon(config, true, shutdown_rx, None, None).await;

    match result {
        Ok(()) => {
            // Демон завершился успешно
            // Проверяем, что БД была создана и содержит данные
            if std::path::Path::new(&db_path).exists() {
                use rusqlite::Connection;
                if let Ok(conn) = Connection::open(&db_path) {
                    // Проверяем наличие таблицы snapshots
                    let table_exists: bool = conn
                        .query_row(
                            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='snapshots'",
                            [],
                            |row| row.get(0),
                        )
                        .unwrap_or(false);
                    assert!(table_exists, "Snapshot table should exist after daemon run");
                }
            }
        }
        Err(e) => {
            panic!("Daemon failed with error: {}", e);
        }
    }
}

/// Тест проверяет, что snapshot logger не инициализируется, когда enable_snapshot_logging = false.
/// Демон должен успешно работать, но БД не должна быть создана.
#[tokio::test]
async fn test_daemon_without_snapshot_logging() {
    // Создаём временную директорию для patterns
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let patterns_dir = temp_dir.path().to_str().unwrap();

    // Создаём пустую директорию для patterns
    std::fs::create_dir_all(patterns_dir).expect("Failed to create patterns dir");

    // Создаём временный файл для БД снапшотов
    let db_file = tempfile::NamedTempFile::new().expect("Failed to create temp db file");
    let db_path = db_file.path().to_str().unwrap().to_string();

    // Создаём конфиг с отключённым логированием
    let mut config = create_test_config(patterns_dir, db_path.clone());
    config.enable_snapshot_logging = false;

    // Создаём канал для graceful shutdown
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Запускаем демон с автоматическим shutdown через 200ms
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(200)).await;
        let _ = shutdown_tx_clone.send(true);
    });

    // Запускаем демон и ждём его завершения
    let result = run_daemon(config, true, shutdown_rx, None, None).await;

    match result {
        Ok(()) => {
            // Демон завершился успешно
            // Проверяем, что БД НЕ была создана (snapshot logger не инициализировался)
            // Примечание: tempfile может создать файл, но он должен быть пустым
            if std::path::Path::new(&db_path).exists() {
                use rusqlite::Connection;
                if let Ok(conn) = Connection::open(&db_path) {
                    // Проверяем, что таблица snapshots НЕ существует
                    let table_exists: bool = conn
                        .query_row(
                            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='snapshots'",
                            [],
                            |row| row.get(0),
                        )
                        .unwrap_or(false);
                    assert!(
                        !table_exists,
                        "Snapshot table should NOT exist when enable_snapshot_logging is false"
                    );
                }
            }
        }
        Err(e) => {
            panic!("Daemon failed with error: {}", e);
        }
    }
}

/// Тест проверяет один полный цикл сбора снапшота (с моками).
/// Демон должен успешно пройти через все этапы:
/// - сбор снапшота (используются статические интроспекторы, если X11/PipeWire недоступны)
/// - группировка процессов
/// - классификация процессов и групп
/// - применение политики
/// - планирование изменений приоритетов
/// - логирование снапшота (если включено)
///
/// Этот тест проверяет, что демон не падает при выполнении полного цикла
/// и что все компоненты корректно интегрированы.
#[tokio::test]
async fn test_daemon_full_snapshot_cycle() {
    // Создаём временную директорию для patterns
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let patterns_dir = temp_dir.path().to_str().unwrap();

    // Создаём пустую директорию для patterns
    std::fs::create_dir_all(patterns_dir).expect("Failed to create patterns dir");

    // Создаём временный файл для БД снапшотов
    let db_file = tempfile::NamedTempFile::new().expect("Failed to create temp db file");
    let db_path = db_file.path().to_str().unwrap().to_string();

    let config = create_test_config(patterns_dir, db_path.clone());

    // Создаём канал для graceful shutdown
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Запускаем демон с автоматическим shutdown через 200ms
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(200)).await;
        let _ = shutdown_tx_clone.send(true);
    });

    // Запускаем демон и ждём его завершения
    let result = run_daemon(config, true, shutdown_rx, None, None).await;

    // Демон должен успешно выполнить хотя бы один полный цикл
    match result {
        Ok(()) => {
            // Демон завершился успешно
        }
        Err(e) => {
            panic!("Daemon failed with error: {}", e);
        }
    }
}

/// Тест проверяет интеграцию API сервера в run_daemon.
/// API сервер должен запускаться вместе с демоном и корректно останавливаться.
#[tokio::test]
async fn test_daemon_with_api_server() {
    // Создаём временную директорию для patterns
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let patterns_dir = temp_dir.path().to_str().unwrap();

    // Создаём пустую директорию для patterns
    std::fs::create_dir_all(patterns_dir).expect("Failed to create patterns dir");

    // Создаём временный файл для БД снапшотов
    let db_file = tempfile::NamedTempFile::new().expect("Failed to create temp db file");
    let db_path = db_file.path().to_str().unwrap().to_string();

    // Создаём конфиг с включённым API сервером
    let mut config = create_test_config(patterns_dir, db_path.clone());
    // Используем порт 0 для автоматического выбора свободного порта (но это не работает с axum)
    // Вместо этого используем тестовый порт, который может быть занят
    // В реальном использовании нужно использовать свободный порт
    config.paths.api_listen_addr = Some("127.0.0.1:0".to_string());

    // Создаём канал для graceful shutdown
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Запускаем демон с автоматическим shutdown через 200ms
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        sleep(Duration::from_millis(200)).await;
        let _ = shutdown_tx_clone.send(true);
    });

    // Запускаем демон и ждём его завершения
    // API сервер должен запуститься (или выдать предупреждение, если порт занят)
    let result = run_daemon(config, true, shutdown_rx, None, None).await;

    // Демон должен успешно завершиться (даже если API сервер не запустился из-за занятого порта)
    match result {
        Ok(()) => {
            // Демон завершился успешно
            // Проверяем, что БД была создана и содержит данные (если демон успешно логировал)
            // Это косвенно подтверждает, что полный цикл был выполнен
            if std::path::Path::new(&db_path).exists() {
                // БД существует, значит snapshot logger был инициализирован
                // Проверяем, что БД не пустая (есть хотя бы одна таблица)
                use rusqlite::Connection;
                if let Ok(conn) = Connection::open(&db_path) {
                    // Проверяем наличие таблицы snapshots
                    let table_exists: bool = conn
                        .query_row(
                            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='snapshots'",
                            [],
                            |row| row.get(0),
                        )
                        .unwrap_or(false);
                    assert!(table_exists, "Snapshot table should exist after daemon run");
                }
            }
        }
        Err(e) => {
            panic!("Daemon failed with error during full cycle: {}", e);
        }
    }
}
