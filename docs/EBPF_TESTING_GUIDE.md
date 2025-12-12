# Руководство по тестированию eBPF функциональности SmoothTask

## Введение

Это руководство описывает комплексный подход к тестированию eBPF функциональности SmoothTask на реальных системах. eBPF (extended Berkeley Packet Filter) является критически важным компонентом для высокопроизводительного сбора системных метрик с минимальными накладными расходами.

## 1. Анализ текущего состояния

### 1.1 Текущая реализация

Анализ текущей реализации eBPF в SmoothTask выявил следующие ключевые моменты:

**Структура модуля:**
- `EbpfConfig`: Конфигурация с поддержкой CPU, памяти, системных вызовов, сети, GPU и файловой системы
- `EbpfMetrics`: Структура для хранения метрик с детализированной статистикой
- `EbpfMetricsCollector`: Основной коллектор с поддержкой кэширования и обработки ошибок

**eBPF программы:**
- `cpu_metrics.c`: Базовый мониторинг CPU через kprobe
- `cpu_metrics_optimized.c`: Оптимизированная версия с tracepoint
- `syscall_monitor.c`: Базовый мониторинг системных вызовов
- `syscall_monitor_optimized.c`: Оптимизированная версия
- `syscall_monitor_advanced.c`: Расширенный мониторинг с детализированной статистикой
- `network_monitor.c`: Мониторинг сетевой активности
- `gpu_monitor.c`: Мониторинг GPU (базовая версия)
- `gpu_monitor_optimized.c`: Оптимизированная версия GPU мониторинга
- `gpu_monitor_high_perf.c`: Высокопроизводительная версия GPU мониторинга
- `filesystem_monitor.c`: Мониторинг файловой системы
- `filesystem_monitor_optimized.c`: Оптимизированная версия
- `filesystem_monitor_high_perf.c`: Высокопроизводительная версия

**Текущие ограничения:**
- eBPF программы существуют, но реальная загрузка и выполнение не реализованы
- Используются заглушки (stubs) для тестирования структуры
- Нет реального сбора данных из eBPF карт
- Отсутствует интеграция с libbpf-rs для загрузки и управления программами

### 1.2 Требования к тестированию

Для обеспечения стабильности и производительности eBPF функциональности необходимо:

1. **Реализация реальной eBPF функциональности:**
   - Загрузка и выполнение eBPF программ
   - Сбор данных из eBPF карт
   - Интеграция с libbpf-rs

2. **Тестирование на различных конфигурациях:**
   - Разные версии ядра Linux (4.4+, 5.4+, 6.x)
   - Разные дистрибутивы (Ubuntu, Debian, Fedora, Arch, RHEL)
   - Разные архитектуры (x86_64, ARM64)

3. **Производительность и стабильность:**
   - Измерение накладных расходов
   - Тестирование долговременной стабильности
   - Оценка влияния на систему

4. **Обработка ошибок и graceful degradation:**
   - Тестирование сценариев без eBPF поддержки
   - Проверка обработки ошибок загрузки программ
   - Валидация graceful degradation

## 2. План тестирования eBPF функциональности

### 2.1 Фаза 1: Реализация реальной eBPF функциональности

**Цель:** Реализовать реальную загрузку и выполнение eBPF программ

**Задачи:**
- [ ] Интеграция с libbpf-rs для загрузки eBPF программ
- [ ] Реализация сбора данных из eBPF карт
- [ ] Интеграция с существующей структурой метрик
- [ ] Обработка ошибок и graceful degradation

**Ожидаемые результаты:**
- Реальная загрузка eBPF программ из файлов
- Сбор реальных метрик из eBPF
- Интеграция с существующими тестами

### 2.2 Фаза 2: Тестирование на различных конфигурациях ядра

**Цель:** Обеспечить совместимость с различными версиями ядра Linux

**Тестовые конфигурации:**

| Версия ядра | Дистрибутив | Архитектура | Ожидаемая поддержка |
|-------------|-------------|-------------|---------------------|
| 4.4.x | Ubuntu 16.04 | x86_64 | Базовая поддержка |
| 4.19.x | Debian 10 | x86_64 | Базовая поддержка |
| 5.4.x | Ubuntu 20.04 | x86_64 | Полная поддержка |
| 5.10.x | Fedora 33 | x86_64 | Полная поддержка |
| 5.15.x | Arch Linux | x86_64 | Полная поддержка |
| 6.0.x | Ubuntu 22.04 | x86_64 | Полная поддержка |
| 6.2.x | Fedora 37 | x86_64 | Полная поддержка |
| 6.5.x | Arch Linux | x86_64 | Полная поддержка |
| 6.5.x | Ubuntu 23.04 | ARM64 | Полная поддержка |

