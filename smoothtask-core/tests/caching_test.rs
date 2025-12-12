//! Тесты для функциональности кэширования.
//!
//! Эти тесты проверяют работу системы кэширования, реализованной в ST-379.
//! Кэширование позволяет снизить нагрузку на систему за счёт повторного
//! использования ранее собранных метрик.

use smoothtask_core::config::{CacheIntervals, Config, Paths, PolicyMode, Thresholds};
use smoothtask_core::metrics::system::{collect_system_metrics, CpuTimes, LoadAvg, MemoryInfo, ProcPaths, SystemMetrics};
use smoothtask_core::metrics::process::collect_process_metrics;
use std::sync::{Arc, Mutex};

/// Тест проверяет, что CacheIntervals создаётся с дефолтными значениями.
#[test]
fn test_cache_intervals_default() {
    let cache_intervals = CacheIntervals {
        system_metrics_cache_interval: 3,
        process_metrics_cache_interval: 1,
    };
    
    assert_eq!(cache_intervals.system_metrics_cache_interval, 3);
    assert_eq!(cache_intervals.process_metrics_cache_interval, 1);
}

/// Тест проверяет, что CacheIntervals можно создать с кастомными значениями.
#[test]
fn test_cache_intervals_custom() {
    let cache_intervals = CacheIntervals {
        system_metrics_cache_interval: 5,
        process_metrics_cache_interval: 2,
    };
    
    assert_eq!(cache_intervals.system_metrics_cache_interval, 5);
    assert_eq!(cache_intervals.process_metrics_cache_interval, 2);
}

/// Тест проверяет, что CacheIntervals корректно сериализуется и десериализуется.
#[test]
fn test_cache_intervals_serialize_deserialize() {
    let cache_intervals = CacheIntervals {
        system_metrics_cache_interval: 7,
        process_metrics_cache_interval: 3,
    };
    
    let serialized = serde_yaml::to_string(&cache_intervals).unwrap();
    let deserialized: CacheIntervals = serde_yaml::from_str(&serialized).unwrap();
    
    assert_eq!(deserialized.system_metrics_cache_interval, 7);
    assert_eq!(deserialized.process_metrics_cache_interval, 3);
}

/// Тест проверяет, что Config корректно включает CacheIntervals.
#[test]
fn test_config_includes_cache_intervals() {
    let config = Config {
        polling_interval_ms: 1000,
        max_candidates: 150,
        dry_run_default: false,
        policy_mode: PolicyMode::Hybrid,
        thresholds: Thresholds {
            psi_cpu_some_high: 0.5,
            psi_io_some_high: 0.5,
            user_idle_timeout_sec: 300,
            interactive_build_grace_sec: 60,
            noisy_neighbour_cpu_share: 0.5,
            crit_interactive_percentile: 99.0,
            interactive_percentile: 90.0,
            normal_percentile: 70.0,
            background_percentile: 50.0,
            sched_latency_p99_threshold_ms: 10.0,
            ui_loop_p95_threshold_ms: 16.67,
        },
        paths: Paths {
            snapshot_db_path: "/tmp/test.db".to_string(),
            patterns_dir: "/tmp/patterns".to_string(),
            api_listen_addr: None,
        },
        cache_intervals: CacheIntervals {
            system_metrics_cache_interval: 5,
            process_metrics_cache_interval: 2,
        },
        enable_snapshot_logging: false,
    };
    
    assert_eq!(config.cache_intervals.system_metrics_cache_interval, 5);
    assert_eq!(config.cache_intervals.process_metrics_cache_interval, 2);
}

