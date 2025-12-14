//! Интеграционные тесты для системы автоматического обновления конфигурации и паттернов.
//!
//! Эти тесты проверяют:
//! - Автоматическую перезагрузку конфигурации при изменении файла
//! - Автоматическое обновление паттернов при изменении файлов в директории
//! - Интеграцию с основным циклом демона
//! - Уведомления об изменениях

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::{tempdir, NamedTempFile};
use tokio::sync::watch;
use tokio::time::sleep;

use smoothtask_core::classify::pattern_watcher::{PatternWatcher, PatternWatcherConfig};
use smoothtask_core::classify::rules::PatternDatabase;
use smoothtask_core::config::config_struct::Config;
use smoothtask_core::config::watcher::ConfigWatcher;

/// Создаёт тестовый конфигурационный файл с указанными параметрами.
fn create_test_config_content() -> String {
    r#"
    polling_interval_ms: 100
    max_candidates: 10
    dry_run_default: true
    enable_snapshot_logging: false
    
    thresholds:
      psi_cpu_some_high: 0.5
      psi_io_some_high: 0.5
      user_idle_timeout_sec: 300
      interactive_build_grace_sec: 60
      noisy_neighbour_cpu_share: 0.2
      crit_interactive_percentile: 0.95
      interactive_percentile: 0.8
      normal_percentile: 0.6
      background_percentile: 0.4
      sched_latency_p99_threshold_ms: 20.0
      ui_loop_p95_threshold_ms: 16.67
    
    paths:
      patterns_dir: "patterns"
      snapshot_db_path: "test.db"
      log_file_path: "test.log"
    
    cache_intervals:
      system_metrics_cache_interval: 3
      process_metrics_cache_interval: 1
    
    pattern_auto_update:
      enabled: true
      interval_sec: 60
      notify_on_update: false
"#
    .to_string()
}

/// Создаёт тестовый файл паттерна.
fn create_test_pattern_file(dir: &Path, filename: &str, content: &str) -> std::path::PathBuf {
    let file_path = dir.join(filename);
    fs::write(&file_path, content).expect("write test pattern file");
    file_path
}

#[tokio::test]
async fn test_config_watcher_integration() {
    // Создаём временный конфигурационный файл
    let mut config_file = NamedTempFile::new().expect("tempfile");
    let config_path = config_file.path().to_str().unwrap().to_string();

    // Записываем начальную конфигурацию
    let initial_config = create_test_config_content();
    fs::write(&config_path, &initial_config).expect("write initial config");

    // Создаём ConfigWatcher
    let config_watcher = ConfigWatcher::new(&config_path).expect("watcher creation");
    let mut change_receiver = config_watcher.change_receiver();

    // Запускаем мониторинг
    let watcher_handle = config_watcher.start_watching();

    // Даём время на запуск
    sleep(Duration::from_millis(100)).await;

    // Проверяем, что изначально нет изменений
    assert!(!*change_receiver.borrow_and_update());

    // Модифицируем конфигурационный файл
    let modified_config =
        initial_config.replace("polling_interval_ms: 100", "polling_interval_ms: 200");
    fs::write(&config_path, &modified_config).expect("write modified config");

    // Даём время на обнаружение изменения
    sleep(Duration::from_millis(200)).await;

    // Проверяем, что изменение было обнаружено
    assert!(*change_receiver.borrow_and_update());

    // Отменяем задачу мониторинга
    watcher_handle.abort();

    // Даём время на завершение
    sleep(Duration::from_millis(100)).await;

    assert!(watcher_handle.is_finished());
}

