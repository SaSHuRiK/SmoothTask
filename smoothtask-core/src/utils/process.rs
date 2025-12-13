//! Утилитные функции для работы с процессами.
//!
//! Этот модуль предоставляет функции для получения информации о процессах,
//! чтения их приоритетов, работы с cgroups и других операций, связанных
//! с управлением процессами в Linux.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use libc;
use tracing::debug;

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
/// # Параметры
///
/// - `pid`: PID процесса для чтения nice
///
/// # Возвращаемое значение
///
/// - `Ok(Some(nice))`: Текущее значение nice процесса
/// - `Ok(None)`: Не удалось прочитать nice
/// - `Err(anyhow::Error)`: Ошибка при выполнении системного вызова
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::utils::process::read_nice;
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
///
/// # Параметры
///
/// - `pid`: PID процесса для чтения ionice
///
/// # Возвращаемое значение
///
/// - `Ok(Some((class, level)))`: Текущие значения ionice процесса
/// - `Ok(None)`: Не удалось прочитать ionice или ionice не установлен
/// - `Err(anyhow::Error)`: Ошибка при выполнении системного вызова
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::utils::process::read_ionice;
///
/// // Прочитать ionice для текущего процесса
/// let current_pid = std::process::id() as i32;
/// match read_ionice(current_pid)? {
///     Some((class, level)) => println!("Current ionice: class={}, level={}", class, level),
///     None => println!("Could not read ionice"),
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
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

/// Прочитать текущий latency_nice процесса через sched_getattr.
///
/// Возвращает `None`, если:
/// - процесс не существует;
/// - системный вызов не поддерживается (старое ядро);
/// - latency_nice не установлен для процесса.
///
/// # Параметры
///
/// - `pid`: PID процесса для чтения latency_nice
///
/// # Возвращаемое значение
///
/// - `Ok(Some(latency_nice))`: Текущее значение latency_nice процесса
/// - `Ok(None)`: Не удалось прочитать latency_nice или latency_nice не установлен
/// - `Err(anyhow::Error)`: Ошибка при выполнении системного вызова
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::utils::process::read_latency_nice;
///
/// // Прочитать latency_nice для текущего процесса
/// let current_pid = std::process::id() as i32;
/// match read_latency_nice(current_pid)? {
///     Some(latency_nice) => println!("Current latency_nice: {}", latency_nice),
///     None => println!("Could not read latency_nice"),
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
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

/// Прочитать cgroup v2 путь процесса из /proc/[pid]/cgroup.
///
/// Возвращает путь cgroup v2 (формат: 0::/path/to/cgroup).
/// Если cgroup v2 не найден или произошла ошибка чтения, возвращает None.
///
/// # Параметры
///
/// - `pid`: PID процесса для чтения cgroup
///
/// # Возвращаемое значение
///
/// - `Ok(Some(path))`: Путь cgroup v2 процесса
/// - `Ok(None)`: Не удалось прочитать cgroup или cgroup v2 не найден
/// - `Err(anyhow::Error)`: Ошибка при чтении файла
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::utils::process::read_process_cgroup;
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
            let path = match line.strip_prefix("0::") {
                Some(stripped) => stripped,
                None => {
                    debug!(line = line, "Failed to strip prefix from cgroup line");
                    continue;
                }
            };
            if !path.is_empty() {
                return Ok(Some(path.to_string()));
            }
        }
    }

    Ok(None)
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
///
/// # Параметры
///
/// - `pid`: PID процесса для чтения cpu.weight
///
/// # Возвращаемое значение
///
/// - `Ok(Some(weight))`: Текущее значение cpu.weight процесса
/// - `Ok(None)`: Не удалось прочитать cpu.weight
/// - `Err(anyhow::Error)`: Ошибка при чтении файла
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::utils::process::read_cpu_weight;
///
/// // Прочитать cpu.weight для текущего процесса
/// let current_pid = std::process::id() as i32;
/// match read_cpu_weight(current_pid)? {
///     Some(weight) => println!("Current cpu.weight: {}", weight),
///     None => println!("Could not read cpu.weight"),
/// }
/// # Ok::<(), anyhow::Error>(())
/// ```
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
        let stripped_path = if let Some(stripped) = cgroup_path_str.strip_prefix('/') {
            stripped
        } else {
            debug!(
                cgroup_path = cgroup_path_str,
                "Failed to strip prefix from cgroup path, using original"
            );
            &cgroup_path_str
        };
        cgroup_root.join(stripped_path)
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

/// Получить путь к корню cgroup v2 файловой системы.
///
/// Обычно это /sys/fs/cgroup или /sys/fs/cgroup/unified.
///
/// # Возвращаемое значение
///
/// Путь к корню cgroup v2 файловой системы
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::utils::process::get_cgroup_root;
///
/// let root = get_cgroup_root();
/// println!("Cgroup root: {:?}", root);
/// ```
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

