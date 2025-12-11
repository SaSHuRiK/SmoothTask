//! Классы приоритетов и маппинг на системные параметры.
//!
//! Определяет классы QoS (Quality of Service) и их соответствие
//! nice, ionice и cpu.weight согласно POLICY.md.

use serde::{Deserialize, Serialize};

/// Класс приоритета процесса или AppGroup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PriorityClass {
    /// Критически интерактивные процессы (фокус + аудио/игра).
    CritInteractive,
    /// Обычные интерактивные процессы (UI/CLI).
    Interactive,
    /// Дефолтный приоритет.
    Normal,
    /// Фоновые процессы (batch/maintenance).
    Background,
    /// Процессы, которые можно выполнять "на остатке".
    Idle,
}

impl PriorityClass {
    /// Получить строковое представление класса для логирования и БД.
    pub fn as_str(&self) -> &'static str {
        match self {
            PriorityClass::CritInteractive => "CRIT_INTERACTIVE",
            PriorityClass::Interactive => "INTERACTIVE",
            PriorityClass::Normal => "NORMAL",
            PriorityClass::Background => "BACKGROUND",
            PriorityClass::Idle => "IDLE",
        }
    }

    /// Парсинг из строки (для чтения из БД/конфига).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "CRIT_INTERACTIVE" => Some(PriorityClass::CritInteractive),
            "INTERACTIVE" => Some(PriorityClass::Interactive),
            "NORMAL" => Some(PriorityClass::Normal),
            "BACKGROUND" => Some(PriorityClass::Background),
            "IDLE" => Some(PriorityClass::Idle),
            _ => None,
        }
    }
}

/// Параметры nice для класса приоритета.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NiceParams {
    /// Значение nice (от -20 до +19, но мы ограничиваемся -8..+10).
    pub nice: i32,
}

/// Параметры latency_nice для класса приоритета.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatencyNiceParams {
    /// Значение latency_nice (от -20 до +19).
    /// -20 = максимальная чувствительность к задержке (UI, аудио, игры)
    /// +19 = безразличие к задержке (batch, индексация)
    pub latency_nice: i32,
}

/// Параметры ionice для класса приоритета.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IoNiceParams {
    /// Класс IO: 1 (realtime), 2 (best-effort), 3 (idle).
    pub class: i32,
    /// Уровень приоритета внутри класса (0-7 для best-effort).
    pub level: i32,
}

/// Параметры cgroup v2 для класса приоритета.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CgroupParams {
    /// cpu.weight (от 1 до 10000, но мы используем диапазон 25-200).
    pub cpu_weight: u32,
}

/// Полные параметры приоритета для класса.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PriorityParams {
    pub nice: NiceParams,
    pub latency_nice: LatencyNiceParams,
    pub ionice: IoNiceParams,
    pub cgroup: CgroupParams,
}

impl PriorityClass {
    /// Получить параметры приоритета для данного класса.
    pub fn params(&self) -> PriorityParams {
        match self {
            PriorityClass::CritInteractive => PriorityParams {
                nice: NiceParams { nice: -8 },
                latency_nice: LatencyNiceParams { latency_nice: -15 },
                ionice: IoNiceParams { class: 2, level: 0 },
                cgroup: CgroupParams { cpu_weight: 200 },
            },
            PriorityClass::Interactive => PriorityParams {
                nice: NiceParams { nice: -4 },
                latency_nice: LatencyNiceParams { latency_nice: -10 },
                ionice: IoNiceParams { class: 2, level: 2 },
                cgroup: CgroupParams { cpu_weight: 150 },
            },
            PriorityClass::Normal => PriorityParams {
                nice: NiceParams { nice: 0 },
                latency_nice: LatencyNiceParams { latency_nice: 0 },
                ionice: IoNiceParams { class: 2, level: 4 },
                cgroup: CgroupParams { cpu_weight: 100 },
            },
            PriorityClass::Background => PriorityParams {
                nice: NiceParams { nice: 5 },
                latency_nice: LatencyNiceParams { latency_nice: 10 },
                ionice: IoNiceParams { class: 2, level: 6 },
                cgroup: CgroupParams { cpu_weight: 50 },
            },
            PriorityClass::Idle => PriorityParams {
                nice: NiceParams { nice: 10 },
                latency_nice: LatencyNiceParams { latency_nice: 15 },
                ionice: IoNiceParams { class: 3, level: 0 }, // idle class
                cgroup: CgroupParams { cpu_weight: 25 },
            },
        }
    }

    /// Получить значение nice.
    pub fn nice(&self) -> i32 {
        self.params().nice.nice
    }

    /// Получить параметры latency_nice.
    pub fn latency_nice(&self) -> i32 {
        self.params().latency_nice.latency_nice
    }

    /// Получить параметры ionice.
    pub fn ionice(&self) -> IoNiceParams {
        self.params().ionice
    }

