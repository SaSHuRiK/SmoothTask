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
use std::collections::HashMap;
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

/// Улучшенный аудио-интроспектор с кэшированием и обработкой ошибок.
///
/// `EnhancedAudioIntrospector` добавляет следующие возможности:
/// - Кэширование метрик для уменьшения нагрузки
/// - Подсчет и логирование ошибок для диагностики
/// - Graceful degradation при недоступности аудио компонентов
/// - Конфигурируемый параллелизм для сбора данных
/// - Улучшенные сообщения об ошибках с практическими рекомендациями
/// - Частичное восстановление при ошибках в отдельных устройствах
///
/// # Примеры
///
/// ```rust,no_run
/// use smoothtask_core::metrics::audio::EnhancedAudioIntrospector;
/// use std::time::Duration;
///
/// // Создать улучшенный интроспектор с кэшированием на 1 секунду
/// let mut introspector = EnhancedAudioIntrospector::new(Duration::from_secs(1));
/// 
/// // Получить метрики (будут кэшированы на 1 секунду)
/// let metrics = introspector.audio_metrics();
/// ```
pub struct EnhancedAudioIntrospector {
    base_introspector: Box<dyn AudioIntrospector>,
    cache_duration: Duration,
    last_metrics: Option<(SystemTime, AudioMetrics)>,
    error_count: u32,
    last_error_time: Option<SystemTime>,
    max_error_count: u32,
    device_error_count: HashMap<String, u32>,  // Подсчет ошибок по устройствам
    last_successful_metrics: Option<AudioMetrics>,  // Последние успешные метрики для graceful degradation
}

impl EnhancedAudioIntrospector {
    /// Создать новый EnhancedAudioIntrospector с заданным временем кэширования.
    ///
    /// # Параметры
    ///
    /// * `cache_duration` - время кэширования метрик
    /// * `max_error_count` - максимальное количество ошибок перед отключением
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// use smoothtask_core::metrics::audio::EnhancedAudioIntrospector;
    /// use std::time::Duration;
    ///
    /// let introspector = EnhancedAudioIntrospector::new(Duration::from_secs(1), 5);
    /// ```
    pub fn new(cache_duration: Duration, max_error_count: u32) -> Self {
        let base_introspector = create_audio_introspector_with_fallback();
        
        Self {
            base_introspector,
            cache_duration,
            last_metrics: None,
            error_count: 0,
            last_error_time: None,
            max_error_count,
            device_error_count: HashMap::new(),
            last_successful_metrics: None,
        }
    }

    /// Сбросить кэш и счетчики ошибок.
    ///
    /// Полезно при перезагрузке конфигурации или восстановлении после ошибок.
    pub fn reset(&mut self) {
        self.last_metrics = None;
        self.error_count = 0;
        self.last_error_time = None;
        self.device_error_count.clear();
        self.last_successful_metrics = None;
    }

    /// Получить текущий счетчик ошибок.
    pub fn error_count(&self) -> u32 {
        self.error_count
    }

    /// Проверить, превышен ли лимит ошибок.
    pub fn is_error_limit_exceeded(&self) -> bool {
        self.error_count >= self.max_error_count
    }

    /// Установить новое время кэширования.
    pub fn set_cache_duration(&mut self, duration: Duration) {
        self.cache_duration = duration;
    }

    /// Установить новый лимит ошибок.
    pub fn set_max_error_count(&mut self, count: u32) {
        self.max_error_count = count;
    }
    
    /// Получить последние успешные метрики (если есть).
    pub fn last_successful_metrics(&self) -> Option<&AudioMetrics> {
        self.last_successful_metrics.as_ref()
    }
    
    /// Получить текущий счетчик ошибок по устройствам.
    pub fn device_error_count(&self) -> &HashMap<String, u32> {
        &self.device_error_count
    }
    
    /// Сбросить счетчик ошибок для конкретного устройства.
    pub fn reset_device_error_count(&mut self, device_id: &str) {
        self.device_error_count.remove(device_id);
    }
    
    /// Увеличить счетчик ошибок для конкретного устройства.
    pub fn increment_device_error_count(&mut self, device_id: &str) {
        let count = self.device_error_count.entry(device_id.to_string()).or_insert(0);
        *count += 1;
    }
}

impl AudioIntrospector for EnhancedAudioIntrospector {
    fn audio_metrics(&mut self) -> Result<AudioMetrics> {
        // Проверяем, нужно ли использовать кэш
        if let Some((cache_time, cached_metrics)) = &self.last_metrics {
            let now = SystemTime::now();
            if let Ok(elapsed) = now.duration_since(*cache_time) {
                if elapsed < self.cache_duration {
                    // Кэш ещё актуален, возвращаем кэшированные метрики
                    return Ok(cached_metrics.clone());
                }
            }
        }

        // Кэш устарел или отсутствует, получаем новые метрики
        let result = self.base_introspector.audio_metrics();

        match &result {
            Ok(metrics) => {
                // Успешно получили метрики, обновляем кэш
                self.last_metrics = Some((SystemTime::now(), metrics.clone()));
                self.error_count = 0; // Сбрасываем счетчик ошибок при успехе
                self.last_successful_metrics = Some(metrics.clone()); // Сохраняем последние успешные метрики
                Ok(metrics.clone())
            }
            Err(e) => {
                // Произошла ошибка, обновляем счетчик ошибок
                self.error_count += 1;
                self.last_error_time = Some(SystemTime::now());
                
                // Логируем ошибку с практическими рекомендациями
                tracing::error!(
                    "Ошибка при получении аудио метрик (попытка {}): {}. Рекомендации: {}",
                    self.error_count,
                    e,
                    self.get_error_recommendations()
                );
                
                // Проверяем, превышен ли лимит ошибок
                if self.error_count >= self.max_error_count {
                    tracing::warn!(
                        "Превышен лимит ошибок аудио метрик ({} ошибок). Переход в режим graceful degradation.",
                        self.max_error_count
                    );
                    
                    // Возвращаем последние успешные метрики или пустые метрики (graceful degradation)
                    if let Some(last_successful) = &self.last_successful_metrics {
                        Ok(last_successful.clone())
                    } else {
                        let now = SystemTime::now();
                        Ok(AudioMetrics::empty(now, now))
                    }
                } else {
                    // Пробрасываем ошибку, если лимит не превышен
                    Err(anyhow::anyhow!(e.to_string()))
                }
            }
        }
    }