**Тестовые сценарии:**

1. **Проверка поддержки eBPF:**
   ```bash
   # Проверка версии ядра
   uname -r
   
   # Проверка доступности eBPF
   ls /sys/kernel/debug/tracing/available_filter_functions
   
   # Проверка конфигурации ядра
   grep CONFIG_BPF /boot/config-$(uname -r)
   ```

2. **Тестирование загрузки eBPF программ:**
   ```bash
   # Тестирование загрузки CPU программы
   sudo ./smoothtaskd --test-ebpf-cpu
   
   # Тестирование загрузки программы системных вызовов
   sudo ./smoothtaskd --test-ebpf-syscall
   
   # Тестирование загрузки сетевой программы
   sudo ./smoothtaskd --test-ebpf-network
   ```

3. **Тестирование сбора метрик:**
   ```bash
   # Тестирование сбора CPU метрик
   sudo ./smoothtaskd --test-ebpf-metrics --type cpu
   
   # Тестирование сбора метрик системных вызовов
   sudo ./smoothtaskd --test-ebpf-metrics --type syscall
   
   # Тестирование сбора сетевых метрик
   sudo ./smoothtaskd --test-ebpf-metrics --type network
   ```

### 2.3 Фаза 3: Тестирование совместимости с дистрибутивами

**Цель:** Обеспечить совместимость с различными дистрибутивами Linux

**Тестовые дистрибутивы:**

1. **Ubuntu/Debian:**
   - Проверка зависимостей: `libelf-dev`, `libglib2.0-dev`, `clang`, `llvm`
   - Тестирование сборки: `cargo build --features ebpf`
   - Тестирование выполнения: `sudo ./smoothtaskd --test-ebpf`

2. **Fedora/RHEL:**
   - Проверка зависимостей: `elfutils-libelf-devel`, `glib2-devel`, `clang`, `llvm`
   - Тестирование сборки: `cargo build --features ebpf`
   - Тестирование выполнения: `sudo ./smoothtaskd --test-ebpf`

3. **Arch Linux:**
   - Проверка зависимостей: `libelf`, `glib2`, `clang`, `llvm`
   - Тестирование сборки: `cargo build --features ebpf`
   - Тестирование выполнения: `sudo ./smoothtaskd --test-ebpf`

**Тестовые сценарии:**

1. **Проверка зависимостей:**
   ```bash
   # Ubuntu/Debian
   sudo apt-get install libelf-dev libglib2.0-dev clang llvm linux-headers-$(uname -r)
   
   # Fedora/RHEL
   sudo dnf install elfutils-libelf-devel glib2-devel clang llvm kernel-headers
   
   # Arch Linux
   sudo pacman -S libelf glib2 clang llvm linux-headers
   ```

2. **Тестирование сборки:**
   ```bash
   # Сборка с eBPF поддержкой
   cargo build --features ebpf
   
   # Проверка успешной сборки
   ls target/debug/smoothtaskd
   ```

3. **Тестирование выполнения:**
   ```bash
   # Тестирование базовой функциональности
   sudo ./smoothtaskd --test-ebpf-basic
   
   # Тестирование расширенной функциональности
   sudo ./smoothtaskd --test-ebpf-advanced
   ```

### 2.4 Фаза 4: Сбор метрик производительности и стабильности

**Цель:** Оценить производительность и стабильность eBPF функциональности

**Метрики производительности:**

1. **Накладные расходы:**
   - Время инициализации eBPF программ
   - Время сбора метрик
   - Использование CPU при сборе метрик
   - Использование памяти при сборе метрик

2. **Точность метрик:**
   - Сравнение eBPF метрик с традиционными методами
   - Оценка точности измерений
   - Сравнение с системными инструментами (top, vmstat, etc.)

3. **Стабильность:**
   - Долговременное тестирование (24+ часа)
   - Тестирование под нагрузкой
   - Тестирование в условиях ограниченных ресурсов

**Тестовые сценарии:**

