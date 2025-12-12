//! Бенчмарки для измерения производительности сбора метрик процессов с оптимизациями.
//!
//! Эти бенчмарки помогают измерить производительность различных конфигураций
//! сбора метрик процессов, включая кэширование и параллельную обработку.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use smoothtask_core::metrics::process::{
    clear_process_cache, collect_process_metrics, ProcessCacheConfig,
};

/// Бенчмарк для измерения времени сбора метрик процессов без кэширования
///
/// Этот бенчмарк измеряет производительность функции collect_process_metrics
/// без использования кэширования.
fn benchmark_process_metrics_no_cache(c: &mut Criterion) {
    // Очищаем кэш перед бенчмарком
    clear_process_cache();

    let config = ProcessCacheConfig {
        enable_caching: false,
        enable_parallel_processing: true,
        ..Default::default()
    };

    c.bench_function("process_metrics_no_cache", |b| {
        b.iter(|| {
            let _result = collect_process_metrics(Some(config.clone()));
        })
    });
}

/// Бенчмарк для измерения времени сбора метрик процессов с кэшированием
///
/// Этот бенчмарк измеряет производительность функции collect_process_metrics
/// с использованием кэширования.
fn benchmark_process_metrics_with_cache(c: &mut Criterion) {
    // Очищаем кэш перед бенчмарком
    clear_process_cache();

    let config = ProcessCacheConfig {
        enable_caching: true,
        cache_ttl_seconds: 300, // Длительный TTL для бенчмарка
        max_cached_processes: 5000,
        enable_parallel_processing: true,
        ..Default::default()
    };

    c.bench_function("process_metrics_with_cache", |b| {
        b.iter(|| {
            let _result = collect_process_metrics(Some(config.clone()));
        })
    });
}

/// Бенчмарк для измерения времени сбора метрик процессов с последовательной обработкой
///
/// Этот бенчмарк измеряет производительность функции collect_process_metrics
/// с последовательной обработкой (без параллелизма).
fn benchmark_process_metrics_sequential(c: &mut Criterion) {
    // Очищаем кэш перед бенчмарком
    clear_process_cache();

    let config = ProcessCacheConfig {
        enable_caching: false,
        enable_parallel_processing: false,
        ..Default::default()
    };

    c.bench_function("process_metrics_sequential", |b| {
        b.iter(|| {
            let _result = collect_process_metrics(Some(config.clone()));
        })
    });
}

/// Бенчмарк для измерения времени сбора метрик процессов с ограниченным параллелизмом
///
/// Этот бенчмарк измеряет производительность функции collect_process_metrics
/// с ограниченным количеством параллельных потоков.
fn benchmark_process_metrics_limited_threads(c: &mut Criterion) {
    // Очищаем кэш перед бенчмарком
    clear_process_cache();

    let config = ProcessCacheConfig {
        enable_caching: false,
        enable_parallel_processing: true,
        max_parallel_threads: Some(4), // Ограничиваем до 4 потоков
        ..Default::default()
    };

    c.bench_function("process_metrics_limited_threads", |b| {
        b.iter(|| {
            let _result = collect_process_metrics(Some(config.clone()));
        })
    });
}

/// Бенчмарк для измерения времени сбора метрик процессов с кэшированием и ограниченным параллелизмом
///
/// Этот бенчмарк измеряет производительность функции collect_process_metrics
/// с кэшированием и ограниченным количеством параллельных потоков.
fn benchmark_process_metrics_cache_limited_threads(c: &mut Criterion) {
    // Очищаем кэш перед бенчмарком
    clear_process_cache();

    let config = ProcessCacheConfig {
        enable_caching: true,
        cache_ttl_seconds: 300, // Длительный TTL для бенчмарка
        max_cached_processes: 5000,
        enable_parallel_processing: true,
        max_parallel_threads: Some(4), // Ограничиваем до 4 потоков
        ..Default::default()
    };

    c.bench_function("process_metrics_cache_limited_threads", |b| {
        b.iter(|| {
            let _result = collect_process_metrics(Some(config.clone()));
        })
    });
}

/// Бенчмарк для измерения времени кэширования процессов
///
/// Этот бенчмарк измеряет производительность кэширования метрик процессов
/// при первом вызове и последующих вызовах с кэшем.
fn benchmark_process_metrics_cache_warmup(c: &mut Criterion) {
    // Очищаем кэш перед бенчмарком
    clear_process_cache();

    let config = ProcessCacheConfig {
        enable_caching: true,
        cache_ttl_seconds: 300, // Длительный TTL для бенчмарка
        max_cached_processes: 5000,
        enable_parallel_processing: true,
        ..Default::default()
    };

    c.bench_function("process_metrics_cache_warmup", |b| {
        // Первый вызов - холодный кэш
        let _first_result = collect_process_metrics(Some(config.clone()));

        b.iter(|| {
            // Последующие вызовы - теплый кэш
            let _result = collect_process_metrics(Some(config.clone()));
        })
    });
}

/// Бенчмарк для измерения времени сбора метрик процессов с разными размерами кэша
///
/// Этот бенчмарк измеряет производительность функции collect_process_metrics
/// с разными ограничениями на размер кэша.
fn benchmark_process_metrics_cache_size_variations(c: &mut Criterion) {
    // Очищаем кэш перед бенчмарком
    clear_process_cache();

    let cache_sizes = [100, 500, 1000, 2000, 5000];

    for &size in &cache_sizes {
        let config = ProcessCacheConfig {
            enable_caching: true,
            cache_ttl_seconds: 300,
            max_cached_processes: size,
            enable_parallel_processing: true,
            ..Default::default()
        };

        c.bench_function(&format!("process_metrics_cache_size_{}", size), |b| {
            b.iter(|| {
                let _result = collect_process_metrics(Some(config.clone()));
            })
        });
    }
}

/// Бенчмарк для измерения времени сбора метрик процессов с разными TTL кэша
///
/// Этот бенчмарк измеряет производительность функции collect_process_metrics
/// с разными значениями TTL кэша.
fn benchmark_process_metrics_cache_ttl_variations(c: &mut Criterion) {
    // Очищаем кэш перед бенчмарком
    clear_process_cache();

    let ttl_values = [1, 5, 10, 30, 60];

    for &ttl in &ttl_values {
        let config = ProcessCacheConfig {
            enable_caching: true,
            cache_ttl_seconds: ttl,
            max_cached_processes: 1000,
            enable_parallel_processing: true,
            ..Default::default()
        };

        c.bench_function(&format!("process_metrics_cache_ttl_{}", ttl), |b| {
            b.iter(|| {
                let _result = collect_process_metrics(Some(config.clone()));
            })
        });
    }
}

criterion_group! {
    name = process_metrics_benchmarks;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(5));
    targets =
        benchmark_process_metrics_no_cache,
        benchmark_process_metrics_with_cache,
        benchmark_process_metrics_sequential,
        benchmark_process_metrics_limited_threads,
        benchmark_process_metrics_cache_limited_threads,
        benchmark_process_metrics_cache_warmup,
        benchmark_process_metrics_cache_size_variations,
        benchmark_process_metrics_cache_ttl_variations
}

criterion_main!(process_metrics_benchmarks);