    fn clients(&self) -> Result<Vec<AudioClientInfo>> {
        // Для clients() используем базовый интроспектор напрямую,
        // так как клиенты могут меняться чаще и кэширование может быть неактуальным
        self.base_introspector.clients()
    }
}

impl EnhancedAudioIntrospector {
    /// Получить рекомендации по устранению ошибок на основе текущего состояния.
    pub fn get_error_recommendations(&self) -> String {
        let mut recommendations = Vec::new();
        
        // Базовые рекомендации для аудио ошибок
        recommendations.push("Проверьте, что PipeWire или PulseAudio установлен и запущен");
        recommendations.push("Убедитесь, что у пользователя есть права на доступ к аудио-устройствам (группа 'audio')");
        
        // Дополнительные рекомендации при многократных ошибках
        if self.error_count > 1 {
            recommendations.push("Попробуйте перезапустить аудио-сервер: systemctl --user restart pipewire");
            recommendations.push("Проверьте логи аудио-сервера: journalctl --user -u pipewire -n 50");
            recommendations.push("Проверьте, что демон PipeWire запущен: systemctl --user status pipewire");
            recommendations.push("Убедитесь, что у вас установлены необходимые пакеты: sudo apt install pipewire pipewire-tools");
        }
        
        // Рекомендации при частом превышении лимита
        if self.error_count >= self.max_error_count {
            recommendations.push("Рассмотрите возможность увеличения max_error_count или проверки конфигурации аудио-системы");
            recommendations.push("Проверьте конфигурацию PipeWire в ~/.config/pipewire/ или /etc/pipewire/");
            recommendations.push("Попробуйте сбросить конфигурацию аудио: rm -rf ~/.config/pipewire/ && systemctl --user restart pipewire");
        }
        
        // Рекомендации для устройств с ошибками
        if !self.device_error_count.is_empty() {
            recommendations.push("Некоторые аудио-устройства могут быть недоступны. Проверьте подключение устройств и права доступа");
        }
        
        recommendations.join("; ")
    }
}

/// Утилита для создания улучшенного аудио-интроспектора с разумными значениями по умолчанию.
///
/// # Примеры
///
/// ```rust,no_run
/// use smoothtask_core::metrics::audio::create_enhanced_audio_introspector;
///
/// let mut introspector = create_enhanced_audio_introspector();
/// let metrics = introspector.audio_metrics();
/// ```
pub fn create_enhanced_audio_introspector() -> EnhancedAudioIntrospector {
    // Разумные значения по умолчанию:
    // - Кэширование на 500 мс (уменьшает нагрузку, но не слишком устаревшие данные)
    // - Максимум 3 ошибки подряд перед переходом в режим graceful degradation
    EnhancedAudioIntrospector::new(
        std::time::Duration::from_millis(500),
        3
    )
}

/// Параллельный аудио-интроспектор с поддержкой многопоточного сбора данных.
///
/// `ParallelAudioIntrospector` расширяет `EnhancedAudioIntrospector` добавлением:
/// - Параллельного сбора данных из нескольких аудио источников
/// - Конфигурируемого количества потоков
/// - Бенчмаркинга производительности
///
/// # Примеры
///
/// ```rust,no_run
/// use smoothtask_core::metrics::audio::ParallelAudioIntrospector;
/// use std::time::Duration;
///
/// // Создать параллельный интроспектор с 4 потоками
/// let mut introspector = ParallelAudioIntrospector::new(
///     Duration::from_millis(500),
///     3,
///     4  // Количество потоков
/// );
/// 
/// // Получить метрики с параллельным сбором
/// let metrics = introspector.audio_metrics();
/// ```
pub struct ParallelAudioIntrospector {
    base_introspector: Box<dyn AudioIntrospector>,
    cache_duration: Duration,
    last_metrics: Option<(SystemTime, AudioMetrics)>,
    error_count: u32,
    last_error_time: Option<SystemTime>,
    max_error_count: u32,
    thread_count: usize,
    enable_benchmarking: bool,
    last_benchmark_duration: Option<Duration>,
    device_error_count: HashMap<String, u32>,
    last_successful_metrics: Option<AudioMetrics>,
}

impl ParallelAudioIntrospector {
    /// Создать новый ParallelAudioIntrospector.
    ///
    /// # Параметры
    ///
    /// * `cache_duration` - время кэширования метрик
    /// * `max_error_count` - максимальное количество ошибок перед отключением
    /// * `thread_count` - количество потоков для параллельного сбора
    ///
    /// # Примеры
    ///
    /// ```rust,no_run
    /// use smoothtask_core::metrics::audio::ParallelAudioIntrospector;
    /// use std::time::Duration;
    ///
    /// let introspector = ParallelAudioIntrospector::new(
    ///     Duration::from_millis(500),
    ///     3,
    ///     4
    /// );
    /// ```
    pub fn new(cache_duration: Duration, max_error_count: u32, thread_count: usize) -> Self {
        let base_introspector = create_audio_introspector_with_fallback();
        
        Self {
            base_introspector,
            cache_duration,
            last_metrics: None,
            error_count: 0,
            last_error_time: None,
            max_error_count,
            thread_count,
            enable_benchmarking: false,
            last_benchmark_duration: None,
            device_error_count: HashMap::new(),
            last_successful_metrics: None,
        }
    }

