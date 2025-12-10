//! Метрики аудио-стека (PipeWire/PulseAudio, XRUN).
//!
//! Реальные бекенды (PipeWire/PulseAudio) будут подключаться позже, но каркас
//! позволяет уже сейчас работать с нормализованными структурами и писать
//! юнит-тесты вокруг логики обработки XRUN и определения аудио-клиентов.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

/// Информация об XRUN событии (underrun/overrun в аудио-буфере).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct XrunInfo {
    /// Время события (системное время).
    pub timestamp: SystemTime,
    /// PID процесса-клиента, вызвавшего XRUN, если известен.
    pub client_pid: Option<u32>,
}

/// Информация об аудио-клиенте.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AudioClientInfo {
    /// PID процесса-клиента.
    pub pid: u32,
    /// Размер буфера в сэмплах (если известен).
    pub buffer_size_samples: Option<u32>,
    /// Частота дискретизации в Гц (если известна).
    pub sample_rate_hz: Option<u32>,
}

/// Агрегированные метрики аудио-стека за период.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioMetrics {
    /// Количество XRUN событий за период.
    pub xrun_count: u32,
    /// Список XRUN событий (опционально, для детального анализа).
    pub xruns: Vec<XrunInfo>,
    /// Список активных аудио-клиентов.
    pub clients: Vec<AudioClientInfo>,
    /// Время начала периода сбора метрик.
    pub period_start: SystemTime,
    /// Время конца периода сбора метрик.
    pub period_end: SystemTime,
}

impl AudioMetrics {
    /// Создать пустые метрики для заданного периода.
    pub fn empty(period_start: SystemTime, period_end: SystemTime) -> Self {
        Self {
            xrun_count: 0,
            xruns: Vec::new(),
            clients: Vec::new(),
            period_start,
            period_end,
        }
    }

    /// Длительность периода в миллисекундах.
    pub fn period_duration_ms(&self) -> u64 {
        self.period_end
            .duration_since(self.period_start)
            .unwrap_or(Duration::ZERO)
            .as_millis() as u64
    }

    /// Средняя частота XRUN в событиях в секунду.
    pub fn xrun_rate_per_sec(&self) -> f64 {
        let duration_secs = self.period_duration_ms() as f64 / 1000.0;
        if duration_secs > 0.0 {
            self.xrun_count as f64 / duration_secs
        } else {
            0.0
        }
    }

    /// Есть ли XRUN события за период.
    pub fn has_xruns(&self) -> bool {
        self.xrun_count > 0
    }

    /// Найти клиента по PID.
    pub fn find_client(&self, pid: u32) -> Option<&AudioClientInfo> {
        self.clients.iter().find(|c| c.pid == pid)
    }
}

/// Общий интерфейс для получения метрик аудио-стека из конкретного бекенда.
pub trait AudioIntrospector: Send + Sync {
    /// Возвращает метрики аудио-стека за период с последнего вызова.
    ///
    /// Первый вызов возвращает метрики с момента инициализации интроспектора.
    /// Последующие вызовы возвращают метрики за период с предыдущего вызова.
    fn audio_metrics(&mut self) -> Result<AudioMetrics>;

    /// Возвращает список активных аудио-клиентов на текущий момент.
    fn clients(&self) -> Result<Vec<AudioClientInfo>>;
}

/// Простой бекенд для тестов и отладки, возвращающий заранее подготовленные метрики.
#[derive(Debug, Clone)]
pub struct StaticAudioIntrospector {
    metrics: AudioMetrics,
    clients: Vec<AudioClientInfo>,
}

impl StaticAudioIntrospector {
    /// Создать статический интроспектор с заданными метриками и клиентами.
    pub fn new(metrics: AudioMetrics, clients: Vec<AudioClientInfo>) -> Self {
        Self { metrics, clients }
    }