1. **Бенчмаркинг производительности:**
   ```bash
   # Запуск бенчмарков
   cargo bench --features ebpf
   
   # Измерение времени инициализации
   cargo bench --features ebpf --bench ebpf_initialization
   
   # Измерение времени сбора метрик
   cargo bench --features ebpf --bench ebpf_metrics_collection
   ```

2. **Сравнение с традиционными методами:**
   ```bash
   # Сбор метрик через eBPF
   sudo ./smoothtaskd --test-ebpf-metrics --output ebpf_metrics.json
   
   # Сбор метрик через традиционные методы
   ./smoothtaskd --test-traditional-metrics --output traditional_metrics.json
   
   # Сравнение результатов
   python3 compare_metrics.py ebpf_metrics.json traditional_metrics.json
   ```

3. **Долговременное тестирование:**
   ```bash
   # Запуск долговременного теста
   sudo ./smoothtaskd --test-ebpf-long-term --duration 24h --output long_term_results.json
   
   # Анализ результатов
   python3 analyze_long_term.py long_term_results.json
   ```

### 2.5 Фаза 5: Документирование результатов и ограничений

**Цель:** Документировать результаты тестирования и известные ограничения

**Документация:**

1. **Отчет о тестировании:**
   - Результаты тестирования на различных конфигурациях
   - Производительность и стабильность
   - Известные проблемы и ограничения

2. **Руководство по развертыванию:**
   - Требования к системе
   - Инструкции по установке
   - Рекомендации по конфигурации

3. **Руководство по устранению неполадок:**
   - Распространенные проблемы
   - Методы диагностики
   - Решения и обходные пути

## 3. Реализация реальной eBPF функциональности

### 3.1 Интеграция с libbpf-rs

**Требования:**
- Добавление зависимости libbpf-rs в Cargo.toml
- Интеграция с существующей структурой
- Обработка ошибок и graceful degradation

**Реализация:**

```rust
// В Cargo.toml
[dependencies]
libbpf-rs = "0.20"

// В ebpf.rs
use libbpf_rs::{Program, Skel, SkelBuilder};

/// Загрузка eBPF программы с использованием libbpf-rs
fn load_ebpf_program(program_path: &str) -> Result<Program> {
    // Создание билдера
    let mut skel_builder = SkelBuilder::default();
    
    // Загрузка программы
    let open_skel = skel_builder.open(program_path)
        .context(format!("Не удалось открыть eBPF программу: {}", program_path))?;
    
    // Компиляция и загрузка
    let mut skel = open_skel.load()
        .context("Не удалось загрузить eBPF программу")?;
    
    // Прикрепление программы
    skel.attach()
        .context("Не удалось прикрепить eBPF программу")?;
    
    Ok(skel)
}
```

### 3.2 Сбор данных из eBPF карт

**Требования:**
- Доступ к eBPF картам
- Чтение данных из карт
- Преобразование данных в структуры Rust

**Реализация:**

```rust
/// Чтение данных из eBPF карты
fn read_ebpf_map(skel: &Skel, map_name: &str) -> Result<Vec<u8>> {
    // Получение карты
    let map = skel.map(map_name)
        .context(format!("Не удалось получить карту: {}", map_name))?;
    
    // Чтение данных
    let data = map.lookup(&0, 0)
        .context("Не удалось прочитать данные из карты")?;
    
    Ok(data)
}

/// Преобразование данных в структуру метрик
fn parse_ebpf_metrics(data: &[u8]) -> Result<EbpfMetrics> {
    // Парсинг данных
    // ... реализация парсинга
    
    Ok(EbpfMetrics {
        cpu_usage: parsed_cpu_usage,
        memory_usage: parsed_memory_usage,
        // ... другие поля
    })
}
```

### 3.3 Интеграция с существующей структурой

**Требования:**
- Интеграция с EbpfMetricsCollector
- Поддержка существующих тестов
- Обратная совместимость

**Реализация:**