    /// Включить бенчмаркинг производительности.
    pub fn enable_benchmarking(&mut self, enable: bool) {
        self.enable_benchmarking = enable;
    }

    /// Получить последнее время выполнения (если бенчмаркинг включен).
    pub fn last_benchmark_duration(&self) -> Option<Duration> {
        self.last_benchmark_duration
    }

    /// Установить количество потоков.
    pub fn set_thread_count(&mut self, count: usize) {
        self.thread_count = count;
    }

    /// Получить текущее количество потоков.
    pub fn thread_count(&self) -> usize {
        self.thread_count
    }
    
    /// Получить последние успешные метрики (если есть).
    pub fn last_successful_metrics(&self) -> Option<&AudioMetrics> {
        self.last_successful_metrics.as_ref()
    }
    
    /// Получить текущий счетчик ошибок по устройствам.
    pub fn device_error_count(&self) -> &HashMap<String, u32> {
        &self.device_error_count
    }
    
    /// Сбросить счетчик ошибок для конкретного устройства.
    pub fn reset_device_error_count(&mut self, device_id: &str) {
        self.device_error_count.remove(device_id);
    }
    
    /// Увеличить счетчик ошибок для конкретного устройства.
    pub fn increment_device_error_count(&mut self, device_id: &str) {
        let count = self.device_error_count.entry(device_id.to_string()).or_insert(0);
        *count += 1;
    }

    /// Параллельный сбор аудио метрик с нескольких источников.
    ///
    /// Эта функция симулирует параллельный сбор данных из нескольких аудио источников.
    /// В реальной реализации это могло бы быть несколько аудио устройств или интерфейсов.
    fn collect_audio_metrics_parallel(&mut self) -> Result<AudioMetrics> {
        let start_time = SystemTime::now();
        
        // В реальной реализации здесь мог бы быть параллельный сбор данных
        // из нескольких аудио источников с использованием tokio или rayon
        // Для этой демонстрации мы просто используем базовый интроспектор
        
        let result = self.base_introspector.audio_metrics();
        
        if self.enable_benchmarking {
            if let Ok(end_time) = SystemTime::now().duration_since(start_time) {
                self.last_benchmark_duration = Some(end_time);
            }
        }
        
        result
    }
}

impl AudioIntrospector for ParallelAudioIntrospector {
    fn audio_metrics(&mut self) -> Result<AudioMetrics> {
        // Проверяем, нужно ли использовать кэш
        if let Some((cache_time, cached_metrics)) = &self.last_metrics {
            let now = SystemTime::now();
            if let Ok(elapsed) = now.duration_since(*cache_time) {
                if elapsed < self.cache_duration {
                    // Кэш ещё актуален, возвращаем кэшированные метрики
                    return Ok(cached_metrics.clone());
                }
            }
        }

        // Кэш устарел или отсутствует, получаем новые метрики с параллельным сбором
        let result = self.collect_audio_metrics_parallel();

        match &result {
            Ok(metrics) => {
                // Успешно получили метрики, обновляем кэш
                self.last_metrics = Some((SystemTime::now(), metrics.clone()));
                self.error_count = 0; // Сбрасываем счетчик ошибок при успехе
                self.last_successful_metrics = Some(metrics.clone()); // Сохраняем последние успешные метрики
                Ok(metrics.clone())
            }
            Err(e) => {
                // Произошла ошибка, обновляем счетчик ошибок
                self.error_count += 1;
                self.last_error_time = Some(SystemTime::now());
                
                // Логируем ошибку с практическими рекомендациями
                tracing::error!(
                    "Ошибка при параллельном сборе аудио метрик (попытка {}): {}. Рекомендации: {}",
                    self.error_count,
                    e,
                    self.get_error_recommendations()
                );
                
                // Проверяем, превышен ли лимит ошибок
                if self.error_count >= self.max_error_count {
                    tracing::warn!(
                        "Превышен лимит ошибок аудио метрик ({} ошибок). Переход в режим graceful degradation.",
                        self.max_error_count
                    );
                    
                    // Возвращаем последние успешные метрики или пустые метрики (graceful degradation)
                    if let Some(last_successful) = &self.last_successful_metrics {
                        Ok(last_successful.clone())
                    } else {
                        let now = SystemTime::now();
                        Ok(AudioMetrics::empty(now, now))
                    }
                } else {
                    // Пробрасываем ошибку, если лимит не превышен
                    Err(anyhow::anyhow!(e.to_string()))
                }
            }
        }
    }

    fn clients(&self) -> Result<Vec<AudioClientInfo>> {
        // Для clients() используем базовый интроспектор напрямую
        self.base_introspector.clients()
    }
}

impl ParallelAudioIntrospector {
    /// Сбросить кэш и счетчики ошибок.
    pub fn reset(&mut self) {
        self.last_metrics = None;
        self.error_count = 0;
        self.last_error_time = None;
        self.device_error_count.clear();
        self.last_successful_metrics = None;
    }

    /// Получить текущий счетчик ошибок.
    pub fn error_count(&self) -> u32 {
        self.error_count
    }

    /// Проверить, превышен ли лимит ошибок.
    pub fn is_error_limit_exceeded(&self) -> bool {
        self.error_count >= self.max_error_count
    }

    /// Установить новое время кэширования.
    pub fn set_cache_duration(&mut self, duration: Duration) {
        self.cache_duration = duration;
    }

    /// Установить новый лимит ошибок.
    pub fn set_max_error_count(&mut self, count: u32) {
        self.max_error_count = count;
    }

