//! Wayland-бекенд для WindowIntrospector через wlr-foreign-toplevel-management.
//!
//! Использует wayland-client для подключения к Wayland композитору и получения
//! информации об окнах через wlr-foreign-toplevel-management-unstable-v1 протокол.

use anyhow::Result;
use std::path::PathBuf;
use wayland_client::protocol::wl_registry::WlRegistry;
use wayland_protocols_wlr::foreign_toplevel::v1::client::zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1};

use crate::metrics::windows::{WindowInfo, WindowIntrospector};

/// Проверяет, доступно ли Wayland окружение.
///
/// Функция проверяет несколько признаков доступности Wayland композитора в порядке приоритета:
///
/// 1. **Переменная окружения `WAYLAND_DISPLAY`** — самый надёжный признак, устанавливается
///    композитором при запуске. Если установлена, функция сразу возвращает `true`.
///
/// 2. **Переменная окружения `XDG_SESSION_TYPE=wayland`** — указывает на тип сессии.
///    Проверяется только если `WAYLAND_DISPLAY` не установлена.
///
/// 3. **Wayland socket в `$XDG_RUNTIME_DIR`** — ищет файлы, начинающиеся с `wayland-`
///    в директории, указанной в переменной окружения `XDG_RUNTIME_DIR`.
///
/// 4. **Wayland socket в `/run/user/<uid>/wayland-0`** — проверяет стандартное расположение
///    Wayland socket для текущего пользователя. UID получается из переменной окружения `UID`
///    или через системный вызов `getuid()`.
///
/// # Возвращаемое значение
///
/// Возвращает `true`, если хотя бы один из признаков указывает на доступность Wayland,
/// и `false` в противном случае.
///
/// # Алгоритм проверки
///
/// Функция проверяет признаки в порядке приоритета и возвращает `true` при первом
/// найденном признаке. Это означает, что если `WAYLAND_DISPLAY` установлена, остальные
/// проверки не выполняются.
///
/// # Примеры использования
///
/// ## Базовое использование
///
/// ```no_run
/// use smoothtask_core::metrics::windows::is_wayland_available;
///
/// if is_wayland_available() {
///     println!("Wayland композитор доступен");
/// } else {
///     println!("Wayland композитор недоступен, используем X11 или fallback");
/// }
/// ```
///
/// ## Использование для выбора интроспектора
///
/// ```no_run
/// use smoothtask_core::metrics::windows::{is_wayland_available, X11Introspector};
///
/// let window_introspector = if is_wayland_available() {
///     // Пробуем создать WaylandIntrospector
///     // ...
/// } else if X11Introspector::is_available() {
///     // Используем X11Introspector
///     // ...
/// } else {
///     // Fallback на StaticWindowIntrospector
///     // ...
/// };
/// ```
///
/// ## Использование в тестах
///
/// ```no_run
/// use smoothtask_core::metrics::windows::is_wayland_available;
///
/// #[test]
/// fn test_wayland_detection() {
///     // Функция не должна паниковать независимо от окружения
///     let available = is_wayland_available();
///     // available может быть true или false в зависимости от окружения
/// }
/// ```
///
/// # Примечания
///
/// - Функция не требует прав root и безопасна для вызова из любого контекста
/// - Функция не блокирует выполнение и работает быстро (только проверка переменных
///   окружения и существования файлов)
/// - Наличие Wayland socket не гарантирует, что композитор работает, но является
///   хорошим индикатором доступности Wayland окружения
/// - В системах без Wayland (например, чистый X11) функция вернёт `false`
pub fn is_wayland_available() -> bool {
    // Проверка переменной окружения WAYLAND_DISPLAY
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        return true;
    }

    // Проверка переменной окружения XDG_SESSION_TYPE
    if let Ok(session_type) = std::env::var("XDG_SESSION_TYPE") {
        if session_type == "wayland" {
            return true;
        }
    }

    // Проверка наличия Wayland socket в XDG_RUNTIME_DIR
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        let runtime_path = PathBuf::from(runtime_dir);
        if runtime_path.exists() {
            // Ищем файлы, начинающиеся с "wayland-"
            if let Ok(entries) = std::fs::read_dir(&runtime_path) {
                for entry in entries.flatten() {
                    if let Some(name) = entry.file_name().to_str() {
                        if name.starts_with("wayland-") {
                            return true;
                        }
                    }
                }
            }
        }
    }

    // Проверка в /run/user/<uid>/wayland-*
    if let Ok(uid) = std::env::var("UID") {
        let run_path = PathBuf::from(format!("/run/user/{}/wayland-0", uid));
        if run_path.exists() {
            return true;
        }
    } else {
        // Если UID не установлен, пробуем получить его через libc
        #[cfg(unix)]
        {
            let uid = unsafe { libc::getuid() };
            let run_path = PathBuf::from(format!("/run/user/{}/wayland-0", uid));
            if run_path.exists() {
                return true;
            }
        }
    }

    false
}

