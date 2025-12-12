//! Бенчмарки для измерения производительности eBPF метрик
//!
//! Эти бенчмарки помогают измерить производительность eBPF функциональности
//! до и после оптимизаций.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use smoothtask_core::metrics::ebpf::{EbpfConfig, EbpfMetricsCollector};
use std::time::Duration;

/// Бенчмарк для измерения времени инициализации eBPF коллектора
fn benchmark_ebpf_initialization(c: &mut Criterion) {
    c.bench_function("ebpf_initialization", |b| {
        b.iter(|| {
            let config = EbpfConfig::default();
            let mut collector = EbpfMetricsCollector::new(config);
            black_box(collector.initialize());
        })
    });
}

/// Бенчмарк для измерения времени сбора eBPF метрик (базовый)
fn benchmark_ebpf_metrics_collection_basic(c: &mut Criterion) {
    let config = EbpfConfig::default();
    let mut collector = EbpfMetricsCollector::new(config);
    collector.initialize().unwrap();

    c.bench_function("ebpf_metrics_collection_basic", |b| {
        b.iter(|| {
            let _metrics = black_box(collector.collect_metrics());
        })
    });
}

/// Бенчмарк для измерения времени сбора eBPF метрик (с кэшированием)
fn benchmark_ebpf_metrics_collection_cached(c: &mut Criterion) {
    let config = EbpfConfig {
        enable_caching: true,
        batch_size: 10,
        ..Default::default()
    };
    let mut collector = EbpfMetricsCollector::new(config);
    collector.initialize().unwrap();

    c.bench_function("ebpf_metrics_collection_cached", |b| {
        b.iter(|| {
            let _metrics = black_box(collector.collect_metrics());
        })
    });
}

/// Бенчмарк для измерения времени сбора eBPF метрик (с агрессивным кэшированием)
fn benchmark_ebpf_metrics_collection_aggressive_cached(c: &mut Criterion) {
    let config = EbpfConfig {
        enable_aggressive_caching: true,
        aggressive_cache_interval_ms: 10000,
        ..Default::default()
    };
    let mut collector = EbpfMetricsCollector::new(config);
    collector.initialize().unwrap();

    c.bench_function("ebpf_metrics_collection_aggressive_cached", |b| {
        b.iter(|| {
            let _metrics = black_box(collector.collect_metrics());
        })
    });
}

/// Бенчмарк для измерения времени сбора eBPF метрик (с высокопроизводительным режимом)
fn benchmark_ebpf_metrics_collection_high_performance(c: &mut Criterion) {
    let config = EbpfConfig {
        enable_high_performance_mode: true,
        enable_aggressive_caching: true,
        aggressive_cache_interval_ms: 5000,
        ..Default::default()
    };
    let mut collector = EbpfMetricsCollector::new(config);
    collector.initialize().unwrap();

    c.bench_function("ebpf_metrics_collection_high_performance", |b| {
        b.iter(|| {
            let _metrics = black_box(collector.collect_metrics());
        })
    });
}

/// Бенчмарк для измерения времени сбора eBPF метрик с различными конфигурациями
fn benchmark_ebpf_metrics_collection_configs(c: &mut Criterion) {
    let configs = vec![
        (
            "no_caching",
            EbpfConfig {
                enable_caching: false,
                enable_aggressive_caching: false,
                ..Default::default()
            },
        ),
        (
            "basic_caching",
            EbpfConfig {
                enable_caching: true,
                enable_aggressive_caching: false,
                ..Default::default()
            },
        ),
        (
            "aggressive_caching",
            EbpfConfig {
                enable_caching: false,
                enable_aggressive_caching: true,
                aggressive_cache_interval_ms: 5000,
                ..Default::default()
            },
        ),
        (
            "both_caching",
            EbpfConfig {
                enable_caching: true,
                enable_aggressive_caching: true,
                aggressive_cache_interval_ms: 5000,
                ..Default::default()
            },
        ),
    ];

    for (name, config) in configs {
        let mut collector = EbpfMetricsCollector::new(config);
        collector.initialize().unwrap();

        c.bench_function(&format!("ebpf_metrics_collection_{}", name), |b| {
            b.iter(|| {
                let _metrics = black_box(collector.collect_metrics());
            })
        });
    }
}

/// Бенчмарк для измерения времени сериализации конфигурации eBPF
fn benchmark_ebpf_config_serialization(c: &mut Criterion) {
    let config = EbpfConfig {
        enable_high_performance_mode: true,
        enable_aggressive_caching: true,
        aggressive_cache_interval_ms: 5000,
        ..Default::default()
    };

    c.bench_function("ebpf_config_serialization", |b| {
        b.iter(|| {
            let serialized = serde_json::to_string(black_box(&config)).unwrap();
            black_box(serialized);
        })
    });
}

