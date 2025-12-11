//! Абстракции для получения информации об окнах и текущем фокусе.
//!
//! Реальные бекенды (X11/Wayland) будут подключаться позже, но каркас
//! позволяет уже сейчас работать с нормализованными структурами и писать
//! юнит-тесты вокруг логики выбора активного окна.

pub use crate::metrics::windows_wayland::{is_wayland_available, WaylandIntrospector};
pub use crate::metrics::windows_x11::X11Introspector;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Состояние окна с точки зрения фокуса/видимости.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowState {
    Focused,
    Fullscreen,
    Background,
    Minimized,
}

impl WindowState {
    /// Считается ли окно активным для пользователя.
    pub fn is_focused(self) -> bool {
        matches!(self, WindowState::Focused | WindowState::Fullscreen)
    }
}

/// Нормализованная информация об окне, независимая от конкретного композитора.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowInfo {
    pub app_id: Option<String>,
    pub title: Option<String>,
    pub workspace: Option<u32>,
    pub state: WindowState,
    pub pid: Option<u32>,
    /// Насколько уверенно композитор сообщил PID (0.0–1.0).
    pub pid_confidence: f32,
}

impl WindowInfo {
    /// Упрощённый конструктор с клэмпингом confidence в диапазон [0, 1].
    pub fn new(
        app_id: Option<String>,
        title: Option<String>,
        workspace: Option<u32>,
        state: WindowState,
        pid: Option<u32>,
        pid_confidence: f32,
    ) -> Self {
        // Если PID неизвестен — confidence принудительно обнуляем, даже если бекенд прислал значение.
        let pid_confidence = if pid.is_none() || pid_confidence.is_nan() {
            0.0
        } else {
            pid_confidence.clamp(0.0, 1.0)
        };
        Self {
            app_id,
            title,
            workspace,
            state,
            pid,
            pid_confidence,
        }
    }

    /// Активное ли это окно с точки зрения пользователя.
    pub fn is_focused(&self) -> bool {
        self.state.is_focused()
    }
}

/// Общий интерфейс для получения списка окон из конкретного бекенда.
pub trait WindowIntrospector: Send + Sync {
    /// Возвращает снапшот окон.
    fn windows(&self) -> Result<Vec<WindowInfo>>;

    /// Удобный шорткат для активного окна.
    fn focused_window(&self) -> Result<Option<WindowInfo>> {
        let windows = self.windows()?;
        Ok(select_focused_window(&windows))
    }
}

/// Выбрать наиболее релевантное активное окно из списка.
///
/// Правила:
/// - если есть fullscreen — возвращаем первое fullscreen-окно;
/// - иначе возвращаем первое окно со статусом Focused;
/// - если активных нет — None.
pub fn select_focused_window(windows: &[WindowInfo]) -> Option<WindowInfo> {
    pick_best_by_confidence(
        windows
            .iter()
            .filter(|w| w.state == WindowState::Fullscreen),
    )
    .or_else(|| pick_best_by_confidence(windows.iter().filter(|w| w.state == WindowState::Focused)))
}

fn pick_best_by_confidence<'a>(
    candidates: impl Iterator<Item = &'a WindowInfo>,
) -> Option<WindowInfo> {
    candidates
        .max_by(|a, b| a.pid_confidence.total_cmp(&b.pid_confidence))
        .cloned()
}