    /// Создать пустой интроспектор без XRUN и клиентов.
    pub fn empty() -> Self {
        let now = SystemTime::now();
        Self {
            metrics: AudioMetrics::empty(now, now),
            clients: Vec::new(),
        }
    }
}

impl AudioIntrospector for StaticAudioIntrospector {
    fn audio_metrics(&mut self) -> Result<AudioMetrics> {
        Ok(self.metrics.clone())
    }

    fn clients(&self) -> Result<Vec<AudioClientInfo>> {
        Ok(self.clients.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn xrun(pid: Option<u32>) -> XrunInfo {
        XrunInfo {
            timestamp: SystemTime::now(),
            client_pid: pid,
        }
    }

    fn client(pid: u32) -> AudioClientInfo {
        AudioClientInfo {
            pid,
            buffer_size_samples: Some(1024),
            sample_rate_hz: Some(48000),
        }
    }

    #[test]
    fn empty_metrics_have_no_xruns() {
        let now = SystemTime::now();
        let metrics = AudioMetrics::empty(now, now);
        assert!(!metrics.has_xruns());
        assert_eq!(metrics.xrun_count, 0);
        assert_eq!(metrics.xrun_rate_per_sec(), 0.0);
    }

    #[test]
    fn metrics_with_xruns() {
        let start = SystemTime::now();
        let end = start + Duration::from_secs(1);
        let mut metrics = AudioMetrics::empty(start, end);
        metrics.xrun_count = 5;
        metrics.xruns = vec![xrun(Some(42)), xrun(Some(42)), xrun(None)];

        assert!(metrics.has_xruns());
        assert_eq!(metrics.xrun_count, 5);
        assert!((metrics.xrun_rate_per_sec() - 5.0).abs() < 0.1);
    }

    #[test]
    fn find_client_by_pid() {
        let now = SystemTime::now();
        let mut metrics = AudioMetrics::empty(now, now);
        metrics.clients = vec![client(42), client(100)];

        assert_eq!(metrics.find_client(42).unwrap().pid, 42);
        assert_eq!(metrics.find_client(100).unwrap().pid, 100);
        assert!(metrics.find_client(999).is_none());
    }

    #[test]
    fn period_duration_calculation() {
        let start = SystemTime::now();
        let end = start + Duration::from_millis(500);
        let metrics = AudioMetrics::empty(start, end);
        assert_eq!(metrics.period_duration_ms(), 500);
    }

    #[test]
    fn static_introspector_returns_prepared_metrics() {
        let start = SystemTime::now();
        let end = start + Duration::from_secs(1);
        let mut metrics = AudioMetrics::empty(start, end);
        metrics.xrun_count = 3;
        let clients = vec![client(42)];

        let mut introspector = StaticAudioIntrospector::new(metrics.clone(), clients.clone());
        let returned_metrics = introspector.audio_metrics().unwrap();
        assert_eq!(returned_metrics.xrun_count, 3);

        let returned_clients = introspector.clients().unwrap();
        assert_eq!(returned_clients.len(), 1);
        assert_eq!(returned_clients[0].pid, 42);
    }

    #[test]
    fn static_introspector_empty() {
        let mut introspector = StaticAudioIntrospector::empty();
        let metrics = introspector.audio_metrics().unwrap();
        assert!(!metrics.has_xruns());
        assert_eq!(metrics.clients.len(), 0);
    }

    #[test]
    fn xrun_rate_calculation() {
        let start = SystemTime::now();
        let end = start + Duration::from_millis(2000); // 2 секунды
        let mut metrics = AudioMetrics::empty(start, end);
        metrics.xrun_count = 10;

        // 10 XRUN за 2 секунды = 5 в секунду
        assert!((metrics.xrun_rate_per_sec() - 5.0).abs() < 0.1);
    }

    #[test]
    fn zero_duration_period_has_zero_rate() {
        let now = SystemTime::now();
        let metrics = AudioMetrics::empty(now, now);
        assert_eq!(metrics.xrun_rate_per_sec(), 0.0);
    }
}
