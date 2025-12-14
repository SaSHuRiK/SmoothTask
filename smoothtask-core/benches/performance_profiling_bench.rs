//! Бенчмарки для профилирования и оптимизации производительности SmoothTask.
//!
//! Этот модуль содержит бенчмарки, специально предназначенные для:
//! 1. Профилирования производительности критических путей
//! 2. Идентификации узких мест
//! 3. Измерения эффективности оптимизаций
//! 4. Обеспечения регрессионного тестирования производительности

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use smoothtask_core::config::config_struct::Config;
use smoothtask_core::metrics::process::collect_process_metrics;
use smoothtask_core::metrics::system::{collect_system_metrics, ProcPaths};
use std::path::PathBuf;

/// Бенчмарк для измерения полного цикла сбора метрик
///
/// Этот бенчмарк измеряет производительность полного цикла:
/// 1. Сбор системных метрик
/// 2. Сбор метрик процессов
///
/// Это наиболее реалистичный сценарий, который имитирует основной цикл работы SmoothTask.
fn benchmark_full_metrics_collection(c: &mut Criterion) {
    let proc_paths = ProcPaths {
        stat: PathBuf::from("/proc/stat"),
        meminfo: PathBuf::from("/proc/meminfo"),
        loadavg: PathBuf::from("/proc/loadavg"),
        pressure_cpu: PathBuf::from("/proc/pressure/cpu"),
        pressure_io: PathBuf::from("/proc/pressure/io"),
        pressure_memory: PathBuf::from("/proc/pressure/memory"),
    };

    c.bench_function("full_metrics_collection", |b| {
        b.iter(|| {
            // 1. Сбор системных метрик
            let system_metrics = collect_system_metrics(black_box(&proc_paths)).ok();

            // 2. Сбор метрик процессов
            let processes = collect_process_metrics(None).unwrap_or_default();

            black_box((system_metrics, processes))
        })
    });
}

/// Бенчмарк для измерения производительности сбора системных метрик
///
/// Этот бенчмарк тестирует производительность функции collect_system_metrics,
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

/// Бенчмарк для измерения производительности сбора метрик процессов
///
/// Этот бенчмарк тестирует производительность функции collect_process_metrics,
/// которая собирает метрики для всех процессов в системе.
fn benchmark_process_metrics_collection(c: &mut Criterion) {
    c.bench_function("process_metrics_collection", |b| {
        b.iter(|| {
            let _result = collect_process_metrics(None);
        })
    });
}