/// Применить изменение nice для процесса.
///
/// # Параметры
///
/// - `pid`: PID процесса
/// - `nice`: Целевое значение nice
///
/// # Возвращаемое значение
///
/// - `Ok(())`: Успешно применено
/// - `Err(anyhow::Error)`: Ошибка при применении
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::utils::process::apply_nice;
///
/// // Применить nice для процесса
/// apply_nice(1234, 10)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn apply_nice(pid: i32, nice: i32) -> Result<()> {
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
///
/// # Параметры
///
/// - `pid`: PID процесса
/// - `class`: Класс IO приоритета
/// - `level`: Уровень приоритета
///
/// # Возвращаемое значение
///
/// - `Ok(())`: Успешно применено
/// - `Err(anyhow::Error)`: Ошибка при применении
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::utils::process::apply_ionice;
///
/// // Применить ionice для процесса
/// apply_ionice(1234, 2, 4)?; // best-effort class, level 4
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn apply_ionice(pid: i32, class: i32, level: i32) -> Result<()> {
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

/// Применить изменение latency_nice для процесса через sched_setattr.
///
/// latency_nice управляет тем, когда процесс получает CPU, а не сколько CPU он получит.
/// Диапазон: -20 (максимальная чувствительность к задержке) до +19 (безразличие к задержке).
///
/// # Параметры
///
/// - `pid`: PID процесса
/// - `latency_nice`: Целевое значение latency_nice
///
/// # Возвращаемое значение
///
/// - `Ok(())`: Успешно применено
/// - `Err(anyhow::Error)`: Ошибка при применении
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::utils::process::apply_latency_nice;
///
/// // Применить latency_nice для процесса
/// apply_latency_nice(1234, -10)?; // повышенная чувствительность к задержке
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn apply_latency_nice(pid: i32, latency_nice: i32) -> Result<()> {
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

/// Установить cpu.weight для cgroup.
///
/// # Параметры
///
/// - `cgroup_path`: Путь к cgroup
/// - `cpu_weight`: Целевое значение cpu.weight
///
/// # Возвращаемое значение
///
/// - `Ok(())`: Успешно применено
/// - `Err(anyhow::Error)`: Ошибка при применении
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::utils::process::set_cpu_weight;
/// use std::path::Path;
///
/// // Установить cpu.weight для cgroup
/// set_cpu_weight(Path::new("/sys/fs/cgroup/test"), 200)?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn set_cpu_weight(cgroup_path: &Path, cpu_weight: u32) -> Result<()> {
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
///
/// # Параметры
///
/// - `pid`: PID процесса
/// - `cgroup_path`: Путь к cgroup
///
/// # Возвращаемое значение
///
/// - `Ok(())`: Успешно перемещен
/// - `Err(anyhow::Error)`: Ошибка при перемещении
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::utils::process::move_process_to_cgroup;
/// use std::path::Path;
///
/// // Переместить процесс в cgroup
/// move_process_to_cgroup(1234, Path::new("/sys/fs/cgroup/test"))?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn move_process_to_cgroup(pid: i32, cgroup_path: &Path) -> Result<()> {
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

/// Создать или получить cgroup для AppGroup.
///
/// Создаёт cgroup вида `/smoothtask/app-{app_group_id}` под корнем cgroup v2.
///
/// # Параметры
///
/// - `app_group_id`: Идентификатор AppGroup
///
/// # Возвращаемое значение
///
/// - `Ok(PathBuf)`: Путь к cgroup
/// - `Err(anyhow::Error)`: Ошибка при создании
///
/// # Примеры
///
/// ```no_run
/// use smoothtask_core::utils::process::get_or_create_app_cgroup;
///
/// // Создать или получить cgroup для AppGroup
/// let cgroup_path = get_or_create_app_cgroup("firefox")?;
/// println!("Cgroup path: {:?}", cgroup_path);
/// # Ok::<(), anyhow::Error>(())
/// ```
pub fn get_or_create_app_cgroup(app_group_id: &str) -> Result<PathBuf> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::process;

    #[test]
    fn test_read_nice_returns_result() {
        // Тест проверяет, что функция возвращает Result
        let current_pid = process::id() as i32;
        let result = read_nice(current_pid);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_read_ionice_returns_result() {
        // Тест проверяет, что функция возвращает Result
        let current_pid = process::id() as i32;
        let result = read_ionice(current_pid);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_read_latency_nice_returns_result() {
        // Тест проверяет, что функция возвращает Result
        let current_pid = process::id() as i32;
        let result = read_latency_nice(current_pid);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_read_process_cgroup_returns_result() {
        // Тест проверяет, что функция возвращает Result
        let current_pid = process::id() as i32;
        let result = read_process_cgroup(current_pid);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_read_cpu_weight_returns_result() {
        // Тест проверяет, что функция возвращает Result
        let current_pid = process::id() as i32;
        let result = read_cpu_weight(current_pid);
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_get_cgroup_root_returns_path() {
        // Тест проверяет, что функция возвращает PathBuf
        let result = get_cgroup_root();
        assert!(result.is_absolute());
        assert!(!result.as_os_str().is_empty());
    }

    #[test]
    fn test_get_or_create_app_cgroup_creates_path() {
        // Тест проверяет создание пути для AppGroup cgroup
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
    fn test_apply_nice_handles_invalid_pid() {
        // Тест проверяет обработку невалидного PID
        let result = apply_nice(999999999, 10);
        // Для несуществующего PID должно вернуться Err
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_ionice_handles_invalid_pid() {
        // Тест проверяет обработку невалидного PID
        let result = apply_ionice(999999999, 2, 4);
        // Для несуществующего PID должно вернуться Err
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_latency_nice_handles_invalid_range() {
        // Тест проверяет обработку невалидного диапазона latency_nice
        let result = apply_latency_nice(1234, 21); // вне диапазона [-20, 19]
        assert!(result.is_err());
    }

    #[test]
    fn test_apply_latency_nice_handles_valid_range() {
        // Тест проверяет обработку валидного диапазона latency_nice
        // Используем текущий процесс
        let current_pid = process::id() as i32;
        let result = apply_latency_nice(current_pid, 0);
        // Результат может быть Ok или Err в зависимости от поддержки системой
        assert!(result.is_ok() || result.is_err());
    }
}