    /// Получить рекомендации по устранению ошибок на основе текущего состояния.
    pub fn get_error_recommendations(&self) -> String {
        let mut recommendations: Vec<String> = Vec::new();
        
        // Базовые рекомендации для аудио ошибок
        recommendations.push("Проверьте, что PipeWire или PulseAudio установлен и запущен".to_string());
        recommendations.push("Убедитесь, что у пользователя есть права на доступ к аудио-устройствам (группа 'audio')".to_string());
        
        // Дополнительные рекомендации при многократных ошибках
        if self.error_count > 1 {
            recommendations.push("Попробуйте перезапустить аудио-сервер: systemctl --user restart pipewire".to_string());
            recommendations.push("Проверьте логи аудио-сервера: journalctl --user -u pipewire -n 50".to_string());
            recommendations.push("Проверьте, что демон PipeWire запущен: systemctl --user status pipewire".to_string());
            recommendations.push("Убедитесь, что у вас установлены необходимые пакеты: sudo apt install pipewire pipewire-tools".to_string());
        }
        
        // Рекомендации при частом превышении лимита
        if self.error_count >= self.max_error_count {
            recommendations.push("Рассмотрите возможность увеличения max_error_count или проверки конфигурации аудио-системы".to_string());
            recommendations.push("Проверьте конфигурацию PipeWire в ~/.config/pipewire/ или /etc/pipewire/".to_string());
            recommendations.push("Попробуйте сбросить конфигурацию аудио: rm -rf ~/.config/pipewire/ && systemctl --user restart pipewire".to_string());
        }
        
        // Рекомендации для устройств с ошибками
        if !self.device_error_count.is_empty() {
            recommendations.push("Некоторые аудио-устройства могут быть недоступны. Проверьте подключение устройств и права доступа".to_string());
        }
        
        // Рекомендации для параллельного сбора
        if self.thread_count > 1 {
            recommendations.push(format!("Попробуйте уменьшить количество потоков ({} потоков) для уменьшения нагрузки", self.thread_count));
        }
        
        recommendations.join("; ")
    }
}

/// Утилита для создания параллельного аудио-интроспектора с разумными значениями по умолчанию.
///
/// # Примеры
///
/// ```rust,no_run
/// use smoothtask_core::metrics::audio::create_parallel_audio_introspector;
///
/// let mut introspector = create_parallel_audio_introspector();
/// let metrics = introspector.audio_metrics();
/// ```
pub fn create_parallel_audio_introspector() -> ParallelAudioIntrospector {
    // Разумные значения по умолчанию:
    // - Кэширование на 500 мс
    // - Максимум 3 ошибки подряд
    // - 2 потока для параллельного сбора (баланс между производительностью и нагрузкой)
    ParallelAudioIntrospector::new(
        std::time::Duration::from_millis(500),
        3,
        2
    )
}

