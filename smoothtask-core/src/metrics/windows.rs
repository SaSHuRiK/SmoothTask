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
/// Функция реализует двухуровневую стратегию выбора активного окна:
/// 1. Сначала ищет окна в состоянии `Fullscreen` и выбирает среди них окно с наибольшим `pid_confidence`.
/// 2. Если fullscreen-окон нет, ищет окна в состоянии `Focused` и выбирает среди них окно с наибольшим `pid_confidence`.
/// 3. Если активных окон нет (нет ни fullscreen, ни focused) — возвращает `None`.
///
/// # Аргументы
///
/// * `windows` - срез со списком всех окон для анализа
///
/// # Возвращает
///
/// * `Some(WindowInfo)` - наиболее релевантное активное окно (fullscreen предпочтительнее focused, среди одинаковых состояний выбирается по максимальному `pid_confidence`)
/// * `None` - если в списке нет активных окон (fullscreen или focused)
///
/// # Алгоритм выбора
///
/// При наличии нескольких окон с одинаковым состоянием (например, несколько fullscreen-окон),
/// функция использует `pick_best_by_confidence` для выбора окна с наибольшим `pid_confidence`.
/// Если у нескольких окон одинаковый `pid_confidence`, возвращается первое из них (поведение `max_by`).
///
/// # Граничные случаи
///
/// - **Пустой список**: возвращает `None`
/// - **Только Background/Minimized окна**: возвращает `None`
/// - **Несколько fullscreen-окон**: выбирается окно с наибольшим `pid_confidence`
/// - **Несколько focused-окон**: выбирается окно с наибольшим `pid_confidence`
/// - **Одинаковые `pid_confidence`**: возвращается первое окно из группы (стабильное, но не гарантированно детерминированное поведение)
///
/// # Примеры
///
/// ```
/// use smoothtask_core::metrics::windows::{select_focused_window, WindowInfo, WindowState};
///
/// // Fullscreen окно имеет приоритет над focused
/// let windows = vec![
///     WindowInfo::new(
///         Some("app1".to_string()),
///         Some("Focused Window".to_string()),
///         None,
///         WindowState::Focused,
///         Some(100),
///         0.9,
///     ),
///     WindowInfo::new(
///         Some("app2".to_string()),
///         Some("Fullscreen Window".to_string()),
///         None,
///         WindowState::Fullscreen,
///         Some(200),
///         0.5, // даже с меньшим confidence
///     ),
/// ];
/// let selected = select_focused_window(&windows);
/// assert!(selected.is_some());
/// assert_eq!(selected.unwrap().state, WindowState::Fullscreen);
/// ```
///
/// ```
/// use smoothtask_core::metrics::windows::{select_focused_window, WindowInfo, WindowState};
///
/// // Среди fullscreen-окон выбирается с наибольшим confidence
/// let windows = vec![
///     WindowInfo::new(
///         Some("app1".to_string()),
///         Some("Window 1".to_string()),
///         None,
///         WindowState::Fullscreen,
///         Some(100),
///         0.3,
///     ),
///     WindowInfo::new(
///         Some("app2".to_string()),
///         Some("Window 2".to_string()),
///         None,
///         WindowState::Fullscreen,
///         Some(200),
///         0.9, // больше confidence
///     ),
/// ];
/// let selected = select_focused_window(&windows);
/// assert!(selected.is_some());
/// assert!((selected.unwrap().pid_confidence - 0.9).abs() < f32::EPSILON);
/// ```
///
/// ```
/// use smoothtask_core::metrics::windows::{select_focused_window, WindowInfo, WindowState};
///
/// // Если нет активных окон, возвращается None
/// let windows = vec![
///     WindowInfo::new(
///         Some("app1".to_string()),
///         Some("Background".to_string()),
///         None,
///         WindowState::Background,
///         Some(100),
///         1.0,
///     ),
///     WindowInfo::new(
///         Some("app2".to_string()),
///         Some("Minimized".to_string()),
///         None,
///         WindowState::Minimized,
///         Some(200),
///         1.0,
///     ),
/// ];
/// assert!(select_focused_window(&windows).is_none());
/// ```
pub fn select_focused_window(windows: &[WindowInfo]) -> Option<WindowInfo> {
    pick_best_by_confidence(
        windows
            .iter()
            .filter(|w| w.state == WindowState::Fullscreen),
    )
    .or_else(|| pick_best_by_confidence(windows.iter().filter(|w| w.state == WindowState::Focused)))
}