```rust
impl EbpfMetricsCollector {
    /// Загрузка eBPF программы для CPU метрик
    #[cfg(feature = "ebpf")]
    fn load_cpu_program(&mut self) -> Result<()> {
        let program_path = "src/ebpf_programs/cpu_metrics.c";
        
        // Загрузка программы
        let program = load_ebpf_program(program_path)?;
        
        // Сохранение программы
        self.cpu_program = Some(program);
        
        Ok(())
    }
    
    /// Сбор метрик из eBPF программ
    fn collect_metrics(&mut self) -> Result<EbpfMetrics> {
        // Проверка инициализации
        if !self.initialized {
            return Ok(EbpfMetrics::default());
        }
        
        // Сбор метрик из eBPF программ
        let cpu_metrics = self.collect_cpu_metrics()?;
        let memory_metrics = self.collect_memory_metrics()?;
        let syscall_metrics = self.collect_syscall_metrics()?;
        
        // Объединение метрик
        Ok(EbpfMetrics {
            cpu_usage: cpu_metrics,
            memory_usage: memory_metrics,
            syscall_count: syscall_metrics,
            // ... другие поля
        })
    }
}
```

## 4. Тестовые сценарии для реальной eBPF функциональности

### 4.1 Базовые тесты

**Тест загрузки eBPF программ:**
```rust
#[test]
fn test_ebpf_program_loading() {
    let config = EbpfConfig::default();
    let mut collector = EbpfMetricsCollector::new(config);
    
    // Тестирование загрузки
    assert!(collector.initialize().is_ok());
    assert!(collector.is_initialized());
    
    // Проверка загруженных программ
    #[cfg(feature = "ebpf")]
    {
        assert!(collector.cpu_program.is_some());
        assert!(collector.memory_program.is_some());
    }
}
```

**Тест сбора метрик:**
```rust
#[test]
fn test_ebpf_metrics_collection() {
    let config = EbpfConfig::default();
    let mut collector = EbpfMetricsCollector::new(config);
    
    // Инициализация
    assert!(collector.initialize().is_ok());
    
    // Сбор метрик
    let metrics = collector.collect_metrics();
    assert!(metrics.is_ok());
    
    let metrics = metrics.unwrap();
    
    // Проверка метрик
    assert!(metrics.cpu_usage >= 0.0);
    assert!(metrics.memory_usage > 0);
    assert!(metrics.syscall_count >= 0);
}
```

### 4.2 Тесты производительности

**Тест времени инициализации:**
```rust
#[bench]
fn bench_ebpf_initialization(b: &mut Bencher) {
    b.iter(|| {
        let config = EbpfConfig::default();
        let mut collector = EbpfMetricsCollector::new(config);
        collector.initialize().unwrap();
    });
}
```

**Тест времени сбора метрик:**
```rust
#[bench]
fn bench_ebpf_metrics_collection(b: &mut Bencher) {
    let config = EbpfConfig::default();
    let mut collector = EbpfMetricsCollector::new(config);
    collector.initialize().unwrap();
    
    b.iter(|| {
        let _ = collector.collect_metrics().unwrap();
    });
}
```

### 4.3 Тесты стабильности

**Тест долговременного выполнения:**
```rust
#[test]
fn test_ebpf_long_term_stability() {
    let config = EbpfConfig::default();
    let mut collector = EbpfMetricsCollector::new(config);
    
    // Инициализация
    assert!(collector.initialize().is_ok());
    
    // Долговременный сбор метрик
    for i in 0..1000 {
        let metrics = collector.collect_metrics();
        assert!(metrics.is_ok(), "Итерация {}", i);
        
        // Проверка метрик
        let metrics = metrics.unwrap();
        assert!(metrics.cpu_usage >= 0.0);
        assert!(metrics.memory_usage > 0);
    }
}
```

**Тест под нагрузкой:**
```rust
#[test]
fn test_ebpf_under_load() {
    let config = EbpfConfig::default();
    let mut collector = EbpfMetricsCollector::new(config);
    
    // Инициализация
    assert!(collector.initialize().is_ok());
    
    // Создание нагрузки
    let load_handles: Vec<_> = (0..10).map(|_| {
        std::thread::spawn(|| {
            // Имитация нагрузки
            let mut x = 0.0;
            for _ in 0..1000000 {
                x += (x * 0.1).sin();
            }
        })
    }).collect();
    
    // Сбор метрик под нагрузкой
    for _ in 0..100 {
        let metrics = collector.collect_metrics();
        assert!(metrics.is_ok());
    }
    
    // Ожидание завершения нагрузки
    for handle in load_handles {
        handle.join().unwrap();
    }
}
```

## 5. Интеграция с системой CI/CD

### 5.1 Тестирование в CI/CD