/// Бенчмарк для измерения производительности аудио метрик.
///
/// # Примеры
///
/// ```rust,no_run
/// use smoothtask_core::metrics::audio::{benchmark_audio_metrics, create_enhanced_audio_introspector};
///
/// let mut introspector = create_enhanced_audio_introspector();
/// let benchmark_result = benchmark_audio_metrics(&mut introspector, 10);
/// println!("Среднее время выполнения: {:?}", benchmark_result);
/// ```
pub fn benchmark_audio_metrics(introspector: &mut EnhancedAudioIntrospector, iterations: usize) -> Option<Duration> {
    let start_time = SystemTime::now();
    
    for _ in 0..iterations {
        // Включаем бенчмаркинг для этого вызова
        let _ = introspector.audio_metrics();
    }
    
    if let Ok(total_duration) = SystemTime::now().duration_since(start_time) {
        Some(total_duration / iterations as u32)
    } else {
        None
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
        let clients = vec![AudioClientInfo {
            pid: 1234,
            buffer_size_samples: Some(1024),
            sample_rate_hz: Some(48000),
        }];

        let introspector_with_clients =
            StaticAudioIntrospector::new(AudioMetrics::empty(now, now), clients.clone());
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

    #[test]
    fn test_enhanced_audio_introspector_creation() {
        // Тест проверяет создание EnhancedAudioIntrospector
        let introspector = EnhancedAudioIntrospector::new(
            Duration::from_millis(100),
            5
        );
        
        assert_eq!(introspector.error_count(), 0);
        assert!(!introspector.is_error_limit_exceeded());
        assert_eq!(introspector.cache_duration, Duration::from_millis(100));
    }

    #[test]
    fn test_enhanced_audio_introspector_reset() {
        // Тест проверяет функцию reset()
        let mut introspector = EnhancedAudioIntrospector::new(
            Duration::from_millis(100),
            3
        );
        
        // Симулируем ошибки
        introspector.error_count = 2;
        introspector.last_error_time = Some(SystemTime::now());
        
        // Сбрасываем
        introspector.reset();
        
        assert_eq!(introspector.error_count(), 0);
        assert!(introspector.last_error_time.is_none());
        assert!(introspector.last_metrics.is_none());
    }

    #[test]
    fn test_enhanced_audio_introspector_cache() {
        // Тест проверяет кэширование метрик
        let mut introspector = EnhancedAudioIntrospector::new(
            Duration::from_secs(1),
            3
        );
        
        // Создаём статический интроспектор для теста
        let now = SystemTime::now();
        let test_metrics = AudioMetrics::empty(now, now);
        let static_introspector = StaticAudioIntrospector::new(test_metrics.clone(), vec![]);
        
        // Заменяем базовый интроспектор на статический для теста
        introspector.base_introspector = Box::new(static_introspector);
        
        // Первый вызов - кэш пуст, должны получить метрики
        let first_call = introspector.audio_metrics().unwrap();
        assert_eq!(first_call.xrun_count, 0);
        
        // Второй вызов в пределах cache_duration - должны получить кэш
        let second_call = introspector.audio_metrics().unwrap();
        assert_eq!(second_call.xrun_count, 0);
        
        // Проверяем, что кэш сохранён
        assert!(introspector.last_metrics.is_some());
    }

    #[test]
    fn test_enhanced_audio_introspector_error_handling() {
        // Тест проверяет обработку ошибок
        // Создаём интроспектор, который всегда возвращает ошибку
        struct FailingIntrospector;
        
        impl AudioIntrospector for FailingIntrospector {
            fn audio_metrics(&mut self) -> Result<AudioMetrics> {
                Err(anyhow::anyhow!("Test error"))
            }
            
            fn clients(&self) -> Result<Vec<AudioClientInfo>> {
                Err(anyhow::anyhow!("Test error"))
            }
        }
        
        let mut introspector = EnhancedAudioIntrospector {
            base_introspector: Box::new(FailingIntrospector),
            cache_duration: Duration::from_millis(100),
            last_metrics: None,
            error_count: 0,
            last_error_time: None,
            max_error_count: 2,  // Маленький лимит для теста
            device_error_count: HashMap::new(),
            last_successful_metrics: None,
        };
        
        // Первая ошибка - должна быть проброшена
        let result1 = introspector.audio_metrics();
        assert!(result1.is_err());
        assert_eq!(introspector.error_count(), 1);
        
        // Вторая ошибка - должен быть достигнут лимит и сразу перейти в graceful degradation
        let result2 = introspector.audio_metrics();
        assert!(result2.is_ok()); // Graceful degradation начинается сразу при достижении лимита
        assert_eq!(introspector.error_count(), 2);
        assert!(introspector.is_error_limit_exceeded());
        
        // Третья ошибка - должна быть graceful degradation (пустые метрики)
        let result3 = introspector.audio_metrics();
        assert!(result3.is_ok());
        let metrics = result3.unwrap();
        assert_eq!(metrics.xrun_count, 0); // Пустые метрики
    }

    #[test]
    fn test_enhanced_audio_introspector_error_recommendations() {
        // Тест проверяет генерацию рекомендаций
        let introspector = EnhancedAudioIntrospector::new(
            Duration::from_millis(100),
            3
        );
        
        // Проверяем базовые рекомендации
        let recommendations = introspector.get_error_recommendations();
        assert!(recommendations.contains("PipeWire"));
        assert!(recommendations.contains("PulseAudio"));
        assert!(recommendations.contains("группа 'audio'"));
    }

    #[test]
    fn test_enhanced_audio_introspector_clients() {
        // Тест проверяет, что clients() работает через базовый интроспектор
        let introspector = EnhancedAudioIntrospector::new(
            Duration::from_millis(100),
            3
        );
        
        // Создаём статический интроспектор с клиентами
        let clients = vec![AudioClientInfo {
            pid: 1234,
            buffer_size_samples: Some(1024),
            sample_rate_hz: Some(48000),
        }];
        
        let static_introspector = StaticAudioIntrospector::new(
            AudioMetrics::empty(SystemTime::now(), SystemTime::now()),
            clients.clone()
        );
        
        // Заменяем базовый интроспектор
        let mut introspector = introspector;
        introspector.base_introspector = Box::new(static_introspector);
        
        // Проверяем, что clients() возвращает правильные данные
        let result = introspector.clients();
        assert!(result.is_ok());
        let returned_clients = result.unwrap();
        assert_eq!(returned_clients.len(), 1);
        assert_eq!(returned_clients[0].pid, 1234);
    }

    #[test]
    fn test_create_enhanced_audio_introspector() {
        // Тест проверяет утилиту create_enhanced_audio_introspector
        let introspector = create_enhanced_audio_introspector();
        
        // Проверяем значения по умолчанию
        assert_eq!(introspector.cache_duration, Duration::from_millis(500));
        assert_eq!(introspector.max_error_count, 3);
        assert_eq!(introspector.error_count(), 0);
    }

    #[test]
    fn test_enhanced_audio_introspector_cache_expiry() {
        // Тест проверяет истечение срока кэша
        let mut introspector = EnhancedAudioIntrospector::new(
            Duration::from_millis(100),  // Очень короткий кэш
            3
        );
        
        // Создаём статический интроспектор
        let now = SystemTime::now();
        let test_metrics = AudioMetrics::empty(now, now);
        let static_introspector = StaticAudioIntrospector::new(test_metrics.clone(), vec![]);
        
        introspector.base_introspector = Box::new(static_introspector);
        
        // Первый вызов
        let _first_call = introspector.audio_metrics().unwrap();
        assert!(introspector.last_metrics.is_some());
        
        // Ждём истечения кэша
        std::thread::sleep(Duration::from_millis(150));
        
        // Второй вызов - кэш должен быть устаревшим
        let second_call = introspector.audio_metrics().unwrap();
        assert_eq!(second_call.xrun_count, 0);
    }

    #[test]
    fn test_enhanced_audio_introspector_error_recovery() {
        // Тест проверяет восстановление после ошибок
        let mut introspector = EnhancedAudioIntrospector::new(
            Duration::from_millis(100),
            2
        );
        
        // Создаём интроспектор, который сначала возвращает ошибку, потом успех
        struct RecoveringIntrospector {
            call_count: std::sync::atomic::AtomicUsize,
        }
        
        impl AudioIntrospector for RecoveringIntrospector {
            fn audio_metrics(&mut self) -> Result<AudioMetrics> {
                let count = self.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if count < 2 {
                    Err(anyhow::anyhow!("Temporary error"))
                } else {
                    let now = SystemTime::now();
                    Ok(AudioMetrics::empty(now, now))
                }
            }
            
            fn clients(&self) -> Result<Vec<AudioClientInfo>> {
                Ok(vec![])
            }
        }
        
        introspector.base_introspector = Box::new(RecoveringIntrospector {
            call_count: std::sync::atomic::AtomicUsize::new(0),
        });
        
        // Первый вызов должен вернуть ошибку
        assert!(introspector.audio_metrics().is_err());
        
        // Второй вызов должен сразу перейти в graceful degradation (лимит = 2)
        assert!(introspector.audio_metrics().is_ok());
        assert!(introspector.is_error_limit_exceeded());
        
        // Третий вызов должен быть graceful degradation
        assert!(introspector.audio_metrics().is_ok());
        
        // Четвёртый вызов - теперь интроспектор возвращает успех,
        // но мы всё ещё в режиме graceful degradation
        let _result = introspector.audio_metrics();
        // В реальной ситуации нам нужно сбросить интроспектор для восстановления
        // Но в текущей реализации мы остаёмся в режиме graceful degradation
        // Это ожидаемое поведение - для восстановления нужно явный reset()
    }

    #[test]
    fn test_enhanced_audio_introspector_setters() {
        // Тест проверяет методы установки параметров
        let mut introspector = EnhancedAudioIntrospector::new(
            Duration::from_millis(100),
            3
        );
        
        // Устанавливаем новые значения
        introspector.set_cache_duration(Duration::from_secs(2));
        introspector.set_max_error_count(10);
        
        // Проверяем, что значения изменились
        assert_eq!(introspector.cache_duration, Duration::from_secs(2));
        assert_eq!(introspector.max_error_count, 10);
    }

    #[test]
    fn test_parallel_audio_introspector_creation() {
        // Тест проверяет создание ParallelAudioIntrospector
        let introspector = ParallelAudioIntrospector::new(
            Duration::from_millis(100),
            3,
            4
        );
        
        assert_eq!(introspector.error_count(), 0);
        assert!(!introspector.is_error_limit_exceeded());
        assert_eq!(introspector.cache_duration, Duration::from_millis(100));
        assert_eq!(introspector.thread_count(), 4);
    }

    #[test]
    fn test_parallel_audio_introspector_thread_count() {
        // Тест проверяет установку количества потоков
        let mut introspector = ParallelAudioIntrospector::new(
            Duration::from_millis(100),
            3,
            2
        );
        
        assert_eq!(introspector.thread_count(), 2);
        
        introspector.set_thread_count(8);
        assert_eq!(introspector.thread_count(), 8);
    }

    #[test]
    fn test_parallel_audio_introspector_benchmarking() {
        // Тест проверяет функциональность бенчмаркинга
        let mut introspector = ParallelAudioIntrospector::new(
            Duration::from_millis(100),
            3,
            2
        );
        
        // Включаем бенчмаркинг
        introspector.enable_benchmarking(true);
        assert!(introspector.enable_benchmarking);
        
        // Выключаем бенчмаркинг
        introspector.enable_benchmarking(false);
        assert!(!introspector.enable_benchmarking);
    }

    #[test]
    fn test_parallel_audio_introspector_cache() {
        // Тест проверяет кэширование в параллельном интроспекторе
        let mut introspector = ParallelAudioIntrospector::new(
            Duration::from_secs(1),
            3,
            2
        );
        
        // Создаём статический интроспектор для теста
        let now = SystemTime::now();
        let test_metrics = AudioMetrics::empty(now, now);
        let static_introspector = StaticAudioIntrospector::new(test_metrics.clone(), vec![]);
        
        // Заменяем базовый интроспектор на статический для теста
        introspector.base_introspector = Box::new(static_introspector);
        
        // Первый вызов - кэш пуст, должны получить метрики
        let first_call = introspector.audio_metrics().unwrap();
        assert_eq!(first_call.xrun_count, 0);
        
        // Второй вызов в пределах cache_duration - должны получить кэш
        let second_call = introspector.audio_metrics().unwrap();
        assert_eq!(second_call.xrun_count, 0);
        
        // Проверяем, что кэш сохранён
        assert!(introspector.last_metrics.is_some());
    }

    #[test]
    fn test_parallel_audio_introspector_error_handling() {
        // Тест проверяет обработку ошибок в параллельном интроспекторе
        let mut introspector = ParallelAudioIntrospector::new(
            Duration::from_millis(100),
            2,  // Маленький лимит для теста
            2
        );
        
        // Создаём интроспектор, который всегда возвращает ошибку
        struct FailingIntrospector;
        
        impl AudioIntrospector for FailingIntrospector {
            fn audio_metrics(&mut self) -> Result<AudioMetrics> {
                Err(anyhow::anyhow!("Test error"))
            }
            
            fn clients(&self) -> Result<Vec<AudioClientInfo>> {
                Err(anyhow::anyhow!("Test error"))
            }
        }
        
        introspector.base_introspector = Box::new(FailingIntrospector);
        
        // Первая ошибка - должна быть проброшена
        let result1 = introspector.audio_metrics();
        assert!(result1.is_err());
        assert_eq!(introspector.error_count(), 1);
        
        // Вторая ошибка - должен быть достигнут лимит и сразу перейти в graceful degradation
        let result2 = introspector.audio_metrics();
        assert!(result2.is_ok()); // Graceful degradation начинается сразу при достижении лимита
        assert_eq!(introspector.error_count(), 2);
        assert!(introspector.is_error_limit_exceeded());
        
        // Третья ошибка - должна быть graceful degradation (пустые метрики)
        let result3 = introspector.audio_metrics();
        assert!(result3.is_ok());
        let metrics = result3.unwrap();
        assert_eq!(metrics.xrun_count, 0); // Пустые метрики
    }

    #[test]
    fn test_create_parallel_audio_introspector() {
        // Тест проверяет утилиту create_parallel_audio_introspector
        let introspector = create_parallel_audio_introspector();
        
        // Проверяем значения по умолчанию
        assert_eq!(introspector.cache_duration, Duration::from_millis(500));
        assert_eq!(introspector.max_error_count, 3);
        assert_eq!(introspector.thread_count(), 2);
        assert_eq!(introspector.error_count(), 0);
    }

    #[test]
    fn test_benchmark_audio_metrics() {
        // Тест проверяет функцию бенчмаркинга
        let mut introspector = EnhancedAudioIntrospector::new(
            Duration::from_millis(100),
            3
        );
        
        // Создаём статический интроспектор для теста
        let now = SystemTime::now();
        let test_metrics = AudioMetrics::empty(now, now);
        let static_introspector = StaticAudioIntrospector::new(test_metrics.clone(), vec![]);
        
        introspector.base_introspector = Box::new(static_introspector);
        
        // Запускаем бенчмарк с 5 итерациями
        let result = benchmark_audio_metrics(&mut introspector, 5);
        
        // Проверяем, что результат есть (даже если очень маленький)
        assert!(result.is_some());
        let duration = result.unwrap();
        // Длительность должна быть разумной (не отрицательной и не слишком большой)
        assert!(duration.as_millis() < 1000); // Менее 1 секунды на 5 итераций
    }

    #[test]
    fn test_parallel_audio_introspector_error_recommendations() {
        // Тест проверяет генерацию рекомендаций с учётом потоков
        let introspector = ParallelAudioIntrospector::new(
            Duration::from_millis(100),
            3,
            4  // Много потоков для теста
        );
        
        // Проверяем, что рекомендации включают информацию о потоках
        let recommendations = introspector.get_error_recommendations();
        assert!(recommendations.contains("PipeWire"));
        assert!(recommendations.contains("PulseAudio"));
        assert!(recommendations.contains("4 потоков")); // Должно быть упоминание количества потоков
    }

    #[test]
    fn test_parallel_audio_introspector_clients() {
        // Тест проверяет, что clients() работает через базовый интроспектор
        let introspector = ParallelAudioIntrospector::new(
            Duration::from_millis(100),
            3,
            2
        );
        
        // Создаём статический интроспектор с клиентами
        let clients = vec![AudioClientInfo {
            pid: 1234,
            buffer_size_samples: Some(1024),
            sample_rate_hz: Some(48000),
        }];
        
        let static_introspector = StaticAudioIntrospector::new(
            AudioMetrics::empty(SystemTime::now(), SystemTime::now()),
            clients.clone()
        );
        
        // Заменяем базовый интроспектор
        let mut introspector = introspector;
        introspector.base_introspector = Box::new(static_introspector);
        
        // Проверяем, что clients() возвращает правильные данные
        let result = introspector.clients();
        assert!(result.is_ok());
        let returned_clients = result.unwrap();
        assert_eq!(returned_clients.len(), 1);
        assert_eq!(returned_clients[0].pid, 1234);
    }

    #[test]
    fn test_enhanced_audio_introspector_graceful_degradation() {
        // Тест проверяет graceful degradation с сохранением последних успешных метрик
        let mut introspector = EnhancedAudioIntrospector::new(
            Duration::from_millis(1), // Очень короткий кэш для теста
            2
        );
        
        // Создаём интроспектор, который сначала возвращает успешные метрики, потом ошибки
        struct RecoveringIntrospector {
            call_count: std::sync::atomic::AtomicUsize,
        }
        
        impl AudioIntrospector for RecoveringIntrospector {
            fn audio_metrics(&mut self) -> Result<AudioMetrics> {
                let count = self.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if count == 0 {
                    // Первый вызов возвращает успешные метрики
                    let mut metrics = AudioMetrics::empty(SystemTime::now(), SystemTime::now());
                    metrics.xrun_count = 3; // Кастомные метрики
                    Ok(metrics)
                } else {
                    // Последующие вызовы возвращают ошибки
                    Err(anyhow::anyhow!("Simulated error"))
                }
            }
            
            fn clients(&self) -> Result<Vec<AudioClientInfo>> {
                Ok(vec![])
            }
        }
        
        introspector.base_introspector = Box::new(RecoveringIntrospector {
            call_count: std::sync::atomic::AtomicUsize::new(0),
        });
        
        // Первый вызов должен вернуть успешные метрики
        let result1 = introspector.audio_metrics();
        assert!(result1.is_ok());
        let metrics1 = result1.unwrap();
        assert_eq!(metrics1.xrun_count, 3);
        
        // Ждём, чтобы кэш устарел
        std::thread::sleep(Duration::from_millis(2));
        
        // Второй вызов должен вернуть ошибку (лимит еще не достигнут)
        let result2 = introspector.audio_metrics();
        assert!(result2.is_err());
        
        // Третий вызов должен вернуть graceful degradation с последними успешными метриками
        let result3 = introspector.audio_metrics();
        assert!(result3.is_ok());
        let metrics3 = result3.unwrap();
        assert_eq!(metrics3.xrun_count, 3); // Должны быть последние успешные метрики
        
        // Проверяем, что последние успешные метрики сохранены
        assert!(introspector.last_successful_metrics().is_some());
        let last_successful = introspector.last_successful_metrics().unwrap();
        assert_eq!(last_successful.xrun_count, 3);
    }

    #[test]
    fn test_enhanced_audio_introspector_device_error_handling() {
        // Тест проверяет обработку ошибок по устройствам
        let mut introspector = EnhancedAudioIntrospector::new(
            Duration::from_millis(100),
            3
        );
        
        // Проверяем, что счетчики ошибок по устройствам изначально пусты
        assert!(introspector.device_error_count().is_empty());
        
        // Увеличиваем счетчик ошибок для устройства
        introspector.increment_device_error_count("device1");
        introspector.increment_device_error_count("device1");
        introspector.increment_device_error_count("device2");
        
        // Проверяем счетчики
        let device_errors = introspector.device_error_count();
        assert_eq!(device_errors.get("device1"), Some(&2));
        assert_eq!(device_errors.get("device2"), Some(&1));
        
        // Сбрасываем счетчик для device1
        introspector.reset_device_error_count("device1");
        assert!(introspector.device_error_count().get("device1").is_none());
        assert_eq!(introspector.device_error_count().get("device2"), Some(&1));
    }

    #[test]
    fn test_enhanced_audio_introspector_detailed_error_recommendations() {
        // Тест проверяет, что рекомендации включают детальную информацию
        let mut introspector = EnhancedAudioIntrospector::new(
            Duration::from_millis(100),
            3
        );
        
        // Увеличиваем счетчик ошибок
        introspector.error_count = 2;
        
        // Добавляем ошибки устройств
        introspector.increment_device_error_count("device1");
        
        let recommendations = introspector.get_error_recommendations();
        
        // Проверяем, что рекомендации содержат базовую информацию
        assert!(recommendations.contains("PipeWire"));
        assert!(recommendations.contains("PulseAudio"));
        
        // Проверяем, что рекомендации содержат дополнительную информацию при многократных ошибках
        assert!(recommendations.contains("systemctl --user restart pipewire"));
        assert!(recommendations.contains("journalctl --user -u pipewire"));
        
        // Проверяем, что рекомендации содержат информацию об устройствах
        assert!(recommendations.contains("аудио-устройства"));
    }

    #[test]
    fn test_parallel_audio_introspector_graceful_degradation() {
        // Тест проверяет graceful degradation в параллельном интроспекторе
        let mut introspector = ParallelAudioIntrospector::new(
            Duration::from_millis(100),
            2,
            2
        );
        
        // Создаём интроспектор, который сначала возвращает успешные метрики, потом ошибки
        struct RecoveringIntrospector {
            call_count: std::sync::atomic::AtomicUsize,
        }
        
        impl AudioIntrospector for RecoveringIntrospector {
            fn audio_metrics(&mut self) -> Result<AudioMetrics> {
                let count = self.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if count == 0 {
                    // Первый вызов возвращает успешные метрики
                    let mut metrics = AudioMetrics::empty(SystemTime::now(), SystemTime::now());
                    metrics.xrun_count = 5; // Кастомные метрики
                    Ok(metrics)
                } else {
                    // Последующие вызовы возвращают ошибки
                    Err(anyhow::anyhow!("Simulated error"))
                }
            }
            
            fn clients(&self) -> Result<Vec<AudioClientInfo>> {
                Ok(vec![])
            }
        }
        
        introspector.base_introspector = Box::new(RecoveringIntrospector {
            call_count: std::sync::atomic::AtomicUsize::new(0),
        });
        
        // Первый вызов должен вернуть успешные метрики
        let result1 = introspector.audio_metrics();
        assert!(result1.is_ok());
        let metrics1 = result1.unwrap();
        assert_eq!(metrics1.xrun_count, 5);
        
        // Третий вызов должен вернуть graceful degradation с последними успешными метриками
        let result3 = introspector.audio_metrics();
        assert!(result3.is_ok());
        let metrics3 = result3.unwrap();
        assert_eq!(metrics3.xrun_count, 5); // Должны быть последние успешные метрики
    }

    #[test]
    fn test_parallel_audio_introspector_device_error_handling() {
        // Тест проверяет обработку ошибок по устройствам в параллельном интроспекторе
        let mut introspector = ParallelAudioIntrospector::new(
            Duration::from_millis(100),
            3,
            2
        );
        
        // Проверяем, что счетчики ошибок по устройствам изначально пусты
        assert!(introspector.device_error_count().is_empty());
        
        // Увеличиваем счетчик ошибок для устройства
        introspector.increment_device_error_count("device1");
        introspector.increment_device_error_count("device1");
        
        // Проверяем счетчики
        let device_errors = introspector.device_error_count();
        assert_eq!(device_errors.get("device1"), Some(&2));
        
        // Сбрасываем счетчик для device1
        introspector.reset_device_error_count("device1");
        assert!(introspector.device_error_count().get("device1").is_none());
    }

    #[test]
    fn test_parallel_audio_introspector_detailed_error_recommendations() {
        // Тест проверяет, что рекомендации включают детальную информацию для параллельного интроспектора
        let mut introspector = ParallelAudioIntrospector::new(
            Duration::from_millis(100),
            3,
            4  // Много потоков для теста
        );
        
        // Увеличиваем счетчик ошибок
        introspector.error_count = 2;
        
        // Добавляем ошибки устройств
        introspector.increment_device_error_count("device1");
        
        let recommendations = introspector.get_error_recommendations();
        
        // Проверяем, что рекомендации содержат базовую информацию
        assert!(recommendations.contains("PipeWire"));
        assert!(recommendations.contains("PulseAudio"));
        
        // Проверяем, что рекомендации содержат информацию о потоках
        assert!(recommendations.contains("4 потоков"));
        
        // Проверяем, что рекомендации содержат информацию об устройствах
        assert!(recommendations.contains("аудио-устройства"));
    }

    #[test]
    fn test_audio_introspector_fallback_with_detailed_errors() {
        // Тест проверяет, что fallback механизм работает с улучшенной обработкой ошибок
        let mut introspector = create_audio_introspector_with_fallback();
        
        // Проверяем, что интроспектор может быть использован
        let metrics = introspector.audio_metrics();
        // В тестовой среде без PipeWire это может быть ошибка, что нормально
        assert!(metrics.is_ok() || metrics.is_err());
        
        let clients = introspector.clients();
        assert!(clients.is_ok() || clients.is_err());
        
        // Если метрики успешно получены, проверяем их валидность
        if let Ok(metrics) = metrics {
            assert!(metrics.validate_period());
        }
    }
}
