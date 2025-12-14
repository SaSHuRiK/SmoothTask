// Интеграционный тест для мониторинга энергопотребления процессов
//
// Этот тест проверяет интеграцию всех компонентов мониторинга энергопотребления
// и их взаимодействие с другими частями системы.

use smoothtask_core::metrics::process_energy::{ProcessEnergyMonitor, ProcessEnergyStats, EnergySource};
use smoothtask_core::logging::snapshots::ProcessRecord;
use tokio::test;

#[test]
async fn test_process_energy_integration_with_system() {
    // Тестируем интеграцию с реальной системой
    let monitor = ProcessEnergyMonitor::new();
    
    // Пробуем получить метрики для системных процессов
    let test_pids = [1, 2]; // PID 1 - обычно init/systemd, PID 2 - обычно kthreadd
    
    for &pid in &test_pids {
        let result = monitor.collect_process_energy(pid).await;
        
        // Должно всегда возвращать Ok
        assert!(result.is_ok(), "Failed to collect energy for PID {}", pid);
        
        let stats = result.unwrap();
        
        // Может быть None, если данные недоступны, но не должно быть ошибок
        if let Some(energy_stats) = stats {
            // Проверяем, что статистика корректна
            assert_eq!(energy_stats.pid, pid);
            assert!(energy_stats.energy_uj > 0 || !energy_stats.is_reliable);
            assert!(energy_stats.power_w >= 0.0);
            assert!(energy_stats.timestamp > 0);
            
            // Проверяем, что источник данных корректен
            match energy_stats.source {
                EnergySource::ProcPower => assert!(energy_stats.is_reliable),
                EnergySource::Ebpf => assert!(energy_stats.is_reliable),
                EnergySource::Rapl => assert!(energy_stats.is_reliable),
                EnergySource::None => assert!(!energy_stats.is_reliable), // Fallback источник
            }
        }
    }
}

#[test]
async fn test_process_energy_batch_collection() {
    // Тестируем пакетный сбор метрик
    let monitor = ProcessEnergyMonitor::new();
    
    let pids = [1, 2, 3, 4, 5]; // Несколько системных процессов
    
    let results = monitor.collect_batch_energy(&pids).await;
    
    assert!(results.is_ok(), "Batch collection failed");
    
    let stats = results.unwrap();
    
    // Должны получить результаты только для существующих процессов
    assert!(stats.len() <= pids.len());
    
    // Проверяем, что все результаты корректны
    for stat in &stats {
        assert!(pids.contains(&stat.pid));
        assert!(stat.energy_uj > 0 || !stat.is_reliable);
        assert!(stat.power_w >= 0.0);
        assert!(stat.timestamp > 0);
    }
}

#[test]
async fn test_process_energy_record_enhancement() {
    // Тестируем улучшение ProcessRecord данными о энергопотреблении
    let monitor = ProcessEnergyMonitor::new();
    
    // Создаем тестовый ProcessRecord
    let mut record = ProcessRecord::default();
    record.pid = 1;
    
    // Пробуем улучшить запись
    let result = monitor.collect_process_energy(1).await;
    
    if let Ok(Some(energy_stats)) = result {
        let enhanced_record = monitor.enhance_process_record(record, Some(energy_stats));
        
        // Проверяем, что запись была улучшена
        assert!(enhanced_record.energy_uj.is_some());
        assert!(enhanced_record.power_w.is_some());
        assert!(enhanced_record.energy_timestamp.is_some());
        
        // Проверяем, что значения корректны
        assert!(enhanced_record.energy_uj.unwrap() > 0 || !energy_stats.is_reliable);
        assert!(enhanced_record.power_w.unwrap() >= 0.0);
        assert!(enhanced_record.energy_timestamp.unwrap() > 0);
    }
}

#[test]
async fn test_process_energy_with_different_configurations() {
    // Тестируем монитор с разными конфигурациями
    let configs = [
        (true, true),   // RAPL и eBPF включены
        (true, false),  // Только RAPL
        (false, true),  // Только eBPF
        (false, false), // Все отключено
    ];
    
    for (enable_rapl, enable_ebpf) in configs {
        let monitor = ProcessEnergyMonitor::with_config(enable_rapl, enable_ebpf);
        
        let result = monitor.collect_process_energy(1).await;
        
        // Должно всегда возвращать Ok
        assert!(result.is_ok(), "Failed with config RAPL={}, eBPF={}", enable_rapl, enable_ebpf);
        
        // Если все отключено, должно вернуть None (или fallback)
        if !enable_rapl && !enable_ebpf {
            let stats = result.unwrap();
            // Может быть None или fallback данные
            if let Some(energy_stats) = stats {
                // Если есть данные, они должны быть помечены как ненадежные (fallback)
                assert!(!energy_stats.is_reliable);
            }
        }
    }
}

