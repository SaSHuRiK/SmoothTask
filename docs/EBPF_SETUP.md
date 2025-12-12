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
   - `collection_interval`: Интервал сбора метрик
   - `enable_caching`: Включение кэширования для уменьшения накладных расходов
   - `batch_size`: Размер batches для пакетной обработки

2. **EbpfMetrics** - Структура для хранения метрик:
   - `cpu_usage`: Использование CPU в процентах
   - `memory_usage`: Использование памяти в байтах
   - `syscall_count`: Количество системных вызовов
   - `timestamp`: Временная метка в наносекундах

3. **EbpfMetricsCollector** - Основной коллектор:
   - `initialize()`: Инициализация eBPF программ
   - `collect_metrics()`: Сбор текущих метрик
   - `check_ebpf_support()`: Проверка поддержки eBPF в системе
   - `load_cpu_program()`: Загрузка eBPF программы для CPU метрик
   - `load_syscall_program()`: Загрузка eBPF программы для мониторинга системных вызовов

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

3. **syscall_monitor.c** - Мониторинг системных вызовов:
   - Отслеживание всех системных вызовов через `tracepoint/syscalls/sys_enter_*`
   - Счетчик системных вызовов с временными метками
   - Поддержка анализа активности процессов

4. **syscall_monitor_optimized.c** - Оптимизированная версия мониторинга системных вызовов:
   - Использует более специфичную точку трассировки `tracepoint/syscalls/sys_enter_execve`
   - Атомарное увеличение счетчика для минимизации конфликтов
   - Уменьшенная нагрузка на систему

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
    }

    #[test]
    fn test_ebpf_collector_creation() {
        let config = EbpfConfig::default();
        let mut collector = EbpfMetricsCollector::new(config);
        assert!(collector.initialize().is_ok());
        assert!(collector.collect_metrics().is_ok());
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

- **Расширенные метрики**: Добавление поддержки сетевых метрик, метрик диска
- **Улучшенная интеграция**: Более тесная интеграция с основной системой метрик
- **Динамическая загрузка**: Поддержка динамической загрузки и выгрузки eBPF программ
- **Безопасность**: Улучшенная проверка безопасности eBPF программ
- **Мониторинг**: Расширенный мониторинг и логирование eBPF операций

## Примеры использования

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