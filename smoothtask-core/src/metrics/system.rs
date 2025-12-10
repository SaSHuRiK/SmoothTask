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
    pub fn mem_used_kb(&self) -> u64 {
        self.mem_total_kb.saturating_sub(self.mem_available_kb)
    }

    pub fn swap_used_kb(&self) -> u64 {
        self.swap_total_kb.saturating_sub(self.swap_free_kb)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LoadAvg {
    pub one: f64,
    pub five: f64,
    pub fifteen: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PressureRecord {
    pub avg10: f64,
    pub avg60: f64,
    pub avg300: f64,
    pub total: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Pressure {
    pub some: Option<PressureRecord>,
    pub full: Option<PressureRecord>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PressureMetrics {
    pub cpu: Pressure,
    pub io: Pressure,
    pub memory: Pressure,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_times: CpuTimes,
    pub memory: MemoryInfo,
    pub load_avg: LoadAvg,
    pub pressure: PressureMetrics,
}

impl SystemMetrics {
    /// Доли использования CPU относительно предыдущего снапшота.
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
pub fn collect_system_metrics(paths: &ProcPaths) -> Result<SystemMetrics> {
    let cpu_contents = read_file(&paths.stat)?;
    let meminfo_contents = read_file(&paths.meminfo)?;
    let loadavg_contents = read_file(&paths.loadavg)?;

    let cpu_times = parse_cpu_times(&cpu_contents)?;
    let memory = parse_meminfo(&meminfo_contents)?;
    let load_avg = parse_loadavg(&loadavg_contents)?;

    // PSI может быть недоступен на старых ядрах, поэтому обрабатываем ошибки gracefully
    let pressure_cpu = read_file(&paths.pressure_cpu)
        .and_then(|contents| parse_pressure(&contents))
        .unwrap_or_else(|e| {
            warn!(
                "Не удалось прочитать PSI CPU из {}: {}, используем пустые метрики",
                paths.pressure_cpu.display(),
                e
            );
            Pressure::default()
        });
    let pressure_io = read_file(&paths.pressure_io)
        .and_then(|contents| parse_pressure(&contents))
        .unwrap_or_else(|e| {
            warn!(
                "Не удалось прочитать PSI IO из {}: {}, используем пустые метрики",
                paths.pressure_io.display(),
                e
            );
            Pressure::default()
        });
    let pressure_memory = read_file(&paths.pressure_memory)
        .and_then(|contents| parse_pressure(&contents))
        .unwrap_or_else(|e| {
            warn!(
                "Не удалось прочитать PSI Memory из {}: {}, используем пустые метрики",
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
    fs::read_to_string(path).with_context(|| format!("Не удалось прочитать {}", path.display()))
}

fn parse_cpu_times(contents: &str) -> Result<CpuTimes> {
    let line = contents
        .lines()
        .find(|l| l.starts_with("cpu "))
        .ok_or_else(|| anyhow!("Нет строки с общими cpu счетчиками"))?;

    let mut fields = line.split_whitespace();
    let _cpu_label = fields
        .next()
        .ok_or_else(|| anyhow!("Пустая строка cpu в /proc/stat"))?;

    let parse_field = |name: &str, iter: &mut std::str::SplitWhitespace<'_>| -> Result<u64> {
        iter.next()
            .ok_or_else(|| anyhow!("Поле {} отсутствует в /proc/stat", name))?
            .parse::<u64>()
            .with_context(|| format!("Некорректное значение {} в /proc/stat", name))
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
                .with_context(|| format!("Некорректное значение {} в /proc/meminfo", key))?,
            None => continue,
        };
        values.insert(key, value);
    }

    let take = |name: &str| -> Result<u64> {
        values
            .get(name)
            .copied()
            .ok_or_else(|| anyhow!("В /proc/meminfo нет поля {}", name))
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
        .ok_or_else(|| anyhow!("Пустой /proc/loadavg"))?
        .parse::<f64>()
        .context("Некорректное значение loadavg 1m")?;
    let five = parts
        .next()
        .ok_or_else(|| anyhow!("Нет значения loadavg 5m"))?
        .parse::<f64>()
        .context("Некорректное значение loadavg 5m")?;
    let fifteen = parts
        .next()
        .ok_or_else(|| anyhow!("Нет значения loadavg 15m"))?
        .parse::<f64>()
        .context("Некорректное значение loadavg 15m")?;

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
        return Err(anyhow!("В файле pressure нет записей some/full"));
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
        let key = kv
            .next()
            .ok_or_else(|| anyhow!("Некорректный токен pressure: {}", token))?;
        let value = kv
            .next()
            .ok_or_else(|| anyhow!("Некорректный токен pressure: {}", token))?;
        match key {
            "avg10" => avg10 = Some(value.parse::<f64>().context("avg10 parse error")?),
            "avg60" => avg60 = Some(value.parse::<f64>().context("avg60 parse error")?),
            "avg300" => avg300 = Some(value.parse::<f64>().context("avg300 parse error")?),
            "total" => total = Some(value.parse::<u64>().context("total parse error")?),
            _ => {}
        }
    }

    Ok(PressureRecord {
        avg10: avg10.ok_or_else(|| anyhow!("Нет avg10 в pressure"))?,
        avg60: avg60.ok_or_else(|| anyhow!("Нет avg60 в pressure"))?,
        avg300: avg300.ok_or_else(|| anyhow!("Нет avg300 в pressure"))?,
        total: total.ok_or_else(|| anyhow!("Нет total в pressure"))?,
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
}
