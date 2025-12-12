//! Интеграционные тесты для ML-классификатора.
//!
//! Эти тесты проверяют работу ML-классификатора в реальных сценариях:
//! - Интеграция с системой классификации процессов
//! - Взаимодействие с PatternWatcher
//! - Обработка ошибок и fallback механизмы
//! - Производительность и надежность

#[cfg(any(feature = "catboost", feature = "onnx"))]
use smoothtask_core::classify::ml_classifier::{create_ml_classifier, MLClassifier, MLClassificationResult};
use smoothtask_core::classify::pattern_watcher::{PatternWatcher, PatternWatcherConfig};
use smoothtask_core::classify::rules::{classify_process, PatternDatabase};
use smoothtask_core::config::config_struct::MLClassifierConfig;
use smoothtask_core::logging::snapshots::ProcessRecord;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tempfile::{tempdir, TempDir};

fn create_test_process() -> ProcessRecord {
    ProcessRecord {
        pid: 1000,
        ppid: 1,
        uid: 1000,
        gid: 1000,
        exe: Some("test-app".to_string()),
        cmdline: None,
        cgroup_path: None,
        systemd_unit: None,
        app_group_id: None,
        state: "R".to_string(),
        start_time: 0,
        uptime_sec: 100,
        tty_nr: 0,
        has_tty: false,
        cpu_share_1s: Some(0.2),
        cpu_share_10s: Some(0.3),
        io_read_bytes: Some(1024),
        io_write_bytes: Some(512),
        rss_mb: Some(50),
        swap_mb: Some(10),
        voluntary_ctx: Some(100),
        involuntary_ctx: Some(50),
        has_gui_window: false,
        is_focused_window: false,
        window_state: None,
        env_has_display: false,
        env_has_wayland: false,
        env_term: None,
        env_ssh: false,
        is_audio_client: false,
        has_active_stream: false,
        process_type: None,
        tags: Vec::new(),
        nice: 0,
        ionice_class: None,
        ionice_prio: None,
        teacher_priority_class: None,
        teacher_score: None,
    }
}

fn create_test_pattern_file(dir: &Path, filename: &str, content: &str) {
    use std::fs;
    let file_path = dir.join(filename);
    fs::write(&file_path, content).expect("write test pattern file");
}

#[test]
#[cfg(any(feature = "catboost", feature = "onnx"))]
fn test_ml_classifier_integration_with_pattern_matching() {
    let temp_dir = tempdir().expect("temp dir");
    let patterns_dir = temp_dir.path();

    // Создаем тестовые паттерны
    create_test_pattern_file(
        patterns_dir,
        "audio.yml",
        r#"
category: audio
apps:
  - name: "audacity"
    label: "Audacity"
    exe_patterns: ["audacity"]
    tags: ["audio", "realtime"]
"#,
    );

    let db = PatternDatabase::load(patterns_dir).expect("load patterns");

    // Создаем ML-классификатор (используем заглушку для тестирования)
    let config = MLClassifierConfig {
        enabled: true,
        model_path: "test.json".to_string(),
        confidence_threshold: 0.7,
        model_type: smoothtask_core::config::config_struct::ModelType::Catboost,
    };

    let classifier = create_ml_classifier(config);
    assert!(classifier.is_ok());
    let classifier = classifier.unwrap();

    // Создаем процесс, который соответствует паттерну
    let mut process = create_test_process();
    process.exe = Some("audacity".to_string());
    process.is_audio_client = true;
    process.has_active_stream = true;

    // Классифицируем процесс
    classify_process(&mut process, &db, Some(&*classifier), None);

    // Проверяем результаты
    assert_eq!(process.process_type, Some("audio".to_string()));
    assert!(process.tags.contains(&"audio".to_string()));
    assert!(process.tags.contains(&"realtime".to_string()));
}

#[test]
#[cfg(any(feature = "catboost", feature = "onnx"))]
fn test_ml_classifier_fallback_when_disabled() {
    let temp_dir = tempdir().expect("temp dir");
    let patterns_dir = temp_dir.path();

    create_test_pattern_file(
        patterns_dir,
        "terminals.yml",
        r#"
category: terminal
apps:
  - name: "gnome-terminal"
    label: "GNOME Terminal"
    exe_patterns: ["gnome-terminal"]
    tags: ["terminal", "interactive"]
"#,
    );

    let db = PatternDatabase::load(patterns_dir).expect("load patterns");

    // Создаем отключенный ML-классификатор
    let config = MLClassifierConfig {
        enabled: false,
        model_path: "test.json".to_string(),
        confidence_threshold: 0.7,
        model_type: smoothtask_core::config::config_struct::ModelType::Catboost,
    };

    let classifier = create_ml_classifier(config);
    assert!(classifier.is_ok());
    let classifier = classifier.unwrap();

    let mut process = create_test_process();
    process.exe = Some("gnome-terminal".to_string());

    // Классифицируем процесс
    classify_process(&mut process, &db, Some(&*classifier), None);

    // Должны получить результат только от паттернов
    assert_eq!(process.process_type, Some("terminal".to_string()));
    assert!(process.tags.contains(&"terminal".to_string()));
    assert!(process.tags.contains(&"interactive".to_string()));
}

