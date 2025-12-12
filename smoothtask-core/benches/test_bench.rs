//! Тестовый бенчмарк для проверки работоспособности системы бенчмаркинга.

use criterion::{black_box, criterion_group, criterion_main, Criterion};

/// Простой бенчмарк для тестирования
fn simple_test(c: &mut Criterion) {
    c.bench_function("simple_test", |b| {
        b.iter(|| {
            let mut sum = 0u64;
            for i in 0..100 {
                sum = black_box(sum.wrapping_add(black_box(i)));
            }
            sum
        })
    });
}

criterion_group! {
    name = test_bench;
    config = Criterion::default()
        .sample_size(5)
        .warm_up_time(std::time::Duration::from_secs(1))
        .measurement_time(std::time::Duration::from_secs(2));
    targets = simple_test
}

criterion_main!(test_bench);