**GitHub Actions:**
```yaml
name: eBPF Testing

on:
  push:
    branches: [ main ]
  pull_request:
    branches: [ main ]

jobs:
  test:
    name: eBPF Tests
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v2
    
    - name: Install dependencies
      run: |
        sudo apt-get update
        sudo apt-get install -y libelf-dev libglib2.0-dev clang llvm linux-headers-$(uname -r)
    
    - name: Build with eBPF
      run: cargo build --features ebpf
    
    - name: Run tests
      run: cargo test --features ebpf
    
    - name: Run benchmarks
      run: cargo bench --features ebpf -- --sample-size 10
```

### 5.2 Тестирование на различных платформах

**Matrix тестирование:**
```yaml
jobs:
  test:
    name: eBPF Tests
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-20.04, ubuntu-22.04, fedora-latest]
    
    steps:
    - uses: actions/checkout@v2
    
    - name: Install dependencies
      run: |
        if [ "${{ matrix.os }}" == "ubuntu-20.04" ] || [ "${{ matrix.os }}" == "ubuntu-22.04" ]; then
          sudo apt-get update
          sudo apt-get install -y libelf-dev libglib2.0-dev clang llvm linux-headers-$(uname -r)
        elif [ "${{ matrix.os }}" == "fedora-latest" ]; then
          sudo dnf install -y elfutils-libelf-devel glib2-devel clang llvm kernel-headers
        fi
    
    - name: Build with eBPF
      run: cargo build --features ebpf
    
    - name: Run tests
      run: cargo test --features ebpf
```

## 6. Документирование результатов

### 6.1 Шаблон отчета о тестировании

```markdown
# Отчет о тестировании eBPF функциональности

## Тестовая среда

- **Дата тестирования:** [Дата]
- **Тестировщик:** [Имя]
- **Версия SmoothTask:** [Версия]

## Тестовая конфигурация

| Параметр | Значение |
|----------|---------|
| Операционная система | [Ubuntu 22.04, Fedora 37, etc.] |
| Версия ядра | [uname -r] |
| Архитектура | [x86_64, ARM64] |
| Процессор | [Модель] |
| Память | [Объем] |
| Диск | [Тип и объем] |

## Результаты тестирования

### Базовая функциональность

| Тест | Результат | Примечания |
|------|-----------|------------|
| Загрузка CPU программы | ✅/❌ | [Примечания] |
| Загрузка программы памяти | ✅/❌ | [Примечания] |
| Загрузка программы системных вызовов | ✅/❌ | [Примечания] |
| Загрузка сетевой программы | ✅/❌ | [Примечания] |
| Сбор CPU метрик | ✅/❌ | [Примечания] |
| Сбор метрик памяти | ✅/❌ | [Примечания] |
| Сбор метрик системных вызовов | ✅/❌ | [Примечания] |
| Сбор сетевых метрик | ✅/❌ | [Примечания] |

### Производительность

| Метрика | Значение | Примечания |
|---------|---------|------------|
| Время инициализации | [ms] | [Примечания] |
| Время сбора метрик | [ms] | [Примечания] |
| Использование CPU | [%] | [Примечания] |
| Использование памяти | [MB] | [Примечания] |

### Стабильность

| Тест | Результат | Примечания |
|------|-----------|------------|
| Долговременное тестирование (1 час) | ✅/❌ | [Примечания] |
| Долговременное тестирование (24 часа) | ✅/❌ | [Примечания] |
| Тестирование под нагрузкой | ✅/❌ | [Примечания] |
| Тестирование в условиях ограниченных ресурсов | ✅/❌ | [Примечания] |

## Известные проблемы

1. **Проблема:** [Описание]
   - **Влияние:** [Низкое/Среднее/Высокое]
   - **Обходной путь:** [Описание]
   - **Статус:** [Открыто/В процессе/Решено]

2. **Проблема:** [Описание]
   - **Влияние:** [Низкое/Среднее/Высокое]
   - **Обходной путь:** [Описание]
   - **Статус:** [Открыто/В процессе/Решено]

## Рекомендации

1. **Рекомендация:** [Описание]
   - **Приоритет:** [Низкий/Средний/Высокий]
   - **Обоснование:** [Описание]

2. **Рекомендация:** [Описание]
   - **Приоритет:** [Низкий/Средний/Высокий]
   - **Обоснование:** [Описание]

## Заключение

[Общее резюме результатов тестирования, основные выводы и рекомендации]
```

### 6.2 Шаблон документации по развертыванию

