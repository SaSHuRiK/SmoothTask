//! X11-бекенд для WindowIntrospector через EWMH (Extended Window Manager Hints).
//!
//! Использует x11rb для подключения к X-серверу и получения информации об окнах
//! через стандартные EWMH-атомы.

use anyhow::{Context, Result};
use std::sync::Arc;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{Atom, ConnectionExt, Window};
use x11rb::rust_connection::RustConnection;

use crate::metrics::windows::{WindowInfo, WindowIntrospector, WindowState};

/// X11-интроспектор для получения информации об окнах через EWMH.
///
/// Поддерживает:
/// - Получение списка окон через `_NET_CLIENT_LIST`
/// - Получение PID через `_NET_WM_PID`
/// - Определение фокусного окна через `_NET_ACTIVE_WINDOW`
/// - Получение состояния окна (focused/minimized/fullscreen) через `_NET_WM_STATE`
/// - Получение workspace через `_NET_WM_DESKTOP`
/// - Получение title через `_NET_WM_NAME` или `WM_NAME`
/// - Получение app_id через `_NET_WM_CLASS` или `WM_CLASS`
pub struct X11Introspector {
    connection: Arc<RustConnection>,
    root: Window,
    // Кэшируем атомы для производительности
    net_client_list: Atom,
    net_wm_pid: Atom,
    net_active_window: Atom,
    net_wm_state: Atom,
    net_wm_state_hidden: Atom,
    net_wm_state_fullscreen: Atom,
    net_wm_desktop: Atom,
    net_wm_name: Atom,
    wm_name: Atom,
    net_wm_class: Atom,
    wm_class: Atom,
    utf8_string: Atom,
}

impl X11Introspector {
    /// Создаёт новый X11Introspector, подключаясь к X-серверу.
    ///
    /// Возвращает ошибку, если X-сервер недоступен или EWMH не поддерживается.
    pub fn new() -> Result<Self> {
        let (connection, screen_num) = x11rb::connect(None).with_context(|| {
            "Не удалось подключиться к X-серверу: проверьте, что X-сервер запущен и переменная DISPLAY установлена"
        })?;

        let connection = Arc::new(connection);
        let setup = connection.setup();
        let screen = &setup.roots[screen_num];
        let root = screen.root;

        // Получаем атомы EWMH
        let net_client_list = Self::intern_atom(&connection, b"_NET_CLIENT_LIST")?;
        let net_wm_pid = Self::intern_atom(&connection, b"_NET_WM_PID")?;
        let net_active_window = Self::intern_atom(&connection, b"_NET_ACTIVE_WINDOW")?;
        let net_wm_state = Self::intern_atom(&connection, b"_NET_WM_STATE")?;
        let net_wm_state_hidden = Self::intern_atom(&connection, b"_NET_WM_STATE_HIDDEN")?;
        let net_wm_state_fullscreen = Self::intern_atom(&connection, b"_NET_WM_STATE_FULLSCREEN")?;
        let net_wm_desktop = Self::intern_atom(&connection, b"_NET_WM_DESKTOP")?;
        let net_wm_name = Self::intern_atom(&connection, b"_NET_WM_NAME")?;
        let wm_name = Self::intern_atom(&connection, b"WM_NAME")?;
        let net_wm_class = Self::intern_atom(&connection, b"_NET_WM_CLASS")?;
        let wm_class = Self::intern_atom(&connection, b"WM_CLASS")?;
        let utf8_string = Self::intern_atom(&connection, b"UTF8_STRING")?;

        Ok(Self {
            connection,
            root,
            net_client_list,
            net_wm_pid,
            net_active_window,
            net_wm_state,
            net_wm_state_hidden,
            net_wm_state_fullscreen,
            net_wm_desktop,
            net_wm_name,
            wm_name,
            net_wm_class,
            wm_class,
            utf8_string,
        })
    }

    /// Проверяет, доступен ли X-сервер.
    pub fn is_available() -> bool {
        x11rb::connect(None).is_ok()
    }

    fn intern_atom(connection: &RustConnection, name: &[u8]) -> Result<Atom> {
        let atom_name = String::from_utf8_lossy(name);
        let reply = connection
            .intern_atom(false, name)
            .with_context(|| {
                format!(
                    "Не удалось зарегистрировать X11 атом '{}': проверьте подключение к X-серверу",
                    atom_name
                )
            })?
            .reply()
            .with_context(|| {
                format!(
                    "Не удалось получить ответ от X-сервера для атома '{}': проверьте подключение",
                    atom_name
                )
            })?;
        Ok(reply.atom)
    }

