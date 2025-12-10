//! Метрики активности пользователя на основе событий evdev.
//!
//! Здесь реализован небольшой трекер, который принимает события ввода
//! (клавиатура, мышь, сенсорный ввод) и отвечает, активен ли пользователь,
//! а также сколько времени прошло с последнего ввода.

use evdev::{Device, EventType, InputEvent, Key};
use std::path::Path;
use std::time::{Duration, Instant};
use tracing::{debug, warn};

/// Метрики активности пользователя.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputMetrics {
    pub user_active: bool,
    /// Время с последнего ввода, миллисекунды. `None`, если событий ещё не было.
    pub time_since_last_input_ms: Option<u64>,
}

/// Трекер активности по событиям evdev.
#[derive(Debug, Clone)]
pub struct InputActivityTracker {
    last_event: Option<Instant>,
    idle_threshold: Duration,
}

impl InputActivityTracker {
    /// Создать трекер с заданным таймаутом простоя.
    pub fn new(idle_threshold: Duration) -> Self {
        Self {
            last_event: None,
            idle_threshold,
        }
    }

    /// Зарегистрировать событие ввода, используя текущее время.
    pub fn register_activity(&mut self, now: Instant) {
        self.last_event = Some(now);
    }

    /// Обновить состояние на основе полученных событий.
    ///
    /// Все события, кроме `EV_SYN`, считаются пользовательской активностью.
    pub fn ingest_events<'a, I>(&mut self, events: I, now: Instant) -> InputMetrics
    where
        I: IntoIterator<Item = &'a InputEvent>,
    {
        for ev in events {
            if is_user_activity_event(ev) {
                self.register_activity(now);
            }
        }
        self.metrics(now)
    }

    /// Текущие метрики активности.
    pub fn metrics(&self, now: Instant) -> InputMetrics {
        match self.last_event {
            Some(ts) => {
                let elapsed = now.saturating_duration_since(ts);
                InputMetrics {
                    user_active: elapsed <= self.idle_threshold,
                    time_since_last_input_ms: Some(duration_to_ms(elapsed)),
                }
            }
            None => InputMetrics {
                user_active: false,
                time_since_last_input_ms: None,
            },
        }
    }
}

fn is_user_activity_event(ev: &InputEvent) -> bool {
    match ev.event_type() {
        EventType::SYNCHRONIZATION => false,
        EventType::KEY => {
            let code = ev.code();
            // Игнорируем зарезервированные/неизвестные коды.
            code != Key::KEY_RESERVED.code()
        }
        EventType::RELATIVE | EventType::ABSOLUTE | EventType::SWITCH | EventType::MISC => true,
        _ => false,
    }
}

fn duration_to_ms(d: Duration) -> u64 {
    d.as_secs()
        .saturating_mul(1000)
        .saturating_add(u64::from(d.subsec_millis()))
}

/// Трекер активности пользователя, читающий события из реальных evdev устройств.
///
/// Автоматически обнаруживает доступные устройства ввода (клавиатура, мышь)
/// и читает события из `/dev/input/event*` в неблокирующем режиме.
pub struct EvdevInputTracker {
    devices: Vec<Device>,
    activity_tracker: InputActivityTracker,
}

/// Абстракция для различных типов трекеров активности пользователя.
pub enum InputTracker {
    /// Трекер, читающий события из реальных evdev устройств.
    Evdev(EvdevInputTracker),
    /// Простой трекер, который обновляется вручную.
    Simple(InputActivityTracker),
}

impl InputTracker {
    /// Создать трекер, автоматически выбирая между evdev и простым трекером.
    pub fn new(idle_threshold: Duration) -> Self {
        if EvdevInputTracker::is_available() {
            match EvdevInputTracker::new(idle_threshold) {
                Ok(tracker) => {
                    debug!("Using EvdevInputTracker for input metrics");
                    Self::Evdev(tracker)
                }
                Err(e) => {
                    warn!(
                        "Failed to create EvdevInputTracker: {}, falling back to simple tracker",
                        e
                    );
                    Self::Simple(InputActivityTracker::new(idle_threshold))
                }
            }
        } else {
            debug!("Evdev devices not available, using simple input tracker");
            Self::Simple(InputActivityTracker::new(idle_threshold))
        }
    }

    /// Обновить метрики, прочитав новые события.
    pub fn update(&mut self, now: Instant) -> InputMetrics {
        match self {
            Self::Evdev(tracker) => tracker.update(now),
            Self::Simple(tracker) => tracker.metrics(now),
        }
    }

    /// Получить текущие метрики без чтения новых событий.
    pub fn metrics(&self, now: Instant) -> InputMetrics {
        match self {
            Self::Evdev(tracker) => tracker.metrics(now),
            Self::Simple(tracker) => tracker.metrics(now),
        }
    }
}

impl EvdevInputTracker {
    /// Проверить, доступны ли evdev устройства.
    pub fn is_available() -> bool {
        Self::discover_devices().is_ok() && !Self::discover_devices().unwrap().is_empty()
    }

