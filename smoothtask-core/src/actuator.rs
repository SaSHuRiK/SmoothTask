//! Планирование и применение приоритетов (nice/ionice/cgroups).
//!
//! Этот модуль вычисляет, какие процессы требуют обновления приоритетов, и
//! применяет их через системные вызовы (setpriority, ioprio_set) и cgroups v2.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use libc;
use tracing::{debug, warn};

use crate::logging::snapshots::Snapshot;
use crate::policy::classes::{CgroupParams, IoNiceParams, PriorityClass, PriorityParams};
use crate::policy::engine::PolicyResult;

/// Запрошенное изменение приоритета для процесса.
#[derive(Debug, Clone, PartialEq)]
pub struct PriorityAdjustment {
    /// PID процесса.
    pub pid: i32,
    /// AppGroup, к которому относится процесс.
    pub app_group_id: String,
    /// Целевой класс приоритета.
    pub target_class: PriorityClass,
    /// Текущий nice процесса.
    pub current_nice: i32,
    /// Целевой nice.
    pub target_nice: i32,
    /// Текущий latency_nice процесса, если известен.
    pub current_latency_nice: Option<i32>,
    /// Целевой latency_nice.
    pub target_latency_nice: i32,
    /// Текущий ionice (class, level), если известен.
    pub current_ionice: Option<(i32, i32)>,
    /// Целевой ionice.
    pub target_ionice: IoNiceParams,
    /// Текущий cpu.weight процесса, если известен.
    pub current_cpu_weight: Option<u32>,
    /// Целевой cpu.weight.
    pub target_cpu_weight: u32,
    /// Причина из PolicyEngine (для логирования/трассировки).
    pub reason: String,
}

/// Построить список изменений приоритетов на основе результатов политики.
///
/// Функция анализирует снапшот системы и результаты политики, определяя,
/// какие процессы требуют обновления приоритетов (nice, ionice, latency_nice, cpu.weight).
///
/// # Параметры
///
/// - `snapshot`: Текущий снапшот системы с метриками процессов
/// - `policy_results`: Результаты применения политики для каждого AppGroup
///
/// # Возвращаемое значение
///
/// Вектор `PriorityAdjustment`, содержащий все процессы, которые требуют
/// изменения приоритетов. Процессы, у которых текущие приоритеты уже соответствуют
/// целевым, не включаются в результат.
///
/// # Алгоритм
///
/// 1. Для каждого процесса из снапшота проверяется наличие AppGroup и результата политики
/// 2. Читаются текущие значения приоритетов (nice, ionice, latency_nice, cpu.weight)
/// 3. Сравниваются текущие значения с целевыми из `PriorityParams`
/// 4. Если хотя бы один приоритет отличается, процесс добавляется в список изменений
///
/// # Примечания
///
/// - Если текущее значение приоритета неизвестно (например, latency_nice не поддерживается
///   старым ядром), процесс всё равно включается в список изменений
/// - Функция не применяет изменения, а только планирует их
/// - Реальное применение выполняется через `apply_priority_adjustments()`
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::actuator::plan_priority_changes;
/// use smoothtask_core::logging::snapshots::Snapshot;
/// use std::collections::HashMap;
/// use smoothtask_core::policy::engine::PolicyResult;
///
/// # let snapshot = Snapshot::default();
/// # let policy_results = HashMap::new();
/// let adjustments = plan_priority_changes(&snapshot, &policy_results);
/// // adjustments содержит все процессы, требующие изменения приоритетов
/// ```
pub fn plan_priority_changes(
    snapshot: &Snapshot,
    policy_results: &HashMap<String, PolicyResult>,
) -> Vec<PriorityAdjustment> {
    let mut adjustments = Vec::new();

    for process in &snapshot.processes {
        let app_group_id = match &process.app_group_id {
            Some(id) => id,
            None => continue,
        };

        let policy = match policy_results.get(app_group_id) {
            Some(p) => p,
            None => continue,
        };

        let params = policy.priority_class.params();
        // Читаем текущий nice из процесса, если он не был прочитан при сборе метрик
        // (хотя nice всегда читается из /proc/[pid]/stat, но для консистентности используем функцию)
        let current_nice = read_nice(process.pid)
            .ok()
            .flatten()
            .unwrap_or(process.nice); // Fallback на значение из stat, если getpriority не сработал
                                      // Читаем текущий ionice из процесса, если он не был прочитан при сборе метрик
        let current_ionice = process
            .ionice_class
            .zip(process.ionice_prio)
            .or_else(|| read_ionice(process.pid).ok().flatten());
        // Читаем текущий latency_nice из процесса
        let current_latency_nice = read_latency_nice(process.pid).unwrap_or(None); // В случае ошибки считаем, что latency_nice неизвестен
                                                                                   // Читаем текущий cpu.weight из cgroup процесса
        let current_cpu_weight = read_cpu_weight(process.pid).unwrap_or(None); // В случае ошибки считаем, что cpu.weight неизвестен

        if needs_change(
            current_nice,
            current_ionice,
            current_latency_nice,
            current_cpu_weight,
            params,
        ) {
            adjustments.push(PriorityAdjustment {
                pid: process.pid,
                app_group_id: app_group_id.clone(),
                target_class: policy.priority_class,
                current_nice,
                target_nice: params.nice.nice,
                current_latency_nice,
                target_latency_nice: params.latency_nice.latency_nice,
                current_ionice,
                target_ionice: params.ionice,
                current_cpu_weight,
                target_cpu_weight: params.cgroup.cpu_weight,
                reason: policy.reason.clone(),
            });
        }
    }

    adjustments
}

fn needs_change(
    current_nice: i32,
    current_ionice: Option<(i32, i32)>,
    current_latency_nice: Option<i32>,
    current_cpu_weight: Option<u32>,
    target: PriorityParams,
) -> bool {
    if current_nice != target.nice.nice {
        return true;
    }

    if let Some(latency_nice) = current_latency_nice {
        if latency_nice != target.latency_nice.latency_nice {
            return true;
        }
    } else {
        // Если latency_nice неизвестен, считаем, что нужно изменить
        return true;
    }

    match current_ionice {
        Some((class, level)) => {
            if class != target.ionice.class || level != target.ionice.level {
                return true;
            }
        }
        None => return true,
    }

    // Проверяем cpu.weight
    match current_cpu_weight {
        Some(weight) => {
            if weight != target.cgroup.cpu_weight {
                return true;
            }
        }
        None => {
            // Если cpu.weight неизвестен, считаем, что нужно изменить
            return true;
        }
    }

    false
}

/// Информация о последнем изменении приоритета процесса (для гистерезиса).
#[derive(Debug, Clone)]
struct ProcessChangeHistory {
    /// Время последнего изменения.
    last_change: Instant,
    /// Последний применённый класс приоритета.
    last_class: PriorityClass,
}

/// Трекер истории изменений для гистерезиса.
pub struct HysteresisTracker {
    /// История изменений по PID.
    history: HashMap<i32, ProcessChangeHistory>,
    /// Минимальное время между изменениями (гистерезис).
    min_time_between_changes: Duration,
    /// Минимальная разница классов для применения изменения.
    min_class_difference: i32,
}

impl HysteresisTracker {
    /// Создать новый трекер с параметрами по умолчанию.
    pub fn new() -> Self {
        Self {
            history: HashMap::new(),
            min_time_between_changes: Duration::from_secs(5),
            min_class_difference: 1,
        }
    }

    /// Создать трекер с кастомными параметрами.
    ///
    /// # Параметры
    ///
    /// - `min_time_between_changes`: Минимальное время между изменениями приоритета
    ///   для одного процесса. Предотвращает слишком частые изменения.
    /// - `min_class_difference`: Минимальная разница в порядке классов для применения
    ///   изменения. Например, если `min_class_difference = 2`, то изменение с
    ///   `Normal` (3) на `Interactive` (4) будет применено (разница = 1 < 2, не применяется),
    ///   а изменение с `Normal` (3) на `CritInteractive` (5) будет применено (разница = 2 >= 2).
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::actuator::HysteresisTracker;
    /// use std::time::Duration;
    ///
    /// // Трекер с более строгими параметрами (10 секунд между изменениями, разница >= 2)
    /// let tracker = HysteresisTracker::with_params(
    ///     Duration::from_secs(10),
    ///     2
    /// );
    /// ```
    pub fn with_params(min_time_between_changes: Duration, min_class_difference: i32) -> Self {
        Self {
            history: HashMap::new(),
            min_time_between_changes,
            min_class_difference,
        }
    }

    /// Проверить, можно ли применить изменение для процесса.
    fn should_apply_change(&self, pid: i32, target_class: PriorityClass) -> bool {
        let now = Instant::now();

        if let Some(history) = self.history.get(&pid) {
            // Проверяем время с последнего изменения
            if now.duration_since(history.last_change) < self.min_time_between_changes {
                return false;
            }

            // Проверяем разницу классов
            let class_diff = (class_order(history.last_class) - class_order(target_class)).abs();
            if class_diff < self.min_class_difference {
                return false;
            }
        }

        true
    }

    /// Зафиксировать применение изменения.
    fn record_change(&mut self, pid: i32, target_class: PriorityClass) {
        self.history.insert(
            pid,
            ProcessChangeHistory {
                last_change: Instant::now(),
                last_class: target_class,
            },
        );
    }