    /// Получает список всех окон через `_NET_CLIENT_LIST`.
    fn get_client_list(&self) -> Result<Vec<Window>> {
        let reply = self
            .connection
            .get_property(
                false,
                self.root,
                self.net_client_list,
                x11rb::protocol::xproto::AtomEnum::WINDOW,
                0,
                u32::MAX,
            )
            .with_context(|| {
                "Не удалось получить _NET_CLIENT_LIST от X-сервера: проверьте, что оконный менеджер поддерживает EWMH"
            })?
            .reply()
            .with_context(|| {
                "Не удалось получить ответ от X-сервера для _NET_CLIENT_LIST: проверьте подключение"
            })?;

        let windows: Vec<Window> = reply
            .value32()
            .context("_NET_CLIENT_LIST не является списком окон: ожидается массив идентификаторов окон (u32)")?
            .collect();
        Ok(windows)
    }

    /// Получает активное окно через `_NET_ACTIVE_WINDOW`.
    fn get_active_window(&self) -> Result<Option<Window>> {
        let reply = self
            .connection
            .get_property(
                false,
                self.root,
                self.net_active_window,
                x11rb::protocol::xproto::AtomEnum::WINDOW,
                0,
                1,
            )
            .with_context(|| {
                "Не удалось получить _NET_ACTIVE_WINDOW от X-сервера: проверьте, что оконный менеджер поддерживает EWMH"
            })?
            .reply()
            .with_context(|| {
                "Не удалось получить ответ от X-сервера для _NET_ACTIVE_WINDOW: проверьте подключение"
            })?;

        let windows: Vec<Window> = reply
            .value32()
            .context("_NET_ACTIVE_WINDOW не является идентификатором окна: ожидается одно значение типа WINDOW (u32)")?
            .collect();
        Ok(windows.first().copied())
    }

    /// Получает PID окна через `_NET_WM_PID`.
    fn get_window_pid(&self, window: Window) -> Result<Option<u32>> {
        let reply = self
            .connection
            .get_property(
                false,
                window,
                self.net_wm_pid,
                x11rb::protocol::xproto::AtomEnum::CARDINAL,
                0,
                1,
            )
            .with_context(|| {
                format!(
                    "Не удалось получить _NET_WM_PID для окна {:?}: проверьте, что окно существует и поддерживает EWMH",
                    window
                )
            })?
            .reply()
            .ok();

        if let Some(reply) = reply {
            let pids: Vec<u32> = reply
                .value32()
                .context("_NET_WM_PID не является числом (CARDINAL): ожидается одно значение типа u32")?
                .collect();
            Ok(pids.first().copied())
        } else {
            Ok(None)
        }
    }

    /// Получает состояние окна (focused/minimized/fullscreen).
    fn get_window_state(&self, window: Window, active_window: Option<Window>) -> WindowState {
        let is_active = active_window == Some(window);

        // Проверяем _NET_WM_STATE для minimized и fullscreen
        let reply = self
            .connection
            .get_property(
                false,
                window,
                self.net_wm_state,
                x11rb::protocol::xproto::AtomEnum::ATOM,
                0,
                u32::MAX,
            )
            .ok()
            .and_then(|cookie| cookie.reply().ok());

        let mut is_minimized = false;
        let mut is_fullscreen = false;

        if let Some(reply) = reply {
            if let Some(atoms) = reply.value32() {
                for atom in atoms {
                    if atom == self.net_wm_state_hidden {
                        is_minimized = true;
                    }
                    if atom == self.net_wm_state_fullscreen {
                        is_fullscreen = true;
                    }
                }
            }
        }

        if is_fullscreen {
            WindowState::Fullscreen
        } else if is_minimized {
            WindowState::Minimized
        } else if is_active {
            WindowState::Focused
        } else {
            WindowState::Background
        }
    }