    /// Создать новый трекер с автоматическим обнаружением устройств.
    pub fn new(idle_threshold: Duration) -> Result<Self, anyhow::Error> {
        let devices = Self::discover_devices()?;
        if devices.is_empty() {
            anyhow::bail!("No input devices found");
        }

        debug!("Found {} input device(s)", devices.len());
        for device in &devices {
            debug!("  - {}", device.name().unwrap_or("Unknown"));
        }

        Ok(Self {
            devices,
            activity_tracker: InputActivityTracker::new(idle_threshold),
        })
    }

    /// Обновить метрики, прочитав новые события из всех устройств.
    ///
    /// Читает события в неблокирующем режиме и обновляет внутренний трекер активности.
    pub fn update(&mut self, now: Instant) -> InputMetrics {
        let mut all_events = Vec::new();

        for device in &mut self.devices {
            // Читаем все доступные события в неблокирующем режиме
            // fetch_events() возвращает итератор, который читает все доступные события
            match device.fetch_events() {
                Ok(events) => {
                    for event in events {
                        all_events.push(event);
                    }
                }
                Err(e) => {
                    // EAGAIN/WouldBlock означает, что больше нет событий (неблокирующий режим)
                    if e.kind() != std::io::ErrorKind::WouldBlock {
                        // Другие ошибки (например, устройство отключено) логируем, но продолжаем
                        warn!("Error reading from input device: {}", e);
                    }
                }
            }
        }

        // Обновляем трекер активности на основе всех собранных событий
        self.activity_tracker.ingest_events(all_events.iter(), now)
    }

    /// Получить текущие метрики без чтения новых событий.
    pub fn metrics(&self, now: Instant) -> InputMetrics {
        self.activity_tracker.metrics(now)
    }

    /// Обнаружить доступные устройства ввода.
    ///
    /// Ищет устройства в `/dev/input/event*` и фильтрует их по типу
    /// (клавиатура, мышь, сенсорный ввод).
    fn discover_devices() -> Result<Vec<Device>, anyhow::Error> {
        let input_dir = Path::new("/dev/input");
        let mut devices = Vec::new();

        // Читаем все файлы event* в /dev/input
        let entries = std::fs::read_dir(input_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Проверяем, что это файл event*
            if let Some(file_name) = path.file_name() {
                let file_name_str = file_name.to_string_lossy();
                if !file_name_str.starts_with("event") {
                    continue;
                }
            } else {
                continue;
            }

            // Пытаемся открыть устройство
            match Device::open(&path) {
                Ok(device) => {
                    // Проверяем, что устройство поддерживает нужные типы событий
                    let supported = device.supported_events();
                    if supported.contains(EventType::KEY)
                        || supported.contains(EventType::RELATIVE)
                        || supported.contains(EventType::ABSOLUTE)
                    {
                        // Не используем grab(), так как это захватывает устройство эксклюзивно
                        // и может помешать другим приложениям. Вместо этого просто читаем события.
                        devices.push(device);
                    }
                }
                Err(e) => {
                    debug!("Failed to open device {:?}: {}", path, e);
                }
            }
        }

        Ok(devices)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_events_means_inactive() {
        let tracker = InputActivityTracker::new(Duration::from_secs(5));
        let metrics = tracker.metrics(Instant::now());
        assert!(!metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, None);
    }

    #[test]
    fn key_event_marks_active() {
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        let now = Instant::now();
        let key = InputEvent::new(EventType::KEY, Key::KEY_A.code(), 1);
        let metrics = tracker.ingest_events([key].iter(), now);
        assert!(metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, Some(0));
    }

    #[test]
    fn idle_after_threshold() {
        let mut tracker = InputActivityTracker::new(Duration::from_millis(100));
        let start = Instant::now();
        let key = InputEvent::new(EventType::KEY, Key::KEY_B.code(), 1);
        tracker.ingest_events([key].iter(), start);

        let later = start + Duration::from_millis(250);
        let metrics = tracker.metrics(later);
        assert!(!metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, Some(250));
    }

    #[test]
    fn syn_events_are_ignored() {
        let mut tracker = InputActivityTracker::new(Duration::from_secs(1));
        let now = Instant::now();
        let syn = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);
        let metrics = tracker.ingest_events([syn].iter(), now);
        assert!(!metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, None);
    }

    #[test]
    fn evdev_tracker_availability_check() {
        // Проверяем, что is_available() не паникует
        let _ = EvdevInputTracker::is_available();
    }

    #[test]
    fn evdev_tracker_creation_without_devices() {
        // Если устройств нет, создание должно вернуть ошибку
        // Но в тестовом окружении могут быть устройства, поэтому просто проверяем,
        // что функция не паникует
        let result = EvdevInputTracker::new(Duration::from_secs(5));
        // Результат зависит от окружения, но функция не должна паниковать
        match result {
            Ok(_) => {
                // Если устройства есть, трекер создан успешно
            }
            Err(_) => {
                // Если устройств нет, это ожидаемо
            }
        }
    }

    #[test]
    fn evdev_tracker_metrics_without_update() {
        // Если трекер создан успешно, metrics() должен работать
        if let Ok(tracker) = EvdevInputTracker::new(Duration::from_secs(5)) {
            let now = Instant::now();
            let metrics = tracker.metrics(now);
            // Без событий user_active должен быть false
            assert!(!metrics.user_active);
        }
        // Если трекер не создан (нет устройств), тест просто пропускается
    }
}
