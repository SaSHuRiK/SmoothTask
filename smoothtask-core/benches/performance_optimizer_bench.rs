//! Performance Optimizer Benchmarks
//!
//! This module contains benchmarks for the performance optimization module,
//! including:
//! - Performance profiling overhead
//! - Critical path analysis performance
//! - Optimization strategy application
//! - Overall optimization impact

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use smoothtask_core::performance_optimizer::{
    CriticalPathOptimizer, PerformanceMetrics, PerformanceOptimizer, PerformanceProfiler,
};
use std::collections::HashMap;
use std::time::Duration;

/// Benchmark for performance profiler overhead
fn benchmark_performance_profiler_overhead(c: &mut Criterion) {
    c.bench_function("performance_profiler_overhead", |b| {
        b.iter(|| {
            let profiler = PerformanceProfiler::new();
            
            // Simulate profiling multiple components
            for i in 0..10 {
                let timer = profiler.start_profiling(&format!("component_{}", i));
                // Simulate some work
                std::thread::sleep(Duration::from_micros(1));
                timer.stop();
            }
            
            // Get metrics
            let metrics = profiler.get_all_metrics();
            black_box(metrics);
        })
    });
}

/// Benchmark for critical path analysis
fn benchmark_critical_path_analysis(c: &mut Criterion) {
    let optimizer = CriticalPathOptimizer::new();
    
    // Create test metrics with varying performance characteristics
    let mut metrics = HashMap::new();
    for i in 0..100 {
        let execution_time = if i % 10 == 0 {
            Duration::from_millis(100) // Slow components
        } else {
            Duration::from_micros(100) // Fast components
        };
        
        metrics.insert(format!("component_{}", i), PerformanceMetrics {
            execution_time,
            invocations: if i % 5 == 0 { 100 } else { 10 }, // Varying invocation counts
            cache_hits: if i % 3 == 0 { 1000 } else { 100 },
            cache_misses: if i % 3 == 0 { 100 } else { 10 },
            ..Default::default()
        });
    }

    c.bench_function("critical_path_analysis", |b| {
        b.iter(|| {
            let critical_paths = optimizer.analyze_critical_paths(&metrics);
            black_box(critical_paths);
        })
    });
}

/// Benchmark for optimization strategy application
fn benchmark_optimization_strategy_application(c: &mut Criterion) {
    let mut optimizer = PerformanceOptimizer::new();
    let critical_path_optimizer = CriticalPathOptimizer::new();
    optimizer.add_strategy(critical_path_optimizer);
    
    // Create test metrics
    let mut metrics = HashMap::new();
    for i in 0..50 {
        metrics.insert(format!("component_{}", i), PerformanceMetrics {
            execution_time: if i % 5 == 0 {
                Duration::from_millis(50) // Some slow components
            } else {
                Duration::from_micros(50) // Mostly fast components
            },
            invocations: 10,
            ..Default::default()
        });
    }

    c.bench_function("optimization_strategy_application", |b| {
        b.iter(|| {
            let results = optimizer.apply_optimizations().unwrap();
            black_box(results);
        })
    });
}

/// Benchmark for performance optimizer with realistic workload
fn benchmark_performance_optimizer_realistic(c: &mut Criterion) {
    let mut optimizer = PerformanceOptimizer::new();
    let critical_path_optimizer = CriticalPathOptimizer::new();
    optimizer.add_strategy(critical_path_optimizer);
    
    // Create realistic test metrics
    let mut metrics = HashMap::new();
    
    // System metrics components
    metrics.insert("system_cpu_metrics".to_string(), PerformanceMetrics {
        execution_time: Duration::from_micros(500),
        invocations: 100,
        ..Default::default()
    });
    
    metrics.insert("system_memory_metrics".to_string(), PerformanceMetrics {
        execution_time: Duration::from_micros(300),
        invocations: 100,
        ..Default::default()
    });
    
    // Process metrics components
    metrics.insert("process_collection".to_string(), PerformanceMetrics {
        execution_time: Duration::from_millis(5),
        invocations: 50,
        ..Default::default()
    });
    
    // Network metrics components
    metrics.insert("network_interface_metrics".to_string(), PerformanceMetrics {
        execution_time: Duration::from_micros(800),
        invocations: 100,
        ..Default::default()
    });
    
    // Add profiler to optimizer
    let profiler = optimizer.profiler();
    for (component_name, _) in &metrics {
        let timer = profiler.start_profiling(component_name);
        // Simulate the actual execution time
        std::thread::sleep(Duration::from_micros(1));
        timer.stop();
    }

    c.bench_function("performance_optimizer_realistic", |b| {
        b.iter(|| {
            let results = optimizer.apply_optimizations().unwrap();
            black_box(results);
        })
    });
}

/// Benchmark for critical path optimizer with different threshold configurations
fn benchmark_critical_path_optimizer_thresholds(c: &mut Criterion) {
    // Create test metrics
    let mut metrics = HashMap::new();
    for i in 0..100 {
        metrics.insert(format!("component_{}", i), PerformanceMetrics {
            execution_time: Duration::from_micros(100 + i * 10),
            invocations: 10,
            cache_hits: 100,
            cache_misses: 10,
            ..Default::default()
        });
    }

    c.bench_function("critical_path_optimizer_default_thresholds", |b| {
        b.iter(|| {
            let optimizer = CriticalPathOptimizer::new();
            let critical_paths = optimizer.analyze_critical_paths(&metrics);
            black_box(critical_paths);
        })
    });

    c.bench_function("critical_path_optimizer_custom_thresholds", |b| {
        b.iter(|| {
            let thresholds = crate::performance_optimizer::CriticalPathThresholds {
                slow_execution_threshold: 0.01, // 10ms
                ..Default::default()
            };
            let optimizer = CriticalPathOptimizer::with_thresholds(thresholds);
            let critical_paths = optimizer.analyze_critical_paths(&metrics);
            black_box(critical_paths);
        })
    });
}

criterion_group! {
    name = performance_optimizer_benches;
    config = Criterion::default().sample_size(10);
    targets = 
        benchmark_performance_profiler_overhead,
        benchmark_critical_path_analysis,
        benchmark_optimization_strategy_application,
        benchmark_performance_optimizer_realistic,
        benchmark_critical_path_optimizer_thresholds
}

criterion_main!(performance_optimizer_benches);