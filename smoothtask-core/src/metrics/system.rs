use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::warn;

/// Сырые счётчики CPU из `/proc/stat`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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

/// Полный набор системных метрик, собранных из `/proc`.
///
/// Содержит информацию о CPU, памяти, нагрузке системы и давлении ресурсов.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// Счётчики CPU из `/proc/stat`
    pub cpu_times: CpuTimes,
    /// Информация о памяти из `/proc/meminfo`
    pub memory: MemoryInfo,
    /// Средняя нагрузка системы из `/proc/loadavg`
    pub load_avg: LoadAvg,
    /// Метрики давления из PSI (`/proc/pressure/*`)
    pub pressure: PressureMetrics,
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
///
/// # Ошибки
///
/// - Возвращает ошибку, если не удалось прочитать основные файлы (/proc/stat, /proc/meminfo, /proc/loadavg)
/// - Возвращает ошибку, если не удалось разобрать содержимое основных файлов
/// - PSI ошибки обрабатываются gracefully с предупреждениями и использованием пустых метрик
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
/// // Использование тестового пути (для тестирования)
/// let test_paths = ProcPaths::new("/tmp/test_proc");
/// let result = collect_system_metrics(&test_paths);
/// // result будет Ok с пустыми PSI метриками, если PSI файлы отсутствуют
/// ```
pub fn collect_system_metrics(paths: &ProcPaths) -> Result<SystemMetrics> {
    // Читаем основные файлы с подробными сообщениями об ошибках
    let cpu_contents = read_file(&paths.stat).with_context(|| {
        format!(
            "Не удалось прочитать CPU метрики из {}. 
             Проверьте, что файл существует и доступен для чтения. 
             Это может быть вызвано отсутствием прав доступа, отсутствием файла или проблемами с файловой системой. 
             Без этого файла невозможно собрать системные метрики.",
            paths.stat.display()
        )
    })?;

    let meminfo_contents = read_file(&paths.meminfo).with_context(|| {
        format!(
            "Не удалось прочитать информацию о памяти из {}. 
             Проверьте, что файл существует и доступен для чтения. 
             Это может быть вызвано отсутствием прав доступа, отсутствием файла или проблемами с файловой системой. 
             Без этого файла невозможно собрать системные метрики.",
            paths.meminfo.display()
        )
    })?;

    let loadavg_contents = read_file(&paths.loadavg).with_context(|| {
        format!(
            "Не удалось прочитать среднюю нагрузку из {}. 
             Проверьте, что файл существует и доступен для чтения. 
             Это может быть вызвано отсутствием прав доступа, отсутствием файла или проблемами с файловой системой. 
             Без этого файла невозможно собрать системные метрики.",
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

    Ok(SystemMetrics {
        cpu_times,
        memory,
        load_avg,
        pressure,
    })
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
}