/// Получить информацию об окне для конкретного PID.
///
/// Функция ищет окно с указанным PID среди всех окон, возвращаемых introspector.
/// Если для PID найдено несколько окон, возвращается окно с наибольшим `pid_confidence`.
/// Если окно не найдено (PID отсутствует или нет окон с таким PID), возвращается `None`.
/// Окна без PID (где `pid == None`) игнорируются.
///
/// # Аргументы
///
/// * `introspector` - интроспектор окон для получения списка окон
/// * `pid` - PID процесса, для которого нужно найти окно
///
/// # Возвращает
///
/// * `Ok(Some(WindowInfo))` - если окно с указанным PID найдено
/// * `Ok(None)` - если окно с указанным PID не найдено
/// * `Err(...)` - если произошла ошибка при получении списка окон от introspector
///
/// # Примеры использования
///
/// ## Базовое использование
///
/// ```rust
/// use smoothtask_core::metrics::windows::{get_window_info_by_pid, StaticWindowIntrospector, WindowInfo, WindowState};
///
/// let windows = vec![
///     WindowInfo::new(
///         Some("firefox".to_string()),
///         Some("Mozilla Firefox".to_string()),
///         Some(1),
///         WindowState::Focused,
///         Some(1234),
///         1.0,
///     ),
/// ];
/// let introspector = StaticWindowIntrospector::new(windows);
///
/// // Найти окно для PID 1234
/// let window = get_window_info_by_pid(&introspector, 1234).unwrap();
/// assert!(window.is_some());
/// assert_eq!(window.unwrap().pid, Some(1234));
///
/// // Попытка найти несуществующий PID
/// let window = get_window_info_by_pid(&introspector, 9999).unwrap();
/// assert!(window.is_none());
/// ```
///
/// ## Выбор окна с наибольшим confidence при множественных окнах
///
/// ```rust
/// use smoothtask_core::metrics::windows::{get_window_info_by_pid, StaticWindowIntrospector, WindowInfo, WindowState};
///
/// let windows = vec![
///     WindowInfo::new(
///         Some("app".to_string()),
///         Some("Window 1".to_string()),
///         Some(1),
///         WindowState::Focused,
///         Some(100),
///         0.5, // меньший confidence
///     ),
///     WindowInfo::new(
///         Some("app".to_string()),
///         Some("Window 2".to_string()),
///         Some(1),
///         WindowState::Background,
///         Some(100), // тот же PID
///         0.9, // больший confidence - будет выбран
///     ),
/// ];
/// let introspector = StaticWindowIntrospector::new(windows);
///
/// let window = get_window_info_by_pid(&introspector, 100).unwrap();
/// assert!(window.is_some());
/// assert_eq!(window.unwrap().title, Some("Window 2".to_string()));
/// ```
///
/// ## Обработка ошибок introspector
///
/// ```rust
/// use smoothtask_core::metrics::windows::{get_window_info_by_pid, WindowIntrospector};
/// use anyhow::Result;
///
/// struct ErrorIntrospector;
///
/// impl WindowIntrospector for ErrorIntrospector {
///     fn windows(&self) -> Result<Vec<smoothtask_core::metrics::windows::WindowInfo>> {
///         anyhow::bail!("Failed to get windows")
///     }
/// }
///
/// let introspector = ErrorIntrospector;
/// let result = get_window_info_by_pid(&introspector, 100);
/// assert!(result.is_err());
/// ```
///
/// # Примечания
///
/// - Функция игнорирует окна без PID (`pid == None`)
/// - Если для одного PID есть несколько окон, выбирается окно с наибольшим `pid_confidence`
/// - Функция не гарантирует, что выбранное окно является "активным" или "фокусным" - она просто
///   выбирает окно с наибольшим confidence среди всех окон с указанным PID
pub fn get_window_info_by_pid(
    introspector: &dyn WindowIntrospector,
    pid: u32,
) -> Result<Option<WindowInfo>> {
    let windows = introspector.windows()?;
    Ok(windows
        .into_iter()
        .filter(|w| w.pid == Some(pid))
        .max_by(|a, b| a.pid_confidence.total_cmp(&b.pid_confidence)))
}