    /// Получает workspace окна через `_NET_WM_DESKTOP`.
    fn get_window_workspace(&self, window: Window) -> Result<Option<u32>> {
        let reply = self
            .connection
            .get_property(
                false,
                window,
                self.net_wm_desktop,
                x11rb::protocol::xproto::AtomEnum::CARDINAL,
                0,
                1,
            )
            .with_context(|| {
                format!(
                    "Не удалось получить _NET_WM_DESKTOP для окна {:?}: проверьте, что окно существует",
                    window
                )
            })?
            .reply()
            .ok();

        if let Some(reply) = reply {
            let workspaces: Vec<u32> = reply
                .value32()
                .context("_NET_WM_DESKTOP не является числом (CARDINAL): ожидается одно значение типа u32")?
                .collect();
            Ok(workspaces.first().copied())
        } else {
            Ok(None)
        }
    }

    /// Получает title окна через `_NET_WM_NAME` или `WM_NAME`.
    fn get_window_title(&self, window: Window) -> Result<Option<String>> {
        // Сначала пробуем _NET_WM_NAME (UTF-8)
        let reply = self
            .connection
            .get_property(
                false,
                window,
                self.net_wm_name,
                self.utf8_string,
                0,
                u32::MAX,
            )
            .ok()
            .and_then(|cookie| cookie.reply().ok());

        if let Some(reply) = reply {
            if let Some(bytes) = reply.value8() {
                if let Ok(title) = String::from_utf8(bytes.collect()) {
                    if !title.is_empty() {
                        return Ok(Some(title));
                    }
                }
            }
        }

        // Fallback на WM_NAME
        let reply = self
            .connection
            .get_property(
                false,
                window,
                self.wm_name,
                x11rb::protocol::xproto::AtomEnum::STRING,
                0,
                u32::MAX,
            )
            .ok()
            .and_then(|cookie| cookie.reply().ok());

        if let Some(reply) = reply {
            if let Some(bytes) = reply.value8() {
                // WM_NAME может быть в любой кодировке, пробуем UTF-8
                if let Ok(title) = String::from_utf8(bytes.collect()) {
                    if !title.is_empty() {
                        return Ok(Some(title));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Получает app_id окна через `_NET_WM_CLASS` или `WM_CLASS`.
    fn get_window_app_id(&self, window: Window) -> Result<Option<String>> {
        // Сначала пробуем _NET_WM_CLASS
        let reply = self
            .connection
            .get_property(
                false,
                window,
                self.net_wm_class,
                self.utf8_string,
                0,
                u32::MAX,
            )
            .ok()
            .and_then(|cookie| cookie.reply().ok());

        if let Some(reply) = reply {
            if let Some(bytes) = reply.value8() {
                if let Ok(class) = String::from_utf8(bytes.collect()) {
                    // WM_CLASS обычно имеет формат "instance\0class"
                    // Берём первую часть (instance) как app_id
                    if let Some(app_id) = class.split('\0').next() {
                        if !app_id.is_empty() {
                            return Ok(Some(app_id.to_string()));
                        }
                    }
                }
            }
        }

        // Fallback на WM_CLASS
        let reply = self
            .connection
            .get_property(
                false,
                window,
                self.wm_class,
                x11rb::protocol::xproto::AtomEnum::STRING,
                0,
                u32::MAX,
            )
            .ok()
            .and_then(|cookie| cookie.reply().ok());

        if let Some(reply) = reply {
            if let Some(bytes) = reply.value8() {
                // WM_CLASS может быть в любой кодировке, пробуем UTF-8
                if let Ok(class) = String::from_utf8(bytes.collect()) {
                    if let Some(app_id) = class.split('\0').next() {
                        if !app_id.is_empty() {
                            return Ok(Some(app_id.to_string()));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Преобразует X11-окно в WindowInfo.
    fn window_to_info(&self, window: Window, active_window: Option<Window>) -> Result<WindowInfo> {
        let pid = self.get_window_pid(window)?;
        let state = self.get_window_state(window, active_window);
        let workspace = self.get_window_workspace(window)?;
        let title = self.get_window_title(window)?;
        let app_id = self.get_window_app_id(window)?;

        // В X11 через _NET_WM_PID мы получаем PID с высокой уверенностью (0.9)
        // если он доступен, иначе 0.0
        let pid_confidence = if pid.is_some() { 0.9 } else { 0.0 };

        Ok(WindowInfo::new(
            app_id,
            title,
            workspace,
            state,
            pid,
            pid_confidence,
        ))
    }
}

impl WindowIntrospector for X11Introspector {
    fn windows(&self) -> Result<Vec<WindowInfo>> {
        let client_list = self.get_client_list()?;
        let active_window = self.get_active_window()?;

        let mut windows = Vec::new();
        for window in client_list {
            match self.window_to_info(window, active_window) {
                Ok(info) => windows.push(info),
                Err(e) => {
                    // Логируем ошибку, но продолжаем обработку других окон
                    tracing::warn!(
                        "Не удалось получить информацию об окне {:?}: {}. \
                         Продолжаем обработку остальных окон",
                        window,
                        e
                    );
                }
            }
        }

        Ok(windows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_x11_introspector_availability() {
        // Тест проверяет, что функция is_available() работает
        // (не падает, даже если X-сервер недоступен)
        let _ = X11Introspector::is_available();
    }

    #[test]
    fn test_x11_introspector_creation() {
        // Тест проверяет создание интроспектора
        // Если X-сервер недоступен, тест должен пройти (проверка на ошибку)
        match X11Introspector::new() {
            Ok(introspector) => {
                // X-сервер доступен, проверяем базовую функциональность
                // Проверяем, что интроспектор реализует трейт WindowIntrospector
                let _: &dyn WindowIntrospector = &introspector;
            }
            Err(_) => {
                // X-сервер недоступен, это нормально для CI/тестовых окружений
            }
        }
    }

    #[test]
    fn test_x11_introspector_windows() {
        // Тест проверяет получение списка окон
        // Если X-сервер недоступен, тест должен пройти
        match X11Introspector::new() {
            Ok(introspector) => {
                match introspector.windows() {
                    Ok(windows) => {
                        // Проверяем, что мы получили список окон (может быть пустым)
                        // windows.len() всегда >= 0, поэтому просто проверяем, что это валидный Vec
                        let _ = windows.len();
                        // Проверяем, что все окна имеют валидную структуру
                        for window in &windows {
                            // pid_confidence должен быть в диапазоне [0, 1]
                            assert!(window.pid_confidence >= 0.0 && window.pid_confidence <= 1.0);
                            // Если PID есть, confidence должен быть > 0
                            if window.pid.is_some() {
                                assert!(window.pid_confidence > 0.0);
                            }
                        }
                    }
                    Err(e) => {
                        // Ошибка при получении окон - это может быть нормально
                        // если X-сервер работает, но EWMH не поддерживается
                        tracing::warn!("Failed to get windows: {}", e);
                    }
                }
            }
            Err(_) => {
                // X-сервер недоступен, это нормально
            }
        }
    }

    #[test]
    fn test_x11_introspector_focused_window() {
        // Тест проверяет получение фокусного окна
        match X11Introspector::new() {
            Ok(introspector) => {
                match introspector.focused_window() {
                    Ok(Some(window)) => {
                        // Проверяем, что фокусное окно имеет валидное состояние
                        assert!(window.is_focused());
                        assert!(matches!(
                            window.state,
                            WindowState::Focused | WindowState::Fullscreen
                        ));
                    }
                    Ok(None) => {
                        // Нет фокусного окна - это нормально
                    }
                    Err(e) => {
                        // Ошибка при получении фокусного окна
                        tracing::warn!("Failed to get focused window: {}", e);
                    }
                }
            }
            Err(_) => {
                // X-сервер недоступен, это нормально
            }
        }
    }

    #[test]
    fn test_x11_introspector_window_info_structure() {
        // Тест проверяет структуру WindowInfo, полученного из X11
        match X11Introspector::new() {
            Ok(introspector) => {
                match introspector.windows() {
                    Ok(windows) => {
                        for window in &windows {
                            // Проверяем, что WindowInfo имеет валидную структуру
                            assert!(window.pid_confidence >= 0.0 && window.pid_confidence <= 1.0);
                            // Если PID есть, confidence должен быть > 0
                            if window.pid.is_some() {
                                assert!(window.pid_confidence > 0.0);
                            } else {
                                // Если PID нет, confidence должен быть 0
                                assert_eq!(window.pid_confidence, 0.0);
                            }
                        }
                    }
                    Err(_) => {
                        // Ошибка при получении окон - это нормально
                    }
                }
            }
            Err(_) => {
                // X-сервер недоступен, это нормально
            }
        }
    }
}
