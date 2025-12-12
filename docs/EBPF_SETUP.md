# Настройка eBPF для SmoothTask

## Введение

eBPF (extended Berkeley Packet Filter) - это революционная технология, которая позволяет безопасно выполнять пользовательский код в ядре Linux без изменения исходного кода ядра или загрузки модулей ядра. SmoothTask использует eBPF для высокопроизводительного сбора системных метрик с минимальными накладными расходами.

## Требования к системе

Для работы eBPF метрик в SmoothTask требуются следующие системные зависимости:

### 1. Зависимости для сборки

#### Ubuntu/Debian
```bash
sudo apt-get update
sudo apt-get install -y \
    libelf-dev \
    libglib2.0-dev \
    pkg-config \
    clang \
    llvm \
    linux-headers-$(uname -r)
```

#### Fedora/RHEL
```bash
sudo dnf install -y \
    elfutils-libelf-devel \
    glib2-devel \
    pkg-config \
    clang \
    llvm \
    kernel-headers
```

#### Arch Linux
```bash
sudo pacman -S \
    libelf \
    glib2 \
    pkgconf \
    clang \
    llvm \
    linux-headers
```

### 2. Требования к ядру

- **Минимальная версия**: Linux ядро 4.4+ (базовая поддержка eBPF)
- **Рекомендуемая версия**: Linux ядро 5.4+ для расширенных возможностей eBPF
- **Необходимые права**: Права для загрузки eBPF программ (CAP_BPF или root)
- **Конфигурация ядра**: Должны быть включены следующие опции:
  - `CONFIG_BPF=y`
  - `CONFIG_BPF_SYSCALL=y`
  - `CONFIG_BPF_JIT=y`
  - `CONFIG_HAVE_EBPF_JIT=y`
  - `CONFIG_BPF_EVENTS=y`

### 3. Включение eBPF поддержки в SmoothTask

Добавьте feature `ebpf` при сборке:

```bash
cargo build --features ebpf
```

Или в конфигурации:

```toml
[features]
default = ["libnotify", "ebpf"]
```

## Архитектура eBPF модуля

### Основные компоненты

1. **EbpfConfig** - Конфигурация eBPF метрик:
   - `enable_cpu_metrics`: Включение сбора CPU метрик через eBPF
   - `enable_memory_metrics`: Включение сбора метрик памяти через eBPF
   - `enable_syscall_monitoring`: Включение мониторинга системных вызовов
   - `enable_network_monitoring`: Включение мониторинга сетевой активности
   - `collection_interval`: Интервал сбора метрик
   - `enable_caching`: Включение кэширования для уменьшения накладных расходов
   - `batch_size`: Размер batches для пакетной обработки
   - `max_init_attempts`: Максимальное количество попыток инициализации
   - `operation_timeout_ms`: Таймаут для операций eBPF в миллисекундах

2. **EbpfMetrics** - Структура для хранения метрик:
   - `cpu_usage`: Использование CPU в процентах
   - `memory_usage`: Использование памяти в байтах
   - `syscall_count`: Количество системных вызовов
   - `network_packets`: Количество сетевых пакетов
   - `network_bytes`: Сетевой трафик в байтах
   - `timestamp`: Временная метка в наносекундах
   - `syscall_details`: Детализированная статистика по системным вызовам (опционально)
   - `network_details`: Детализированная статистика по сетевой активности (опционально)

3. **EbpfMetricsCollector** - Основной коллектор:
   - `initialize()`: Инициализация eBPF программ с улучшенной обработкой ошибок
   - `collect_metrics()`: Сбор текущих метрик с поддержкой кэширования
   - `check_ebpf_support()`: Проверка поддержки eBPF в системе
   - `load_cpu_program()`: Загрузка eBPF программы для CPU метрик
   - `load_syscall_program()`: Загрузка eBPF программы для мониторинга системных вызовов
   - `load_network_program()`: Загрузка eBPF программы для мониторинга сетевой активности
   - `validate_config()`: Валидация конфигурации с проверкой корректности параметров
   - `get_last_error()`: Получение последней ошибки инициализации
   - `is_initialized()`: Проверка состояния инициализации
   - `reset()`: Сброс состояния коллектора
   - `get_initialization_stats()`: Получение статистики инициализации

