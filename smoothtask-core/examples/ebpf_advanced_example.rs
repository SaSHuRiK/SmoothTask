//! Продвинутый пример использования eBPF модуля с демонстрацией
//! конфигурации, обработки ошибок и интеграции с другими компонентами.
//!
//! Этот пример показывает, как интегрировать eBPF модуль в реальное приложение
//! с поддержкой динамической конфигурации и обработки ошибок.

use smoothtask_core::metrics::ebpf::{
    EbpfConfig, EbpfFilterConfig, EbpfMetrics, EbpfMetricsCollector, EbpfNotificationThresholds,
};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Структура для хранения глобального состояния приложения
struct AppState {
    ebpf_collector: Option<EbpfMetricsCollector>,
    config: EbpfConfig,
    last_metrics: Option<EbpfMetrics>,
    error_count: u32,
}

impl AppState {
    fn new() -> Self {
        Self {
            ebpf_collector: None,
            config: EbpfConfig::default(),
            last_metrics: None,
            error_count: 0,
        }
    }

    /// Инициализация eBPF коллектора
    fn initialize_ebpf(&mut self) -> anyhow::Result<()> {
        println!("Initializing eBPF collector...");

        // Проверка поддержки eBPF
        if !EbpfMetricsCollector::check_ebpf_support()? {
            println!("eBPF not supported, skipping initialization");
            return Ok(());
        }

        // Создание нового коллектора
        let mut collector = EbpfMetricsCollector::new(self.config.clone());
        collector.initialize()?;

        self.ebpf_collector = Some(collector);
        println!("eBPF collector initialized successfully");

        Ok(())
    }

    /// Сбор метрик с обработкой ошибок
    fn collect_metrics(&mut self) -> anyhow::Result<Option<EbpfMetrics>> {
        if let Some(collector) = &mut self.ebpf_collector {
            match collector.collect_metrics() {
                Ok(metrics) => {
                    self.last_metrics = Some(metrics.clone());
                    self.error_count = 0;
                    Ok(Some(metrics))
                }
                Err(e) => {
                    self.error_count += 1;
                    println!(
                        "Error collecting metrics (count: {}): {}",
                        self.error_count, e
                    );

                    // Попытка восстановления после нескольких ошибок
                    if self.error_count >= 3 {
                        println!("Too many errors, attempting recovery...");
                        self.attempt_recovery()?;
                    }

                    // Возвращаем последние успешные метрики (если есть)
                    Ok(self.last_metrics.clone())
                }
            }
        } else {
            Ok(None)
        }
    }

    /// Попытка восстановления
    fn attempt_recovery(&mut self) -> anyhow::Result<()> {
        if let Some(collector) = &mut self.ebpf_collector {
            match collector.attempt_recovery() {
                Ok(_) => {
                    println!("Recovery successful");
                    self.error_count = 0;
                    Ok(())
                }
                Err(e) => {
                    println!("Recovery failed: {}", e);
                    Err(e)
                }
            }
        } else {
            // Попробуем переинициализировать
            self.initialize_ebpf()
        }
    }

    /// Обновление конфигурации
    fn update_config(&mut self, new_config: EbpfConfig) -> anyhow::Result<()> {
        println!("Updating eBPF configuration...");
        self.config = new_config;

        // Переинициализация коллектора с новой конфигурацией
        self.initialize_ebpf()?;

        println!("Configuration updated successfully");
        Ok(())
    }

    /// Получение текущего состояния
    fn get_status(&self) -> String {
        if let Some(collector) = &self.ebpf_collector {
            if collector.is_initialized() {
                format!(
                    "eBPF Status: Initialized, Errors: {}",
                    if collector.has_errors() { "Yes" } else { "No" }
                )
            } else {
                "eBPF Status: Not initialized".to_string()
            }
        } else {
            "eBPF Status: Disabled".to_string()
        }
    }
}

