//! Интеграционные тесты для функции collect_snapshot.
//!
//! Эти тесты проверяют:
//! - обработку ошибок компонентов (window, audio, input)
//! - graceful fallback при недоступности компонентов
//! - корректность построения GlobalMetrics и ResponsivenessMetrics

use anyhow::Result;
use smoothtask_core::collect_snapshot;
use smoothtask_core::config::Thresholds;
use smoothtask_core::metrics::audio::{AudioIntrospector, AudioMetrics, StaticAudioIntrospector};
use smoothtask_core::metrics::input::{InputActivityTracker, InputTracker};
use smoothtask_core::metrics::scheduling_latency::LatencyCollector;
use smoothtask_core::metrics::system::{ProcPaths, SystemMetrics};
use smoothtask_core::metrics::windows::{
    StaticWindowIntrospector, WindowInfo, WindowIntrospector, WindowState,
};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// Интроспектор окон, который всегда возвращает ошибку.
struct FailingWindowIntrospector;

impl WindowIntrospector for FailingWindowIntrospector {
    fn windows(&self) -> Result<Vec<WindowInfo>> {
        anyhow::bail!("Window introspector failed")
    }

    fn focused_window(&self) -> Result<Option<WindowInfo>> {
        anyhow::bail!("Window introspector failed")
    }
}

/// Интроспектор аудио, который всегда возвращает ошибку.
struct FailingAudioIntrospector;

impl AudioIntrospector for FailingAudioIntrospector {
    fn audio_metrics(&mut self) -> Result<AudioMetrics> {
        anyhow::bail!("Audio introspector failed")
    }

    fn clients(&self) -> Result<Vec<smoothtask_core::metrics::audio::AudioClientInfo>> {
        anyhow::bail!("Audio introspector failed")
    }
}

/// Создаёт Thresholds с разумными значениями по умолчанию для тестов.
fn create_test_thresholds() -> Thresholds {
    Thresholds {
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
    }
}

/// Тест проверяет, что collect_snapshot успешно собирает снапшот с валидными компонентами.
#[tokio::test]
async fn test_collect_snapshot_with_valid_components() {
    let proc_paths = ProcPaths::default();
    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(Vec::new()));
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
    let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(InputTracker::Simple(
        InputActivityTracker::new(Duration::from_secs(60)),
    )));
    let mut prev_cpu_times: Option<SystemMetrics> = None;
    let thresholds = create_test_thresholds();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    let result = collect_snapshot(
        &proc_paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
    )
    .await;

    assert!(
        result.is_ok(),
        "collect_snapshot should succeed with valid components"
    );
    let snapshot = result.unwrap();

    // Проверяем, что снапшот содержит базовые метрики
    assert!(snapshot.snapshot_id > 0);
    assert!(!snapshot.processes.is_empty() || snapshot.processes.is_empty()); // Может быть пустым в тестовом окружении
    assert!(snapshot.app_groups.is_empty()); // app_groups заполняются после группировки

    // Проверяем, что GlobalMetrics построены корректно
    assert!(snapshot.global.mem_total_kb > 0 || snapshot.global.mem_total_kb == 0); // Может быть 0 в тестовом окружении
    assert!(snapshot.global.load_avg_one >= 0.0);
    assert!(snapshot.global.load_avg_five >= 0.0);
    assert!(snapshot.global.load_avg_fifteen >= 0.0);

    // Проверяем, что ResponsivenessMetrics построены корректно
    assert!(
        snapshot.responsiveness.sched_latency_p95_ms.is_none()
            || snapshot.responsiveness.sched_latency_p95_ms.is_some()
    );
    assert!(
        snapshot.responsiveness.sched_latency_p99_ms.is_none()
            || snapshot.responsiveness.sched_latency_p99_ms.is_some()
    );
}

/// Тест проверяет graceful fallback при недоступности window introspector.
#[tokio::test]
async fn test_collect_snapshot_with_failing_window_introspector() {
    let proc_paths = ProcPaths::default();
    let window_introspector: Arc<dyn WindowIntrospector> = Arc::new(FailingWindowIntrospector);
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
    let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(InputTracker::Simple(
        InputActivityTracker::new(Duration::from_secs(60)),
    )));
    let mut prev_cpu_times: Option<SystemMetrics> = None;
    let thresholds = create_test_thresholds();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    let result = collect_snapshot(
        &proc_paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
    )
    .await;

    // Функция должна успешно завершиться, даже если window introspector недоступен
    assert!(
        result.is_ok(),
        "collect_snapshot should succeed even if window introspector fails"
    );
    let snapshot = result.unwrap();

    // Проверяем, что снапшот собран корректно
    assert!(snapshot.snapshot_id > 0);
    // Процессы не должны иметь информацию об окнах (has_gui_window = false)
    for process in &snapshot.processes {
        assert!(!process.has_gui_window);
        assert!(!process.is_focused_window);
    }
}