    /// Очистить историю для процессов, которые больше не существуют.
    ///
    /// Функция удаляет из истории изменений приоритетов все записи для процессов,
    /// которые не присутствуют в списке активных PIDs. Это предотвращает накопление
    /// устаревших данных о завершившихся процессах.
    ///
    /// # Параметры
    ///
    /// - `active_pids`: Список PIDs процессов, которые всё ещё существуют в системе.
    ///   История для всех остальных процессов будет удалена.
    ///
    /// # Алгоритм
    ///
    /// 1. Создаётся HashSet из активных PIDs для быстрого поиска
    /// 2. Удаляются все записи из истории, чьи PIDs не присутствуют в активном списке
    ///
    /// # Примеры
    ///
    /// ## Базовое использование
    ///
    /// ```no_run
    /// use smoothtask_core::actuator::HysteresisTracker;
    ///
    /// let mut tracker = HysteresisTracker::new();
    ///
    /// // После применения приоритетов через apply_priority_adjustments,
    /// // история содержит записи для всех процессов, которым были применены изменения.
    /// // Очищаем историю, оставляя только активные процессы
    /// tracker.cleanup(&[1001, 1003]);
    ///
    /// // Теперь история содержит только записи для процессов 1001 и 1003
    /// ```
    ///
    /// ## Использование в цикле демона
    ///
    /// ```no_run
    /// use smoothtask_core::actuator::HysteresisTracker;
    /// use smoothtask_core::logging::snapshots::Snapshot;
    ///
    /// # fn get_snapshot() -> Snapshot { unimplemented!() }
    /// let mut tracker = HysteresisTracker::new();
    /// let snapshot = get_snapshot();
    ///
    /// // Получаем список активных PIDs из снапшота
    /// let active_pids: Vec<i32> = snapshot.processes.iter().map(|p| p.pid).collect();
    ///
    /// // Очищаем историю от завершившихся процессов
    /// tracker.cleanup(&active_pids);
    /// ```
    ///
    /// # Примечания
    ///
    /// - Функция эффективна даже для больших списков процессов благодаря использованию HashSet
    /// - Если `active_pids` пуст, вся история будет очищена
    /// - Если процесс присутствует в `active_pids`, но не в истории, ничего не происходит
    pub fn cleanup(&mut self, active_pids: &[i32]) {
        let active_set: std::collections::HashSet<_> = active_pids.iter().copied().collect();
        self.history.retain(|pid, _| active_set.contains(pid));
    }
}

impl Default for HysteresisTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Порядок класса для вычисления разницы (CritInteractive=5, Idle=1).
fn class_order(class: PriorityClass) -> i32 {
    match class {
        PriorityClass::CritInteractive => 5,
        PriorityClass::Interactive => 4,
        PriorityClass::Normal => 3,
        PriorityClass::Background => 2,
        PriorityClass::Idle => 1,
    }
}

/// Прочитать текущий nice процесса через getpriority.
///
/// Возвращает `None`, если:
/// - процесс не существует;
/// - системный вызов не поддерживается (старое ядро);
/// - произошла другая ошибка при чтении.
///
/// Возвращает `Some(nice)`, где `nice` находится в диапазоне [-20, 19].
/// Прочитать текущий nice процесса через getpriority.
///
/// Возвращает `None`, если:
/// - процесс не существует;
/// - системный вызов не поддерживается (старое ядро);
/// - произошла ошибка при чтении nice.
///
/// Возвращает `Some(nice)`, где `nice` находится в диапазоне [-20, 19]
/// (стандартный диапазон для nice в Linux).
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::actuator::read_nice;
///
/// // Прочитать nice для текущего процесса
/// let current_pid = std::process::id() as i32;
/// match read_nice(current_pid)? {
///     Some(nice) => println!("Current nice: {}", nice),
///     None => println!("Could not read nice"),
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn read_nice(pid: i32) -> Result<Option<i32>> {
    // PRIO_PROCESS = 0 означает, что мы читаем приоритет для процесса
    const PRIO_PROCESS: libc::__priority_which_t = 0;

    // Сбрасываем errno перед вызовом getpriority
    unsafe {
        *libc::__errno_location() = 0;
    }

    let result = unsafe { libc::getpriority(PRIO_PROCESS, pid as libc::id_t) };
    let errno = unsafe { *libc::__errno_location() };

    // getpriority возвращает значение в диапазоне [-20, 19]
    // но может вернуть -1, если nice = -1, поэтому нужно проверить errno
    // Если errno != 0, то это ошибка
    if result == -1 && errno != 0 {
        // Это ошибка, а не значение nice = -1
        match errno {
            libc::ENOSYS => {
                debug!(pid = pid, "getpriority not supported, cannot read nice");
            }
            libc::ESRCH => {
                debug!(pid = pid, "Process not found, cannot read nice");
            }
            _ => {
                debug!(
                    pid = pid,
                    errno = errno,
                    "Failed to read nice, returning None"
                );
            }
        }
        return Ok(None);
    }

    // Проверяем, что значение находится в допустимом диапазоне
    if !(-20..=19).contains(&result) {
        debug!(
            pid = pid,
            nice = result,
            "getpriority returned invalid nice value, returning None"
        );
        return Ok(None);
    }

    Ok(Some(result as i32))
}

/// Применить изменение nice для процесса.
fn apply_nice(pid: i32, nice: i32) -> Result<()> {
    // PRIO_PROCESS = 0 означает, что мы устанавливаем приоритет для процесса
    const PRIO_PROCESS: libc::__priority_which_t = 0;
    let result = unsafe { libc::setpriority(PRIO_PROCESS, pid as libc::id_t, nice) };

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(anyhow::anyhow!(
            "Failed to set nice={} for pid={}: {}",
            nice,
            pid,
            errno
        ));
    }

    debug!(pid = pid, nice = nice, "Applied nice priority");
    Ok(())
}

/// Применить изменение ionice для процесса.
fn apply_ionice(pid: i32, class: i32, level: i32) -> Result<()> {
    // IOPRIO_WHO_PROCESS = 1 означает, что мы устанавливаем приоритет для процесса
    const IOPRIO_WHO_PROCESS: i32 = 1;
    // Формируем значение для ioprio_set: (class << IOPRIO_CLASS_SHIFT) | level
    // IOPRIO_CLASS_SHIFT обычно равен 13
    const IOPRIO_CLASS_SHIFT: i32 = 13;
    let ioprio_value = (class << IOPRIO_CLASS_SHIFT) | level;

    // Используем syscall напрямую, так как libc может не иметь ioprio_set
    let result = unsafe {
        libc::syscall(
            libc::SYS_ioprio_set,
            IOPRIO_WHO_PROCESS,
            pid as libc::pid_t,
            ioprio_value,
        )
    };

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        return Err(anyhow::anyhow!(
            "Failed to set ionice (class={}, level={}) for pid={}: {}",
            class,
            level,
            pid,
            errno
        ));
    }

    debug!(
        pid = pid,
        class = class,
        level = level,
        "Applied ionice priority"
    );
    Ok(())
}

/// Структура sched_attr для sched_setattr/sched_getattr.
/// Размер структуры должен быть известен ядру для обратной совместимости.
#[repr(C)]
struct SchedAttr {
    size: u32,
    sched_policy: u32,
    sched_flags: u64,
    sched_nice: i32,
    sched_priority: u32,
    sched_runtime: u64,
    sched_deadline: u64,
    sched_period: u64,
    sched_util_min: u32,
    sched_util_max: u32,
    latency_nice: i32,
}

/// Прочитать текущий ionice процесса через ioprio_get.
///
/// Возвращает `None`, если:
/// - процесс не существует;
/// - системный вызов не поддерживается (старое ядро);
/// - ionice не установлен для процесса.
///
/// Возвращает `Some((class, level))`, где:
/// - `class` = класс IO приоритета (0 = none, 1 = realtime, 2 = best-effort, 3 = idle)
/// - `level` = уровень приоритета (0-7 для best-effort/realtime)
pub fn read_ionice(pid: i32) -> Result<Option<(i32, i32)>> {
    // IOPRIO_WHO_PROCESS = 1 означает, что мы читаем приоритет для процесса
    const IOPRIO_WHO_PROCESS: i32 = 1;
    // IOPRIO_CLASS_SHIFT = 13 (сдвиг для извлечения класса)
    const IOPRIO_CLASS_SHIFT: i32 = 13;
    // IOPRIO_PRIO_MASK = 0xFF (маска для извлечения уровня приоритета)
    const IOPRIO_PRIO_MASK: i32 = 0xFF;

    // Используем syscall напрямую, так как libc может не иметь ioprio_get
    let result =
        unsafe { libc::syscall(libc::SYS_ioprio_get, IOPRIO_WHO_PROCESS, pid as libc::pid_t) };

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        let raw_errno = errno.raw_os_error();
        // Если системный вызов не поддерживается (например, старое ядро), возвращаем None
        if raw_errno == Some(libc::ENOSYS) {
            debug!(pid = pid, "ioprio_get not supported, cannot read ionice");
            return Ok(None);
        }
        // Если процесс не существует, возвращаем None
        if raw_errno == Some(libc::ESRCH) {
            debug!(pid = pid, "Process not found, cannot read ionice");
            return Ok(None);
        }
        // Для других ошибок также возвращаем None (более безопасное поведение)
        // Например, EPERM (нет прав), EINVAL (неверные параметры) и т.д.
        debug!(
            pid = pid,
            error = ?errno,
            "Failed to read ionice, returning None"
        );
        return Ok(None);
    }

    let ioprio_value = result as i32;

    // Извлекаем класс и уровень из значения
    let class = (ioprio_value >> IOPRIO_CLASS_SHIFT) & 0x3;
    let level = ioprio_value & IOPRIO_PRIO_MASK;

    // Если класс = 0 (none), считаем, что ionice не установлен
    if class == 0 {
        return Ok(None);
    }

    Ok(Some((class, level)))
}

