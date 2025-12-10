//! Wayland-бекенд для WindowIntrospector (базовая структура).
//!
//! Пока реализована только проверка доступности Wayland окружения.
//! Полная реализация WaylandIntrospector через wlr-foreign-toplevel-management
//! будет добавлена в будущем.

use anyhow::Result;
use std::path::PathBuf;

use crate::metrics::windows::{WindowInfo, WindowIntrospector};

/// Проверяет, доступно ли Wayland окружение.
///
/// Проверяет несколько признаков:
/// 1. Переменная окружения `WAYLAND_DISPLAY` установлена
/// 2. Переменная окружения `XDG_SESSION_TYPE=wayland`
/// 3. Наличие Wayland socket в `/run/user/<uid>/wayland-*` или `$XDG_RUNTIME_DIR/wayland-*`
///
/// Возвращает `true`, если хотя бы один из признаков указывает на Wayland.
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
/// Пока не реализован. Будет использовать wlr-foreign-toplevel-management
/// для получения списка окон и их состояния.
pub struct WaylandIntrospector {
    // TODO: добавить поля для wayland-client соединения
}

impl WaylandIntrospector {
    /// Создаёт новый WaylandIntrospector, подключаясь к Wayland композитору.
    ///
    /// Возвращает ошибку, если Wayland недоступен или не поддерживается.
    pub fn new() -> Result<Self> {
        // TODO: реализовать подключение к Wayland композитору
        anyhow::bail!("WaylandIntrospector not yet implemented")
    }

    /// Проверяет, доступен ли Wayland композитор.
    pub fn is_available() -> bool {
        is_wayland_available()
    }
}

impl WindowIntrospector for WaylandIntrospector {
    fn windows(&self) -> Result<Vec<WindowInfo>> {
        // TODO: реализовать получение списка окон через wlr-foreign-toplevel-management
        anyhow::bail!("WaylandIntrospector::windows() not yet implemented")
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
        // Пока функция не реализована, она должна возвращать ошибку
        match WaylandIntrospector::new() {
            Ok(_) => {
                // Если Wayland доступен и реализация готова, это нормально
                // Но пока мы ожидаем ошибку
                panic!("WaylandIntrospector::new() should return error until implemented")
            }
            Err(_) => {
                // Ожидаемая ошибка, пока не реализовано
            }
        }
    }

    #[test]
    fn test_wayland_introspector_windows() {
        // Тест проверяет получение списка окон
        // Пока функция не реализована, она должна возвращать ошибку
        match WaylandIntrospector::new() {
            Ok(introspector) => {
                match introspector.windows() {
                    Ok(_) => {
                        // Если реализация готова, это нормально
                        // Но пока мы ожидаем ошибку
                        panic!(
                            "WaylandIntrospector::windows() should return error until implemented"
                        )
                    }
                    Err(_) => {
                        // Ожидаемая ошибка, пока не реализовано
                    }
                }
            }
            Err(_) => {
                // Ошибка при создании - это нормально, пока не реализовано
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
}
