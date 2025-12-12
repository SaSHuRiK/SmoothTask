//! Бенчмарки для измерения производительности основных функций SmoothTask.
//!
//! Эти бенчмарки помогают измерить производительность критических путей
//! в SmoothTask, включая сбор метрик, обработку процессов и другие операции.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use smoothtask_core::metrics::process::collect_process_metrics;
use smoothtask_core::metrics::system::{collect_system_metrics, ProcPaths};
use smoothtask_core::metrics::windows::{build_pid_to_window_map, get_window_info_by_pid, select_focused_window, StaticWindowIntrospector, WindowInfo, WindowState};
use smoothtask_core::config::{CacheIntervals, Config, Paths, PolicyMode, Thresholds};
use std::path::PathBuf;

/// Бенчмарк для измерения времени выполнения простой операции
///
/// Этот бенчмарк измеряет производительность простой операции для проверки
/// работоспособности системы бенчмаркинга.
fn benchmark_simple_operation(c: &mut Criterion) {
    c.bench_function("simple_operation", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for i in 0..1000 {
                sum = black_box(sum.wrapping_add(black_box(i)));
            }
            sum
        })
    });
}

/// Бенчмарк для измерения времени выполнения операции с выделением памяти
///
/// Этот бенчмарк измеряет производительность операций с выделением памяти.
fn benchmark_memory_allocation(c: &mut Criterion) {
    c.bench_function("memory_allocation", |b| {
        b.iter(|| {
            let mut vec = Vec::new();
            for i in 0..1000 {
                vec.push(black_box(i));
            }
            vec.len()
        })
    });
}

/// Бенчмарк для измерения времени выполнения строковых операций
///
/// Этот бенчмарк измеряет производительность операций со строками.
fn benchmark_string_operations(c: &mut Criterion) {
    c.bench_function("string_operations", |b| {
        b.iter(|| {
            let mut s = String::new();
            for i in 0..100 {
                s.push_str(black_box(&i.to_string()));
            }
            s.len()
        })
    });
}

/// Бенчмарк для измерения времени сбора системных метрик
///
/// Этот бенчмарк измеряет производительность функции collect_system_metrics,
/// которая собирает метрики CPU, памяти и других системных ресурсов.
fn benchmark_system_metrics_collection(c: &mut Criterion) {
    let proc_paths = ProcPaths {
        stat: PathBuf::from("/proc/stat"),
        meminfo: PathBuf::from("/proc/meminfo"),
        loadavg: PathBuf::from("/proc/loadavg"),
        pressure_cpu: PathBuf::from("/proc/pressure/cpu"),
        pressure_io: PathBuf::from("/proc/pressure/io"),
        pressure_memory: PathBuf::from("/proc/pressure/memory"),
    };

    c.bench_function("system_metrics_collection", |b| {
        b.iter(|| {
            let _result = collect_system_metrics(black_box(&proc_paths));
        })
    });
}

/// Бенчмарк для измерения времени сбора метрик процессов
///
/// Этот бенчмарк измеряет производительность функции collect_process_metrics,
/// которая собирает метрики для всех процессов в системе.
fn benchmark_process_metrics_collection(c: &mut Criterion) {
    c.bench_function("process_metrics_collection", |b| {
        b.iter(|| {
            let _result = collect_process_metrics();
        })
    });
}

/// Бенчмарк для измерения времени обработки данных процессов
///
/// Этот бенчмарк измеряет производительность обработки данных процессов,
/// включая фильтрацию и преобразование.
fn benchmark_process_data_processing(c: &mut Criterion) {
    c.bench_function("process_data_processing", |b| {
        b.iter(|| {
            // Собираем метрики процессов
            let processes = collect_process_metrics().unwrap_or_default();
            
            // Фильтруем и обрабатываем данные
            let filtered: Vec<_> = processes
                .into_iter()
                .filter(|p| black_box(p.pid) > 100) // Фильтруем системные процессы
                .map(|p| {
                    // Преобразуем данные
                    let cpu_usage = p.cpu_share_1s.unwrap_or(0.0);
                    let mem_usage = p.rss_mb.unwrap_or(0);
                    (p.pid, cpu_usage, mem_usage)
                })
                .collect();
            
            black_box(filtered)
        })
    });
}

