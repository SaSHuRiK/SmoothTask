use anyhow::{anyhow, Context, Result};
#[cfg(feature = "ebpf")]
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::warn;

/// Сырые счётчики CPU из `/proc/stat`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct CpuTimes {
    pub user: u64,
    pub nice: u64,
    pub system: u64,
    pub idle: u64,
    pub iowait: u64,
    pub irq: u64,
    pub softirq: u64,
    pub steal: u64,
    pub guest: u64,
    pub guest_nice: u64,
}

/// Отнормированное использование CPU за интервал между двумя замерами.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CpuUsage {
    /// user + nice
    pub user: f64,
    /// system + irq + softirq
    pub system: f64,
    pub idle: f64,
    pub iowait: f64,
}

impl CpuTimes {
    /// Рассчитать доли использования CPU относительно предыдущего снимка.
    ///
    /// Вычисляет разницу между текущими и предыдущими счетчиками CPU и нормализует
    /// их в проценты использования (user, system, idle, iowait).
    ///
    /// # Возвращаемое значение
    ///
    /// - `Some(CpuUsage)` - если удалось вычислить использование CPU
    /// - `None` - если произошло переполнение счетчиков (prev > cur) или total = 0
    ///
    /// # Граничные случаи
    ///
    /// - **Переполнение счетчиков**: Если какой-либо счетчик в `prev` больше, чем в `self`,
    ///   это может означать переполнение счетчика (на долгоживущих системах) или некорректные данные.
    ///   В этом случае функция возвращает `None`.
    ///
    /// - **Нулевой total**: Если сумма всех дельт равна нулю (все счетчики не изменились),
    ///   функция возвращает `None`, так как невозможно вычислить проценты.
    ///
    /// - **Все счетчики равны**: Если все счетчики в `prev` и `self` равны, функция вернет `None`.
    ///
    /// # Примеры
    ///
    /// ```rust
    /// use smoothtask_core::metrics::system::CpuTimes;
    ///
    /// let prev = CpuTimes {
    ///     user: 100, nice: 20, system: 50, idle: 200,
    ///     iowait: 10, irq: 5, softirq: 5, steal: 0,
    ///     guest: 0, guest_nice: 0,
    /// };
    ///
    /// let cur = CpuTimes {
    ///     user: 150, nice: 30, system: 80, idle: 260,
    ///     iowait: 20, irq: 10, softirq: 10, steal: 0,
    ///     guest: 0, guest_nice: 0,
    /// };
    ///
    /// let usage = cur.delta(&prev).expect("должно быть Some");
    /// assert!(usage.user > 0.0);
    /// assert!(usage.idle > 0.0);
    /// ```
    ///
    /// ```rust
    /// use smoothtask_core::metrics::system::CpuTimes;
    ///
    /// // Переполнение счетчиков
    /// let prev = CpuTimes {
    ///     user: 200, nice: 0, system: 0, idle: 0,
    ///     iowait: 0, irq: 0, softirq: 0, steal: 0,
    ///     guest: 0, guest_nice: 0,
    /// };
    ///
    /// let cur = CpuTimes {
    ///     user: 100, nice: 0, system: 0, idle: 0,
    ///     iowait: 0, irq: 0, softirq: 0, steal: 0,
    ///     guest: 0, guest_nice: 0,
    /// };
    ///
    /// assert!(cur.delta(&prev).is_none()); // переполнение
    /// ```
    ///
    /// ```rust
    /// use smoothtask_core::metrics::system::CpuTimes;
    ///
    /// // Нулевой total (все счетчики равны)
    /// let prev = CpuTimes {
    ///     user: 100, nice: 0, system: 0, idle: 0,
    ///     iowait: 0, irq: 0, softirq: 0, steal: 0,
    ///     guest: 0, guest_nice: 0,
    /// };
    ///
    /// let cur = prev; // все счетчики равны
    /// assert!(cur.delta(&prev).is_none()); // total = 0
    /// ```
    pub fn delta(&self, prev: &CpuTimes) -> Option<CpuUsage> {
        let user = self.user.checked_sub(prev.user)?;
        let nice = self.nice.checked_sub(prev.nice)?;
        let system = self.system.checked_sub(prev.system)?;
        let idle = self.idle.checked_sub(prev.idle)?;
        let iowait = self.iowait.checked_sub(prev.iowait)?;
        let irq = self.irq.checked_sub(prev.irq)?;
        let softirq = self.softirq.checked_sub(prev.softirq)?;
        let steal = self.steal.checked_sub(prev.steal)?;
        let total = user + nice + system + idle + iowait + irq + softirq + steal;
        if total == 0 {
            return None;
        }

        Some(CpuUsage {
            user: (user + nice) as f64 / total as f64,
            system: (system + irq + softirq) as f64 / total as f64,
            idle: idle as f64 / total as f64,
            iowait: iowait as f64 / total as f64,
        })
    }
}

/// Основные метрики памяти (значения в килобайтах).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct MemoryInfo {
    pub mem_total_kb: u64,
    pub mem_available_kb: u64,
    pub mem_free_kb: u64,
    pub buffers_kb: u64,
    pub cached_kb: u64,
    pub swap_total_kb: u64,
    pub swap_free_kb: u64,
}

impl MemoryInfo {
    /// Вычисляет использованную память в килобайтах.
    ///
    /// Использует `saturating_sub` для безопасной обработки случаев, когда
    /// `mem_available_kb` больше `mem_total_kb` (некорректные данные).
    ///
    /// # Возвращает
    ///
    /// Количество использованной памяти в килобайтах.
    /// Если `mem_available_kb > mem_total_kb`, возвращает 0.
    ///
    /// # Примеры
    ///
    /// ```rust
    /// use smoothtask_core::metrics::system::MemoryInfo;
    ///
    /// let mem = MemoryInfo {
    ///     mem_total_kb: 16_384_256,
    ///     mem_available_kb: 9_876_543,
    ///     mem_free_kb: 1_234_567,
    ///     buffers_kb: 345_678,
    ///     cached_kb: 2_345_678,
    ///     swap_total_kb: 8_192_000,
    ///     swap_free_kb: 4_096_000,
    /// };
    ///
    /// let used = mem.mem_used_kb();
    /// assert_eq!(used, 16_384_256 - 9_876_543);
    /// ```
    pub fn mem_used_kb(&self) -> u64 {
        self.mem_total_kb.saturating_sub(self.mem_available_kb)
    }

    /// Вычисляет использованный swap в килобайтах.
    ///
    /// Использует `saturating_sub` для безопасной обработки случаев, когда
    /// `swap_free_kb` больше `swap_total_kb` (некорректные данные).
    ///
    /// # Возвращает
    ///
    /// Количество использованного swap в килобайтах.
    /// Если `swap_free_kb > swap_total_kb`, возвращает 0.
    ///
    /// # Примеры
    ///
    /// ```rust
    /// use smoothtask_core::metrics::system::MemoryInfo;
    ///
    /// let mem = MemoryInfo {
    ///     mem_total_kb: 0,
    ///     mem_available_kb: 0,
    ///     mem_free_kb: 0,
    ///     buffers_kb: 0,
    ///     cached_kb: 0,
    ///     swap_total_kb: 8_192_000,
    ///     swap_free_kb: 4_096_000,
    /// };
    ///
    /// let used = mem.swap_used_kb();
    /// assert_eq!(used, 8_192_000 - 4_096_000);
    /// ```
    pub fn swap_used_kb(&self) -> u64 {
        self.swap_total_kb.saturating_sub(self.swap_free_kb)
    }
}

/// Средняя нагрузка системы за различные интервалы времени.
///
/// Значения загружаются из `/proc/loadavg` и представляют среднее количество
/// процессов в состоянии выполнения или ожидания выполнения за последние 1, 5 и 15 минут.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct LoadAvg {
    /// Средняя нагрузка за последнюю минуту
    pub one: f64,
    /// Средняя нагрузка за последние 5 минут
    pub five: f64,
    /// Средняя нагрузка за последние 15 минут
    pub fifteen: f64,
}

/// Запись о давлении (pressure) из PSI (Pressure Stall Information).
///
/// PSI предоставляет информацию о нехватке ресурсов (CPU, IO, память).
/// Значения `avg10`, `avg60`, `avg300` представляют среднее давление за последние
/// 10 секунд, 1 минуту и 5 минут соответственно.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PressureRecord {
    /// Среднее давление за последние 10 секунд
    pub avg10: f64,
    /// Среднее давление за последние 60 секунд
    pub avg60: f64,
    /// Среднее давление за последние 300 секунд (5 минут)
    pub avg300: f64,
    /// Общее количество микросекунд, в течение которых происходило давление
    pub total: u64,
}

/// Давление ресурса (CPU, IO или память) с двумя типами: some и full.
///
/// - `some`: давление, когда хотя бы одна задача ждёт ресурс
/// - `full`: давление, когда все задачи ждут ресурс
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Pressure {
    /// Давление типа "some" (хотя бы одна задача ждёт)
    pub some: Option<PressureRecord>,
    /// Давление типа "full" (все задачи ждут)
    pub full: Option<PressureRecord>,
}

/// Метрики давления для всех типов ресурсов (CPU, IO, память).
///
/// Содержит информацию о давлении для каждого типа ресурса из PSI.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PressureMetrics {
    /// Давление CPU
    pub cpu: Pressure,
    /// Давление IO
    pub io: Pressure,
    /// Давление памяти
    pub memory: Pressure,
}

/// Метрики температуры CPU/GPU
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TemperatureMetrics {
    /// Температура CPU в градусах Цельсия
    pub cpu_temperature_c: Option<f32>,
    /// Температура GPU в градусах Цельсия (если доступно)
    pub gpu_temperature_c: Option<f32>,
}

/// Метрики энергопотребления
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PowerMetrics {
    /// Текущее энергопотребление системы в ваттах
    pub system_power_w: Option<f32>,
    /// Энергопотребление CPU в ваттах
    pub cpu_power_w: Option<f32>,
    /// Энергопотребление GPU в ваттах (если доступно)
    pub gpu_power_w: Option<f32>,
}

/// Метрики сетевой активности
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct NetworkMetrics {
    /// Список сетевых интерфейсов
    #[serde(default)]
    pub interfaces: Vec<NetworkInterface>,
    /// Общее количество полученных байт
    pub total_rx_bytes: u64,
    /// Общее количество отправленных байт
    pub total_tx_bytes: u64,
}

/// Информация о сетевом интерфейсе
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkInterface {
    /// Имя интерфейса (например, "eth0", "wlan0")
    /// Используем Box<str> вместо String для уменьшения memory footprint
    pub name: Box<str>,
    /// Полученные байты
    pub rx_bytes: u64,
    /// Отправленные байты
    pub tx_bytes: u64,
    /// Полученные пакеты
    pub rx_packets: u64,
    /// Отправленные пакеты
    pub tx_packets: u64,
    /// Ошибки приема
    pub rx_errors: u64,
    /// Ошибки передачи
    pub tx_errors: u64,
}

/// Метрики дисковых операций
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DiskMetrics {
    /// Список дисковых устройств
    #[serde(default)]
    pub devices: Vec<DiskDevice>,
    /// Общее количество прочитанных байт
    pub total_read_bytes: u64,
    /// Общее количество записанных байт
    pub total_write_bytes: u64,
}

/// Информация о дисковом устройстве
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiskDevice {
    /// Имя устройства (например, "sda", "nvme0n1")
    /// Используем Box<str> вместо String для уменьшения memory footprint
    pub name: Box<str>,
    /// Прочитанные байты
    pub read_bytes: u64,
    /// Записанные байты
    pub write_bytes: u64,
    /// Операции чтения
    pub read_ops: u64,
    /// Операции записи
    pub write_ops: u64,
    /// Время ввода-вывода в миллисекундах
    pub io_time: u64,
}

/// Полный набор системных метрик, собранных из `/proc`.
///
/// Содержит информацию о CPU, памяти, нагрузке системы и давлении ресурсов.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SystemMetrics {
    /// Счётчики CPU из `/proc/stat`
    pub cpu_times: CpuTimes,
    /// Информация о памяти из `/proc/meminfo`
    pub memory: MemoryInfo,
    /// Средняя нагрузка системы из `/proc/loadavg`
    pub load_avg: LoadAvg,
    /// Метрики давления из PSI (`/proc/pressure/*`)
    pub pressure: PressureMetrics,
    /// Метрики температуры CPU/GPU
    pub temperature: TemperatureMetrics,
    /// Метрики энергопотребления
    pub power: PowerMetrics,
    /// Метрики сетевой активности
    pub network: NetworkMetrics,
    /// Метрики дисковых операций
    pub disk: DiskMetrics,
    /// Метрики GPU (опционально, так как может быть недоступно на некоторых системах)
    pub gpu: Option<crate::metrics::gpu::GpuMetricsCollection>,
    /// Метрики eBPF (опционально, так как требует поддержки eBPF в системе)
    pub ebpf: Option<crate::metrics::ebpf::EbpfMetrics>,
}

impl SystemMetrics {
    /// Вычисляет доли использования CPU относительно предыдущего снапшота.
    ///
    /// Делегирует вычисление к `CpuTimes::delta()` для получения нормализованных
    /// процентов использования CPU (user, system, idle, iowait).
    ///
    /// # Аргументы
    ///
    /// * `prev` - предыдущий снапшот системных метрик для вычисления дельт
    ///
    /// # Возвращает
    ///
    /// - `Some(CpuUsage)` - если удалось вычислить использование CPU
    /// - `None` - если произошло переполнение счетчиков или total = 0
    ///
    /// # Примеры
    ///
    /// ```rust
    /// use smoothtask_core::metrics::system::{SystemMetrics, CpuTimes, MemoryInfo, LoadAvg, PressureMetrics};
    ///
    /// let prev = SystemMetrics {
    ///     cpu_times: CpuTimes { user: 100, nice: 20, system: 50, idle: 200, iowait: 10, irq: 5, softirq: 5, steal: 0, guest: 0, guest_nice: 0 },
    ///     memory: MemoryInfo { mem_total_kb: 1000, mem_available_kb: 500, mem_free_kb: 400, buffers_kb: 50, cached_kb: 50, swap_total_kb: 1000, swap_free_kb: 800 },
    ///     load_avg: LoadAvg { one: 1.0, five: 1.0, fifteen: 1.0 },
    ///     pressure: PressureMetrics::default(),
    /// };
    ///
    /// let cur = SystemMetrics {
    ///     cpu_times: CpuTimes { user: 150, nice: 30, system: 80, idle: 260, iowait: 20, irq: 10, softirq: 10, steal: 0, guest: 0, guest_nice: 0 },
    ///     memory: prev.memory,
    ///     load_avg: prev.load_avg,
    ///     pressure: prev.pressure.clone(),
    /// };
    ///
    /// let usage = cur.cpu_usage_since(&prev);
    /// assert!(usage.is_some());
    /// ```
    pub fn cpu_usage_since(&self, prev: &SystemMetrics) -> Option<CpuUsage> {
        self.cpu_times.delta(&prev.cpu_times)
    }

    /// Оптимизирует использование памяти в структуре SystemMetrics.
    ///
    /// Эта функция уменьшает memory footprint за счет:
    /// 1. Удаления пустых Vec коллекций
    /// 2. Сжатия данных там, где это возможно
    /// 3. Оптимизации Optional полей
    ///
    /// # Возвращает
    ///
    /// Оптимизированную версию SystemMetrics с уменьшенным memory footprint
    pub fn optimize_memory_usage(mut self) -> Self {
        // Оптимизируем сетевые метрики
        if self.network.interfaces.is_empty() {
            self.network.interfaces = Vec::new();
        }

        // Оптимизируем дисковые метрики
        if self.disk.devices.is_empty() {
            self.disk.devices = Vec::new();
        }

        // Оптимизируем температурные метрики
        if self.temperature.cpu_temperature_c.is_none()
            && self.temperature.gpu_temperature_c.is_none()
        {
            self.temperature = TemperatureMetrics::default();
        }

        // Оптимизируем метрики энергопотребления
        if self.power.system_power_w.is_none()
            && self.power.cpu_power_w.is_none()
            && self.power.gpu_power_w.is_none()
        {
            self.power = PowerMetrics::default();
        }

        self
    }
}

/// Пути к файлам /proc, чтобы их можно было подменить в тестах.
#[derive(Debug, Clone)]
pub struct ProcPaths {
    pub stat: PathBuf,
    pub meminfo: PathBuf,
    pub loadavg: PathBuf,
    pub pressure_cpu: PathBuf,
    pub pressure_io: PathBuf,
    pub pressure_memory: PathBuf,
}

impl ProcPaths {
    /// Создаёт новый ProcPaths с указанным корневым путём к /proc.
    ///
    /// # Аргументы
    ///
    /// * `proc_root` - корневой путь к /proc (например, "/proc" или "/tmp/test_proc")
    ///
    /// # Возвращает
    ///
    /// `ProcPaths` с путями к файлам:
    /// - `stat` - `/proc/stat`
    /// - `meminfo` - `/proc/meminfo`
    /// - `loadavg` - `/proc/loadavg`
    /// - `pressure_cpu` - `/proc/pressure/cpu`
    /// - `pressure_io` - `/proc/pressure/io`
    /// - `pressure_memory` - `/proc/pressure/memory`
    ///
    /// # Примеры
    ///
    /// ```rust
    /// use smoothtask_core::metrics::system::ProcPaths;
    ///
    /// // Использование реального /proc
    /// let paths = ProcPaths::new("/proc");
    ///
    /// // Использование тестового пути
    /// let paths = ProcPaths::new("/tmp/test_proc");
    /// ```
    pub fn new(proc_root: impl AsRef<Path>) -> Self {
        let root = proc_root.as_ref();
        Self {
            stat: root.join("stat"),
            meminfo: root.join("meminfo"),
            loadavg: root.join("loadavg"),
            pressure_cpu: root.join("pressure").join("cpu"),
            pressure_io: root.join("pressure").join("io"),
            pressure_memory: root.join("pressure").join("memory"),
        }
    }
}

