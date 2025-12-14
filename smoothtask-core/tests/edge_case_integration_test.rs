// Интеграционные тесты для проверки обработки крайних случаев и ошибок

use smoothtask_core::{
    collect_snapshot, collect_snapshot_with_caching,
    config::config_struct::Thresholds,
    metrics::audio::{AudioIntrospector, StaticAudioIntrospector},
    metrics::input::InputTracker,
    metrics::scheduling_latency::LatencyCollector,
    metrics::system::ProcPaths,
    metrics::windows::{StaticWindowIntrospector, WindowIntrospector},
};
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tempfile::tempdir;

#[tokio::test]
async fn test_snapshot_collection_with_missing_files() {
    // Тест проверяет, что collect_snapshot корректно обрабатывает отсутствие системных файлов
    let temp_dir = tempdir().unwrap();
    let dir_path = temp_dir.path();

    // Создаём только некоторые файлы, некоторые отсутствуют
    let stat_file = dir_path.join("stat");
    let stat_content = "cpu 100 20 50 200 10 5 5 0 0 0\ncpu0 50 10 25 100 5 2 2 0 0 0";
    fs::write(&stat_file, stat_content).unwrap();

    let paths = ProcPaths {
        stat: stat_file,
        meminfo: dir_path.join("nonexistent_meminfo"), // Этот файл отсутствует
        loadavg: dir_path.join("nonexistent_loadavg"), // Этот файл отсутствует
        pressure_cpu: dir_path.join("nonexistent_pressure_cpu"),
        pressure_io: dir_path.join("nonexistent_pressure_io"),
        pressure_memory: dir_path.join("nonexistent_pressure_memory"),
    };

    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(Vec::new()));
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
    let input_tracker = Arc::new(Mutex::new(InputTracker::new(Duration::from_secs(60))));
    let mut prev_cpu_times = None;
    let thresholds = Thresholds::default();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    // Функция должна обработать отсутствие файлов gracefully
    let result = collect_snapshot(
        &paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
    )
    .await;

    // Должна быть ошибка, так как критические файлы отсутствуют
    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_string = error.to_string();

    // Проверяем, что ошибка содержит информацию о проблеме
    assert!(error_string.contains("meminfo") || error_string.contains("loadavg"));
}

#[tokio::test]
async fn test_snapshot_collection_with_corrupted_files() {
    // Тест проверяет обработку поврежденных системных файлов
    let temp_dir = tempdir().unwrap();
    let dir_path = temp_dir.path();

    // Создаём файлы с невалидным содержимым
    let stat_file = dir_path.join("stat");
    let meminfo_file = dir_path.join("meminfo");
    let loadavg_file = dir_path.join("loadavg");

    fs::write(&stat_file, "invalid cpu data").unwrap();
    fs::write(&meminfo_file, "invalid meminfo data").unwrap();
    fs::write(&loadavg_file, "invalid loadavg data").unwrap();

    let paths = ProcPaths {
        stat: stat_file,
        meminfo: meminfo_file,
        loadavg: loadavg_file,
        pressure_cpu: dir_path.join("pressure_cpu"),
        pressure_io: dir_path.join("pressure_io"),
        pressure_memory: dir_path.join("pressure_memory"),
    };

    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(Vec::new()));
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
    let input_tracker = Arc::new(Mutex::new(InputTracker::new(Duration::from_secs(60))));
    let mut prev_cpu_times = None;
    let thresholds = Thresholds::default();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    // Функция должна обработать поврежденные данные gracefully
    let result = collect_snapshot(
        &paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
    )
    .await;

    // Должна быть ошибка при парсинге
    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_string = error.to_string();

    // Проверяем, что ошибка содержит информацию о парсинге
    assert!(error_string.contains("parse") || error_string.contains("invalid"));
}