/// Бенчмарк для измерения времени создания статического интроспектора окон
///
/// Этот бенчмарк измеряет производительность создания StaticWindowIntrospector
/// с тестовыми данными.
fn benchmark_static_window_introspector_creation(c: &mut Criterion) {
    // Создаем тестовые данные окон
    let test_windows = vec![
        WindowInfo::new(
            Some("test.app".to_string()),
            Some("Test Window 1".to_string()),
            Some(1),
            WindowState::Background,
            Some(100),
            1.0,
        ),
        WindowInfo::new(
            Some("test.app".to_string()),
            Some("Test Window 2".to_string()),
            Some(2),
            WindowState::Minimized,
            Some(200),
            0.8,
        ),
    ];

    c.bench_function("static_window_introspector_creation", |b| {
        b.iter(|| {
            let introspector = StaticWindowIntrospector::new(black_box(test_windows.clone()));
            black_box(introspector)
        })
    });
}

/// Бенчмарк для измерения времени выбора фокусного окна
///
/// Этот бенчмарк измеряет производительность функции select_focused_window
/// при работе с набором окон.
fn benchmark_select_focused_window(c: &mut Criterion) {
    // Создаем тестовые данные окон
    let test_windows = vec![
        WindowInfo::new(
            Some("test.app".to_string()),
            Some("Normal Window".to_string()),
            Some(1),
            WindowState::Background,
            Some(100),
            1.0,
        ),
        WindowInfo::new(
            Some("test.app".to_string()),
            Some("Fullscreen Window".to_string()),
            Some(2),
            WindowState::Fullscreen,
            Some(200),
            0.9,
        ),
        WindowInfo::new(
            Some("test.app".to_string()),
            Some("Minimized Window".to_string()),
            Some(3),
            WindowState::Minimized,
            Some(300),
            0.8,
        ),
    ];

    c.bench_function("select_focused_window", |b| {
        b.iter(|| {
            let result = select_focused_window(black_box(&test_windows));
            black_box(result)
        })
    });
}

/// Бенчмарк для измерения времени построения карты PID к окнам
///
/// Этот бенчмарк измеряет производительность функции build_pid_to_window_map
/// при работе с набором окон.
fn benchmark_build_pid_to_window_map(c: &mut Criterion) {
    // Создаем тестовые данные окон
    let test_windows = vec![
        WindowInfo::new(
            Some("test.app".to_string()),
            Some("Window 1".to_string()),
            Some(1),
            WindowState::Background,
            Some(100),
            1.0,
        ),
        WindowInfo::new(
            Some("test.app".to_string()),
            Some("Window 2".to_string()),
            Some(2),
            WindowState::Background,
            Some(200),
            0.9,
        ),
        WindowInfo::new(
            Some("test.app".to_string()),
            Some("Window 1 Alt".to_string()),
            Some(1),
            WindowState::Background,
            Some(100), // Дублирующийся PID для тестирования
            0.7,
        ),
    ];

    let introspector = StaticWindowIntrospector::new(test_windows);

    c.bench_function("build_pid_to_window_map", |b| {
        b.iter(|| {
            let result = build_pid_to_window_map(black_box(&introspector));
            black_box(result)
        })
    });
}

/// Бенчмарк для измерения времени получения информации об окне по PID
///
/// Этот бенчмарк измеряет производительность функции get_window_info_by_pid
/// при работе с набором окон.
fn benchmark_get_window_info_by_pid(c: &mut Criterion) {
    // Создаем тестовые данные окон
    let test_windows = vec![
        WindowInfo::new(
            Some("test.app".to_string()),
            Some("Window 1".to_string()),
            Some(1),
            WindowState::Background,
            Some(100),
            1.0,
        ),
        WindowInfo::new(
            Some("test.app".to_string()),
            Some("Window 2".to_string()),
            Some(2),
            WindowState::Background,
            Some(200),
            0.9,
        ),
    ];

    let introspector = StaticWindowIntrospector::new(test_windows);

    c.bench_function("get_window_info_by_pid", |b| {
        b.iter(|| {
            let result = get_window_info_by_pid(black_box(&introspector), black_box(100));
            black_box(result)
        })
    });
}

