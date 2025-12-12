//! Пример использования eBPF модуля для сбора системных метрик.
//!
//! Этот пример демонстрирует, как использовать eBPF модуль из smoothtask-core
//! для сбора высокопроизводительных системных метрик.

use smoothtask_core::metrics::ebpf::{EbpfMetricsCollector, EbpfConfig, EbpfMetrics, EbpfNotificationThresholds, EbpfFilterConfig, SyscallStat, NetworkStat, ConnectionStat};
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    println!("=== SmoothTask eBPF Metrics Example ===\n");

    // 1. Проверка поддержки eBPF
    println!("1. Checking eBPF support...");
    match EbpfMetricsCollector::check_ebpf_support() {
        Ok(supported) => {
            if supported {
                println!("   ✓ eBPF is supported on this system");
            } else {
                println!("   ✗ eBPF is not supported on this system");
                println!("   Note: Falling back to basic metrics collection");
                // В реальном приложении здесь можно использовать альтернативные методы сбора метрик
            }
        }
        Err(e) => {
            println!("   ✗ Error checking eBPF support: {}", e);
            return Ok(());
        }
    }

    // 2. Создание конфигурации
    println!("\n2. Creating eBPF configuration...");
    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: true,
        enable_network_monitoring: true,
        enable_network_connections: true,
        enable_gpu_monitoring: false, // Отключено для примера
        enable_cpu_temperature_monitoring: false, // Отключено для примера
        enable_filesystem_monitoring: false, // Отключено для примера
        enable_process_monitoring: false, // Отключено для примера
        enable_notifications: false,
        notification_thresholds: EbpfNotificationThresholds::default(),
        filter_config: EbpfFilterConfig::default(),
        collection_interval: Duration::from_secs(1),
        enable_caching: true,
        batch_size: 50,
        max_init_attempts: 3,
        operation_timeout_ms: 1000,
        enable_high_performance_mode: true,
        enable_aggressive_caching: false,
        aggressive_cache_interval_ms: 5000,
    };

    println!("   Configuration created with:");
    println!("   - CPU metrics: {}", config.enable_cpu_metrics);
    println!("   - Memory metrics: {}", config.enable_memory_metrics);
    println!("   - System call monitoring: {}", config.enable_syscall_monitoring);
    println!("   - Network monitoring: {}", config.enable_network_monitoring);
    println!("   - Network connections: {}", config.enable_network_connections);

    // 3. Создание коллектора
    println!("\n3. Creating eBPF metrics collector...");
    let mut collector = EbpfMetricsCollector::new(config);
    println!("   ✓ Collector created successfully");

    // 4. Инициализация eBPF программ
    println!("\n4. Initializing eBPF programs...");
    match collector.initialize() {
        Ok(_) => {
            println!("   ✓ eBPF programs initialized successfully");
            
            // Получение статистики инициализации
            let (success_count, error_count) = collector.get_initialization_stats();
            println!("   Initialization stats: {} programs loaded, {} errors", 
                success_count, error_count);
            
            // Получение информации о картах
            let maps_info = collector.get_maps_info();
            println!("   Maps info: {}", maps_info);
        }
        Err(e) => {
            println!("   ✗ Failed to initialize eBPF programs: {}", e);
            
            // Получение детальной информации об ошибке
            if let Some(error_info) = collector.get_detailed_error_info() {
                println!("   Error details: {}", error_info);
            }
            
            // В реальном приложении здесь можно продолжить с альтернативными методами
            return Ok(());
        }
    }

    // 5. Сбор и отображение метрик
    println!("\n5. Collecting eBPF metrics...");
    
    // Основной цикл сбора метрик
    for iteration in 1..=5 {
        println!("\n   --- Iteration {} ---", iteration);
        
        match collector.collect_metrics() {
            Ok(metrics) => {
                display_metrics(&metrics);
                
                // Отображение детализированной статистики (если доступно)
                if let Some(syscall_details) = &metrics.syscall_details {
                    display_top_syscalls(syscall_details, 3);
                }
                
                if let Some(network_details) = &metrics.network_details {
                    display_top_network_stats(network_details, 2);
                }
                
                if let Some(connection_details) = &metrics.connection_details {
                    display_active_connections(connection_details, 2);
                }
            }
            Err(e) => {
                println!("   ✗ Error collecting metrics: {}", e);
                
                // Попытка восстановления
                println!("   Attempting recovery...");
                match collector.attempt_recovery() {
                    Ok(_) => println!("   ✓ Recovery successful"),
                    Err(recovery_err) => println!("   ✗ Recovery failed: {}", recovery_err),
                }
            }
        }
        
        // Пауза между итерациями
        std::thread::sleep(Duration::from_secs(1));
    }

    // 6. Демонстрация управления конфигурацией
    println!("\n6. Demonstrating configuration management...");
    
    // Установка ограничения на кэшируемые детали
    collector.set_max_cached_details(100);
    println!("   ✓ Set max cached details to 100");
    
    // Включение очистки неиспользуемых карт
    collector.set_cleanup_unused_maps(true);
    println!("   ✓ Enabled cleanup of unused maps");
    
    // Оптимизация использования памяти
    collector.optimize_memory_usage();
    println!("   ✓ Memory usage optimized");
    
    // Получение оценки использования памяти
    let memory_usage = collector.get_memory_usage_estimate();
    println!("   Memory usage estimate: {} bytes", memory_usage);

    // 7. Демонстрация сброса состояния (для тестирования)
    println!("\n7. Demonstrating state reset (for testing)...");
    collector.reset();
    println!("   ✓ Collector state reset");
    println!("   Initialized: {}", collector.is_initialized());

    println!("\n=== Example completed successfully! ===");
    Ok(())
}