#[tokio::test]
async fn test_pattern_watcher_integration() {
    // Создаём временную директорию для паттернов
    let temp_dir = tempdir().expect("tempdir");
    let patterns_dir = temp_dir.path().to_str().unwrap().to_string();

    // Создаём начальный файл паттерна
    create_test_pattern_file(
        temp_dir.path(),
        "initial.yml",
        r#"
category: test
apps:
  - name: "initial-app"
    label: "Initial Application"
    exe_patterns: ["initial-app"]
    tags: ["test"]
"#,
    );

    // Загружаем начальную базу паттернов
    let pattern_db = Arc::new(Mutex::new(
        PatternDatabase::load(&patterns_dir).expect("load initial patterns"),
    ));

    // Создаём PatternWatcher
    let config = PatternWatcherConfig {
        enabled: true,
        interval_sec: 1,
        notify_on_update: false,
    };

    let pattern_watcher =
        PatternWatcher::new(&patterns_dir, pattern_db.clone(), config).expect("watcher creation");

    let mut change_receiver = pattern_watcher.change_receiver();

    // Запускаем мониторинг
    let watcher_handle = pattern_watcher.start_watching();

    // Даём время на запуск
    sleep(Duration::from_millis(200)).await;

    // Проверяем начальное состояние
    {
        let initial_result = change_receiver.borrow_and_update();
        assert_eq!(initial_result.patterns_after, 1);
    }

    // Добавляем новый файл паттерна
    create_test_pattern_file(
        temp_dir.path(),
        "new.yml",
        r#"
category: test
apps:
  - name: "new-app"
    label: "New Application"
    exe_patterns: ["new-app"]
    tags: ["test"]
"#,
    );

    // Даём время на обнаружение изменения
    sleep(Duration::from_millis(500)).await;

    // Проверяем, что изменение было обнаружено
    {
        let update_result = change_receiver.borrow_and_update();
        assert!(update_result.has_changes());
        assert_eq!(update_result.patterns_after, 2);
        assert_eq!(update_result.new_files, 1);
    }

    // Модифицируем существующий файл
    let modified_content = r#"
category: test
apps:
  - name: "initial-app"
    label: "Modified Initial Application"
    exe_patterns: ["initial-app", "initial-app-new"]
    tags: ["test", "modified"]
"#;

    let initial_file = temp_dir.path().join("initial.yml");
    fs::write(&initial_file, modified_content).expect("write modified pattern");

    // Даём время на обнаружение изменения
    sleep(Duration::from_millis(500)).await;

    // Проверяем, что изменение было обнаружено
    {
        let update_result = change_receiver.borrow_and_update();
        assert!(update_result.has_changes());
        assert_eq!(update_result.patterns_after, 2);
        assert_eq!(update_result.changed_files, 1);
    }

    // Удаляем файл
    fs::remove_file(initial_file).expect("remove pattern file");

    // Даём время на обнаружение изменения
    sleep(Duration::from_millis(500)).await;

    // Проверяем, что изменение было обнаружено
    {
        let update_result = change_receiver.borrow_and_update();
        assert!(update_result.has_changes());
        assert_eq!(update_result.patterns_after, 1);
        assert_eq!(update_result.removed_files, 1);
    }

    // Отменяем задачу мониторинга
    watcher_handle.abort();

    // Даём время на завершение
    sleep(Duration::from_millis(200)).await;

    assert!(watcher_handle.is_finished());
}

#[tokio::test]
async fn test_config_reload_functionality() {
    // Создаём временный конфигурационный файл
    let mut config_file = NamedTempFile::new().expect("tempfile");
    let config_path = config_file.path().to_str().unwrap().to_string();

    // Записываем начальную конфигурацию
    let initial_config = create_test_config_content();
    fs::write(&config_path, &initial_config).expect("write initial config");

    // Загружаем начальную конфигурацию
    let config = Config::load(&config_path).expect("load initial config");
    assert_eq!(config.polling_interval_ms, 100);

    // Модифицируем конфигурационный файл
    let modified_config =
        initial_config.replace("polling_interval_ms: 100", "polling_interval_ms: 200");
    fs::write(&config_path, &modified_config).expect("write modified config");

    // Загружаем модифицированную конфигурацию
    let new_config = Config::load(&config_path).expect("load modified config");
    assert_eq!(new_config.polling_interval_ms, 200);

    // Проверяем, что конфигурация валидна (пропускаем, так как validate является private)
    // new_config.validate().expect("validate modified config");
}