    /// Получить cpu.weight для cgroup v2.
    pub fn cpu_weight(&self) -> u32 {
        self.params().cgroup.cpu_weight
    }
}

/// Сравнение классов по "важности" (для сортировки и выбора максимального).
impl PartialOrd for PriorityClass {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityClass {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Сортируем по убыванию важности (CritInteractive > Interactive > ... > Idle)
        let order = |pc: PriorityClass| match pc {
            PriorityClass::CritInteractive => 5,
            PriorityClass::Interactive => 4,
            PriorityClass::Normal => 3,
            PriorityClass::Background => 2,
            PriorityClass::Idle => 1,
        };
        order(*self).cmp(&order(*other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_class_ordering() {
        assert!(PriorityClass::CritInteractive > PriorityClass::Interactive);
        assert!(PriorityClass::Interactive > PriorityClass::Normal);
        assert!(PriorityClass::Normal > PriorityClass::Background);
        assert!(PriorityClass::Background > PriorityClass::Idle);
    }

    #[test]
    fn test_crit_interactive_params() {
        let params = PriorityClass::CritInteractive.params();
        assert_eq!(params.nice.nice, -8);
        assert_eq!(params.latency_nice.latency_nice, -15);
        assert_eq!(params.ionice.class, 2);
        assert_eq!(params.ionice.level, 0);
        assert_eq!(params.cgroup.cpu_weight, 200);
    }

    #[test]
    fn test_interactive_params() {
        let params = PriorityClass::Interactive.params();
        assert_eq!(params.nice.nice, -4);
        assert_eq!(params.latency_nice.latency_nice, -10);
        assert_eq!(params.ionice.class, 2);
        assert_eq!(params.ionice.level, 2);
        assert_eq!(params.cgroup.cpu_weight, 150);
    }

    #[test]
    fn test_normal_params() {
        let params = PriorityClass::Normal.params();
        assert_eq!(params.nice.nice, 0);
        assert_eq!(params.latency_nice.latency_nice, 0);
        assert_eq!(params.ionice.class, 2);
        assert_eq!(params.ionice.level, 4);
        assert_eq!(params.cgroup.cpu_weight, 100);
    }

    #[test]
    fn test_background_params() {
        let params = PriorityClass::Background.params();
        assert_eq!(params.nice.nice, 5);
        assert_eq!(params.latency_nice.latency_nice, 10);
        assert_eq!(params.ionice.class, 2);
        assert_eq!(params.ionice.level, 6);
        assert_eq!(params.cgroup.cpu_weight, 50);
    }

    #[test]
    fn test_idle_params() {
        let params = PriorityClass::Idle.params();
        assert_eq!(params.nice.nice, 10);
        assert_eq!(params.latency_nice.latency_nice, 15);
        assert_eq!(params.ionice.class, 3); // idle class
        assert_eq!(params.ionice.level, 0);
        assert_eq!(params.cgroup.cpu_weight, 25);
    }

    #[test]
    fn test_as_str() {
        assert_eq!(PriorityClass::CritInteractive.as_str(), "CRIT_INTERACTIVE");
        assert_eq!(PriorityClass::Interactive.as_str(), "INTERACTIVE");
        assert_eq!(PriorityClass::Normal.as_str(), "NORMAL");
        assert_eq!(PriorityClass::Background.as_str(), "BACKGROUND");
        assert_eq!(PriorityClass::Idle.as_str(), "IDLE");
    }

    #[test]
    fn test_from_str() {
        assert_eq!(
            PriorityClass::from_str("CRIT_INTERACTIVE"),
            Some(PriorityClass::CritInteractive)
        );
        assert_eq!(
            PriorityClass::from_str("INTERACTIVE"),
            Some(PriorityClass::Interactive)
        );
        assert_eq!(
            PriorityClass::from_str("NORMAL"),
            Some(PriorityClass::Normal)
        );
        assert_eq!(
            PriorityClass::from_str("BACKGROUND"),
            Some(PriorityClass::Background)
        );
        assert_eq!(PriorityClass::from_str("IDLE"), Some(PriorityClass::Idle));
        assert_eq!(PriorityClass::from_str("UNKNOWN"), None);
    }

    #[test]
    fn test_serde_roundtrip() {
        let class = PriorityClass::CritInteractive;
        let json = serde_json::to_string(&class).expect("serialize");
        assert_eq!(json, "\"CRIT_INTERACTIVE\"");
        let deserialized: PriorityClass = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized, class);
    }

    #[test]
    fn test_convenience_methods() {
        let class = PriorityClass::Interactive;
        assert_eq!(class.nice(), -4);
        assert_eq!(class.latency_nice(), -10);
        assert_eq!(class.ionice().class, 2);
        assert_eq!(class.ionice().level, 2);
        assert_eq!(class.cpu_weight(), 150);
    }
}