/// Построить маппинг PID -> WindowInfo для всех окон.
///
/// Функция создаёт HashMap, где ключом является PID процесса, а значением - информация об окне.
/// Если для одного PID есть несколько окон, выбирается окно с наибольшим `pid_confidence`.
/// Окна без PID (где `pid == None`) игнорируются и не попадают в результирующий мап.
///
/// # Аргументы
///
/// * `introspector` - интроспектор окон для получения списка окон
///
/// # Возвращает
///
/// * `Ok(HashMap<u32, WindowInfo>)` - маппинг PID -> WindowInfo для всех окон с известным PID
/// * `Err(...)` - если произошла ошибка при получении списка окон от introspector
///
/// # Примеры использования
///
/// ## Базовое использование
///
/// ```rust
/// use smoothtask_core::metrics::windows::{build_pid_to_window_map, StaticWindowIntrospector, WindowInfo, WindowState};
///
/// let windows = vec![
///     WindowInfo::new(
///         Some("firefox".to_string()),
///         Some("Mozilla Firefox".to_string()),
///         Some(1),
///         WindowState::Focused,
///         Some(1234),
///         1.0,
///     ),
///     WindowInfo::new(
///         Some("code".to_string()),
///         Some("VS Code".to_string()),
///         Some(1),
///         WindowState::Background,
///         Some(5678),
///         0.9,
///     ),
/// ];
/// let introspector = StaticWindowIntrospector::new(windows);
///
/// let map = build_pid_to_window_map(&introspector).unwrap();
/// assert_eq!(map.len(), 2);
/// assert!(map.contains_key(&1234));
/// assert!(map.contains_key(&5678));
/// assert_eq!(map.get(&1234).unwrap().app_id, Some("firefox".to_string()));
/// ```
///
/// ## Выбор окна с наибольшим confidence при множественных окнах
///
/// ```rust
/// use smoothtask_core::metrics::windows::{build_pid_to_window_map, StaticWindowIntrospector, WindowInfo, WindowState};
///
/// let windows = vec![
///     WindowInfo::new(
///         Some("app".to_string()),
///         Some("Window 1".to_string()),
///         Some(1),
///         WindowState::Focused,
///         Some(100),
///         0.5, // меньший confidence
///     ),
///     WindowInfo::new(
///         Some("app".to_string()),
///         Some("Window 2".to_string()),
///         Some(1),
///         WindowState::Background,
///         Some(100), // тот же PID
///         0.9, // больший confidence - будет выбран
///     ),
/// ];
/// let introspector = StaticWindowIntrospector::new(windows);
///
/// let map = build_pid_to_window_map(&introspector).unwrap();
/// assert_eq!(map.len(), 1);
/// assert_eq!(map.get(&100).unwrap().title, Some("Window 2".to_string()));
/// ```
///
/// ## Игнорирование окон без PID
///
/// ```rust
/// use smoothtask_core::metrics::windows::{build_pid_to_window_map, StaticWindowIntrospector, WindowInfo, WindowState};
///
/// let windows = vec![
///     WindowInfo::new(
///         Some("app1".to_string()),
///         Some("Window 1".to_string()),
///         Some(1),
///         WindowState::Focused,
///         Some(100), // есть PID
///         1.0,
///     ),
///     WindowInfo::new(
///         Some("app2".to_string()),
///         Some("Window 2".to_string()),
///         Some(1),
///         WindowState::Background,
///         None, // нет PID - будет проигнорировано
///         0.9,
///     ),
/// ];
/// let introspector = StaticWindowIntrospector::new(windows);
///
/// let map = build_pid_to_window_map(&introspector).unwrap();
/// assert_eq!(map.len(), 1); // только окно с PID
/// assert!(map.contains_key(&100));
/// ```
///
/// ## Обработка ошибок introspector
///
/// ```rust
/// use smoothtask_core::metrics::windows::{build_pid_to_window_map, WindowIntrospector};
/// use anyhow::Result;
///
/// struct ErrorIntrospector;
///
/// impl WindowIntrospector for ErrorIntrospector {
///     fn windows(&self) -> Result<Vec<smoothtask_core::metrics::windows::WindowInfo>> {
///         anyhow::bail!("Failed to get windows")
///     }
/// }
///
/// let introspector = ErrorIntrospector;
/// let result = build_pid_to_window_map(&introspector);
/// assert!(result.is_err());
/// ```
///
/// ## Использование в цикле демона
///
/// ```rust
/// use smoothtask_core::metrics::windows::{build_pid_to_window_map, WindowIntrospector};
///
/// fn process_windows(introspector: &dyn WindowIntrospector) -> anyhow::Result<()> {
///     let pid_to_window = build_pid_to_window_map(introspector)?;
///
///     for (pid, window) in pid_to_window {
///         println!("PID {}: {} ({:?})", pid, window.app_id.as_deref().unwrap_or("unknown"), window.state);
///     }
///
///     Ok(())
/// }
/// ```
///
/// # Примечания
///
/// - Функция игнорирует окна без PID (`pid == None`)
/// - Если для одного PID есть несколько окон, выбирается окно с наибольшим `pid_confidence`
/// - Результирующий HashMap содержит только окна с известным PID
/// - Пустой список окон вернёт пустой HashMap (не ошибку)
pub fn build_pid_to_window_map(
    introspector: &dyn WindowIntrospector,
) -> Result<std::collections::HashMap<u32, WindowInfo>> {
    let windows = introspector.windows()?;
    let mut map = std::collections::HashMap::new();

    for window in windows {
        if let Some(pid) = window.pid {
            // Если для этого PID уже есть окно, выбираем то, у которого больше confidence
            map.entry(pid)
                .and_modify(|existing: &mut WindowInfo| {
                    if window.pid_confidence > existing.pid_confidence {
                        *existing = window.clone();
                    }
                })
                .or_insert(window);
        }
    }

    Ok(map)
}