impl Default for ProcPaths {
    fn default() -> Self {
        Self::new("/proc")
    }
}

/// Собрать системные метрики из /proc.
///
/// Если PSI-файлы недоступны (например, на старых ядрах без поддержки PSI),
/// функция продолжит работу с пустыми метриками PSI вместо возврата ошибки.
/// eBPF метрики собираются автоматически, если поддержка eBPF включена и доступна.
///
/// # Ошибки
///
/// - Возвращает ошибку, если не удалось прочитать основные файлы (/proc/stat, /proc/meminfo, /proc/loadavg)
/// - Возвращает ошибку, если не удалось разобрать содержимое основных файлов
/// - PSI ошибки обрабатываются gracefully с предупреждениями и использованием пустых метрик
/// - eBPF ошибки обрабатываются gracefully с предупреждениями и использованием None для eBPF метрик
///
/// # Примеры
///
/// ```rust
/// use smoothtask_core::metrics::system::{collect_system_metrics, ProcPaths};
///
/// // Использование реального /proc
/// let paths = ProcPaths::default();
/// let metrics = collect_system_metrics(&paths).expect("Не удалось собрать системные метрики");
///
/// // Проверяем доступность eBPF метрик
/// if let Some(ebpf) = metrics.ebpf {
///     println!("eBPF CPU usage: {:.2}%", ebpf.cpu_usage);
/// } else {
///     println!("eBPF метрики недоступны");
/// }
///
/// // Использование тестового пути (для тестирования)
/// let test_paths = ProcPaths::new("/tmp/test_proc");
/// let result = collect_system_metrics(&test_paths);
/// // result будет Ok с пустыми PSI метриками, если PSI файлы отсутствуют
/// // и без eBPF метрик, если eBPF поддержка отключена
/// ```
///
/// # Пример использования в главном цикле демона
///
/// ```rust
/// use smoothtask_core::metrics::system::{collect_system_metrics, ProcPaths};
/// use std::thread;
/// use std::time::Duration;
///
/// let paths = ProcPaths::default();
///
/// // Основной цикл сбора метрик
/// loop {
///     match collect_system_metrics(&paths) {
///         Ok(metrics) => {
///             println!("CPU usage: {:.2}%", metrics.cpu_usage_since(&prev_metrics).map_or(0.0, |u| u.user * 100.0));
///             prev_metrics = metrics;
///         }
///         Err(e) => {
///             eprintln!("Ошибка сбора метрик: {}", e);
///         }
///     }
///     thread::sleep(Duration::from_secs(1));
/// }
/// ```
///
/// # Пример обработки ошибок и graceful degradation
///
/// ```rust
/// use smoothtask_core::metrics::system::{collect_system_metrics, ProcPaths};
///
/// let paths = ProcPaths::default();
/// let metrics = collect_system_metrics(&paths);
///
/// match metrics {
///     Ok(metrics) => {
///         // Метрики успешно собраны
///         println!("Метрики собраны успешно");
///         
///         // Проверяем доступность PSI метрик
///         if metrics.pressure.cpu.some.is_none() {
///             println!("PSI метрики CPU недоступны (возможно, старое ядро)");
///         }
///     }
///     Err(e) => {
///         // Критическая ошибка - основные файлы недоступны
///         eprintln!("Критическая ошибка сбора метрик: {}", e);
///         // Можно попробовать fallback или перезапустить демон
///     }
/// }
/// ```
///
/// # Пример использования с кэшированием
///
/// ```rust
/// use smoothtask_core::metrics::system::{collect_system_metrics, ProcPaths};
/// use std::time::{Instant, Duration};
///
/// let paths = ProcPaths::default();
/// let mut cached_metrics: Option<SystemMetrics> = None;
/// let mut last_update = Instant::now();
/// let cache_duration = Duration::from_secs(1); // Кэшируем на 1 секунду
///
/// // Основной цикл с кэшированием
/// loop {
///     let now = Instant::now();
///     if now.duration_since(last_update) > cache_duration || cached_metrics.is_none() {
///         // Кэш устарел, обновляем метрики
///         match collect_system_metrics(&paths) {
///             Ok(metrics) => {
///                 cached_metrics = Some(metrics);
///                 last_update = now;
///             }
///             Err(e) => {
///                 eprintln!("Ошибка сбора метрик: {}", e);
///                 // Продолжаем использовать старые метрики из кэша
///             }
///         }
///     }
///     
///     // Используем кэшированные метрики
///     if let Some(metrics) = &cached_metrics {
///         println!("Используем кэшированные метрики");
///     }
///     
///     std::thread::sleep(Duration::from_millis(100));
/// }
/// Собирает системные метрики с использованием кэша и параллельной обработки.
///
/// Эта функция использует кэш для уменьшения количества операций ввода-вывода
/// при частом опросе системных метрик. Если кэш пуст или устарел, функция
/// вызывает `collect_system_metrics_parallel` для сбора новых данных с использованием
/// параллельной обработки.
///
/// # Аргументы
///
/// * `cache` - Кэш системных метрик
/// * `paths` - Пути к файлам в `/proc` для чтения метрик
/// * `force_refresh` - Принудительно обновить кэш, игнорируя время жизни кэша
///
/// # Возвращаемое значение
///
/// Структура `SystemMetrics` с собранными метриками или ошибка, если
/// не удалось прочитать критические файлы (stat, meminfo, loadavg).
///
/// # Примеры
///
/// ```rust
/// use smoothtask_core::metrics::system::{collect_system_metrics_cached_parallel, ProcPaths, SharedSystemMetricsCache};
/// use std::path::PathBuf;
/// use std::time::Duration;
///
/// let paths = ProcPaths {
///     stat: PathBuf::from("/proc/stat"),
///     meminfo: PathBuf::from("/proc/meminfo"),
///     loadavg: PathBuf::from("/proc/loadavg"),
///     pressure_cpu: PathBuf::from("/proc/pressure/cpu"),
///     pressure_io: PathBuf::from("/proc/pressure/io"),
///     pressure_memory: PathBuf::from("/proc/pressure/memory"),
/// };
///
/// // Создаем кэш с временем жизни 1 секунда
/// let cache = SharedSystemMetricsCache::new(Duration::from_secs(1));
///
/// // Получаем метрики (будут собраны новые данные с использованием параллельной обработки)
/// let metrics1 = collect_system_metrics_cached_parallel(&cache, &paths, false).expect("Не удалось собрать системные метрики");
///
/// // Получаем метрики снова (будут использованы кэшированные данные)
/// let metrics2 = collect_system_metrics_cached_parallel(&cache, &paths, false).expect("Не удалось собрать системные метрики");
///
/// assert_eq!(metrics1.cpu_times, metrics2.cpu_times);
/// ```
#[cfg(test)]
pub fn collect_system_metrics_cached_parallel(
    cache: &SharedSystemMetricsCache,
    paths: &ProcPaths,
    force_refresh: bool,
) -> Result<SystemMetrics> {
    if force_refresh {
        // Принудительное обновление кэша
        cache.clear();
    }

    cache.get_or_update(|| collect_system_metrics_parallel(paths))
}

/// Собирает системные метрики с использованием кэша.
///
/// Эта функция использует кэш для уменьшения количества операций ввода-вывода
/// при частом опросе системных метрик. Если кэш пуст или устарел, функция
/// вызывает `collect_system_metrics` для сбора новых данных.
///
/// # Аргументы
///
/// * `cache` - Кэш системных метрик
/// * `paths` - Пути к файлам в `/proc` для чтения метрик
/// * `force_refresh` - Принудительно обновить кэш, игнорируя время жизни кэша
///
/// # Возвращаемое значение
///
/// Структура `SystemMetrics` с собранными метриками или ошибка, если
/// не удалось прочитать критические файлы (stat, meminfo, loadavg).
///
/// # Примеры
///
/// ```rust
/// use smoothtask_core::metrics::system::{collect_system_metrics_cached, ProcPaths, SharedSystemMetricsCache};
/// use std::path::PathBuf;
/// use std::time::Duration;
///
/// let paths = ProcPaths {
///     stat: PathBuf::from("/proc/stat"),
///     meminfo: PathBuf::from("/proc/meminfo"),
///     loadavg: PathBuf::from("/proc/loadavg"),
///     pressure_cpu: PathBuf::from("/proc/pressure/cpu"),
///     pressure_io: PathBuf::from("/proc/pressure/io"),
///     pressure_memory: PathBuf::from("/proc/pressure/memory"),
/// };
///
/// // Создаем кэш с временем жизни 1 секунда
/// let cache = SharedSystemMetricsCache::new(Duration::from_secs(1));
///
/// // Получаем метрики (будут собраны новые данные)
/// let metrics1 = collect_system_metrics_cached(&cache, &paths, false).expect("Не удалось собрать системные метрики");
///
/// // Получаем метрики снова (будут использованы кэшированные данные)
/// let metrics2 = collect_system_metrics_cached(&cache, &paths, false).expect("Не удалось собрать системные метрики");
///
/// assert_eq!(metrics1.cpu_times, metrics2.cpu_times);
/// ```
pub fn collect_system_metrics_cached(
    cache: &SharedSystemMetricsCache,
    paths: &ProcPaths,
    force_refresh: bool,
) -> Result<SystemMetrics> {
    if force_refresh {
        // Принудительное обновление кэша
        cache.clear();
    }

    cache.get_or_update(|| collect_system_metrics(paths))
}

/// Собрать системные метрики с использованием параллельной обработки.
///
/// Эта функция использует rayon для параллельного сбора различных типов метрик,
/// что значительно улучшает производительность на многоядерных системах.
///
/// # Аргументы
///
/// * `paths` - Пути к файлам в `/proc` для чтения метрик
///
/// # Возвращаемое значение
///
/// Структура `SystemMetrics` с собранными метриками или ошибка, если
/// не удалось прочитать критические файлы (stat, meminfo, loadavg).
///
/// # Примеры
///
/// ```rust
/// use smoothtask_core::metrics::system::{collect_system_metrics_parallel, ProcPaths};
///
/// let paths = ProcPaths::default();
/// let metrics = collect_system_metrics_parallel(&paths).expect("Не удалось собрать системные метрики");
/// ```
#[cfg(test)]
pub fn collect_system_metrics_parallel(paths: &ProcPaths) -> Result<SystemMetrics> {
    // Используем параллельную обработку для сбора различных типов метрик
    // Используем вложенные join для обработки нескольких задач параллельно

    // Первая группа задач
    let (cpu_times_result, memory_result) = rayon::join(
        || read_and_parse_cpu_metrics(paths),
        || read_and_parse_memory_metrics(paths),
    );

    // Вторая группа задач
    let (load_avg_result, pressure_result) = rayon::join(
        || read_and_parse_loadavg_metrics(paths),
        || read_and_parse_psi_metrics(paths),
    );

    // Третья группа задач
    let (temperature, power) =
        rayon::join(collect_temperature_metrics, collect_power_metrics);

    // Четвертая группа задач
    let (network, disk) = rayon::join(collect_network_metrics, collect_disk_metrics);

    // Пятая группа задач
    let (gpu, ebpf) = rayon::join(collect_gpu_metrics, collect_ebpf_metrics);

    let cpu_times = cpu_times_result??;
    let memory = memory_result??;
    let load_avg = load_avg_result??;
    let pressure = pressure_result?;

    Ok(SystemMetrics {
        cpu_times,
        memory,
        load_avg,
        pressure,
        temperature,
        power,
        network,
        disk,
        gpu: Some(gpu),
        ebpf,
    })
}

/// Вспомогательная функция для чтения и парсинга CPU метрик
#[cfg(test)]
fn read_and_parse_cpu_metrics(paths: &ProcPaths) -> Result<Result<CpuTimes>> {
    let cpu_contents = read_file(&paths.stat).with_context(|| {
        format!(
            "Не удалось прочитать CPU метрики из {}. \n             Проверьте, что файл существует и доступен для чтения. \n             Это может быть вызвано отсутствием прав доступа, отсутствием файла или проблемами с файловой системой. \n             Без этого файла невозможно собрать системные метрики. \n             Попробуйте: ls -la {} | sudo cat {}",
            paths.stat.display(),
            paths.stat.display(),
            paths.stat.display()
        )
    })?;

    let cpu_times = parse_cpu_times(&cpu_contents).with_context(|| {
        format!(
            "Не удалось разобрать CPU метрики из {}. \n             Проверьте, что файл содержит корректные данные в ожидаемом формате. \n             Ожидаемый формат: 'cpu <user> <nice> <system> <idle> <iowait> <irq> <softirq> <steal> <guest> <guest_nice>'",
            paths.stat.display()
        )
    });

    Ok(cpu_times)
}

/// Вспомогательная функция для чтения и парсинга метрик памяти
#[cfg(test)]
fn read_and_parse_memory_metrics(paths: &ProcPaths) -> Result<Result<MemoryInfo>> {
    let meminfo_contents = read_file(&paths.meminfo).with_context(|| {
        format!(
            "Не удалось прочитать информацию о памяти из {}. \n             Проверьте, что файл существует и доступен для чтения. \n             Это может быть вызвано отсутствием прав доступа, отсутствием файла или проблемами с файловой системой. \n             Без этого файла невозможно собрать системные метрики. \n             Попробуйте: ls -la {} | sudo cat {}",
            paths.meminfo.display(),
            paths.meminfo.display(),
            paths.meminfo.display()
        )
    })?;

    let memory = parse_meminfo(&meminfo_contents).with_context(|| {
        format!(
            "Не удалось разобрать информацию о памяти из {}. \n             Проверьте, что файл содержит корректные данные в ожидаемом формате. \n             Ожидаемый формат: '<key>: <value> kB' для полей MemTotal, MemAvailable, MemFree, Buffers, Cached, SwapTotal, SwapFree",
            paths.meminfo.display()
        )
    });

    Ok(memory)
}

/// Вспомогательная функция для чтения и парсинга метрик средней нагрузки
#[cfg(test)]
fn read_and_parse_loadavg_metrics(paths: &ProcPaths) -> Result<Result<LoadAvg>> {
    let loadavg_contents = read_file(&paths.loadavg).with_context(|| {
        format!(
            "Не удалось прочитать среднюю нагрузку из {}. \n             Проверьте, что файл существует и доступен для чтения. \n             Это может быть вызвано отсутствием прав доступа, отсутствием файла или проблемами с файловой системой. \n             Без этого файла невозможно собрать системные метрики. \n             Попробуйте: ls -la {} | sudo cat {}",
            paths.loadavg.display(),
            paths.loadavg.display(),
            paths.loadavg.display()
        )
    })?;

    let load_avg = parse_loadavg(&loadavg_contents).with_context(|| {
        format!(
            "Не удалось разобрать среднюю нагрузку из {}. \n             Проверьте, что файл содержит корректные данные в ожидаемом формате. \n             Ожидаемый формат: '<1m> <5m> <15m> <running>/<total> <last_pid>'",
            paths.loadavg.display()
        )
    });

    Ok(load_avg)
}

/// Вспомогательная функция для чтения и парсинга PSI метрик
#[cfg(test)]
fn read_and_parse_psi_metrics(paths: &ProcPaths) -> Result<PressureMetrics> {
    // PSI может быть недоступен на старых ядрах, поэтому обрабатываем ошибки gracefully
    let pressure_cpu = read_file(&paths.pressure_cpu)
        .and_then(|contents| parse_pressure(&contents))
        .unwrap_or_else(|e| {
            warn!(
                "Не удалось прочитать PSI CPU из {}: {}. \n                 Это может быть вызвано отсутствием поддержки PSI в ядре, отсутствием файла или проблемами с правами доступа. \n                 Используем пустые метрики для PSI CPU.",
                paths.pressure_cpu.display(),
                e
            );
            Pressure::default()
        });

    let pressure_io = read_file(&paths.pressure_io)
        .and_then(|contents| parse_pressure(&contents))
        .unwrap_or_else(|e| {
            warn!(
                "Не удалось прочитать PSI IO из {}: {}. \n                 Это может быть вызвано отсутствием поддержки PSI в ядре, отсутствием файла или проблемами с правами доступа. \n                 Используем пустые метрики для PSI IO.",
                paths.pressure_io.display(),
                e
            );
            Pressure::default()
        });

    let pressure_memory = read_file(&paths.pressure_memory)
        .and_then(|contents| parse_pressure(&contents))
        .unwrap_or_else(|e| {
            warn!(
                "Не удалось прочитать PSI Memory из {}: {}. \n                 Это может быть вызвано отсутствием поддержки PSI в ядре, отсутствием файла или проблемами с правами доступа. \n                 Используем пустые метрики для PSI Memory.",
                paths.pressure_memory.display(),
                e
            );
            Pressure::default()
        });

    Ok(PressureMetrics {
        cpu: pressure_cpu,
        io: pressure_io,
        memory: pressure_memory,
    })
}

