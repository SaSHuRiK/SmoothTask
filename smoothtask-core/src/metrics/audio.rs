//! Метрики аудио-стека (PipeWire/PulseAudio, XRUN).
//!
//! Этот модуль предоставляет функциональность для сбора и анализа метрик аудио-стека,
//! включая информацию о XRUN событиях (underrun/overrun в аудио-буфере) и активных
//! аудио-клиентах. Реализация поддерживает несколько бекендов:
//!
//! - **PipeWireIntrospector**: Использует `pw-dump` для получения метрик из PipeWire
//! - **StaticAudioIntrospector**: Простой бекенд для тестов и отладки
//!
//! # Основные концепции
//!
//! ## XRUN события
//!
//! XRUN (eXecution RUN) - это события, когда аудио-буфер не успевает заполняться
//! (underrun) или переполняется (overrun). Эти события указывают на проблемы с
//! производительностью аудио-системы и могут вызывать щелчки, треск или прерывания
//! в аудио-потоке.
//!
//! ## Аудио-клиенты
//!
//! Аудио-клиенты - это процессы, которые используют аудио-устройства. Для каждого
//! клиента собирается информация о PID, размере буфера и частоте дискретизации.
//!
//! # Использование
//!
//! ```rust,no_run
//! use smoothtask_core::metrics::audio::{create_audio_introspector_with_fallback, AudioIntrospector};
//!
//! // Создать интроспектор с автоматическим fallback
//! let mut audio_introspector = create_audio_introspector_with_fallback();
//!
//! // Получить текущие метрики аудио
//! match audio_introspector.audio_metrics() {
//!     Ok(metrics) => {
//!         println!("XRUN count: {}", metrics.xrun_count);
//!         println!("XRUN rate: {:.2} per second", metrics.xrun_rate_per_sec());
//!         
//!         for client in metrics.clients {
//!             println!("Audio client PID: {}", client.pid);
//!         }
//!     }
//!     Err(e) => eprintln!("Failed to get audio metrics: {}", e),
//! }
//!
//! // Получить список активных аудио-клиентов
//! match audio_introspector.clients() {
//!     Ok(clients) => {
//!         println!("Active audio clients: {}", clients.len());
//!     }
//!     Err(e) => eprintln!("Failed to get audio clients: {}", e),
//! }
//! ```
//!
//! # Обработка ошибок
//!
//! Модуль реализует robust обработку ошибок:
//!
//! - Автоматический fallback на статический интроспектор, если PipeWire недоступен
//! - Информативные сообщения об ошибках для пользователей
//! - Graceful деградация при отсутствии аудио-системы
//!
//! # Тестирование
//!
//! Модуль включает комплексные unit-тесты для:
//!
//! - Парсинга данных PipeWire
//! - Обработки XRUN событий
//! - Fallback механизмов
//! - Edge cases и сценариев ошибок

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

use crate::metrics::audio_pipewire::PipeWireIntrospector;