/// Бенчмарк для измерения времени десериализации конфигурации eBPF
fn benchmark_ebpf_config_deserialization(c: &mut Criterion) {
    let config = EbpfConfig {
        enable_high_performance_mode: true,
        enable_aggressive_caching: true,
        aggressive_cache_interval_ms: 5000,
        ..Default::default()
    };
    let serialized = serde_json::to_string(&config).unwrap();

    c.bench_function("ebpf_config_deserialization", |b| {
        b.iter(|| {
            let deserialized: EbpfConfig = serde_json::from_str(black_box(&serialized)).unwrap();
            black_box(deserialized);
        })
    });
}

/// Бенчмарк для измерения производительности с различными комбинациями метрик
fn benchmark_ebpf_metrics_combinations(c: &mut Criterion) {
    let configs = vec![
        (
            "cpu_only",
            EbpfConfig {
                enable_cpu_metrics: true,
                enable_memory_metrics: false,
                enable_syscall_monitoring: false,
                enable_network_monitoring: false,
                enable_gpu_monitoring: false,
                enable_filesystem_monitoring: false,
                ..Default::default()
            },
        ),
        (
            "cpu_memory",
            EbpfConfig {
                enable_cpu_metrics: true,
                enable_memory_metrics: true,
                enable_syscall_monitoring: false,
                enable_network_monitoring: false,
                enable_gpu_monitoring: false,
                enable_filesystem_monitoring: false,
                ..Default::default()
            },
        ),
        (
            "cpu_memory_syscall",
            EbpfConfig {
                enable_cpu_metrics: true,
                enable_memory_metrics: true,
                enable_syscall_monitoring: true,
                enable_network_monitoring: false,
                enable_gpu_monitoring: false,
                enable_filesystem_monitoring: false,
                ..Default::default()
            },
        ),
        (
            "all_metrics",
            EbpfConfig {
                enable_cpu_metrics: true,
                enable_memory_metrics: true,
                enable_syscall_monitoring: true,
                enable_network_monitoring: true,
                enable_gpu_monitoring: true,
                enable_filesystem_monitoring: true,
                ..Default::default()
            },
        ),
    ];

    for (name, config) in configs {
        let mut collector = EbpfMetricsCollector::new(config);
        collector.initialize().unwrap();

        c.bench_function(&format!("ebpf_metrics_{}", name), |b| {
            b.iter(|| {
                let _metrics = black_box(collector.collect_metrics());
            })
        });
    }
}

/// Бенчмарк для измерения производительности с различными размерами batch
fn benchmark_ebpf_batch_sizes(c: &mut Criterion) {
    let batch_sizes = vec![1, 10, 50, 100, 200];

    for batch_size in batch_sizes {
        let config = EbpfConfig {
            enable_caching: true,
            batch_size,
            ..Default::default()
        };
        let mut collector = EbpfMetricsCollector::new(config);
        collector.initialize().unwrap();

        c.bench_function(&format!("ebpf_batch_size_{}", batch_size), |b| {
            b.iter(|| {
                let _metrics = black_box(collector.collect_metrics());
            })
        });
    }
}

/// Бенчмарк для измерения производительности инициализации с различными конфигурациями
fn benchmark_ebpf_initialization_configs(c: &mut Criterion) {
    let configs = vec![
        (
            "minimal",
            EbpfConfig {
                enable_cpu_metrics: true,
                enable_memory_metrics: false,
                enable_syscall_monitoring: false,
                enable_network_monitoring: false,
                enable_gpu_monitoring: false,
                enable_filesystem_monitoring: false,
                ..Default::default()
            },
        ),
        ("standard", EbpfConfig::default()),
        (
            "full",
            EbpfConfig {
                enable_cpu_metrics: true,
                enable_memory_metrics: true,
                enable_syscall_monitoring: true,
                enable_network_monitoring: true,
                enable_gpu_monitoring: true,
                enable_filesystem_monitoring: true,
                ..Default::default()
            },
        ),
    ];

    for (name, config) in configs {
        c.bench_function(&format!("ebpf_initialization_{}", name), |b| {
            b.iter(|| {
                let mut collector = EbpfMetricsCollector::new(config.clone());
                black_box(collector.initialize());
            })
        });
    }
}