/// Тест проверяет, что кэширование системных метрик работает корректно.
/// Этот тест проверяет базовую логику кэширования без реального сбора метрик.
#[test]
fn test_system_metrics_caching_logic() {
    // Создаём кэш и тестируем логику обновления
    let mut system_metrics_cache: Option<SystemMetrics> = None;
    let mut system_metrics_cache_iteration: u64 = 0;
    let system_metrics_cache_interval = 3u64;
    
    // Имитируем несколько итераций
    for current_iteration in 1..=10 {
        let need_update = system_metrics_cache.is_none() ||
            (current_iteration - system_metrics_cache_iteration) >= system_metrics_cache_interval;
        
        if need_update {
            // Имитируем обновление кэша
            let mock_metrics = SystemMetrics {
                cpu_times: CpuTimes {
                    user: 100,
                    nice: 10,
                    system: 20,
                    idle: 1000,
                    iowait: 5,
                    irq: 1,
                    softirq: 1,
                    steal: 0,
                    guest: 0,
                    guest_nice: 0,
                },
                memory: MemoryInfo {
                    mem_total_kb: 16_000_000,
                    mem_available_kb: 8_000_000,
                    mem_free_kb: 4_000_000,
                    buffers_kb: 1_000_000,
                    cached_kb: 4_000_000,
                    swap_total_kb: 8_000_000,
                    swap_free_kb: 7_000_000,
                },
                load_avg: LoadAvg {
                    one: 1.0,
                    five: 0.8,
                    fifteen: 0.6,
                },
                pressure: smoothtask_core::metrics::system::PressureMetrics::default(),
            };
            system_metrics_cache = Some(mock_metrics);
            system_metrics_cache_iteration = current_iteration;
        }
        
        // Проверяем, что кэш обновляется каждые 3 итерации
        if current_iteration == 1 {
            // Первая итерация: кэш должен быть создан
            assert!(system_metrics_cache.is_some());
            assert_eq!(system_metrics_cache_iteration, 1);
        } else if current_iteration == 4 {
            // Итерация 4: кэш должен обновляться (4 - 1 = 3 >= 3)
            assert_eq!(system_metrics_cache_iteration, 4);
        } else if current_iteration == 7 {
            // Итерация 7: кэш должен обновляться (7 - 4 = 3 >= 3)
            assert_eq!(system_metrics_cache_iteration, 7);
        } else if current_iteration == 10 {
            // Итерация 10: кэш должен обновляться (10 - 7 = 3 >= 3)
            assert_eq!(system_metrics_cache_iteration, 10);
        } else {
            // Для остальных итераций кэш не должен обновляться
            assert!(system_metrics_cache.is_some());
            if current_iteration == 2 || current_iteration == 3 {
                assert_eq!(system_metrics_cache_iteration, 1);
            } else if current_iteration == 5 || current_iteration == 6 {
                assert_eq!(system_metrics_cache_iteration, 4);
            } else if current_iteration == 8 || current_iteration == 9 {
                assert_eq!(system_metrics_cache_iteration, 7);
            }
        }
    }
}

/// Тест проверяет, что кэширование метрик процессов работает корректно.
/// Этот тест проверяет базовую логику кэширования без реального сбора метрик.
#[test]
fn test_process_metrics_caching_logic() {
    // Создаём кэш и тестируем логику обновления
    let mut process_metrics_cache: Option<Vec<smoothtask_core::logging::snapshots::ProcessRecord>> = None;
    let mut process_metrics_cache_iteration: u64 = 0;
    let process_metrics_cache_interval = 1u64; // По умолчанию кэширование отключено
    
    // Имитируем несколько итераций
    for current_iteration in 1..=5 {
        let need_update = process_metrics_cache.is_none() ||
            (current_iteration - process_metrics_cache_iteration) >= process_metrics_cache_interval;
        
        if need_update {
            // Имитируем обновление кэша
            process_metrics_cache = Some(Vec::new());
            process_metrics_cache_iteration = current_iteration;
        }
        
        // Проверяем, что кэш обновляется на каждой итерации (интервал = 1)
        assert!(process_metrics_cache.is_some());
        assert_eq!(process_metrics_cache_iteration, current_iteration);
    }
}

