//! Простые бенчмарки для измерения производительности основных функций.
//!
//! Эти бенчмарки помогают измерить базовую производительность функций
//! без сложных зависимостей.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

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

criterion_group! {
    name = simple_benchmarks;
    config = Criterion::default().sample_size(10);
    targets = 
        benchmark_simple_operation,
        benchmark_memory_allocation,
        benchmark_string_operations
}

criterion_main!(simple_benchmarks);