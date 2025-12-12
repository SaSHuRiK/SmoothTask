//! Performance tests using criterion-like approach

use std::time::Instant;
use smoothtask_core::metrics::windows::WindowIntrospector;

#[test]
fn test_simple_operation_performance() {
    // Simple performance test
    let start = Instant::now();
    
    let mut sum = 0u64;
    for i in 0..1000 {
        sum = sum.wrapping_add(i);
    }
    
    let duration = start.elapsed();
    println!("Simple operation took: {:?}", duration);
    
    // Just verify it works
    assert!(sum > 0);
}

#[test]
fn test_system_metrics_collection_performance() {
    use smoothtask_core::metrics::system::{collect_system_metrics, ProcPaths};
    use std::path::PathBuf;
    
    let proc_paths = ProcPaths {
        stat: PathBuf::from("/proc/stat"),
        meminfo: PathBuf::from("/proc/meminfo"),
        loadavg: PathBuf::from("/proc/loadavg"),
        pressure_cpu: PathBuf::from("/proc/pressure/cpu"),
        pressure_io: PathBuf::from("/proc/pressure/io"),
        pressure_memory: PathBuf::from("/proc/pressure/memory"),
    };
    
    let start = Instant::now();
    let result = collect_system_metrics(&proc_paths);
    let duration = start.elapsed();
    
    println!("System metrics collection took: {:?}", duration);
    assert!(result.is_ok());
}

#[test]
fn test_process_metrics_collection_performance() {
    use smoothtask_core::metrics::process::collect_process_metrics;
    
    let start = Instant::now();
    let result = collect_process_metrics();
    let duration = start.elapsed();
    
    println!("Process metrics collection took: {:?}", duration);
    assert!(result.is_ok());
}

#[test]
fn test_window_introspector_performance() {
    use smoothtask_core::metrics::windows::{StaticWindowIntrospector, WindowInfo, WindowState};
    
    // Create test windows
    let test_windows = vec![
        WindowInfo::new(
            Some("test.app".to_string()),
            Some("Test Window 1".to_string()),
            Some(1),
            WindowState::Background,
            Some(100),
            1.0,
        ),
        WindowInfo::new(
            Some("test.app".to_string()),
            Some("Test Window 2".to_string()),
            Some(2),
            WindowState::Minimized,
            Some(200),
            0.8,
        ),
    ];
    
    let start = Instant::now();
    let introspector = StaticWindowIntrospector::new(test_windows);
    let duration = start.elapsed();
    
    println!("Window introspector creation took: {:?}", duration);
    assert_eq!(introspector.windows().unwrap().len(), 2);
}

#[test]
fn test_config_creation_performance() {
    use smoothtask_core::config::{CacheIntervals, Config, LoggingConfig, Paths, PolicyMode, Thresholds};
    
    let start = Instant::now();
    
    // Create multiple configs to measure performance
    for _ in 0..100 {
        let _config = Config {
            polling_interval_ms: 1000,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: PolicyMode::Hybrid,
            enable_snapshot_logging: false,
            thresholds: Thresholds {
                psi_cpu_some_high: 0.5,
                psi_io_some_high: 0.5,
                user_idle_timeout_sec: 300,
                interactive_build_grace_sec: 60,
                noisy_neighbour_cpu_share: 0.2,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.7,
                normal_percentile: 0.5,
                background_percentile: 0.3,
                sched_latency_p99_threshold_ms: 20.0,
                ui_loop_p95_threshold_ms: 16.67,
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
            },
            paths: Paths {
                snapshot_db_path: "/var/log/smoothtask/snapshots.db".to_string(),
                patterns_dir: "/etc/smoothtask/patterns".to_string(),
                api_listen_addr: Some("127.0.0.1:8080".to_string()),
            },
            cache_intervals: CacheIntervals {
                system_metrics_cache_interval: 5,
                process_metrics_cache_interval: 2,
            },
        };
    }
    
    let duration = start.elapsed();
    println!("100 config creations took: {:?}", duration);
}

#[test]
fn test_window_operations_performance() {
    use smoothtask_core::metrics::windows::{build_pid_to_window_map, get_window_info_by_pid, select_focused_window, StaticWindowIntrospector, WindowInfo, WindowState};
    
    // Create test windows
    let test_windows = vec![
        WindowInfo::new(
            Some("test.app".to_string()),
            Some("Window 1".to_string()),
            Some(1),
            WindowState::Background,
            Some(100),
            1.0,
        ),
        WindowInfo::new(
            Some("test.app".to_string()),
            Some("Window 2".to_string()),
            Some(2),
            WindowState::Fullscreen,
            Some(200),
            0.9,
        ),
        WindowInfo::new(
            Some("test.app".to_string()),
            Some("Window 3".to_string()),
            Some(3),
            WindowState::Minimized,
            Some(300),
            0.8,
        ),
    ];
    
    let introspector = StaticWindowIntrospector::new(test_windows.clone());
    
    // Test select_focused_window
    let start = Instant::now();
    let focused = select_focused_window(&test_windows);
    let duration1 = start.elapsed();
    println!("select_focused_window took: {:?}", duration1);
    
    // Test build_pid_to_window_map
    let start = Instant::now();
    let pid_map = build_pid_to_window_map(&introspector);
    let duration2 = start.elapsed();
    println!("build_pid_to_window_map took: {:?}", duration2);
    
    // Test get_window_info_by_pid
    let start = Instant::now();
    let window_info = get_window_info_by_pid(&introspector, 2);
    let duration3 = start.elapsed();
    println!("get_window_info_by_pid took: {:?}", duration3);
    
    assert!(focused.is_some());
    assert!(!pid_map.unwrap().is_empty());
    assert!(window_info.is_ok());
}