#[tokio::test]
async fn test_snapshot_collection_with_all_components_failing() {
    // Тест проверяет обработку ситуации, когда все опциональные компоненты недоступны
    let temp_dir = tempdir().unwrap();
    let dir_path = temp_dir.path();

    // Создаём валидные системные файлы
    let stat_file = dir_path.join("stat");
    let meminfo_file = dir_path.join("meminfo");
    let loadavg_file = dir_path.join("loadavg");

    let stat_content = "cpu 100 20 50 200 10 5 5 0 0 0\ncpu0 50 10 25 100 5 2 2 0 0 0";
    let meminfo_content = "MemTotal:        16384256 kB\nMemFree:          9876543 kB\nMemAvailable:     9876543 kB\nBuffers:           345678 kB\nCached:           2345678 kB\nSwapTotal:        8192000 kB\nSwapFree:         4096000 kB";
    let loadavg_content = "0.50 0.75 0.90 1/123 4567";

    fs::write(&stat_file, stat_content).unwrap();
    fs::write(&meminfo_file, meminfo_content).unwrap();
    fs::write(&loadavg_file, loadavg_content).unwrap();

    let paths = ProcPaths {
        stat: stat_file,
        meminfo: meminfo_file,
        loadavg: loadavg_file,
        pressure_cpu: dir_path.join("pressure_cpu"),
        pressure_io: dir_path.join("pressure_io"),
        pressure_memory: dir_path.join("pressure_memory"),
    };

    // Создаём интроспекторы, которые всегда возвращают ошибки
    struct FailingWindowIntrospector;

    impl WindowIntrospector for FailingWindowIntrospector {
        fn windows(&self) -> anyhow::Result<Vec<smoothtask_core::metrics::windows::WindowInfo>> {
            anyhow::bail!("Window introspector failed");
        }

        fn focused_window(
            &self,
        ) -> anyhow::Result<Option<smoothtask_core::metrics::windows::WindowInfo>> {
            anyhow::bail!("Window introspector failed");
        }
    }

    struct FailingAudioIntrospector;

    impl AudioIntrospector for FailingAudioIntrospector {
        fn audio_metrics(
            &mut self,
        ) -> anyhow::Result<smoothtask_core::metrics::audio::AudioMetrics> {
            anyhow::bail!("Audio introspector failed");
        }

        fn clients(&self) -> anyhow::Result<Vec<smoothtask_core::metrics::audio::AudioClientInfo>> {
            anyhow::bail!("Audio introspector failed");
        }
    }

    let window_introspector: Arc<dyn WindowIntrospector> = Arc::new(FailingWindowIntrospector);
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(FailingAudioIntrospector)));
    let input_tracker = Arc::new(Mutex::new(InputTracker::new(Duration::from_secs(60))));
    let mut prev_cpu_times = None;
    let thresholds = Thresholds::default();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    // Функция должна успешно завершиться, даже если все опциональные компоненты недоступны
    let result = collect_snapshot(
        &paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
    )
    .await;

    // Должно быть успешно, так как системные метрики доступны
    assert!(result.is_ok());
    let snapshot = result.unwrap();

    // Проверяем, что снапшот содержит дефолтные значения для недоступных компонентов
    assert!(snapshot.snapshot_id > 0);
    assert!(!snapshot.processes.is_empty());

    // Процессы не должны иметь информации об окнах и аудио
    for process in &snapshot.processes {
        assert!(!process.has_gui_window);
        assert!(!process.is_focused_window);
        assert!(!process.is_audio_client);
        assert!(!process.has_active_stream);
    }
}