/// Бенчмарк для измерения времени создания конфигурации
///
/// Этот бенчмарк измеряет производительность создания Config
/// с тестовыми параметрами.
fn benchmark_config_creation(c: &mut Criterion) {
    c.bench_function("config_creation", |b| {
        b.iter(|| {
            let config = Config {
                polling_interval_ms: 1000,
                max_candidates: 150,
                dry_run_default: false,
                policy_mode: PolicyMode::Hybrid,
                enable_snapshot_logging: false,
                thresholds: Thresholds {
                    psi_cpu_some_high: 0.5,
                    psi_io_some_high: 0.5,
                    user_idle_timeout_sec: 300,
                    interactive_build_grace_sec: 60,
                    noisy_neighbour_cpu_share: 0.2,
                    crit_interactive_percentile: 0.9,
                    interactive_percentile: 0.7,
                    normal_percentile: 0.5,
                    background_percentile: 0.3,
                    sched_latency_p99_threshold_ms: 20.0,
                    ui_loop_p95_threshold_ms: 16.67,
                },
                paths: Paths {
                    snapshot_db_path: "/var/log/smoothtask/snapshots.db".to_string(),
                    patterns_dir: "/etc/smoothtask/patterns".to_string(),
                    api_listen_addr: Some("127.0.0.1:8080".to_string()),
                },
                cache_intervals: CacheIntervals {
                    system_metrics_cache_interval: 5,
                    process_metrics_cache_interval: 2,
                },
            };
            black_box(config)
        })
    });
}

/// Бенчмарк для измерения времени сериализации конфигурации
///
/// Этот бенчмарк измеряет производительность сериализации Config
/// в YAML формат.
fn benchmark_config_serialization(c: &mut Criterion) {
    let config = Config {
        polling_interval_ms: 1000,
        max_candidates: 150,
        dry_run_default: false,
        policy_mode: PolicyMode::Hybrid,
        enable_snapshot_logging: false,
        thresholds: Thresholds {
            psi_cpu_some_high: 0.5,
            psi_io_some_high: 0.5,
            user_idle_timeout_sec: 300,
            interactive_build_grace_sec: 60,
            noisy_neighbour_cpu_share: 0.2,
            crit_interactive_percentile: 0.9,
            interactive_percentile: 0.7,
            normal_percentile: 0.5,
            background_percentile: 0.3,
            sched_latency_p99_threshold_ms: 20.0,
            ui_loop_p95_threshold_ms: 16.67,
        },
        paths: Paths {
            snapshot_db_path: "/var/log/smoothtask/snapshots.db".to_string(),
            patterns_dir: "/etc/smoothtask/patterns".to_string(),
            api_listen_addr: Some("127.0.0.1:8080".to_string()),
        },
        cache_intervals: CacheIntervals {
            system_metrics_cache_interval: 5,
            process_metrics_cache_interval: 2,
        },
    };

    c.bench_function("config_serialization", |b| {
        b.iter(|| {
            let serialized = serde_yaml::to_string(black_box(&config)).unwrap();
            black_box(serialized)
        })
    });
}

criterion_group! {
    name = smoothtask_benchmarks;
    config = Criterion::default()
        .sample_size(10) // Уменьшаем размер выборки для более быстрого выполнения
        .warm_up_time(std::time::Duration::from_secs(1)) // Уменьшаем время для разогрева
        .measurement_time(std::time::Duration::from_secs(3)); // Уменьшаем время измерения
    targets = 
        benchmark_simple_operation,
        benchmark_memory_allocation,
        benchmark_string_operations,
        benchmark_system_metrics_collection,
        benchmark_process_metrics_collection,
        benchmark_process_data_processing,
        benchmark_static_window_introspector_creation,
        benchmark_select_focused_window,
        benchmark_build_pid_to_window_map,
        benchmark_get_window_info_by_pid,
        benchmark_config_creation,
        benchmark_config_serialization
}

criterion_main!(smoothtask_benchmarks);