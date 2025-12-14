//! Бенчмарки для системы логирования
//!
//! Эти бенчмарки измеряют производительность различных операций логирования,
//! включая ротацию логов, сжатие и управление файлами.

use criterion::{criterion_group, criterion_main, Criterion};
use smoothtask_core::logging::{
    adjust_log_for_memory_pressure, get_log_stats, log_log_stats, optimize_log_rotation,
    rotation::LogRotator,
};
use std::io::Write;
use tempfile::{NamedTempFile, TempDir};

fn benchmark_log_stats(c: &mut Criterion) {
    c.bench_function("get_log_stats", |b| {
        b.iter(|| {
            let stats = get_log_stats();
            // Verify we get some data
            assert!(stats.total_entries > 0);
        })
    });

    c.bench_function("log_log_stats", |b| {
        let stats = get_log_stats();
        b.iter(|| {
            log_log_stats(&stats);
        })
    });
}

fn benchmark_log_rotation(c: &mut Criterion) {
    c.bench_function("log_rotation_small_file", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().expect("temp dir");
            let log_path = temp_dir.path().join("test.log");

            // Create a small log file
            let mut file = std::fs::File::create(&log_path).expect("create log file");
            writeln!(file, "Test log entry").expect("write to log");
            drop(file);

            let mut rotator = LogRotator::new(100, 3, false, 0, 0, 0);
            rotator
                .rotate_log(&log_path)
                .expect("rotation should succeed");
        })
    });

    c.bench_function("log_rotation_large_file", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().expect("temp dir");
            let log_path = temp_dir.path().join("test.log");

            // Create a larger log file
            let mut file = std::fs::File::create(&log_path).expect("create log file");
            for i in 0..1000 {
                writeln!(file, "Test log entry {}", i).expect("write to log");
            }
            drop(file);

            let mut rotator = LogRotator::new(1000, 3, false, 0, 0, 0);
            rotator
                .rotate_log(&log_path)
                .expect("rotation should succeed");
        })
    });

    c.bench_function("log_rotation_with_compression", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().expect("temp dir");
            let log_path = temp_dir.path().join("test.log");

            // Create a log file for compression
            let mut file = std::fs::File::create(&log_path).expect("create log file");
            for i in 0..500 {
                writeln!(file, "Test log entry for compression {}", i).expect("write to log");
            }
            drop(file);

            let mut rotator = LogRotator::new(1000, 3, true, 0, 0, 0);
            rotator
                .rotate_log(&log_path)
                .expect("rotation with compression should succeed");
        })
    });
}

fn benchmark_log_optimization(c: &mut Criterion) {
    c.bench_function("optimize_log_rotation_normal", |b| {
        b.iter(|| {
            let mut rotator = LogRotator::new(10_000, 5, true, 3600, 86400, 1_000_000);
            optimize_log_rotation(&mut rotator, false);
            let (max_size, _, _, interval, _, _) = rotator.get_config();
            assert_eq!(max_size, 10_000);
            assert_eq!(interval, 3600);
        })
    });

    c.bench_function("optimize_log_rotation_pressure", |b| {
        b.iter(|| {
            let mut rotator = LogRotator::new(10_000, 5, true, 3600, 86400, 1_000_000);
            optimize_log_rotation(&mut rotator, true);
            let (max_size, _, _, interval, _, _) = rotator.get_config();
            assert!(max_size < 10_000);
            assert!(interval < 3600);
        })
    });

    c.bench_function("adjust_log_for_memory_pressure", |b| {
        b.iter(|| {
            adjust_log_for_memory_pressure();
        })
    });
}

fn benchmark_log_cleanup(c: &mut Criterion) {
    c.bench_function("log_cleanup_old_files", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().expect("temp dir");
            let log_path = temp_dir.path().join("test.log");

            // Create log file and some rotated files
            let mut file = std::fs::File::create(&log_path).expect("create log file");
            writeln!(file, "Test log entry").expect("write to log");
            drop(file);

            let mut rotator = LogRotator::new(100, 2, false, 0, 0, 0);

            // Create multiple rotated files
            for i in 0..5 {
                rotator
                    .rotate_log(&log_path)
                    .expect("rotation should succeed");
                let mut file = std::fs::File::create(&log_path).expect("recreate log file");
                writeln!(file, "Test log entry {}", i).expect("write to log");
                drop(file);
            }

            // Cleanup should remove old files
            rotator
                .cleanup_logs(&log_path)
                .expect("cleanup should succeed");
        })
    });

    c.bench_function("log_cleanup_by_age", |b| {
        b.iter(|| {
            let temp_dir = TempDir::new().expect("temp dir");
            let log_path = temp_dir.path().join("test.log");

            let mut file = std::fs::File::create(&log_path).expect("create log file");
            writeln!(file, "Test log entry").expect("write to log");
            drop(file);

            let mut rotator = LogRotator::new(100, 3, false, 0, 1, 0);

            // Create some rotated files
            for i in 0..3 {
                rotator
                    .rotate_log(&log_path)
                    .expect("rotation should succeed");
                let mut file = std::fs::File::create(&log_path).expect("recreate log file");
                writeln!(file, "Test log entry {}", i).expect("write to log");
                drop(file);
            }

            // Wait for files to become "old"
            std::thread::sleep(std::time::Duration::from_secs(2));

            // Cleanup by age
            rotator
                .cleanup_by_age(&log_path)
                .expect("cleanup by age should succeed");
        })
    });
}

criterion_group!(
    name = logging_benches;
    config = Criterion::default().sample_size(10);
    targets =
        benchmark_log_stats,
        benchmark_log_rotation,
        benchmark_log_optimization,
        benchmark_log_cleanup
);

criterion_main!(logging_benches);