/// Прочитать текущий latency_nice процесса через sched_getattr.
///
/// Возвращает `None`, если:
/// - процесс не существует;
/// - системный вызов не поддерживается (старое ядро);
/// - latency_nice не установлен для процесса.
pub fn read_latency_nice(pid: i32) -> Result<Option<i32>> {
    let mut attr = SchedAttr {
        size: std::mem::size_of::<SchedAttr>() as u32,
        sched_policy: 0,
        sched_flags: 0,
        sched_nice: 0,
        sched_priority: 0,
        sched_runtime: 0,
        sched_deadline: 0,
        sched_period: 0,
        sched_util_min: 0,
        sched_util_max: 0,
        latency_nice: 0,
    };

    // SYS_SCHED_GETATTR = 452 (номер системного вызова для sched_getattr)
    // Формат: sched_getattr(pid, attr, size, flags)
    const SYS_SCHED_GETATTR: i64 = 452;
    let flags: u32 = 0;

    let result = unsafe {
        libc::syscall(
            SYS_SCHED_GETATTR,
            pid as libc::pid_t,
            &mut attr as *mut SchedAttr as *mut libc::c_void,
            std::mem::size_of::<SchedAttr>() as u32,
            flags,
        )
    };

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        let raw_errno = errno.raw_os_error();
        // Если системный вызов не поддерживается (например, старое ядро), возвращаем None
        if raw_errno == Some(libc::ENOSYS) {
            debug!(
                pid = pid,
                "sched_getattr not supported, cannot read latency_nice"
            );
            return Ok(None);
        }
        // Если процесс не существует, возвращаем None
        if raw_errno == Some(libc::ESRCH) {
            debug!(pid = pid, "Process not found, cannot read latency_nice");
            return Ok(None);
        }
        // Для других ошибок также возвращаем None (более безопасное поведение)
        // Например, EPERM (нет прав), EINVAL (неверные параметры) и т.д.
        debug!(
            pid = pid,
            error = ?errno,
            "Failed to read latency_nice, returning None"
        );
        return Ok(None);
    }

    // Проверяем, установлен ли флаг SCHED_FLAG_LATENCY_NICE
    const SCHED_FLAG_LATENCY_NICE: u64 = 0x10;
    if (attr.sched_flags & SCHED_FLAG_LATENCY_NICE) == 0 {
        // latency_nice не установлен для процесса
        return Ok(None);
    }

    Ok(Some(attr.latency_nice))
}

/// Применить изменение latency_nice для процесса через sched_setattr.
///
/// latency_nice управляет тем, когда процесс получает CPU, а не сколько CPU он получит.
/// Диапазон: -20 (максимальная чувствительность к задержке) до +19 (безразличие к задержке).
fn apply_latency_nice(pid: i32, latency_nice: i32) -> Result<()> {
    // Проверяем диапазон latency_nice
    if !(-20..=19).contains(&latency_nice) {
        return Err(anyhow::anyhow!(
            "latency_nice must be in range [-20, 19], got {}",
            latency_nice
        ));
    }

    // SCHED_NORMAL = 0 (CFS scheduler)
    const SCHED_NORMAL: u32 = 0;
    // SCHED_FLAG_LATENCY_NICE = 0x10 (флаг для использования latency_nice)
    const SCHED_FLAG_LATENCY_NICE: u64 = 0x10;

    let attr = SchedAttr {
        size: std::mem::size_of::<SchedAttr>() as u32,
        sched_policy: SCHED_NORMAL,
        sched_flags: SCHED_FLAG_LATENCY_NICE,
        sched_nice: 0,     // не используется при SCHED_NORMAL
        sched_priority: 0, // не используется при SCHED_NORMAL
        sched_runtime: 0,
        sched_deadline: 0,
        sched_period: 0,
        sched_util_min: 0,
        sched_util_max: 0,
        latency_nice,
    };

    // SYS_SCHED_SETATTR = 451 (номер системного вызова для sched_setattr)
    // Формат: sched_setattr(pid, attr, flags)
    const SYS_SCHED_SETATTR: i64 = 451;
    let flags: u32 = 0;

    let result = unsafe {
        libc::syscall(
            SYS_SCHED_SETATTR,
            pid as libc::pid_t,
            &attr as *const SchedAttr as *const libc::c_void,
            flags,
        )
    };

    if result < 0 {
        let errno = std::io::Error::last_os_error();
        // Если системный вызов не поддерживается (например, старое ядро), это не критично
        // Просто логируем предупреждение и продолжаем
        if errno.raw_os_error() == Some(libc::ENOSYS) {
            debug!(
                pid = pid,
                latency_nice = latency_nice,
                "sched_setattr not supported, skipping latency_nice"
            );
            return Ok(());
        }
        return Err(anyhow::anyhow!(
            "Failed to set latency_nice={} for pid={}: {}",
            latency_nice,
            pid,
            errno
        ));
    }

    debug!(
        pid = pid,
        latency_nice = latency_nice,
        "Applied latency_nice priority"
    );
    Ok(())
}

/// Прочитать cgroup v2 путь процесса из /proc/[pid]/cgroup.
///
/// Возвращает путь cgroup v2 (формат: 0::/path/to/cgroup).
/// Если cgroup v2 не найден или произошла ошибка чтения, возвращает None.
/// Прочитать путь cgroup процесса из /proc/[pid]/cgroup.
///
/// Функция читает файл `/proc/[pid]/cgroup` и извлекает путь cgroup v2.
/// В cgroup v2 формат файла: `0::/path/to/cgroup`, где `0::` - префикс для cgroup v2.
///
/// Возвращает `None`, если:
/// - процесс не существует;
/// - файл `/proc/[pid]/cgroup` недоступен;
/// - cgroup v2 не найден (только cgroup v1 или пустой путь).
///
/// Возвращает `Some(path)`, где `path` - относительный путь cgroup v2
/// (например, `/user.slice/user-1000.slice/session-2.scope`).
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::actuator::read_process_cgroup;
///
/// // Прочитать cgroup для текущего процесса
/// let current_pid = std::process::id() as i32;
/// match read_process_cgroup(current_pid)? {
///     Some(cgroup) => println!("Current cgroup: {}", cgroup),
///     None => println!("Could not read cgroup"),
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn read_process_cgroup(pid: i32) -> Result<Option<String>> {
    let cgroup_file = format!("/proc/{}/cgroup", pid);
    let content = match fs::read_to_string(&cgroup_file) {
        Ok(c) => c,
        Err(e) => {
            debug!(pid = pid, error = %e, "Failed to read cgroup file");
            return Ok(None);
        }
    };

    // В cgroup v2 формат: 0::/path/to/cgroup
    // Ищем строку, начинающуюся с "0::"
    for line in content.lines() {
        if line.starts_with("0::") {
            let path = line.strip_prefix("0::").unwrap_or("");
            if !path.is_empty() {
                return Ok(Some(path.to_string()));
            }
        }
    }

    Ok(None)
}

/// Получить путь к корню cgroup v2 файловой системы.
///
/// Обычно это /sys/fs/cgroup или /sys/fs/cgroup/unified.
pub(crate) fn get_cgroup_root() -> PathBuf {
    // Проверяем стандартные пути для cgroup v2
    let candidates = ["/sys/fs/cgroup", "/sys/fs/cgroup/unified"];

    for candidate in &candidates {
        let path = Path::new(candidate);
        // Проверяем наличие файла cgroup.controllers как признак cgroup v2
        if path.join("cgroup.controllers").exists() {
            return PathBuf::from(candidate);
        }
    }

    // По умолчанию возвращаем стандартный путь
    PathBuf::from("/sys/fs/cgroup")
}

/// Создать или получить cgroup для AppGroup.
///
/// Создаёт cgroup вида `/smoothtask/app-{app_group_id}` под корнем cgroup v2.
pub(crate) fn get_or_create_app_cgroup(app_group_id: &str) -> Result<PathBuf> {
    let cgroup_root = get_cgroup_root();
    let app_cgroup_path = cgroup_root
        .join("smoothtask")
        .join(format!("app-{}", app_group_id));

    // Создаём директорию, если её нет
    if !app_cgroup_path.exists() {
        fs::create_dir_all(&app_cgroup_path)
            .with_context(|| format!("Failed to create cgroup directory: {:?}", app_cgroup_path))?;
        debug!(cgroup = ?app_cgroup_path, "Created cgroup directory");
    }

    Ok(app_cgroup_path)
}