fn main() -> anyhow::Result<()> {
    println!("=== SmoothTask Advanced eBPF Example ===\n");

    // Создание общего состояния (для многопоточности)
    let state = Arc::new(Mutex::new(AppState::new()));

    // 1. Начальная конфигурация
    println!("1. Setting up initial configuration...");
    let initial_config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        enable_syscall_monitoring: true,
        enable_network_monitoring: true,
        enable_network_connections: true,
        enable_caching: true,
        batch_size: 100,
        ..Default::default()
    };

    {
        let mut state_lock = state.lock().unwrap();
        state_lock.update_config(initial_config)?;
    }

    // 2. Запуск мониторинга в отдельном потоке
    println!("\n2. Starting monitoring thread...");
    let monitoring_state = Arc::clone(&state);
    let monitoring_handle = thread::spawn(move || monitoring_thread(monitoring_state));

    // 3. Запуск потока симуляции конфигурации
    println!("3. Starting configuration simulation thread...");
    let config_state = Arc::clone(&state);
    let config_handle = thread::spawn(move || config_simulation_thread(config_state));

    // 4. Главный поток - мониторинг состояния
    println!("\n4. Main thread - monitoring application status...");
    for i in 1..=20 {
        let status = {
            let state_lock = state.lock().unwrap();
            state_lock.get_status()
        };

        println!("Status update {}: {}", i, status);

        // Проверка ошибок
        let error_count = {
            let state_lock = state.lock().unwrap();
            state_lock.error_count
        };

        if error_count > 0 {
            println!("  Warning: {} errors detected", error_count);
        }

        thread::sleep(Duration::from_secs(2));
    }

    // 5. Остановка потоков
    println!("\n5. Stopping threads...");

    // В реальном приложении здесь была бы более изящная остановка
    // Для примера просто дожидаемся завершения
    monitoring_handle.join().unwrap();
    config_handle.join().unwrap();

    println!("\n=== Advanced example completed! ===");
    Ok(())
}

/// Поток мониторинга метрик
fn monitoring_thread(state: Arc<Mutex<AppState>>) {
    println!("Monitoring thread started");

    let mut iteration = 0;
    loop {
        iteration += 1;

        let metrics_result = {
            let mut state_lock = state.lock().unwrap();
            state_lock.collect_metrics()
        };

        match metrics_result {
            Ok(Some(metrics)) => {
                println!("\nMonitoring iteration {}:", iteration);
                println!(
                    "  CPU: {:.2}%, Memory: {} MB",
                    metrics.cpu_usage,
                    metrics.memory_usage / 1024 / 1024
                );

                // Анализ метрик
                analyze_metrics(&metrics);
            }
            Ok(None) => {
                println!("Monitoring iteration {}: No metrics available", iteration);
            }
            Err(e) => {
                println!("Monitoring iteration {}: Error - {}", iteration, e);
            }
        }

        // Пауза между итерациями
        thread::sleep(Duration::from_secs(1));

        // Ограничение для примера
        if iteration >= 15 {
            break;
        }
    }

    println!("Monitoring thread completed");
}

/// Поток симуляции изменений конфигурации
fn config_simulation_thread(state: Arc<Mutex<AppState>>) {
    println!("Configuration simulation thread started");

    // Симуляция изменений конфигурации
    for change in 1..=3 {
        println!("\nConfiguration change {}:", change);

        // Новая конфигурация
        let new_config = match change {
            1 => EbpfConfig {
                enable_cpu_metrics: true,
                enable_memory_metrics: true,
                enable_syscall_monitoring: false, // Отключаем мониторинг системных вызовов
                enable_network_monitoring: true,
                enable_network_connections: true,
                enable_caching: true,
                aggressive_cache_interval_ms: 3000, // Более агрессивное кэширование
                ..Default::default()
            },
            2 => EbpfConfig {
                enable_cpu_metrics: true,
                enable_memory_metrics: true,
                enable_syscall_monitoring: true, // Включаем обратно
                enable_network_monitoring: true,
                enable_network_connections: true,
                enable_caching: false, // Отключаем кэширование для точности
                ..Default::default()
            },
            3 => EbpfConfig {
                enable_cpu_metrics: true,
                enable_memory_metrics: true,
                enable_syscall_monitoring: true,
                enable_network_monitoring: true,
                enable_network_connections: true,
                enable_caching: true,
                batch_size: 200, // Увеличиваем размер batch
                ..Default::default()
            },
            _ => EbpfConfig::default(),
        };

        // Обновление конфигурации
        let result = {
            let mut state_lock = state.lock().unwrap();
            state_lock.update_config(new_config)
        };

        match result {
            Ok(_) => println!("  ✓ Configuration updated successfully"),
            Err(e) => println!("  ✗ Configuration update failed: {}", e),
        }

        // Пауза между изменениями
        thread::sleep(Duration::from_secs(5));
    }

    println!("Configuration simulation thread completed");
}