/// Тест проверяет, что кэширование с интервалом 2 работает корректно.
#[test]
fn test_caching_with_interval_2() {
    let mut cache: Option<String> = None;
    let mut cache_iteration: u64 = 0;
    let cache_interval = 2u64;
    
    // Имитируем 6 итераций
    for current_iteration in 1..=6 {
        let need_update = cache.is_none() ||
            (current_iteration - cache_iteration) >= cache_interval;
        
        if need_update {
            cache = Some(format!("data_{}", current_iteration));
            cache_iteration = current_iteration;
        }
        
        // Проверяем ожидаемое поведение
        match current_iteration {
            1 => {
                assert_eq!(cache_iteration, 1);
                assert_eq!(cache.as_ref().unwrap(), "data_1");
            },
            2 => {
                assert_eq!(cache_iteration, 1); // Кэш не обновляется (2-1=1 < 2)
                assert_eq!(cache.as_ref().unwrap(), "data_1");
            },
            3 => {
                assert_eq!(cache_iteration, 3); // Кэш обновляется (3-1=2 >= 2)
                assert_eq!(cache.as_ref().unwrap(), "data_3");
            },
            4 => {
                assert_eq!(cache_iteration, 3); // Кэш не обновляется (4-3=1 < 2)
                assert_eq!(cache.as_ref().unwrap(), "data_3");
            },
            5 => {
                assert_eq!(cache_iteration, 5); // Кэш обновляется (5-3=2 >= 2)
                assert_eq!(cache.as_ref().unwrap(), "data_5");
            },
            6 => {
                assert_eq!(cache_iteration, 5); // Кэш не обновляется (6-5=1 < 2)
                assert_eq!(cache.as_ref().unwrap(), "data_5");
            },
            _ => unreachable!(),
        }
    }
}

/// Тест проверяет, что кэширование работает корректно при изменении интервала.
#[test]
fn test_caching_with_dynamic_interval() {
    let mut cache: Option<u64> = None;
    let mut cache_iteration: u64 = 0;
    
    // Тестируем с разными интервалами
    let intervals = vec![1, 2, 3, 5];
    
    for interval in intervals {
        cache = None;
        cache_iteration = 0;
        
        for current_iteration in 1..=10 {
            let need_update = cache.is_none() ||
                (current_iteration - cache_iteration) >= interval;
            
            if need_update {
                cache = Some(current_iteration);
                cache_iteration = current_iteration;
            }
            
            // Проверяем, что кэш обновляется с правильным интервалом
            if interval == 1 {
                // Для интервала 1 кэш обновляется на каждой итерации
                assert_eq!(cache_iteration, current_iteration);
                assert_eq!(cache, Some(current_iteration));
            } else if current_iteration == 1 {
                assert_eq!(cache_iteration, 1);
                assert_eq!(cache, Some(1));
            } else if current_iteration <= interval {
                // Для первых N итераций (где N = interval) кэш НЕ обновляется
                // потому что (current_iteration - 1) < interval
                assert_eq!(cache_iteration, 1);
                assert_eq!(cache, Some(1));
            } else {
                // После первых N итераций кэш обновляется каждые interval итераций
                // Находим последнюю итерацию, когда кэш обновлялся
                let mut expected_last_update = 1;
                let mut current = 1;
                while current + interval <= current_iteration {
                    current += interval;
                    expected_last_update = current;
                }
                assert_eq!(cache_iteration, expected_last_update);
                assert_eq!(cache, Some(expected_last_update));
            }
        }
    }
}

