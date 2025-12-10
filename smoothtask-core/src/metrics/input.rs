//! Метрики активности пользователя на основе событий evdev.
//!
//! Здесь реализован небольшой трекер, который принимает события ввода
//! (клавиатура, мышь, сенсорный ввод) и отвечает, активен ли пользователь,
//! а также сколько времени прошло с последнего ввода.

use evdev::{EventType, InputEvent, Key};
use std::time::{Duration, Instant};

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
}