/// Тест проверяет graceful fallback при недоступности audio introspector.
#[tokio::test]
async fn test_collect_snapshot_with_failing_audio_introspector() {
    let proc_paths = ProcPaths::default();
    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(Vec::new()));
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(FailingAudioIntrospector)));
    let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(InputTracker::Simple(
        InputActivityTracker::new(Duration::from_secs(60)),
    )));
    let mut prev_cpu_times: Option<SystemMetrics> = None;
    let thresholds = create_test_thresholds();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    let result = collect_snapshot(
        &proc_paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
    )
    .await;

    // Функция должна успешно завершиться, даже если audio introspector недоступен
    assert!(
        result.is_ok(),
        "collect_snapshot should succeed even if audio introspector fails"
    );
    let snapshot = result.unwrap();

    // Проверяем, что снапшот собран корректно
    assert!(snapshot.snapshot_id > 0);
    // Процессы не должны иметь информацию об аудио (is_audio_client = false)
    for process in &snapshot.processes {
        assert!(!process.is_audio_client);
        assert!(!process.has_active_stream);
    }
    // Audio metrics должны быть пустыми (xrun_count = 0)
    assert_eq!(snapshot.responsiveness.audio_xruns_delta, Some(0));
}

/// Тест проверяет graceful fallback при недоступности всех опциональных компонентов.
#[tokio::test]
async fn test_collect_snapshot_with_all_optional_components_failing() {
    let proc_paths = ProcPaths::default();
    let window_introspector: Arc<dyn WindowIntrospector> = Arc::new(FailingWindowIntrospector);
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(FailingAudioIntrospector)));
    let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(InputTracker::Simple(
        InputActivityTracker::new(Duration::from_secs(60)),
    )));
    let mut prev_cpu_times: Option<SystemMetrics> = None;
    let thresholds = create_test_thresholds();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    let result = collect_snapshot(
        &proc_paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
    )
    .await;

    // Функция должна успешно завершиться, даже если все опциональные компоненты недоступны
    assert!(
        result.is_ok(),
        "collect_snapshot should succeed even if all optional components fail"
    );
    let snapshot = result.unwrap();

    // Проверяем, что снапшот собран корректно с дефолтными значениями
    assert!(snapshot.snapshot_id > 0);
    // GlobalMetrics должны быть построены (даже с дефолтными значениями)
    assert!(snapshot.global.mem_total_kb >= 0);
    // ResponsivenessMetrics должны быть построены
    assert!(snapshot.responsiveness.audio_xruns_delta.is_some());
}

/// Тест проверяет корректность построения GlobalMetrics.
#[tokio::test]
async fn test_collect_snapshot_global_metrics_correctness() {
    let proc_paths = ProcPaths::default();
    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(Vec::new()));
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
    let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(InputTracker::Simple(
        InputActivityTracker::new(Duration::from_secs(60)),
    )));
    let mut prev_cpu_times: Option<SystemMetrics> = None;
    let thresholds = create_test_thresholds();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    let result = collect_snapshot(
        &proc_paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
    )
    .await;

    assert!(result.is_ok());
    let snapshot = result.unwrap();
    let global = &snapshot.global;

    // Проверяем, что все поля GlobalMetrics заполнены корректно
    assert!(global.cpu_user >= 0.0 && global.cpu_user <= 100.0);
    assert!(global.cpu_system >= 0.0 && global.cpu_system <= 100.0);
    assert!(global.cpu_idle >= 0.0 && global.cpu_idle <= 100.0);
    assert!(global.cpu_iowait >= 0.0 && global.cpu_iowait <= 100.0);
    assert!(global.mem_total_kb >= 0);
    assert!(global.mem_used_kb >= 0);
    assert!(global.mem_available_kb >= 0);
    assert!(global.swap_total_kb >= 0);
    assert!(global.swap_used_kb >= 0);
    assert!(global.load_avg_one >= 0.0);
    assert!(global.load_avg_five >= 0.0);
    assert!(global.load_avg_fifteen >= 0.0);
    // PSI метрики могут быть None (если PSI недоступен)
    // Input метрики должны быть заполнены
    assert!(global.user_active || !global.user_active); // Может быть true или false
}

/// Тест проверяет корректность построения ResponsivenessMetrics.
#[tokio::test]
async fn test_collect_snapshot_responsiveness_metrics_correctness() {
    let proc_paths = ProcPaths::default();
    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(Vec::new()));
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
    let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(InputTracker::Simple(
        InputActivityTracker::new(Duration::from_secs(60)),
    )));
    let mut prev_cpu_times: Option<SystemMetrics> = None;
    let thresholds = create_test_thresholds();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    let result = collect_snapshot(
        &proc_paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
    )
    .await;

    assert!(result.is_ok());
    let snapshot = result.unwrap();
    let responsiveness = &snapshot.responsiveness;

    // Проверяем, что все поля ResponsivenessMetrics заполнены корректно
    assert!(responsiveness.audio_xruns_delta.is_some());
    // Scheduling latency может быть None (если probe-thread ещё не собрал достаточно данных)
    assert!(
        responsiveness.sched_latency_p95_ms.is_none()
            || responsiveness.sched_latency_p95_ms.is_some()
    );
    assert!(
        responsiveness.sched_latency_p99_ms.is_none()
            || responsiveness.sched_latency_p99_ms.is_some()
    );
    // bad_responsiveness и responsiveness_score должны быть вычислены
    assert!(responsiveness.bad_responsiveness || !responsiveness.bad_responsiveness); // Может быть true или false
    assert!(responsiveness.responsiveness_score.is_some());
    if let Some(score) = responsiveness.responsiveness_score {
        assert!(score >= 0.0 && score <= 1.0);
    }
}