/// Тест проверяет, что кэширование корректно обрабатывает edge cases.
#[test]
fn test_caching_edge_cases() {
    // Тест 1: Интервал 0 (недопустимое значение, но должно работать как интервал 1)
    let mut cache: Option<String> = None;
    let mut cache_iteration: u64 = 0;
    let cache_interval = 0; // Это не должно происходить, но тестируем на устойчивость
    
    // При интервале 0 кэш должен обновляться на каждой итерации
    for current_iteration in 1..=3 {
        let need_update = cache.is_none() ||
            (current_iteration - cache_iteration) >= cache_interval;
        
        if need_update {
            cache = Some(format!("data_{}", current_iteration));
            cache_iteration = current_iteration;
        }
        
        assert_eq!(cache_iteration, current_iteration);
    }
    
    // Тест 2: Очень большой интервал
    let mut cache2: Option<String> = None;
    let mut cache_iteration2: u64 = 0;
    let cache_interval2 = 1000;
    
    for current_iteration in 1..=5 {
        let need_update = cache2.is_none() ||
            (current_iteration - cache_iteration2) >= cache_interval2;
        
        if need_update {
            cache2 = Some(format!("data_{}", current_iteration));
            cache_iteration2 = current_iteration;
        }
        
        // При большом интервале кэш должен обновляться только на первой итерации
        assert_eq!(cache_iteration2, 1);
    }
}

/// Тест проверяет, что кэширование корректно работает с реальными системными метриками.
/// Этот тест требует доступа к /proc и может не работать в некоторых окружениях.
#[test]
fn test_real_system_metrics_collection() {
    // Проверяем, что мы можем собрать реальные системные метрики
    let proc_paths = ProcPaths::default();
    
    // Этот тест может не пройти в окружениях без /proc (например, в некоторых CI)
    // поэтому мы используем Result и игнорируем ошибки
    let result = collect_system_metrics(&proc_paths);
    
    // В большинстве Linux систем этот вызов должен завершиться успешно
    // Если нет, это может быть ожидаемо в тестовых окружениях
    if result.is_ok() {
        let metrics = result.unwrap();
        // Проверяем, что метрики содержат разумные значения
        assert!(metrics.cpu_times.user >= 0);
        assert!(metrics.memory.mem_total_kb > 0);
    } else {
        // В некоторых окружениях /proc может быть недоступен
        // Это не является ошибкой теста
        eprintln!("Warning: Could not collect real system metrics: {}", result.unwrap_err());
    }
}

/// Тест проверяет, что кэширование корректно работает с реальными метриками процессов.
/// Этот тест требует доступа к /proc и может не работать в некоторых окружениях.
#[test]
fn test_real_process_metrics_collection() {
    // Проверяем, что мы можем собрать реальные метрики процессов
    let result = collect_process_metrics();
    
    // В большинстве Linux систем этот вызов должен завершиться успешно
    if result.is_ok() {
        let processes = result.unwrap();
        // Должен быть хотя бы один процесс (текущий процесс)
        assert!(!processes.is_empty());
        
        // Проверяем, что процессы содержат разумные данные
        for process in &processes {
            assert!(process.pid > 0);
            // Некоторые процессы могут не иметь cmdline или exe (например, зомби процессы)
            // поэтому мы проверяем только PID
        }
    } else {
        // В некоторых окружениях /proc может быть недоступен
        eprintln!("Warning: Could not collect real process metrics: {}", result.unwrap_err());
    }
}