### Новые возможности eBPF

В последних версиях SmoothTask добавлены расширенные возможности eBPF:

1. **Расширенный мониторинг системных вызовов**: Детализированная статистика по каждому системному вызову с информацией о времени выполнения
2. **Мониторинг сетевой активности**: Отслеживание сетевых пакетов и соединений с поддержкой статистики по IP адресам
3. **Улучшенная обработка ошибок**: Graceful degradation при отсутствии eBPF поддержки
4. **Кэширование метрик**: Оптимизация производительности через пакетную обработку
5. **Валидация конфигурации**: Проверка корректности параметров перед инициализацией

### eBPF программы

В настоящее время реализованы:

1. **cpu_metrics.c** - Сбор метрик CPU:
   - Отслеживание времени выполнения процессов
   - Мониторинг использования CPU
   - Сбор статистики по ядрам
   - Использует точку входа `kprobe/run_local_timer`

2. **cpu_metrics_optimized.c** - Оптимизированная версия CPU метрик:
   - Использует более эффективную точку трассировки `tracepoint/sched/sched_process_exec`
   - Минимальные операции обновления для уменьшения накладных расходов
   - Атомарные операции для минимизации конфликтов

3. **syscall_monitor.c** - Базовый мониторинг системных вызовов:
   - Отслеживание всех системных вызовов через `tracepoint/syscalls/sys_enter_*`
   - Счетчик системных вызовов с временными метками
   - Поддержка анализа активности процессов

4. **syscall_monitor_optimized.c** - Оптимизированная версия мониторинга системных вызовов:
   - Использует более специфичную точку трассировки `tracepoint/syscalls/sys_enter_execve`
   - Атомарное увеличение счетчика для минимизации конфликтов
   - Уменьшенная нагрузка на систему

5. **syscall_monitor_advanced.c** - Расширенный мониторинг системных вызовов:
   - Детализированная статистика по каждому системному вызову
   - Отслеживание времени выполнения системных вызовов
   - Поддержка анализа производительности системных вызовов
   - Использует точки входа `tracepoint/syscalls/sys_enter_*` и `tracepoint/syscalls/sys_exit_*`
   - Хранит статистику в хэш-карте для быстрого доступа
   - Поддержка до 256 различных системных вызовов
   - Хранение количества вызовов, общего времени выполнения и среднего времени

6. **network_monitor.c** - Мониторинг сетевой активности:
   - Отслеживание сетевых пакетов через `tracepoint/net/netif_receive_skb`
   - Мониторинг TCP соединений через `tracepoint/sock/sock_inet_sock_set_state`
   - Сбор статистики по IP адресам
   - Поддержка анализа сетевого трафика
   - Хранение статистики по отправленным/полученным пакетам и байтам
   - Поддержка до 1024 различных сетевых соединений

### Интеграция с системой

```rust
use smoothtask_core::metrics::ebpf::{EbpfConfig, EbpfMetricsCollector};

// Создание конфигурации
let config = EbpfConfig {
    enable_cpu_metrics: true,
    enable_syscall_monitoring: true,
    enable_caching: true,
    batch_size: 50,
    ..Default::default()
};

// Создание коллектора
let mut collector = EbpfMetricsCollector::new(config);

// Инициализация
if let Err(e) = collector.initialize() {
    eprintln!("Не удалось инициализировать eBPF: {}", e);
    // Graceful degradation - продолжаем работу без eBPF
}

// Сбор метрик
if let Ok(metrics) = collector.collect_metrics() {
    println!("CPU usage: {:.2}%", metrics.cpu_usage);
    println!("Syscalls: {}", metrics.syscall_count);
}
```

## Расширенный мониторинг системных вызовов