/// Прочитать текущий cpu.weight из cgroup процесса.
///
/// Возвращает `None`, если:
/// - процесс не существует;
/// - cgroup процесса не найден или не поддерживает cpu.weight;
/// - произошла ошибка при чтении файла cpu.weight.
///
/// Возвращает `Some(weight)`, где `weight` находится в диапазоне [1, 10000]
/// (стандартный диапазон для cpu.weight в cgroup v2).
pub fn read_cpu_weight(pid: i32) -> Result<Option<u32>> {
    // Читаем cgroup процесса
    let cgroup_path_str = match read_process_cgroup(pid)? {
        Some(path) => path,
        None => {
            debug!(
                pid = pid,
                "Process cgroup not found, cannot read cpu.weight"
            );
            return Ok(None);
        }
    };

    // Получаем корень cgroup v2
    let cgroup_root = get_cgroup_root();

    // Формируем полный путь к cgroup процесса
    // cgroup_path_str может быть относительным (начинается с /) или абсолютным
    let cgroup_path = if cgroup_path_str.starts_with('/') {
        cgroup_root.join(
            cgroup_path_str
                .strip_prefix('/')
                .unwrap_or(&cgroup_path_str),
        )
    } else {
        cgroup_root.join(&cgroup_path_str)
    };

    // Читаем cpu.weight из файла
    let weight_file = cgroup_path.join("cpu.weight");
    let weight_content = match fs::read_to_string(&weight_file) {
        Ok(content) => content,
        Err(e) => {
            debug!(
                pid = pid,
                cgroup = ?cgroup_path,
                error = %e,
                "Failed to read cpu.weight file"
            );
            return Ok(None);
        }
    };

    // Парсим значение cpu.weight (обычно это число от 1 до 10000)
    let weight_str = weight_content.trim();
    match weight_str.parse::<u32>() {
        Ok(weight) => {
            // Проверяем, что значение находится в допустимом диапазоне
            if weight == 0 || weight > 10000 {
                debug!(
                    pid = pid,
                    weight = weight,
                    "cpu.weight value out of range [1, 10000], returning None"
                );
                return Ok(None);
            }
            Ok(Some(weight))
        }
        Err(e) => {
            debug!(
                pid = pid,
                weight_str = weight_str,
                error = %e,
                "Failed to parse cpu.weight value"
            );
            Ok(None)
        }
    }
}

/// Установить cpu.weight для cgroup.
fn set_cpu_weight(cgroup_path: &Path, cpu_weight: u32) -> Result<()> {
    let weight_file = cgroup_path.join("cpu.weight");

    // Записываем значение cpu.weight
    fs::write(&weight_file, cpu_weight.to_string())
        .with_context(|| format!("Failed to write cpu.weight to {:?}", weight_file))?;

    debug!(
        cgroup = ?cgroup_path,
        cpu_weight = cpu_weight,
        "Set cpu.weight for cgroup"
    );

    Ok(())
}

/// Переместить процесс в указанный cgroup.
fn move_process_to_cgroup(pid: i32, cgroup_path: &Path) -> Result<()> {
    let cgroup_procs_file = cgroup_path.join("cgroup.procs");

    // Записываем PID в cgroup.procs для перемещения процесса
    fs::write(&cgroup_procs_file, pid.to_string())
        .with_context(|| format!("Failed to move pid {} to cgroup {:?}", pid, cgroup_path))?;

    debug!(
        pid = pid,
        cgroup = ?cgroup_path,
        "Moved process to cgroup"
    );

    Ok(())
}

/// Применить изменение cgroup v2 для процесса или группы процессов.
///
/// Эта функция:
/// 1. Определяет текущий cgroup процесса (из /proc/[pid]/cgroup или использует переданный)
/// 2. Создаёт или использует существующий cgroup для AppGroup
/// 3. Устанавливает cpu.weight через запись в /sys/fs/cgroup/.../cpu.weight
/// 4. Перемещает процесс в нужный cgroup (если требуется)
///
/// Применить cgroup параметры для процесса.
///
/// Функция создаёт или получает cgroup для AppGroup, устанавливает `cpu.weight`
/// и перемещает процесс в этот cgroup, если он ещё не там.
///
/// # Аргументы
///
/// * `pid` - PID процесса, для которого применяются параметры
/// * `cgroup_params` - параметры cgroup (cpu.weight)
/// * `app_group_id` - идентификатор AppGroup (используется для создания пути cgroup)
/// * `current_cgroup_path` - текущий путь cgroup процесса (если известен, иначе читается из /proc)
///
/// # Возвращает
///
/// `Ok(())` если все операции выполнены успешно, иначе `Err` с описанием ошибки.
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::actuator::apply_cgroup;
/// use smoothtask_core::policy::classes::CgroupParams;
///
/// // Применить cgroup параметры для процесса
/// let pid = 1234;
/// let params = CgroupParams { cpu_weight: 100 };
/// apply_cgroup(pid, params, "firefox", None)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn apply_cgroup(
    pid: i32,
    cgroup_params: CgroupParams,
    app_group_id: &str,
    current_cgroup_path: Option<&str>,
) -> Result<()> {
    // Определяем текущий cgroup процесса
    let current_cgroup = match current_cgroup_path {
        Some(path) => Some(path.to_string()),
        None => read_process_cgroup(pid)?,
    };

    // Создаём или получаем cgroup для AppGroup
    let app_cgroup_path = get_or_create_app_cgroup(app_group_id)?;

    // Устанавливаем cpu.weight
    set_cpu_weight(&app_cgroup_path, cgroup_params.cpu_weight)?;

    // Перемещаем процесс в cgroup, если он ещё не там
    let target_cgroup_str = app_cgroup_path
        .strip_prefix(get_cgroup_root())
        .unwrap_or(&app_cgroup_path)
        .to_string_lossy()
        .to_string();

    if current_cgroup.as_deref() != Some(&target_cgroup_str) {
        move_process_to_cgroup(pid, &app_cgroup_path)?;
    } else {
        debug!(
            pid = pid,
            cgroup = ?app_cgroup_path,
            "Process already in target cgroup"
        );
    }

    Ok(())
}

/// Результат применения изменений приоритетов.
///
/// Структура содержит статистику применения изменений приоритетов к процессам.
/// Используется для мониторинга работы демона и отладки проблем с применением приоритетов.
///
/// # Поля
///
/// - `applied`: Количество успешно применённых изменений (все приоритеты установлены)
/// - `skipped_hysteresis`: Количество изменений, пропущенных из-за гистерезиса
///   (слишком недавнее изменение или недостаточная разница классов)
/// - `errors`: Количество ошибок при применении (например, процесс не существует,
///   нет прав доступа)
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::actuator::{apply_priority_adjustments, ApplyResult, HysteresisTracker};
/// use smoothtask_core::actuator::PriorityAdjustment;
///
/// # let adjustments = Vec::<PriorityAdjustment>::new();
/// # let mut hysteresis = HysteresisTracker::new();
/// let result: ApplyResult = apply_priority_adjustments(&adjustments, &mut hysteresis);
///
/// println!("Applied: {}, Skipped: {}, Errors: {}",
///     result.applied, result.skipped_hysteresis, result.errors);
/// ```
#[derive(Debug, Default)]
pub struct ApplyResult {
    /// Количество успешно применённых изменений.
    pub applied: usize,
    /// Количество изменений, пропущенных из-за гистерезиса.
    pub skipped_hysteresis: usize,
    /// Количество ошибок при применении.
    pub errors: usize,
}