/// Простой бекенд для тестов и отладки, возвращающий заранее подготовленный список окон.
#[derive(Debug, Clone)]
pub struct StaticWindowIntrospector {
    windows: Vec<WindowInfo>,
}

impl StaticWindowIntrospector {
    /// Создаёт новый StaticWindowIntrospector с заданным списком окон.
    ///
    /// Этот интроспектор используется для тестирования и отладки, возвращая
    /// заранее подготовленный список окон без подключения к реальному композитору.
    ///
    /// # Аргументы
    ///
    /// * `windows` - список окон, который будет возвращаться при вызове `windows()`
    ///
    /// # Примеры
    ///
    /// ```rust
    /// use smoothtask_core::metrics::windows::{StaticWindowIntrospector, WindowInfo, WindowState, WindowIntrospector};
    ///
    /// let windows = vec![
    ///     WindowInfo::new(
    ///         Some("firefox".to_string()),
    ///         Some("Mozilla Firefox".to_string()),
    ///         Some(1),
    ///         WindowState::Focused,
    ///         Some(1234),
    ///         1.0,
    ///     ),
    /// ];
    /// let introspector = StaticWindowIntrospector::new(windows);
    /// let result = introspector.windows().unwrap();
    /// assert_eq!(result.len(), 1);
    /// ```
    pub fn new(windows: Vec<WindowInfo>) -> Self {
        Self { windows }
    }
}

