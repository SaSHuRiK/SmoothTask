# Мониторинг энергопотребления процессов

## Обзор

Модуль мониторинга энергопотребления процессов предоставляет функциональность для сбора метрик энергопотребления на уровне отдельных процессов. Это позволяет SmoothTask оптимизировать политики управления ресурсами с учетом энергоэффективности.

## Архитектура

### Основные компоненты

1. **ProcessEnergyMonitor**: Основной монитор энергопотребления процессов
2. **ProcessEnergyStats**: Структура для хранения статистики энергопотребления
3. **EnergySource**: Перечисление доступных источников данных
4. **GlobalProcessEnergyMonitor**: Глобальный экземпляр для удобного доступа

### Поддерживаемые источники данных

1. **/proc/[pid]/power/energy_uj**: Экспериментальный интерфейс ядра Linux
2. **RAPL (Running Average Power Limit)**: Интерфейс Intel для мониторинга энергопотребления
3. **eBPF**: Высокопроизводительный мониторинг через eBPF программы

## API

### ProcessEnergyMonitor

```rust
use smoothtask_core::metrics::process_energy;

// Создать новый монитор
let monitor = ProcessEnergyMonitor::new();

// Создать монитор с кастомной конфигурацией
let monitor = ProcessEnergyMonitor::with_config(true, false); // RAPL включен, eBPF отключен

// Собрать метрики для процесса (асинхронно)
let stats = monitor.collect_process_energy(1234).await?;

// Собрать метрики для процесса (синхронно)
let stats = monitor.collect_process_energy_sync(1234)?;

// Собрать метрики для нескольких процессов
let stats = monitor.collect_batch_energy(&[1234, 5678]).await?;

// Обновить ProcessRecord данными о энергопотреблении
let enhanced_record = monitor.enhance_process_record(record, stats);
```

### GlobalProcessEnergyMonitor

```rust
use smoothtask_core::metrics::process_energy;

// Собрать метрики для процесса
let stats = GlobalProcessEnergyMonitor::collect_process_energy(1234).await?;

// Обновить ProcessRecord
let enhanced_record = GlobalProcessEnergyMonitor::enhance_process_record(record).await?;
```

## Структуры данных

### ProcessEnergyStats

```rust
pub struct ProcessEnergyStats {
    pub pid: i32,                  // Идентификатор процесса
    pub energy_uj: u64,            // Потребление энергии в микроджоулях
    pub power_w: f32,              // Мгновенная мощность в ваттах
    pub timestamp: u64,            // Время последнего измерения (timestamp в секундах)
    pub source: EnergySource,      // Источник данных
    pub is_reliable: bool,         // Признак достоверности данных
}
```

### EnergySource

```rust
pub enum EnergySource {
    ProcPower,    // Данные из /proc/[pid]/power/energy_uj
    Rapl,         // Данные из RAPL
    Ebpf,         // Данные из eBPF мониторинга
    None,         // Данные недоступны
}
```

## Интеграция с существующей системой

### Интеграция с ProcessRecord

Модуль автоматически интегрируется с существующей системой метрик процессов:

```rust
// В ProcessRecord добавлены поля:
pub energy_uj: Option<u64>,        // Энергопотребление в микроджоулях
pub power_w: Option<f32>,          // Мгновенная мощность в ваттах
pub energy_timestamp: Option<u64>, // Время последнего измерения
```

### Интеграция с eBPF

Модуль интегрируется с существующей инфраструктурой eBPF:

```rust
// В EbpfMetrics добавлено поле:
pub process_energy_details: Option<Vec<ProcessEnergyStat>>,

// В ProcessEnergyStat:
pub pid: u32,
pub tgid: u32,
pub energy_uj: u64,
pub last_update_ns: u64,
pub cpu_id: u32,
pub name: String,
pub energy_w: f32,
```

## Конфигурация

### Параметры конфигурации

```rust
ProcessEnergyMonitor::with_config(enable_rapl: bool, enable_ebpf: bool)
```

- `enable_rapl`: Включить использование RAPL интерфейсов
- `enable_ebpf`: Включить интеграцию с eBPF мониторингом

### Приоритет источников данных

Монитор пробует получить данные из источников в следующем порядке приоритета:

1. `/proc/[pid]/power/energy_uj` (наиболее точный)
2. eBPF мониторинг (если включен)
3. RAPL (если доступен)

