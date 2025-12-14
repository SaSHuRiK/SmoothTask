//! Бенчмарки для ML-классификатора.
//!
//! Этот модуль содержит бенчмарки для измерения производительности
//! ML-классификатора, включая извлечение фич и классификацию процессов.

use criterion::{criterion_group, criterion_main, Criterion};
use smoothtask_core::classify::ml_classifier::{create_ml_classifier, MLClassifier};
use smoothtask_core::config::config_struct::{MLClassifierConfig, ModelType};
use smoothtask_core::logging::snapshots::ProcessRecord;

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
        cpu_share_1s: Some(0.25),
        cpu_share_10s: Some(0.5),
        io_read_bytes: Some(2 * 1024 * 1024),
        io_write_bytes: Some(1024 * 1024),
        io_read_operations: None,
        io_write_operations: None,
        io_total_operations: None,
        io_last_update_ns: None,
        io_data_source: None,
        rss_mb: Some(100),
        swap_mb: Some(50),
        voluntary_ctx: Some(1000),
        involuntary_ctx: Some(500),
        has_gui_window: true,
        is_focused_window: true,
        window_state: None,
        env_has_display: true,
        env_has_wayland: true,
        env_term: None,
        env_ssh: false,
        is_audio_client: true,
        has_active_stream: true,
        process_type: None,
        tags: Vec::new(),
        nice: 0,
        ionice_class: None,
        ionice_prio: None,
        teacher_priority_class: None,
        teacher_score: None,
        energy_uj: None,
        power_w: None,
        energy_timestamp: None,
        network_rx_bytes: None,
        network_tx_bytes: None,
        network_rx_packets: None,
        network_tx_packets: None,
        network_tcp_connections: None,
        network_udp_connections: None,
        network_last_update_ns: None,
        network_data_source: None,
        gpu_utilization: None,
        gpu_memory_bytes: None,
        gpu_time_us: None,
        gpu_api_calls: None,
        gpu_last_update_ns: None,
        gpu_data_source: None,
    }
}

fn benchmark_stub_classifier(c: &mut Criterion) {
    let config = MLClassifierConfig {
        enabled: false,
        model_path: "test.json".to_string(),
        confidence_threshold: 0.7,
        model_type: ModelType::Catboost,
    };

    let mut classifier = create_ml_classifier(config).unwrap();
    let process = create_test_process();

    c.bench_function("stub_classifier_classify", |b| {
        b.iter(|| {
            classifier.classify(&process);
        })
    });
}

fn benchmark_feature_extraction(c: &mut Criterion) {
    use smoothtask_core::classify::ml_classifier::CatBoostMLClassifier;

    let config = MLClassifierConfig {
        enabled: false,
        model_path: "test.json".to_string(),
        confidence_threshold: 0.7,
        model_type: ModelType::Catboost,
    };

    let classifier = CatBoostMLClassifier::new(config).unwrap();
    let process = create_test_process();

    c.bench_function("feature_extraction", |b| {
        b.iter(|| {
            classifier.process_to_features(&process);
        })
    });
}

fn benchmark_feature_extraction_cached(c: &mut Criterion) {
    use smoothtask_core::classify::ml_classifier::CatBoostMLClassifier;

    let config = MLClassifierConfig {
        enabled: false,
        model_path: "test.json".to_string(),
        confidence_threshold: 0.7,
        model_type: ModelType::Catboost,
    };

    let classifier = CatBoostMLClassifier::new(config).unwrap();
    let process = create_test_process();

    // Предварительно заполняем кэш
    classifier.process_to_features(&process);

    c.bench_function("feature_extraction_cached", |b| {
        b.iter(|| {
            classifier.process_to_features(&process);
        })
    });
}

fn benchmark_multiple_classifications(c: &mut Criterion) {
    let config = MLClassifierConfig {
        enabled: false,
        model_path: "test.json".to_string(),
        confidence_threshold: 0.7,
        model_type: ModelType::Catboost,
    };

    let mut classifier = create_ml_classifier(config).unwrap();
    let process = create_test_process();

    c.bench_function("multiple_classifications", |b| {
        b.iter(|| {
            for _ in 0..10 {
                classifier.classify(&process);
            }
        })
    });
}

fn benchmark_feature_extraction_optimized_uncached(c: &mut Criterion) {
    use smoothtask_core::classify::ml_classifier::CatBoostMLClassifier;

    let config = MLClassifierConfig {
        enabled: false,
        model_path: "test.json".to_string(),
        confidence_threshold: 0.7,
        model_type: ModelType::Catboost,
    };

    let classifier = CatBoostMLClassifier::new(config).unwrap();
    let process = create_test_process();

    c.bench_function("feature_extraction_optimized_uncached", |b| {
        b.iter(|| {
            classifier.process_to_features_optimized(&process, false);
        })
    });
}

fn benchmark_feature_extraction_optimized_cached(c: &mut Criterion) {
    use smoothtask_core::classify::ml_classifier::CatBoostMLClassifier;

    let config = MLClassifierConfig {
        enabled: false,
        model_path: "test.json".to_string(),
        confidence_threshold: 0.7,
        model_type: ModelType::Catboost,
    };

    let classifier = CatBoostMLClassifier::new(config).unwrap();
    let process = create_test_process();

    // Предварительно заполняем кэш
    classifier.process_to_features_optimized(&process, true);

    c.bench_function("feature_extraction_optimized_cached", |b| {
        b.iter(|| {
            classifier.process_to_features_optimized(&process, true);
        })
    });
}

fn benchmark_cache_capacity_adjustment(c: &mut Criterion) {
    use smoothtask_core::classify::ml_classifier::CatBoostMLClassifier;

    let config = MLClassifierConfig {
        enabled: false,
        model_path: "test.json".to_string(),
        confidence_threshold: 0.7,
        model_type: ModelType::Catboost,
    };

    let classifier = CatBoostMLClassifier::new(config).unwrap();
    let process = create_test_process();

    // Устанавливаем небольшую емкость кэша
    CatBoostMLClassifier::set_feature_cache_capacity(16);

    c.bench_function("cache_capacity_small", |b| {
        b.iter(|| {
            for i in 0..32 {
                let mut proc = process.clone();
                proc.pid = 1000 + i;
                classifier.process_to_features_optimized(&proc, true);
            }
        })
    });

    // Устанавливаем большую емкость кэша
    CatBoostMLClassifier::set_feature_cache_capacity(1024);

    c.bench_function("cache_capacity_large", |b| {
        b.iter(|| {
            for i in 0..32 {
                let mut proc = process.clone();
                proc.pid = 1000 + i;
                classifier.process_to_features_optimized(&proc, true);
            }
        })
    });

    // Восстанавливаем емкость кэша по умолчанию
    CatBoostMLClassifier::set_feature_cache_capacity(1024);
}

criterion_group!(
    name = ml_classifier_benches;
    config = Criterion::default().sample_size(10);
    targets =
        benchmark_stub_classifier,
        benchmark_feature_extraction,
        benchmark_feature_extraction_cached,
        benchmark_multiple_classifications,
        benchmark_feature_extraction_optimized_uncached,
        benchmark_feature_extraction_optimized_cached,
        benchmark_cache_capacity_adjustment
);

criterion_main!(ml_classifier_benches);