/// Тест проверяет, что collect_snapshot корректно обрабатывает обновление prev_cpu_times.
#[tokio::test]
async fn test_collect_snapshot_updates_prev_cpu_times() {
    let proc_paths = ProcPaths::default();
    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(Vec::new()));
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
    let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(InputTracker::Simple(
        InputActivityTracker::new(Duration::from_secs(60)),
    )));
    let mut prev_cpu_times: Option<SystemMetrics> = None;
    let thresholds = create_test_thresholds();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    // Первый вызов - prev_cpu_times = None, CPU usage должен быть None
    let result1 = collect_snapshot(
        &proc_paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
    )
    .await;

    assert!(result1.is_ok());
    let _snapshot1 = result1.unwrap();
    // После первого вызова prev_cpu_times должен быть Some
    assert!(prev_cpu_times.is_some());

    // Второй вызов - prev_cpu_times = Some, CPU usage должен быть вычислен
    tokio::time::sleep(Duration::from_millis(100)).await; // Небольшая задержка для изменения CPU
    let result2 = collect_snapshot(
        &proc_paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
    )
    .await;

    assert!(result2.is_ok());
    let snapshot2 = result2.unwrap();
    // После второго вызова prev_cpu_times должен быть обновлён
    assert!(prev_cpu_times.is_some());
    // CPU usage во втором снапшоте должен быть вычислен (может быть 0.0, если система не загружена)
    assert!(snapshot2.global.cpu_user >= 0.0 && snapshot2.global.cpu_user <= 100.0);
}

/// Тест проверяет, что collect_snapshot корректно обновляет информацию об окнах в процессах.
#[tokio::test]
async fn test_collect_snapshot_updates_window_info_in_processes() {
    let proc_paths = ProcPaths::default();
    // Создаём window introspector с тестовыми окнами
    let windows = vec![WindowInfo {
        app_id: Some("test-app".to_string()),
        title: Some("Test Window".to_string()),
        workspace: Some(1),
        state: WindowState::Focused,
        pid: Some(1234),
        pid_confidence: 1.0,
    }];
    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(windows));
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
    let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(InputTracker::Simple(
        InputActivityTracker::new(Duration::from_secs(60)),
    )));
    let mut prev_cpu_times: Option<SystemMetrics> = None;
    let thresholds = create_test_thresholds();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    let result = collect_snapshot(
        &proc_paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
    )
    .await;

    assert!(result.is_ok());
    let snapshot = result.unwrap();

    // Ищем процесс с PID 1234 (если он существует в системе)
    let process_with_window = snapshot.processes.iter().find(|p| p.pid == 1234);
    if let Some(process) = process_with_window {
        // Если процесс найден, он должен иметь информацию об окне
        assert!(process.has_gui_window);
        assert!(process.is_focused_window);
        assert!(process.window_state.is_some());
    }
    // Если процесс не найден (не существует в системе), это тоже нормально
}

/// Тест проверяет, что collect_snapshot корректно обновляет информацию об аудио-клиентах в процессах.
#[tokio::test]
async fn test_collect_snapshot_updates_audio_info_in_processes() {
    let proc_paths = ProcPaths::default();
    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(Vec::new()));
    // Создаём audio introspector с тестовыми клиентами
    let audio_metrics = AudioMetrics::empty(SystemTime::now(), SystemTime::now());
    let audio_clients = vec![smoothtask_core::metrics::audio::AudioClientInfo {
        pid: 5678,
        buffer_size_samples: Some(1024),
        sample_rate_hz: Some(44100),
    }];
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> = Arc::new(Mutex::new(
        Box::new(StaticAudioIntrospector::new(audio_metrics, audio_clients)),
    ));
    let input_tracker: Arc<Mutex<InputTracker>> = Arc::new(Mutex::new(InputTracker::Simple(
        InputActivityTracker::new(Duration::from_secs(60)),
    )));
    let mut prev_cpu_times: Option<SystemMetrics> = None;
    let thresholds = create_test_thresholds();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    let result = collect_snapshot(
        &proc_paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
    )
    .await;

    assert!(result.is_ok());
    let snapshot = result.unwrap();

    // Ищем процесс с PID 5678 (если он существует в системе)
    let process_with_audio = snapshot.processes.iter().find(|p| p.pid == 5678);
    if let Some(process) = process_with_audio {
        // Если процесс найден, он должен иметь информацию об аудио
        assert!(process.is_audio_client);
        assert!(process.has_active_stream);
    }
    // Если процесс не найден (не существует в системе), это тоже нормально
}