### Возможности расширенного мониторинга

Расширенный мониторинг системных вызовов предоставляет детализированную информацию о каждом системном вызове:

- **Количество вызовов**: Сколько раз был вызван каждый системный вызов
- **Общее время выполнения**: Суммарное время выполнения всех вызовов в наносекундах
- **Среднее время выполнения**: Среднее время выполнения одного вызова
- **Поддержка до 256 различных системных вызовов**: Одновременное отслеживание множества системных вызовов

### Пример использования

```rust
let config = EbpfConfig {
    enable_syscall_monitoring: true,
    ..Default::default()
};

let mut collector = EbpfMetricsCollector::new(config);
collector.initialize()?;

let metrics = collector.collect_metrics()?;
println!("Количество системных вызовов: {}", metrics.syscall_count);

// Доступ к детализированной статистике по системным вызовам
if let Some(syscall_details) = metrics.syscall_details {
    for detail in syscall_details {
        println!("Системный вызов {}: {} вызовов, среднее время: {} нс",
                 detail.syscall_id, detail.count, detail.avg_time_ns);
    }
}
```

### Анализ производительности

Расширенный мониторинг системных вызовов позволяет анализировать производительность приложений:

- **Выявление узких мест**: Определение системных вызовов, которые занимают больше всего времени
- **Анализ активности**: Понимание, какие системные вызовы наиболее часто используются
- **Оптимизация приложений**: Выявление возможностей для оптимизации приложений

## Мониторинг сетевой активности

### Возможности мониторинга сети

Мониторинг сетевой активности предоставляет информацию о сетевом трафике:

- **Количество пакетов**: Общее количество отправленных и полученных пакетов
- **Сетевой трафик**: Общее количество отправленных и полученных байт
- **Статистика по IP адресам**: Детализированная информация по каждому IP адресу
- **Поддержка до 1024 соединений**: Одновременное отслеживание множества сетевых соединений

### Пример использования

```rust
let config = EbpfConfig {
    enable_network_monitoring: true,
    ..Default::default()
};

let mut collector = EbpfMetricsCollector::new(config);
collector.initialize()?;

let metrics = collector.collect_metrics()?;
println!("Сетевые пакеты: {}", metrics.network_packets);
println!("Сетевой трафик: {} байт", metrics.network_bytes);

// Доступ к детализированной статистике по IP адресам
if let Some(network_details) = metrics.network_details {
    for detail in network_details {
        println!("IP: {:x}, отправлено: {} пакетов, получено: {} пакетов",
                 detail.ip_address, detail.packets_sent, detail.packets_received);
    }
}
```

### Анализ сетевого трафика

Мониторинг сетевой активности позволяет анализировать сетевой трафик:

- **Анализ нагрузки**: Понимание распределения сетевого трафика
- **Выявление аномалий**: Обнаружение необычной сетевой активности
- **Оптимизация сети**: Выявление возможностей для оптимизации сетевых настроек

## Расширенная настройка

### Оптимизация производительности

Для оптимизации производительности eBPF программ:

1. **Кэширование**: Включите кэширование для уменьшения накладных расходов
   ```rust
   let config = EbpfConfig {
       enable_caching: true,
       batch_size: 100,  // Увеличьте для менее частых обновлений
       ..Default::default()
   };
   ```

2. **Выборочный мониторинг**: Отключите ненужные метрики
   ```rust
   let config = EbpfConfig {
       enable_cpu_metrics: true,
       enable_memory_metrics: false,  // Отключите, если не нужно
       enable_syscall_monitoring: false,  // Отключите, если не нужно
       ..Default::default()
   };
   ```

### Проверка поддержки eBPF

```rust
use smoothtask_core::metrics::ebpf::EbpfMetricsCollector;

// Проверка поддержки eBPF в системе
let supported = EbpfMetricsCollector::check_ebpf_support();
match supported {
    Ok(true) => println!("eBPF поддерживается в этой системе"),
    Ok(false) => println!("eBPF не поддерживается в этой системе"),
    Err(e) => println!("Ошибка проверки поддержки eBPF: {}", e),
}
```

