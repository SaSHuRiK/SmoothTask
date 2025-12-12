use criterion::{criterion_group, criterion_main, Criterion};
use smoothtask_core::metrics::system::{
    CpuTimes, DiskDevice, LoadAvg, MemoryInfo, NetworkInterface, PowerMetrics, SystemMetrics,
    TemperatureMetrics,
};
use std::alloc::System;

#[global_allocator]
static A: System = System;

fn create_large_system_metrics() -> SystemMetrics {
    let mut metrics = SystemMetrics::default();

    // Add CPU data
    metrics.cpu_times = CpuTimes {
        user: 1000000,
        nice: 500000,
        system: 2000000,
        idle: 10000000,
        iowait: 100000,
        irq: 50000,
        softirq: 50000,
        steal: 0,
        guest: 0,
        guest_nice: 0,
    };

    // Add memory data
    metrics.memory = MemoryInfo {
        mem_total_kb: 16_384_256,
        mem_available_kb: 9_876_543,
        mem_free_kb: 1_234_567,
        buffers_kb: 345_678,
        cached_kb: 2_345_678,
        swap_total_kb: 8_192_000,
        swap_free_kb: 4_096_000,
    };

    // Add load average
    metrics.load_avg = LoadAvg {
        one: 1.5,
        five: 1.2,
        fifteen: 1.0,
    };

    // Add network interfaces (simulating multiple interfaces)
    for i in 0..10 {
        metrics.network.interfaces.push(NetworkInterface {
            name: format!("eth{}", i).into(),
            rx_bytes: 1000000 + i * 1000,
            tx_bytes: 2000000 + i * 1000,
            rx_packets: 1000 + i,
            tx_packets: 2000 + i,
            rx_errors: i,
            tx_errors: i,
        });
    }

    // Add disk devices (simulating multiple disks)
    for i in 0..5 {
        metrics.disk.devices.push(DiskDevice {
            name: format!("sda{}", i).into(),
            read_bytes: 10000000 + i * 1000000,
            write_bytes: 20000000 + i * 1000000,
            read_ops: 10000 + i,
            write_ops: 20000 + i,
            io_time: 5000 + i,
        });
    }

    // Add temperature data
    metrics.temperature = TemperatureMetrics {
        cpu_temperature_c: Some(45.5),
        gpu_temperature_c: Some(60.2),
    };

    // Add power data
    metrics.power = PowerMetrics {
        system_power_w: Some(120.5),
        cpu_power_w: Some(80.3),
        gpu_power_w: Some(40.1),
    };

    metrics
}

fn create_large_system_metrics_optimized() -> SystemMetrics {
    create_large_system_metrics().optimize_memory_usage()
}

fn benchmark_memory_usage(c: &mut Criterion) {
    c.bench_function("create_large_system_metrics", |b| {
        b.iter(|| create_large_system_metrics())
    });

    c.bench_function("create_large_system_metrics_optimized", |b| {
        b.iter(|| create_large_system_metrics_optimized())
    });

    // Benchmark the optimization function itself
    c.bench_function("optimize_memory_usage", |b| {
        b.iter(|| {
            let metrics = create_large_system_metrics();
            metrics.optimize_memory_usage()
        })
    });
}

fn estimate_memory_size<T>(value: &T) -> usize {
    // This is a rough estimate - in a real benchmark you'd want more precise measurement
    std::mem::size_of_val(value)
}

fn benchmark_memory_size(_c: &mut Criterion) {
    let regular_metrics = create_large_system_metrics();
    let regular_size = estimate_memory_size(&regular_metrics);

    let optimized_metrics = regular_metrics.optimize_memory_usage();
    let optimized_size = estimate_memory_size(&optimized_metrics);

    println!("Regular SystemMetrics size: {} bytes", regular_size);
    println!("Optimized SystemMetrics size: {} bytes", optimized_size);
    println!(
        "Memory reduction: {} bytes ({:.2}%)",
        regular_size.saturating_sub(optimized_size),
        (regular_size.saturating_sub(optimized_size)) as f64 / regular_size as f64 * 100.0
    );
}

criterion_group!(benches, benchmark_memory_usage, benchmark_memory_size);
criterion_main!(benches);