/// Отображение основных метрик
fn display_metrics(metrics: &EbpfMetrics) {
    println!("   Basic Metrics:");
    println!("     CPU Usage: {:.2}%", metrics.cpu_usage);
    println!("     Memory Usage: {} MB", metrics.memory_usage / 1024 / 1024);
    println!("     System Calls: {}", metrics.syscall_count);
    println!("     Network Packets: {}", metrics.network_packets);
    println!("     Network Bytes: {} KB", metrics.network_bytes / 1024);
    println!("     Active Connections: {}", metrics.active_connections);
    println!("     Active Processes: {}", metrics.active_processes);
    println!("     Timestamp: {}", metrics.timestamp);
}

/// Отображение топ системных вызовов
fn display_top_syscalls(details: &[SyscallStat], top_n: usize) {
    println!("   Top {} System Calls:", top_n);
    
    let mut sorted_details = details.to_vec();
    sorted_details.sort_by(|a, b| b.count.cmp(&a.count));
    
    for (i, stat) in sorted_details.iter().take(top_n).enumerate() {
        println!("     {}. Syscall {}: {} calls, avg {:.2} µs",
            i + 1,
            stat.syscall_id,
            stat.count,
            stat.avg_time_ns as f64 / 1000.0
        );
    }
}

/// Отображение топ сетевой статистики
fn display_top_network_stats(details: &[NetworkStat], top_n: usize) {
    println!("   Top {} Network Stats:", top_n);
    
    let mut sorted_details = details.to_vec();
    sorted_details.sort_by(|a, b| {
        let a_total = a.packets_sent + a.packets_received;
        let b_total = b.packets_sent + b.packets_received;
        b_total.cmp(&a_total)
    });
    
    for (i, stat) in sorted_details.iter().take(top_n).enumerate() {
        let total_packets = stat.packets_sent + stat.packets_received;
        let total_bytes = stat.bytes_sent + stat.bytes_received;
        
        println!("     {}. IP {}: {} packets, {} KB",
            i + 1,
            format_ip(stat.ip_address),
            total_packets,
            total_bytes / 1024
        );
    }
}

/// Отображение активных соединений
fn display_active_connections(details: &[ConnectionStat], top_n: usize) {
    println!("   Top {} Active Connections:", top_n);
    
    let mut sorted_details = details.to_vec();
    sorted_details.sort_by(|a, b| b.packets.cmp(&a.packets));
    
    for (i, stat) in sorted_details.iter().take(top_n).enumerate() {
        println!("     {}. {}:{} -> {}:{} ({} packets, {} KB)",
            i + 1,
            format_ip(stat.src_ip),
            stat.src_port,
            format_ip(stat.dst_ip),
            stat.dst_port,
            stat.packets,
            stat.bytes / 1024
        );
    }
}

/// Форматирование IP адреса
fn format_ip(ip: u32) -> String {
    let bytes = ip.to_be_bytes();
    format!("{}.{}.{}.{}", bytes[0], bytes[1], bytes[2], bytes[3])
}