## Устранение неполадок

### Ошибка: "Package libelf was not found"

Установите библиотеку libelf:

```bash
# Ubuntu/Debian
sudo apt-get install libelf-dev

# Fedora/RHEL
sudo dnf install elfutils-libelf-devel

# Arch Linux
sudo pacman -S libelf
```

### Ошибка: "eBPF не поддерживается в этой системе"

1. Проверьте версию ядра:
   ```bash
   uname -r
   ```

2. Убедитесь, что ядро поддерживает eBPF (версия 4.4+)

3. Проверьте наличие необходимых файлов:
   ```bash
   ls /sys/kernel/debug/tracing/available_filter_functions
   ls /proc/kallsyms
   ```

4. Проверьте конфигурацию ядра:
   ```bash
   grep CONFIG_BPF /boot/config-$(uname -r)
   ```

### Ошибка: "Недостаточно прав для загрузки eBPF программ"

Запустите SmoothTask с повышенными привилегиями:

```bash
sudo smoothtaskd
```

Или настройте capabilities:

```bash
sudo setcap cap_bpf+ep /path/to/smoothtaskd
```

### Ошибка: "eBPF программа не найдена"

Убедитесь, что eBPF программы находятся в правильном расположении:

```bash
ls smoothtask-core/src/ebpf_programs/
```

Программы должны быть доступны во время выполнения.

### Ошибка: "Ошибка загрузки программы мониторинга сети"

1. Проверьте, что файл `network_monitor.c` существует:
   ```bash
   ls smoothtask-core/src/ebpf_programs/network_monitor.c
   ```

2. Убедитесь, что у вас достаточно прав для загрузки eBPF программ:
   ```bash
   sudo setcap cap_bpf+ep /path/to/smoothtaskd
   ```

3. Проверьте, что ваше ядро поддерживает сетевые точки трассировки:
   ```bash
   grep -r "netif_receive_skb" /sys/kernel/debug/tracing/available_filter_functions
   ```

### Ошибка: "Ошибка загрузки программы мониторинга системных вызовов"

1. Проверьте, что файл `syscall_monitor_advanced.c` существует:
   ```bash
   ls smoothtask-core/src/ebpf_programs/syscall_monitor_advanced.c
   ```

2. Убедитесь, что у вас достаточно прав для загрузки eBPF программ:
   ```bash
   sudo setcap cap_bpf+ep /path/to/smoothtaskd
   ```

3. Проверьте, что ваше ядро поддерживает точки трассировки системных вызовов:
   ```bash
   grep -r "sys_enter" /sys/kernel/debug/tracing/available_filter_functions
   ```

4. Проверьте, что у вас установлены все необходимые зависимости для компиляции eBPF программ:
   ```bash
   sudo apt-get install clang llvm linux-headers-$(uname -r)
   ```

### Ошибка: "Ошибка загрузки программы мониторинга сети"

1. Проверьте, что файл `network_monitor.c` существует:
   ```bash
   ls smoothtask-core/src/ebpf_programs/network_monitor.c
   ```

2. Убедитесь, что у вас достаточно прав для загрузки eBPF программ:
   ```bash
   sudo setcap cap_bpf+ep /path/to/smoothtaskd
   ```

3. Проверьте, что ваше ядро поддерживает сетевые точки трассировки:
   ```bash
   grep -r "netif_receive_skb" /sys/kernel/debug/tracing/available_filter_functions
   ```

4. Проверьте, что у вас установлены все необходимые зависимости для компиляции eBPF программ:
   ```bash
   sudo apt-get install clang llvm linux-headers-$(uname -r)
   ```

### Ошибка: "Недостаточно памяти для eBPF карт"

1. Увеличьте размер eBPF карт в eBPF программах
2. Уменьшите количество отслеживаемых системных вызовов или соединений
3. Проверьте, что у вас достаточно памяти в системе