impl WindowIntrospector for StaticWindowIntrospector {
    fn windows(&self) -> Result<Vec<WindowInfo>> {
        Ok(self.windows.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(state: WindowState, pid_confidence: f32) -> WindowInfo {
        WindowInfo::new(
            Some("app".to_string()),
            Some("title".to_string()),
            Some(1),
            state,
            Some(42),
            pid_confidence,
        )
    }

    #[test]
    fn fullscreen_takes_priority_over_focused() {
        let windows = vec![
            window(WindowState::Focused, 1.0),
            window(WindowState::Fullscreen, 0.5),
        ];
        let focused = select_focused_window(&windows).expect("fullscreen window expected");
        assert_eq!(focused.state, WindowState::Fullscreen);
    }

    #[test]
    fn fullscreen_prefers_higher_confidence() {
        let windows = vec![
            window(WindowState::Fullscreen, 0.3),
            window(WindowState::Fullscreen, 0.9),
        ];
        let focused = select_focused_window(&windows).expect("fullscreen window expected");
        assert!((focused.pid_confidence - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn no_active_window_returns_none() {
        let windows = vec![
            window(WindowState::Background, 1.0),
            window(WindowState::Minimized, 1.0),
        ];
        assert!(select_focused_window(&windows).is_none());
    }

    #[test]
    fn pid_confidence_is_clamped() {
        let info = WindowInfo::new(None, None, None, WindowState::Focused, Some(1), 2.5);
        assert!((info.pid_confidence - 1.0).abs() < f32::EPSILON);

        let info = WindowInfo::new(None, None, None, WindowState::Focused, Some(1), -0.5);
        assert!((info.pid_confidence - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pid_confidence_is_zero_without_pid() {
        let info = WindowInfo::new(
            Some("app".to_string()),
            None,
            Some(0),
            WindowState::Focused,
            None,
            0.9,
        );
        assert_eq!(info.pid, None);
        assert!((info.pid_confidence - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn static_introspector_exposes_focused_window() {
        let windows = vec![
            window(WindowState::Background, 1.0),
            window(WindowState::Focused, 0.8),
        ];
        let introspector = StaticWindowIntrospector::new(windows);
        let focused = introspector.focused_window().unwrap().unwrap();
        assert_eq!(focused.state, WindowState::Focused);
        assert!(focused.is_focused());
    }

    #[test]
    fn nan_pid_confidence_is_treated_as_zero() {
        let info = WindowInfo::new(None, None, None, WindowState::Focused, None, f32::NAN);
        assert!((info.pid_confidence - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn get_window_info_by_pid_finds_window() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Title1".to_string()),
                Some(1),
                WindowState::Focused,
                Some(100),
                0.9,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Title2".to_string()),
                Some(1),
                WindowState::Background,
                Some(200),
                0.8,
            ),
        ];
        let introspector = StaticWindowIntrospector::new(windows);
        let window = get_window_info_by_pid(&introspector, 100).unwrap();
        assert!(window.is_some());
        let window = window.unwrap();
        assert_eq!(window.pid, Some(100));
        assert_eq!(window.app_id, Some("app1".to_string()));
    }

    #[test]
    fn get_window_info_by_pid_returns_none_for_missing_pid() {
        let windows = vec![WindowInfo::new(
            Some("app1".to_string()),
            None,
            None,
            WindowState::Focused,
            Some(100),
            0.9,
        )];
        let introspector = StaticWindowIntrospector::new(windows);
        let window = get_window_info_by_pid(&introspector, 999).unwrap();
        assert!(window.is_none());
    }

    #[test]
    fn get_window_info_by_pid_prefers_higher_confidence() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Title1".to_string()),
                Some(1),
                WindowState::Focused,
                Some(100),
                0.5,
            ),
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Title2".to_string()),
                Some(1),
                WindowState::Background,
                Some(100),
                0.9,
            ),
        ];
        let introspector = StaticWindowIntrospector::new(windows);
        let window = get_window_info_by_pid(&introspector, 100).unwrap();
        assert!(window.is_some());
        let window = window.unwrap();
        assert!((window.pid_confidence - 0.9).abs() < f32::EPSILON);
        assert_eq!(window.title, Some("Title2".to_string()));
    }

    #[test]
    fn build_pid_to_window_map_creates_correct_mapping() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Title1".to_string()),
                Some(1),
                WindowState::Focused,
                Some(100),
                0.9,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Title2".to_string()),
                Some(1),
                WindowState::Background,
                Some(200),
                0.8,
            ),
            WindowInfo::new(
                Some("app3".to_string()),
                None,
                None,
                WindowState::Minimized,
                None,
                0.0,
            ),
        ];
        let introspector = StaticWindowIntrospector::new(windows);
        let map = build_pid_to_window_map(&introspector).unwrap();

        assert_eq!(map.len(), 2);
        assert!(map.contains_key(&100));
        assert!(map.contains_key(&200));
        assert!(!map.contains_key(&0)); // окно без PID не должно попасть в мап

        assert_eq!(map.get(&100).unwrap().app_id, Some("app1".to_string()));
        assert_eq!(map.get(&200).unwrap().app_id, Some("app2".to_string()));
    }

    #[test]
    fn build_pid_to_window_map_prefers_higher_confidence_for_duplicate_pids() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Title1".to_string()),
                Some(1),
                WindowState::Focused,
                Some(100),
                0.5,
            ),
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Title2".to_string()),
                Some(1),
                WindowState::Background,
                Some(100),
                0.9,
            ),
        ];
        let introspector = StaticWindowIntrospector::new(windows);
        let map = build_pid_to_window_map(&introspector).unwrap();

        assert_eq!(map.len(), 1);
        let window = map.get(&100).unwrap();
        assert!((window.pid_confidence - 0.9).abs() < f32::EPSILON);
        assert_eq!(window.title, Some("Title2".to_string()));
    }

