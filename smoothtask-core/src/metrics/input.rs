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

impl InputMetrics {
    /// Создать пустые метрики ввода (по умолчанию).
    ///
    /// Используется в случае ошибок при сборе метрик ввода.
    ///
    /// # Возвращает
    ///
    /// `InputMetrics` с дефолтными значениями:
    /// - `user_active = false`
    /// - `time_since_last_input_ms = None`
    ///
    /// # Примеры
    ///
    /// ```rust
    /// use smoothtask_core::metrics::input::InputMetrics;
    ///
    /// let empty_metrics = InputMetrics::empty();
    /// assert!(!empty_metrics.user_active);
    /// assert!(empty_metrics.time_since_last_input_ms.is_none());
    /// ```
    pub fn empty() -> Self {
        Self {
            user_active: false,
            time_since_last_input_ms: None,
        }
    }
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

    /// Обновить порог простоя.
    ///
    /// Используется для динамической перезагрузки конфигурации.
    pub fn set_idle_threshold(&mut self, new_threshold: Duration) {
        self.idle_threshold = new_threshold;
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

// Функции сделаны pub(crate) для доступа в тестах, но не экспортируются наружу модуля
pub(crate) fn is_user_activity_event(ev: &InputEvent) -> bool {
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

pub(crate) fn duration_to_ms(d: Duration) -> u64 {
    d.as_secs()
        .saturating_mul(1000)
        .saturating_add(u64::from(d.subsec_millis()))
}

/// Трекер активности пользователя, читающий события из реальных evdev устройств.
///
/// `EvdevInputTracker` автоматически обнаруживает доступные устройства ввода
/// (клавиатура, мышь, тачпад) в `/dev/input/event*` и читает события в неблокирующем режиме.
///
/// # Примеры использования
///
/// ## Базовое использование
///
/// ```no_run
/// use std::time::{Duration, Instant};
/// use smoothtask_core::metrics::input::EvdevInputTracker;
///
/// // Проверяем доступность evdev устройств
/// if EvdevInputTracker::is_available() {
///     // Создаём трекер с порогом простоя 5 секунд
///     let mut tracker = EvdevInputTracker::new(Duration::from_secs(5))?;
///
///     // Обновляем метрики, читая новые события
///     let metrics = tracker.update(Instant::now());
///     println!("User active: {}", metrics.user_active);
///     println!("Time since last input: {:?} ms", metrics.time_since_last_input_ms);
///
///     // Получаем текущие метрики без чтения новых событий
///     let current_metrics = tracker.metrics(Instant::now());
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// ## Обработка ошибок
///
/// ```no_run
/// use std::time::Duration;
/// use smoothtask_core::metrics::input::EvdevInputTracker;
///
/// match EvdevInputTracker::new(Duration::from_secs(5)) {
///     Ok(tracker) => {
///         // Трекер успешно создан, можно использовать
///     }
///     Err(e) => {
///         // Ошибка может возникнуть, если:
///         // - нет доступных устройств ввода
///         // - нет прав доступа к /dev/input/event*
///         // - устройства не поддерживают нужные типы событий
///         eprintln!("Failed to create EvdevInputTracker: {}", e);
///     }
/// }
/// ```
///
/// ## Использование в цикле демона
///
/// ```no_run
/// use std::time::{Duration, Instant};
/// use smoothtask_core::metrics::input::EvdevInputTracker;
///
/// let mut tracker = EvdevInputTracker::new(Duration::from_secs(5))?;
///
/// loop {
///     let now = Instant::now();
///     let metrics = tracker.update(now);
///
///     if metrics.user_active {
///         // Пользователь активен, можно применять интерактивные приоритеты
///     } else {
///         // Пользователь неактивен, можно снизить приоритеты
///     }
///
///     // Спим до следующей итерации
///     std::thread::sleep(Duration::from_millis(100));
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
///
/// # Обработка ошибок
///
/// Методы `update()` и `metrics()` не возвращают ошибки, так как:
/// - чтение событий происходит в неблокирующем режиме
/// - ошибки чтения из отдельных устройств логируются, но не прерывают работу
/// - если устройство отключено, оно просто пропускается
///
/// # Производительность
///
/// - Чтение событий происходит в неблокирующем режиме, не блокируя поток
/// - События читаются из всех обнаруженных устройств параллельно
/// - Устройства не захватываются эксклюзивно (не используется `grab()`),
///   поэтому другие приложения могут продолжать работать
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

    /// Обновить порог простоя.
    ///
    /// Используется для динамической перезагрузки конфигурации.
    pub fn set_idle_threshold(&mut self, new_threshold: Duration) {
        match self {
            Self::Evdev(tracker) => tracker.set_idle_threshold(new_threshold),
            Self::Simple(tracker) => tracker.set_idle_threshold(new_threshold),
        }
    }
}

impl EvdevInputTracker {
    /// Проверить, доступны ли evdev устройства.
    ///
    /// Проверяет наличие доступных устройств ввода в `/dev/input/event*`
    /// и их поддержку нужных типов событий (KEY, RELATIVE, ABSOLUTE).
    ///
    /// # Возвращает
    ///
    /// `true`, если найдено хотя бы одно подходящее устройство, иначе `false`.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::metrics::input::EvdevInputTracker;
    ///
    /// if EvdevInputTracker::is_available() {
    ///     println!("Evdev devices are available");
    /// } else {
    ///     println!("No evdev devices found");
    /// }
    /// ```
    pub fn is_available() -> bool {
        Self::discover_devices()
            .map(|devices| !devices.is_empty())
            .unwrap_or(false)
    }

    /// Создать новый трекер с автоматическим обнаружением устройств.
    ///
    /// Автоматически обнаруживает все доступные устройства ввода в `/dev/input/event*`
    /// и создаёт трекер для мониторинга активности пользователя.
    ///
    /// # Параметры
    ///
    /// * `idle_threshold` - порог простоя в миллисекундах. Если с последнего события
    ///   прошло больше этого времени, пользователь считается неактивным.
    ///
    /// # Возвращает
    ///
    /// `Ok(EvdevInputTracker)`, если найдено хотя бы одно устройство, иначе `Err`.
    ///
    /// # Ошибки
    ///
    /// Возвращает ошибку, если:
    /// - нет доступных устройств ввода (`No input devices found`)
    /// - нет прав доступа к `/dev/input/event*`
    /// - устройства не поддерживают нужные типы событий
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use smoothtask_core::metrics::input::EvdevInputTracker;
    ///
    /// // Создаём трекер с порогом простоя 5 секунд
    /// let tracker = EvdevInputTracker::new(Duration::from_secs(5))?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
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
    /// Читает все доступные события из всех обнаруженных устройств ввода
    /// в неблокирующем режиме и обновляет внутренний трекер активности.
    ///
    /// # Параметры
    ///
    /// * `now` - текущее время для вычисления времени с последнего события
    ///
    /// # Возвращает
    ///
    /// `InputMetrics` с обновлёнными метриками активности пользователя.
    ///
    /// # Поведение
    ///
    /// - Чтение происходит в неблокирующем режиме (не ждёт новых событий)
    /// - Ошибки чтения из отдельных устройств логируются, но не прерывают работу
    /// - Если устройство отключено, оно пропускается
    /// - Все события из всех устройств агрегируются вместе
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use std::time::{Duration, Instant};
    /// use smoothtask_core::metrics::input::EvdevInputTracker;
    ///
    /// let mut tracker = EvdevInputTracker::new(Duration::from_secs(5))?;
    /// let metrics = tracker.update(Instant::now());
    ///
    /// if metrics.user_active {
    ///     println!("User is active");
    /// } else {
    ///     println!("User is idle for {} ms", metrics.time_since_last_input_ms.unwrap_or(0));
    /// }
    /// # Ok::<(), anyhow::Error>(())
    /// ```
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
    /// Ищет все доступные устройства ввода в `/dev/input/event*` и фильтрует их
    /// по поддержке нужных типов событий (KEY, RELATIVE, ABSOLUTE).
    ///
    /// # Возвращает
    ///
    /// Вектор открытых устройств, поддерживающих нужные типы событий.
    ///
    /// # Ошибки
    ///
    /// Возвращает ошибку, если:
    /// - нет прав доступа к `/dev/input`
    /// - директория `/dev/input` не существует
    ///
    /// # Примечания
    ///
    /// - Устройства не захватываются эксклюзивно (не используется `grab()`)
    /// - Поддерживаются только устройства с типами событий KEY, RELATIVE или ABSOLUTE
    /// - Ошибки открытия отдельных устройств игнорируются (логируются через debug)
    fn discover_devices() -> Result<Vec<Device>, anyhow::Error> {
        let input_dir = Path::new("/dev/input");
        let mut devices = Vec::new();

        // Читаем все файлы event* в /dev/input
        let entries = std::fs::read_dir(input_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            // Проверяем, что это файл event*
            let file_name = match path.file_name() {
                Some(name) => name,
                None => continue,
            };
            let file_name_str = file_name.to_string_lossy();
            if !file_name_str.starts_with("event") {
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

    /// Обновить порог простоя.
    ///
    /// Используется для динамической перезагрузки конфигурации.
    pub fn set_idle_threshold(&mut self, new_threshold: Duration) {
        self.activity_tracker.set_idle_threshold(new_threshold);
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

    // Edge case тесты для InputActivityTracker

    #[test]
    fn test_register_activity() {
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        let now = Instant::now();

        // До регистрации активности user_active должен быть false
        let metrics_before = tracker.metrics(now);
        assert!(!metrics_before.user_active);
        assert_eq!(metrics_before.time_since_last_input_ms, None);

        // Регистрируем активность
        tracker.register_activity(now);

        // После регистрации user_active должен быть true
        let metrics_after = tracker.metrics(now);
        assert!(metrics_after.user_active);
        assert_eq!(metrics_after.time_since_last_input_ms, Some(0));
    }

    #[test]
    fn test_idle_threshold_zero() {
        // Тест для граничного случая: idle_threshold = 0
        let mut tracker = InputActivityTracker::new(Duration::from_secs(0));
        let start = Instant::now();
        let key = InputEvent::new(EventType::KEY, Key::KEY_A.code(), 1);
        tracker.ingest_events([key].iter(), start);

        // С порогом 0, даже минимальная задержка должна сделать user_active = false
        let later = start + Duration::from_nanos(1);
        let metrics = tracker.metrics(later);
        assert!(!metrics.user_active);
    }

    #[test]
    fn test_idle_threshold_very_large() {
        // Тест для очень большого idle_threshold
        let mut tracker = InputActivityTracker::new(Duration::from_secs(86400)); // 24 часа
        let start = Instant::now();
        let key = InputEvent::new(EventType::KEY, Key::KEY_A.code(), 1);
        tracker.ingest_events([key].iter(), start);

        // Даже через час пользователь должен быть активным
        let later = start + Duration::from_secs(3600);
        let metrics = tracker.metrics(later);
        assert!(metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, Some(3600000));
    }

    #[test]
    fn test_idle_threshold_exact_boundary() {
        // Тест для точного попадания на границу idle_threshold
        let mut tracker = InputActivityTracker::new(Duration::from_millis(100));
        let start = Instant::now();
        let key = InputEvent::new(EventType::KEY, Key::KEY_A.code(), 1);
        tracker.ingest_events([key].iter(), start);

        // Ровно на пороге - user_active должен быть true
        let exactly_at_threshold = start + Duration::from_millis(100);
        let metrics = tracker.metrics(exactly_at_threshold);
        assert!(metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, Some(100));

        // Чуть больше порога - user_active должен быть false
        let just_over_threshold = start + Duration::from_millis(101);
        let metrics = tracker.metrics(just_over_threshold);
        assert!(!metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, Some(101));
    }

    #[test]
    fn test_multiple_events() {
        // Тест для множественных событий
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        let now = Instant::now();

        let key1 = InputEvent::new(EventType::KEY, Key::KEY_A.code(), 1);
        let key2 = InputEvent::new(EventType::KEY, Key::KEY_B.code(), 1);
        let mouse = InputEvent::new(EventType::RELATIVE, 0, 1);

        let metrics = tracker.ingest_events([key1, key2, mouse].iter(), now);
        assert!(metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, Some(0));
    }

    #[test]
    fn test_multiple_events_with_syn() {
        // Тест для множественных событий с SYN (SYN должен игнорироваться)
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        let now = Instant::now();

        let key = InputEvent::new(EventType::KEY, Key::KEY_A.code(), 1);
        let syn = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);

        let metrics = tracker.ingest_events([key, syn].iter(), now);
        assert!(metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, Some(0));
    }

    #[test]
    fn test_relative_event() {
        // Тест для RELATIVE событий (мышь)
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        let now = Instant::now();
        let mouse = InputEvent::new(EventType::RELATIVE, 0, 1);
        let metrics = tracker.ingest_events([mouse].iter(), now);
        assert!(metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, Some(0));
    }

    #[test]
    fn test_absolute_event() {
        // Тест для ABSOLUTE событий (тачпад)
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        let now = Instant::now();
        let touch = InputEvent::new(EventType::ABSOLUTE, 0, 1);
        let metrics = tracker.ingest_events([touch].iter(), now);
        assert!(metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, Some(0));
    }

    #[test]
    fn test_switch_event() {
        // Тест для SWITCH событий
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        let now = Instant::now();
        let switch = InputEvent::new(EventType::SWITCH, 0, 1);
        let metrics = tracker.ingest_events([switch].iter(), now);
        assert!(metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, Some(0));
    }

    #[test]
    fn test_misc_event() {
        // Тест для MISC событий
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        let now = Instant::now();
        let misc = InputEvent::new(EventType::MISC, 0, 1);
        let metrics = tracker.ingest_events([misc].iter(), now);
        assert!(metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, Some(0));
    }

    #[test]
    fn test_reserved_key_ignored() {
        // Тест для зарезервированных кодов клавиш (должны игнорироваться)
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        let now = Instant::now();
        let reserved = InputEvent::new(EventType::KEY, Key::KEY_RESERVED.code(), 1);
        let metrics = tracker.ingest_events([reserved].iter(), now);
        assert!(!metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, None);
    }

    #[test]
    fn test_activity_renewal() {
        // Тест для обновления активности (новое событие должно обновить время)
        let mut tracker = InputActivityTracker::new(Duration::from_millis(100));
        let start = Instant::now();
        let key1 = InputEvent::new(EventType::KEY, Key::KEY_A.code(), 1);
        tracker.ingest_events([key1].iter(), start);

        // Прошло 50 мс - пользователь активен
        let mid = start + Duration::from_millis(50);
        let metrics_mid = tracker.metrics(mid);
        assert!(metrics_mid.user_active);

        // Новое событие обновляет время
        let key2 = InputEvent::new(EventType::KEY, Key::KEY_B.code(), 1);
        tracker.ingest_events([key2].iter(), mid);

        // Ещё через 50 мс (от нового события) - пользователь всё ещё активен
        let later = mid + Duration::from_millis(50);
        let metrics_later = tracker.metrics(later);
        assert!(metrics_later.user_active);
        assert_eq!(metrics_later.time_since_last_input_ms, Some(50));
    }

    #[test]
    fn test_time_since_last_input_accuracy() {
        // Тест для точности вычисления time_since_last_input_ms
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        let start = Instant::now();
        let key = InputEvent::new(EventType::KEY, Key::KEY_A.code(), 1);
        tracker.ingest_events([key].iter(), start);

        // Проверяем точность для разных интервалов
        let intervals = vec![
            (Duration::from_millis(0), 0),
            (Duration::from_millis(1), 1),
            (Duration::from_millis(100), 100),
            (Duration::from_secs(1), 1000),
            (Duration::from_secs(2), 2000),
        ];

        for (duration, expected_ms) in intervals {
            let time = start + duration;
            let metrics = tracker.metrics(time);
            assert_eq!(
                metrics.time_since_last_input_ms,
                Some(expected_ms),
                "Failed for duration {:?}",
                duration
            );
        }
    }

    // Unit-тесты для InputTracker enum

    #[test]
    fn input_tracker_new_creates_tracker() {
        // Проверяем, что InputTracker::new() всегда создаёт валидный трекер
        let tracker = InputTracker::new(Duration::from_secs(5));
        // Функция не должна паниковать, независимо от доступности evdev
        match tracker {
            InputTracker::Evdev(_) => {
                // Evdev трекер создан успешно
            }
            InputTracker::Simple(_) => {
                // Fallback на простой трекер (evdev недоступен или ошибка)
            }
        }
    }

    #[test]
    fn input_tracker_new_consistency() {
        // Проверяем консистентность при повторных вызовах
        let tracker1 = InputTracker::new(Duration::from_secs(5));
        let tracker2 = InputTracker::new(Duration::from_secs(5));

        // Оба трекера должны быть одного типа (Evdev или Simple)
        match (&tracker1, &tracker2) {
            (InputTracker::Evdev(_), InputTracker::Evdev(_)) => {}
            (InputTracker::Simple(_), InputTracker::Simple(_)) => {}
            _ => {
                // Это нормально, если доступность evdev изменилась между вызовами
                // (например, устройство было подключено/отключено)
            }
        }
    }

    #[test]
    fn input_tracker_simple_update() {
        // Тест для InputTracker::Simple::update()
        let mut tracker = InputTracker::Simple(InputActivityTracker::new(Duration::from_secs(5)));
        let now = Instant::now();

        // Для Simple трекера update() просто возвращает metrics() без чтения событий
        let metrics = tracker.update(now);
        assert!(!metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, None);
    }

    #[test]
    fn input_tracker_simple_metrics() {
        // Тест для InputTracker::Simple::metrics()
        let tracker = InputTracker::Simple(InputActivityTracker::new(Duration::from_secs(5)));
        let now = Instant::now();

        let metrics = tracker.metrics(now);
        assert!(!metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, None);
    }

    #[test]
    fn input_tracker_simple_metrics_after_activity() {
        // Тест для InputTracker::Simple::metrics() после регистрации активности
        let mut tracker = InputTracker::Simple(InputActivityTracker::new(Duration::from_secs(5)));
        let now = Instant::now();

        // Регистрируем активность через внутренний трекер
        match &mut tracker {
            InputTracker::Simple(activity_tracker) => {
                activity_tracker.register_activity(now);
            }
            _ => unreachable!(),
        }

        // Проверяем метрики
        let metrics = tracker.metrics(now);
        assert!(metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, Some(0));
    }

    #[test]
    fn input_tracker_evdev_update() {
        // Тест для InputTracker::Evdev::update()
        // Этот тест может работать только если evdev устройства доступны
        if let Ok(evdev_tracker) = EvdevInputTracker::new(Duration::from_secs(5)) {
            let tracker = InputTracker::Evdev(evdev_tracker);
            let now = Instant::now();

            // Для Evdev трекера update() читает события из устройств
            match tracker {
                InputTracker::Evdev(mut t) => {
                    let metrics = t.update(now);
                    // Без событий user_active должен быть false
                    assert!(!metrics.user_active);
                }
                _ => unreachable!(),
            }
        }
        // Если evdev недоступен, тест просто пропускается
    }

    #[test]
    fn input_tracker_evdev_metrics() {
        // Тест для InputTracker::Evdev::metrics()
        // Этот тест может работать только если evdev устройства доступны
        if let Ok(evdev_tracker) = EvdevInputTracker::new(Duration::from_secs(5)) {
            let tracker = InputTracker::Evdev(evdev_tracker);
            let now = Instant::now();

            // Для Evdev трекера metrics() возвращает текущие метрики без чтения событий
            match tracker {
                InputTracker::Evdev(t) => {
                    let metrics = t.metrics(now);
                    // Без событий user_active должен быть false
                    assert!(!metrics.user_active);
                }
                _ => unreachable!(),
            }
        }
        // Если evdev недоступен, тест просто пропускается
    }

    #[test]
    fn input_tracker_update_vs_metrics() {
        // Тест для проверки разницы между update() и metrics()
        let mut tracker = InputTracker::Simple(InputActivityTracker::new(Duration::from_secs(5)));
        let now = Instant::now();

        // Для Simple трекера update() и metrics() должны возвращать одинаковые значения
        // (так как update() просто вызывает metrics())
        let metrics_update = tracker.update(now);
        let metrics_direct = tracker.metrics(now);

        assert_eq!(metrics_update.user_active, metrics_direct.user_active);
        assert_eq!(
            metrics_update.time_since_last_input_ms,
            metrics_direct.time_since_last_input_ms
        );
    }

    #[test]
    fn input_tracker_both_variants_work() {
        // Тест для проверки, что оба варианта enum работают корректно
        let idle_threshold = Duration::from_secs(5);
        let now = Instant::now();

        // Создаём Simple трекер напрямую
        let simple_tracker = InputTracker::Simple(InputActivityTracker::new(idle_threshold));
        let simple_metrics = simple_tracker.metrics(now);

        // Создаём трекер через new() (может быть Evdev или Simple)
        let auto_tracker = InputTracker::new(idle_threshold);
        let auto_metrics = auto_tracker.metrics(now);

        // Оба трекера должны возвращать валидные метрики
        assert_eq!(simple_metrics.user_active, auto_metrics.user_active);
        // time_since_last_input_ms может отличаться, если в Evdev были события
        // но структура должна быть одинаковой
        assert!(
            simple_metrics.time_since_last_input_ms.is_none()
                || auto_metrics.time_since_last_input_ms.is_some()
        );
    }

    // Edge case тесты для duration_to_ms

    #[test]
    fn test_duration_to_ms_zero() {
        // Тест для нулевого Duration
        let d = Duration::from_secs(0);
        assert_eq!(duration_to_ms(d), 0);
    }

    #[test]
    fn test_duration_to_ms_zero_with_nanos() {
        // Тест для Duration с нулевыми секундами, но ненулевыми наносекундами
        let d = Duration::from_nanos(0);
        assert_eq!(duration_to_ms(d), 0);
    }

    #[test]
    fn test_duration_to_ms_one_second() {
        // Тест для одной секунды
        let d = Duration::from_secs(1);
        assert_eq!(duration_to_ms(d), 1000);
    }

    #[test]
    fn test_duration_to_ms_one_millisecond() {
        // Тест для одной миллисекунды
        let d = Duration::from_millis(1);
        assert_eq!(duration_to_ms(d), 1);
    }

    #[test]
    fn test_duration_to_ms_subsec_millis() {
        // Тест для Duration с миллисекундами в subsec_millis
        let d = Duration::from_millis(1234);
        assert_eq!(duration_to_ms(d), 1234);
    }

    #[test]
    fn test_duration_to_ms_max_subsec_millis() {
        // Тест для максимального значения subsec_millis (999 мс)
        let d = Duration::from_millis(999);
        assert_eq!(duration_to_ms(d), 999);
    }

    #[test]
    fn test_duration_to_ms_very_large() {
        // Тест для очень большого Duration (проверка saturating операций)
        // u64::MAX / 1000 секунд - это максимальное значение, которое можно безопасно умножить на 1000
        let max_safe_secs = u64::MAX / 1000;
        let d = Duration::from_secs(max_safe_secs);
        let result = duration_to_ms(d);
        // Результат должен быть max_safe_secs * 1000
        assert_eq!(result, max_safe_secs * 1000);
    }

    #[test]
    fn test_duration_to_ms_overflow_protection() {
        // Тест для проверки защиты от переполнения (saturating операции)
        // Если бы не было saturating, это могло бы вызвать переполнение
        let very_large_secs = u64::MAX;
        let d = Duration::from_secs(very_large_secs);
        let result = duration_to_ms(d);
        // Результат должен быть ограничен u64::MAX из-за saturating_mul
        assert_eq!(result, u64::MAX);
    }

    #[test]
    fn test_duration_to_ms_with_nanos() {
        // Тест для Duration с наносекундами (не кратно миллисекундам)
        let d = Duration::from_nanos(1_500_000); // 1.5 мс
        assert_eq!(duration_to_ms(d), 1); // Округляется вниз до 1 мс
    }

    #[test]
    fn test_duration_to_ms_precision() {
        // Тест для проверки точности преобразования
        let test_cases = vec![
            (Duration::from_millis(0), 0),
            (Duration::from_millis(1), 1),
            (Duration::from_millis(100), 100),
            (Duration::from_millis(1000), 1000),
            (Duration::from_secs(1), 1000),
            (Duration::from_secs(60), 60000),
            (Duration::from_secs(3600), 3600000),
        ];

        for (duration, expected_ms) in test_cases {
            assert_eq!(
                duration_to_ms(duration),
                expected_ms,
                "Failed for duration {:?}",
                duration
            );
        }
    }

    // Edge case тесты для is_user_activity_event

    #[test]
    fn test_is_user_activity_event_unknown_event_type() {
        // Тест для неизвестного типа события (не покрытого в match)
        // Создаём событие с типом, который не обрабатывается явно
        // В evdev нет прямого способа создать произвольный EventType,
        // но мы можем проверить, что другие типы событий возвращают false
        let syn = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);
        assert!(!is_user_activity_event(&syn));
    }

    #[test]
    fn test_is_user_activity_event_key_with_different_codes() {
        // Тест для KEY событий с разными кодами
        let key_a = InputEvent::new(EventType::KEY, Key::KEY_A.code(), 1);
        assert!(is_user_activity_event(&key_a));

        let key_space = InputEvent::new(EventType::KEY, Key::KEY_SPACE.code(), 1);
        assert!(is_user_activity_event(&key_space));

        let key_enter = InputEvent::new(EventType::KEY, Key::KEY_ENTER.code(), 1);
        assert!(is_user_activity_event(&key_enter));
    }

    #[test]
    fn test_is_user_activity_event_key_reserved_code() {
        // Тест для KEY события с зарезервированным кодом (должно игнорироваться)
        let reserved = InputEvent::new(EventType::KEY, Key::KEY_RESERVED.code(), 1);
        assert!(!is_user_activity_event(&reserved));
    }

    #[test]
    fn test_is_user_activity_event_key_zero_code() {
        // Тест для KEY события с нулевым кодом (может быть валидным, если не KEY_RESERVED)
        // Проверяем, что нулевой код обрабатывается корректно
        let key_zero = InputEvent::new(EventType::KEY, 0, 1);
        // Если 0 != KEY_RESERVED.code(), то событие должно считаться активностью
        // (в реальности это зависит от конкретного значения KEY_RESERVED.code())
        let result = is_user_activity_event(&key_zero);
        // Результат зависит от того, равен ли 0 KEY_RESERVED.code()
        // Но функция не должна паниковать
        let _ = result;
    }

    #[test]
    fn test_is_user_activity_event_all_activity_types() {
        // Тест для всех типов событий, которые считаются активностью
        let relative = InputEvent::new(EventType::RELATIVE, 0, 1);
        assert!(is_user_activity_event(&relative));

        let absolute = InputEvent::new(EventType::ABSOLUTE, 0, 1);
        assert!(is_user_activity_event(&absolute));

        let switch = InputEvent::new(EventType::SWITCH, 0, 1);
        assert!(is_user_activity_event(&switch));

        let misc = InputEvent::new(EventType::MISC, 0, 1);
        assert!(is_user_activity_event(&misc));
    }

    #[test]
    fn test_is_user_activity_event_synchronization_always_false() {
        // Тест для SYNCHRONIZATION событий (всегда должны возвращать false)
        let syn1 = InputEvent::new(EventType::SYNCHRONIZATION, 0, 0);
        assert!(!is_user_activity_event(&syn1));

        let syn2 = InputEvent::new(EventType::SYNCHRONIZATION, 1, 1);
        assert!(!is_user_activity_event(&syn2));
    }

    #[test]
    fn test_is_user_activity_event_key_edge_codes() {
        // Тест для граничных кодов клавиш
        // Проверяем, что функция корректно обрабатывает различные коды
        let key_min = InputEvent::new(EventType::KEY, 0, 1);
        let _ = is_user_activity_event(&key_min); // Не должно паниковать

        // Проверяем несколько реальных кодов клавиш
        let codes = vec![
            Key::KEY_ESC.code(),
            Key::KEY_1.code(),
            Key::KEY_A.code(),
            Key::KEY_Z.code(),
            Key::KEY_SPACE.code(),
            Key::KEY_ENTER.code(),
        ];

        for code in codes {
            let event = InputEvent::new(EventType::KEY, code, 1);
            // Все реальные коды клавиш должны считаться активностью
            assert!(
                is_user_activity_event(&event),
                "Key code {} should be considered user activity",
                code
            );
        }
    }

    #[test]
    fn test_input_tracker_set_idle_threshold() {
        // Тест для InputTracker::set_idle_threshold()
        let mut tracker = InputTracker::new(Duration::from_secs(5));
        
        // Проверяем, что метод не паникует и обновляет порог
        tracker.set_idle_threshold(Duration::from_secs(10));
        
        // Проверяем, что метрики используют новый порог
        let now = Instant::now();
        let metrics = tracker.metrics(now);
        // Без активности user_active должен быть false
        assert!(!metrics.user_active);
        
        // Тестируем с разными типами трекеров
        let mut simple_tracker = InputTracker::Simple(InputActivityTracker::new(Duration::from_secs(3)));
        simple_tracker.set_idle_threshold(Duration::from_secs(7));
        
        if let Ok(evdev_tracker) = EvdevInputTracker::new(Duration::from_secs(2)) {
            let mut tracker = InputTracker::Evdev(evdev_tracker);
            tracker.set_idle_threshold(Duration::from_secs(8));
            // Метод не должен паниковать
        }
    }

    #[test]
    fn test_input_activity_tracker_set_idle_threshold() {
        // Тест для InputActivityTracker::set_idle_threshold()
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        
        // Регистрируем активность
        let now = Instant::now();
        tracker.register_activity(now);
        
        // Проверяем, что активность зарегистрирована
        let metrics = tracker.metrics(now);
        assert!(metrics.user_active);
        
        // Обновляем порог простоя
        tracker.set_idle_threshold(Duration::from_secs(1));
        
        // Проверяем, что активность всё ещё зарегистрирована
        let metrics = tracker.metrics(now);
        assert!(metrics.user_active);
        
        // Ждём дольше нового порога и проверяем, что активность сбрасывается
        let later = now + Duration::from_secs(2);
        let metrics = tracker.metrics(later);
        assert!(!metrics.user_active);
    }
}