```markdown
# Руководство по развертыванию eBPF функциональности

## Требования к системе

### Минимальные требования
- **Операционная система:** Linux (Ubuntu 18.04+, Debian 10+, Fedora 33+, Arch Linux)
- **Версия ядра:** Linux 4.4+ (рекомендуется 5.4+)
- **Архитектура:** x86_64 или ARM64
- **Память:** 2 GB RAM
- **Диск:** 10 GB свободного пространства

### Рекомендуемые требования
- **Операционная система:** Ubuntu 22.04 LTS или Fedora 37
- **Версия ядра:** Linux 6.0+
- **Архитектура:** x86_64
- **Память:** 4 GB RAM
- **Диск:** 20 GB свободного пространства

## Установка зависимостей

### Ubuntu/Debian
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

### Fedora/RHEL
```bash
sudo dnf install -y \
    elfutils-libelf-devel \
    glib2-devel \
    pkg-config \
    clang \
    llvm \
    kernel-headers
```

### Arch Linux
```bash
sudo pacman -S \
    libelf \
    glib2 \
    pkgconf \
    clang \
    llvm \
    linux-headers
```

## Сборка с eBPF поддержкой

```bash
# Клонирование репозитория
git clone https://github.com/smoothtask/smoothtask.git
cd smoothtask

# Сборка с eBPF поддержкой
cargo build --features ebpf --release

# Установка
sudo cp target/release/smoothtaskd /usr/local/bin/
```

## Конфигурация

### Базовая конфигурация
```yaml
# Включение eBPF поддержки
metrics:
  ebpf:
    enable_cpu_metrics: true
    enable_memory_metrics: true
    enable_syscall_monitoring: false
    enable_network_monitoring: false
    enable_gpu_monitoring: false
    enable_filesystem_monitoring: false
    collection_interval: 1s
    enable_caching: true
    batch_size: 100
```

### Расширенная конфигурация
```yaml
# Расширенная конфигурация eBPF
metrics:
  ebpf:
    enable_cpu_metrics: true
    enable_memory_metrics: true
    enable_syscall_monitoring: true
    enable_network_monitoring: true
    enable_gpu_monitoring: true
    enable_filesystem_monitoring: true
    collection_interval: 1s
    enable_caching: true
    batch_size: 100
    max_init_attempts: 3
    operation_timeout_ms: 1000
    enable_high_performance_mode: true
    enable_aggressive_caching: false
    aggressive_cache_interval_ms: 5000
```

## Запуск и тестирование

### Базовое тестирование
```bash
# Тестирование базовой функциональности
sudo /usr/local/bin/smoothtaskd --test-ebpf-basic

# Тестирование сбора метрик
sudo /usr/local/bin/smoothtaskd --test-ebpf-metrics
```

### Расширенное тестирование
```bash
# Тестирование производительности
sudo /usr/local/bin/smoothtaskd --test-ebpf-performance

# Тестирование стабильности
sudo /usr/local/bin/smoothtaskd --test-ebpf-stability --duration 1h
```

### Запуск в производственной среде
```bash
# Запуск демона
sudo /usr/local/bin/smoothtaskd --config /etc/smoothtask/config.yml

# Проверка статуса
sudo systemctl status smoothtaskd

# Просмотр логов
sudo journalctl -u smoothtaskd -f
```

## Устранение неполадок

### Распространенные проблемы

#### Ошибка: "eBPF не поддерживается в этой системе"

**Причина:** Ядро Linux не поддерживает eBPF или версия ядра слишком старая.

**Решение:**
1. Проверьте версию ядра:
   ```bash
   uname -r
   ```
2. Обновите ядро до версии 5.4+:
   ```bash
   # Ubuntu
   sudo apt-get update && sudo apt-get upgrade
   
   # Fedora
   sudo dnf upgrade
   ```
3. Проверьте конфигурацию ядра:
   ```bash
   grep CONFIG_BPF /boot/config-$(uname -r)
   ```

#### Ошибка: "Недостаточно прав для загрузки eBPF программ"

**Причина:** Отсутствуют необходимые права для загрузки eBPF программ.

**Решение:**
1. Запустите с повышенными привилегиями:
   ```bash
   sudo /usr/local/bin/smoothtaskd
   ```
2. Настройте capabilities:
   ```bash
   sudo setcap cap_bpf+ep /usr/local/bin/smoothtaskd
   ```

#### Ошибка: "Не удалось загрузить eBPF программу"

**Причина:** Отсутствуют eBPF программы или проблемы с компиляцией.

**Решение:**
1. Проверьте наличие eBPF программ:
   ```bash
   ls /usr/local/lib/smoothtask/ebpf_programs/
   ```
2. Переустановите SmoothTask:
   ```bash
   sudo rm -rf /usr/local/lib/smoothtask/
   sudo cp -r ebpf_programs /usr/local/lib/smoothtask/
   ```

## Производительность и оптимизация

### Оптимизация производительности

1. **Кэширование:**
   ```yaml
   metrics:
     ebpf:
       enable_caching: true
       batch_size: 200
   ```

2. **Выборочный мониторинг:**
   ```yaml
   metrics:
     ebpf:
       enable_cpu_metrics: true
       enable_memory_metrics: false
       enable_syscall_monitoring: false
   ```

3. **Агрессивное кэширование:**
   ```yaml
   metrics:
     ebpf:
       enable_aggressive_caching: true
       aggressive_cache_interval_ms: 10000
   ```

### Мониторинг производительности

```bash
# Мониторинг использования CPU
sudo /usr/local/bin/smoothtaskd --monitor-ebpf-cpu