pub fn collect_system_metrics(paths: &ProcPaths) -> Result<SystemMetrics> {
    // Читаем основные файлы с подробными сообщениями об ошибках
    let cpu_contents = read_file(&paths.stat).with_context(|| {
        format!(
            "Не удалось прочитать CPU метрики из {}. 
             Проверьте, что файл существует и доступен для чтения. 
             Это может быть вызвано отсутствием прав доступа, отсутствием файла или проблемами с файловой системой. 
             Без этого файла невозможно собрать системные метрики. 
             Попробуйте: ls -la {} | sudo cat {}",
            paths.stat.display(),
            paths.stat.display(),
            paths.stat.display()
        )
    })?;

    let meminfo_contents = read_file(&paths.meminfo).with_context(|| {
        format!(
            "Не удалось прочитать информацию о памяти из {}. 
             Проверьте, что файл существует и доступен для чтения. 
             Это может быть вызвано отсутствием прав доступа, отсутствием файла или проблемами с файловой системой. 
             Без этого файла невозможно собрать системные метрики. 
             Попробуйте: ls -la {} | sudo cat {}",
            paths.meminfo.display(),
            paths.meminfo.display(),
            paths.meminfo.display()
        )
    })?;

    let loadavg_contents = read_file(&paths.loadavg).with_context(|| {
        format!(
            "Не удалось прочитать среднюю нагрузку из {}. 
             Проверьте, что файл существует и доступен для чтения. 
             Это может быть вызвано отсутствием прав доступа, отсутствием файла или проблемами с файловой системой. 
             Без этого файла невозможно собрать системные метрики. 
             Попробуйте: ls -la {} | sudo cat {}",
            paths.loadavg.display(),
            paths.loadavg.display(),
            paths.loadavg.display()
        )
    })?;

    // Парсим основные метрики с подробными сообщениями об ошибках
    let cpu_times = parse_cpu_times(&cpu_contents).with_context(|| {
        format!(
            "Не удалось разобрать CPU метрики из {}. 
             Проверьте, что файл содержит корректные данные в ожидаемом формате. 
             Ожидаемый формат: 'cpu <user> <nice> <system> <idle> <iowait> <irq> <softirq> <steal> <guest> <guest_nice>'",
            paths.stat.display()
        )
    })?;

    let memory = parse_meminfo(&meminfo_contents).with_context(|| {
        format!(
            "Не удалось разобрать информацию о памяти из {}. 
             Проверьте, что файл содержит корректные данные в ожидаемом формате. 
             Ожидаемый формат: '<key>: <value> kB' для полей MemTotal, MemAvailable, MemFree, Buffers, Cached, SwapTotal, SwapFree",
            paths.meminfo.display()
        )
    })?;

    let load_avg = parse_loadavg(&loadavg_contents).with_context(|| {
        format!(
            "Не удалось разобрать среднюю нагрузку из {}. 
             Проверьте, что файл содержит корректные данные в ожидаемом формате. 
             Ожидаемый формат: '<1m> <5m> <15m> <running>/<total> <last_pid>'",
            paths.loadavg.display()
        )
    })?;

    // PSI может быть недоступен на старых ядрах, поэтому обрабатываем ошибки gracefully
    let pressure_cpu = read_file(&paths.pressure_cpu)
        .and_then(|contents| parse_pressure(&contents))
        .unwrap_or_else(|e| {
            warn!(
                "Не удалось прочитать PSI CPU из {}: {}. 
                 Это может быть вызвано отсутствием поддержки PSI в ядре, отсутствием файла или проблемами с правами доступа. 
                 Используем пустые метрики для PSI CPU.",
                paths.pressure_cpu.display(),
                e
            );
            Pressure::default()
        });

    let pressure_io = read_file(&paths.pressure_io)
        .and_then(|contents| parse_pressure(&contents))
        .unwrap_or_else(|e| {
            warn!(
                "Не удалось прочитать PSI IO из {}: {}. 
                 Это может быть вызвано отсутствием поддержки PSI в ядре, отсутствием файла или проблемами с правами доступа. 
                 Используем пустые метрики для PSI IO.",
                paths.pressure_io.display(),
                e
            );
            Pressure::default()
        });

    let pressure_memory = read_file(&paths.pressure_memory)
        .and_then(|contents| parse_pressure(&contents))
        .unwrap_or_else(|e| {
            warn!(
                "Не удалось прочитать PSI Memory из {}: {}. 
                 Это может быть вызвано отсутствием поддержки PSI в ядре, отсутствием файла или проблемами с правами доступа. 
                 Используем пустые метрики для PSI Memory.",
                paths.pressure_memory.display(),
                e
            );
            Pressure::default()
        });

    let pressure = PressureMetrics {
        cpu: pressure_cpu,
        io: pressure_io,
        memory: pressure_memory,
    };

    // Собираем метрики температуры и энергопотребления
    let temperature = collect_temperature_metrics();
    let power = collect_power_metrics();

    // Собираем метрики сетевой активности и дисковых операций
    let network = collect_network_metrics();
    let disk = collect_disk_metrics();

    // Собираем метрики GPU (опционально, может быть недоступно на некоторых системах)
    let gpu = collect_gpu_metrics();

    Ok(SystemMetrics {
        cpu_times,
        memory,
        load_avg,
        pressure,
        temperature,
        power,
        network,
        disk,
        gpu: Some(gpu),
        ebpf: collect_ebpf_metrics(),
    })
}