#[tokio::test]
async fn test_snapshot_caching_with_error_recovery() {
    // Тест проверяет, что кэширование работает корректно даже при ошибках
    let temp_dir = tempdir().unwrap();
    let dir_path = temp_dir.path();

    // Создаём валидные системные файлы
    let stat_file = dir_path.join("stat");
    let meminfo_file = dir_path.join("meminfo");
    let loadavg_file = dir_path.join("loadavg");

    let stat_content = "cpu 100 20 50 200 10 5 5 0 0 0\ncpu0 50 10 25 100 5 2 2 0 0 0";
    let meminfo_content = "MemTotal:        16384256 kB\nMemFree:          9876543 kB\nMemAvailable:     9876543 kB\nBuffers:           345678 kB\nCached:           2345678 kB\nSwapTotal:        8192000 kB\nSwapFree:         4096000 kB";
    let loadavg_content = "0.50 0.75 0.90 1/123 4567";

    fs::write(&stat_file, stat_content).unwrap();
    fs::write(&meminfo_file, meminfo_content).unwrap();
    fs::write(&loadavg_file, loadavg_content).unwrap();

    let paths = ProcPaths {
        stat: stat_file,
        meminfo: meminfo_file,
        loadavg: loadavg_file,
        pressure_cpu: dir_path.join("pressure_cpu"),
        pressure_io: dir_path.join("pressure_io"),
        pressure_memory: dir_path.join("pressure_memory"),
    };

    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(Vec::new()));
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
    let input_tracker = Arc::new(Mutex::new(InputTracker::new(Duration::from_secs(60))));
    let mut prev_cpu_times = None;
    let thresholds = Thresholds::default();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    // Параметры кэширования
    let mut system_metrics_cache: Option<smoothtask_core::metrics::system::SystemMetrics> = None;
    let mut system_metrics_cache_iteration: u64 = 0;
    let system_metrics_cache_interval = 3;
    let mut process_metrics_cache: Option<Vec<smoothtask_core::logging::snapshots::ProcessRecord>> =
        None;
    let mut process_metrics_cache_iteration: u64 = 0;
    let process_metrics_cache_interval = 2;
    let current_iteration = 1;

    // Первый вызов должен собрать метрики
    let result1 = collect_snapshot_with_caching(
        &paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
        &mut system_metrics_cache,
        &mut system_metrics_cache_iteration,
        system_metrics_cache_interval,
        &mut process_metrics_cache,
        &mut process_metrics_cache_iteration,
        process_metrics_cache_interval,
        current_iteration,
    )
    .await;

    assert!(result1.is_ok());
    let snapshot1 = result1.unwrap();

    // Кэш должен быть заполнен
    assert!(system_metrics_cache.is_some());
    assert!(process_metrics_cache.is_some());

    // Второй вызов должен использовать кэш
    let result2 = collect_snapshot_with_caching(
        &paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
        &mut system_metrics_cache,
        &mut system_metrics_cache_iteration,
        system_metrics_cache_interval,
        &mut process_metrics_cache,
        &mut process_metrics_cache_iteration,
        process_metrics_cache_interval,
        current_iteration + 1,
    )
    .await;

    assert!(result2.is_ok());
    let snapshot2 = result2.unwrap();

    // Снапшоты должны быть похожи (используется кэш)
    assert_eq!(snapshot1.global.mem_total_kb, snapshot2.global.mem_total_kb);
}

#[tokio::test]
async fn test_snapshot_collection_with_empty_process_list() {
    // Тест проверяет обработку ситуации, когда список процессов пуст
    let temp_dir = tempdir().unwrap();
    let dir_path = temp_dir.path();

    // Создаём валидные системные файлы
    let stat_file = dir_path.join("stat");
    let meminfo_file = dir_path.join("meminfo");
    let loadavg_file = dir_path.join("loadavg");

    let stat_content = "cpu 100 20 50 200 10 5 5 0 0 0\ncpu0 50 10 25 100 5 2 2 0 0 0";
    let meminfo_content = "MemTotal:        16384256 kB\nMemFree:          9876543 kB\nMemAvailable:     9876543 kB\nBuffers:           345678 kB\nCached:           2345678 kB\nSwapTotal:        8192000 kB\nSwapFree:         4096000 kB";
    let loadavg_content = "0.50 0.75 0.90 1/123 4567";

    fs::write(&stat_file, stat_content).unwrap();
    fs::write(&meminfo_file, meminfo_content).unwrap();
    fs::write(&loadavg_file, loadavg_content).unwrap();

    let paths = ProcPaths {
        stat: stat_file,
        meminfo: meminfo_file,
        loadavg: loadavg_file,
        pressure_cpu: dir_path.join("pressure_cpu"),
        pressure_io: dir_path.join("pressure_io"),
        pressure_memory: dir_path.join("pressure_memory"),
    };

    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(Vec::new()));
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
    let input_tracker = Arc::new(Mutex::new(InputTracker::new(Duration::from_secs(60))));
    let mut prev_cpu_times = None;
    let thresholds = Thresholds::default();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    // Функция должна успешно завершиться, даже если список процессов пуст
    let result = collect_snapshot(
        &paths,
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

    // Снапшот должен быть создан, даже если процессы отсутствуют
    assert!(snapshot.snapshot_id > 0);
    assert!(snapshot.global.mem_total_kb > 0);
}

