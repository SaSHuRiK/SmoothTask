//! Тесты для проверки работоспособности бенчмарков производительности.
//!
//! Эти тесты проверяют, что бенчмарки могут быть выполнены без ошибок
//! и что они корректно измеряют производительность основных компонентов.

use smoothtask_core::metrics::process::collect_process_metrics;
use smoothtask_core::metrics::system::{collect_system_metrics, ProcPaths};
use smoothtask_core::config::config_struct::Config;
use std::path::PathBuf;
use std::time::Instant;

#[test]
fn test_performance_benchmark_compilation() {
    // Этот тест проверяет, что все необходимые компоненты для бенчмарков доступны
    // и что они могут быть скомпилированы и выполнены без ошибок
    
    // 1. Проверяем сбор системных метрик
    let proc_paths = ProcPaths {
        stat: PathBuf::from("/proc/stat"),
        meminfo: PathBuf::from("/proc/meminfo"),
        loadavg: PathBuf::from("/proc/loadavg"),
        pressure_cpu: PathBuf::from("/proc/pressure/cpu"),
        pressure_io: PathBuf::from("/proc/pressure/io"),
        pressure_memory: PathBuf::from("/proc/pressure/memory"),
    };
    
    let system_result = collect_system_metrics(&proc_paths);
    assert!(system_result.is_ok());
    
    // 2. Проверяем сбор метрик процессов
    let processes_result = collect_process_metrics(None);
    assert!(processes_result.is_ok());
    
    // 3. Проверяем сериализацию
    let processes = processes_result.unwrap();
    let serialized = serde_json::to_string(&processes);
    assert!(serialized.is_ok());
    
    // 4. Проверяем десериализацию
    let serialized_str = serialized.unwrap();
    let deserialized: Result<Vec<smoothtask_core::logging::snapshots::ProcessRecord>, _> = 
        serde_json::from_str(&serialized_str);
    assert!(deserialized.is_ok());
}

#[test]
fn test_performance_measurement() {
    // Этот тест проверяет, что мы можем измерять производительность основных операций
    
    // Измеряем время сбора системных метрик
    let proc_paths = ProcPaths {
        stat: PathBuf::from("/proc/stat"),
        meminfo: PathBuf::from("/proc/meminfo"),
        loadavg: PathBuf::from("/proc/loadavg"),
        pressure_cpu: PathBuf::from("/proc/pressure/cpu"),
        pressure_io: PathBuf::from("/proc/pressure/io"),
        pressure_memory: PathBuf::from("/proc/pressure/memory"),
    };
    
    let start = Instant::now();
    let _system_metrics = collect_system_metrics(&proc_paths);
    let system_duration = start.elapsed();
    
    // Измеряем время сбора метрик процессов
    let start = Instant::now();
    let _processes = collect_process_metrics(None);
    let process_duration = start.elapsed();
    
    // Измеряем время загрузки конфигурации
    let start = Instant::now();
    let _config = Config::load("configs/smoothtask.example.yml");
    let config_duration = start.elapsed();
    
    // Проверяем, что все операции выполнились за разумное время
    assert!(system_duration.as_secs() < 5, "System metrics collection took too long");
    assert!(process_duration.as_secs() < 5, "Process metrics collection took too long");
    assert!(config_duration.as_secs() < 5, "Config loading took too long");
}

#[test]
fn test_process_data_processing() {
    // Этот тест проверяет производительность обработки данных процессов
    
    let processes = collect_process_metrics(None).unwrap_or_default();
    
    let start = Instant::now();
    let filtered: Vec<_> = processes
        .iter()
        .filter(|p| p.pid > 100)
        .map(|p| {
            let cpu_usage = p.cpu_share_1s.unwrap_or(0.0);
            let mem_usage = p.rss_mb.unwrap_or(0);
            (p.pid, cpu_usage, mem_usage)
        })
        .collect();
    
    let duration = start.elapsed();
    assert!(duration.as_secs() < 2, "Process data processing took too long");
    assert!(!filtered.is_empty() || processes.is_empty(), "Filtering should work");
}

#[test]
fn test_parallel_processing() {
    // Этот тест проверяет производительность параллельной обработки
    
    use rayon::prelude::*;
    
    let processes = collect_process_metrics(None).unwrap_or_default();
    
    let start = Instant::now();
    let result: Vec<_> = processes
        .par_iter()
        .filter(|p| p.cpu_share_1s.unwrap_or(0.0) > 0.1)
        .map(|p| {
            let cpu_usage = p.cpu_share_1s.unwrap_or(0.0);
            let mem_usage = p.rss_mb.unwrap_or(0);
            (p.pid, cpu_usage, mem_usage)
        })
        .collect();
    
    let duration = start.elapsed();
    assert!(duration.as_secs() < 2, "Parallel processing took too long");
}

#[test]
fn test_config_serialization_performance() {
    // Этот тест проверяет производительность сериализации конфигурации
    
    let config = Config {
        polling_interval_ms: 1000,
        max_candidates: 150,
        ..Default::default()
    };
    
    let start = Instant::now();
    let serialized = serde_yaml::to_string(&config);
    let duration = start.elapsed();
    
    assert!(serialized.is_ok(), "Config serialization should work");
    assert!(duration.as_secs() < 1, "Config serialization took too long");
}