#[test]
#[cfg(any(feature = "catboost", feature = "onnx"))]
fn test_ml_classifier_error_handling() {
    // Тестируем обработку ошибок при несуществующей модели
    let config = MLClassifierConfig {
        enabled: true,
        model_path: "/nonexistent/path/model.json".to_string(),
        confidence_threshold: 0.7,
        model_type: smoothtask_core::config::config_struct::ModelType::Catboost,
    };

    let classifier = create_ml_classifier(config);
    assert!(classifier.is_err());

    let err = classifier.unwrap_err();
    assert!(err.to_string().contains("не найден"));
}

#[tokio::test]
#[cfg(any(feature = "catboost", feature = "onnx"))]
async fn test_ml_classifier_with_pattern_watcher_integration() {
    let temp_dir = tempdir().expect("temp dir");
    let patterns_dir = temp_dir.path();

    // Создаем начальные паттерны
    create_test_pattern_file(
        patterns_dir,
        "browsers.yml",
        r#"
category: browser
apps:
  - name: "firefox"
    label: "Mozilla Firefox"
    exe_patterns: ["firefox"]
    tags: ["browser", "gui"]
"#,
    );

    let db = PatternDatabase::load(patterns_dir).expect("load patterns");
    let pattern_db = Arc::new(Mutex::new(db));

    // Создаем PatternWatcher
    let watcher_config = PatternWatcherConfig {
        enabled: true,
        interval_sec: 60,
        notify_on_update: false,
    };

    let watcher = PatternWatcher::new(
        patterns_dir.to_str().unwrap(),
        pattern_db.clone(),
        watcher_config,
    ).expect("watcher creation");

    // Создаем ML-классификатор
    let ml_config = MLClassifierConfig {
        enabled: true,
        model_path: "test.json".to_string(),
        confidence_threshold: 0.7,
        model_type: smoothtask_core::config::config_struct::ModelType::Catboost,
    };

    let classifier = create_ml_classifier(ml_config);
    assert!(classifier.is_ok());
    let classifier = classifier.unwrap();

    // Тестируем классификацию с текущими паттернами
    let mut process = create_test_process();
    process.exe = Some("firefox".to_string());
    process.has_gui_window = true;

    {
        let db_guard = pattern_db.lock().await;
        classify_process(&mut process, &db_guard, Some(&*classifier), None);
    }

    assert_eq!(process.process_type, Some("browser".to_string()));
    assert!(process.tags.contains(&"browser".to_string()));
    assert!(process.tags.contains(&"gui".to_string()));
}

#[test]
#[cfg(any(feature = "catboost", feature = "onnx"))]
fn test_ml_classifier_confidence_thresholds() {
    let temp_dir = tempdir().expect("temp dir");
    let patterns_dir = temp_dir.path();

    create_test_pattern_file(
        patterns_dir,
        "games.yml",
        r#"
category: game
apps:
  - name: "game-app"
    label: "Game Application"
    exe_patterns: ["game-app"]
    tags: ["game", "interactive"]
"#,
    );

    let db = PatternDatabase::load(patterns_dir).expect("load patterns");

    // Создаем ML-классификатор с разными порогами уверенности
    let config = MLClassifierConfig {
        enabled: true,
        model_path: "test.json".to_string(),
        confidence_threshold: 0.8, // Высокий порог
        model_type: smoothtask_core::config::config_struct::ModelType::Catboost,
    };

    let classifier = create_ml_classifier(config);
    assert!(classifier.is_ok());
    let classifier = classifier.unwrap();

    let mut process = create_test_process();
    process.exe = Some("game-app".to_string());

    classify_process(&mut process, &db, Some(&*classifier), None);

    // При высоком пороге уверенности, паттерн-классификация должна доминировать
    assert_eq!(process.process_type, Some("game".to_string()));
}