#[test]
async fn test_process_energy_error_handling() {
    // Тестируем обработку ошибок
    let monitor = ProcessEnergyMonitor::new();
    
    // Пробуем несуществующий PID
    let result = monitor.collect_process_energy(999999).await;
    
    // Должно вернуть Ok(None) без паники
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
    
    // Пробуем синхронную версию
    let sync_result = monitor.collect_process_energy_sync(999999);
    
    // Должно вернуть Ok(None) без паники
    assert!(sync_result.is_ok());
    assert!(sync_result.unwrap().is_none());
}

#[test]
async fn test_process_energy_global_monitor() {
    // Тестируем глобальный монитор
    use smoothtask_core::metrics::process_energy::GlobalProcessEnergyMonitor;
    
    // Пробуем получить метрики через глобальный монитор
    let result = GlobalProcessEnergyMonitor::collect_process_energy(1).await;
    
    // Должно вернуть Ok
    assert!(result.is_ok());
    
    // Пробуем улучшить запись через глобальный монитор
    let mut record = ProcessRecord::default();
    record.pid = 1;
    
    let enhanced_result = GlobalProcessEnergyMonitor::enhance_process_record(record).await;
    
    // Должно вернуть Ok
    assert!(enhanced_result.is_ok());
    
    let enhanced_record = enhanced_result.unwrap();
    
    // Если есть данные о энергопотреблении, запись должна быть улучшена
    if enhanced_record.energy_uj.is_some() {
        assert!(enhanced_record.power_w.is_some());
        assert!(enhanced_record.energy_timestamp.is_some());
    }
}

#[test]
async fn test_process_energy_serialization() {
    // Тестируем сериализацию и десериализацию статистики энергопотребления
    let stats = ProcessEnergyStats {
        pid: 123,
        energy_uj: 1000000,
        power_w: 15.5,
        timestamp: 1234567890,
        source: EnergySource::ProcPower,
        is_reliable: true,
    };
    
    // Сериализация
    let serialized = serde_json::to_string(&stats).unwrap();
    
    // Десериализация
    let deserialized: ProcessEnergyStats = serde_json::from_str(&serialized).unwrap();
    
    // Проверяем, что данные совпадают
    assert_eq!(stats.pid, deserialized.pid);
    assert_eq!(stats.energy_uj, deserialized.energy_uj);
    assert_eq!(stats.power_w, deserialized.power_w);
    assert_eq!(stats.timestamp, deserialized.timestamp);
    assert_eq!(stats.source, deserialized.source);
    assert_eq!(stats.is_reliable, deserialized.is_reliable);
}

#[test]
async fn test_process_energy_performance() {
    // Тестируем производительность сбора метрик
    let monitor = ProcessEnergyMonitor::new();
    
    // Собираем метрики для нескольких процессов и измеряем время
    let start_time = std::time::Instant::now();
    
    let pids = [1, 2, 3, 4, 5];
    for &pid in &pids {
        let _ = monitor.collect_process_energy(pid).await;
    }
    
    let duration = start_time.elapsed();
    
    // Сбор метрик не должен занимать слишком много времени
    // Это очень щедрый лимит - в реальности должно быть намного быстрее
    assert!(duration.as_secs() < 5, "Process energy collection took too long: {:?}", duration);
}

#[test]
async fn test_process_energy_fallback_mechanism() {
    // Тестируем fallback механизм
    let monitor = ProcessEnergyMonitor::new();
    
    // Пробуем получить метрики для процесса, который может не иметь прямых данных
    let result = monitor.collect_process_energy(1).await;
    
    assert!(result.is_ok());
    
    if let Some(stats) = result.unwrap() {
        // Если есть данные, проверяем их надежность
        if stats.source == EnergySource::None {
            // Fallback данные должны быть помечены как ненадежные
            assert!(!stats.is_reliable);
        } else {
            // Прямые данные должны быть надежными
            assert!(stats.is_reliable);
        }
    }
}