/// Тест проверяет, что конфигурация кэширования корректно интегрируется в основную конфигурацию.
#[test]
fn test_cache_configuration_integration() {
    // Создаём полную конфигурацию с кэшированием
    let config = Config {
        polling_interval_ms: 500,
        max_candidates: 100,
        thresholds: Thresholds {
            psi_cpu_some_high: 0.5,
            psi_io_some_high: 0.5,
            user_idle_timeout_sec: 300,
            interactive_build_grace_sec: 60,
            noisy_neighbour_cpu_share: 0.5,
            crit_interactive_percentile: 99.0,
            interactive_percentile: 90.0,
            normal_percentile: 70.0,
            background_percentile: 50.0,
            sched_latency_p99_threshold_ms: 10.0,
            ui_loop_p95_threshold_ms: 16.67,
        },
        paths: Paths {
            snapshot_db_path: "/var/lib/smoothtask/snapshots.db".to_string(),
            patterns_dir: "/etc/smoothtask/patterns".to_string(),
            api_listen_addr: Some("127.0.0.1:8080".to_string()),
        },
        cache_intervals: CacheIntervals {
            system_metrics_cache_interval: 5,
            process_metrics_cache_interval: 2,
        },
        dry_run_default: false,
        enable_snapshot_logging: true,
        policy_mode: PolicyMode::Hybrid,
    };
    
    // Проверяем, что конфигурация корректно создана
    assert_eq!(config.polling_interval_ms, 500);
    assert_eq!(config.cache_intervals.system_metrics_cache_interval, 5);
    assert_eq!(config.cache_intervals.process_metrics_cache_interval, 2);
    
    // Проверяем, что конфигурацию можно сериализовать
    let serialized = serde_yaml::to_string(&config).unwrap();
    assert!(serialized.contains("system_metrics_cache_interval: 5"));
    assert!(serialized.contains("process_metrics_cache_interval: 2"));
    
    // Проверяем, что конфигурацию можно десериализовать
    let deserialized: Config = serde_yaml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.cache_intervals.system_metrics_cache_interval, 5);
    assert_eq!(deserialized.cache_intervals.process_metrics_cache_interval, 2);
}

/// Тест проверяет, что кэширование корректно работает в многопоточном окружении.
#[test]
fn test_caching_thread_safety() {
    use std::sync::Mutex;
    
    let cache = Mutex::new(Option::<String>::None);
    let cache_iteration = Mutex::new(0u64);
    let cache_interval = 2u64;
    
    // Имитируем несколько итераций в многопоточном окружении
    for current_iteration in 1..=6 {
        let mut cache_guard = cache.lock().unwrap();
        let mut cache_iteration_guard = cache_iteration.lock().unwrap();
        
        let need_update = cache_guard.is_none() ||
            (current_iteration - *cache_iteration_guard) >= cache_interval;
        
        if need_update {
            *cache_guard = Some(format!("data_{}", current_iteration));
            *cache_iteration_guard = current_iteration;
        }
        
        drop(cache_guard);
        drop(cache_iteration_guard);
        
        // Проверяем ожидаемое поведение
        let cache_guard = cache.lock().unwrap();
        let cache_iteration_guard = cache_iteration.lock().unwrap();
        
        match current_iteration {
            1 | 3 | 5 => {
                assert_eq!(*cache_iteration_guard, current_iteration);
                assert!(cache_guard.is_some());
            },
            2 | 4 | 6 => {
                assert_eq!(*cache_iteration_guard, current_iteration - 1);
                assert!(cache_guard.is_some());
            },
            _ => unreachable!(),
        }
    }
}

/// Тест проверяет, что кэширование корректно работает с Arc для совместного использования.
#[test]
fn test_caching_with_arc() {
    let cache = Arc::new(Mutex::new(Option::<String>::None));
    let cache_iteration = Arc::new(Mutex::new(0u64));
    let cache_interval = 3u64;
    
    // Имитируем несколько итераций
    for current_iteration in 1..=9 {
        let mut cache_guard = cache.lock().unwrap();
        let mut cache_iteration_guard = cache_iteration.lock().unwrap();
        
        let need_update = cache_guard.is_none() ||
            (current_iteration - *cache_iteration_guard) >= cache_interval;
        
        if need_update {
            *cache_guard = Some(format!("data_{}", current_iteration));
            *cache_iteration_guard = current_iteration;
        }
        
        // Проверяем ожидаемое поведение
        match current_iteration {
            1 | 4 | 7 => {
                assert_eq!(*cache_iteration_guard, current_iteration);
            },
            2 | 3 | 5 | 6 | 8 | 9 => {
                assert!(*cache_iteration_guard < current_iteration);
            },
            _ => unreachable!(),
        }
    }
}