/// Бенчмарк для измерения производительности обработки данных процессов
///
/// Этот бенчмарк тестирует производительность обработки данных процессов,
/// включая фильтрацию и преобразование.
fn benchmark_process_data_processing(c: &mut Criterion) {
    c.bench_function("process_data_processing", |b| {
        b.iter(|| {
            // Собираем метрики процессов
            let processes = collect_process_metrics(None).unwrap_or_default();

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

/// Бенчмарк для измерения производительности загрузки конфигурации
///
/// Этот бенчмарк тестирует производительность загрузки и обработки конфигурации,
/// что является критическим путем при запуске SmoothTask.
fn benchmark_config_loading(c: &mut Criterion) {
    let config_path = "configs/smoothtask.example.yml";

    c.bench_function("config_loading", |b| {
        b.iter(|| {
            let config_result = Config::load(config_path);
            black_box(config_result)
        })
    });
}

/// Бенчмарк для измерения производительности сериализации и десериализации данных
///
/// Этот бенчмарк тестирует производительность работы с JSON и YAML,
/// что важно для логирования и хранения данных.
fn benchmark_serialization(c: &mut Criterion) {
    // Создаем тестовые данные процессов
    let processes = collect_process_metrics(None).unwrap_or_default();

    c.bench_function("json_serialization", |b| {
        b.iter(|| {
            let serialized = serde_json::to_string(black_box(&processes)).unwrap();
            black_box(serialized)
        })
    });

    c.bench_function("json_deserialization", |b| {
        let serialized = serde_json::to_string(&processes).unwrap();
        b.iter(|| {
            let deserialized: Vec<smoothtask_core::logging::snapshots::ProcessRecord> =
                serde_json::from_str(black_box(&serialized)).unwrap();
            black_box(deserialized)
        })
    });
}

/// Бенчмарк для измерения производительности работы с файловой системой
///
/// Этот бенчмарк тестирует производительность операций с файловой системой,
/// которые используются для мониторинга конфигурационных файлов.
fn benchmark_filesystem_operations(c: &mut Criterion) {
    use std::fs;

    let test_dir = PathBuf::from("configs/patterns");

    c.bench_function("filesystem_listing", |b| {
        b.iter(|| {
            let entries = fs::read_dir(black_box(&test_dir)).unwrap();
            let count = entries.count();
            black_box(count)
        })
    });
}

/// Бенчмарк для измерения производительности параллельной обработки
///
/// Этот бенчмарк тестирует производительность параллельной обработки данных,
/// что важно для масштабируемости SmoothTask.
fn benchmark_parallel_processing(c: &mut Criterion) {
    use rayon::prelude::*;

    let processes = collect_process_metrics(None).unwrap_or_default();

    c.bench_function("parallel_process_processing", |b| {
        b.iter(|| {
            let result: Vec<_> = processes
                .par_iter()
                .filter(|p| p.cpu_share_1s.unwrap_or(0.0) > 0.1)
                .map(|p| {
                    let cpu_usage = p.cpu_share_1s.unwrap_or(0.0);
                    let mem_usage = p.rss_mb.unwrap_or(0);
                    (p.pid, cpu_usage, mem_usage)
                })
                .collect();
            black_box(result)
        })
    });
}

/// Бенчмарк для измерения производительности работы с кэшем процессов
///
/// Этот бенчмарк тестирует производительность кэширования метрик процессов,
/// что является важным для оптимизации производительности.
fn benchmark_process_cache_operations(c: &mut Criterion) {
    use smoothtask_core::metrics::process::{collect_process_metrics, ProcessCacheConfig};

    let config = ProcessCacheConfig::default();

    c.bench_function("process_cache_operations", |b| {
        b.iter(|| {
            let cached_processes = collect_process_metrics(Some(config.clone()));
            black_box(cached_processes)
        })
    });
}

/// Бенчмарк для измерения производительности обработки конфигурации
///
/// Этот бенчмарк тестирует производительность создания и обработки конфигурации,
/// что является критическим путем при запуске SmoothTask.
fn benchmark_config_creation(c: &mut Criterion) {
    c.bench_function("config_creation", |b| {
        b.iter(|| {
            let config = Config {
                polling_interval_ms: 1000,
                max_candidates: 150,
                ..Default::default()
            };
            black_box(config)
        })
    });
}

/// Бенчмарк для измерения производительности сериализации конфигурации
///
/// Этот бенчмарк тестирует производительность сериализации Config
/// в YAML формат.
fn benchmark_config_serialization(c: &mut Criterion) {
    let config = Config {
        polling_interval_ms: 1000,
        max_candidates: 150,
        ..Default::default()
    };

    c.bench_function("config_serialization", |b| {
        b.iter(|| {
            let serialized = serde_yaml::to_string(black_box(&config)).unwrap();
            black_box(serialized)
        })
    });
}

/// Бенчмарк для измерения производительности сбора температуры CPU
///
/// Этот бенчмарк тестирует производительность функции collect_cpu_temperature,
/// которая собирает температуру CPU из системных файлов.
fn benchmark_cpu_temperature_collection(c: &mut Criterion) {
    c.bench_function("cpu_temperature_collection", |b| {
        b.iter(|| {
            let _result = smoothtask_core::metrics::system::collect_cpu_temperature();
        })
    });
}

criterion_group! {
    name = performance_profiling_benchmarks;
    config = Criterion::default()
        .sample_size(20)  // Увеличиваем размер выборки для более точных результатов
        .warm_up_time(std::time::Duration::from_secs(2))
        .measurement_time(std::time::Duration::from_secs(10));
    targets =
        benchmark_full_metrics_collection,
        benchmark_system_metrics_collection,
        benchmark_process_metrics_collection,
        benchmark_process_data_processing,
        benchmark_config_loading,
        benchmark_serialization,
        benchmark_filesystem_operations,
        benchmark_parallel_processing,
        benchmark_process_cache_operations,
        benchmark_config_creation,
        benchmark_config_serialization,
        benchmark_cpu_temperature_collection
}

criterion_main!(performance_profiling_benchmarks);