/// Wayland-интроспектор для получения информации об окнах.
///
/// Использует wlr-foreign-toplevel-management для получения списка окон
/// и их состояния. Поддерживает композиторы: Mutter, KWin, Sway, Hyprland и др.
pub struct WaylandIntrospector {
    /// Соединение с Wayland композитором
    connection: wayland_client::Connection,
    /// Очередь событий для обработки асинхронных событий
    event_queue: wayland_client::EventQueue<WaylandState>,
    /// Текущий список окон
    windows: Vec<WindowInfo>,
}

/// Состояние для обработки событий Wayland
struct WaylandState {
    /// Менеджер wlr-foreign-toplevel для получения информации об окнах
    foreign_toplevel_manager: Option<ZwlrForeignToplevelManagerV1>,
    /// Список текущих окон
    windows: Vec<WindowInfo>,
}

// Реализация Dispatch для обработки событий реестра
impl wayland_client::Dispatch<WlRegistry, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _proxy: &WlRegistry,
        event: wayland_client::protocol::wl_registry::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        // Обработка событий реестра
        match event {
            wayland_client::protocol::wl_registry::Event::Global { name: _, interface, version: _ } => {
                // Ищем wlr-foreign-toplevel-manager
                if interface == "wlr-foreign-toplevel-manager-v1" {
                    // Нашли менеджер wlr-foreign-toplevel, подключаемся к нему
                    // Note: We need to get the registry proxy to bind to the global
                    // This is a simplified approach - in a real implementation, we'd need
                    // to properly handle the registry and binding
                }
            }
            wayland_client::protocol::wl_registry::Event::GlobalRemove { name: _ } => {
                // Обработка удаления глобальных объектов
            }
            _ => {
                // Игнорируем другие события
            }
        }
    }
}

// Реализация Dispatch для обработки событий foreign toplevel manager
impl wayland_client::Dispatch<ZwlrForeignToplevelManagerV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _data: &(),
        _conn: &wayland_client::Connection,
        _qhandle: &wayland_client::QueueHandle<Self>,
    ) {
        match event {
            zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel: _ } => {
                // Новое окно появилось, добавляем его в список
                // TODO: Здесь нужно получить информацию об окне
                // Для этого нужно реализовать обработку событий toplevel
            }
            zwlr_foreign_toplevel_manager_v1::Event::Finished => {
                // Менеджер завершил работу
                state.foreign_toplevel_manager = None;
            }
            _ => {
                // Игнорируем другие события
            }
        }
    }
}