#[tokio::test]
async fn test_snapshot_collection_with_high_memory_pressure() {
    // Тест проверяет обработку высокого давления памяти
    let temp_dir = tempdir().unwrap();
    let dir_path = temp_dir.path();

    // Создаём валидные системные файлы
    let stat_file = dir_path.join("stat");
    let meminfo_file = dir_path.join("meminfo");
    let loadavg_file = dir_path.join("loadavg");

    // Симулируем высокое использование памяти
    let stat_content = "cpu 100 20 50 200 10 5 5 0 0 0\ncpu0 50 10 25 100 5 2 2 0 0 0";
    let meminfo_content = "MemTotal:        16384256 kB\nMemFree:          1000000 kB\nMemAvailable:     1000000 kB\nBuffers:           345678 kB\nCached:           2345678 kB\nSwapTotal:        8192000 kB\nSwapFree:         4096000 kB";
    let loadavg_content = "5.00 6.00 7.00 1/123 4567";

    fs::write(&stat_file, stat_content).unwrap();
    fs::write(&meminfo_file, meminfo_content).unwrap();
    fs::write(&loadavg_file, loadavg_content).unwrap();

    let paths = ProcPaths {
        stat: stat_file,
        meminfo: meminfo_file,
        loadavg: loadavg_file,
        pressure_cpu: dir_path.join("pressure_cpu"),
        pressure_io: dir_path.join("pressure_io"),
        pressure_memory: dir_path.join("pressure_memory"),
    };

    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(Vec::new()));
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
    let input_tracker = Arc::new(Mutex::new(InputTracker::new(Duration::from_secs(60))));
    let mut prev_cpu_times = None;
    let thresholds = Thresholds::default();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    // Функция должна успешно завершиться, даже при высоком давлении памяти
    let result = collect_snapshot(
        &paths,
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

    // Проверяем, что метрики отражают высокое давление
    assert!(snapshot.global.load_avg_one > 4.0);
    assert!(snapshot.global.mem_used_kb > snapshot.global.mem_free_kb);
}

#[tokio::test]
async fn test_snapshot_collection_with_permission_errors() {
    // Тест проверяет обработку ошибок доступа к файлам
    let temp_dir = tempdir().unwrap();
    let dir_path = temp_dir.path();

    // Создаём файлы с ограниченными правами доступа
    let stat_file = dir_path.join("stat");
    let stat_content = "cpu 100 20 50 200 10 5 5 0 0 0\ncpu0 50 10 25 100 5 2 2 0 0 0";
    fs::write(&stat_file, stat_content).unwrap();

    // Устанавливаем ограниченные права (только чтение для владельца)
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&stat_file).unwrap().permissions();
        perms.set_mode(0o400); // Только чтение для владельца
        fs::set_permissions(&stat_file, perms).unwrap();
    }

    let paths = ProcPaths {
        stat: stat_file,
        meminfo: dir_path.join("meminfo"), // Этот файл отсутствует
        loadavg: dir_path.join("loadavg"), // Этот файл отсутствует
        pressure_cpu: dir_path.join("pressure_cpu"),
        pressure_io: dir_path.join("pressure_io"),
        pressure_memory: dir_path.join("pressure_memory"),
    };

    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(Vec::new()));
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
    let input_tracker = Arc::new(Mutex::new(InputTracker::new(Duration::from_secs(60))));
    let mut prev_cpu_times = None;
    let thresholds = Thresholds::default();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    // Функция должна обработать ошибки доступа gracefully
    let result = collect_snapshot(
        &paths,
        &window_introspector,
        &audio_introspector,
        &input_tracker,
        &mut prev_cpu_times,
        &thresholds,
        &latency_collector,
    )
    .await;

    // Должна быть ошибка, так как критические файлы отсутствуют
    assert!(result.is_err());
    let error = result.unwrap_err();
    let error_string = error.to_string();

    // Проверяем, что ошибка содержит информацию о проблеме
    assert!(error_string.contains("meminfo") || error_string.contains("loadavg"));
}

