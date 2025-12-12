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
        ("no_caching", EbpfConfig {
            enable_caching: false,
            enable_aggressive_caching: false,
            ..Default::default()
        }),
        ("basic_caching", EbpfConfig {
            enable_caching: true,
            enable_aggressive_caching: false,
            ..Default::default()
        }),
        ("aggressive_caching", EbpfConfig {
            enable_caching: false,
            enable_aggressive_caching: true,
            aggressive_cache_interval_ms: 5000,
            ..Default::default()
        }),
        ("both_caching", EbpfConfig {
            enable_caching: true,
            enable_aggressive_caching: true,
            aggressive_cache_interval_ms: 5000,
            ..Default::default()
        }),
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
        benchmark_ebpf_config_deserialization
}

criterion_main!(ebpf_benchmarks);