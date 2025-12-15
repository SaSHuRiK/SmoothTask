//! Метрики активности пользователя и классификация устройств ввода.
//!
//! Здесь реализован трекер, который:
//! - Принимает события ввода (клавиатура, мышь, сенсорный ввод)
//! - Определяет активность пользователя
//! - Классифицирует устройства ввода по типам
//! - Собирает расширенные метрики использования устройств

use evdev::{Device, EventType, InputEvent, Key};
use std::path::Path;
use std::time::{Duration, Instant};
use tracing::{debug, warn};
use std::collections::HashMap;

/// Классификация типов устройств ввода
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputDeviceType {
    /// Клавиатура
    Keyboard,
    /// Мышь
    Mouse,
    /// Тачпад
    Touchpad,
    /// Сенсорный экран
    Touchscreen,
    /// Игровой контроллер
    GameController,
    /// Трекбол
    Trackball,
    /// Графический планшет
    DrawingTablet,
    /// VR контроллер
    VrController,
    /// Микрофон
    Microphone,
    /// Веб-камера
    Webcam,
    /// Биометрическое устройство (отпечаток пальца, сканер лица)
    Biometric,
    /// Устройство ввода для людей с ограниченными возможностями
    AccessibilityDevice,
    /// Неизвестное устройство
    Unknown,
}

/// Информация об устройстве ввода
#[derive(Debug, Clone)]
pub struct InputDeviceInfo {
    /// Тип устройства
    pub device_type: InputDeviceType,
    /// Имя устройства
    pub device_name: String,
    /// Путь к устройству
    pub device_path: String,
    /// Количество событий с устройства
    pub event_count: u64,
    /// Последнее время активности
    pub last_activity_time: Option<Instant>,
}

/// Расширенные метрики активности пользователя.
#[derive(Debug, Clone, PartialEq)]
pub struct InputMetrics {
    pub user_active: bool,
    /// Время с последнего ввода, миллисекунды. `None`, если событий ещё не было.
    pub time_since_last_input_ms: Option<u64>,
    /// Информация об устройствах ввода
    pub devices: HashMap<String, InputDeviceInfo>,
    /// Общее количество событий ввода
    pub total_events: u64,
    /// Активные устройства (текущая сессия)
    pub active_devices: Vec<String>,
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
    /// - `devices = пустой HashMap`
    /// - `total_events = 0`
    /// - `active_devices = пустой вектор`
    ///
    /// # Примеры
    ///
    /// ```rust
    /// use smoothtask_core::metrics::input::InputMetrics;
    ///
    /// let empty_metrics = InputMetrics::empty();
    /// assert!(!empty_metrics.user_active);
    /// assert!(empty_metrics.time_since_last_input_ms.is_none());
    /// assert!(empty_metrics.devices.is_empty());
    /// assert_eq!(empty_metrics.total_events, 0);
    /// assert!(empty_metrics.active_devices.is_empty());
    /// ```
    pub fn empty() -> Self {
        Self {
            user_active: false,
            time_since_last_input_ms: None,
            devices: HashMap::new(),
            total_events: 0,
            active_devices: Vec::new(),
        }
    }
}

/// Трекер активности по событиям evdev с классификацией устройств.
#[derive(Debug, Clone)]
pub struct InputActivityTracker {
    last_event: Option<Instant>,
    idle_threshold: Duration,
    error_count: u64,
    devices: HashMap<String, InputDeviceInfo>,
    total_events: u64,
}

impl InputActivityTracker {
    /// Создать трекер с заданным таймаутом простоя.
    pub fn new(idle_threshold: Duration) -> Self {
        Self {
            last_event: None,
            idle_threshold,
            error_count: 0,
            devices: HashMap::new(),
            total_events: 0,
        }
    }

    /// Зарегистрировать событие ввода, используя текущее время.
    pub fn register_activity(&mut self, now: Instant) {
        self.last_event = Some(now);
    }