#[tokio::test]
async fn test_pattern_reload_functionality() {
    // Создаём временную директорию для паттернов
    let temp_dir = tempdir().expect("tempdir");
    let patterns_dir = temp_dir.path().to_str().unwrap().to_string();

    // Создаём начальный файл паттерна
    create_test_pattern_file(
        temp_dir.path(),
        "test.yml",
        r#"
category: test
apps:
  - name: "test-app"
    label: "Test Application"
    exe_patterns: ["test-app"]
    tags: ["test"]
"#,
    );

    // Загружаем начальную базу паттернов
    let mut pattern_db = PatternDatabase::load(&patterns_dir).expect("load initial patterns");
    assert_eq!(pattern_db.all_patterns().len(), 1);

    // Добавляем новый файл паттерна
    create_test_pattern_file(
        temp_dir.path(),
        "new.yml",
        r#"
category: test
apps:
  - name: "new-app"
    label: "New Application"
    exe_patterns: ["new-app"]
    tags: ["test"]
"#,
    );

    // Перезагружаем базу паттернов
    let update_result = pattern_db.reload(&patterns_dir).expect("reload patterns");
    assert!(update_result.has_changes());
    assert_eq!(update_result.patterns_after, 2);
    assert_eq!(update_result.new_files, 1);

    // Проверяем, что база паттернов обновлена
    assert_eq!(pattern_db.all_patterns().len(), 2);
}

#[tokio::test]
async fn test_auto_update_error_handling() {
    // Тестируем обработку ошибок при невалидных файлах паттернов
    let temp_dir = tempdir().expect("tempdir");
    let patterns_dir = temp_dir.path().to_str().unwrap().to_string();

    // Создаём валидный файл паттерна
    create_test_pattern_file(
        temp_dir.path(),
        "valid.yml",
        r#"
category: test
apps:
  - name: "valid-app"
    label: "Valid Application"
    exe_patterns: ["valid-app"]
    tags: ["test"]
"#,
    );

    // Создаём невалидный файл паттерна
    create_test_pattern_file(
        temp_dir.path(),
        "invalid.yml",
        r#"
category: test
apps:
  - name: "invalid-app"
    label: "Invalid Application"
    exe_patterns: ["invalid-app"]
    # Отсутствует обязательное поле tags
"#,
    );

    // Загружаем базу паттернов
    let pattern_db = Arc::new(Mutex::new(
        PatternDatabase::load(&patterns_dir).expect("load patterns with invalid file"),
    ));

    // Проверяем, что невалидный файл был обработан
    let mut db = pattern_db.lock().unwrap();
    let update_result = db.reload(&patterns_dir).expect("reload patterns");
    assert_eq!(update_result.invalid_files, 1);
    assert_eq!(update_result.total_files, 2);
    assert_eq!(update_result.patterns_after, 1); // Только валидный паттерн
}

