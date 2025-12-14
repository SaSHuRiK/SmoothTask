//! Benchmarks for scheduling latency measurement and percentile calculation.
//!
//! These benchmarks measure the performance of:
//! - LatencyCollector::add_sample() - adding measurements
//! - LatencyCollector::percentile() - computing percentiles
//! - LatencyCollector::p95() and p99() - common percentile calculations
//! - Cache performance for repeated percentile calculations

use criterion::{criterion_group, criterion_main, Criterion};
use smoothtask_core::metrics::scheduling_latency::LatencyCollector;
use std::sync::Arc;
use std::thread;

fn benchmark_add_sample(c: &mut Criterion) {
    let mut group = c.benchmark_group("LatencyCollector::add_sample");

    // Benchmark with different window sizes
    for window_size in [100, 1000, 5000] {
        group.bench_function(format!("window_size_{}", window_size), |b| {
            let collector = LatencyCollector::new(window_size);
            let mut sample_value = 0.0;

            b.iter(|| {
                collector.add_sample(sample_value);
                sample_value += 0.1;
                if sample_value > 100.0 {
                    sample_value = 0.0;
                }
            });
        });
    }

    group.finish();
}

fn benchmark_percentile_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("LatencyCollector::percentile");

    // Prepare collector with data
    for data_size in [100, 1000, 5000] {
        group.bench_function(format!("data_size_{}", data_size), |b| {
            let collector = LatencyCollector::new(data_size);

            // Fill with sample data
            for i in 0..data_size {
                collector.add_sample(i as f64 * 0.1);
            }

            b.iter(|| {
                // Benchmark P95 calculation
                let _p95 = collector.percentile(0.95);
            });
        });
    }

    group.finish();
}

fn benchmark_p95_p99_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("LatencyCollector::p95_p99");

    // Prepare collector with realistic data
    let collector = LatencyCollector::new(5000);
    for i in 0..5000 {
        collector.add_sample(i as f64 * 0.02); // 0.0 to 100.0 ms
    }

    group.bench_function("p95", |b| {
        b.iter(|| {
            let _p95 = collector.p95();
        });
    });

    group.bench_function("p99", |b| {
        b.iter(|| {
            let _p99 = collector.p99();
        });
    });

    group.finish();
}

fn benchmark_cache_performance(c: &mut Criterion) {
    let mut group = c.benchmark_group("LatencyCollector::cache_performance");

    // Test cache with 5 second TTL
    let collector = LatencyCollector::new_with_cache_ttl(5000, 5);

    // Fill with data
    for i in 0..5000 {
        collector.add_sample(i as f64 * 0.02);
    }

    // First call (cache miss)
    group.bench_function("cache_miss_p95", |b| {
        b.iter(|| {
            let _p95 = collector.percentile(0.95);
        });
    });

    // Subsequent calls (cache hit)
    group.bench_function("cache_hit_p95", |b| {
        b.iter(|| {
            let _p95 = collector.percentile(0.95);
        });
    });

    group.finish();
}

fn benchmark_concurrent_access(c: &mut Criterion) {
    let mut group = c.benchmark_group("LatencyCollector::concurrent_access");

    let collector = Arc::new(LatencyCollector::new(10000));

    group.bench_function("concurrent_add_samples", |b| {
        b.iter(|| {
            let mut handles = vec![];

            // Spawn 4 threads adding samples
            for thread_id in 0..4 {
                let collector_clone = Arc::clone(&collector);
                let handle = thread::spawn(move || {
                    for i in 0..250 {
                        collector_clone.add_sample((thread_id * 1000 + i) as f64 * 0.01);
                    }
                });
                handles.push(handle);
            }

            // Wait for all threads to complete
            for handle in handles {
                handle.join().unwrap();
            }
        });
    });

    group.finish();
}

fn benchmark_various_percentiles(c: &mut Criterion) {
    let mut group = c.benchmark_group("LatencyCollector::various_percentiles");

    let collector = LatencyCollector::new(5000);

    // Fill with realistic latency data (mostly low, some high values)
    for i in 0..5000 {
        let latency = if i % 100 == 0 {
            // 1% of samples are high latency (10-100ms)
            (i % 100) as f64 * 1.0
        } else {
            // 99% of samples are low latency (0.1-5ms)
            (i % 50) as f64 * 0.02
        };
        collector.add_sample(latency);
    }

    // Benchmark different percentile calculations
    for percentile in [0.5, 0.9, 0.95, 0.99, 0.999] {
        group.bench_function(format!("percentile_{:.3}", percentile), |b| {
            b.iter(|| {
                let _value = collector.percentile(percentile);
            });
        });
    }

    group.finish();
}

criterion_group!(
    name = scheduling_latency_benches;
    config = Criterion::default().sample_size(10);
    targets =
        benchmark_add_sample,
        benchmark_percentile_calculation,
        benchmark_p95_p99_calculation,
        benchmark_cache_performance,
        benchmark_concurrent_access,
        benchmark_various_percentiles
);

criterion_main!(scheduling_latency_benches);