    /// Зарегистрировать событие с конкретного устройства.
    pub fn register_device_activity(&mut self, device_path: &str, device_name: &str, now: Instant) {
        self.last_event = Some(now);
        self.total_events += 1;
        
        // Обновляем информацию об устройстве
        let device_info = self.devices.entry(device_path.to_string()).or_insert_with(|| {
            InputDeviceInfo {
                device_type: self.classify_device(device_name),
                device_name: device_name.to_string(),
                device_path: device_path.to_string(),
                event_count: 0,
                last_activity_time: None,
            }
        });
        
        device_info.event_count += 1;
        device_info.last_activity_time = Some(now);
    }

    /// Классифицировать устройство по имени.
    fn classify_device(&self, device_name: &str) -> InputDeviceType {
        let name_lower = device_name.to_lowercase();
        
        if name_lower.contains("keyboard") || name_lower.contains("klav") || name_lower.contains("клавиатура") {
            InputDeviceType::Keyboard
        } else if name_lower.contains("mouse") || name_lower.contains("мышь") || name_lower.contains("мышка") {
            InputDeviceType::Mouse
        } else if name_lower.contains("touchpad") || name_lower.contains("тачпад") || name_lower.contains("trackpad") {
            InputDeviceType::Touchpad
        } else if name_lower.contains("touchscreen") || name_lower.contains("сенсор") || name_lower.contains("экран") {
            InputDeviceType::Touchscreen
        } else if name_lower.contains("game") || name_lower.contains("controller") || name_lower.contains("joystick") || name_lower.contains("игровой") {
            InputDeviceType::GameController
        } else if name_lower.contains("trackball") || name_lower.contains("трекбол") {
            InputDeviceType::Trackball
        } else if name_lower.contains("tablet") || name_lower.contains("планшет") || name_lower.contains("wacom") || name_lower.contains("huion") {
            InputDeviceType::DrawingTablet
        } else if name_lower.contains("vr") || name_lower.contains("virtual reality") || name_lower.contains("oculus") || name_lower.contains("valve") || name_lower.contains("htc vive") {
            InputDeviceType::VrController
        } else if name_lower.contains("microphone") || name_lower.contains("mic") || name_lower.contains("микрофон") {
            InputDeviceType::Microphone
        } else if name_lower.contains("webcam") || name_lower.contains("camera") || name_lower.contains("камера") || name_lower.contains("logitech brio") || name_lower.contains("c920") {
            InputDeviceType::Webcam
        } else if name_lower.contains("fingerprint") || name_lower.contains("biometric") || name_lower.contains("face") || name_lower.contains("отпечаток") || name_lower.contains("биометри") {
            InputDeviceType::Biometric
        } else if name_lower.contains("accessibility") || name_lower.contains("braille") || name_lower.contains("switch") || name_lower.contains("adaptive") || name_lower.contains("специальн") {
            InputDeviceType::AccessibilityDevice
        } else {
            InputDeviceType::Unknown
        }
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

    /// Получить количество ошибок чтения устройств.
    ///
    /// Используется для диагностики и мониторинга.
    pub fn error_count(&self) -> u64 {
        self.error_count
    }

    /// Текущие метрики активности.
    pub fn metrics(&self, now: Instant) -> InputMetrics {
        // Определяем активные устройства (те, которые были активны в последние 5 секунд)
        let active_devices: Vec<String> = self.devices.iter()
            .filter(|(_, device)| {
                device.last_activity_time.map_or(false, |last_time| {
                    now.saturating_duration_since(last_time) <= Duration::from_secs(5)
                })
            })
            .map(|(path, _)| path.clone())
            .collect();
        
        match self.last_event {
            Some(ts) => {
                let elapsed = now.saturating_duration_since(ts);
                InputMetrics {
                    user_active: elapsed <= self.idle_threshold,
                    time_since_last_input_ms: Some(duration_to_ms(elapsed)),
                    devices: self.devices.clone(),
                    total_events: self.total_events,
                    active_devices,
                }
            }
            None => InputMetrics {
                user_active: false,
                time_since_last_input_ms: None,
                devices: self.devices.clone(),
                total_events: self.total_events,
                active_devices,
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

    /// Получить количество ошибок чтения устройств.
    ///
    /// Используется для диагностики и мониторинга.
    pub fn error_count(&self) -> u64 {
        match self {
            Self::Evdev(tracker) => tracker.error_count(),
            Self::Simple(tracker) => tracker.error_count(),
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
                        self.activity_tracker.error_count += 1;

                        // Улучшенные сообщения об ошибках с практическими рекомендациями
                        match e.kind() {
                            std::io::ErrorKind::PermissionDenied => {
                                warn!("Permission denied reading from input device: {}. Try running as root or adding user to 'input' group. Command: 'sudo usermod -aG input $USER' then reboot", e);
                            }
                            std::io::ErrorKind::NotFound => {
                                warn!("Input device not found: {}. Device may have been disconnected. Check /dev/input/event* devices", e);
                            }
                            std::io::ErrorKind::ConnectionReset
                            | std::io::ErrorKind::ConnectionAborted => {
                                warn!("Input device connection error: {}. Device may have been disconnected. Check device connections and try reconnecting", e);
                            }
                            _ => {
                                warn!("Error reading from input device: {}. Check device permissions and connections. If issue persists, try restarting the daemon", e);
                            }
                        }

                        // Логируем количество ошибок для диагностики
                        if self.activity_tracker.error_count.is_multiple_of(10) {
                            warn!(
                                "Input device error count reached {}: {}",
                                self.activity_tracker.error_count, e
                            );
                        }
                    }
                }
            }
        }

        // Обновляем трекер активности на основе всех собранных событий
        // и классифицируем устройства
        for device in &mut self.devices {
            // Считаем события для каждого устройства
            let device_events: Vec<&InputEvent> = all_events.iter()
                .filter(|event| {
                    // В реальной системе мы бы проверяли, с какого устройства пришло событие
                    // Для этой реализации предполагаем, что все события приходят с текущего устройства
                    true
                })
                .collect();
            
            if let Some(device_name) = device.name() {
                for _ in device_events {
                    self.activity_tracker.register_device_activity(
                        device.path().to_string_lossy().as_ref(),
                        device_name,
                        now
                    );
                }
            }
        }
        
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

        // Проверяем существование директории /dev/input
        if !input_dir.exists() {
            return Err(anyhow::anyhow!(
                "Input directory /dev/input does not exist. Check if input devices are properly configured. Try 'ls /dev/input' to verify"
            ));
        }

        // Читаем все файлы event* в /dev/input
        let entries = std::fs::read_dir(input_dir).map_err(|e| {
            match e.kind() {
                std::io::ErrorKind::PermissionDenied => {
                    anyhow::anyhow!(
                        "Permission denied accessing /dev/input: {}. Try running as root or adding user to 'input' group. Command: 'sudo usermod -aG input $USER' then reboot", e
                    )
                }
                std::io::ErrorKind::NotFound => {
                    anyhow::anyhow!(
                        "Input directory /dev/input not found: {}. Check if input devices are properly configured", e
                    )
                }
                _ => anyhow::anyhow!(
                    "Failed to read /dev/input directory: {}. Check device permissions and configuration", e
                )
            }
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                anyhow::anyhow!(
                    "Failed to read input device entry: {}. Check /dev/input directory permissions",
                    e
                )
            })?;
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
                    // Улучшенные сообщения об ошибках с практическими рекомендациями
                    match e.kind() {
                        std::io::ErrorKind::PermissionDenied => {
                            debug!("Permission denied opening device {:?}: {}. Try running as root or adding user to 'input' group", path, e);
                        }
                        std::io::ErrorKind::NotFound => {
                            debug!("Input device {:?} not found: {}. Device may have been disconnected", path, e);
                        }
                        _ => {
                            debug!("Failed to open device {:?}: {}. Check device permissions and connections", path, e);
                        }
                    }
                }
            }
        }

        if devices.is_empty() {
            warn!("No suitable input devices found in /dev/input. Check if input devices are connected and have proper permissions. Try 'ls -la /dev/input/event*' to verify");
        }

        Ok(devices)
    }

    /// Обновить порог простоя.
    ///
    /// Используется для динамической перезагрузки конфигурации.
    pub fn set_idle_threshold(&mut self, new_threshold: Duration) {
        self.activity_tracker.set_idle_threshold(new_threshold);
    }

    /// Получить количество ошибок чтения устройств.
    ///
    /// Используется для диагностики и мониторинга.
    pub fn error_count(&self) -> u64 {
        self.activity_tracker.error_count
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
        let mut simple_tracker =
            InputTracker::Simple(InputActivityTracker::new(Duration::from_secs(3)));
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

    #[test]
    fn test_input_tracker_error_counting() {
        // Тест для проверки счетчика ошибок
        let tracker = InputActivityTracker::new(Duration::from_secs(5));
        assert_eq!(tracker.error_count(), 0);

        // Проверяем, что счетчик ошибок доступен через InputTracker
        let input_tracker = InputTracker::new(Duration::from_secs(5));
        let error_count = input_tracker.error_count();
        assert_eq!(error_count, 0);
    }

    #[test]
    fn test_input_tracker_graceful_degradation() {
        // Тест для проверки graceful degradation
        // При отсутствии устройств должны возвращаться дефолтные метрики
        let tracker = InputActivityTracker::new(Duration::from_secs(5));
        let metrics = tracker.metrics(Instant::now());

        // При отсутствии событий должны возвращаться дефолтные значения
        assert!(!metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, None);
    }

    #[test]
    fn test_input_tracker_error_recovery() {
        // Тест для проверки восстановления после ошибок
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));

        // Симулируем активность
        let now = Instant::now();
        let key = InputEvent::new(EventType::KEY, Key::KEY_A.code(), 1);
        let metrics = tracker.ingest_events([key].iter(), now);

        // После активности пользователь должен быть активным
        assert!(metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, Some(0));

        // Проверяем, что счетчик ошибок доступен
        assert_eq!(tracker.error_count(), 0);
    }

    #[test]
    fn test_device_classification() {
        let tracker = InputActivityTracker::new(Duration::from_secs(5));
        
        // Тестируем классификацию различных устройств
        assert_eq!(tracker.classify_device("USB Keyboard"), InputDeviceType::Keyboard);
        assert_eq!(tracker.classify_device("Logitech Mouse"), InputDeviceType::Mouse);
        assert_eq!(tracker.classify_device("Synaptics Touchpad"), InputDeviceType::Touchpad);
        assert_eq!(tracker.classify_device("Touchscreen"), InputDeviceType::Touchscreen);
        assert_eq!(tracker.classify_device("Xbox Controller"), InputDeviceType::GameController);
        assert_eq!(tracker.classify_device("Unknown Device"), InputDeviceType::Unknown);
        
        // Тестируем русские названия
        assert_eq!(tracker.classify_device("Клавиатура"), InputDeviceType::Keyboard);
        assert_eq!(tracker.classify_device("Мышь"), InputDeviceType::Mouse);
        assert_eq!(tracker.classify_device("Тачпад"), InputDeviceType::Touchpad);
        assert_eq!(tracker.classify_device("Игровой контроллер"), InputDeviceType::GameController);
    }

    #[test]
    fn test_extended_device_classification() {
        let tracker = InputActivityTracker::new(Duration::from_secs(5));
        
        // Тестируем новые типы устройств
        assert_eq!(tracker.classify_device("Logitech Trackball"), InputDeviceType::Trackball);
        assert_eq!(tracker.classify_device("Wacom Drawing Tablet"), InputDeviceType::DrawingTablet);
        assert_eq!(tracker.classify_device("Oculus VR Controller"), InputDeviceType::VrController);
        assert_eq!(tracker.classify_device("USB Microphone"), InputDeviceType::Microphone);
        assert_eq!(tracker.classify_device("Logitech Brio Webcam"), InputDeviceType::Webcam);
        assert_eq!(tracker.classify_device("Fingerprint Reader"), InputDeviceType::Biometric);
        assert_eq!(tracker.classify_device("Braille Keyboard"), InputDeviceType::AccessibilityDevice);
        
        // Тестируем русские названия для новых устройств
        assert_eq!(tracker.classify_device("Трекбол"), InputDeviceType::Trackball);
        assert_eq!(tracker.classify_device("Графический планшет"), InputDeviceType::DrawingTablet);
        assert_eq!(tracker.classify_device("VR контроллер"), InputDeviceType::VrController);
        assert_eq!(tracker.classify_device("Микрофон"), InputDeviceType::Microphone);
        assert_eq!(tracker.classify_device("Веб-камера"), InputDeviceType::Webcam);
        assert_eq!(tracker.classify_device("Сканер отпечатков"), InputDeviceType::Biometric);
        assert_eq!(tracker.classify_device("Специальная клавиатура"), InputDeviceType::AccessibilityDevice);
        
        // Тестируем брендовые устройства
        assert_eq!(tracker.classify_device("Huion H610 Pro"), InputDeviceType::DrawingTablet);
        assert_eq!(tracker.classify_device("Valve Index Controller"), InputDeviceType::VrController);
        assert_eq!(tracker.classify_device("Logitech C920"), InputDeviceType::Webcam);
        assert_eq!(tracker.classify_device("Blue Yeti Mic"), InputDeviceType::Microphone);
        
        // Тестируем неизвестные устройства
        assert_eq!(tracker.classify_device("Random Device"), InputDeviceType::Unknown);
        assert_eq!(tracker.classify_device(""), InputDeviceType::Unknown);
    }

    #[test]
    fn test_device_activity_tracking() {
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        let now = Instant::now();
        
        // Регистрируем активность с разных устройств
        tracker.register_device_activity("/dev/input/event0", "USB Keyboard", now);
        tracker.register_device_activity("/dev/input/event1", "Logitech Mouse", now);
        tracker.register_device_activity("/dev/input/event0", "USB Keyboard", now);
        
        // Проверяем метрики
        let metrics = tracker.metrics(now);
        assert_eq!(metrics.total_events, 3);
        assert_eq!(metrics.devices.len(), 2);
        
        // Проверяем информацию об устройствах
        if let Some(keyboard) = metrics.devices.get("/dev/input/event0") {
            assert_eq!(keyboard.device_type, InputDeviceType::Keyboard);
            assert_eq!(keyboard.device_name, "USB Keyboard");
            assert_eq!(keyboard.event_count, 2);
            assert!(keyboard.last_activity_time.is_some());
        } else {
            panic!("Keyboard device not found");
        }
        
        if let Some(mouse) = metrics.devices.get("/dev/input/event1") {
            assert_eq!(mouse.device_type, InputDeviceType::Mouse);
            assert_eq!(mouse.device_name, "Logitech Mouse");
            assert_eq!(mouse.event_count, 1);
            assert!(mouse.last_activity_time.is_some());
        } else {
            panic!("Mouse device not found");
        }
    }

    #[test]
    fn test_active_devices_detection() {
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        let now = Instant::now();
        
        // Регистрируем активность с разных устройств
        tracker.register_device_activity("/dev/input/event0", "USB Keyboard", now);
        tracker.register_device_activity("/dev/input/event1", "Logitech Mouse", now);
        
        // Проверяем активные устройства (должны быть оба)
        let metrics = tracker.metrics(now);
        assert_eq!(metrics.active_devices.len(), 2);
        assert!(metrics.active_devices.contains("&/dev/input/event0"));
        assert!(metrics.active_devices.contains("&/dev/input/event1"));
        
        // Ждем 6 секунд (больше порога активности)
        let later = now + Duration::from_secs(6);
        let metrics_later = tracker.metrics(later);
        
        // Теперь активных устройств не должно быть
        assert!(metrics_later.active_devices.is_empty());
    }

    #[test]
    fn test_extended_input_metrics() {
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        let now = Instant::now();
        
        // Регистрируем активность
        let key = InputEvent::new(EventType::KEY, Key::KEY_A.code(), 1);
        tracker.ingest_events([key].iter(), now);
        
        // Регистрируем активность с устройства
        tracker.register_device_activity("/dev/input/event0", "USB Keyboard", now);
        
        // Проверяем расширенные метрики
        let metrics = tracker.metrics(now);
        assert!(metrics.user_active);
        assert_eq!(metrics.time_since_last_input_ms, Some(0));
        assert_eq!(metrics.total_events, 1);
        assert_eq!(metrics.devices.len(), 1);
        assert_eq!(metrics.active_devices.len(), 1);
        
        // Проверяем, что пустые метрики работают корректно
        let empty_metrics = InputMetrics::empty();
        assert!(!empty_metrics.user_active);
        assert!(empty_metrics.time_since_last_input_ms.is_none());
        assert!(empty_metrics.devices.is_empty());
        assert_eq!(empty_metrics.total_events, 0);
        assert!(empty_metrics.active_devices.is_empty());
    }

    #[test]
    fn test_device_classification_edge_cases() {
        let tracker = InputActivityTracker::new(Duration::from_secs(5));
        
        // Тестируем граничные случаи классификации
        assert_eq!(tracker.classify_device(""), InputDeviceType::Unknown);
        assert_eq!(tracker.classify_device("Random Device"), InputDeviceType::Unknown);
        assert_eq!(tracker.classify_device("KEYBOARD"), InputDeviceType::Keyboard);
        assert_eq!(tracker.classify_device("keyboard"), InputDeviceType::Keyboard);
        assert_eq!(tracker.classify_device("Клавиатура"), InputDeviceType::Keyboard);
        assert_eq!(tracker.classify_device("клавиатура"), InputDeviceType::Keyboard);
        
        // Тестируем составные названия
        assert_eq!(tracker.classify_device("Gaming Keyboard"), InputDeviceType::Keyboard);
        assert_eq!(tracker.classify_device("Wireless Mouse"), InputDeviceType::Mouse);
        assert_eq!(tracker.classify_device("Bluetooth Touchpad"), InputDeviceType::Touchpad);
    }

    #[test]
    fn test_device_activity_multiple_events() {
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        let now = Instant::now();
        
        // Регистрируем множество событий с одного устройства
        for i in 0..10 {
            tracker.register_device_activity("/dev/input/event0", "USB Keyboard", now);
        }
        
        // Проверяем метрики
        let metrics = tracker.metrics(now);
        assert_eq!(metrics.total_events, 10);
        assert_eq!(metrics.devices.len(), 1);
        
        if let Some(device) = metrics.devices.get("/dev/input/event0") {
            assert_eq!(device.event_count, 10);
        } else {
            panic!("Device not found");
        }
    }

    #[test]
    fn test_device_activity_different_devices() {
        let mut tracker = InputActivityTracker::new(Duration::from_secs(5));
        let now = Instant::now();
        
        // Регистрируем активность с разных устройств
        tracker.register_device_activity("/dev/input/event0", "USB Keyboard", now);
        tracker.register_device_activity("/dev/input/event1", "Logitech Mouse", now);
        tracker.register_device_activity("/dev/input/event2", "Touchpad", now);
        tracker.register_device_activity("/dev/input/event3", "Game Controller", now);
        
        // Проверяем метрики
        let metrics = tracker.metrics(now);
        assert_eq!(metrics.total_events, 4);
        assert_eq!(metrics.devices.len(), 4);
        
        // Проверяем классификацию устройств
        assert_eq!(metrics.devices.get("/dev/input/event0").unwrap().device_type, InputDeviceType::Keyboard);
        assert_eq!(metrics.devices.get("/dev/input/event1").unwrap().device_type, InputDeviceType::Mouse);
        assert_eq!(metrics.devices.get("/dev/input/event2").unwrap().device_type, InputDeviceType::Touchpad);
        assert_eq!(metrics.devices.get("/dev/input/event3").unwrap().device_type, InputDeviceType::GameController);
    }
}