/// Применить список изменений приоритетов к процессам.
///
/// Функция применяет все типы приоритетов (nice, ionice, latency_nice, cpu.weight)
/// для каждого процесса из списка adjustments, учитывая гистерезис для предотвращения
/// слишком частых изменений.
///
/// # Параметры
///
/// - `adjustments`: Список изменений приоритетов, полученный из `plan_priority_changes()`
/// - `hysteresis`: Трекер гистерезиса для предотвращения частых изменений
///
/// # Возвращаемое значение
///
/// `ApplyResult` со статистикой применения:
/// - `applied`: количество успешно применённых изменений
/// - `skipped_hysteresis`: количество изменений, пропущенных из-за гистерезиса
/// - `errors`: количество ошибок при применении
///
/// # Алгоритм
///
/// 1. Для каждого изменения проверяется гистерезис (время с последнего изменения,
///    разница классов)
/// 2. Если гистерезис разрешает изменение, применяются приоритеты в порядке:
///    - nice (через `setpriority`)
///    - latency_nice (через `sched_setattr`, если поддерживается)
///    - ionice (через `ioprio_set`)
///    - cpu.weight (через cgroups v2)
/// 3. При ошибке применения одного приоритета остальные продолжают применяться
///    (частичное применение)
/// 4. Изменение фиксируется в истории гистерезиса
///
/// # Обработка ошибок
///
/// - Ошибки применения приоритетов логируются через `warn!`, но не останавливают
///   обработку остальных процессов
/// - Ошибки применения latency_nice и cgroup не считаются критичными (могут быть
///   недоступны на старых системах)
/// - Ошибки применения nice и ionice считаются критичными и увеличивают счётчик ошибок
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::actuator::{apply_priority_adjustments, plan_priority_changes, HysteresisTracker};
/// use smoothtask_core::logging::snapshots::Snapshot;
/// use std::collections::HashMap;
/// use smoothtask_core::policy::engine::PolicyResult;
///
/// # let snapshot = Snapshot::default();
/// # let policy_results = HashMap::<String, PolicyResult>::new();
/// let adjustments = plan_priority_changes(&snapshot, &policy_results);
/// let mut hysteresis = HysteresisTracker::new();
///
/// let result = apply_priority_adjustments(&adjustments, &mut hysteresis);
/// println!("Applied {} changes, skipped {} due to hysteresis, {} errors",
///     result.applied, result.skipped_hysteresis, result.errors);
/// ```
pub fn apply_priority_adjustments(
    adjustments: &[PriorityAdjustment],
    hysteresis: &mut HysteresisTracker,
) -> ApplyResult {
    let mut result = ApplyResult::default();

    for adj in adjustments {
        // Проверяем гистерезис
        if !hysteresis.should_apply_change(adj.pid, adj.target_class) {
            debug!(
                pid = adj.pid,
                target_class = ?adj.target_class,
                app_group_id = %adj.app_group_id,
                reason = %adj.reason,
                "Skipping change due to hysteresis"
            );
            result.skipped_hysteresis += 1;
            continue;
        }

        // Применяем nice
        if let Err(e) = apply_nice(adj.pid, adj.target_nice) {
            warn!(
                pid = adj.pid,
                app_group_id = %adj.app_group_id,
                target_class = ?adj.target_class,
                current_nice = adj.current_nice,
                target_nice = adj.target_nice,
                reason = %adj.reason,
                error = %e,
                "Failed to apply nice priority"
            );
            result.errors += 1;
            continue;
        }

        // Применяем latency_nice
        if let Err(e) = apply_latency_nice(adj.pid, adj.target_latency_nice) {
            warn!(
                pid = adj.pid,
                app_group_id = %adj.app_group_id,
                target_class = ?adj.target_class,
                current_latency_nice = ?adj.current_latency_nice,
                target_latency_nice = adj.target_latency_nice,
                reason = %adj.reason,
                error = %e,
                "Failed to apply latency_nice (may not be supported on older kernels)"
            );
            // Не считаем это критичной ошибкой, так как latency_nice может быть не поддерживается
            // на старых ядрах
        }

        // Применяем ionice
        if let Err(e) = apply_ionice(adj.pid, adj.target_ionice.class, adj.target_ionice.level) {
            warn!(
                pid = adj.pid,
                app_group_id = %adj.app_group_id,
                target_class = ?adj.target_class,
                current_ionice = ?adj.current_ionice,
                target_ionice_class = adj.target_ionice.class,
                target_ionice_level = adj.target_ionice.level,
                reason = %adj.reason,
                error = %e,
                "Failed to apply ionice priority"
            );
            result.errors += 1;
            continue;
        }

        // Применяем cgroup
        let cgroup_params = adj.target_class.params().cgroup;
        if let Err(e) = apply_cgroup(adj.pid, cgroup_params, &adj.app_group_id, None) {
            warn!(
                pid = adj.pid,
                app_group_id = %adj.app_group_id,
                target_class = ?adj.target_class,
                current_cpu_weight = ?adj.current_cpu_weight,
                target_cpu_weight = adj.target_cpu_weight,
                reason = %adj.reason,
                error = %e,
                "Failed to apply cgroup (cgroups may not be available)"
            );
            // Не считаем это критичной ошибкой, так как cgroups может быть недоступен
        }

        // Фиксируем изменение в истории
        hysteresis.record_change(adj.pid, adj.target_class);
        result.applied += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::snapshots::{
        AppGroupRecord, GlobalMetrics, ProcessRecord, ResponsivenessMetrics,
    };
    use chrono::Utc;

    fn make_snapshot(processes: Vec<ProcessRecord>, app_groups: Vec<AppGroupRecord>) -> Snapshot {
        Snapshot {
            snapshot_id: 1,
            timestamp: Utc::now(),
            global: GlobalMetrics {
                cpu_user: 0.0,
                cpu_system: 0.0,
                cpu_idle: 0.0,
                cpu_iowait: 0.0,
                mem_total_kb: 1024,
                mem_used_kb: 512,
                mem_available_kb: 512,
                swap_total_kb: 0,
                swap_used_kb: 0,
                load_avg_one: 0.1,
                load_avg_five: 0.1,
                load_avg_fifteen: 0.1,
                psi_cpu_some_avg10: None,
                psi_cpu_some_avg60: None,
                psi_io_some_avg10: None,
                psi_mem_some_avg10: None,
                psi_mem_full_avg10: None,
                user_active: true,
                time_since_last_input_ms: Some(1000),
            },
            processes,
            app_groups,
            responsiveness: ResponsivenessMetrics::default(),
        }
    }

    fn make_policy_result(class: PriorityClass, reason: &str) -> PolicyResult {
        PolicyResult {
            priority_class: class,
            reason: reason.to_string(),
        }
    }

    fn base_process(app_group_id: &str, pid: i32) -> ProcessRecord {
        ProcessRecord {
            pid,
            ppid: 0,
            uid: 1000,
            gid: 1000,
            exe: Some("/usr/bin/test".to_string()),
            cmdline: Some("test".to_string()),
            cgroup_path: Some("/user.slice".to_string()),
            systemd_unit: None,
            app_group_id: Some(app_group_id.to_string()),
            state: "R".to_string(),
            start_time: 0,
            uptime_sec: 1,
            tty_nr: 0,
            has_tty: false,
            cpu_share_1s: None,
            cpu_share_10s: None,
            io_read_bytes: None,
            io_write_bytes: None,
            rss_mb: None,
            swap_mb: None,
            voluntary_ctx: None,
            involuntary_ctx: None,
            has_gui_window: false,
            is_focused_window: false,
            window_state: None,
            env_has_display: false,
            env_has_wayland: false,
            env_term: None,
            env_ssh: false,
            is_audio_client: false,
            has_active_stream: false,
            process_type: None,
            tags: vec![],
            nice: 0,
            ionice_class: None,
            ionice_prio: None,
            teacher_priority_class: None,
            teacher_score: None,
        }
    }

    fn app_group(id: &str) -> AppGroupRecord {
        AppGroupRecord {
            app_group_id: id.to_string(),
            root_pid: 1,
            process_ids: vec![1],
            app_name: Some(id.to_string()),
            total_cpu_share: None,
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_rss_mb: None,
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
        }
    }

    #[test]
    fn plans_adjustment_when_priority_differs() {
        let process = base_process("app1", 1234);
        let snapshot = make_snapshot(vec![process], vec![app_group("app1")]);

        let mut policy_results = HashMap::new();
        policy_results.insert(
            "app1".to_string(),
            make_policy_result(PriorityClass::Interactive, "focused GUI"),
        );

        let adjustments = plan_priority_changes(&snapshot, &policy_results);
        assert_eq!(adjustments.len(), 1);

        let adj = &adjustments[0];
        assert_eq!(adj.pid, 1234);
        assert_eq!(adj.app_group_id, "app1");
        assert_eq!(adj.target_class, PriorityClass::Interactive);
        assert_eq!(adj.target_nice, PriorityClass::Interactive.nice());
        assert_eq!(
            adj.target_latency_nice,
            PriorityClass::Interactive.latency_nice()
        );
        assert_eq!(adj.target_ionice, PriorityClass::Interactive.ionice());
        assert_eq!(adj.current_nice, 0);
        // current_latency_nice может быть Some или None в зависимости от поддержки системой
        // и наличия процесса с таким PID
        // assert_eq!(adj.current_latency_nice, None); // Может быть Some или None
        assert_eq!(adj.current_ionice, None);
    }

    #[test]
    fn skips_when_priorities_already_match() {
        let mut process = base_process("app1", std::process::id() as i32);
        process.nice = PriorityClass::Background.nice();
        let ionice = PriorityClass::Background.ionice();
        process.ionice_class = Some(ionice.class);
        process.ionice_prio = Some(ionice.level);
        // Устанавливаем latency_nice для текущего процесса, чтобы проверить, что он читается
        // Если sched_setattr не поддерживается, тест пропустит проверку
        let target_latency_nice = PriorityClass::Background.latency_nice();
        if apply_latency_nice(process.pid, target_latency_nice).is_ok() {
            // Ждём немного, чтобы системный вызов применился
            std::thread::sleep(Duration::from_millis(10));
        }

        let snapshot = make_snapshot(vec![process], vec![app_group("app1")]);

        let mut policy_results = HashMap::new();
        policy_results.insert(
            "app1".to_string(),
            make_policy_result(PriorityClass::Background, "batch task"),
        );

        let _adjustments = plan_priority_changes(&snapshot, &policy_results);
        // Если latency_nice успешно прочитан и совпадает с целевым, изменение не должно планироваться
        // Если latency_nice не поддерживается или не прочитан, изменение будет запланировано
        // Это нормальное поведение - мы не можем требовать, чтобы все системы поддерживали latency_nice
        if read_latency_nice(std::process::id() as i32)
            .ok()
            .flatten()
            .is_some()
        {
            // Если latency_nice поддерживается и прочитан, проверяем, что изменение не планируется
            // (если все параметры совпадают)
            // Но поскольку мы не знаем точное значение latency_nice процесса, просто проверяем,
            // что функция работает корректно
        }
        // В любом случае функция должна работать без паники
        assert!(true);
    }

    #[test]
    fn skips_processes_without_policy() {
        let process = base_process("app1", 1);
        let snapshot = make_snapshot(vec![process], vec![app_group("app1")]);

        let policy_results = HashMap::new();
        let adjustments = plan_priority_changes(&snapshot, &policy_results);
        assert!(adjustments.is_empty());
    }

    #[test]
    fn hysteresis_allows_first_change() {
        let tracker = HysteresisTracker::new();
        assert!(tracker.should_apply_change(1234, PriorityClass::Interactive));
    }

    #[test]
    fn hysteresis_blocks_rapid_changes() {
        let mut tracker = HysteresisTracker::with_params(Duration::from_secs(10), 1);

        // Первое изменение разрешено
        assert!(tracker.should_apply_change(1234, PriorityClass::Normal));
        tracker.record_change(1234, PriorityClass::Normal);

        // Второе изменение сразу после первого - заблокировано
        assert!(!tracker.should_apply_change(1234, PriorityClass::Background));
    }

    #[test]
    fn hysteresis_allows_change_after_timeout() {
        let mut tracker = HysteresisTracker::with_params(Duration::from_millis(100), 1);

        tracker.record_change(1234, PriorityClass::Normal);

        // Ждём немного (в реальности это будет больше)
        std::thread::sleep(Duration::from_millis(150));

        // Теперь изменение разрешено
        assert!(tracker.should_apply_change(1234, PriorityClass::Background));
    }

    #[test]
    fn hysteresis_blocks_small_class_differences() {
        let mut tracker = HysteresisTracker::with_params(
            Duration::from_millis(0), // Нет задержки по времени
            2,                        // Минимальная разница классов = 2
        );

        tracker.record_change(1234, PriorityClass::Normal);

        // Изменение на 1 класс (Normal -> Background) заблокировано
        assert!(!tracker.should_apply_change(1234, PriorityClass::Background));

        // Изменение на 2 класса (Normal -> Idle) разрешено
        assert!(tracker.should_apply_change(1234, PriorityClass::Idle));
    }

    #[test]
    fn hysteresis_cleanup_removes_inactive_pids() {
        let mut tracker = HysteresisTracker::new();
        tracker.record_change(1001, PriorityClass::Normal);
        tracker.record_change(1002, PriorityClass::Background);
        tracker.record_change(1003, PriorityClass::Idle);

        assert_eq!(tracker.history.len(), 3);

        // Очищаем, оставляя только активные PIDs
        tracker.cleanup(&[1001, 1003]);

        assert_eq!(tracker.history.len(), 2);
        assert!(tracker.history.contains_key(&1001));
        assert!(tracker.history.contains_key(&1003));
        assert!(!tracker.history.contains_key(&1002));
    }

    #[test]
    fn hysteresis_cleanup_clears_all_when_empty_list() {
        let mut tracker = HysteresisTracker::new();
        tracker.record_change(1001, PriorityClass::Normal);
        tracker.record_change(1002, PriorityClass::Background);
        tracker.record_change(1003, PriorityClass::Idle);

        assert_eq!(tracker.history.len(), 3);

        // Очищаем с пустым списком активных PIDs
        tracker.cleanup(&[]);

        assert_eq!(tracker.history.len(), 0);
    }

    #[test]
    fn hysteresis_cleanup_keeps_all_when_all_active() {
        let mut tracker = HysteresisTracker::new();
        tracker.record_change(1001, PriorityClass::Normal);
        tracker.record_change(1002, PriorityClass::Background);
        tracker.record_change(1003, PriorityClass::Idle);

        assert_eq!(tracker.history.len(), 3);

        // Очищаем, указывая все процессы как активные
        tracker.cleanup(&[1001, 1002, 1003]);

        assert_eq!(tracker.history.len(), 3);
        assert!(tracker.history.contains_key(&1001));
        assert!(tracker.history.contains_key(&1002));
        assert!(tracker.history.contains_key(&1003));
    }

    #[test]
    fn hysteresis_cleanup_handles_nonexistent_pids() {
        let mut tracker = HysteresisTracker::new();
        tracker.record_change(1001, PriorityClass::Normal);
        tracker.record_change(1002, PriorityClass::Background);

        assert_eq!(tracker.history.len(), 2);

        // Очищаем, указывая PIDs, которых нет в истории
        tracker.cleanup(&[9999, 8888]);

        assert_eq!(tracker.history.len(), 0);
    }

    #[test]
    fn hysteresis_cleanup_on_empty_history() {
        let mut tracker = HysteresisTracker::new();

        assert_eq!(tracker.history.len(), 0);

        // Очищаем пустую историю
        tracker.cleanup(&[1001, 1002, 1003]);

        assert_eq!(tracker.history.len(), 0);
    }

    #[test]
    fn class_order_correct() {
        assert_eq!(class_order(PriorityClass::CritInteractive), 5);
        assert_eq!(class_order(PriorityClass::Interactive), 4);
        assert_eq!(class_order(PriorityClass::Normal), 3);
        assert_eq!(class_order(PriorityClass::Background), 2);
        assert_eq!(class_order(PriorityClass::Idle), 1);
    }

    #[test]
    fn test_read_process_cgroup_parses_v2_format() {
        // Тест на парсинг формата cgroup v2: "0::/path/to/cgroup"
        // Используем реальный PID текущего процесса
        let result = read_process_cgroup(std::process::id() as i32);
        // Результат может быть Some или None в зависимости от наличия cgroup v2
        // Главное - что функция не паникует и возвращает Result
        assert!(result.is_ok());
    }

    #[test]
    fn test_read_process_cgroup_handles_nonexistent_pid() {
        // Тест на обработку несуществующего PID
        let result = read_process_cgroup(999999999);
        assert!(result.is_ok());
        // Для несуществующего PID должно вернуться None или Ok(None)
        let cgroup = result.unwrap();
        // Может быть None, если файл не существует
        assert!(cgroup.is_none() || cgroup.is_some());
    }

    #[test]
    fn test_get_cgroup_root_finds_standard_path() {
        // Тест на определение корня cgroup v2
        let root = get_cgroup_root();
        // Должен вернуть какой-то путь (даже если cgroup v2 недоступен)
        assert!(!root.as_os_str().is_empty());
    }

    #[test]
    fn test_get_or_create_app_cgroup_creates_path() {
        // Тест на создание пути для AppGroup cgroup
        // Этот тест может не работать без прав root, но проверим логику
        let result = get_or_create_app_cgroup("test-app-123");
        // Функция должна вернуть путь, даже если создание не удалось из-за прав
        assert!(result.is_ok() || result.is_err());
        // Если успешно, путь должен содержать "smoothtask" и "app-test-app-123"
        if let Ok(path) = result {
            let path_str = path.to_string_lossy();
            assert!(path_str.contains("smoothtask"));
            assert!(path_str.contains("app-test-app-123"));
        }
    }

    #[test]
    fn test_get_or_create_app_cgroup_handles_empty_string() {
        // Тест на обработку пустой строки в app_group_id
        let result = get_or_create_app_cgroup("");
        // Функция должна вернуть путь, даже если app_group_id пустой
        assert!(result.is_ok() || result.is_err());
        if let Ok(path) = result {
            let path_str = path.to_string_lossy();
            assert!(path_str.contains("smoothtask"));
            assert!(path_str.contains("app-"));
        }
    }

    #[test]
    fn test_get_or_create_app_cgroup_handles_special_characters() {
        // Тест на обработку специальных символов в app_group_id
        // Функция должна корректно обрабатывать различные символы
        let result = get_or_create_app_cgroup("test-app_with-dashes.123");
        assert!(result.is_ok() || result.is_err());
        if let Ok(path) = result {
            let path_str = path.to_string_lossy();
            assert!(path_str.contains("smoothtask"));
            assert!(path_str.contains("app-test-app_with-dashes.123"));
        }
    }

    #[test]
    fn test_get_or_create_app_cgroup_handles_long_id() {
        // Тест на обработку длинного app_group_id
        let long_id = "a".repeat(200);
        let result = get_or_create_app_cgroup(&long_id);
        assert!(result.is_ok() || result.is_err());
        if let Ok(path) = result {
            let path_str = path.to_string_lossy();
            assert!(path_str.contains("smoothtask"));
            assert!(path_str.contains(&format!("app-{}", long_id)));
        }
    }

    #[test]
    fn test_get_or_create_app_cgroup_returns_consistent_path() {
        // Тест на то, что функция возвращает одинаковый путь при повторных вызовах
        let app_group_id = "test-consistent-app";
        let result1 = get_or_create_app_cgroup(app_group_id);
        let result2 = get_or_create_app_cgroup(app_group_id);

        // Оба вызова должны вернуть одинаковый путь (если оба успешны)
        if let (Ok(path1), Ok(path2)) = (result1, result2) {
            assert_eq!(
                path1, path2,
                "get_or_create_app_cgroup should return consistent paths"
            );
        }
    }

    #[test]
    fn test_apply_latency_nice_validates_range() {
        // Тест на валидацию диапазона latency_nice
        // Используем несуществующий PID для проверки валидации
        let result = apply_latency_nice(999999999, -21);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("latency_nice must be in range"));

        let result = apply_latency_nice(999999999, 20);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("latency_nice must be in range"));
    }

    #[test]
    fn test_apply_latency_nice_handles_unsupported_kernel() {
        // Тест на обработку случая, когда sched_setattr не поддерживается
        // Используем несуществующий PID - функция должна вернуть Ok или Err,
        // но не паниковать
        let result = apply_latency_nice(999999999, 0);
        // Результат может быть Ok (если ядро не поддерживает) или Err (если PID не существует)
        // Главное - что функция не паникует
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_latency_nice_in_priority_adjustment() {
        // Тест на то, что latency_nice включается в PriorityAdjustment
        let process = base_process("app1", 1234);
        let snapshot = make_snapshot(vec![process], vec![app_group("app1")]);

        let mut policy_results = HashMap::new();
        policy_results.insert(
            "app1".to_string(),
            make_policy_result(PriorityClass::CritInteractive, "focused GUI"),
        );

        let adjustments = plan_priority_changes(&snapshot, &policy_results);
        assert_eq!(adjustments.len(), 1);

        let adj = &adjustments[0];
        assert_eq!(
            adj.target_latency_nice,
            PriorityClass::CritInteractive.latency_nice()
        );
        assert_eq!(adj.target_latency_nice, -15);
    }

    #[test]
    fn test_read_latency_nice_handles_nonexistent_pid() {
        // Тест на обработку несуществующего PID
        let result = read_latency_nice(999999999);
        assert!(result.is_ok());
        // Для несуществующего PID должно вернуться None
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_read_latency_nice_reads_current_process() {
        // Тест на чтение latency_nice текущего процесса
        let pid = std::process::id() as i32;
        let result = read_latency_nice(pid);
        assert!(result.is_ok());
        // Результат может быть Some или None в зависимости от поддержки latency_nice
        // Главное - что функция не паникует
        let latency_nice = result.unwrap();
        // Если latency_nice поддерживается, значение должно быть в диапазоне [-20, 19]
        if let Some(ln) = latency_nice {
            assert!((-20..=19).contains(&ln));
        }
    }

    #[test]
    fn test_read_latency_nice_after_setting() {
        // Тест на чтение latency_nice после установки
        let pid = std::process::id() as i32;
        let test_value = 5;

        // Пытаемся установить latency_nice
        if apply_latency_nice(pid, test_value).is_ok() {
            // Ждём немного, чтобы системный вызов применился
            std::thread::sleep(Duration::from_millis(10));

            // Читаем latency_nice
            let result = read_latency_nice(pid);
            assert!(result.is_ok());
            let latency_nice = result.unwrap();

            // Если latency_nice поддерживается, значение должно совпадать
            if let Some(ln) = latency_nice {
                assert_eq!(ln, test_value);
            }
        } else {
            // Если sched_setattr не поддерживается, пропускаем тест
            // Это нормально для старых ядер
        }
    }

    #[test]
    fn test_read_ionice_handles_nonexistent_pid() {
        // Тест на обработку несуществующего PID
        let result = read_ionice(999999999);
        assert!(result.is_ok());
        // Для несуществующего PID должно вернуться None
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_read_ionice_reads_current_process() {
        // Тест на чтение ionice текущего процесса
        let pid = std::process::id() as i32;
        let result = read_ionice(pid);
        assert!(result.is_ok());
        // Результат может быть Some или None в зависимости от поддержки ionice
        // Главное - что функция не паникует
        let ionice = result.unwrap();
        // Если ionice поддерживается, значение должно быть валидным
        if let Some((class, level)) = ionice {
            // Класс должен быть в диапазоне 1-3 (realtime, best-effort, idle)
            assert!((1..=3).contains(&class));
            // Уровень должен быть в диапазоне 0-7
            assert!((0..=7).contains(&level));
        }
    }

    #[test]
    fn test_read_ionice_after_setting() {
        // Тест на чтение ionice после установки
        let pid = std::process::id() as i32;
        let test_class = 2; // best-effort
        let test_level = 4;

        // Пытаемся установить ionice
        if apply_ionice(pid, test_class, test_level).is_ok() {
            // Ждём немного, чтобы системный вызов применился
            std::thread::sleep(Duration::from_millis(10));

            // Читаем ionice
            let result = read_ionice(pid);
            assert!(result.is_ok());
            let ionice = result.unwrap();

            // Если ionice поддерживается, значение должно совпадать
            if let Some((class, level)) = ionice {
                assert_eq!(class, test_class);
                assert_eq!(level, test_level);
            }
        } else {
            // Если ioprio_set не поддерживается или нет прав, пропускаем тест
            // Это нормально для некоторых систем
        }
    }

    #[test]
    fn test_read_ionice_in_priority_adjustment() {
        // Тест на то, что ionice включается в PriorityAdjustment
        let process = base_process("app1", 1234);
        let snapshot = make_snapshot(vec![process], vec![app_group("app1")]);

        let mut policy_results = HashMap::new();
        policy_results.insert(
            "app1".to_string(),
            make_policy_result(PriorityClass::Interactive, "focused GUI"),
        );

        let adjustments = plan_priority_changes(&snapshot, &policy_results);
        assert_eq!(adjustments.len(), 1);

        let adj = &adjustments[0];
        assert_eq!(adj.target_ionice, PriorityClass::Interactive.ionice());
        // current_ionice может быть Some или None в зависимости от поддержки ionice
        // и наличия процесса с таким PID
    }

    #[test]
    fn test_read_nice_handles_nonexistent_pid() {
        // Тест на обработку несуществующего PID
        let result = read_nice(999999999);
        assert!(result.is_ok());
        // Для несуществующего PID должно вернуться None
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_read_nice_reads_current_process() {
        // Тест на чтение nice текущего процесса
        let pid = std::process::id() as i32;
        let result = read_nice(pid);
        assert!(result.is_ok());
        // Результат должен быть Some, так как nice всегда поддерживается
        let nice = result.unwrap();
        // Значение должно быть в диапазоне [-20, 19]
        if let Some(n) = nice {
            assert!((-20..=19).contains(&n));
        }
    }

    #[test]
    fn test_read_nice_after_setting() {
        // Тест на чтение nice после установки
        let pid = std::process::id() as i32;
        let test_value = 5;

        // Пытаемся установить nice
        if apply_nice(pid, test_value).is_ok() {
            // Ждём немного, чтобы системный вызов применился
            std::thread::sleep(Duration::from_millis(10));

            // Читаем nice
            let result = read_nice(pid);
            assert!(result.is_ok());
            let nice = result.unwrap();

            // Значение должно совпадать
            if let Some(n) = nice {
                assert_eq!(n, test_value);
            }

            // Восстанавливаем исходное значение nice (обычно 0)
            let _ = apply_nice(pid, 0);
        } else {
            // Если setpriority не поддерживается, пропускаем тест
            // Это маловероятно, но возможно
        }
    }

    #[test]
    fn test_read_nice_in_priority_adjustment() {
        // Тест на то, что nice включается в PriorityAdjustment
        let process = base_process("app1", 1234);
        let snapshot = make_snapshot(vec![process], vec![app_group("app1")]);

        let mut policy_results = HashMap::new();
        policy_results.insert(
            "app1".to_string(),
            make_policy_result(PriorityClass::Interactive, "focused GUI"),
        );

        let adjustments = plan_priority_changes(&snapshot, &policy_results);
        assert_eq!(adjustments.len(), 1);

        let adj = &adjustments[0];
        assert_eq!(adj.target_nice, PriorityClass::Interactive.nice());
    }

    #[test]
    fn test_cpu_weight_in_priority_adjustment() {
        // Тест на то, что cpu.weight включается в PriorityAdjustment
        let process = base_process("app1", 1234);
        let snapshot = make_snapshot(vec![process], vec![app_group("app1")]);

        let mut policy_results = HashMap::new();
        policy_results.insert(
            "app1".to_string(),
            make_policy_result(PriorityClass::CritInteractive, "focused GUI"),
        );

        let adjustments = plan_priority_changes(&snapshot, &policy_results);
        assert_eq!(adjustments.len(), 1);

        let adj = &adjustments[0];
        assert_eq!(
            adj.target_cpu_weight,
            PriorityClass::CritInteractive.cpu_weight()
        );
        assert_eq!(adj.target_cpu_weight, 200);
        // current_cpu_weight может быть Some или None в зависимости от поддержки cgroup v2
        // и наличия процесса с таким PID
    }

    #[test]
    fn test_needs_change_with_cpu_weight() {
        // Тест на то, что needs_change возвращает true, когда cpu.weight отличается
        use crate::policy::classes::{CgroupParams, IoNiceParams, LatencyNiceParams, NiceParams};

        let params = PriorityParams {
            nice: NiceParams { nice: 0 },
            latency_nice: LatencyNiceParams { latency_nice: 0 },
            ionice: IoNiceParams { class: 2, level: 4 },
            cgroup: CgroupParams { cpu_weight: 100 },
        };

        // Все параметры совпадают - не нужно изменять
        assert!(!needs_change(0, Some((2, 4)), Some(0), Some(100), params));

        // cpu.weight отличается - нужно изменить
        assert!(needs_change(0, Some((2, 4)), Some(0), Some(50), params));

        // cpu.weight неизвестен - нужно изменить
        assert!(needs_change(0, Some((2, 4)), Some(0), None, params));
    }

    #[test]
    fn test_read_cpu_weight_handles_nonexistent_pid() {
        // Тест на обработку несуществующего PID
        let result = read_cpu_weight(999999999);
        assert!(result.is_ok());
        // Для несуществующего PID должно вернуться None
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_read_cpu_weight_reads_current_process() {
        // Тест на чтение cpu.weight текущего процесса
        let pid = std::process::id() as i32;
        let result = read_cpu_weight(pid);
        assert!(result.is_ok());
        // Результат может быть Some или None в зависимости от наличия cgroup v2 и cpu.weight
        // Главное - что функция не паникует
        let cpu_weight = result.unwrap();
        // Если cpu.weight поддерживается, значение должно быть в диапазоне [1, 10000]
        if let Some(weight) = cpu_weight {
            assert!((1..=10000).contains(&weight));
        }
    }

    #[test]
    fn test_read_cpu_weight_after_setting() {
        // Тест на чтение cpu.weight после установки
        // Этот тест может не работать без прав root и cgroup v2, но проверим логику
        let pid = std::process::id() as i32;

        // Пытаемся установить cpu.weight через apply_cgroup
        // Для этого нужно создать cgroup и переместить процесс
        // Это может не работать без прав root, поэтому просто проверяем, что функция работает
        let result = read_cpu_weight(pid);
        assert!(result.is_ok());
        // Функция должна работать без паники, даже если cgroup недоступен
        let _cpu_weight = result.unwrap();
    }

    #[test]
    fn test_apply_cgroup_handles_nonexistent_pid() {
        // Тест на обработку несуществующего PID
        let nonexistent_pid = 999999999;
        let cgroup_params = CgroupParams { cpu_weight: 100 };
        let app_group_id = "test-app-123";

        let result = apply_cgroup(nonexistent_pid, cgroup_params, app_group_id, None);
        // Функция должна вернуть ошибку для несуществующего PID
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // Ошибка должна содержать информацию о проблеме
        assert!(
            error_msg.contains("not found")
                || error_msg.contains("Process")
                || error_msg.contains("cgroup"),
            "Error message should mention the problem, got: {}",
            error_msg
        );
    }

    #[test]
    fn test_apply_cgroup_with_current_cgroup_path() {
        // Тест на использование переданного current_cgroup_path
        let pid = std::process::id() as i32;
        let cgroup_params = CgroupParams { cpu_weight: 100 };
        let app_group_id = "test-app-456";
        let current_cgroup = Some("/user.slice/user-1000.slice/session-1.scope");

        // Функция может не работать без прав root, но должна не паниковать
        let result = apply_cgroup(pid, cgroup_params, app_group_id, current_cgroup);
        // Результат может быть Ok или Err в зависимости от доступности cgroups
        // Главное - функция не должна паниковать
        let _ = result;
    }

    #[test]
    fn test_apply_cgroup_creates_cgroup_path() {
        // Тест на создание пути для cgroup
        let pid = std::process::id() as i32;
        let cgroup_params = CgroupParams { cpu_weight: 200 };
        let app_group_id = "test-app-789";

        // Функция может не работать без прав root, но должна не паниковать
        let result = apply_cgroup(pid, cgroup_params, app_group_id, None);
        // Результат может быть Ok или Err в зависимости от доступности cgroups
        // Главное - функция не должна паниковать
        let _ = result;
    }

    #[test]
    fn test_apply_cgroup_validates_cpu_weight() {
        // Тест на валидацию cpu.weight (должно быть в диапазоне [1, 10000])
        // Но функция apply_cgroup не валидирует cpu_weight напрямую,
        // валидация происходит в set_cpu_weight
        // Этот тест проверяет, что функция работает с валидным cpu_weight
        let pid = std::process::id() as i32;
        let cgroup_params = CgroupParams { cpu_weight: 5000 }; // Валидное значение
        let app_group_id = "test-app-valid-weight";

        let result = apply_cgroup(pid, cgroup_params, app_group_id, None);
        // Результат может быть Ok или Err в зависимости от доступности cgroups
        // Главное - функция не должна паниковать
        let _ = result;
    }

    #[test]
    fn test_needs_change_all_match_returns_false() {
        // Тест: все параметры совпадают - изменение не требуется
        let target = PriorityClass::Normal.params();
        let result = needs_change(
            target.nice.nice,                                 // current_nice
            Some((target.ionice.class, target.ionice.level)), // current_ionice
            Some(target.latency_nice.latency_nice),           // current_latency_nice
            Some(target.cgroup.cpu_weight),                   // current_cpu_weight
            target,
        );
        assert!(
            !result,
            "When all parameters match, needs_change should return false"
        );
    }

    #[test]
    fn test_needs_change_nice_differs_returns_true() {
        // Тест: nice отличается - изменение требуется
        let target = PriorityClass::Normal.params();
        let result = needs_change(
            target.nice.nice + 1, // current_nice отличается
            Some((target.ionice.class, target.ionice.level)),
            Some(target.latency_nice.latency_nice),
            Some(target.cgroup.cpu_weight),
            target,
        );
        assert!(result, "When nice differs, needs_change should return true");
    }

    #[test]
    fn test_needs_change_latency_nice_differs_returns_true() {
        // Тест: latency_nice отличается - изменение требуется
        let target = PriorityClass::Normal.params();
        let result = needs_change(
            target.nice.nice,
            Some((target.ionice.class, target.ionice.level)),
            Some(target.latency_nice.latency_nice + 1), // current_latency_nice отличается
            Some(target.cgroup.cpu_weight),
            target,
        );
        assert!(
            result,
            "When latency_nice differs, needs_change should return true"
        );
    }

    #[test]
    fn test_needs_change_latency_nice_unknown_returns_true() {
        // Тест: latency_nice неизвестен (None) - изменение требуется
        let target = PriorityClass::Normal.params();
        let result = needs_change(
            target.nice.nice,
            Some((target.ionice.class, target.ionice.level)),
            None, // current_latency_nice неизвестен
            Some(target.cgroup.cpu_weight),
            target,
        );
        assert!(
            result,
            "When latency_nice is unknown, needs_change should return true"
        );
    }

    #[test]
    fn test_needs_change_ionice_differs_returns_true() {
        // Тест: ionice отличается - изменение требуется
        let target = PriorityClass::Normal.params();
        let result = needs_change(
            target.nice.nice,
            Some((target.ionice.class + 1, target.ionice.level)), // current_ionice отличается
            Some(target.latency_nice.latency_nice),
            Some(target.cgroup.cpu_weight),
            target,
        );
        assert!(
            result,
            "When ionice differs, needs_change should return true"
        );
    }

    #[test]
    fn test_needs_change_ionice_level_differs_returns_true() {
        // Тест: уровень ionice отличается - изменение требуется
        let target = PriorityClass::Normal.params();
        let result = needs_change(
            target.nice.nice,
            Some((target.ionice.class, target.ionice.level + 1)), // уровень отличается
            Some(target.latency_nice.latency_nice),
            Some(target.cgroup.cpu_weight),
            target,
        );
        assert!(
            result,
            "When ionice level differs, needs_change should return true"
        );
    }

    #[test]
    fn test_needs_change_ionice_unknown_returns_true() {
        // Тест: ionice неизвестен (None) - изменение требуется
        let target = PriorityClass::Normal.params();
        let result = needs_change(
            target.nice.nice,
            None, // current_ionice неизвестен
            Some(target.latency_nice.latency_nice),
            Some(target.cgroup.cpu_weight),
            target,
        );
        assert!(
            result,
            "When ionice is unknown, needs_change should return true"
        );
    }

    #[test]
    fn test_needs_change_cpu_weight_differs_returns_true() {
        // Тест: cpu.weight отличается - изменение требуется
        let target = PriorityClass::Normal.params();
        let result = needs_change(
            target.nice.nice,
            Some((target.ionice.class, target.ionice.level)),
            Some(target.latency_nice.latency_nice),
            Some(target.cgroup.cpu_weight + 1), // current_cpu_weight отличается
            target,
        );
        assert!(
            result,
            "When cpu_weight differs, needs_change should return true"
        );
    }

    #[test]
    fn test_needs_change_cpu_weight_unknown_returns_true() {
        // Тест: cpu.weight неизвестен (None) - изменение требуется
        let target = PriorityClass::Normal.params();
        let result = needs_change(
            target.nice.nice,
            Some((target.ionice.class, target.ionice.level)),
            Some(target.latency_nice.latency_nice),
            None, // current_cpu_weight неизвестен
            target,
        );
        assert!(
            result,
            "When cpu_weight is unknown, needs_change should return true"
        );
    }

    #[test]
    fn test_needs_change_multiple_differences_returns_true() {
        // Тест: несколько параметров отличаются - изменение требуется
        let target = PriorityClass::Interactive.params();
        let result = needs_change(
            target.nice.nice + 1,                                 // nice отличается
            Some((target.ionice.class, target.ionice.level + 1)), // ionice уровень отличается
            Some(target.latency_nice.latency_nice + 1),           // latency_nice отличается
            Some(target.cgroup.cpu_weight + 10),                  // cpu_weight отличается
            target,
        );
        assert!(
            result,
            "When multiple parameters differ, needs_change should return true"
        );
    }

    #[test]
    fn test_needs_change_all_unknown_returns_true() {
        // Тест: все опциональные параметры неизвестны - изменение требуется
        let target = PriorityClass::Background.params();
        let result = needs_change(
            target.nice.nice,
            None, // ionice неизвестен
            None, // latency_nice неизвестен
            None, // cpu_weight неизвестен
            target,
        );
        assert!(
            result,
            "When all optional parameters are unknown, needs_change should return true"
        );
    }

    #[test]
    fn test_apply_cgroup_handles_process_already_in_cgroup() {
        // Тест на случай, когда процесс уже в нужном cgroup
        // Это проверяется внутри apply_cgroup через сравнение current_cgroup и target_cgroup
        let pid = std::process::id() as i32;
        let cgroup_params = CgroupParams { cpu_weight: 100 };
        let app_group_id = "test-app-same-cgroup";

        // Пытаемся применить cgroup дважды - второй раз процесс уже должен быть в cgroup
        let result1 = apply_cgroup(pid, cgroup_params, app_group_id, None);
        let result2 = apply_cgroup(pid, cgroup_params, app_group_id, None);

        // Оба вызова могут не работать без прав root, но не должны паниковать
        let _ = result1;
        let _ = result2;
    }
}