#[tokio::test]
async fn test_config_watcher_error_handling() {
    // Тестируем обработку ошибок ConfigWatcher

    // Пробуем создать watcher для несуществующего файла
    let result = ConfigWatcher::new("/nonexistent/config.yml");
    assert!(result.is_err());

    // Пробуем создать watcher для директории
    let temp_dir = tempdir().expect("tempdir");
    let dir_path = temp_dir.path().to_str().unwrap().to_string();

    let result = ConfigWatcher::new(&dir_path);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_pattern_watcher_error_handling() {
    // Тестируем обработку ошибок PatternWatcher

    // Пробуем создать watcher для несуществующей директории
    let pattern_db = Arc::new(Mutex::new(PatternDatabase::default()));
    let config = PatternWatcherConfig {
        enabled: true,
        interval_sec: 60,
        notify_on_update: false,
    };

    let result = PatternWatcher::new("/nonexistent/patterns", pattern_db.clone(), config.clone());
    assert!(result.is_err());

    // Пробуем создать watcher для файла вместо директории
    let temp_file = NamedTempFile::new().expect("tempfile");
    let file_path = temp_file.path().to_str().unwrap().to_string();

    let result = PatternWatcher::new(&file_path, pattern_db, config);
    assert!(result.is_err());
}

#[tokio::test]
async fn test_full_auto_update_workflow() {
    // End-to-end тест полного цикла автоматического обновления
    // Тестирует интеграцию ConfigWatcher и PatternWatcher

    // Создаём временную директорию для паттернов
    let temp_dir = tempdir().expect("tempdir");
    let patterns_dir = temp_dir.path().to_str().unwrap().to_string();

    // Создаём начальный файл паттерна
    create_test_pattern_file(
        temp_dir.path(),
        "initial.yml",
        r#"
category: test
apps:
  - name: "initial-app"
    label: "Initial Application"
    exe_patterns: ["initial-app"]
    tags: ["test"]
"#,
    );

    // Создаём временный конфигурационный файл
    let mut config_file = NamedTempFile::new().expect("tempfile");
    let config_path = config_file.path().to_str().unwrap().to_string();

    // Создаём конфигурацию с указанием нашей временной директории паттернов
    let config_content = create_test_config_content().replace(
        "patterns_dir: \"patterns\"",
        &format!("patterns_dir: \"{}\"", patterns_dir),
    );
    fs::write(&config_path, &config_content).expect("write initial config");

    // Загружаем начальную конфигурацию
    let initial_config = Config::load(&config_path).expect("load initial config");
    assert_eq!(initial_config.polling_interval_ms, 100);

    // Создаём ConfigWatcher
    let config_watcher = ConfigWatcher::new(&config_path).expect("config watcher creation");
    let mut config_change_receiver = config_watcher.change_receiver();
    let config_watcher_handle = config_watcher.start_watching();

    // Создаём PatternWatcher
    let pattern_db = Arc::new(Mutex::new(
        PatternDatabase::load(&patterns_dir).expect("load initial patterns"),
    ));

    let pattern_config = PatternWatcherConfig {
        enabled: true,
        interval_sec: 1,
        notify_on_update: false,
    };

    let pattern_watcher = PatternWatcher::new(&patterns_dir, pattern_db.clone(), pattern_config)
        .expect("pattern watcher creation");

    let mut pattern_change_receiver = pattern_watcher.change_receiver();
    let pattern_watcher_handle = pattern_watcher.start_watching();

    // Даём время на запуск
    sleep(Duration::from_millis(200)).await;

    // Проверяем начальное состояние
    assert!(!*config_change_receiver.borrow_and_update());
    {
        let initial_pattern_result = pattern_change_receiver.borrow_and_update();
        assert_eq!(initial_pattern_result.patterns_after, 1);
    }

    // Тестируем обновление конфигурации
    let modified_config =
        config_content.replace("polling_interval_ms: 100", "polling_interval_ms: 200");
    fs::write(&config_path, &modified_config).expect("write modified config");

    // Даём время на обнаружение изменения конфигурации
    sleep(Duration::from_millis(300)).await;

    // Проверяем, что изменение конфигурации было обнаружено
    {
        let config_changed = config_change_receiver.borrow_and_update();
        assert!(*config_changed);
    }

    // Загружаем новую конфигурацию
    let new_config = Config::load(&config_path).expect("load modified config");
    assert_eq!(new_config.polling_interval_ms, 200);

    // Тестируем обновление паттернов
    create_test_pattern_file(
        temp_dir.path(),
        "new.yml",
        r#"
category: test
apps:
  - name: "new-app"
    label: "New Application"
    exe_patterns: ["new-app"]
    tags: ["test"]
"#,
    );

    // Даём время на обнаружение изменения паттернов
    sleep(Duration::from_millis(500)).await;

    // Проверяем, что изменение паттернов было обнаружено
    {
        let pattern_result = pattern_change_receiver.borrow_and_update();
        assert!(pattern_result.has_changes());
        assert_eq!(pattern_result.patterns_after, 2);
        assert_eq!(pattern_result.new_files, 1);
    }

    // Проверяем, что база паттернов обновлена
    let db = pattern_db.lock().unwrap();
    assert_eq!(db.all_patterns().len(), 2);

    // Завершаем задачи мониторинга
    config_watcher_handle.abort();
    pattern_watcher_handle.abort();

    // Даём время на завершение
    sleep(Duration::from_millis(200)).await;

    assert!(config_watcher_handle.is_finished());
    assert!(pattern_watcher_handle.is_finished());
}