## Производительность

### Оптимизации

1. **Кэширование**: В будущем можно добавить кэширование последних измерений
2. **Batch обработка**: Поддержка сбора метрик для нескольких процессов одновременно
3. **Асинхронный API**: Полная поддержка асинхронных операций
4. **Синхронный wrapper**: Для интеграции с синхронным кодом

### Производительность

- Низкие накладные расходы на сбор метрик
- Поддержка параллельной обработки нескольких процессов
- Оптимизированное использование системных вызовов

## Тестирование

### Unit тесты

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_process_energy_stats_default() {
        let stats = ProcessEnergyStats::default();
        assert_eq!(stats.pid, 0);
        assert_eq!(stats.energy_uj, 0);
        assert_eq!(stats.power_w, 0.0);
        assert_eq!(stats.timestamp, 0);
        assert_eq!(stats.source, EnergySource::None);
        assert!(!stats.is_reliable);
    }
    
    #[test]
    fn test_sync_wrapper() {
        let monitor = ProcessEnergyMonitor::new();
        let result = monitor.collect_process_energy_sync(999999);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
```

### Интеграционные тесты

Тесты покрывают:
- Все варианты EnergySource
- Различные конфигурации монитора
- Сериализацию и десериализацию
- Интеграцию с ProcessRecord
- Batch обработку
- Обработку ошибок

## Примеры использования

### Базовый пример

```rust
use smoothtask_core::metrics::process_energy;

async fn monitor_process_energy() {
    let monitor = ProcessEnergyMonitor::new();
    
    // Собрать метрики для текущего процесса
    let current_pid = std::process::id() as i32;
    let stats = monitor.collect_process_energy(current_pid).await;
    
    if let Ok(Some(stats)) = stats {
        println!("Process {} energy: {} µJ ({} W) from {:?}", 
                 stats.pid, stats.energy_uj, stats.power_w, stats.source);
    } else {
        println!("Energy data not available");
    }
}
```

### Интеграция с системой метрик

```rust
use smoothtask_core::metrics::process;

// Автоматический сбор метрик энергопотребления включен в process::collect_process_metrics()
let records = process::collect_process_metrics(None).unwrap();

for record in records {
    if let Some(energy) = record.energy_uj {
        println!("Process {}: {} µJ", record.pid, energy);
    }
}
```

## Ограничения и будущие улучшения

### Текущие ограничения

1. **RAPL**: Упрощенное сопоставление процессов с доменами RAPL
2. **eBPF**: Требует более тесной интеграции с существующей инфраструктурой
3. **Кэширование**: Пока не реализовано, но запланировано

### Будущие улучшения

1. **Улучшенное сопоставление RAPL**: Более точное сопоставление процессов с доменами RAPL
2. **Расширенная интеграция eBPF**: Более тесная интеграция с существующими eBPF программами
3. **Кэширование**: Реализация кэширования последних измерений
4. **Мониторинг трендов**: Отслеживание изменений энергопотребления во времени
5. **Агрегация данных**: Агрегация данных по группам процессов и приложений

## Отладка

### Логирование

Модуль использует `tracing` для логирования:

```rust
tracing::debug!("Энергопотребление процесса PID {}: {} мкДж ({} Вт) [источник: {:?}]",
                pid, stats.energy_uj, stats.power_w, stats.source);
```

### Обработка ошибок

Модуль предоставляет подробную информацию об ошибках:

```rust
tracing::warn!("Ошибка при сборе метрик энергопотребления для процесса PID {}: {}",
                pid, e);
```

## Совместимость

### Требования к системе

- Linux с поддержкой RAPL (для Intel процессоров)
- Ядро Linux с поддержкой `/proc/[pid]/power/energy_uj` (экспериментальный интерфейс)
- eBPF поддержка для расширенного мониторинга

### Зависимости

- `tokio`: Для асинхронных операций
- `serde`: Для сериализации/десериализации
- `tracing`: Для логирования
- `anyhow`: Для обработки ошибок

## Ссылки

- [RAPL Documentation](https://www.kernel.org/doc/html/latest/power/powercap/powercap.html)
- [eBPF Documentation](https://ebpf.io/)
- [Linux Power Management](https://www.kernel.org/doc/html/latest/power/index.html)
