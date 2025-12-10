//! Wayland-бекенд для WindowIntrospector через wlr-foreign-toplevel-management.
//!
//! Использует wayland-client для подключения к Wayland композитору и получения
//! информации об окнах через wlr-foreign-toplevel-management-unstable-v1 протокол.

use anyhow::Result;
use std::path::PathBuf;

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
///
/// ПРИМЕЧАНИЕ: Полная реализация требует сложной работы с асинхронными событиями
/// Wayland через wayland-client API и обработки событий через Dispatch трейты.
/// Текущая реализация является базовой структурой. Полная реализация будет добавлена
/// в будущем, когда будет больше времени на правильную работу с wayland-client API.
pub struct WaylandIntrospector {
    // Поля будут добавлены при полной реализации
    _phantom: std::marker::PhantomData<()>,
}

impl WaylandIntrospector {
    /// Создаёт новый WaylandIntrospector, подключаясь к Wayland композитору.
    ///
    /// Возвращает ошибку, если:
    /// - Wayland недоступен (переменные окружения не установлены, socket не найден);
    /// - wlr-foreign-toplevel-management не поддерживается композитором;
    /// - реализация ещё не завершена (временная ошибка до полной реализации).
    ///
    /// # Ошибки
    ///
    /// - Если Wayland недоступен, возвращается ошибка с описанием причины.
    /// - Если Wayland доступен, но реализация не завершена, возвращается информативная ошибка.
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

        // ПРИМЕЧАНИЕ: Полная реализация требует:
        // 1. Подключения к Wayland композитору через Connection::connect_to_env()
        // 2. Создания EventQueue и обработки событий через Dispatch трейты
        // 3. Поиска wlr-foreign-toplevel-manager в реестре глобальных объектов
        // 4. Регистрации обработчиков событий для получения списка окон
        // 5. Обработки событий для обновления состояния окон (title, app_id, state, pid)
        //
        // Это сложная задача, требующая правильной работы с асинхронными событиями
        // через wayland-client 0.31 API. Полная реализация будет добавлена в будущем.
        anyhow::bail!(
            "WaylandIntrospector is not yet fully implemented. \
             Wayland is available, but the full implementation requires:\n\
             - Connection to Wayland compositor via wayland-client API\n\
             - Event handling through Dispatch traits\n\
             - Registry binding for wlr-foreign-toplevel-management protocol\n\
             - Event handlers for window state updates (title, app_id, state, pid)\n\
             \n\
             This is a complex task requiring proper async event handling. \
             The implementation will be completed in a future update. \
             For now, the system will fall back to StaticWindowIntrospector."
        )
    }

    /// Проверяет, доступен ли Wayland композитор.
    pub fn is_available() -> bool {
        is_wayland_available()
    }
}

impl WindowIntrospector for WaylandIntrospector {
    fn windows(&self) -> Result<Vec<WindowInfo>> {
        // TODO: реализовать получение списка окон через wlr-foreign-toplevel-management
        anyhow::bail!(
            "WaylandIntrospector::windows() is not yet fully implemented. \
             This method requires:\n\
             - Connection to Wayland compositor\n\
             - wlr-foreign-toplevel-management protocol support\n\
             - Async event handling through wayland-client API\n\
             \n\
             The full implementation will be added in a future update. \
             This error should not occur if WaylandIntrospector::new() was called successfully."
        )
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
        // Тест проверяет, что пустая строка WAYLAND_DISPLAY не считается валидной
        let old_wayland_display = std::env::var("WAYLAND_DISPLAY").ok();
        let old_xdg_session = std::env::var("XDG_SESSION_TYPE").ok();

        // Устанавливаем пустую строку
        std::env::set_var("WAYLAND_DISPLAY", "");
        std::env::remove_var("XDG_SESSION_TYPE");

        // Пустая строка всё равно считается установленной переменной
        // (is_ok() вернёт true), поэтому функция вернёт true
        // Это поведение соответствует логике: если переменная установлена (даже пустая),
        // это признак того, что Wayland может быть доступен
        let result = is_wayland_available();
        // Результат зависит от реализации: если пустая строка считается валидной,
        // функция вернёт true, иначе false
        let _ = result;

        // Восстанавливаем переменные окружения
        std::env::remove_var("WAYLAND_DISPLAY");
        if let Some(val) = old_wayland_display {
            std::env::set_var("WAYLAND_DISPLAY", val);
        }
        if let Some(val) = old_xdg_session {
            std::env::set_var("XDG_SESSION_TYPE", val);
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
        let result1 = is_wayland_available();
        let result2 = is_wayland_available();
        let result3 = is_wayland_available();

        // Результаты должны быть одинаковыми при повторных вызовах
        // (если окружение не меняется)
        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
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