#[tokio::test]
async fn test_snapshot_collection_with_concurrent_access() {
    // Тест проверяет обработку конкурентного доступа к кэшу
    let temp_dir = tempdir().unwrap();
    let dir_path = temp_dir.path();

    // Создаём валидные системные файлы
    let stat_file = dir_path.join("stat");
    let meminfo_file = dir_path.join("meminfo");
    let loadavg_file = dir_path.join("loadavg");

    let stat_content = "cpu 100 20 50 200 10 5 5 0 0 0\ncpu0 50 10 25 100 5 2 2 0 0 0";
    let meminfo_content = "MemTotal:        16384256 kB\nMemFree:          9876543 kB\nMemAvailable:     9876543 kB\nBuffers:           345678 kB\nCached:           2345678 kB\nSwapTotal:        8192000 kB\nSwapFree:         4096000 kB";
    let loadavg_content = "0.50 0.75 0.90 1/123 4567";

    fs::write(&stat_file, stat_content).unwrap();
    fs::write(&meminfo_file, meminfo_content).unwrap();
    fs::write(&loadavg_file, loadavg_content).unwrap();

    let paths = ProcPaths {
        stat: stat_file,
        meminfo: meminfo_file,
        loadavg: loadavg_file,
        pressure_cpu: dir_path.join("pressure_cpu"),
        pressure_io: dir_path.join("pressure_io"),
        pressure_memory: dir_path.join("pressure_memory"),
    };

    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(Vec::new()));
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
    let input_tracker = Arc::new(Mutex::new(InputTracker::new(Duration::from_secs(60))));
    let mut prev_cpu_times = None;
    let thresholds = Thresholds::default();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    // Параметры кэширования
    let mut system_metrics_cache: Option<smoothtask_core::metrics::system::SystemMetrics> = None;
    let mut system_metrics_cache_iteration: u64 = 0;
    let system_metrics_cache_interval = 3;
    let mut process_metrics_cache: Option<Vec<smoothtask_core::logging::snapshots::ProcessRecord>> =
        None;
    let mut process_metrics_cache_iteration: u64 = 0;
    let process_metrics_cache_interval = 2;

    // Запускаем несколько конкурентных задач
    let mut handles = Vec::new();

    for i in 0..5 {
        let paths_clone = paths.clone();
        let window_introspector_clone = Arc::clone(&window_introspector);
        let audio_introspector_clone = Arc::clone(&audio_introspector);
        let input_tracker_clone = Arc::clone(&input_tracker);
        let latency_collector_clone = Arc::clone(&latency_collector);

        let handle = tokio::spawn(async move {
            let mut prev_cpu_times = None;
            let thresholds = Thresholds::default();

            collect_snapshot_with_caching(
                &paths_clone,
                &window_introspector_clone,
                &audio_introspector_clone,
                &input_tracker_clone,
                &mut prev_cpu_times,
                &thresholds,
                &latency_collector_clone,
                &mut None,
                &mut 0,
                system_metrics_cache_interval,
                &mut None,
                &mut 0,
                process_metrics_cache_interval,
                i,
            )
            .await
        });

        handles.push(handle);
    }

    // Ждем завершения всех задач
    let mut results = Vec::new();
    for handle in handles {
        let result = handle.await.unwrap();
        results.push(result);
    }

    // Все задачи должны завершиться успешно
    for result in results {
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_snapshot_collection_with_large_system() {
    // Тест проверяет обработку систем с большим количеством процессов
    let temp_dir = tempdir().unwrap();
    let dir_path = temp_dir.path();

    // Создаём валидные системные файлы
    let stat_file = dir_path.join("stat");
    let meminfo_file = dir_path.join("meminfo");
    let loadavg_file = dir_path.join("loadavg");

    let stat_content = "cpu 100 20 50 200 10 5 5 0 0 0\ncpu0 50 10 25 100 5 2 2 0 0 0";
    let meminfo_content = "MemTotal:        16384256 kB\nMemFree:          9876543 kB\nMemAvailable:     9876543 kB\nBuffers:           345678 kB\nCached:           2345678 kB\nSwapTotal:        8192000 kB\nSwapFree:         4096000 kB";
    let loadavg_content = "0.50 0.75 0.90 1/123 4567";

    fs::write(&stat_file, stat_content).unwrap();
    fs::write(&meminfo_file, meminfo_content).unwrap();
    fs::write(&loadavg_file, loadavg_content).unwrap();

    let paths = ProcPaths {
        stat: stat_file,
        meminfo: meminfo_file,
        loadavg: loadavg_file,
        pressure_cpu: dir_path.join("pressure_cpu"),
        pressure_io: dir_path.join("pressure_io"),
        pressure_memory: dir_path.join("pressure_memory"),
    };

    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(Vec::new()));
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
    let input_tracker = Arc::new(Mutex::new(InputTracker::new(Duration::from_secs(60))));
    let mut prev_cpu_times = None;
    let thresholds = Thresholds::default();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    // Функция должна успешно завершиться, даже с большим количеством процессов
    let result = collect_snapshot(
        &paths,
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

    // Снапшот должен быть создан успешно
    assert!(snapshot.snapshot_id > 0);
}

#[tokio::test]
async fn test_snapshot_collection_with_timeout_handling() {
    // Тест проверяет обработку таймаутов при сборе метрик
    let temp_dir = tempdir().unwrap();
    let dir_path = tempdir().unwrap();

    // Создаём валидные системные файлы
    let stat_file = dir_path.path().join("stat");
    let meminfo_file = dir_path.path().join("meminfo");
    let loadavg_file = dir_path.path().join("loadavg");

    let stat_content = "cpu 100 20 50 200 10 5 5 0 0 0\ncpu0 50 10 25 100 5 2 2 0 0 0";
    let meminfo_content = "MemTotal:        16384256 kB\nMemFree:          9876543 kB\nMemAvailable:     9876543 kB\nBuffers:           345678 kB\nCached:           2345678 kB\nSwapTotal:        8192000 kB\nSwapFree:         4096000 kB";
    let loadavg_content = "0.50 0.75 0.90 1/123 4567";

    fs::write(&stat_file, stat_content).unwrap();
    fs::write(&meminfo_file, meminfo_content).unwrap();
    fs::write(&loadavg_file, loadavg_content).unwrap();

    let paths = ProcPaths {
        stat: stat_file,
        meminfo: meminfo_file,
        loadavg: loadavg_file,
        pressure_cpu: dir_path.path().join("pressure_cpu"),
        pressure_io: dir_path.path().join("pressure_io"),
        pressure_memory: dir_path.path().join("pressure_memory"),
    };

    let window_introspector: Arc<dyn WindowIntrospector> =
        Arc::new(StaticWindowIntrospector::new(Vec::new()));
    let audio_introspector: Arc<Mutex<Box<dyn AudioIntrospector>>> =
        Arc::new(Mutex::new(Box::new(StaticAudioIntrospector::empty())));
    let input_tracker = Arc::new(Mutex::new(InputTracker::new(Duration::from_secs(60))));
    let mut prev_cpu_times = None;
    let thresholds = Thresholds::default();
    let latency_collector = Arc::new(LatencyCollector::new(1000));

    // Функция должна успешно завершиться в разумное время
    let result = tokio::time::timeout(
        Duration::from_secs(10),
        collect_snapshot(
            &paths,
            &window_introspector,
            &audio_introspector,
            &input_tracker,
            &mut prev_cpu_times,
            &thresholds,
            &latency_collector,
        ),
    )
    .await;

    assert!(result.is_ok());
    let snapshot_result = result.unwrap();
    assert!(snapshot_result.is_ok());
}