### Ошибка: "Не поддерживаемая версия ядра"

1. Проверьте версию вашего ядра:
   ```bash
   uname -r
   ```

2. Убедитесь, что ваше ядро поддерживает eBPF (версия 4.4+ для базовой поддержки, 5.4+ для расширенных возможностей)

3. Обновите ядро, если необходимо:
   ```bash
   sudo apt-get update && sudo apt-get upgrade
   ```

## Тестирование eBPF функциональности

### Unit тесты

```bash
# Запуск тестов с включенной eBPF поддержкой
cargo test --features ebpf
```

### Интеграционные тесты

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ebpf_config_default() {
        let config = EbpfConfig::default();
        assert!(config.enable_cpu_metrics);
        assert!(config.enable_memory_metrics);
        assert!(!config.enable_syscall_monitoring);
        assert!(!config.enable_network_monitoring);
    }

    #[test]
    fn test_ebpf_collector_creation() {
        let config = EbpfConfig::default();
        let mut collector = EbpfMetricsCollector::new(config);
        assert!(collector.initialize().is_ok());
        assert!(collector.collect_metrics().is_ok());
    }

    #[test]
    fn test_ebpf_network_monitoring() {
        let mut config = EbpfConfig::default();
        config.enable_network_monitoring = true;
        
        let mut collector = EbpfMetricsCollector::new(config);
        assert!(collector.initialize().is_ok());
        
        let metrics = collector.collect_metrics().unwrap();
        assert!(metrics.network_packets >= 0);
        assert!(metrics.network_bytes >= 0);
    }

    #[test]
    fn test_ebpf_config_validation() {
        let mut config = EbpfConfig::default();
        let mut collector = EbpfMetricsCollector::new(config.clone());
        
        // Корректная конфигурация должна проходить валидацию
        assert!(collector.validate_config().is_ok());
        
        // Тестируем некорректные конфигурации
        config.batch_size = 0;
        let mut collector = EbpfMetricsCollector::new(config.clone());
        assert!(collector.validate_config().is_err());
    }
}
```

### Бенчмаркинг

Для оценки производительности eBPF программ:

```bash
# Запуск бенчмарков
cargo bench --features ebpf
```

## Будущие улучшения

- **Расширенные метрики**: Добавление поддержки метрик диска, метрик GPU
- **Улучшенная интеграция**: Более тесная интеграция с основной системой метрик
- **Динамическая загрузка**: Поддержка динамической загрузки и выгрузки eBPF программ
- **Безопасность**: Улучшенная проверка безопасности eBPF программ
- **Мониторинг**: Расширенный мониторинг и логирование eBPF операций
- **Производительность**: Оптимизация существующих eBPF программ для снижения накладных расходов
- **Совместимость**: Тестирование и поддержка различных версий ядра Linux
- **Документация**: Расширение примеров использования и руководств по устранению неполадок

## Интеграция с основной системой метрик

### Интеграция eBPF метрик в основную систему

eBPF метрики могут быть интегрированы в основную систему метрик SmoothTask:

```rust
use smoothtask_core::metrics::ebpf::{EbpfConfig, EbpfMetricsCollector};
use smoothtask_core::metrics::system::SystemMetricsCollector;

// Создание eBPF коллектора
let ebpf_config = EbpfConfig {
    enable_cpu_metrics: true,
    enable_syscall_monitoring: true,
    enable_network_monitoring: true,
    ..Default::default()
};

let mut ebpf_collector = EbpfMetricsCollector::new(ebpf_config);
ebpf_collector.initialize()?;

// Создание системного коллектора
let mut system_collector = SystemMetricsCollector::new();

// Сбор метрик из обоих источников
let ebpf_metrics = ebpf_collector.collect_metrics()?;
let system_metrics = system_collector.collect_metrics()?;

// Комбинирование метрик
println!("Системные метрики:");
println!("  CPU загрузка: {:.2}%", system_metrics.cpu_usage);
println!("  Использование памяти: {} MB", system_metrics.memory_usage / 1024 / 1024);