/// Бенчмарк для измерения производительности сериализации метрик
fn benchmark_ebpf_metrics_serialization(c: &mut Criterion) {
    let config = EbpfConfig::default();
    let mut collector = EbpfMetricsCollector::new(config);
    collector.initialize().unwrap();
    let metrics = collector.collect_metrics().unwrap();

    c.bench_function("ebpf_metrics_serialization", |b| {
        b.iter(|| {
            let serialized = serde_json::to_string(black_box(&metrics)).unwrap();
            black_box(serialized);
        })
    });
}

/// Бенчмарк для измерения производительности десериализации метрик
fn benchmark_ebpf_metrics_deserialization(c: &mut Criterion) {
    let config = EbpfConfig::default();
    let mut collector = EbpfMetricsCollector::new(config);
    collector.initialize().unwrap();
    let metrics = collector.collect_metrics().unwrap();
    let serialized = serde_json::to_string(&metrics).unwrap();

    c.bench_function("ebpf_metrics_deserialization", |b| {
        b.iter(|| {
            let deserialized: EbpfMetrics = serde_json::from_str(black_box(&serialized)).unwrap();
            black_box(deserialized);
        })
    });
}

/// Бенчмарк для измерения производительности с детализированной статистикой
fn benchmark_ebpf_detailed_stats(c: &mut Criterion) {
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: true,
        enable_network_monitoring: true,
        enable_gpu_monitoring: true,
        enable_filesystem_monitoring: true,
        ..Default::default()
    };
    let mut collector = EbpfMetricsCollector::new(config);
    collector.initialize().unwrap();

    c.bench_function("ebpf_detailed_stats_collection", |b| {
        b.iter(|| {
            let metrics = black_box(collector.collect_metrics()).unwrap();
            // Принудительно собираем детализированную статистику
            if let Some(details) = metrics.syscall_details {
                black_box(details);
            }
            if let Some(details) = metrics.network_details {
                black_box(details);
            }
            if let Some(details) = metrics.gpu_details {
                black_box(details);
            }
            if let Some(details) = metrics.filesystem_details {
                black_box(details);
            }
        })
    });
}

/// Бенчмарк для измерения производительности валидации конфигурации
fn benchmark_ebpf_config_validation(c: &mut Criterion) {
    let configs = vec![
        ("valid", EbpfConfig::default()),
        (
            "invalid_batch_size",
            EbpfConfig {
                batch_size: 0,
                ..Default::default()
            },
        ),
        (
            "invalid_init_attempts",
            EbpfConfig {
                max_init_attempts: 0,
                ..Default::default()
            },
        ),
        (
            "invalid_collection_interval",
            EbpfConfig {
                collection_interval: Duration::from_secs(0),
                ..Default::default()
            },
        ),
    ];

    for (name, config) in configs {
        let collector = EbpfMetricsCollector::new(config);

        c.bench_function(&format!("ebpf_config_validation_{}", name), |b| {
            b.iter(|| {
                let _result = black_box(collector.validate_config());
            })
        });
    }
}

/// Бенчмарк для измерения производительности с различными интервалами агрессивного кэширования
fn benchmark_ebpf_aggressive_cache_intervals(c: &mut Criterion) {
    let intervals = vec![1000, 5000, 10000, 30000];

    for interval in intervals {
        let config = EbpfConfig {
            enable_aggressive_caching: true,
            aggressive_cache_interval_ms: interval,
            ..Default::default()
        };
        let mut collector = EbpfMetricsCollector::new(config);
        collector.initialize().unwrap();

        c.bench_function(&format!("ebpf_aggressive_cache_{}ms", interval), |b| {
            b.iter(|| {
                let _metrics = black_box(collector.collect_metrics());
            })
        });
    }
}

criterion_group! {
    name = ebpf_benchmarks;
    config = Criterion::default()
        .sample_size(10)
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(5));
    targets =
        benchmark_ebpf_initialization,
        benchmark_ebpf_metrics_collection_basic,
        benchmark_ebpf_metrics_collection_cached,
        benchmark_ebpf_metrics_collection_aggressive_cached,
        benchmark_ebpf_metrics_collection_high_performance,
        benchmark_ebpf_metrics_collection_configs,
        benchmark_ebpf_config_serialization,
        benchmark_ebpf_config_deserialization,
        benchmark_ebpf_metrics_combinations,
        benchmark_ebpf_batch_sizes,
        benchmark_ebpf_initialization_configs,
        benchmark_ebpf_metrics_serialization,
        benchmark_ebpf_metrics_deserialization,
        benchmark_ebpf_detailed_stats,
        benchmark_ebpf_config_validation,
        benchmark_ebpf_aggressive_cache_intervals
}

criterion_main!(ebpf_benchmarks);