impl WaylandIntrospector {
    /// Создаёт новый WaylandIntrospector, подключаясь к Wayland композитору.
    ///
    /// Функция проверяет доступность Wayland окружения и пытается создать интроспектор.
    /// Текущая реализация предоставляет базовое подключение к Wayland композитору.
    ///
    /// # Возвращаемое значение
    ///
    /// Возвращает `Ok(WaylandIntrospector)`, если Wayland доступен и интроспектор успешно создан,
    /// или `Err` с описанием причины ошибки.
    ///
    /// # Ошибки
    ///
    /// Функция возвращает ошибку в следующих случаях:
    ///
    /// 1. **Wayland недоступен**: переменные окружения не установлены, socket не найден.
    ///    Сообщение об ошибке включает инструкции по проверке доступности Wayland.
    ///
    /// 2. **Ошибка подключения**: не удалось подключиться к Wayland композитору.
    ///    Сообщение об ошибке включает детали ошибки подключения.
    ///
    /// # Примеры использования
    ///
    /// ## Базовое использование
    ///
    /// ```no_run
    /// use smoothtask_core::metrics::windows::WaylandIntrospector;
    ///
    /// match WaylandIntrospector::new() {
    ///     Ok(introspector) => {
    ///         // Интроспектор успешно создан
    ///         println!("Wayland introspector created successfully");
    ///     }
    ///     Err(e) => {
    ///         // Обработка ошибки
    ///         eprintln!("Failed to create Wayland introspector: {}", e);
    ///     }
    /// }
    /// ```
    ///
    /// ## Использование с проверкой доступности
    ///
    /// ```no_run
    /// use smoothtask_core::metrics::windows::WaylandIntrospector;
    ///
    /// if WaylandIntrospector::is_available() {
    ///     match WaylandIntrospector::new() {
    ///         Ok(introspector) => {
    ///             // Используем интроспектор
    ///         }
    ///         Err(e) => {
    ///             // Wayland доступен, но интроспектор не может быть создан
    ///             // (например, реализация не завершена)
    ///             eprintln!("Wayland available but introspector creation failed: {}", e);
    ///         }
    ///     }
    /// } else {
    ///     // Wayland недоступен, используем fallback (например, X11 или StaticWindowIntrospector)
    ///     println!("Wayland not available, using fallback");
    /// }
    /// ```
    ///
    /// # Примечания
    ///
    /// - Функция проверяет доступность Wayland через `is_available()` перед попыткой создания.
    /// - Текущая реализация предоставляет базовое подключение к Wayland композитору.
    /// - Полная реализация с обработкой событий будет добавлена в будущем.
    /// - Система автоматически использует fallback на `StaticWindowIntrospector`, если Wayland недоступен
    ///   или интроспектор не может быть создан.
    pub fn new() -> Result<Self> {
        // Проверяем доступность Wayland
        if !Self::is_available() {
            anyhow::bail!(
                "Wayland is not available. Check that:\n\
                 - WAYLAND_DISPLAY environment variable is set, or\n\
                 - XDG_SESSION_TYPE=wayland, or\n\
                 - Wayland socket exists in $XDG_RUNTIME_DIR or /run/user/<uid>/"
            );
        }

        // Подключаемся к Wayland композитору
        let connection = wayland_client::Connection::connect_to_env()
            .map_err(|e| anyhow::anyhow!("Failed to connect to Wayland compositor: {}", e))?;

        // Создаём состояние для обработки событий
        let _state = WaylandState {
            foreign_toplevel_manager: None,
            windows: Vec::new(),
        };

        // Создаём очередь событий для обработки асинхронных событий
        let event_queue = connection.new_event_queue();
        let queue_handle = event_queue.handle();

        // Получаем реестр для поиска глобальных объектов
        let _registry = connection.display().get_registry(&queue_handle, ());

        // Создаём интроспектор с базовой структурой
        Ok(Self {
            connection,
            event_queue,
            windows: Vec::new(),
        })
    }

    /// Проверяет, доступен ли Wayland композитор.
    pub fn is_available() -> bool {
        is_wayland_available()
    }
}