# Мониторинг использования памяти
sudo /usr/local/bin/smoothtaskd --monitor-ebpf-memory

# Мониторинг системных вызовов
sudo /usr/local/bin/smoothtaskd --monitor-ebpf-syscalls
```

## Безопасность

### Безопасность eBPF

1. **Ограничение возможностей:**
   ```bash
   sudo setcap cap_bpf,cap_perfmon+ep /usr/local/bin/smoothtaskd
   ```

2. **Ограничение доступа:**
   ```bash
   sudo chown root:smoothtask /usr/local/bin/smoothtaskd
   sudo chmod 750 /usr/local/bin/smoothtaskd
   ```

3. **Мониторинг активности:**
   ```bash
   sudo /usr/local/bin/smoothtaskd --monitor-ebpf-security
   ```

## Обновление и поддержка

### Обновление SmoothTask

```bash
# Остановка сервиса
sudo systemctl stop smoothtaskd

# Обновление кода
git pull

# Пересборка
cargo build --features ebpf --release

# Обновление бинарника
sudo cp target/release/smoothtaskd /usr/local/bin/

# Запуск сервиса
sudo systemctl start smoothtaskd
```

### Получение поддержки

1. **Проверка логов:**
   ```bash
   sudo journalctl -u smoothtaskd -f
   ```

2. **Сбор диагностической информации:**
   ```bash
   sudo /usr/local/bin/smoothtaskd --diagnostics > diagnostics.txt
   ```

3. **Создание issue:**
   - Опишите проблему
   - Приложите диагностическую информацию
   - Укажите версию SmoothTask и конфигурацию системы

## Заключение

Это руководство предоставляет комплексный подход к развертыванию, тестированию и поддержке eBPF функциональности SmoothTask. Следуя этим рекомендациям, вы сможете обеспечить стабильную и производительную работу системы на различных конфигурациях.
```

## 7. Заключение

Это руководство предоставляет комплексный план тестирования eBPF функциональности SmoothTask на реальных системах. План включает:

1. **Анализ текущего состояния** и идентификацию требований
2. **Реализацию реальной eBPF функциональности** с интеграцией libbpf-rs
3. **Тестирование на различных конфигурациях** ядра и дистрибутивов
4. **Сбор метрик производительности и стабильности**
5. **Документирование результатов и ограничений**

Реализация этого плана обеспечит стабильную и производительную работу eBPF функциональности SmoothTask на различных системах, что является критически важным для обеспечения высокопроизводительного сбора системных метрик с минимальными накладными расходами.

## 8. Следующие шаги

1. **Реализация реальной eBPF функциональности** (ST-549-3)
2. **Обновление интеграционных тестов** (ST-549-4)
3. **Создание производительных бенчмарков** (ST-549-5)
4. **Тестирование на различных системах**
5. **Документирование результатов**

## 9. Ссылки

- [libbpf-rs документация](https://docs.rs/libbpf-rs)
- [eBPF официальная документация](https://ebpf.io/)
- [Linux eBPF документация](https://www.kernel.org/doc/html/latest/bpf/)
- [BPF Performance Tools](https://github.com/brendangregg/bpf-perf-tools-book)
- [Cilium eBPF Documentation](https://docs.cilium.io/en/stable/bpf/)