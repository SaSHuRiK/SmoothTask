//! Интеграционные тесты для системы автоматического масштабирования виртуальных машин
//!
//! Эти тесты проверяют:
//! - Полный цикл автоматического масштабирования
//! - Интеграцию с системой мониторинга VM
//! - Корректность расчета ресурсов
//! - Обработку различных сценариев нагрузки

use anyhow::Result;
use smoothtask_core::metrics::vm::*;

#[tokio::test]
async fn test_vm_auto_scaling_complete_cycle() -> Result<()> {
    // Тестируем полный цикл автоматического масштабирования
    
    // 1. Мониторинг нагрузки VM
    let metrics = monitor_vm_load("test_vm").await?;
    assert_eq!(metrics.id, "test_vm");
    assert!(metrics.cpu_usage.total_usage > 0.0);
    
    // 2. Анализ использования ресурсов
    let analysis = analyze_vm_resource_usage(&metrics)?;
    assert_eq!(analysis.vm_id, "test_vm");
    assert!(analysis.overall_utilization >= 0.0 && analysis.overall_utilization <= 100.0);
    
    // 3. Расчет потребностей в масштабировании
    let scaling_plan = calculate_vm_scaling_needs(&analysis)?;
    assert_eq!(scaling_plan.vm_id, "test_vm");
    assert!(!scaling_plan.recommendation.is_empty());
    
    // 4. Применение автоматического масштабирования
    let result = apply_vm_auto_scaling("test_vm", &scaling_plan, &metrics).await?;
    assert!(result.success);
    
    Ok(())
}

#[tokio::test]
async fn test_vm_scaling_overloaded_scenario() -> Result<()> {
    // Тестируем сценарий перегруженной VM
    
    // Создаем метрики для перегруженной VM
    let mut metrics = monitor_vm_load("vm2").await?; // vm2 имеет высокую нагрузку
    
    // Анализируем ресурсы
    let analysis = analyze_vm_resource_usage(&metrics)?;
    assert!(analysis.is_overloaded || analysis.overall_utilization > 50.0);
    
    // Рассчитываем масштабирование
    let scaling_plan = calculate_vm_scaling_needs(&analysis)?;
    if analysis.is_overloaded {
        assert!(scaling_plan.should_scale);
        assert!(scaling_plan.recommendation.contains("Scale up"));
    }
    
    Ok(())
}

#[tokio::test]
async fn test_vm_scaling_underutilized_scenario() -> Result<()> {
    // Тестируем сценарий недогруженной VM
    
    // Создаем метрики для недогруженной VM
    let mut metrics = monitor_vm_load("vm3").await?; // vm3 имеет низкую нагрузку
    
    // Анализируем ресурсы
    let analysis = analyze_vm_resource_usage(&metrics)?;
    assert!(analysis.is_underutilized || analysis.overall_utilization < 40.0);
    
    // Рассчитываем масштабирование
    let scaling_plan = calculate_vm_scaling_needs(&analysis)?;
    if analysis.is_underutilized {
        assert!(scaling_plan.should_scale);
        assert!(scaling_plan.recommendation.contains("Scale down"));
    }
    
    Ok(())
}

#[tokio::test]
async fn test_vm_scaling_optimal_scenario() -> Result<()> {
    // Тестируем сценарий оптимально загруженной VM
    
    // Создаем метрики для оптимально загруженной VM
    let metrics = monitor_vm_load("test_vm").await?;
    
    // Анализируем ресурсы
    let analysis = analyze_vm_resource_usage(&metrics)?;
    assert!(!analysis.is_overloaded);
    assert!(!analysis.is_underutilized);
    
    // Рассчитываем масштабирование
    let scaling_plan = calculate_vm_scaling_needs(&analysis)?;
    assert!(!scaling_plan.should_scale);
    assert!(scaling_plan.recommendation.contains("No scaling needed"));
    
    // Применяем масштабирование (должно вернуть success без изменений)
    let result = apply_vm_auto_scaling("test_vm", &scaling_plan, &metrics).await?;
    assert!(result.success);
    assert!(result.output.contains("No scaling needed"));
    
    Ok(())
}