/// Анализ метрик и генерация предупреждений
fn analyze_metrics(metrics: &EbpfMetrics) {
    let mut warnings = Vec::new();

    // Проверка высокой загрузки CPU
    if metrics.cpu_usage > 80.0 {
        warnings.push(format!("High CPU usage: {:.1}%", metrics.cpu_usage));
    }

    // Проверка высокого потребления памяти
    let memory_mb = metrics.memory_usage / 1024 / 1024;
    if memory_mb > 8000 {
        // 8GB
        warnings.push(format!("High memory usage: {} MB", memory_mb));
    }

    // Проверка высокой активности системных вызовов
    if metrics.syscall_count > 10000 {
        warnings.push(format!(
            "High system call activity: {}",
            metrics.syscall_count
        ));
    }

    // Проверка большого количества сетевых соединений
    if metrics.active_connections > 100 {
        warnings.push(format!(
            "High network connection count: {}",
            metrics.active_connections
        ));
    }

    // Отображение предупреждений
    if !warnings.is_empty() {
        println!("  ⚠ Warnings:");
        for warning in warnings {
            println!("    - {}", warning);
        }
    } else {
        println!("  ✓ System metrics look normal");
    }
}

/// Демонстрация интеграции с другими компонентами
fn demonstrate_integration() -> anyhow::Result<()> {
    println!("\n=== Integration Demonstration ===");

    // 1. Интеграция с системой уведомлений
    println!("\n1. Notification System Integration:");

    let config = EbpfConfig {
        enable_cpu_metrics: true,
        enable_memory_metrics: true,
        ..Default::default()
    };

    let mut collector = EbpfMetricsCollector::new(config.clone());

    if collector.initialize().is_ok() {
        if let Ok(metrics) = collector.collect_metrics() {
            // Симуляция отправки уведомления
            if metrics.cpu_usage > 90.0 {
                println!(
                    "   🔔 Notification: High CPU usage detected ({:.1}%)",
                    metrics.cpu_usage
                );
                println!("   Action: Consider adjusting process priorities");
            }
        }
    }

    // 2. Интеграция с системой логирования
    println!("\n2. Logging System Integration:");

    let status = collector.is_initialized();
    println!(
        "   📝 Log: eBPF collector status - {}",
        if status {
            "initialized"
        } else {
            "not initialized"
        }
    );

    if collector.has_errors() {
        if let Some(error_info) = collector.get_detailed_error_info() {
            println!("   ❌ Log: eBPF error detected - {}", error_info);
        }
    }

    // 3. Интеграция с системой мониторинга
    println!("\n3. Monitoring System Integration:");

    let memory_usage = collector.get_memory_usage_estimate();
    println!("   📊 Metric: eBPF memory usage - {} bytes", memory_usage);

    let (success, errors) = collector.get_initialization_stats();
    println!(
        "   📊 Metric: eBPF programs loaded - {}, errors - {}",
        success, errors
    );

    // 4. Интеграция с системой конфигурации
    println!("\n4. Configuration System Integration:");

    // Симуляция динамического изменения конфигурации
    let new_config = EbpfConfig {
        enable_caching: true,
        aggressive_cache_interval_ms: 10000,
        ..config
    };

    let mut new_collector = EbpfMetricsCollector::new(new_config);
    if new_collector.initialize().is_ok() {
        println!("   ✅ Config: Successfully updated to aggressive caching mode");
    }

    Ok(())
}