    #[test]
    fn get_window_info_by_pid_returns_none_for_empty_windows() {
        let windows = vec![];
        let introspector = StaticWindowIntrospector::new(windows);
        let window = get_window_info_by_pid(&introspector, 100).unwrap();
        assert!(window.is_none());
    }

    #[test]
    fn get_window_info_by_pid_returns_none_when_all_windows_have_no_pid() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Title1".to_string()),
                Some(1),
                WindowState::Focused,
                None,
                0.9,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Title2".to_string()),
                Some(1),
                WindowState::Background,
                None,
                0.8,
            ),
        ];
        let introspector = StaticWindowIntrospector::new(windows);
        let window = get_window_info_by_pid(&introspector, 100).unwrap();
        assert!(window.is_none());
    }

    #[test]
    fn get_window_info_by_pid_handles_mixed_windows_with_and_without_pid() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Title1".to_string()),
                Some(1),
                WindowState::Focused,
                None,
                0.9,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Title2".to_string()),
                Some(1),
                WindowState::Background,
                Some(200),
                0.8,
            ),
        ];
        let introspector = StaticWindowIntrospector::new(windows);
        // Должен найти окно с PID 200
        let window = get_window_info_by_pid(&introspector, 200).unwrap();
        assert!(window.is_some());
        assert_eq!(window.unwrap().pid, Some(200));
        // Не должен найти окно с PID 100 (его нет)
        let window = get_window_info_by_pid(&introspector, 100).unwrap();
        assert!(window.is_none());
    }

    #[test]
    fn build_pid_to_window_map_returns_empty_map_for_empty_windows() {
        let windows = vec![];
        let introspector = StaticWindowIntrospector::new(windows);
        let map = build_pid_to_window_map(&introspector).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn build_pid_to_window_map_returns_empty_map_when_all_windows_have_no_pid() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Title1".to_string()),
                Some(1),
                WindowState::Focused,
                None,
                0.9,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Title2".to_string()),
                Some(1),
                WindowState::Background,
                None,
                0.8,
            ),
        ];
        let introspector = StaticWindowIntrospector::new(windows);
        let map = build_pid_to_window_map(&introspector).unwrap();
        assert!(map.is_empty());
    }

    #[test]
    fn build_pid_to_window_map_handles_mixed_windows_with_and_without_pid() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Title1".to_string()),
                Some(1),
                WindowState::Focused,
                None,
                0.9,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Title2".to_string()),
                Some(1),
                WindowState::Background,
                Some(200),
                0.8,
            ),
            WindowInfo::new(
                Some("app3".to_string()),
                Some("Title3".to_string()),
                Some(1),
                WindowState::Minimized,
                Some(300),
                0.7,
            ),
        ];
        let introspector = StaticWindowIntrospector::new(windows);
        let map = build_pid_to_window_map(&introspector).unwrap();
        // Должны быть только окна с PID
        assert_eq!(map.len(), 2);
        assert!(map.contains_key(&200));
        assert!(map.contains_key(&300));
        assert!(!map.contains_key(&0)); // окно без PID не должно попасть в мап
    }

    // Тест для обработки ошибок introspector
    struct ErrorIntrospector;

    impl WindowIntrospector for ErrorIntrospector {
        fn windows(&self) -> Result<Vec<WindowInfo>> {
            anyhow::bail!("Test error from introspector")
        }
    }

    #[test]
    fn get_window_info_by_pid_propagates_introspector_error() {
        let introspector = ErrorIntrospector;
        let result = get_window_info_by_pid(&introspector, 100);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Test error from introspector"));
    }

    #[test]
    fn build_pid_to_window_map_propagates_introspector_error() {
        let introspector = ErrorIntrospector;
        let result = build_pid_to_window_map(&introspector);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Test error from introspector"));
    }
}