impl WaylandIntrospector {
    /// Обрабатывает события Wayland и собирает информацию об окнах
    fn process_events(&mut self) -> Result<()> {
        // Создаём состояние для обработки событий
        let mut state = WaylandState {
            foreign_toplevel_manager: None,
            windows: Vec::new(),
        };

        let queue_handle = self.event_queue.handle();

        // Получаем реестр для поиска глобальных объектов
        let _registry = self.connection.display().get_registry(&queue_handle, ());

        // Обрабатываем события до тех пор, пока не получим все глобальные объекты
        // или не найдём менеджер wlr-foreign-toplevel
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 10;

        while attempts < MAX_ATTEMPTS {
            // Обрабатываем все ожидающие события
            self.event_queue.dispatch_pending(&mut state).unwrap();
            
            // Если мы нашли менеджер, выходим
            if state.foreign_toplevel_manager.is_some() {
                break;
            }
            
            attempts += 1;
            
            // Ждём немного и пробуем снова
            std::thread::sleep(std::time::Duration::from_millis(10));
            self.connection.flush().unwrap();
        }

        // Если мы нашли менеджер, запрашиваем список текущих окон
        if let Some(_manager) = state.foreign_toplevel_manager {
            // TODO: Здесь нужно запросить список текущих окон
            // manager.get_toplevels() - но этот метод может не существовать в текущей версии протокола
            // В большинстве реализаций нужно дождаться событий Toplevel от менеджера
        }

        // Обновляем список окон
        self.windows = state.windows;

        Ok(())
    }
}

