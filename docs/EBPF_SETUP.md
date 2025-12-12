# Настройка eBPF для SmoothTask

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

- Linux ядро версии 4.4+ (базовая поддержка eBPF)
- Рекомендуется ядро 5.4+ для расширенных возможностей eBPF
- Необходимы права для загрузки eBPF программ (CAP_BPF или root)

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
   - `enable_cpu_metrics`: Включение сбора CPU метрик
   - `enable_memory_metrics`: Включение сбора метрик памяти
   - `enable_syscall_monitoring`: Включение мониторинга системных вызовов
   - `collection_interval`: Интервал сбора метрик

2. **EbpfMetrics** - Структура для хранения метрик:
   - `cpu_usage`: Использование CPU в процентах
   - `memory_usage`: Использование памяти в байтах
   - `syscall_count`: Количество системных вызовов
   - `timestamp`: Временная метка

3. **EbpfMetricsCollector** - Основной коллектор:
   - `initialize()`: Инициализация eBPF программ
   - `collect_metrics()`: Сбор текущих метрик
   - `check_ebpf_support()`: Проверка поддержки eBPF в системе

### eBPF программы

В настоящее время реализованы:

1. **cpu_metrics.c** - Сбор метрик CPU:
   - Отслеживание времени выполнения процессов
   - Мониторинг использования CPU
   - Сбор статистики по ядрам

### Интеграция с системой

```rust
use smoothtask_core::metrics::ebpf::{EbpfConfig, EbpfMetricsCollector};

let config = EbpfConfig::default();
let mut collector = EbpfMetricsCollector::new(config);

// Инициализация
collector.initialize()?;

// Сбор метрик
let metrics = collector.collect_metrics()?;
println!("CPU usage: {}%", metrics.cpu_usage);
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

### Ошибка: "Недостаточно прав для загрузки eBPF программ"

Запустите SmoothTask с повышенными привилегиями:

```bash
sudo smoothtaskd
```

Или настройте capabilities:

```bash
sudo setcap cap_bpf+ep /path/to/smoothtaskd
```

## Будущие улучшения

- Интеграция с основной системой метрик
- Добавление мониторинга системных вызовов
- Оптимизация производительности eBPF программ
- Расширенная поддержка метрик (сеть, диск, и т.д.)
- Улучшенная обработка ошибок и логирование

## Ссылки

- [libbpf-rs документация](https://docs.rs/libbpf-rs)
- [eBPF официальная документация](https://ebpf.io/)
- [Linux eBPF документация](https://www.kernel.org/doc/html/latest/bpf/)