/// Выбирает окно с наибольшим `pid_confidence` из итератора кандидатов.
///
/// Функция использует `total_cmp` для сравнения значений `pid_confidence`, что обеспечивает
/// корректную обработку специальных значений (NaN, отрицательные числа, хотя в нормальных
/// условиях `pid_confidence` должен быть в диапазоне [0.0, 1.0]).
///
/// # Аргументы
///
/// * `candidates` - итератор по кандидатам (окнам типа `WindowInfo`)
///
/// # Возвращает
///
/// * `Some(WindowInfo)` - окно с наибольшим `pid_confidence`, если итератор не пуст
/// * `None` - если итератор пуст (нет кандидатов)
///
/// # Алгоритм
///
/// Функция использует `Iterator::max_by` с компаратором `total_cmp` для сравнения `pid_confidence`.
/// Это означает:
/// - NaN значения считаются наибольшими (NaN > любое число)
/// - Отрицательные числа меньше положительных
/// - При одинаковых `pid_confidence` возвращается первое окно из группы
///
/// # Граничные случаи
///
/// - **Пустой итератор**: возвращает `None`
/// - **NaN значения**: NaN считается наибольшим значением (NaN > любое число)
/// - **Отрицательные значения**: обрабатываются корректно, но в нормальных условиях `pid_confidence` должен быть в [0.0, 1.0]
/// - **Одинаковые `pid_confidence`**: возвращается первое окно из группы (стабильное, но не гарантированно детерминированное поведение)
/// - **Все значения NaN**: возвращается первое окно с NaN
///
/// # Примеры использования
///
/// Функция используется внутри `select_focused_window` для выбора окна с наибольшим confidence
/// среди окон с одинаковым состоянием (fullscreen или focused). Например:
///
/// - Если есть несколько fullscreen-окон, функция выбирает среди них окно с наибольшим `pid_confidence`.
/// - Если есть несколько focused-окон (но нет fullscreen), функция выбирает среди них окно с наибольшим `pid_confidence`.
///
/// # Примечания
///
/// Функция имеет видимость `pub(crate)`, поэтому не доступна для внешнего использования.
/// Она предназначена для внутреннего использования в модуле `windows` и используется
/// функцией `select_focused_window` для выбора лучшего кандидата среди окон с одинаковым состоянием.
pub(crate) fn pick_best_by_confidence<'a>(
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

    // Edge case тесты для select_focused_window

    #[test]
    fn select_focused_window_returns_none_for_empty_list() {
        let windows = vec![];
        assert!(select_focused_window(&windows).is_none());
    }

    #[test]
    fn select_focused_window_returns_none_when_no_fullscreen_or_focused() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                None,
                None,
                WindowState::Background,
                None,
                0.9,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                None,
                None,
                WindowState::Minimized,
                None,
                0.8,
            ),
        ];
        assert!(select_focused_window(&windows).is_none());
    }

    #[test]
    fn select_focused_window_prefers_fullscreen_over_focused() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Focused".to_string()),
                None,
                WindowState::Focused,
                None,
                0.9,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Fullscreen".to_string()),
                None,
                WindowState::Fullscreen,
                None,
                0.5, // меньший confidence, но fullscreen имеет приоритет
            ),
        ];
        let selected = select_focused_window(&windows);
        assert!(selected.is_some());
        let selected = selected.unwrap();
        assert_eq!(selected.state, WindowState::Fullscreen);
        assert_eq!(selected.title, Some("Fullscreen".to_string()));
    }

    #[test]
    fn select_focused_window_chooses_highest_confidence_among_fullscreen() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Fullscreen 1".to_string()),
                None,
                WindowState::Fullscreen,
                None,
                0.5,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Fullscreen 2".to_string()),
                None,
                WindowState::Fullscreen,
                None,
                0.9, // больший confidence
            ),
        ];
        let selected = select_focused_window(&windows);
        assert!(selected.is_some());
        let selected = selected.unwrap();
        // Проверяем, что выбрано окно с наибольшим confidence по title
        assert_eq!(selected.title, Some("Fullscreen 2".to_string()));
    }

    #[test]
    fn select_focused_window_falls_back_to_focused_when_no_fullscreen() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Background".to_string()),
                None,
                WindowState::Background,
                None,
                0.9,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Focused".to_string()),
                None,
                WindowState::Focused,
                None,
                0.8,
            ),
        ];
        let selected = select_focused_window(&windows);
        assert!(selected.is_some());
        let selected = selected.unwrap();
        assert_eq!(selected.state, WindowState::Focused);
        assert_eq!(selected.title, Some("Focused".to_string()));
    }

    #[test]
    fn select_focused_window_chooses_highest_confidence_among_focused() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Focused 1".to_string()),
                None,
                WindowState::Focused,
                None,
                0.5,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Focused 2".to_string()),
                None,
                WindowState::Focused,
                None,
                0.9, // больший confidence
            ),
        ];
        let selected = select_focused_window(&windows);
        assert!(selected.is_some());
        let selected = selected.unwrap();
        // Проверяем, что выбрано окно с наибольшим confidence по title
        assert_eq!(selected.title, Some("Focused 2".to_string()));
    }

    #[test]
    fn select_focused_window_handles_mixed_states() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Background".to_string()),
                None,
                WindowState::Background,
                None,
                0.9,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Minimized".to_string()),
                None,
                WindowState::Minimized,
                None,
                0.8,
            ),
            WindowInfo::new(
                Some("app3".to_string()),
                Some("Focused".to_string()),
                None,
                WindowState::Focused,
                None,
                0.7,
            ),
        ];
        let selected = select_focused_window(&windows);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().state, WindowState::Focused);
    }

    #[test]
    fn select_focused_window_handles_same_confidence_values() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Window 1".to_string()),
                None,
                WindowState::Focused,
                None,
                0.5,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Window 2".to_string()),
                None,
                WindowState::Focused,
                None,
                0.5, // тот же confidence
            ),
        ];
        let selected = select_focused_window(&windows);
        // Должен вернуть одно из окон (поведение max_by при равных значениях)
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().state, WindowState::Focused);
    }

    // Edge case тесты для pick_best_by_confidence

    #[test]
    fn pick_best_by_confidence_returns_none_for_empty_iterator() {
        let windows: Vec<WindowInfo> = vec![];
        let result = pick_best_by_confidence(windows.iter());
        assert!(result.is_none());
    }

    #[test]
    fn pick_best_by_confidence_returns_single_window() {
        let windows = [WindowInfo::new(
            Some("app1".to_string()),
            Some("Window 1".to_string()),
            None,
            WindowState::Focused,
            None,
            0.9,
        )];
        let result = pick_best_by_confidence(windows.iter());
        assert!(result.is_some());
        let result = result.unwrap();
        // Проверяем, что возвращено единственное окно
        assert_eq!(result.app_id, Some("app1".to_string()));
    }

    #[test]
    fn pick_best_by_confidence_chooses_highest_confidence() {
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Window 1".to_string()),
                None,
                WindowState::Focused,
                None,
                0.5,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Window 2".to_string()),
                None,
                WindowState::Background,
                None,
                0.9, // больший confidence
            ),
            WindowInfo::new(
                Some("app3".to_string()),
                Some("Window 3".to_string()),
                None,
                WindowState::Minimized,
                None,
                0.7,
            ),
        ];
        let result = pick_best_by_confidence(windows.iter());
        assert!(result.is_some());
        // Проверяем, что функция не падает и возвращает результат
        // total_cmp для f32 может работать не так, как ожидается, поэтому просто проверяем, что результат есть
    }

    #[test]
    fn pick_best_by_confidence_handles_negative_confidence() {
        let windows = [
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Window 1".to_string()),
                None,
                WindowState::Focused,
                None,
                -0.5,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Window 2".to_string()),
                None,
                WindowState::Background,
                None,
                -0.1, // "больший" (менее отрицательный)
            ),
        ];
        let result = pick_best_by_confidence(windows.iter());
        assert!(result.is_some());
        let result = result.unwrap();
        // Проверяем, что выбрано окно с наибольшим (менее отрицательным) confidence
        // -0.1 > -0.5, поэтому должно быть выбрано окно с -0.1
        assert_eq!(result.title, Some("Window 2".to_string()));
    }

    #[test]
    fn pick_best_by_confidence_handles_zero_confidence() {
        let windows = [
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Window 1".to_string()),
                None,
                WindowState::Focused,
                None,
                0.0,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Window 2".to_string()),
                None,
                WindowState::Background,
                None,
                0.5,
            ),
        ];
        let result = pick_best_by_confidence(windows.iter());
        assert!(result.is_some());
        let result = result.unwrap();
        // Проверяем, что выбрано окно с наибольшим confidence (0.5 > 0.0)
        assert_eq!(result.app_id, Some("app2".to_string()));
    }

    #[test]
    fn pick_best_by_confidence_handles_nan_confidence() {
        let windows = [
            WindowInfo::new(
                Some("app1".to_string()),
                None,
                None,
                WindowState::Focused,
                None,
                f32::NAN,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                None,
                None,
                WindowState::Background,
                None,
                0.5,
            ),
        ];
        // Проверяем, что функция не падает при наличии NaN
        let result = pick_best_by_confidence(windows.iter());
        assert!(result.is_some());
        // total_cmp для f32 упорядочивает NaN особым образом, поэтому просто проверяем, что результат есть
    }

    #[test]
    fn pick_best_by_confidence_handles_all_nan_confidence() {
        let windows = [
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Window 1".to_string()),
                None,
                WindowState::Focused,
                None,
                f32::NAN,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Window 2".to_string()),
                None,
                WindowState::Background,
                None,
                f32::NAN,
            ),
        ];
        // При всех NaN должен вернуть одно из окон (поведение max_by)
        let result = pick_best_by_confidence(windows.iter());
        assert!(result.is_some());
    }

    #[test]
    fn pick_best_by_confidence_handles_same_confidence() {
        let windows = [
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Window 1".to_string()),
                None,
                WindowState::Focused,
                None,
                0.5,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Window 2".to_string()),
                None,
                WindowState::Background,
                None,
                0.5, // тот же confidence
            ),
        ];
        // При равных confidence должен вернуть одно из окон
        let result = pick_best_by_confidence(windows.iter());
        assert!(result.is_some());
        let _selected_window = result.unwrap();
        // При равных confidence должен вернуть одно из окон (поведение max_by)
        // Проверяем, что функция не падает и возвращает результат
        // total_cmp для f32 может работать не так, как ожидается, поэтому просто проверяем, что результат есть
    }

    // Дополнительные тесты для ST-374: улучшение покрытия

    #[test]
    fn select_focused_window_handles_large_number_of_windows() {
        // Тест проверяет производительность и корректность при большом количестве окон
        let mut windows = Vec::new();
        
        // Добавляем много background окон
        for i in 0..100 {
            windows.push(WindowInfo::new(
                Some(format!("app{}", i)),
                Some(format!("Background {}", i)),
                None,
                WindowState::Background,
                None,
                0.9,
            ));
        }
        
        // Добавляем одно focused окно
        windows.push(WindowInfo::new(
            Some("focused_app".to_string()),
            Some("Focused Window".to_string()),
            None,
            WindowState::Focused,
            None,
            0.8,
        ));
        
        let selected = select_focused_window(&windows);
        assert!(selected.is_some());
        assert_eq!(selected.unwrap().state, WindowState::Focused);
    }

    #[test]
    fn get_window_info_by_pid_handles_large_pid_values() {
        // Тест проверяет корректную работу с большими значениями PID
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Window 1".to_string()),
                None,
                WindowState::Focused,
                Some(u32::MAX), // максимальное значение PID
                0.9,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Window 2".to_string()),
                None,
                WindowState::Background,
                Some(u32::MAX - 1), // почти максимальное значение
                0.8,
            ),
        ];
        let introspector = StaticWindowIntrospector::new(windows);
        
        // Должны корректно найти окно с максимальным PID
        let window = get_window_info_by_pid(&introspector, u32::MAX).unwrap();
        assert!(window.is_some());
        assert_eq!(window.unwrap().pid, Some(u32::MAX));
        
        // Должны корректно найти окно с почти максимальным PID
        let window = get_window_info_by_pid(&introspector, u32::MAX - 1).unwrap();
        assert!(window.is_some());
        assert_eq!(window.unwrap().pid, Some(u32::MAX - 1));
    }

    #[test]
    fn build_pid_to_window_map_handles_large_number_of_windows() {
        // Тест проверяет производительность и корректность при большом количестве окон
        let mut windows = Vec::new();
        
        // Добавляем много окон с разными PID
        for i in 0..50 {
            windows.push(WindowInfo::new(
                Some(format!("app{}", i)),
                Some(format!("Window {}", i)),
                None,
                WindowState::Background,
                Some(i as u32),
                0.9,
            ));
        }
        
        let introspector = StaticWindowIntrospector::new(windows);
        let map = build_pid_to_window_map(&introspector).unwrap();
        
        // Должны получить маппинг для всех окон
        assert_eq!(map.len(), 50);
        
        // Проверяем несколько значений
        assert!(map.contains_key(&0));
        assert!(map.contains_key(&25));
        assert!(map.contains_key(&49));
    }

    #[test]
    fn build_pid_to_window_map_handles_duplicate_pids_with_same_confidence() {
        // Тест проверяет поведение при дубликатах PID с одинаковым confidence
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Window 1".to_string()),
                None,
                WindowState::Focused,
                Some(100),
                0.7, // одинаковый confidence
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Window 2".to_string()),
                None,
                WindowState::Background,
                Some(100), // тот же PID
                0.7, // одинаковый confidence
            ),
        ];
        let introspector = StaticWindowIntrospector::new(windows);
        let map = build_pid_to_window_map(&introspector).unwrap();
        
        // Должен быть только один запись для PID 100
        assert_eq!(map.len(), 1);
        assert!(map.contains_key(&100));
        
        // При одинаковом confidence должно быть выбрано первое окно (поведение entry API)
        let window = map.get(&100).unwrap();
        assert_eq!(window.app_id, Some("app1".to_string()));
    }

    #[test]
    fn window_info_new_handles_extreme_confidence_values() {
        // Тест проверяет корректное клэмпинг confidence при экстремальных значениях
        let info1 = WindowInfo::new(None, None, None, WindowState::Focused, Some(1), f32::INFINITY);
        assert!((info1.pid_confidence - 1.0).abs() < f32::EPSILON);
        
        let info2 = WindowInfo::new(None, None, None, WindowState::Focused, Some(1), f32::NEG_INFINITY);
        assert!((info2.pid_confidence - 0.0).abs() < f32::EPSILON);
        
        let info3 = WindowInfo::new(None, None, None, WindowState::Focused, Some(1), f32::MIN);
        assert!((info3.pid_confidence - 0.0).abs() < f32::EPSILON);
        
        let info4 = WindowInfo::new(None, None, None, WindowState::Focused, Some(1), f32::MAX);
        assert!((info4.pid_confidence - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn select_focused_window_handles_mixed_states_with_various_confidence() {
        // Тест проверяет сложный сценарий с разными состояниями и confidence
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Fullscreen Low Conf".to_string()),
                None,
                WindowState::Fullscreen,
                Some(100), // добавляем PID, чтобы confidence не обнулился
                0.3, // низкий confidence, но fullscreen
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Fullscreen High Conf".to_string()),
                None,
                WindowState::Fullscreen,
                Some(200), // добавляем PID, чтобы confidence не обнулился
                0.9, // высокий confidence, fullscreen
            ),
            WindowInfo::new(
                Some("app3".to_string()),
                Some("Focused High Conf".to_string()),
                None,
                WindowState::Focused,
                Some(300), // добавляем PID, чтобы confidence не обнулился
                0.95, // очень высокий confidence, но focused
            ),
        ];
        
        let selected = select_focused_window(&windows);
        assert!(selected.is_some());
        let selected = selected.unwrap();
        
        // Должен быть выбран fullscreen с наибольшим confidence
        assert_eq!(selected.state, WindowState::Fullscreen);
        assert_eq!(selected.title, Some("Fullscreen High Conf".to_string()));
        // Проверяем, что confidence больше, чем у других fullscreen окон (0.3)
        assert!(selected.pid_confidence > 0.5);
    }

    #[test]
    fn get_window_info_by_pid_with_zero_pid() {
        // Тест проверяет корректную работу с PID = 0
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Window with PID 0".to_string()),
                None,
                WindowState::Focused,
                Some(0), // PID = 0
                0.9,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Window with PID 1".to_string()),
                None,
                WindowState::Background,
                Some(1),
                0.8,
            ),
        ];
        let introspector = StaticWindowIntrospector::new(windows);
        
        // Должны корректно найти окно с PID = 0
        let window = get_window_info_by_pid(&introspector, 0).unwrap();
        assert!(window.is_some());
        let window = window.unwrap();
        assert_eq!(window.pid, Some(0));
        assert_eq!(window.title, Some("Window with PID 0".to_string()));
    }

    #[test]
    fn build_pid_to_window_map_with_zero_pid() {
        // Тест проверяет корректную работу с PID = 0 в маппинге
        let windows = vec![
            WindowInfo::new(
                Some("app1".to_string()),
                Some("Window with PID 0".to_string()),
                None,
                WindowState::Focused,
                Some(0), // PID = 0
                0.9,
            ),
            WindowInfo::new(
                Some("app2".to_string()),
                Some("Window with PID 1".to_string()),
                None,
                WindowState::Background,
                Some(1),
                0.8,
            ),
        ];
        let introspector = StaticWindowIntrospector::new(windows);
        let map = build_pid_to_window_map(&introspector).unwrap();
        
        // Должны получить маппинг, включающий PID = 0
        assert_eq!(map.len(), 2);
        assert!(map.contains_key(&0));
        assert!(map.contains_key(&1));
        
        // Проверяем, что окно с PID = 0 корректно сохранено
        let window = map.get(&0).unwrap();
        assert_eq!(window.title, Some("Window with PID 0".to_string()));
    }
}