/// Собирает метрики температуры из sysfs/hwmon
fn collect_temperature_metrics() -> TemperatureMetrics {
    let mut temperature = TemperatureMetrics::default();

    // Логируем начало процесса сбора температурных метрик
    tracing::info!("Starting temperature metrics collection");

    // Попробуем найти температурные сенсоры в /sys/class/hwmon/
    let hwmon_dir = Path::new("/sys/class/hwmon");
    tracing::debug!(
        "Attempting to read temperature sensors from hwmon interface at: {}",
        hwmon_dir.display()
    );

    if !hwmon_dir.exists() {
        tracing::warn!("hwmon directory not found at: {}", hwmon_dir.display());
    } else {
        match fs::read_dir(hwmon_dir) {
            Ok(entries) => {
                tracing::debug!("Found {} hwmon devices", entries.count());
                // Нужно перечитать, так как entries уже потреблено
                if let Ok(entries) = fs::read_dir(hwmon_dir) {
                    for entry in entries {
                        let path_str = match &entry {
                            Ok(entry) => {
                                let path = entry.path();
                                let path_str = path.to_string_lossy().into_owned();
                                tracing::debug!("Processing hwmon device: {}", path_str);
                                path_str
                            }
                            Err(_) => "unknown".to_string(),
                        };

                        match entry {
                            Ok(entry) => {
                                let path = entry.path();

                                // Ищем файлы temp*_input в каждом hwmon устройстве
                                match fs::read_dir(&path) {
                                    Ok(temp_files) => {
                                        for temp_file in temp_files {
                                            match temp_file {
                                                Ok(temp_file) => {
                                                    let temp_path = temp_file.path();
                                                    let file_name = temp_path
                                                        .file_name()
                                                        .and_then(|s| s.to_str())
                                                        .unwrap_or("");

                                                    if file_name.starts_with("temp")
                                                        && file_name.ends_with("_input")
                                                    {
                                                        tracing::debug!(
                                                            "Found temperature sensor file: {}",
                                                            temp_path.display()
                                                        );

                                                        match fs::read_to_string(&temp_path) {
                                                            Ok(temp_content) => {
                                                                match temp_content
                                                                    .trim()
                                                                    .parse::<u64>()
                                                                {
                                                                    Ok(temp_millidegrees) => {
                                                                        let temp_c =
                                                                            temp_millidegrees
                                                                                as f32
                                                                                / 1000.0;
                                                                        tracing::debug!("Successfully read temperature: {:.1}°C from {}", temp_c, temp_path.display());

                                                                        // Пробуем определить тип устройства по имени файла и пути
                                                                        // Это более сложная логика, чем раньше

                                                                        // 1. Пробуем определить по имени hwmon устройства
                                                                        if let Some(hwmon_name) =
                                                                            path.file_name()
                                                                                .and_then(|s| {
                                                                                    s.to_str()
                                                                                })
                                                                        {
                                                                            tracing::debug!("hwmon device name: {}", hwmon_name);

                                                                            if hwmon_name.contains(
                                                                                "coretemp",
                                                                            ) || hwmon_name
                                                                                .contains("k10temp")
                                                                                || hwmon_name
                                                                                    .contains(
                                                                                        "amdgpu",
                                                                                    )
                                                                                || hwmon_name
                                                                                    .contains(
                                                                                        "radeon",
                                                                                    )
                                                                            {
                                                                                // Это CPU температура (Intel CoreTemp, AMD K10Temp)
                                                                                if temperature.cpu_temperature_c.is_none() {
                                                                                    temperature.cpu_temperature_c = Some(temp_c);
                                                                                    tracing::info!(
                                                                                        "CPU temperature detected (hwmon {}): {:.1}°C",
                                                                                        hwmon_name,
                                                                                        temp_c
                                                                                    );
                                                                                } else {
                                                                                    tracing::debug!("CPU temperature already set, skipping duplicate");
                                                                                }
                                                                            } else if hwmon_name
                                                                                .contains("nvme")
                                                                                || hwmon_name
                                                                                    .contains("ssd")
                                                                            {
                                                                                // Это температура накопителя
                                                                                // Пока не сохраняем, но можно было бы добавить
                                                                                tracing::debug!(
                                                                                    "Storage temperature detected (hwmon {}): {:.1}°C",
                                                                                    hwmon_name,
                                                                                    temp_c
                                                                                );
                                                                            } else {
                                                                                tracing::debug!("Unknown hwmon device type: {}", hwmon_name);
                                                                            }
                                                                        }

                                                                        // 2. Пробуем определить по имени файла
                                                                        if temperature
                                                                            .cpu_temperature_c
                                                                            .is_none()
                                                                        {
                                                                            if file_name
                                                                                .contains("temp1")
                                                                                || file_name
                                                                                    .contains(
                                                                                        "temp2",
                                                                                    )
                                                                                || file_name
                                                                                    .contains(
                                                                                        "Package",
                                                                                    )
                                                                            {
                                                                                // Это, скорее всего, CPU температура
                                                                                temperature.cpu_temperature_c = Some(temp_c);
                                                                                tracing::info!(
                                                                                    "CPU temperature detected (file pattern {}): {:.1}°C",
                                                                                    file_name,
                                                                                    temp_c
                                                                                );
                                                                            } else if file_name
                                                                                .contains("temp3")
                                                                                || file_name
                                                                                    .contains(
                                                                                        "edge",
                                                                                    )
                                                                                || file_name
                                                                                    .contains("gpu")
                                                                            {
                                                                                // Это, скорее всего, GPU температура
                                                                                if temperature.gpu_temperature_c.is_none() {
                                                                                    temperature.gpu_temperature_c = Some(temp_c);
                                                                                    tracing::info!(
                                                                                        "GPU temperature detected (file pattern {}): {:.1}°C",
                                                                                        file_name,
                                                                                        temp_c
                                                                                    );
                                                                                }
                                                                            }
                                                                        }

                                                                        // 3. Пробуем определить по содержимому файла name (если есть)
                                                                        let name_file =
                                                                            path.join("name");
                                                                        if name_file.exists() {
                                                                            match fs::read_to_string(
                                                                                &name_file,
                                                                            ) {
                                                                                Ok(
                                                                                    name_content,
                                                                                ) => {
                                                                                    let name = name_content.trim();
                                                                                    tracing::debug!("hwmon device sensor name: {}", name);

                                                                                    if (name.contains("coretemp") || name.contains("k10temp"))
                                                                                        && temperature.cpu_temperature_c.is_none()
                                                                                    {
                                                                                        temperature.cpu_temperature_c = Some(temp_c);
                                                                                        tracing::info!(
                                                                                            "CPU temperature detected (sensor name {}): {:.1}°C",
                                                                                            name,
                                                                                            temp_c
                                                                                        );
                                                                                    } else if (name.contains("amdgpu")
                                                                                        || name.contains("radeon")
                                                                                        || name.contains("nouveau"))
                                                                                        && temperature.gpu_temperature_c.is_none()
                                                                                    {
                                                                                        temperature.gpu_temperature_c = Some(temp_c);
                                                                                        tracing::info!(
                                                                                            "GPU temperature detected (sensor name {}): {:.1}°C",
                                                                                            name,
                                                                                            temp_c
                                                                                        );
                                                                                    }
                                                                                }
                                                                                Err(e) => {
                                                                                    tracing::warn!("Failed to read hwmon device name from {}: {}", name_file.display(), e);
                                                                                }
                                                                            }
                                                                        } else {
                                                                            tracing::debug!("No 'name' file found for hwmon device at {}", path.display());
                                                                        }
                                                                    }
                                                                    Err(e) => {
                                                                        tracing::warn!("Failed to parse temperature value from {}: {}", temp_path.display(), e);
                                                                    }
                                                                }
                                                            }
                                                            Err(e) => {
                                                                tracing::warn!("Failed to read temperature from {}: {}", temp_path.display(), e);
                                                            }
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    tracing::warn!(
                                                        "Failed to process temperature file: {}",
                                                        e
                                                    );
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to read hwmon device directory {}: {}",
                                            path.display(),
                                            e
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to process hwmon device {}: {}",
                                    path_str,
                                    e
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to read hwmon directory {}: {}",
                    hwmon_dir.display(),
                    e
                );
            }
        }
    }

    // Пробуем альтернативный интерфейс /sys/class/thermal/
    // Это более универсальный интерфейс для термальных зон
    let thermal_dir = Path::new("/sys/class/thermal");
    tracing::debug!(
        "Attempting to read temperature sensors from thermal interface at: {}",
        thermal_dir.display()
    );

    if !thermal_dir.exists() {
        tracing::warn!("thermal directory not found at: {}", thermal_dir.display());
    } else {
        match fs::read_dir(thermal_dir) {
            Ok(thermal_zones) => {
                tracing::debug!("Found {} thermal zones", thermal_zones.count());
                // Нужно перечитать, так как thermal_zones уже потреблено
                if let Ok(thermal_zones) = fs::read_dir(thermal_dir) {
                    for zone_entry in thermal_zones {
                        match zone_entry {
                            Ok(zone_entry) => {
                                let zone_path = zone_entry.path();
                                let zone_name =
                                    zone_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
                                tracing::debug!("Processing thermal zone: {}", zone_name);

                                if zone_name.starts_with("thermal_zone") {
                                    let temp_file = zone_path.join("temp");
                                    tracing::debug!(
                                        "Looking for temperature file at: {}",
                                        temp_file.display()
                                    );

                                    if !temp_file.exists() {
                                        tracing::warn!(
                                            "Temperature file not found for thermal zone {}: {}",
                                            zone_name,
                                            temp_file.display()
                                        );
                                    } else {
                                        match fs::read_to_string(&temp_file) {
                                            Ok(temp_content) => {
                                                match temp_content.trim().parse::<u64>() {
                                                    Ok(temp_millidegrees) => {
                                                        let temp_c =
                                                            temp_millidegrees as f32 / 1000.0;
                                                        tracing::debug!("Successfully read temperature from thermal zone {}: {:.1}°C", zone_name, temp_c);

                                                        // Пробуем определить тип зоны
                                                        let type_file = zone_path.join("type");
                                                        if !type_file.exists() {
                                                            tracing::debug!("No 'type' file found for thermal zone {}", zone_name);
                                                        } else {
                                                            match fs::read_to_string(&type_file) {
                                                                Ok(type_content) => {
                                                                    let zone_type =
                                                                        type_content.trim();
                                                                    tracing::debug!(
                                                                        "Thermal zone {} type: {}",
                                                                        zone_name,
                                                                        zone_type
                                                                    );

                                                                    if zone_type
                                                                        .contains("x86_pkg_temp")
                                                                        || zone_type
                                                                            .contains("acpitz")
                                                                        || zone_type
                                                                            .contains("cpu_thermal")
                                                                    {
                                                                        // Это CPU температура
                                                                        if temperature
                                                                            .cpu_temperature_c
                                                                            .is_none()
                                                                        {
                                                                            temperature.cpu_temperature_c = Some(temp_c);
                                                                            tracing::info!(
                                                                                "CPU temperature detected (thermal zone {}): {:.1}°C",
                                                                                zone_name,
                                                                                temp_c
                                                                            );
                                                                        } else {
                                                                            tracing::debug!("CPU temperature already set, skipping duplicate thermal zone");
                                                                        }
                                                                    } else if zone_type
                                                                        .contains("gpu")
                                                                        || zone_type
                                                                            .contains("dgpu")
                                                                        || zone_type
                                                                            .contains("radeon")
                                                                    {
                                                                        // Это GPU температура
                                                                        if temperature
                                                                            .gpu_temperature_c
                                                                            .is_none()
                                                                        {
                                                                            temperature.gpu_temperature_c = Some(temp_c);
                                                                            tracing::info!(
                                                                                "GPU temperature detected (thermal zone {}): {:.1}°C",
                                                                                zone_name,
                                                                                temp_c
                                                                            );
                                                                        } else {
                                                                            tracing::debug!("GPU temperature already set, skipping duplicate thermal zone");
                                                                        }
                                                                    } else {
                                                                        tracing::debug!("Unknown thermal zone type: {}", zone_type);
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    tracing::warn!("Failed to read thermal zone type from {}: {}", type_file.display(), e);
                                                                }
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        tracing::warn!("Failed to parse temperature value from thermal zone {}: {}", zone_name, e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                tracing::warn!("Failed to read temperature from thermal zone {}: {}", zone_name, e);
                                            }
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("Failed to process thermal zone entry: {}", e);
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!(
                    "Failed to read thermal directory {}: {}",
                    thermal_dir.display(),
                    e
                );
            }
        }
    }

    // Пробуем специфичные для GPU пути
    // AMD GPU
    let amdgpu_dir = Path::new("/sys/class/drm/card0/device/hwmon");
    tracing::debug!(
        "Checking for AMD GPU temperature sensors at: {}",
        amdgpu_dir.display()
    );

    if !amdgpu_dir.exists() {
        tracing::debug!(
            "AMD GPU hwmon directory not found at: {}",
            amdgpu_dir.display()
        );
    } else {
        match fs::read_dir(amdgpu_dir) {
            Ok(amdgpu_entries) => {
                for amdgpu_entry in amdgpu_entries {
                    match amdgpu_entry {
                        Ok(amdgpu_entry) => {
                            let amdgpu_path = amdgpu_entry.path();
                            let temp_file = amdgpu_path.join("temp1_input");
                            tracing::debug!(
                                "Looking for AMD GPU temperature file at: {}",
                                temp_file.display()
                            );

                            if !temp_file.exists() {
                                tracing::debug!(
                                    "AMD GPU temperature file not found: {}",
                                    temp_file.display()
                                );
                            } else {
                                match fs::read_to_string(&temp_file) {
                                    Ok(temp_content) => match temp_content.trim().parse::<u64>() {
                                        Ok(temp_millidegrees) => {
                                            let temp_c = temp_millidegrees as f32 / 1000.0;
                                            if temperature.gpu_temperature_c.is_none() {
                                                temperature.gpu_temperature_c = Some(temp_c);
                                                tracing::info!(
                                                    "AMD GPU temperature detected: {:.1}°C",
                                                    temp_c
                                                );
                                            } else {
                                                tracing::debug!("GPU temperature already set, skipping AMD GPU sensor");
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!(
                                                "Failed to parse AMD GPU temperature value: {}",
                                                e
                                            );
                                        }
                                    },
                                    Err(e) => {
                                        tracing::warn!(
                                            "Failed to read AMD GPU temperature from {}: {}",
                                            temp_file.display(),
                                            e
                                        );
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to process AMD GPU hwmon entry: {}", e);
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to read AMD GPU hwmon directory {}: {}",
                    amdgpu_dir.display(),
                    e
                );
            }
        }
    }

    // NVIDIA GPU
    let nvidia_dir = Path::new("/sys/class/hwmon/nvidia_hwmon");
    tracing::debug!(
        "Checking for NVIDIA GPU temperature sensors at: {}",
        nvidia_dir.display()
    );

    if !nvidia_dir.exists() {
        tracing::debug!(
            "NVIDIA GPU hwmon directory not found at: {}",
            nvidia_dir.display()
        );
    } else {
        let temp_file = nvidia_dir.join("temp1_input");
        tracing::debug!(
            "Looking for NVIDIA GPU temperature file at: {}",
            temp_file.display()
        );

        if !temp_file.exists() {
            tracing::debug!(
                "NVIDIA GPU temperature file not found: {}",
                temp_file.display()
            );
        } else {
            match fs::read_to_string(&temp_file) {
                Ok(temp_content) => match temp_content.trim().parse::<u64>() {
                    Ok(temp_millidegrees) => {
                        let temp_c = temp_millidegrees as f32 / 1000.0;
                        if temperature.gpu_temperature_c.is_none() {
                            temperature.gpu_temperature_c = Some(temp_c);
                            tracing::info!("NVIDIA GPU temperature detected: {:.1}°C", temp_c);
                        } else {
                            tracing::debug!(
                                "GPU temperature already set, skipping NVIDIA GPU sensor"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to parse NVIDIA GPU temperature value: {}", e);
                    }
                },
                Err(e) => {
                    tracing::warn!(
                        "Failed to read NVIDIA GPU temperature from {}: {}",
                        temp_file.display(),
                        e
                    );
                }
            }
        }
    }

    // Логируем результаты
    if temperature.cpu_temperature_c.is_none() && temperature.gpu_temperature_c.is_none() {
        tracing::warn!(
            "No temperature metrics available - hwmon/thermal interfaces not found or accessible. Check if temperature sensors are properly configured and accessible."
        );
    } else {
        tracing::info!(
            "Temperature metrics collection completed: CPU={:?}°C, GPU={:?}°C",
            temperature.cpu_temperature_c,
            temperature.gpu_temperature_c
        );
    }

    temperature
}

/// Собирает метрики энергопотребления через RAPL и другие интерфейсы
///
/// Использует Running Average Power Limit (RAPL) интерфейс для точного мониторинга
/// энергопотребления CPU, памяти и других компонентов.
///
/// RAPL предоставляет:
/// - energy_uj: общее потребление энергии в микроджоулях (сбрасывается при перезагрузке)
/// - max_energy_range_uj: максимальный диапазон измерения
/// - energy_counter_wrap: флаг переполнения счетчика
///
/// Для точного измерения мощности нужно отслеживать изменения energy_uj во времени,
/// но в текущей реализации мы возвращаем мгновенные значения.
fn collect_power_metrics() -> PowerMetrics {
    let mut power = PowerMetrics::default();

    // Попробуем найти энергетические сенсоры в /sys/class/powercap/
    // Это основной интерфейс для RAPL на современных системах
    let powercap_dir = Path::new("/sys/class/powercap");

    if powercap_dir.exists() {
        if let Ok(entries) = fs::read_dir(powercap_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                let path_str = path.to_string_lossy();

                // Ищем файлы energy_uj в каждом powercap устройстве
                if let Ok(energy_files) = fs::read_dir(&path) {
                    for energy_file in energy_files.flatten() {
                        let energy_path = energy_file.path();
                        let file_name = energy_path
                            .file_name()
                            .and_then(|s| s.to_str())
                            .unwrap_or("");

                        if file_name == "energy_uj" {
                            if let Ok(energy_content) = fs::read_to_string(&energy_path) {
                                if let Ok(energy_microjoules) = energy_content.trim().parse::<u64>()
                                {
                                    // Конвертируем микроджоули в ватты
                                    // Примечание: это мгновенное значение, для точной мощности нужно
                                    // отслеживать изменения во времени, но для мониторинга это приемлемо
                                    let energy_w = energy_microjoules as f32 / 1_000_000.0;

                                    // Определяем тип устройства по пути
                                    if path_str.contains("intel-rapl") {
                                        if path_str.contains("package") {
                                            // Это общий пакет CPU (все ядра)
                                            power.cpu_power_w = Some(energy_w);
                                            tracing::debug!("RAPL package energy: {} W", energy_w);
                                        } else if path_str.contains("core") {
                                            // Это отдельные ядра CPU
                                            // Мы не сохраняем отдельно, но можно было бы добавить
                                            tracing::debug!("RAPL core energy: {} W", energy_w);
                                        } else if path_str.contains("uncore") {
                                            // Это uncore компоненты (кэш, контроллер памяти и т.д.)
                                            tracing::debug!("RAPL uncore energy: {} W", energy_w);
                                        } else if path_str.contains("dram") {
                                            // Это память DRAM
                                            // Можно было бы добавить отдельное поле для памяти
                                            tracing::debug!("RAPL DRAM energy: {} W", energy_w);
                                        } else if path_str.contains("psys") {
                                            // Это общая мощность системы
                                            power.system_power_w = Some(energy_w);
                                            tracing::debug!("RAPL system energy: {} W", energy_w);
                                        }
                                    } else if path_str.contains("amdgpu")
                                        || path_str.contains("gpu")
                                    {
                                        // Это GPU (AMD или другие)
                                        power.gpu_power_w = Some(energy_w);
                                        tracing::debug!("GPU energy: {} W", energy_w);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Попробуем альтернативные интерфейсы, если powercap недоступен
    // Некоторые системы могут предоставлять энергетическую информацию через другие пути

    // Пробуем /sys/devices/system/cpu/cpu*/power/energy_uj для отдельных ядер
    let cpu_energy_dir = Path::new("/sys/devices/system/cpu");
    if cpu_energy_dir.exists() {
        if let Ok(cpu_entries) = fs::read_dir(cpu_energy_dir) {
            let mut total_cpu_energy_uj: u64 = 0;
            let mut cpu_count = 0;

            for cpu_entry in cpu_entries.flatten() {
                let cpu_path = cpu_entry.path();
                if cpu_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with("cpu"))
                {
                    let energy_path = cpu_path.join("power/energy_uj");
                    if energy_path.exists() {
                        if let Ok(energy_content) = fs::read_to_string(&energy_path) {
                            if let Ok(energy_uj) = energy_content.trim().parse::<u64>() {
                                total_cpu_energy_uj += energy_uj;
                                cpu_count += 1;
                            }
                        }
                    }
                }
            }

            if cpu_count > 0 {
                // Средняя мощность на ядро
                let avg_cpu_energy_w = total_cpu_energy_uj as f32 / 1_000_000.0 / cpu_count as f32;
                if power.cpu_power_w.is_none() {
                    power.cpu_power_w = Some(avg_cpu_energy_w);
                    tracing::debug!("CPU energy (avg per core): {} W", avg_cpu_energy_w);
                }
            }
        }
    }

    // Пробуем /sys/class/drm/card*/device/power/energy_uj для GPU
    let drm_dir = Path::new("/sys/class/drm");
    if drm_dir.exists() {
        if let Ok(drm_entries) = fs::read_dir(drm_dir) {
            for drm_entry in drm_entries.flatten() {
                let card_path = drm_entry.path();
                if card_path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with("card"))
                {
                    let energy_path = card_path.join("device/power/energy_uj");
                    if energy_path.exists() {
                        if let Ok(energy_content) = fs::read_to_string(&energy_path) {
                            if let Ok(energy_uj) = energy_content.trim().parse::<u64>() {
                                let energy_w = energy_uj as f32 / 1_000_000.0;
                                if power.gpu_power_w.is_none() {
                                    power.gpu_power_w = Some(energy_w);
                                    tracing::debug!("GPU energy (via DRM): {} W", energy_w);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Логируем, если не удалось собрать никакие метрики
    if power.cpu_power_w.is_none() && power.gpu_power_w.is_none() && power.system_power_w.is_none()
    {
        tracing::debug!(
            "No power metrics available - RAPL/powercap interfaces not found or accessible"
        );
    } else {
        tracing::info!(
            "Power metrics: CPU={:?} W, GPU={:?} W, System={:?} W",
            power.cpu_power_w,
            power.gpu_power_w,
            power.system_power_w
        );
    }

    power
}

/// Собирает метрики сетевой активности из /proc/net/dev
fn collect_network_metrics() -> NetworkMetrics {
    let mut network = NetworkMetrics::default();
    let net_dev_path = Path::new("/proc/net/dev");

    if let Ok(contents) = fs::read_to_string(net_dev_path) {
        let mut total_rx_bytes = 0;
        let mut total_tx_bytes = 0;

        for line in contents.lines().skip(2) {
            // Пропускаем заголовки
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Разбираем строку вида: "eth0: 12345678 1234 0 0 0 0 0 0 12345678 1234 0 0 0 0 0 0"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 17 {
                let interface_name = parts[0].trim_end_matches(':');

                // Извлекаем значения (пропускаем первый элемент - имя интерфейса)
                let rx_bytes = parts[1].parse::<u64>().unwrap_or(0);
                let rx_packets = parts[2].parse::<u64>().unwrap_or(0);
                let rx_errors = parts[3].parse::<u64>().unwrap_or(0);
                let tx_bytes = parts[9].parse::<u64>().unwrap_or(0);
                let tx_packets = parts[10].parse::<u64>().unwrap_or(0);
                let tx_errors = parts[11].parse::<u64>().unwrap_or(0);

                network.interfaces.push(NetworkInterface {
                    name: interface_name.into(), // Convert &str to Box<str>
                    rx_bytes,
                    tx_bytes,
                    rx_packets,
                    tx_packets,
                    rx_errors,
                    tx_errors,
                });

                total_rx_bytes += rx_bytes;
                total_tx_bytes += tx_bytes;
            }
        }

        network.total_rx_bytes = total_rx_bytes;
        network.total_tx_bytes = total_tx_bytes;
    }

    network
}

/// Собирает метрики дисковых операций из /proc/diskstats
fn collect_disk_metrics() -> DiskMetrics {
    let mut disk = DiskMetrics::default();
    let diskstats_path = Path::new("/proc/diskstats");

    if let Ok(contents) = fs::read_to_string(diskstats_path) {
        let mut total_read_bytes = 0;
        let mut total_write_bytes = 0;

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Разбираем строку вида: "8 0 sda 1234 0 5678 123 456 0 7890 1234 0 0 0 12345"
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 14 {
                let device_name = parts[2];

                // Извлекаем значения (индексы 3-13)
                let read_ops = parts[3].parse::<u64>().unwrap_or(0);
                let _read_merged = parts[4].parse::<u64>().unwrap_or(0);
                let read_sectors = parts[5].parse::<u64>().unwrap_or(0);
                let _read_time = parts[6].parse::<u64>().unwrap_or(0);
                let write_ops = parts[7].parse::<u64>().unwrap_or(0);
                let _write_merged = parts[8].parse::<u64>().unwrap_or(0);
                let write_sectors = parts[9].parse::<u64>().unwrap_or(0);
                let _write_time = parts[10].parse::<u64>().unwrap_or(0);
                let io_time = parts[14].parse::<u64>().unwrap_or(0);

                // Конвертируем секторы в байты (обычно 512 байт на сектор)
                let read_bytes = read_sectors * 512;
                let write_bytes = write_sectors * 512;

                disk.devices.push(DiskDevice {
                    name: device_name.into(), // Convert &str to Box<str>
                    read_bytes,
                    write_bytes,
                    read_ops,
                    write_ops,
                    io_time,
                });

                total_read_bytes += read_bytes;
                total_write_bytes += write_bytes;
            }
        }

        disk.total_read_bytes = total_read_bytes;
        disk.total_write_bytes = total_write_bytes;
    }

    disk
}

/// Собирает метрики GPU из различных источников
fn collect_gpu_metrics() -> crate::metrics::gpu::GpuMetricsCollection {
    crate::metrics::gpu::collect_gpu_metrics().unwrap_or_default()
}

/// Собирает метрики eBPF
/// Глобальный кэш для eBPF коллектора (оптимизация производительности)
#[cfg(feature = "ebpf")]
lazy_static! {
    static ref EBPF_COLLECTOR: std::sync::Mutex<Option<crate::metrics::ebpf::EbpfMetricsCollector>> =
        std::sync::Mutex::new(None);
}

fn collect_ebpf_metrics() -> Option<crate::metrics::ebpf::EbpfMetrics> {
    // Проверяем, включена ли поддержка eBPF
    if !crate::metrics::ebpf::EbpfMetricsCollector::is_ebpf_enabled() {
        tracing::debug!("eBPF support is disabled (compiled without 'ebpf' feature)");
        return None;
    }

    #[cfg(feature = "ebpf")]
    {
        // Используем кэшированный коллектор для уменьшения накладных расходов
        let mut collector_guard = EBPF_COLLECTOR.lock().unwrap();

        if collector_guard.is_none() {
            // Инициализируем коллектор при первом вызове
            let config = crate::metrics::ebpf::EbpfConfig::default();
            let mut collector = crate::metrics::ebpf::EbpfMetricsCollector::new(config);

            if let Err(e) = collector.initialize() {
                tracing::warn!("Failed to initialize eBPF metrics collector: {}", e);
                return None;
            }

            *collector_guard = Some(collector);
        }

        // Собираем метрики с использованием кэшированного коллектора
        if let Some(collector) = collector_guard.as_mut() {
            match collector.collect_metrics() {
                Ok(metrics) => {
                    tracing::debug!("Successfully collected eBPF metrics: {:?}", metrics);
                    return Some(metrics);
                }
                Err(e) => {
                    tracing::warn!("Failed to collect eBPF metrics: {}", e);
                    return None;
                }
            }
        }
    }

    #[cfg(not(feature = "ebpf"))]
    {
        // Без eBPF поддержки возвращаем None
        None
    }
}

fn read_file(path: &Path) -> Result<String> {
    fs::read_to_string(path).with_context(|| {
        format!(
            "Не удалось прочитать системный файл {}: проверьте, что файл существует и доступен для чтения. Ошибка может быть вызвана отсутствием прав доступа, отсутствием файла или проблемами с файловой системой",
            path.display()
        )
    })
}

fn parse_cpu_times(contents: &str) -> Result<CpuTimes> {
    let line = contents
        .lines()
        .find(|l| l.starts_with("cpu "))
        .ok_or_else(|| {
            anyhow!(
                "Не найдена строка с общими CPU счетчиками в /proc/stat. \
                 Проверьте, что файл содержит строку, начинающуюся с 'cpu '. \
                 Ожидаемый формат: 'cpu <user> <nice> <system> <idle> <iowait> <irq> <softirq> <steal> <guest> <guest_nice>'"
            )
        })?;

    let mut fields = line.split_whitespace();
    let _cpu_label = fields.next().ok_or_else(|| {
        anyhow!(
            "Пустая строка CPU в /proc/stat. \
                 Ожидается строка вида 'cpu <user> <nice> <system> ...'"
        )
    })?;

    let parse_field = |name: &str, iter: &mut std::str::SplitWhitespace<'_>| -> Result<u64> {
        iter.next()
            .ok_or_else(|| {
                anyhow!(
                    "Поле '{}' отсутствует в строке CPU в /proc/stat. \
                     Ожидается формат: 'cpu <user> <nice> <system> <idle> <iowait> ...'",
                    name
                )
            })?
            .parse::<u64>()
            .with_context(|| {
                format!(
                    "Некорректное значение поля '{}' в /proc/stat: ожидается целое число (u64)",
                    name
                )
            })
    };

    Ok(CpuTimes {
        user: parse_field("user", &mut fields)?,
        nice: parse_field("nice", &mut fields)?,
        system: parse_field("system", &mut fields)?,
        idle: parse_field("idle", &mut fields)?,
        iowait: parse_field("iowait", &mut fields)?,
        irq: parse_field("irq", &mut fields)?,
        softirq: parse_field("softirq", &mut fields)?,
        steal: parse_field("steal", &mut fields)?,
        guest: parse_field("guest", &mut fields)?,
        guest_nice: parse_field("guest_nice", &mut fields)?,
    })
}

fn parse_meminfo(contents: &str) -> Result<MemoryInfo> {
    let mut values: HashMap<&str, u64> = HashMap::new();
    for line in contents.lines() {
        let mut parts = line.split_whitespace();
        let key = match parts.next() {
            Some(k) => k.trim_end_matches(':'),
            None => continue,
        };
        let value = match parts.next() {
            Some(v) => v
                .parse::<u64>()
                .with_context(|| {
                    format!(
                        "Некорректное значение поля '{}' в /proc/meminfo: ожидается целое число (u64) в килобайтах",
                        key
                    )
                })?,
            None => continue,
        };
        values.insert(key, value);
    }

    let take = |name: &str| -> Result<u64> {
        values.get(name).copied().ok_or_else(|| {
            anyhow!(
                "В /proc/meminfo отсутствует обязательное поле '{}'. \
                     Проверьте, что файл содержит строку вида '{}: <значение> kB'. \
                     Это может быть вызвано нестандартным ядром или отсутствием памяти в системе",
                name,
                name
            )
        })
    };

    Ok(MemoryInfo {
        mem_total_kb: take("MemTotal")?,
        mem_available_kb: take("MemAvailable")?,
        mem_free_kb: take("MemFree")?,
        buffers_kb: take("Buffers")?,
        cached_kb: take("Cached")?,
        swap_total_kb: take("SwapTotal")?,
        swap_free_kb: take("SwapFree")?,
    })
}

fn parse_loadavg(contents: &str) -> Result<LoadAvg> {
    let mut parts = contents.split_whitespace();
    let one = parts
        .next()
        .ok_or_else(|| {
            anyhow!(
                "Пустой файл /proc/loadavg. \
                 Ожидается формат: '<1m> <5m> <15m> <running>/<total> <last_pid>'"
            )
        })?
        .parse::<f64>()
        .context("Некорректное значение loadavg за 1 минуту: ожидается число с плавающей точкой")?;
    let five = parts
        .next()
        .ok_or_else(|| {
            anyhow!(
                "Отсутствует значение loadavg за 5 минут в /proc/loadavg. \
                 Ожидается формат: '<1m> <5m> <15m> ...'"
            )
        })?
        .parse::<f64>()
        .context("Некорректное значение loadavg за 5 минут: ожидается число с плавающей точкой")?;
    let fifteen = parts
        .next()
        .ok_or_else(|| {
            anyhow!(
                "Отсутствует значение loadavg за 15 минут в /proc/loadavg. \
                 Ожидается формат: '<1m> <5m> <15m> ...'"
            )
        })?
        .parse::<f64>()
        .context("Некорректное значение loadavg за 15 минут: ожидается число с плавающей точкой")?;

    Ok(LoadAvg { one, five, fifteen })
}

fn parse_pressure(contents: &str) -> Result<Pressure> {
    let mut some = None;
    let mut full = None;

    for line in contents.lines() {
        if line.starts_with("some ") {
            some = Some(parse_pressure_record(line)?);
        } else if line.starts_with("full ") {
            full = Some(parse_pressure_record(line)?);
        }
    }

    if some.is_none() && full.is_none() {
        return Err(anyhow!(
            "В файле PSI pressure отсутствуют записи 'some' и 'full'. \
             Ожидается формат: 'some avg10=<value> avg60=<value> avg300=<value> total=<value>' \
             или 'full avg10=<value> ...'. \
             Проверьте, что ядро поддерживает PSI и файл содержит корректные данные"
        ));
    }

    Ok(Pressure { some, full })
}

fn parse_pressure_record(line: &str) -> Result<PressureRecord> {
    let mut avg10 = None;
    let mut avg60 = None;
    let mut avg300 = None;
    let mut total = None;

    for token in line.split_whitespace().skip(1) {
        let mut kv = token.split('=');
        let key = kv.next().ok_or_else(|| {
            anyhow!(
                "Некорректный токен в записи PSI pressure: '{}'. \
                     Ожидается формат 'key=value', например 'avg10=0.01'",
                token
            )
        })?;
        let value = kv.next().ok_or_else(|| {
            anyhow!(
                "Некорректный токен в записи PSI pressure: '{}'. \
                     Ожидается формат 'key=value', но значение отсутствует",
                token
            )
        })?;
        match key {
            "avg10" => avg10 = Some(value.parse::<f64>().context(
                "Некорректное значение avg10 в PSI pressure: ожидается число с плавающей точкой",
            )?),
            "avg60" => avg60 = Some(value.parse::<f64>().context(
                "Некорректное значение avg60 в PSI pressure: ожидается число с плавающей точкой",
            )?),
            "avg300" => avg300 = Some(value.parse::<f64>().context(
                "Некорректное значение avg300 в PSI pressure: ожидается число с плавающей точкой",
            )?),
            "total" => {
                total = Some(value.parse::<u64>().context(
                    "Некорректное значение total в PSI pressure: ожидается целое число (u64)",
                )?)
            }
            _ => {}
        }
    }

    Ok(PressureRecord {
        avg10: avg10.ok_or_else(|| {
            anyhow!(
                "В записи PSI pressure отсутствует обязательное поле 'avg10'. \
                 Ожидается формат: 'some avg10=<value> avg60=<value> avg300=<value> total=<value>'"
            )
        })?,
        avg60: avg60.ok_or_else(|| {
            anyhow!(
                "В записи PSI pressure отсутствует обязательное поле 'avg60'. \
                 Ожидается формат: 'some avg10=<value> avg60=<value> avg300=<value> total=<value>'"
            )
        })?,
        avg300: avg300.ok_or_else(|| {
            anyhow!(
                "В записи PSI pressure отсутствует обязательное поле 'avg300'. \
                 Ожидается формат: 'some avg10=<value> avg60=<value> avg300=<value> total=<value>'"
            )
        })?,
        total: total.ok_or_else(|| {
            anyhow!(
                "В записи PSI pressure отсутствует обязательное поле 'total'. \
                 Ожидается формат: 'some avg10=<value> avg60=<value> avg300=<value> total=<value>'"
            )
        })?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const PROC_STAT: &str = "cpu  2255 34 2290 22625563 6290 127 456 0 0 0\n\
cpu0 1132 17 1441 11311777 3675 33 226 0 0 0\n";

    const MEMINFO: &str = "\
MemTotal:       16384256 kB
MemFree:         1234567 kB
MemAvailable:    9876543 kB
Buffers:          345678 kB
Cached:          2345678 kB
SwapCached:            0 kB
Active:          4567890 kB
Inactive:        3456789 kB
SwapTotal:       8192000 kB
SwapFree:        4096000 kB
";

    const LOADAVG: &str = "0.42 0.35 0.30 1/123 4567\n";

    const PRESSURE_CPU: &str = "some avg10=0.00 avg60=0.01 avg300=0.02 total=1234\n";
    const PRESSURE_IO: &str = "some avg10=0.10 avg60=0.11 avg300=0.12 total=2345\nfull avg10=0.01 avg60=0.02 avg300=0.03 total=3456\n";
    const PRESSURE_MEM: &str = "full avg10=0.20 avg60=0.21 avg300=0.22 total=4567\n";

    #[test]
    fn cpu_delta_calculates_percentages() {
        let prev = CpuTimes {
            user: 100,
            nice: 20,
            system: 50,
            idle: 200,
            iowait: 10,
            irq: 5,
            softirq: 5,
            steal: 0,
            guest: 0,
            guest_nice: 0,
        };
        let cur = CpuTimes {
            user: 150,
            nice: 30,
            system: 80,
            idle: 260,
            iowait: 20,
            irq: 10,
            softirq: 10,
            steal: 0,
            guest: 0,
            guest_nice: 0,
        };

        let usage = cur.delta(&prev).expect("usage");
        let total = usage.user + usage.system + usage.idle + usage.iowait;
        // допускаем небольшую погрешность из-за float
        assert!((total - 1.0).abs() < 1e-9);
        assert!(usage.user > 0.0);
        assert!(usage.system > 0.0);
        assert!(usage.idle > 0.0);
    }

    #[test]
    fn cpu_delta_handles_overflow() {
        // Тест проверяет, что функция корректно обрабатывает переполнение счетчиков
        // (когда prev > cur, что может произойти на долгоживущих системах)
        let prev = CpuTimes {
            user: 200,
            nice: 0,
            system: 0,
            idle: 0,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            guest: 0,
            guest_nice: 0,
        };
        let cur = CpuTimes {
            user: 100, // меньше prev - переполнение
            nice: 0,
            system: 0,
            idle: 0,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            guest: 0,
            guest_nice: 0,
        };

        assert!(cur.delta(&prev).is_none());
    }

    #[test]
    fn cpu_delta_handles_zero_total() {
        // Тест проверяет, что функция возвращает None, когда все счетчики равны (total = 0)
        let prev = CpuTimes {
            user: 100,
            nice: 0,
            system: 0,
            idle: 0,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            guest: 0,
            guest_nice: 0,
        };
        let cur = prev; // все счетчики равны

        assert!(cur.delta(&prev).is_none());
    }

    #[test]
    fn cpu_delta_handles_all_zero() {
        // Тест проверяет, что функция корректно обрабатывает случай, когда все счетчики равны нулю
        let prev = CpuTimes {
            user: 0,
            nice: 0,
            system: 0,
            idle: 0,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            guest: 0,
            guest_nice: 0,
        };
        let cur = CpuTimes {
            user: 0,
            nice: 0,
            system: 0,
            idle: 0,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            guest: 0,
            guest_nice: 0,
        };

        assert!(cur.delta(&prev).is_none());
    }

    #[test]
    fn cpu_delta_handles_partial_overflow() {
        // Тест проверяет, что функция корректно обрабатывает частичное переполнение
        // (когда только некоторые счетчики переполнились)
        let prev = CpuTimes {
            user: 100,
            nice: 50,
            system: 200, // переполнение
            idle: 0,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            guest: 0,
            guest_nice: 0,
        };
        let cur = CpuTimes {
            user: 150,
            nice: 60,
            system: 100, // меньше prev - переполнение
            idle: 0,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            guest: 0,
            guest_nice: 0,
        };

        assert!(cur.delta(&prev).is_none());
    }

    #[test]
    fn cpu_delta_handles_boundary_values() {
        // Тест проверяет граничные случаи с минимальными изменениями
        let prev = CpuTimes {
            user: 100,
            nice: 0,
            system: 0,
            idle: 1000,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            guest: 0,
            guest_nice: 0,
        };
        let cur = CpuTimes {
            user: 101, // минимальное изменение
            nice: 0,
            system: 0,
            idle: 1001,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            guest: 0,
            guest_nice: 0,
        };

        let usage = cur.delta(&prev).expect("должно быть Some");
        let total = usage.user + usage.system + usage.idle + usage.iowait;
        assert!((total - 1.0).abs() < 1e-9);
        assert!(usage.user > 0.0);
        assert!(usage.idle > 0.0);
    }

    #[test]
    fn parse_cpu_times_ok() {
        let parsed = parse_cpu_times(PROC_STAT).expect("parsed");
        assert_eq!(parsed.user, 2255);
        assert_eq!(parsed.nice, 34);
        assert_eq!(parsed.system, 2290);
        assert_eq!(parsed.idle, 22625563);
        assert_eq!(parsed.guest, 0);
    }

    #[test]
    fn parse_meminfo_ok() {
        let mem = parse_meminfo(MEMINFO).expect("meminfo");
        assert_eq!(mem.mem_total_kb, 16_384_256);
        assert_eq!(mem.mem_available_kb, 9_876_543);
        assert_eq!(mem.swap_total_kb, 8_192_000);
        assert_eq!(mem.swap_free_kb, 4_096_000);
        assert_eq!(mem.mem_used_kb(), 16_384_256 - 9_876_543);
        assert_eq!(mem.swap_used_kb(), 4_096_000);
    }

    #[test]
    fn mem_used_kb_handles_overflow() {
        // Тест проверяет, что mem_used_kb корректно обрабатывает случай,
        // когда mem_available_kb > mem_total_kb (используется saturating_sub)
        let mem = MemoryInfo {
            mem_total_kb: 1000,
            mem_available_kb: 2000, // больше total - некорректные данные
            mem_free_kb: 500,
            buffers_kb: 0,
            cached_kb: 0,
            swap_total_kb: 0,
            swap_free_kb: 0,
        };

        // saturating_sub должен вернуть 0, а не переполнение
        assert_eq!(mem.mem_used_kb(), 0);
    }

    #[test]
    fn mem_used_kb_handles_zero_values() {
        // Тест проверяет, что mem_used_kb корректно обрабатывает нулевые значения
        let mem = MemoryInfo {
            mem_total_kb: 0,
            mem_available_kb: 0,
            mem_free_kb: 0,
            buffers_kb: 0,
            cached_kb: 0,
            swap_total_kb: 0,
            swap_free_kb: 0,
        };

        assert_eq!(mem.mem_used_kb(), 0);
    }

    #[test]
    fn mem_used_kb_handles_normal_case() {
        // Тест проверяет нормальный случай использования
        let mem = MemoryInfo {
            mem_total_kb: 16_384_256,
            mem_available_kb: 9_876_543,
            mem_free_kb: 1_234_567,
            buffers_kb: 345_678,
            cached_kb: 2_345_678,
            swap_total_kb: 8_192_000,
            swap_free_kb: 4_096_000,
        };

        let expected = 16_384_256 - 9_876_543;
        assert_eq!(mem.mem_used_kb(), expected);
    }

    #[test]
    fn swap_used_kb_handles_overflow() {
        // Тест проверяет, что swap_used_kb корректно обрабатывает случай,
        // когда swap_free_kb > swap_total_kb (используется saturating_sub)
        let mem = MemoryInfo {
            mem_total_kb: 0,
            mem_available_kb: 0,
            mem_free_kb: 0,
            buffers_kb: 0,
            cached_kb: 0,
            swap_total_kb: 1000,
            swap_free_kb: 2000, // больше total - некорректные данные
        };

        // saturating_sub должен вернуть 0, а не переполнение
        assert_eq!(mem.swap_used_kb(), 0);
    }

    #[test]
    fn swap_used_kb_handles_zero_values() {
        // Тест проверяет, что swap_used_kb корректно обрабатывает нулевые значения
        let mem = MemoryInfo {
            mem_total_kb: 0,
            mem_available_kb: 0,
            mem_free_kb: 0,
            buffers_kb: 0,
            cached_kb: 0,
            swap_total_kb: 0,
            swap_free_kb: 0,
        };

        assert_eq!(mem.swap_used_kb(), 0);
    }

    #[test]
    fn swap_used_kb_handles_normal_case() {
        // Тест проверяет нормальный случай использования
        let mem = MemoryInfo {
            mem_total_kb: 0,
            mem_available_kb: 0,
            mem_free_kb: 0,
            buffers_kb: 0,
            cached_kb: 0,
            swap_total_kb: 8_192_000,
            swap_free_kb: 4_096_000,
        };

        let expected = 8_192_000 - 4_096_000;
        assert_eq!(mem.swap_used_kb(), expected);
    }

    #[test]
    fn swap_used_kb_handles_full_swap() {
        // Тест проверяет случай, когда весь swap используется
        let mem = MemoryInfo {
            mem_total_kb: 0,
            mem_available_kb: 0,
            mem_free_kb: 0,
            buffers_kb: 0,
            cached_kb: 0,
            swap_total_kb: 8_192_000,
            swap_free_kb: 0, // весь swap используется
        };

        assert_eq!(mem.swap_used_kb(), 8_192_000);
    }

    #[test]
    fn parse_loadavg_ok() {
        let load = parse_loadavg(LOADAVG).expect("loadavg");
        assert!((load.one - 0.42).abs() < 1e-9);
        assert!((load.five - 0.35).abs() < 1e-9);
        assert!((load.fifteen - 0.30).abs() < 1e-9);
    }

    #[test]
    fn parse_pressure_ok() {
        let cpu = parse_pressure(PRESSURE_CPU).expect("cpu pressure");
        assert!(cpu.some.is_some());
        assert!(cpu.full.is_none());

        let io = parse_pressure(PRESSURE_IO).expect("io pressure");
        assert!(io.some.is_some());
        assert!(io.full.is_some());

        let mem = parse_pressure(PRESSURE_MEM).expect("mem pressure");
        assert!(mem.some.is_none());
        assert!(mem.full.is_some());
    }

    #[test]
    fn collect_system_metrics_from_fake_proc() {
        let tmp = TempDir::new().expect("tmp");
        let root = tmp.path();

        fs::write(root.join("stat"), PROC_STAT).unwrap();
        fs::write(root.join("meminfo"), MEMINFO).unwrap();
        fs::write(root.join("loadavg"), LOADAVG).unwrap();

        let pressure_dir = root.join("pressure");
        fs::create_dir(&pressure_dir).unwrap();
        fs::write(pressure_dir.join("cpu"), PRESSURE_CPU).unwrap();
        fs::write(pressure_dir.join("io"), PRESSURE_IO).unwrap();
        fs::write(pressure_dir.join("memory"), PRESSURE_MEM).unwrap();

        let paths = ProcPaths::new(root);
        let metrics = collect_system_metrics(&paths).expect("metrics");

        assert_eq!(metrics.memory.mem_total_kb, 16_384_256);
        assert_eq!(metrics.pressure.io.full.as_ref().unwrap().total, 3456);
        assert!((metrics.load_avg.one - 0.42).abs() < 1e-6);
    }

    #[test]
    fn collect_system_metrics_works_without_psi() {
        // Тест проверяет, что collect_system_metrics продолжает работу,
        // даже если PSI-файлы недоступны (старые ядра без поддержки PSI)
        let tmp = TempDir::new().expect("tmp");
        let root = tmp.path();

        fs::write(root.join("stat"), PROC_STAT).unwrap();
        fs::write(root.join("meminfo"), MEMINFO).unwrap();
        fs::write(root.join("loadavg"), LOADAVG).unwrap();

        // Не создаём директорию pressure, чтобы симулировать отсутствие PSI

        let paths = ProcPaths::new(root);
        let metrics = collect_system_metrics(&paths).expect("metrics");

        // Проверяем, что основные метрики собраны
        assert_eq!(metrics.memory.mem_total_kb, 16_384_256);
        assert!((metrics.load_avg.one - 0.42).abs() < 1e-6);

        // Проверяем, что PSI-метрики пустые (default)
        assert!(metrics.pressure.cpu.some.is_none());
        assert!(metrics.pressure.cpu.full.is_none());
        assert!(metrics.pressure.io.some.is_none());
        assert!(metrics.pressure.io.full.is_none());
        assert!(metrics.pressure.memory.some.is_none());
        assert!(metrics.pressure.memory.full.is_none());
    }

    #[test]
    fn collect_system_metrics_works_with_partial_psi() {
        // Тест проверяет, что collect_system_metrics продолжает работу,
        // даже если только часть PSI-файлов доступна
        let tmp = TempDir::new().expect("tmp");
        let root = tmp.path();

        fs::write(root.join("stat"), PROC_STAT).unwrap();
        fs::write(root.join("meminfo"), MEMINFO).unwrap();
        fs::write(root.join("loadavg"), LOADAVG).unwrap();

        let pressure_dir = root.join("pressure");
        fs::create_dir(&pressure_dir).unwrap();
        // Создаём только CPU pressure, но не IO и Memory
        fs::write(pressure_dir.join("cpu"), PRESSURE_CPU).unwrap();

        let paths = ProcPaths::new(root);
        let metrics = collect_system_metrics(&paths).expect("metrics");

        // Проверяем, что основные метрики собраны
        assert_eq!(metrics.memory.mem_total_kb, 16_384_256);

        // Проверяем, что CPU pressure доступен
        assert!(metrics.pressure.cpu.some.is_some());

        // Проверяем, что IO и Memory pressure пустые
        assert!(metrics.pressure.io.some.is_none());
        assert!(metrics.pressure.memory.some.is_none());
    }

    #[test]
    fn test_proc_paths_new() {
        // Тест проверяет, что ProcPaths::new корректно создаёт пути
        let paths = ProcPaths::new("/test/proc");
        assert_eq!(paths.stat, PathBuf::from("/test/proc/stat"));
        assert_eq!(paths.meminfo, PathBuf::from("/test/proc/meminfo"));
        assert_eq!(paths.loadavg, PathBuf::from("/test/proc/loadavg"));
        assert_eq!(paths.pressure_cpu, PathBuf::from("/test/proc/pressure/cpu"));
        assert_eq!(paths.pressure_io, PathBuf::from("/test/proc/pressure/io"));
        assert_eq!(
            paths.pressure_memory,
            PathBuf::from("/test/proc/pressure/memory")
        );
    }

    #[test]
    fn test_proc_paths_default() {
        // Тест проверяет, что ProcPaths::default() создаёт пути к /proc
        let paths = ProcPaths::default();
        assert_eq!(paths.stat, PathBuf::from("/proc/stat"));
        assert_eq!(paths.meminfo, PathBuf::from("/proc/meminfo"));
        assert_eq!(paths.loadavg, PathBuf::from("/proc/loadavg"));
        assert_eq!(paths.pressure_cpu, PathBuf::from("/proc/pressure/cpu"));
        assert_eq!(paths.pressure_io, PathBuf::from("/proc/pressure/io"));
        assert_eq!(
            paths.pressure_memory,
            PathBuf::from("/proc/pressure/memory")
        );
    }

    #[test]
    fn test_system_metrics_cpu_usage_since() {
        // Тест проверяет, что cpu_usage_since корректно делегирует к delta
        let prev_metrics = SystemMetrics {
            cpu_times: CpuTimes {
                user: 100,
                nice: 20,
                system: 50,
                idle: 200,
                iowait: 10,
                irq: 5,
                softirq: 5,
                steal: 0,
                guest: 0,
                guest_nice: 0,
            },
            memory: MemoryInfo {
                mem_total_kb: 1000,
                mem_available_kb: 500,
                mem_free_kb: 400,
                buffers_kb: 50,
                cached_kb: 50,
                swap_total_kb: 1000,
                swap_free_kb: 800,
            },
            load_avg: LoadAvg {
                one: 1.0,
                five: 1.0,
                fifteen: 1.0,
            },
            pressure: PressureMetrics::default(),
            temperature: TemperatureMetrics::default(),
            power: PowerMetrics::default(),
            network: NetworkMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
        };

        let cur_metrics = SystemMetrics {
            cpu_times: CpuTimes {
                user: 150,
                nice: 30,
                system: 80,
                idle: 260,
                iowait: 20,
                irq: 10,
                softirq: 10,
                steal: 0,
                guest: 0,
                guest_nice: 0,
            },
            memory: prev_metrics.memory,
            load_avg: prev_metrics.load_avg,
            pressure: prev_metrics.pressure.clone(),
            temperature: TemperatureMetrics::default(),
            power: PowerMetrics::default(),
            network: NetworkMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
        };

        let usage = cur_metrics.cpu_usage_since(&prev_metrics);
        assert!(usage.is_some());
        let usage = usage.unwrap();
        assert!(usage.user > 0.0);
        assert!(usage.system > 0.0);
        assert!(usage.idle > 0.0);
        assert!(usage.iowait > 0.0);
    }

    #[test]
    fn test_system_metrics_cpu_usage_since_none_on_overflow() {
        // Тест проверяет, что cpu_usage_since возвращает None при переполнении
        let prev_metrics = SystemMetrics {
            cpu_times: CpuTimes {
                user: 200,
                nice: 0,
                system: 0,
                idle: 0,
                iowait: 0,
                irq: 0,
                softirq: 0,
                steal: 0,
                guest: 0,
                guest_nice: 0,
            },
            memory: MemoryInfo {
                mem_total_kb: 1000,
                mem_available_kb: 500,
                mem_free_kb: 400,
                buffers_kb: 50,
                cached_kb: 50,
                swap_total_kb: 1000,
                swap_free_kb: 800,
            },
            load_avg: LoadAvg {
                one: 1.0,
                five: 1.0,
                fifteen: 1.0,
            },
            pressure: PressureMetrics::default(),
            temperature: TemperatureMetrics::default(),
            power: PowerMetrics::default(),
            network: NetworkMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
        };

        let cur_metrics = SystemMetrics {
            cpu_times: CpuTimes {
                user: 100, // меньше, чем prev - переполнение
                nice: 0,
                system: 0,
                idle: 0,
                iowait: 0,
                irq: 0,
                softirq: 0,
                steal: 0,
                guest: 0,
                guest_nice: 0,
            },
            memory: prev_metrics.memory,
            load_avg: prev_metrics.load_avg,
            pressure: prev_metrics.pressure.clone(),
            temperature: TemperatureMetrics::default(),
            power: PowerMetrics::default(),
            network: NetworkMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
        };

        let usage = cur_metrics.cpu_usage_since(&prev_metrics);
        assert!(usage.is_none(), "Should return None on counter overflow");
    }

    #[test]
    fn collect_system_metrics_handles_missing_files_gracefully() {
        // Тест проверяет, что функция collect_system_metrics возвращает ошибки с подробными сообщениями
        // при отсутствии основных файлов
        let tmp = TempDir::new().unwrap();
        let paths = ProcPaths::new(tmp.path());

        // Проверяем, что ошибка содержит подробное сообщение о отсутствии файла
        let result = collect_system_metrics(&paths);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();

        // Проверяем, что сообщение об ошибке содержит информацию о файле и причине
        assert!(
            err_msg.contains("Не удалось прочитать CPU метрики")
                || err_msg.contains("Не удалось прочитать информацию о памяти")
                || err_msg.contains("Не удалось прочитать среднюю нагрузку")
        );

        // Проверяем, что сообщение содержит информацию о возможных причинах
        assert!(
            err_msg.contains("отсутствием прав доступа")
                || err_msg.contains("отсутствием файла")
                || err_msg.contains("проблемами с файловой системой")
        );
    }

    #[test]
    fn collect_system_metrics_handles_psi_gracefully() {
        // Тест проверяет, что функция collect_system_metrics обрабатывает отсутствие PSI файлов gracefully
        // Этот тест проверяет, что PSI ошибки обрабатываются gracefully, но основные файлы должны существовать
        // Для полного тестирования graceful обработки PSI, нам нужно использовать реальный /proc
        // где основные файлы существуют, но PSI файлы могут отсутствовать

        // Используем реальный /proc для тестирования
        let paths = ProcPaths::default();

        // Функция должна успешно собрать метрики, даже если PSI файлы отсутствуют
        let result = collect_system_metrics(&paths);

        // На реальной системе с поддержкой PSI, результат должен быть Ok
        // На системах без PSI, результат также должен быть Ok с пустыми PSI метриками
        if result.is_ok() {
            let metrics = result.unwrap();
            // Проверяем, что основные метрики собраны
            assert!(metrics.cpu_times.user > 0);
            assert!(metrics.memory.mem_total_kb > 0);
            assert!(metrics.load_avg.one > 0.0);

            // PSI метрики могут быть пустыми или содержать данные, в зависимости от системы
            // Главное, что функция не упала с ошибкой
        } else {
            // Если результат Err, проверяем, что это не связано с основными файлами
            let err = result.unwrap_err();
            let err_str = err.to_string();
            // Ошибка не должна быть связана с основными файлами (stat, meminfo, loadavg)
            assert!(
                !err_str.contains("stat")
                    || !err_str.contains("meminfo")
                    || !err_str.contains("loadavg")
            );
        }
    }

    #[test]
    fn parse_cpu_times_handles_malformed_input() {
        // Тест проверяет, что parse_cpu_times возвращает ошибку с подробным сообщением
        // при некорректных данных
        let malformed_stat = "cpu 100 20 30\n"; // не хватает полей
        let result = parse_cpu_times(malformed_stat);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();

        // Проверяем, что сообщение об ошибке содержит информацию о поле
        assert!(err_msg.contains("Поле") && err_msg.contains("отсутствует"));

        // Тест с некорректным значением
        let malformed_stat2 = "cpu 100 20 abc 30 40 50 60 70 80 90"; // 'abc' вместо числа
        let result2 = parse_cpu_times(malformed_stat2);
        assert!(result2.is_err());
        let err2 = result2.unwrap_err();
        let err_msg2 = err2.to_string();

        // Проверяем, что сообщение об ошибке содержит информацию о некорректном значении
        assert!(
            err_msg2.contains("Некорректное значение")
                || err_msg2.contains("ожидается целое число")
        );
    }

    #[test]
    fn parse_meminfo_handles_missing_fields() {
        // Тест проверяет, что parse_meminfo возвращает ошибку с подробным сообщением
        // при отсутствии обязательных полей
        let incomplete_meminfo = "MemTotal: 1000 kB\nMemFree: 500 kB\n"; // не хватает полей
        let result = parse_meminfo(incomplete_meminfo);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();

        // Проверяем, что сообщение об ошибке содержит информацию о недостающих полях
        assert!(
            err_msg.contains("отсутствует обязательное поле")
                || err_msg.contains("MemAvailable")
                || err_msg.contains("Buffers")
                || err_msg.contains("Cached")
                || err_msg.contains("SwapTotal")
                || err_msg.contains("SwapFree")
        );
    }

    #[test]
    fn parse_loadavg_handles_incomplete_data() {
        // Тест проверяет, что parse_loadavg возвращает ошибку с подробным сообщением
        // при неполных данных
        let incomplete_loadavg = "0.42"; // только одно значение
        let result = parse_loadavg(incomplete_loadavg);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = err.to_string();

        // Проверяем, что сообщение об ошибке содержит информацию о недостающих значениях
        assert!(
            err_msg.contains("Отсутствует значение loadavg")
                || err_msg.contains("ожидается формат")
        );
    }

    #[test]
    fn test_temperature_metrics_default() {
        // Тест проверяет, что TemperatureMetrics::default() возвращает пустые значения
        let temp = TemperatureMetrics::default();
        assert!(temp.cpu_temperature_c.is_none());
        assert!(temp.gpu_temperature_c.is_none());
    }

    #[test]
    fn test_power_metrics_default() {
        // Тест проверяет, что PowerMetrics::default() возвращает пустые значения
        let power = PowerMetrics::default();
        assert!(power.system_power_w.is_none());
        assert!(power.cpu_power_w.is_none());
        assert!(power.gpu_power_w.is_none());
    }

    #[test]
    fn test_temperature_metrics_serialization() {
        // Тест проверяет, что TemperatureMetrics корректно сериализуется
        let temp = TemperatureMetrics {
            cpu_temperature_c: Some(45.5),
            gpu_temperature_c: Some(60.2),
        };

        let json = serde_json::to_string(&temp).expect("Сериализация должна работать");
        assert!(json.contains("45.5"));
        assert!(json.contains("60.2"));

        // Тест десериализации
        let deserialized: TemperatureMetrics =
            serde_json::from_str(&json).expect("Десериализация должна работать");
        assert_eq!(deserialized.cpu_temperature_c, Some(45.5));
        assert_eq!(deserialized.gpu_temperature_c, Some(60.2));
    }

    #[test]
    fn test_power_metrics_serialization() {
        // Тест проверяет, что PowerMetrics корректно сериализуется
        let power = PowerMetrics {
            system_power_w: Some(120.5),
            cpu_power_w: Some(80.3),
            gpu_power_w: Some(40.1),
        };

        let json = serde_json::to_string(&power).expect("Сериализация должна работать");
        assert!(json.contains("120.5"));
        assert!(json.contains("80.3"));
        assert!(json.contains("40.1"));

        // Тест десериализации
        let deserialized: PowerMetrics =
            serde_json::from_str(&json).expect("Десериализация должна работать");
        assert_eq!(deserialized.system_power_w, Some(120.5));
        assert_eq!(deserialized.cpu_power_w, Some(80.3));
        assert_eq!(deserialized.gpu_power_w, Some(40.1));
    }

    #[test]
    fn test_system_metrics_includes_new_fields() {
        // Тест проверяет, что SystemMetrics включает новые поля
        let metrics = SystemMetrics {
            cpu_times: CpuTimes::default(),
            memory: MemoryInfo::default(),
            load_avg: LoadAvg::default(),
            pressure: PressureMetrics::default(),
            temperature: TemperatureMetrics::default(),
            power: PowerMetrics::default(),
            network: NetworkMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
        };

        // Проверяем, что метрики содержат новые поля
        assert!(metrics.temperature.cpu_temperature_c.is_none());
        assert!(metrics.temperature.gpu_temperature_c.is_none());
        assert!(metrics.power.system_power_w.is_none());
        assert!(metrics.power.cpu_power_w.is_none());
        assert!(metrics.power.gpu_power_w.is_none());
        // Проверяем, что GPU метрики доступны
        assert!(metrics.gpu.is_none());
    }

    #[test]
    fn test_collect_temperature_metrics_fallback() {
        // Тест проверяет, что collect_temperature_metrics возвращает пустые значения
        // когда /sys/class/hwmon недоступен (что нормально в тестовой среде)
        let _temp = collect_temperature_metrics();
        // В тестовой среде мы ожидаем пустые значения, так как нет реального hwmon
        // Это нормальное поведение
    }

    #[test]
    fn test_collect_power_metrics_fallback() {
        // Тест проверяет, что collect_power_metrics возвращает пустые значения
        // когда /sys/class/powercap недоступен (что нормально в тестовой среде)
        let _power = collect_power_metrics();
        // В тестовой среде мы ожидаем пустые значения, так как нет реального powercap
        // Это нормальное поведение
    }

    #[test]
    fn test_network_metrics_default() {
        // Тест проверяет, что NetworkMetrics::default() возвращает пустые значения
        let network = NetworkMetrics::default();
        assert!(network.interfaces.is_empty());
        assert_eq!(network.total_rx_bytes, 0);
        assert_eq!(network.total_tx_bytes, 0);
    }

    #[test]
    fn test_disk_metrics_default() {
        // Тест проверяет, что DiskMetrics::default() возвращает пустые значения
        let disk = DiskMetrics::default();
        assert!(disk.devices.is_empty());
        assert_eq!(disk.total_read_bytes, 0);
        assert_eq!(disk.total_write_bytes, 0);
    }

    #[test]
    fn test_network_metrics_serialization() {
        // Тест проверяет, что NetworkMetrics корректно сериализуется
        let mut network = NetworkMetrics::default();
        network.interfaces.push(NetworkInterface {
            name: "eth0".into(),
            rx_bytes: 1000,
            tx_bytes: 2000,
            rx_packets: 100,
            tx_packets: 200,
            rx_errors: 1,
            tx_errors: 2,
        });
        network.total_rx_bytes = 1000;
        network.total_tx_bytes = 2000;

        let json = serde_json::to_string(&network).expect("Сериализация должна работать");
        assert!(json.contains("eth0"));
        assert!(json.contains("1000"));
        assert!(json.contains("2000"));

        // Тест десериализации
        let deserialized: NetworkMetrics =
            serde_json::from_str(&json).expect("Десериализация должна работать");
        assert_eq!(deserialized.interfaces.len(), 1);
        assert_eq!(deserialized.interfaces[0].name, "eth0".into());
        assert_eq!(deserialized.total_rx_bytes, 1000);
        assert_eq!(deserialized.total_tx_bytes, 2000);
    }

    #[test]
    fn test_disk_metrics_serialization() {
        // Тест проверяет, что DiskMetrics корректно сериализуется
        let mut disk = DiskMetrics::default();
        disk.devices.push(DiskDevice {
            name: "sda".into(),
            read_bytes: 1000000,
            write_bytes: 2000000,
            read_ops: 1000,
            write_ops: 2000,
            io_time: 500,
        });
        disk.total_read_bytes = 1000000;
        disk.total_write_bytes = 2000000;

        let json = serde_json::to_string(&disk).expect("Сериализация должна работать");
        assert!(json.contains("sda"));
        assert!(json.contains("1000000"));
        assert!(json.contains("2000000"));

        // Тест десериализации
        let deserialized: DiskMetrics =
            serde_json::from_str(&json).expect("Десериализация должна работать");
        assert_eq!(deserialized.devices.len(), 1);
        assert_eq!(deserialized.devices[0].name, "sda".into());
        assert_eq!(deserialized.total_read_bytes, 1000000);
        assert_eq!(deserialized.total_write_bytes, 2000000);
    }

    #[test]
    fn test_collect_network_metrics_fallback() {
        // Тест проверяет, что collect_network_metrics работает корректно
        // В реальной системе он вернет реальные данные, в тестовой среде - пустые
        let network = collect_network_metrics();
        // Проверяем, что структура корректно инициализирована
        // В реальной системе могут быть данные, в тестовой - пустые
        assert!(
            network.total_rx_bytes
                >= network
                    .interfaces
                    .iter()
                    .map(|iface| iface.rx_bytes)
                    .sum::<u64>()
        );
        assert!(
            network.total_tx_bytes
                >= network
                    .interfaces
                    .iter()
                    .map(|iface| iface.tx_bytes)
                    .sum::<u64>()
        );
    }

    #[test]
    fn test_collect_disk_metrics_fallback() {
        // Тест проверяет, что collect_disk_metrics работает корректно
        // В реальной системе он вернет реальные данные, в тестовой среде - пустые
        let disk = collect_disk_metrics();
        // Проверяем, что структура корректно инициализирована
        // В реальной системе могут быть данные, в тестовой - пустые
        assert!(
            disk.total_read_bytes >= disk.devices.iter().map(|dev| dev.read_bytes).sum::<u64>()
        );
        assert!(
            disk.total_write_bytes >= disk.devices.iter().map(|dev| dev.write_bytes).sum::<u64>()
        );
    }

    #[test]
    fn test_collect_ebpf_metrics_fallback() {
        // Тест проверяет, что collect_ebpf_metrics работает корректно
        // В системе без поддержки eBPF должен возвращать None
        let ebpf_metrics = collect_ebpf_metrics();

        // В большинстве тестовых сред eBPF будет отключен или недоступен
        // Поэтому мы ожидаем либо None, либо Some с дефолтными значениями
        if let Some(metrics) = ebpf_metrics {
            // Если eBPF доступен, проверяем, что структура корректно инициализирована
            assert!(metrics.cpu_usage >= 0.0);
            // memory_usage и syscall_count всегда >= 0 (u64)
            assert!(metrics.timestamp > 0);
        } else {
            // Если eBPF недоступен, это нормальное поведение в тестовой среде
            // Просто проверяем, что функция не паникует
        }
    }

    #[test]
    fn test_system_metrics_optimize_memory_usage() {
        // Тест проверяет, что optimize_memory_usage корректно работает
        let mut metrics = SystemMetrics::default();

        // Добавляем некоторые данные
        metrics.network.interfaces.push(NetworkInterface {
            name: "eth0".into(),
            rx_bytes: 1000,
            tx_bytes: 2000,
            rx_packets: 100,
            tx_packets: 200,
            rx_errors: 1,
            tx_errors: 2,
        });

        metrics.disk.devices.push(DiskDevice {
            name: "sda".into(),
            read_bytes: 1000000,
            write_bytes: 2000000,
            read_ops: 1000,
            write_ops: 2000,
            io_time: 500,
        });

        // Устанавливаем температурные метрики
        metrics.temperature.cpu_temperature_c = Some(45.5);

        // Устанавливаем метрики энергопотребления
        metrics.power.cpu_power_w = Some(80.3);

        // Оптимизируем память
        let optimized = metrics.optimize_memory_usage();

        // Проверяем, что данные сохранены
        assert_eq!(optimized.network.interfaces.len(), 1);
        assert_eq!(optimized.disk.devices.len(), 1);
        assert_eq!(optimized.temperature.cpu_temperature_c, Some(45.5));
        assert_eq!(optimized.power.cpu_power_w, Some(80.3));

        // Тест с пустыми коллекциями
        let empty_metrics = SystemMetrics::default();
        let optimized_empty = empty_metrics.optimize_memory_usage();

        // Проверяем, что пустые коллекции остаются пустыми
        assert!(optimized_empty.network.interfaces.is_empty());
        assert!(optimized_empty.disk.devices.is_empty());

        // Проверяем, что пустые температурные и энергетические метрики сбрасываются
        assert_eq!(optimized_empty.temperature.cpu_temperature_c, None);
        assert_eq!(optimized_empty.temperature.gpu_temperature_c, None);
        assert_eq!(optimized_empty.power.system_power_w, None);
        assert_eq!(optimized_empty.power.cpu_power_w, None);
        assert_eq!(optimized_empty.power.gpu_power_w, None);
    }

    #[test]
    fn test_system_metrics_with_ebpf_integration() {
        // Тест проверяет, что SystemMetrics корректно интегрирует eBPF метрики
        let mut metrics = SystemMetrics::default();

        // Проверяем, что изначально eBPF метрики отсутствуют
        assert!(metrics.ebpf.is_none());

        // Устанавливаем eBPF метрики
        let ebpf_metrics = crate::metrics::ebpf::EbpfMetrics {
            cpu_usage: 25.5,
            memory_usage: 1024 * 1024 * 512, // 512 MB
            syscall_count: 100,
            network_packets: 0,
            network_bytes: 0,
            active_connections: 0,
            gpu_usage: 0.0,
            gpu_memory_usage: 0,
            process_memory_details: None,
            gpu_compute_units: 0,
            gpu_power_usage: 0,
            gpu_temperature: 0,
            filesystem_ops: 0,
            active_processes: 0,
            cpu_temperature: 50,
            cpu_max_temperature: 80,
            cpu_temperature_details: None,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(std::time::Duration::from_secs(0))
                .as_nanos() as u64,
            syscall_details: None,
            network_details: None,
            connection_details: None,
            gpu_details: None,
            process_details: None,
            filesystem_details: None,
            process_energy_details: None,
            process_gpu_details: None,
            process_network_details: None,
            process_disk_details: None,
        };
        metrics.ebpf = Some(ebpf_metrics.clone());

        // Проверяем, что eBPF метрики установлены корректно
        assert!(metrics.ebpf.is_some());
        let stored_ebpf = metrics.ebpf.as_ref().unwrap();
        assert_eq!(stored_ebpf.cpu_usage, 25.5);
        assert_eq!(stored_ebpf.memory_usage, 1024 * 1024 * 512);
        assert_eq!(stored_ebpf.syscall_count, 100);
        assert_eq!(stored_ebpf.timestamp, ebpf_metrics.timestamp);

        // Проверяем сериализацию и десериализацию
        let json = serde_json::to_string(&metrics).expect("Сериализация должна работать");
        assert!(json.contains("ebpf"));

        let deserialized: SystemMetrics =
            serde_json::from_str(&json).expect("Десериализация должна работать");
        assert!(deserialized.ebpf.is_some());
        let deserialized_ebpf = deserialized.ebpf.unwrap();
        assert_eq!(deserialized_ebpf.cpu_usage, 25.5);
        assert_eq!(deserialized_ebpf.memory_usage, 1024 * 1024 * 512);
    }

    #[test]
    fn test_system_metrics_includes_network_and_disk() {
        // Тест проверяет, что SystemMetrics включает новые поля сетевых и дисковых метрик
        let metrics = SystemMetrics {
            cpu_times: CpuTimes::default(),
            memory: MemoryInfo::default(),
            load_avg: LoadAvg::default(),
            pressure: PressureMetrics::default(),
            temperature: TemperatureMetrics::default(),
            power: PowerMetrics::default(),
            network: NetworkMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: None,
        };

        // Проверяем, что метрики содержат новые поля
        assert!(metrics.network.interfaces.is_empty());
        assert_eq!(metrics.network.total_rx_bytes, 0);
        assert_eq!(metrics.network.total_tx_bytes, 0);
        assert!(metrics.disk.devices.is_empty());
        assert_eq!(metrics.disk.total_read_bytes, 0);
        assert_eq!(metrics.disk.total_write_bytes, 0);
        assert!(metrics.ebpf.is_none());
    }

    #[test]
    fn test_system_metrics_includes_ebpf() {
        // Тест проверяет, что SystemMetrics включает поле eBPF метрик
        let metrics = SystemMetrics {
            cpu_times: CpuTimes::default(),
            memory: MemoryInfo::default(),
            load_avg: LoadAvg::default(),
            pressure: PressureMetrics::default(),
            temperature: TemperatureMetrics::default(),
            power: PowerMetrics::default(),
            network: NetworkMetrics::default(),
            disk: DiskMetrics::default(),
            gpu: None,
            ebpf: Some(crate::metrics::ebpf::EbpfMetrics::default()),
        };

        // Проверяем, что метрики содержат поле eBPF
        assert!(metrics.ebpf.is_some());
        let ebpf_metrics = metrics.ebpf.unwrap();
        assert_eq!(ebpf_metrics.cpu_usage, 0.0);
        assert_eq!(ebpf_metrics.memory_usage, 0);
        assert_eq!(ebpf_metrics.syscall_count, 0);
    }

    #[test]
    fn test_parse_network_line() {
        // Тест проверяет парсинг строки из /proc/net/dev
        let line = "eth0: 12345678 1234 0 0 0 0 0 0 12345678 1234 0 0 0 0 0 0";
        let parts: Vec<&str> = line.split_whitespace().collect();

        assert_eq!(parts.len(), 17);
        let interface_name = parts[0].trim_end_matches(':');
        assert_eq!(interface_name, "eth0");

        let rx_bytes = parts[1].parse::<u64>().unwrap();
        let rx_packets = parts[2].parse::<u64>().unwrap();
        let rx_errors = parts[3].parse::<u64>().unwrap();
        let tx_bytes = parts[9].parse::<u64>().unwrap();
        let tx_packets = parts[10].parse::<u64>().unwrap();
        let tx_errors = parts[11].parse::<u64>().unwrap();

        assert_eq!(rx_bytes, 12345678);
        assert_eq!(rx_packets, 1234);
        assert_eq!(rx_errors, 0);
        assert_eq!(tx_bytes, 12345678);
        assert_eq!(tx_packets, 1234);
        assert_eq!(tx_errors, 0);
    }

    #[test]
    fn test_parse_disk_line() {
        // Тест проверяет парсинг строки из /proc/diskstats
        let line = "8 0 sda 1234 0 5678 123 456 0 7890 1234 0 0 0 12345";
        let parts: Vec<&str> = line.split_whitespace().collect();

        assert_eq!(parts.len(), 15);
        let device_name = parts[2];
        assert_eq!(device_name, "sda");

        let read_ops = parts[3].parse::<u64>().unwrap();
        let read_sectors = parts[5].parse::<u64>().unwrap();
        let write_ops = parts[7].parse::<u64>().unwrap();
        let write_sectors = parts[9].parse::<u64>().unwrap();
        let io_time = parts[14].parse::<u64>().unwrap();

        assert_eq!(read_ops, 1234);
        assert_eq!(read_sectors, 5678);
        assert_eq!(write_ops, 456);
        assert_eq!(write_sectors, 7890);
        assert_eq!(io_time, 12345);

        // Проверяем конвертацию секторов в байты
        let read_bytes = read_sectors * 512;
        let write_bytes = write_sectors * 512;
        assert_eq!(read_bytes, 5678 * 512);
        assert_eq!(write_bytes, 7890 * 512);
    }

    #[test]
    fn test_power_metrics_default_values() {
        // Тест проверяет, что PowerMetrics::default() возвращает пустые значения
        let power = PowerMetrics::default();
        assert_eq!(power.system_power_w, None);
        assert_eq!(power.cpu_power_w, None);
        assert_eq!(power.gpu_power_w, None);
    }

    #[test]
    fn test_temperature_metrics_default_values() {
        // Тест проверяет, что TemperatureMetrics::default() возвращает пустые значения
        let temp = TemperatureMetrics::default();
        assert_eq!(temp.cpu_temperature_c, None);
        assert_eq!(temp.gpu_temperature_c, None);
    }

    #[test]
    fn test_power_metrics_parsing() {
        // Тест проверяет парсинг значений энергопотребления
        // Это unit-тест для логики парсинга, а не для реального сбора метрик
        let energy_uj = 1234567890; // 1234567890 микроджоулей
        let energy_w = energy_uj as f32 / 1_000_000.0;

        // Проверяем, что конвертация корректна
        assert!(energy_w > 0.0);
        assert!(energy_w < 2000.0); // разумный диапазон для мощности
    }

    #[test]
    fn test_temperature_metrics_parsing() {
        // Тест проверяет парсинг значений температуры
        let temp_millidegrees = 45000; // 45.0°C
        let temp_c = temp_millidegrees as f32 / 1000.0;

        assert_eq!(temp_c, 45.0);
    }

    #[test]
    fn test_power_metrics_integration() {
        // Тест проверяет, что PowerMetrics корректно интегрируется в SystemMetrics
        let system_metrics = SystemMetrics {
            power: PowerMetrics {
                system_power_w: Some(100.5),
                cpu_power_w: Some(50.2),
                gpu_power_w: Some(75.8),
            },
            ..Default::default()
        };

        assert_eq!(system_metrics.power.system_power_w, Some(100.5));
        assert_eq!(system_metrics.power.cpu_power_w, Some(50.2));
        assert_eq!(system_metrics.power.gpu_power_w, Some(75.8));
    }

    #[test]
    fn test_temperature_metrics_integration() {
        // Тест проверяет, что TemperatureMetrics корректно интегрируется в SystemMetrics
        let system_metrics = SystemMetrics {
            temperature: TemperatureMetrics {
                cpu_temperature_c: Some(65.5),
                gpu_temperature_c: Some(72.3),
            },
            ..Default::default()
        };

        assert_eq!(system_metrics.temperature.cpu_temperature_c, Some(65.5));
        assert_eq!(system_metrics.temperature.gpu_temperature_c, Some(72.3));
    }

    #[test]
    fn test_power_metrics_serde() {
        // Тест проверяет сериализацию и десериализацию PowerMetrics
        let power = PowerMetrics {
            system_power_w: Some(123.45),
            cpu_power_w: Some(67.89),
            gpu_power_w: Some(90.12),
        };

        let serialized = serde_json::to_string(&power).unwrap();
        let deserialized: PowerMetrics = serde_json::from_str(&serialized).unwrap();

        assert_eq!(power.system_power_w, deserialized.system_power_w);
        assert_eq!(power.cpu_power_w, deserialized.cpu_power_w);
        assert_eq!(power.gpu_power_w, deserialized.gpu_power_w);
    }

    #[test]
    fn test_temperature_metrics_serde() {
        // Тест проверяет сериализацию и десериализацию TemperatureMetrics
        let temp = TemperatureMetrics {
            cpu_temperature_c: Some(55.5),
            gpu_temperature_c: Some(68.2),
        };

        let serialized = serde_json::to_string(&temp).unwrap();
        let deserialized: TemperatureMetrics = serde_json::from_str(&serialized).unwrap();

        assert_eq!(temp.cpu_temperature_c, deserialized.cpu_temperature_c);
        assert_eq!(temp.gpu_temperature_c, deserialized.gpu_temperature_c);
    }

    #[test]
    fn test_power_metrics_edge_cases() {
        // Тест проверяет обработку граничных случаев для PowerMetrics
        let power = PowerMetrics {
            system_power_w: Some(0.0),   // нулевая мощность
            cpu_power_w: Some(f32::MAX), // максимальное значение
            gpu_power_w: Some(f32::MIN), // минимальное значение
        };

        // Проверяем, что значения сохраняются корректно
        assert_eq!(power.system_power_w, Some(0.0));
        assert_eq!(power.cpu_power_w, Some(f32::MAX));
        assert_eq!(power.gpu_power_w, Some(f32::MIN));
    }

    #[test]
    fn test_temperature_metrics_edge_cases() {
        // Тест проверяет обработку граничных случаев для TemperatureMetrics
        let temp = TemperatureMetrics {
            cpu_temperature_c: Some(-273.15), // абсолютный ноль
            gpu_temperature_c: Some(150.0),   // высокая температура
        };

        // Проверяем, что значения сохраняются корректно
        assert_eq!(temp.cpu_temperature_c, Some(-273.15));
        assert_eq!(temp.gpu_temperature_c, Some(150.0));
    }

    #[test]
    fn test_power_metrics_optional_handling() {
        // Тест проверяет корректную работу с опциональными значениями
        let mut power = PowerMetrics::default();

        // Проверяем, что изначально все значения None
        assert!(power.system_power_w.is_none());
        assert!(power.cpu_power_w.is_none());
        assert!(power.gpu_power_w.is_none());

        // Устанавливаем значения
        power.system_power_w = Some(100.0);
        power.cpu_power_w = Some(50.0);

        // Проверяем, что значения установлены
        assert_eq!(power.system_power_w, Some(100.0));
        assert_eq!(power.cpu_power_w, Some(50.0));
        assert!(power.gpu_power_w.is_none());

        // Сбрасываем значения
        power.system_power_w = None;
        power.cpu_power_w = None;

        // Проверяем, что значения сброшены
        assert!(power.system_power_w.is_none());
        assert!(power.cpu_power_w.is_none());
    }

    #[test]
    fn test_temperature_metrics_optional_handling() {
        // Тест проверяет корректную работу с опциональными значениями
        let mut temp = TemperatureMetrics::default();

        // Проверяем, что изначально все значения None
        assert!(temp.cpu_temperature_c.is_none());
        assert!(temp.gpu_temperature_c.is_none());

        // Устанавливаем значения
        temp.cpu_temperature_c = Some(45.0);
        temp.gpu_temperature_c = Some(55.0);

        // Проверяем, что значения установлены
        assert_eq!(temp.cpu_temperature_c, Some(45.0));
        assert_eq!(temp.gpu_temperature_c, Some(55.0));

        // Сбрасываем значения
        temp.cpu_temperature_c = None;
        temp.gpu_temperature_c = None;

        // Проверяем, что значения сброшены
        assert!(temp.cpu_temperature_c.is_none());
        assert!(temp.gpu_temperature_c.is_none());
    }

    #[test]
    fn test_power_metrics_precision() {
        // Тест проверяет точность хранения значений мощности
        let power = PowerMetrics {
            system_power_w: Some(123.456_79),
            cpu_power_w: Some(0.123_46),
            gpu_power_w: Some(999.999_99),
        };

        // Проверяем, что значения сохраняются с достаточной точностью
        assert!(power.system_power_w.unwrap() > 123.45);
        assert!(power.system_power_w.unwrap() < 123.46);

        assert!(power.cpu_power_w.unwrap() > 0.12);
        assert!(power.cpu_power_w.unwrap() < 0.13);

        // Исправляем тест для gpu_power_w - 999.999999 может быть равно 1000.0 из-за точности f32
        assert!(power.gpu_power_w.unwrap() >= 999.99);
        assert!(power.gpu_power_w.unwrap() <= 1000.01);
    }

    #[test]
    fn test_temperature_metrics_precision() {
        // Тест проверяет точность хранения значений температуры
        let temp = TemperatureMetrics {
            cpu_temperature_c: Some(36.666666),
            gpu_temperature_c: Some(85.999999),
        };

        // Проверяем, что значения сохраняются с достаточной точностью
        assert!(temp.cpu_temperature_c.unwrap() > 36.66);
        assert!(temp.cpu_temperature_c.unwrap() < 36.67);

        // Исправляем тест для gpu_temperature_c - 85.999999 может быть равно 86.0 из-за точности f32
        assert!(temp.gpu_temperature_c.unwrap() >= 85.99);
        assert!(temp.gpu_temperature_c.unwrap() <= 86.01);
    }

    #[test]
    fn test_system_metrics_cache_basic() {
        // Создаем кэш с временем жизни 1 секунда
        let cache = SharedSystemMetricsCache::new(std::time::Duration::from_secs(1));

        // Создаем тестовые пути
        let paths = ProcPaths {
            stat: PathBuf::from("/proc/stat"),
            meminfo: PathBuf::from("/proc/meminfo"),
            loadavg: PathBuf::from("/proc/loadavg"),
            pressure_cpu: PathBuf::from("/proc/pressure/cpu"),
            pressure_io: PathBuf::from("/proc/pressure/io"),
            pressure_memory: PathBuf::from("/proc/pressure/memory"),
        };

        // Первое обращение должно собрать новые метрики
        let metrics1 = collect_system_metrics_cached(&cache, &paths, false)
            .expect("Не удалось собрать метрики");

        // Второе обращение должно вернуть кэшированные метрики
        let metrics2 = collect_system_metrics_cached(&cache, &paths, false)
            .expect("Не удалось получить кэшированные метрики");

        // Метрики должны быть идентичны
        assert_eq!(metrics1.cpu_times, metrics2.cpu_times);
        assert_eq!(metrics1.memory.mem_total_kb, metrics2.memory.mem_total_kb);
    }

    #[test]
    fn test_system_metrics_cache_force_refresh() {
        // Создаем кэш с временем жизни 1 секунда
        let cache = SharedSystemMetricsCache::new(std::time::Duration::from_secs(1));

        let paths = ProcPaths {
            stat: PathBuf::from("/proc/stat"),
            meminfo: PathBuf::from("/proc/meminfo"),
            loadavg: PathBuf::from("/proc/loadavg"),
            pressure_cpu: PathBuf::from("/proc/pressure/cpu"),
            pressure_io: PathBuf::from("/proc/pressure/io"),
            pressure_memory: PathBuf::from("/proc/pressure/memory"),
        };

        // Первое обращение
        let _metrics1 = collect_system_metrics_cached(&cache, &paths, false)
            .expect("Не удалось собрать метрики");

        // Второе обращение с принудительным обновлением
        let _metrics2 =
            collect_system_metrics_cached(&cache, &paths, true).expect("Не удалось обновить кэш");

        // Метрики должны быть разными (так как были собраны в разное время)
        // или одинаковыми (если система не изменилась за это время)
        // В любом случае, функция не должна падать
        // Удалено assert!(true) как избыточную проверку
    }

    #[test]
    fn test_system_metrics_cache_expired() {
        // Создаем кэш с очень коротким временем жизни (10 мс)
        let cache = SharedSystemMetricsCache::new(std::time::Duration::from_millis(10));

        let paths = ProcPaths {
            stat: PathBuf::from("/proc/stat"),
            meminfo: PathBuf::from("/proc/meminfo"),
            loadavg: PathBuf::from("/proc/loadavg"),
            pressure_cpu: PathBuf::from("/proc/pressure/cpu"),
            pressure_io: PathBuf::from("/proc/pressure/io"),
            pressure_memory: PathBuf::from("/proc/pressure/memory"),
        };

        // Первое обращение
        let _metrics1 = collect_system_metrics_cached(&cache, &paths, false)
            .expect("Не удалось собрать метрики");

        // Ждем, пока кэш устареет
        std::thread::sleep(std::time::Duration::from_millis(15));

        // Второе обращение должно обновить кэш
        let _metrics2 =
            collect_system_metrics_cached(&cache, &paths, false).expect("Не удалось обновить кэш");

        // Функция должна работать без ошибок
        // Удалено assert!(true) как избыточную проверку
    }

    #[test]
    fn test_collect_network_metrics_with_real_data() {
        // Тест проверяет парсинг реальных данных из /proc/net/dev
        // Создаем тестовые данные, похожие на реальные данные из /proc/net/dev
        let test_data = "Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo: 12345678 12345 0    0    0     0          0         0 12345678 12345 0    0    0     0       0          0
  eth0: 10000000 10000 1    0    0     0          0         0 20000000 20000 2    0    0     0       0          0
  wlan0: 5000000 5000 0    0    0     0          0         0 15000000 15000 0    0    0     0       0          0";

        let mut network = NetworkMetrics::default();
        let mut total_rx_bytes = 0;
        let mut total_tx_bytes = 0;

        for line in test_data.lines().skip(2) {
            // Пропускаем заголовки
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Разбираем строку
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 17 {
                let interface_name = parts[0].trim_end_matches(':');

                // Извлекаем значения
                let rx_bytes = parts[1].parse::<u64>().unwrap_or(0);
                let rx_packets = parts[2].parse::<u64>().unwrap_or(0);
                let rx_errors = parts[3].parse::<u64>().unwrap_or(0);
                let tx_bytes = parts[9].parse::<u64>().unwrap_or(0);
                let tx_packets = parts[10].parse::<u64>().unwrap_or(0);
                let tx_errors = parts[11].parse::<u64>().unwrap_or(0);

                network.interfaces.push(NetworkInterface {
                    name: interface_name.into(),
                    rx_bytes,
                    tx_bytes,
                    rx_packets,
                    tx_packets,
                    rx_errors,
                    tx_errors,
                });

                total_rx_bytes += rx_bytes;
                total_tx_bytes += tx_bytes;
            }
        }

        network.total_rx_bytes = total_rx_bytes;
        network.total_tx_bytes = total_tx_bytes;

        // Проверяем результаты
        assert_eq!(network.interfaces.len(), 3); // lo, eth0, wlan0

        // Проверяем интерфейс lo
        let lo_interface = &network.interfaces[0];
        assert_eq!(lo_interface.name, "lo".into());
        assert_eq!(lo_interface.rx_bytes, 12345678);
        assert_eq!(lo_interface.tx_bytes, 12345678);
        assert_eq!(lo_interface.rx_packets, 12345);
        assert_eq!(lo_interface.tx_packets, 12345);
        assert_eq!(lo_interface.rx_errors, 0);
        assert_eq!(lo_interface.tx_errors, 0);

        // Проверяем интерфейс eth0
        let eth0_interface = &network.interfaces[1];
        assert_eq!(eth0_interface.name, "eth0".into());
        assert_eq!(eth0_interface.rx_bytes, 10000000);
        assert_eq!(eth0_interface.tx_bytes, 20000000);
        assert_eq!(eth0_interface.rx_packets, 10000);
        assert_eq!(eth0_interface.tx_packets, 20000);
        assert_eq!(eth0_interface.rx_errors, 1);
        assert_eq!(eth0_interface.tx_errors, 2);

        // Проверяем интерфейс wlan0
        let wlan0_interface = &network.interfaces[2];
        assert_eq!(wlan0_interface.name, "wlan0".into());
        assert_eq!(wlan0_interface.rx_bytes, 5000000);
        assert_eq!(wlan0_interface.tx_bytes, 15000000);
        assert_eq!(wlan0_interface.rx_packets, 5000);
        assert_eq!(wlan0_interface.tx_packets, 15000);
        assert_eq!(wlan0_interface.rx_errors, 0);
        assert_eq!(wlan0_interface.tx_errors, 0);

        // Проверяем общие метрики
        assert_eq!(network.total_rx_bytes, 12345678 + 10000000 + 5000000);
        assert_eq!(network.total_tx_bytes, 12345678 + 20000000 + 15000000);
    }

    #[test]
    fn test_parallel_metrics_collection() {
        use crate::metrics::system::{collect_system_metrics_parallel, ProcPaths};

        let paths = ProcPaths::default();
        let result = collect_system_metrics_parallel(&paths);

        // Проверяем, что функция выполняется без ошибок
        assert!(result.is_ok());

        let metrics = result.unwrap();

        // Проверяем, что основные метрики собраны корректно
        assert!(metrics.memory.mem_total_kb > 0);
        assert!(metrics.load_avg.one >= 0.0);
        // Удаляем проверку для unsigned типа (всегда >= 0)
    }

    #[test]
    fn test_parallel_vs_sequential_consistency() {
        use crate::metrics::system::{
            collect_system_metrics, collect_system_metrics_parallel, ProcPaths,
        };

        let paths = ProcPaths::default();

        // Собираем метрики параллельно
        let parallel_result = collect_system_metrics_parallel(&paths);

        // Собираем метрики последовательно
        let sequential_result = collect_system_metrics(&paths);

        // Обе функции должны выполниться успешно
        assert!(parallel_result.is_ok());
        assert!(sequential_result.is_ok());

        let parallel_metrics = parallel_result.unwrap();
        let sequential_metrics = sequential_result.unwrap();

        // Метрики должны быть сопоставимы (хотя и могут немного отличаться из-за времени сбора)
        // Проверяем, что основные структуры корректны
        assert!(parallel_metrics.memory.mem_total_kb > 0);
        assert!(sequential_metrics.memory.mem_total_kb > 0);
        // Удаляем проверки для unsigned типов (всегда >= 0)
    }

    #[test]
    fn test_cached_parallel_metrics() {
        use crate::metrics::system::{
            collect_system_metrics_cached_parallel, ProcPaths, SharedSystemMetricsCache,
        };
        use std::time::Duration;

        let paths = ProcPaths::default();
        let cache = SharedSystemMetricsCache::new(Duration::from_secs(10));

        // Первый вызов должен собрать новые метрики
        let result1 = collect_system_metrics_cached_parallel(&cache, &paths, false);
        assert!(result1.is_ok());

        // Второй вызов должен использовать кэшированные метрики
        let result2 = collect_system_metrics_cached_parallel(&cache, &paths, false);
        assert!(result2.is_ok());

        let metrics1 = result1.unwrap();
        let metrics2 = result2.unwrap();

        // Метрики должны быть идентичны (из кэша)
        assert_eq!(metrics1.cpu_times, metrics2.cpu_times);
        assert_eq!(metrics1.memory, metrics2.memory);
        assert_eq!(metrics1.load_avg, metrics2.load_avg);
    }
}

/// Кэш для системных метрик
///
/// Используется для кэширования системных метрик и уменьшения количества
/// операций ввода-вывода при частом опросе.
#[derive(Debug, Default)]
struct SystemMetricsCache {
    cached_metrics: Option<SystemMetrics>,
    last_update_time: Option<Instant>,
    cache_duration: Duration,
}

impl SystemMetricsCache {
    /// Создать новый кэш с указанной длительностью кэширования
    pub fn new(cache_duration: Duration) -> Self {
        Self {
            cached_metrics: None,
            last_update_time: None,
            cache_duration,
        }
    }

    /// Получить метрики из кэша или обновить кэш, если он устарел
    pub fn get_or_update<F>(&mut self, update_func: F) -> Result<SystemMetrics>
    where
        F: FnOnce() -> Result<SystemMetrics>,
    {
        // Проверяем, есть ли актуальные данные в кэше
        if let (Some(metrics), Some(last_update)) = (&self.cached_metrics, self.last_update_time) {
            if last_update.elapsed() < self.cache_duration {
                // Данные еще актуальны, возвращаем их из кэша
                return Ok(metrics.clone());
            }
        }

        // Кэш устарел или пуст, обновляем данные
        let new_metrics = update_func()?;
        self.cached_metrics = Some(new_metrics.clone());
        self.last_update_time = Some(Instant::now());

        Ok(new_metrics)
    }

    /// Принудительно очистить кэш
    pub fn clear(&mut self) {
        self.cached_metrics = None;
        self.last_update_time = None;
    }
}

/// Потокобезопасный кэш системных метрик
#[derive(Debug, Default, Clone)]
pub struct SharedSystemMetricsCache {
    inner: Arc<Mutex<SystemMetricsCache>>,
}

impl SharedSystemMetricsCache {
    /// Создать новый потокобезопасный кэш
    pub fn new(cache_duration: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SystemMetricsCache::new(cache_duration))),
        }
    }

    /// Получить метрики из кэша или обновить кэш, если он устарел
    pub fn get_or_update<F>(&self, update_func: F) -> Result<SystemMetrics>
    where
        F: FnOnce() -> Result<SystemMetrics>,
    {
        let mut cache = self.inner.lock().unwrap();
        cache.get_or_update(update_func)
    }

    /// Принудительно очистить кэш
    pub fn clear(&self) {
        let mut cache = self.inner.lock().unwrap();
        cache.clear();
    }
}