#[tokio::test]
async fn test_vm_scaling_resource_calculation() -> Result<()> {
    // Тестируем корректность расчета ресурсов при масштабировании
    
    let metrics = monitor_vm_load("test_vm").await?;
    let analysis = analyze_vm_resource_usage(&metrics)?;
    let scaling_plan = calculate_vm_scaling_needs(&analysis)?;
    
    // Проверяем, что факторы масштабирования корректны
    assert!(scaling_plan.cpu_scaling_factor > 0.0);
    assert!(scaling_plan.memory_scaling_factor > 0.0);
    assert!(scaling_plan.disk_scaling_factor > 0.0);
    assert!(scaling_plan.network_scaling_factor > 0.0);
    
    // Проверяем, что факторы масштабирования находятся в разумных пределах
    assert!(scaling_plan.cpu_scaling_factor >= 0.5 && scaling_plan.cpu_scaling_factor <= 2.0);
    assert!(scaling_plan.memory_scaling_factor >= 0.5 && scaling_plan.memory_scaling_factor <= 2.0);
    assert!(scaling_plan.disk_scaling_factor >= 0.5 && scaling_plan.disk_scaling_factor <= 2.0);
    assert!(scaling_plan.network_scaling_factor >= 0.5 && scaling_plan.network_scaling_factor <= 2.0);
    
    Ok(())
}

#[tokio::test]
async fn test_vm_scaling_multiple_vms() -> Result<()> {
    // Тестируем масштабирование для нескольких VM
    
    let vm_ids = ["test_vm", "vm1", "vm2", "vm3"];
    
    for vm_id in vm_ids.iter() {
        // Мониторинг нагрузки
        let metrics = monitor_vm_load(vm_id).await?;
        
        // Анализ ресурсов
        let analysis = analyze_vm_resource_usage(&metrics)?;
        
        // Расчет масштабирования
        let scaling_plan = calculate_vm_scaling_needs(&analysis)?;
        
        // Применение масштабирования
        let result = apply_vm_auto_scaling(vm_id, &scaling_plan, &metrics).await?;
        
        assert!(result.success);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_vm_scaling_error_handling() -> Result<()> {
    // Тестируем обработку ошибок
    
    // Пытаемся применить масштабирование для несуществующей VM
    let metrics = monitor_vm_load("test_vm").await?;
    let analysis = analyze_vm_resource_usage(&metrics)?;
    let scaling_plan = calculate_vm_scaling_needs(&analysis)?;
    
    let result = apply_vm_auto_scaling("nonexistent_vm", &scaling_plan, &metrics).await?;
    assert!(!result.success);
    assert!(result.output.contains("VM not found"));
    
    Ok(())
}

#[tokio::test]
async fn test_vm_scaling_statistics() -> Result<()> {
    // Тестируем статистику масштабирования
    
    let metrics = monitor_vm_load("test_vm").await?;
    let analysis = analyze_vm_resource_usage(&metrics)?;
    
    // Проверяем, что статистика анализа корректна
    assert!(analysis.cpu_utilization >= 0.0 && analysis.cpu_utilization <= 100.0);
    assert!(analysis.memory_utilization >= 0.0 && analysis.memory_utilization <= 100.0);
    assert!(analysis.disk_utilization >= 0.0 && analysis.disk_utilization <= 100.0);
    assert!(analysis.network_utilization >= 0.0 && analysis.network_utilization <= 100.0);
    assert!(analysis.overall_utilization >= 0.0 && analysis.overall_utilization <= 100.0);
    
    // Проверяем, что сумма процентов примерно равна общему проценту
    let sum = analysis.cpu_utilization + analysis.memory_utilization + analysis.disk_utilization + analysis.network_utilization;
    let avg = sum / 4.0;
    assert!((avg - analysis.overall_utilization).abs() < 1.0);
    
    Ok(())
}