#[test]
#[cfg(any(feature = "catboost", feature = "onnx"))]
fn test_ml_classifier_feature_extraction_comprehensive() {
    let config = MLClassifierConfig {
        enabled: false, // Используем заглушку для тестирования
        model_path: "test.json".to_string(),
        confidence_threshold: 0.7,
        model_type: smoothtask_core::config::config_struct::ModelType::Catboost,
    };

    let classifier = create_ml_classifier(config);
    assert!(classifier.is_ok());
    let classifier = classifier.unwrap();

    // Создаем процесс с различными характеристиками
    let mut process = create_test_process();
    process.cpu_share_1s = Some(0.8); // Высокий CPU
    process.cpu_share_10s = Some(0.6);
    process.io_read_bytes = Some(10 * 1024 * 1024); // 10MB чтения
    process.io_write_bytes = Some(5 * 1024 * 1024); // 5MB записи
    process.rss_mb = Some(500); // 500MB памяти
    process.swap_mb = Some(200); // 200MB swap
    process.voluntary_ctx = Some(10000); // Много добровольных переключений
    process.involuntary_ctx = Some(5000); // Много принудительных переключений
    process.has_tty = true;
    process.has_gui_window = true;
    process.is_focused_window = true;
    process.env_has_display = true;
    process.env_has_wayland = true;
    process.env_ssh = true;
    process.is_audio_client = true;
    process.has_active_stream = true;

    // Тестируем извлечение фич
    let features = classifier.process_to_features(&process);
    
    // Проверяем, что все фичи извлечены правильно
    assert_eq!(features.len(), 16); // 8 числовых + 8 булевых
    
    // Проверяем числовые фичи
    assert_eq!(features[0], 0.8); // cpu_share_1s
    assert_eq!(features[1], 0.6); // cpu_share_10s
    assert_eq!(features[2], 10.0); // io_read_bytes в MB
    assert_eq!(features[3], 5.0); // io_write_bytes в MB
    assert_eq!(features[4], 500.0); // rss_mb
    assert_eq!(features[5], 200.0); // swap_mb
    assert_eq!(features[6], 10000.0); // voluntary_ctx
    assert_eq!(features[7], 5000.0); // involuntary_ctx
    
    // Проверяем булевые фичи (должны быть 1.0)
    for i in 8..16 {
        assert_eq!(features[i], 1.0);
    }
}

#[test]
#[cfg(any(feature = "catboost", feature = "onnx"))]
fn test_ml_classifier_tag_merging() {
    let temp_dir = tempdir().expect("temp dir");
    let patterns_dir = temp_dir.path();

    create_test_pattern_file(
        patterns_dir,
        "ide.yml",
        r#"
category: ide
apps:
  - name: "vscode"
    label: "Visual Studio Code"
    exe_patterns: ["code"]
    tags: ["ide", "development"]
"#,
    );

    let db = PatternDatabase::load(patterns_dir).expect("load patterns");

    let config = MLClassifierConfig {
        enabled: true,
        model_path: "test.json".to_string(),
        confidence_threshold: 0.7,
        model_type: smoothtask_core::config::config_struct::ModelType::Catboost,
    };

    let classifier = create_ml_classifier(config);
    assert!(classifier.is_ok());
    let classifier = classifier.unwrap();

    let mut process = create_test_process();
    process.exe = Some("code".to_string());
    process.has_gui_window = true;
    process.cpu_share_10s = Some(0.4);

    classify_process(&mut process, &db, Some(&*classifier), None);

    // Проверяем, что теги из паттернов и ML объединены
    let expected_tags = HashSet::from([
        "ide".to_string(),
        "development".to_string(),
        "gui".to_string(),
        "interactive".to_string(),
    ]);
    
    let actual_tags = HashSet::from_iter(process.tags.iter().cloned());
    
    for tag in expected_tags {
        assert!(actual_tags.contains(&tag), "Expected tag {} not found", tag);
    }
}

#[test]
#[cfg(any(feature = "catboost", feature = "onnx"))]
fn test_ml_classifier_performance_metrics() {
    let config = MLClassifierConfig {
        enabled: false, // Используем заглушку для тестирования производительности
        model_path: "test.json".to_string(),
        confidence_threshold: 0.7,
        model_type: smoothtask_core::config::config_struct::ModelType::Catboost,
    };

    let classifier = create_ml_classifier(config);
    assert!(classifier.is_ok());
    let classifier = classifier.unwrap();

    // Тестируем производительность на большом количестве процессов
    let start_time = std::time::Instant::now();
    
    for _ in 0..1000 {
        let process = create_test_process();
        let _result = classifier.classify(&process);
    }
    
    let duration = start_time.elapsed();
    
    // Должно выполняться быстро (менее 100мс для 1000 процессов)
    assert!(duration.as_millis() < 100, "Performance test failed: took {}ms", duration.as_millis());
}
