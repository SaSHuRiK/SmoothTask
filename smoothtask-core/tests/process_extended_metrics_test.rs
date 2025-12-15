// Интеграционные тесты для расширенных метрик процессов
//
// Эти тесты проверяют, что расширенные метрики (ProcessMemoryUsage, ProcessPerformanceMetrics, 
// ProcessResourceUtilization) корректно интегрированы в основной процесс сбора метрик.

use smoothtask_core::metrics::process::collect_process_metrics;
use smoothtask_core::logging::snapshots::{ProcessMemoryUsage, ProcessPerformanceMetrics, ProcessResourceUtilization};

#[test]
fn test_collect_process_metrics_includes_extended_metrics() {
    // Тест: проверяем, что collect_process_metrics возвращает процессы с расширенными метриками
    let result = collect_process_metrics(None);
    
    // Функция должна вернуть Ok результат
    assert!(result.is_ok(), "collect_process_metrics должен вернуть Ok результат");
    
    let processes = result.unwrap();
    
    // Должен быть хотя бы один процесс (текущий процесс теста)
    assert!(!processes.is_empty(), "Должен быть хотя бы один процесс");
    
    // Проверяем, что у всех процессов есть расширенные метрики
    for process in &processes {
        assert!(process.pid > 0, "PID должен быть положительным");
        
        // Проверяем расширенные метрики памяти
        assert!(process.memory_usage_details.is_some(), 
               "Процесс {} должен иметь memory_usage_details", process.pid);
        
        // Проверяем расширенные метрики производительности
        assert!(process.performance_metrics.is_some(), 
               "Процесс {} должен иметь performance_metrics", process.pid);
        
        // Проверяем расширенные метрики использования ресурсов
        assert!(process.resource_utilization.is_some(), 
               "Процесс {} должен иметь resource_utilization", process.pid);
        
        // Проверяем, что расширенные метрики содержат осмысленные значения
        let memory_usage = process.memory_usage_details.as_ref().unwrap();
        assert!(memory_usage.total_rss_bytes > 0, 
               "Процесс {} должен иметь положительное использование памяти", process.pid);
        
        let performance = process.performance_metrics.as_ref().unwrap();
        // CPU usage должен быть в разумном диапазоне (0-100%)
        if let Some(cpu_usage) = performance.cpu_usage_10s {
            assert!(cpu_usage >= 0.0 && cpu_usage <= 100.0, 
                   "CPU usage должен быть в диапазоне 0-100% для процесса {}", process.pid);
        }
        
        let utilization = process.resource_utilization.as_ref().unwrap();
        // Efficiency score должен быть в разумном диапазоне (0.0-1.0)
        if let Some(efficiency) = utilization.efficiency_score {
            assert!(efficiency >= 0.0 && efficiency <= 1.0, 
                   "Efficiency score должен быть в диапазоне 0.0-1.0 для процесса {}", process.pid);
        }
    }
}

#[test]
fn test_extended_metrics_values_are_reasonable() {
    // Тест: проверяем, что расширенные метрики содержат разумные значения
    let result = collect_process_metrics(None);
    
    assert!(result.is_ok(), "collect_process_metrics должен вернуть Ok результат");
    
    let processes = result.unwrap();
    assert!(!processes.is_empty(), "Должен быть хотя бы один процесс");
    
    // Берем первый процесс для детальной проверки
    let process = &processes[0];
    
    // Проверяем метрики памяти
    let memory_usage = process.memory_usage_details.as_ref().unwrap();
    assert!(memory_usage.total_rss_bytes > 0, "Total RSS должен быть положительным");
    
    if let Some(heap_usage) = memory_usage.heap_usage_bytes {
        assert!(heap_usage > 0, "Heap usage должен быть положительным");
        assert!(heap_usage <= memory_usage.total_rss_bytes, 
               "Heap usage не должен превышать total RSS");
    }
    
    if let Some(stack_usage) = memory_usage.stack_usage_bytes {
        assert!(stack_usage > 0, "Stack usage должен быть положительным");
        assert!(stack_usage <= memory_usage.total_rss_bytes, 
               "Stack usage не должен превышать total RSS");
    }
    
    if let Some(memory_pressure) = memory_usage.memory_pressure {
        assert!(memory_pressure >= 0.0 && memory_pressure <= 1.0, 
               "Memory pressure должен быть в диапазоне 0.0-1.0");
    }
    
    // Проверяем метрики производительности
    let performance = process.performance_metrics.as_ref().unwrap();
    
    if let Some(cpu_usage_10s) = performance.cpu_usage_10s {
        assert!(cpu_usage_10s >= 0.0 && cpu_usage_10s <= 100.0, 
               "CPU usage 10s должен быть в диапазоне 0-100%");
    }
    
    if let Some(cpu_usage_60s) = performance.cpu_usage_60s {
        assert!(cpu_usage_60s >= 0.0 && cpu_usage_60s <= 100.0, 
               "CPU usage 60s должен быть в диапазоне 0-100%");
    }
    
    if let Some(cpu_usage_300s) = performance.cpu_usage_300s {
        assert!(cpu_usage_300s >= 0.0 && cpu_usage_300s <= 100.0, 
               "CPU usage 300s должен быть в диапазоне 0-100%");
    }
    
    if let Some(syscalls_per_second) = performance.syscalls_per_second {
        assert!(syscalls_per_second >= 0.0, 
               "Syscalls per second должен быть неотрицательным");
    }
    
    if let Some(context_switches_per_second) = performance.context_switches_per_second {
        assert!(context_switches_per_second >= 0.0, 
               "Context switches per second должен быть неотрицательным");
    }
    
    if let Some(performance_score) = performance.performance_score {
        assert!(performance_score >= 0.0 && performance_score <= 1.0, 
               "Performance score должен быть в диапазоне 0.0-1.0");
    }
    
    // Проверяем метрики использования ресурсов
    let utilization = process.resource_utilization.as_ref().unwrap();
    
    if let Some(overall_utilization) = utilization.overall_utilization {
        assert!(overall_utilization >= 0.0 && overall_utilization <= 1.0, 
               "Overall utilization должен быть в диапазоне 0.0-1.0");
    }
    
    if let Some(cpu_utilization) = utilization.cpu_utilization {
        assert!(cpu_utilization >= 0.0 && cpu_utilization <= 1.0, 
               "CPU utilization должен быть в диапазоне 0.0-1.0");
    }
    
    if let Some(memory_utilization) = utilization.memory_utilization {
        assert!(memory_utilization >= 0.0 && memory_utilization <= 1.0, 
               "Memory utilization должен быть в диапазоне 0.0-1.0");
    }
    
    if let Some(io_utilization) = utilization.io_utilization {
        assert!(io_utilization >= 0.0 && io_utilization <= 1.0, 
               "IO utilization должен быть в диапазоне 0.0-1.0");
    }
    
    if let Some(network_utilization) = utilization.network_utilization {
        assert!(network_utilization >= 0.0 && network_utilization <= 1.0, 
               "Network utilization должен быть в диапазоне 0.0-1.0");
    }
    
    if let Some(gpu_utilization) = utilization.gpu_utilization {
        assert!(gpu_utilization >= 0.0 && gpu_utilization <= 1.0, 
               "GPU utilization должен быть в диапазоне 0.0-1.0");
    }
    
    if let Some(efficiency_score) = utilization.efficiency_score {
        assert!(efficiency_score >= 0.0 && efficiency_score <= 1.0, 
               "Efficiency score должен быть в диапазоне 0.0-1.0");
    }
}