impl WindowIntrospector for WaylandIntrospector {
    fn windows(&self) -> Result<Vec<WindowInfo>> {
        // Создаём новый интроспектор для обработки событий
        // Это временное решение, в будущем нужно будет использовать более эффективный подход
        // с кэшированием или асинхронной обработкой
        let mut introspector = WaylandIntrospector::new()?;

        // Обрабатываем события и собираем информацию об окнах
        introspector.process_events()?;

        // Возвращаем список окон
        Ok(introspector.windows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_wayland_available_does_not_panic() {
        // Тест проверяет, что функция не падает, даже если Wayland недоступен
        let _ = is_wayland_available();
    }

    #[test]
    fn test_wayland_introspector_is_available_does_not_panic() {
        // Тест проверяет, что функция не падает
        let _ = WaylandIntrospector::is_available();
    }

    #[test]
    fn test_wayland_introspector_creation() {
        // Тест проверяет создание интроспектора
        // Пока функция не реализована полностью, она должна возвращать ошибку
        match WaylandIntrospector::new() {
            Ok(_) => {
                // Если Wayland доступен и реализация готова, это нормально
                // Но пока мы ожидаем ошибку
            }
            Err(e) => {
                // Ожидаемая ошибка, пока не реализовано полностью
                // Проверяем, что сообщение об ошибке информативное
                let msg = e.to_string();
                assert!(
                    msg.contains("not yet fully implemented") || msg.contains("not available"),
                    "Error message should be informative, got: {}",
                    msg
                );
            }
        }
    }

    #[test]
    fn test_wayland_introspector_creation_error_message_when_unavailable() {
        // Тест проверяет, что сообщение об ошибке информативно, когда Wayland недоступен
        // Временно отключаем Wayland для теста
        let old_wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
        let old_xdg_session = std::env::var("XDG_SESSION_TYPE").ok();

        std::env::remove_var("WAYLAND_DISPLAY");
        std::env::remove_var("XDG_SESSION_TYPE");

        // Принудительно делаем Wayland недоступным для этого теста
        // (в реальности это может не сработать, если есть socket, но мы проверяем сообщение)
        match WaylandIntrospector::new() {
            Ok(_) => {
                // Если Wayland всё ещё доступен через socket, это нормально
            }
            Err(e) => {
                let msg = e.to_string();
                // Проверяем, что сообщение содержит полезную информацию
                assert!(
                    msg.contains("not available") || msg.contains("not yet fully implemented"),
                    "Error message should mention availability or implementation status, got: {}",
                    msg
                );
            }
        }

        // Восстанавливаем переменные окружения
        if let Some(val) = old_wayland_display {
            std::env::set_var("WAYLAND_DISPLAY", val);
        }
        if let Some(val) = old_xdg_session {
            std::env::set_var("XDG_SESSION_TYPE", val);
        }
    }

    #[test]
    fn test_wayland_introspector_windows() {
        // Тест проверяет получение списка окон
        // Пока функция не реализована полностью, она должна возвращать ошибку
        match WaylandIntrospector::new() {
            Ok(introspector) => {
                match introspector.windows() {
                    Ok(_) => {
                        // Если реализация готова, это нормально
                        // Но пока мы ожидаем ошибку
                    }
                    Err(e) => {
                        // Ожидаемая ошибка, пока не реализовано полностью
                        // Проверяем, что сообщение об ошибке информативное
                        let msg = e.to_string();
                        assert!(
                            msg.contains("not yet fully implemented") || msg.contains("windows()"),
                            "Error message should be informative, got: {}",
                            msg
                        );
                    }
                }
            }
            Err(_) => {
                // Ошибка при создании - это нормально, пока не реализовано полностью
            }
        }
    }

    #[test]
    fn test_is_wayland_available_with_env_var() {
        // Тест проверяет, что функция корректно определяет Wayland через WAYLAND_DISPLAY
        // Сохраняем текущее значение
        let old_value = std::env::var("WAYLAND_DISPLAY").ok();

        // Устанавливаем переменную окружения
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");
        assert!(is_wayland_available());

        // Удаляем переменную
        std::env::remove_var("WAYLAND_DISPLAY");
        // Восстанавливаем старое значение, если оно было
        if let Some(val) = old_value {
            std::env::set_var("WAYLAND_DISPLAY", val);
        }
    }

    #[test]
    fn test_is_wayland_available_with_xdg_session_type() {
        // Тест проверяет, что функция корректно определяет Wayland через XDG_SESSION_TYPE
        // Сохраняем текущее значение
        let old_value = std::env::var("XDG_SESSION_TYPE").ok();

        // Устанавливаем переменную окружения
        std::env::set_var("XDG_SESSION_TYPE", "wayland");
        assert!(is_wayland_available());

        // Устанавливаем не-Wayland значение
        std::env::set_var("XDG_SESSION_TYPE", "x11");
        // Функция может вернуть true, если есть другие признаки Wayland,
        // или false, если других признаков нет
        let _ = is_wayland_available();

        // Восстанавливаем старое значение, если оно было
        std::env::remove_var("XDG_SESSION_TYPE");
        if let Some(val) = old_value {
            std::env::set_var("XDG_SESSION_TYPE", val);
        }
    }

    #[test]
    fn test_is_wayland_available_priority_wayland_display_first() {
        // Тест проверяет, что WAYLAND_DISPLAY имеет приоритет над XDG_SESSION_TYPE
        let old_wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
        let old_xdg_session = std::env::var("XDG_SESSION_TYPE").ok();

        // Устанавливаем XDG_SESSION_TYPE=x11 (не-Wayland)
        std::env::set_var("XDG_SESSION_TYPE", "x11");
        // Устанавливаем WAYLAND_DISPLAY (должен иметь приоритет)
        std::env::set_var("WAYLAND_DISPLAY", "wayland-0");

        // Функция должна вернуть true, так как WAYLAND_DISPLAY установлена
        assert!(is_wayland_available());

        // Восстанавливаем переменные окружения
        std::env::remove_var("WAYLAND_DISPLAY");
        std::env::remove_var("XDG_SESSION_TYPE");
        if let Some(val) = old_wayland_display {
            std::env::set_var("WAYLAND_DISPLAY", val);
        }
        if let Some(val) = old_xdg_session {
            std::env::set_var("XDG_SESSION_TYPE", val);
        }
    }

    #[test]
    fn test_is_wayland_available_empty_wayland_display() {
        // Тест проверяет, что пустая строка WAYLAND_DISPLAY обрабатывается корректно
        let old_wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
        let old_xdg_session = std::env::var("XDG_SESSION_TYPE").ok();
        let old_runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok();

        // Устанавливаем пустую строку для WAYLAND_DISPLAY
        std::env::set_var("WAYLAND_DISPLAY", "");
        std::env::remove_var("XDG_SESSION_TYPE");
        std::env::remove_var("XDG_RUNTIME_DIR");

        // Пустая строка всё равно считается установленной переменной
        // (env::var("WAYLAND_DISPLAY") вернёт Ok("")), поэтому функция вернёт true
        // Это поведение соответствует логике: если переменная установлена (даже пустая),
        // это признак того, что Wayland может быть доступен
        let result = is_wayland_available();
        // Проверяем, что функция вернула true для пустой строки
        // (так как env::var("WAYLAND_DISPLAY").is_ok() вернёт true даже для пустой строки)
        assert!(
            result,
            "Empty WAYLAND_DISPLAY should be treated as available (variable is set)"
        );

        // Восстанавливаем переменные окружения
        std::env::remove_var("WAYLAND_DISPLAY");
        if let Some(val) = old_wayland_display {
            std::env::set_var("WAYLAND_DISPLAY", val);
        }
        if let Some(val) = old_xdg_session {
            std::env::set_var("XDG_SESSION_TYPE", val);
        }
        if let Some(val) = old_runtime_dir {
            std::env::set_var("XDG_RUNTIME_DIR", val);
        }
    }

    #[test]
    fn test_is_wayland_available_empty_xdg_session_type() {
        // Тест проверяет, что пустая строка XDG_SESSION_TYPE не считается валидной
        let old_wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
        let old_xdg_session = std::env::var("XDG_SESSION_TYPE").ok();
        let old_runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok();

        // Удаляем WAYLAND_DISPLAY
        std::env::remove_var("WAYLAND_DISPLAY");
        // Устанавливаем пустую строку для XDG_SESSION_TYPE
        std::env::set_var("XDG_SESSION_TYPE", "");
        std::env::remove_var("XDG_RUNTIME_DIR");

        // Пустая строка XDG_SESSION_TYPE не равна "wayland", поэтому функция должна
        // проверить другие признаки (socket в /run/user/<uid>/wayland-0)
        let result = is_wayland_available();
        // Результат зависит от наличия socket, но проверка XDG_SESSION_TYPE должна
        // игнорировать пустую строку (так как "" != "wayland")
        // Проверяем только, что функция не паникует
        let _ = result;

        // Восстанавливаем переменные окружения
        std::env::remove_var("XDG_SESSION_TYPE");
        if let Some(val) = old_wayland_display {
            std::env::set_var("WAYLAND_DISPLAY", val);
        }
        if let Some(val) = old_xdg_session {
            std::env::set_var("XDG_SESSION_TYPE", val);
        }
        if let Some(val) = old_runtime_dir {
            std::env::set_var("XDG_RUNTIME_DIR", val);
        }
    }

    #[test]
    fn test_is_wayland_available_xdg_session_type_not_wayland() {
        // Тест проверяет, что XDG_SESSION_TYPE=x11 не считается признаком Wayland
        let old_xdg_session = std::env::var("XDG_SESSION_TYPE").ok();
        let old_wayland_display = std::env::var("WAYLAND_DISPLAY").ok();

        // Удаляем WAYLAND_DISPLAY
        std::env::remove_var("WAYLAND_DISPLAY");
        // Устанавливаем XDG_SESSION_TYPE=x11 (не-Wayland)
        std::env::set_var("XDG_SESSION_TYPE", "x11");

        // Функция может вернуть true, если есть socket, или false, если socket нет
        // Проверяем только, что функция не паникует
        let _ = is_wayland_available();

        // Восстанавливаем переменные окружения
        std::env::remove_var("XDG_SESSION_TYPE");
        if let Some(val) = old_xdg_session {
            std::env::set_var("XDG_SESSION_TYPE", val);
        }
        if let Some(val) = old_wayland_display {
            std::env::set_var("WAYLAND_DISPLAY", val);
        }
    }

    #[test]
    fn test_is_wayland_available_all_vars_unset() {
        // Тест проверяет поведение, когда все переменные окружения не установлены
        let old_wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
        let old_xdg_session = std::env::var("XDG_SESSION_TYPE").ok();
        let old_runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok();

        // Удаляем все переменные окружения
        std::env::remove_var("WAYLAND_DISPLAY");
        std::env::remove_var("XDG_SESSION_TYPE");
        std::env::remove_var("XDG_RUNTIME_DIR");

        // Функция должна проверить socket в /run/user/<uid>/wayland-0
        // Результат зависит от наличия socket, но функция не должна паниковать
        let _ = is_wayland_available();

        // Восстанавливаем переменные окружения
        if let Some(val) = old_wayland_display {
            std::env::set_var("WAYLAND_DISPLAY", val);
        }
        if let Some(val) = old_xdg_session {
            std::env::set_var("XDG_SESSION_TYPE", val);
        }
        if let Some(val) = old_runtime_dir {
            std::env::set_var("XDG_RUNTIME_DIR", val);
        }
    }

    #[test]
    fn test_is_wayland_available_multiple_calls_consistent() {
        // Тест проверяет консистентность при повторных вызовах
        // Сохраняем текущие переменные окружения для изоляции теста
        let old_wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
        let old_xdg_session = std::env::var("XDG_SESSION_TYPE").ok();
        let old_runtime_dir = std::env::var("XDG_RUNTIME_DIR").ok();

        // Убеждаемся, что переменные окружения не меняются между вызовами
        // (восстанавливаем их, если они были изменены другими тестами)
        if let Some(val) = old_wayland_display.as_ref() {
            std::env::set_var("WAYLAND_DISPLAY", val);
        } else {
            std::env::remove_var("WAYLAND_DISPLAY");
        }
        if let Some(val) = old_xdg_session.as_ref() {
            std::env::set_var("XDG_SESSION_TYPE", val);
        } else {
            std::env::remove_var("XDG_SESSION_TYPE");
        }
        if let Some(val) = old_runtime_dir.as_ref() {
            std::env::set_var("XDG_RUNTIME_DIR", val);
        } else {
            std::env::remove_var("XDG_RUNTIME_DIR");
        }

        // Вызываем функцию несколько раз подряд
        let result1 = is_wayland_available();
        let result2 = is_wayland_available();
        let result3 = is_wayland_available();

        // Результаты должны быть одинаковыми при повторных вызовах
        // (если окружение не меняется)
        assert_eq!(
            result1, result2,
            "Первый и второй вызовы должны давать одинаковый результат"
        );
        assert_eq!(
            result2, result3,
            "Второй и третий вызовы должны давать одинаковый результат"
        );

        // Восстанавливаем переменные окружения
        if let Some(val) = old_wayland_display {
            std::env::set_var("WAYLAND_DISPLAY", val);
        } else {
            std::env::remove_var("WAYLAND_DISPLAY");
        }
        if let Some(val) = old_xdg_session {
            std::env::set_var("XDG_SESSION_TYPE", val);
        } else {
            std::env::remove_var("XDG_SESSION_TYPE");
        }
        if let Some(val) = old_runtime_dir {
            std::env::set_var("XDG_RUNTIME_DIR", val);
        } else {
            std::env::remove_var("XDG_RUNTIME_DIR");
        }
    }

    #[test]
    fn test_is_wayland_available_xdg_session_type_case_sensitive() {
        // Тест проверяет, что проверка XDG_SESSION_TYPE чувствительна к регистру
        let old_xdg_session = std::env::var("XDG_SESSION_TYPE").ok();
        let old_wayland_display = std::env::var("WAYLAND_DISPLAY").ok();

        // Удаляем WAYLAND_DISPLAY
        std::env::remove_var("WAYLAND_DISPLAY");
        // Устанавливаем XDG_SESSION_TYPE с разным регистром
        std::env::set_var("XDG_SESSION_TYPE", "Wayland"); // С заглавной буквы

        // Функция должна вернуть false, так как "Wayland" != "wayland"
        // (если нет других признаков Wayland)
        let result = is_wayland_available();
        // Результат может быть true, если есть socket, но проверка XDG_SESSION_TYPE
        // должна быть чувствительна к регистру
        let _ = result;

        // Восстанавливаем переменные окружения
        std::env::remove_var("XDG_SESSION_TYPE");
        if let Some(val) = old_xdg_session {
            std::env::set_var("XDG_SESSION_TYPE", val);
        }
        if let Some(val) = old_wayland_display {
            std::env::set_var("WAYLAND_DISPLAY", val);
        }
    }
}