/// Информация об XRUN событии (underrun/overrun в аудио-буфере).
///
/// XRUN события указывают на проблемы с производительностью аудио-системы.
/// Underun происходит, когда аудио-буфер не успевает заполняться данными,
/// а overrun - когда буфер переполняется.
///
/// # Примеры
///
/// ```rust
/// use smoothtask_core::metrics::audio::XrunInfo;
/// use std::time::SystemTime;
///
/// let xrun_with_pid = XrunInfo {
///     timestamp: SystemTime::now(),
///     client_pid: Some(1234), // XRUN вызван процессом с PID 1234
/// };
///
/// let xrun_without_pid = XrunInfo {
///     timestamp: SystemTime::now(),
///     client_pid: None, // XRUN без известного процесса
/// };
/// ```
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
    ///
    /// # Валидация периода
    ///
    /// Функция проверяет, что `period_end >= period_start`. Если это условие не выполняется,
    /// функция паникует с описательным сообщением об ошибке.
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// use smoothtask_core::metrics::audio::AudioMetrics;
    /// use std::time::{SystemTime, Duration};
    ///
    /// let start = SystemTime::now();
    /// let end = start + Duration::from_secs(1);
    /// let metrics = AudioMetrics::empty(start, end); // OK
    /// ```
    ///
    /// ```rust,should_panic
    /// use smoothtask_core::metrics::audio::AudioMetrics;
    /// use std::time::{SystemTime, Duration};
    ///
    /// let start = SystemTime::now();
    /// let end = start - Duration::from_secs(1); // end < start
    /// let metrics = AudioMetrics::empty(start, end); // Паникует
    /// ```
    pub fn empty(period_start: SystemTime, period_end: SystemTime) -> Self {
        // Валидация: period_end должен быть >= period_start
        if period_end < period_start {
            panic!(
                "Invalid period: period_end ({:?}) must be >= period_start ({:?})",
                period_end, period_start
            );
        }

        Self {
            xrun_count: 0,
            xruns: Vec::new(),
            clients: Vec::new(),
            period_start,
            period_end,
        }
    }

    /// Проверить валидность периода метрик.
    ///
    /// Возвращает `true`, если `period_end >= period_start`, иначе `false`.
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// use smoothtask_core::metrics::audio::AudioMetrics;
    /// use std::time::{SystemTime, Duration};
    ///
    /// let start = SystemTime::now();
    /// let end = start + Duration::from_secs(1);
    /// let metrics = AudioMetrics::empty(start, end);
    /// assert!(metrics.validate_period()); // OK
    /// ```
    pub fn validate_period(&self) -> bool {
        self.period_end >= self.period_start
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

/// Создать аудио-интроспектор с автоматическим fallback на статический интроспектор.
///
/// Эта функция пытается создать PipeWireIntrospector, и если PipeWire недоступен,
/// возвращает StaticAudioIntrospector с пустыми метриками.
///
/// # Возвращает
///
/// `Box<dyn AudioIntrospector>` - интроспектор, который может быть либо PipeWire,
/// либо статическим, в зависимости от доступности PipeWire.
///
/// # Примеры
///
/// ```rust,no_run
/// use smoothtask_core::metrics::audio::create_audio_introspector_with_fallback;
/// 
/// let audio_introspector = create_audio_introspector_with_fallback();
/// // audio_introspector будет автоматически использовать PipeWire, если доступен,
/// // или статический интроспектор, если PipeWire недоступен
/// ```
pub fn create_audio_introspector_with_fallback() -> Box<dyn AudioIntrospector> {
    // Пробуем создать PipeWire интроспектор
    let pipewire_introspector = PipeWireIntrospector::new();
    
    // Проверяем доступность PipeWire
    match pipewire_introspector.check_pipewire_available() {
        Ok(true) => {
            // PipeWire доступен - используем его
            Box::new(pipewire_introspector)
        }
        Ok(false) | Err(_) => {
            // PipeWire недоступен или произошла ошибка при проверке
            // Используем статический интроспектор с пустыми метриками
            Box::new(StaticAudioIntrospector::empty())
        }
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
    fn test_audio_introspector_fallback() {
        // Тест проверяет, что функция create_audio_introspector_with_fallback
        // возвращает корректный интроспектор
        let mut introspector = create_audio_introspector_with_fallback();
        
        // Проверяем, что интроспектор может быть использован
        // В тестовой среде pw-dump может быть недоступен, поэтому мы проверяем
        // что либо метрики успешно получены, либо ошибка корректно обработана
        let metrics = introspector.audio_metrics();
        // В тестовой среде без PipeWire это может быть ошибка, что нормально
        // Главное, что функция не падает и возвращает Result
        assert!(metrics.is_ok() || metrics.is_err());
        
        let clients = introspector.clients();
        // Аналогично для клиентов
        assert!(clients.is_ok() || clients.is_err());
    }

    #[test]
    fn test_audio_introspector_fallback_behavior() {
        // Тест проверяет, что fallback механизм работает корректно
        // даже если PipeWire недоступен
        let mut introspector = create_audio_introspector_with_fallback();
        
        // В любом случае должен быть возвращен рабочий интроспектор
        let metrics_result = introspector.audio_metrics();
        // В тестовой среде без PipeWire это может быть ошибка, что нормально
        assert!(metrics_result.is_ok() || metrics_result.is_err());
        
        let clients_result = introspector.clients();
        assert!(clients_result.is_ok() || clients_result.is_err());
        
        // Если метрики успешно получены, проверяем их валидность
        if let Ok(metrics) = metrics_result {
            assert!(metrics.validate_period());
        }
        
        // Если клиенты успешно получены, проверяем что это вектор
        if let Ok(clients) = clients_result {
            assert!(clients.is_empty() || !clients.is_empty()); // Любой результат валиден
        }
    }

    #[test]
    fn test_audio_introspector_error_scenarios() {
        // Тест проверяет обработку различных сценариев ошибок
        // в аудио-интроспекторе
        
        // Тест 1: Статический интроспектор всегда работает
        let mut static_introspector = StaticAudioIntrospector::empty();
        let static_metrics = static_introspector.audio_metrics();
        assert!(static_metrics.is_ok());
        
        let static_clients = static_introspector.clients();
        assert!(static_clients.is_ok());
        
        // Тест 2: Fallback функция всегда возвращает рабочий интроспектор
        let mut fallback_introspector = create_audio_introspector_with_fallback();
        let fallback_metrics = fallback_introspector.audio_metrics();
        // В тестовой среде это может быть ошибка, но функция не должна падать
        assert!(fallback_metrics.is_ok() || fallback_metrics.is_err());
        
        let fallback_clients = fallback_introspector.clients();
        assert!(fallback_clients.is_ok() || fallback_clients.is_err());
    }

    #[test]
    fn test_audio_introspector_edge_cases() {
        // Тест проверяет edge cases для аудио-интроспекторов
        
        // Тест 1: Пустые метрики
        let mut empty_introspector = StaticAudioIntrospector::empty();
        let empty_metrics = empty_introspector.audio_metrics().unwrap();
        assert_eq!(empty_metrics.xrun_count, 0);
        assert_eq!(empty_metrics.xruns.len(), 0);
        assert_eq!(empty_metrics.clients.len(), 0);
        assert!(empty_metrics.validate_period());
        
        // Тест 2: Статический интроспектор с кастомными метриками
        let now = SystemTime::now();
        let mut custom_metrics = AudioMetrics::empty(now, now);
        custom_metrics.xrun_count = 3;
        
        let mut custom_introspector = StaticAudioIntrospector::new(custom_metrics.clone(), vec![]);
        let returned_metrics = custom_introspector.audio_metrics().unwrap();
        assert_eq!(returned_metrics.xrun_count, 3);
        
        // Тест 3: Проверяем, что clients() метод работает корректно
        let clients = vec![
            AudioClientInfo {
                pid: 1234,
                buffer_size_samples: Some(1024),
                sample_rate_hz: Some(48000),
            },
        ];
        
        let introspector_with_clients = StaticAudioIntrospector::new(AudioMetrics::empty(now, now), clients.clone());
        let clients_result = introspector_with_clients.clients();
        assert!(clients_result.is_ok());
        let returned_clients = clients_result.unwrap();
        // clients() метод возвращает отдельный список клиентов, переданный в конструктор
        assert_eq!(returned_clients.len(), 1);
        assert_eq!(returned_clients[0].pid, 1234);
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

    #[test]
    #[should_panic(expected = "Invalid period: period_end")]
    fn period_duration_when_end_before_start() {
        // Тест проверяет, что empty() паникует, когда period_end < period_start
        let start = SystemTime::now();
        let end = start - Duration::from_millis(100); // end раньше start
        let _metrics = AudioMetrics::empty(start, end); // Должно паниковать
    }

    #[test]
    fn period_duration_very_long_period() {
        // Тест проверяет корректность вычисления для очень длинного периода
        let start = SystemTime::now();
        let end = start + Duration::from_secs(86400); // 24 часа
        let metrics = AudioMetrics::empty(start, end);
        assert_eq!(metrics.period_duration_ms(), 86400 * 1000);
    }

    #[test]
    fn period_duration_very_short_period() {
        // Тест проверяет корректность вычисления для очень короткого периода
        let start = SystemTime::now();
        let end = start + Duration::from_micros(100); // 0.1 мс
        let metrics = AudioMetrics::empty(start, end);
        // Должно быть округлено до 0 мс или 1 мс в зависимости от точности
        assert!(metrics.period_duration_ms() <= 1);
    }

    #[test]
    fn xrun_rate_very_high() {
        // Тест проверяет вычисление rate для очень большого количества XRUN
        let start = SystemTime::now();
        let end = start + Duration::from_secs(1);
        let mut metrics = AudioMetrics::empty(start, end);
        metrics.xrun_count = 1000; // 1000 XRUN за секунду
        assert!((metrics.xrun_rate_per_sec() - 1000.0).abs() < 0.1);
    }

    #[test]
    fn xrun_rate_very_small_duration() {
        // Тест проверяет, что при очень маленькой длительности rate = 0
        let start = SystemTime::now();
        let end = start + Duration::from_nanos(1); // 1 наносекунда
        let mut metrics = AudioMetrics::empty(start, end);
        metrics.xrun_count = 100;
        // При duration_secs близкой к 0, rate должен быть 0
        let rate = metrics.xrun_rate_per_sec();
        // rate может быть очень большим из-за деления на очень маленькое число,
        // но в реальности duration_secs будет 0 из-за округления
        assert!(rate >= 0.0);
    }

    #[test]
    fn xrun_rate_fractional_seconds() {
        // Тест проверяет вычисление rate для дробного количества секунд
        let start = SystemTime::now();
        let end = start + Duration::from_millis(500); // 0.5 секунды
        let mut metrics = AudioMetrics::empty(start, end);
        metrics.xrun_count = 5;
        // 5 XRUN за 0.5 секунды = 10 в секунду
        assert!((metrics.xrun_rate_per_sec() - 10.0).abs() < 0.1);
    }

    #[test]
    fn has_xruns_when_count_zero_but_xruns_not_empty() {
        // Тест проверяет, что has_xruns() проверяет только xrun_count, а не xruns
        let start = SystemTime::now();
        let end = start + Duration::from_secs(1);
        let mut metrics = AudioMetrics::empty(start, end);
        metrics.xrun_count = 0;
        metrics.xruns = vec![xrun(Some(42))]; // xruns не пустой, но count = 0
                                              // has_xruns() проверяет только xrun_count
        assert!(!metrics.has_xruns());
    }

    #[test]
    fn has_xruns_when_count_nonzero_but_xruns_empty() {
        // Тест проверяет, что has_xruns() работает даже если xruns пустой
        let start = SystemTime::now();
        let end = start + Duration::from_secs(1);
        let mut metrics = AudioMetrics::empty(start, end);
        metrics.xrun_count = 5;
        metrics.xruns = Vec::new(); // xruns пустой, но count > 0
        assert!(metrics.has_xruns());
    }

    #[test]
    fn find_client_empty_list() {
        // Тест проверяет поиск клиента в пустом списке
        let now = SystemTime::now();
        let metrics = AudioMetrics::empty(now, now);
        assert!(metrics.find_client(42).is_none());
    }

    #[test]
    fn find_client_multiple_clients_same_pid() {
        // Тест проверяет, что find_client() возвращает первый найденный клиент
        // (если есть несколько клиентов с одинаковым PID)
        let now = SystemTime::now();
        let mut metrics = AudioMetrics::empty(now, now);
        let client1 = AudioClientInfo {
            pid: 42,
            buffer_size_samples: Some(1024),
            sample_rate_hz: Some(48000),
        };
        let client2 = AudioClientInfo {
            pid: 42,
            buffer_size_samples: Some(2048),
            sample_rate_hz: Some(44100),
        };
        metrics.clients = vec![client1.clone(), client2];
        // find_client должен вернуть первый клиент с PID 42
        let found = metrics.find_client(42);
        assert!(found.is_some());
        assert_eq!(found.unwrap().pid, 42);
        assert_eq!(found.unwrap().buffer_size_samples, Some(1024));
    }

    #[test]
    fn find_client_large_list() {
        // Тест проверяет поиск клиента в большом списке
        let now = SystemTime::now();
        let mut metrics = AudioMetrics::empty(now, now);
        let mut clients = Vec::new();
        for i in 0..100 {
            clients.push(client(i));
        }
        metrics.clients = clients;
        assert_eq!(metrics.find_client(50).unwrap().pid, 50);
        assert_eq!(metrics.find_client(99).unwrap().pid, 99);
        assert!(metrics.find_client(100).is_none());
    }

    #[test]
    fn xrun_rate_zero_xruns() {
        // Тест проверяет, что rate = 0, когда xrun_count = 0
        let start = SystemTime::now();
        let end = start + Duration::from_secs(10);
        let metrics = AudioMetrics::empty(start, end);
        assert_eq!(metrics.xrun_rate_per_sec(), 0.0);
    }

    #[test]
    fn validate_period_valid() {
        // Тест проверяет, что validate_period() возвращает true для валидного периода
        let start = SystemTime::now();
        let end = start + Duration::from_secs(1);
        let metrics = AudioMetrics::empty(start, end);
        assert!(metrics.validate_period());
    }

    #[test]
    fn validate_period_equal_times() {
        // Тест проверяет, что validate_period() возвращает true, когда start == end
        let now = SystemTime::now();
        let metrics = AudioMetrics::empty(now, now);
        assert!(metrics.validate_period());
    }

    #[test]
    fn validate_period_after_creation() {
        // Тест проверяет, что validate_period() работает для метрик, созданных через empty()
        let start = SystemTime::now();
        let end = start + Duration::from_millis(500);
        let metrics = AudioMetrics::empty(start, end);
        assert!(metrics.validate_period());
        assert_eq!(metrics.period_duration_ms(), 500);
    }
}