#[test]
fn test_extended_metrics_consistency_across_calls() {
    // Тест: проверяем, что расширенные метрики возвращают консистентные результаты
    // при многократном вызове
    
    let result1 = collect_process_metrics(None);
    let result2 = collect_process_metrics(None);
    
    assert!(result1.is_ok(), "Первый вызов должен быть успешным");
    assert!(result2.is_ok(), "Второй вызов должен быть успешным");
    
    let processes1 = result1.unwrap();
    let processes2 = result2.unwrap();
    
    // Оба вызова должны возвращать процессы с расширенными метриками
    assert!(!processes1.is_empty(), "Первый вызов должен вернуть процессы");
    assert!(!processes2.is_empty(), "Второй вызов должен вернуть процессы");
    
    // Проверяем, что расширенные метрики присутствуют в обоих вызовах
    for process in &processes1 {
        assert!(process.memory_usage_details.is_some(), 
               "Первый вызов: процесс {} должен иметь memory_usage_details", process.pid);
        assert!(process.performance_metrics.is_some(), 
               "Первый вызов: процесс {} должен иметь performance_metrics", process.pid);
        assert!(process.resource_utilization.is_some(), 
               "Первый вызов: процесс {} должен иметь resource_utilization", process.pid);
    }
    
    for process in &processes2 {
        assert!(process.memory_usage_details.is_some(), 
               "Второй вызов: процесс {} должен иметь memory_usage_details", process.pid);
        assert!(process.performance_metrics.is_some(), 
               "Второй вызов: процесс {} должен иметь performance_metrics", process.pid);
        assert!(process.resource_utilization.is_some(), 
               "Второй вызов: процесс {} должен иметь resource_utilization", process.pid);
    }
}

#[test]
fn test_extended_metrics_with_cache_config() {
    // Тест: проверяем, что расширенные метрики работают корректно с кэшированием
    use smoothtask_core::metrics::process::ProcessCacheConfig;
    
    let cache_config = ProcessCacheConfig {
        enable_caching: true,
        enable_parallel_processing: true,
        max_cache_size: 100,
        cache_ttl_sec: 60,
        batch_size: Some(50),
    };
    
    let result = collect_process_metrics(Some(cache_config));
    
    assert!(result.is_ok(), "collect_process_metrics с кэшированием должен вернуть Ok результат");
    
    let processes = result.unwrap();
    assert!(!processes.is_empty(), "Должен быть хотя бы один процесс");
    
    // Проверяем, что расширенные метрики присутствуют даже с кэшированием
    for process in &processes {
        assert!(process.memory_usage_details.is_some(), 
               "Процесс {} должен иметь memory_usage_details с кэшированием", process.pid);
        assert!(process.performance_metrics.is_some(), 
               "Процесс {} должен иметь performance_metrics с кэшированием", process.pid);
        assert!(process.resource_utilization.is_some(), 
               "Процесс {} должен иметь resource_utilization с кэшированием", process.pid);
    }
}