println!("eBPF метрики:");
println!("  Системные вызовы: {}", ebpf_metrics.syscall_count);
println!("  Сетевые пакеты: {}", ebpf_metrics.network_packets);
```

### Использование в основном цикле SmoothTask

```rust
use smoothtask_core::metrics::ebpf::{EbpfConfig, EbpfMetricsCollector};
use std::time::Duration;

// Настройка eBPF коллектора
let ebpf_config = EbpfConfig {
    enable_cpu_metrics: true,
    enable_syscall_monitoring: true,
    enable_network_monitoring: true,
    collection_interval: Duration::from_secs(1),
    enable_caching: true,
    batch_size: 50,
    ..Default::default()
};

let mut ebpf_collector = EbpfMetricsCollector::new(ebpf_config);

// Основной цикл сбора метрик
loop {
    // Сбор eBPF метрик
    match ebpf_collector.collect_metrics() {
        Ok(metrics) => {
            // Обработка метрик
            println!("Собраны eBPF метрики: {:?}", metrics);
            
            // Интеграция с основной системой
            // ... обработка и анализ метрик
        }
        Err(e) => {
            eprintln!("Ошибка сбора eBPF метрик: {}", e);
            // Graceful degradation - продолжаем работу
        }
    }
    
    // Задержка перед следующим сбором
    std::thread::sleep(Duration::from_secs(1));
}
```

### Примеры использования

### Мониторинг системных вызовов

```rust
let config = EbpfConfig {
    enable_syscall_monitoring: true,
    ..Default::default()
};

let mut collector = EbpfMetricsCollector::new(config);
collector.initialize()?;

let metrics = collector.collect_metrics()?;
println!("Количество системных вызовов: {}", metrics.syscall_count);
```

### Мониторинг сетевой активности

```rust
let config = EbpfConfig {
    enable_network_monitoring: true,
    ..Default::default()
};

let mut collector = EbpfMetricsCollector::new(config);
collector.initialize()?;

let metrics = collector.collect_metrics()?;
println!("Сетевые пакеты: {}", metrics.network_packets);
println!("Сетевой трафик: {} байт", metrics.network_bytes);

// Доступ к детализированной статистике по IP адресам
if let Some(network_details) = metrics.network_details {
    for detail in network_details {
        println!("IP: {:x}, отправлено: {} пакетов, получено: {} пакетов",
                 detail.ip_address, detail.packets_sent, detail.packets_received);
    }
}
```

### Расширенный мониторинг системных вызовов

```rust
let config = EbpfConfig {
    enable_syscall_monitoring: true,
    ..Default::default()
};

let mut collector = EbpfMetricsCollector::new(config);
collector.initialize()?;

let metrics = collector.collect_metrics()?;
println!("Количество системных вызовов: {}", metrics.syscall_count);

// Доступ к детализированной статистике по системным вызовам
if let Some(syscall_details) = metrics.syscall_details {
    for detail in syscall_details {
        println!("Системный вызов {}: {} вызовов, среднее время: {} нс",
                 detail.syscall_id, detail.count, detail.avg_time_ns);
    }
}
```

### Оптимизированный сбор метрик

```rust
let config = EbpfConfig {
    enable_caching: true,
    batch_size: 200,
    collection_interval: Duration::from_secs(2),
    ..Default::default()
};

let mut collector = EbpfMetricsCollector::new(config);
collector.initialize()?;

// Метрики будут кэшироваться и обновляться пакетами
let metrics = collector.collect_metrics()?;
```

## Ссылки

- [libbpf-rs документация](https://docs.rs/libbpf-rs)
- [eBPF официальная документация](https://ebpf.io/)
- [Linux eBPF документация](https://www.kernel.org/doc/html/latest/bpf/)
- [BPF Performance Tools](https://github.com/brendangregg/bpf-perf-tools-book)
- [Cilium eBPF Documentation](https://docs.cilium.io/en/stable/bpf/)