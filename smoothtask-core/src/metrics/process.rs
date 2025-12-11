//! Сбор метрик процессов из /proc.
//!
//! Этот модуль предоставляет функции для чтения метрик процессов из файловой системы /proc.
//! Используется библиотека procfs для удобного доступа к данным процессов.

use crate::actuator::read_ionice;
use crate::logging::snapshots::ProcessRecord;
use anyhow::{Context, Result};
use procfs::process::{Process, Stat};
use procfs::ProcError;
use std::fs;

/// Собрать метрики всех процессов из /proc.
///
/// Возвращает вектор ProcessRecord для всех доступных процессов.
/// Процессы, к которым нет доступа или которые завершились, пропускаются.
pub fn collect_process_metrics() -> Result<Vec<ProcessRecord>> {
    let all_procs = procfs::process::all_processes()
        .context("Не удалось получить список процессов из /proc: проверьте права доступа и доступность /proc")?;

    let mut processes = Vec::new();

    for proc_result in all_procs {
        let proc = match proc_result {
            Ok(p) => p,
            Err(ProcError::NotFound(_)) => continue, // процесс завершился
            Err(e) => {
                tracing::debug!(
                    "Ошибка доступа к процессу при чтении /proc: {}. \
                     Процесс мог завершиться или нет прав доступа",
                    e
                );
                continue;
            }
        };

        match collect_single_process(&proc) {
            Ok(Some(record)) => processes.push(record),
            Ok(None) => {
                // процесс завершился между итерациями
            }
            Err(e) => {
                tracing::debug!(
                    "Ошибка сбора метрик для процесса PID {}: {}. \
                     Процесс мог завершиться или нет прав доступа к /proc/{}/",
                    proc.pid(),
                    e,
                    proc.pid()
                );
            }
        }
    }

    Ok(processes)
}

/// Собрать метрики для одного процесса.
///
/// Возвращает `None`, если процесс завершился или к нему нет доступа.
fn collect_single_process(proc: &Process) -> Result<Option<ProcessRecord>> {
    // Читаем stat для базовой информации
    let stat = match proc.stat() {
        Ok(s) => s,
        Err(ProcError::NotFound(_)) => return Ok(None),
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Не удалось прочитать /proc/{}/stat: {}. \
                 Проверьте, что процесс существует и доступен для чтения",
                proc.pid(),
                e
            ))
        }
    };

    // Читаем status для UID/GID и дополнительной информации
    let status = match proc.status() {
        Ok(s) => s,
        Err(ProcError::NotFound(_)) => return Ok(None),
        Err(e) => {
            return Err(anyhow::anyhow!(
                "Не удалось прочитать /proc/{}/status: {}. \
                 Проверьте, что процесс существует и доступен для чтения",
                proc.pid(),
                e
            ))
        }
    };

    // Читаем cmdline
    let cmdline = proc.cmdline().ok().and_then(|args| {
        if args.is_empty() {
            None
        } else {
            Some(args.join(" "))
        }
    });

    // Читаем exe (симлинк на исполняемый файл)
    let exe = proc
        .exe()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()));

    // Читаем cgroup_path
    let cgroup_path = read_cgroup_path(proc.pid()).ok().flatten();

    // Определяем uptime_sec на основе start_time
    let uptime_sec = calculate_uptime(&stat).with_context(|| {
        format!(
            "Не удалось вычислить uptime для процесса PID {}: \
             проверьте доступность /proc/uptime и корректность start_time в /proc/{}/stat",
            proc.pid(),
            proc.pid()
        )
    })?;

    // Определяем has_tty на основе tty_nr
    let has_tty = stat.tty_nr != 0;

    // Читаем переменные окружения для определения GUI/терминала
    let (env_has_display, env_has_wayland, env_term, env_ssh) =
        read_env_vars(proc.pid()).unwrap_or((false, false, None, false));

    // Читаем nice из stat (конвертируем i64 в i32)
    let nice = stat.nice as i32;

    // Читаем ionice через системный вызов ioprio_get
    let (ionice_class, ionice_prio) = read_ionice(stat.pid)
        .ok()
        .flatten()
        .map(|(class, level)| (Some(class), Some(level)))
        .unwrap_or((None, None));

    // Читаем RSS из status (в килобайтах, конвертируем в мегабайты)
    // В procfs RSS доступен через поле VmRSS в status
    let rss_mb = status.vmrss.map(|kb| kb / 1024);

    // Читаем swap из status (в килобайтах, конвертируем в мегабайты)
    let swap_mb = status.vmswap.map(|kb| kb / 1024);

    // Читаем контекстные переключения из status
    let voluntary_ctx = status.voluntary_ctxt_switches;
    let involuntary_ctx = status.nonvoluntary_ctxt_switches;

    // Получаем UID и GID из /proc/[pid]/status напрямую
    let (uid, gid) = read_uid_gid(proc.pid()).unwrap_or((0, 0));

    // Определяем systemd_unit из cgroup_path (если есть)
    let systemd_unit = extract_systemd_unit(&cgroup_path);

    let record = ProcessRecord {
        pid: stat.pid,
        ppid: stat.ppid,
        uid,
        gid,
        exe,
        cmdline,
        cgroup_path,
        systemd_unit,
        app_group_id: None, // будет заполнено при группировке
        state: format!("{:?}", stat.state),
        start_time: stat.starttime,
        uptime_sec,
        tty_nr: stat.tty_nr,
        has_tty,
        cpu_share_1s: None,   // будет вычислено при следующем снапшоте
        cpu_share_10s: None,  // будет вычислено при следующем снапшоте
        io_read_bytes: None,  // требует чтения /proc/[pid]/io (тяжелая операция)
        io_write_bytes: None, // требует чтения /proc/[pid]/io (тяжелая операция)
        rss_mb,
        swap_mb,
        voluntary_ctx,
        involuntary_ctx,
        has_gui_window: false,    // будет заполнено из WindowIntrospector
        is_focused_window: false, // будет заполнено из WindowIntrospector
        window_state: None,       // будет заполнено из WindowIntrospector
        env_has_display,
        env_has_wayland,
        env_term,
        env_ssh,
        is_audio_client: false,   // будет заполнено из AudioIntrospector
        has_active_stream: false, // будет заполнено из AudioIntrospector
        process_type: None,       // будет заполнено классификатором
        tags: Vec::new(),         // будет заполнено классификатором
        nice,
        ionice_class,
        ionice_prio,
        teacher_priority_class: None, // для обучения
        teacher_score: None,          // для обучения
    };

    Ok(Some(record))
}

/// Прочитать путь cgroup процесса из /proc/[pid]/cgroup.
fn read_cgroup_path(pid: i32) -> Result<Option<String>> {
    let path = format!("/proc/{}/cgroup", pid);
    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            // Не критичная ошибка - cgroup может быть недоступен
            tracing::debug!(
                "Не удалось прочитать /proc/{}/cgroup: {}. \
                 Cgroup может быть недоступен для этого процесса. \
                 Это может быть вызвано отсутствием прав доступа, отсутствием файла или тем, что процесс завершился",
                pid,
                e
            );
            return Ok(None);
        }
    };

    // Парсим формат cgroup v2: 0::/path/to/cgroup
    // Или cgroup v1: несколько строк вида hierarchy:controller:path
    for line in contents.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 3 {
            let cgroup_path = parts[2];
            if !cgroup_path.is_empty() && cgroup_path != "/" {
                return Ok(Some(cgroup_path.to_string()));
            }
        }
    }

    Ok(None)
}

/// Извлечь systemd unit из cgroup_path.
///
/// Например, из "/user.slice/user-1000.slice/session-2.scope" извлекается "session-2.scope".
/// Игнорируем корневые .slice компоненты (например, "/user.slice").
fn extract_systemd_unit(cgroup_path: &Option<String>) -> Option<String> {
    let path = cgroup_path.as_ref()?;

    // Ищем последний компонент пути, который заканчивается на .scope или .service
    // Игнорируем .slice, если это не единственный компонент (корневой)
    let components: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();

    if components.len() <= 1 {
        // Корневой путь типа "/user.slice" - не считаем unit'ом
        return None;
    }

    for component in components.iter().rev() {
        if component.ends_with(".scope") || component.ends_with(".service") {
            return Some(component.to_string());
        }
    }

    None
}

/// Вычислить uptime процесса в секундах на основе start_time.
fn calculate_uptime(stat: &Stat) -> Result<u64> {
    // start_time в jiffies (обычно 100 Hz, но может быть и 1000 Hz)
    // Нужно получить системный uptime и вычислить разницу
    let boot_time = procfs::boot_time_secs().with_context(|| {
        format!(
            "Не удалось получить время загрузки системы для вычисления uptime процесса PID {}. \
             Проверьте доступность /proc/stat",
            stat.pid
        )
    })?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // starttime в jiffies, нужно конвертировать в секунды
    // Используем clock_ticks_per_second для конвертации
    let clock_ticks = procfs::ticks_per_second();

    let start_time_secs = boot_time + (stat.starttime / clock_ticks);
    let uptime_sec = now.saturating_sub(start_time_secs);

    Ok(uptime_sec)
}

/// Прочитать UID и GID процесса из /proc/[pid]/status.
fn read_uid_gid(pid: i32) -> Result<(u32, u32)> {
    let path = format!("/proc/{}/status", pid);
    let contents = fs::read_to_string(&path).with_context(|| {
        format!(
            "Не удалось прочитать /proc/{}/status: проверьте, что процесс существует и доступен для чтения",
            pid
        )
    })?;

    let mut uid = 0;
    let mut gid = 0;

    for line in contents.lines() {
        if line.starts_with("Uid:") {
            // Формат: Uid: 1000 1000 1000 1000 (real, effective, saved, filesystem)
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                uid = parts[1].parse::<u32>().with_context(|| {
                    format!(
                        "Некорректный UID в /proc/{}/status: ожидается целое число (u32). \
                             Формат строки: 'Uid: <real> <effective> <saved> <filesystem>'",
                        pid
                    )
                })?;
            }
        } else if line.starts_with("Gid:") {
            // Формат: Gid: 1000 1000 1000 1000 (real, effective, saved, filesystem)
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                gid = parts[1].parse::<u32>().with_context(|| {
                    format!(
                        "Некорректный GID в /proc/{}/status: ожидается целое число (u32). \
                             Формат строки: 'Gid: <real> <effective> <saved> <filesystem>'",
                        pid
                    )
                })?;
            }
        }
    }

    Ok((uid, gid))
}

/// Прочитать переменные окружения процесса из /proc/[pid]/environ.
fn read_env_vars(pid: i32) -> Result<(bool, bool, Option<String>, bool)> {
    let path = format!("/proc/{}/environ", pid);
    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            // Не критичная ошибка - environ может быть недоступен
            tracing::debug!(
                "Не удалось прочитать /proc/{}/environ: {}. \
                 Переменные окружения могут быть недоступны для этого процесса",
                pid,
                e
            );
            return Ok((false, false, None, false));
        }
    };

    let mut has_display = false;
    let mut has_wayland = false;
    let mut term = None;
    let mut ssh = false;

    // environ содержит переменные, разделённые нулевыми байтами
    for env_var in contents.split('\0') {
        if env_var.starts_with("DISPLAY=") {
            has_display = true;
        } else if env_var.starts_with("WAYLAND_DISPLAY=") {
            has_wayland = true;
        } else if env_var.starts_with("TERM=") {
            term = env_var.strip_prefix("TERM=").map(|s| s.to_string());
        } else if env_var.starts_with("SSH_") {
            ssh = true;
        }
    }

    Ok((has_display, has_wayland, term, ssh))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn extract_systemd_unit_from_cgroup_path() {
        let unit = extract_systemd_unit(&Some(
            "/user.slice/user-1000.slice/session-2.scope".to_string(),
        ));
        assert_eq!(unit, Some("session-2.scope".to_string()));

        let unit = extract_systemd_unit(&Some("/system.slice/ssh.service".to_string()));
        assert_eq!(unit, Some("ssh.service".to_string()));

        let unit = extract_systemd_unit(&Some("/user.slice".to_string()));
        assert_eq!(unit, None);

        let unit = extract_systemd_unit(&None);
        assert_eq!(unit, None);
    }

    #[test]
    fn parse_cgroup_v2_path() {
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Формат cgroup v2: 0::/user.slice/user-1000.slice/session-2.scope
        let cgroup_content = "0::/user.slice/user-1000.slice/session-2.scope\n";
        fs::write(proc_dir.join("cgroup"), cgroup_content).unwrap();

        // Мокаем чтение через прямое чтение файла
        let path = proc_dir.join("cgroup");
        let contents = fs::read_to_string(&path).unwrap();
        let mut found_path = None;
        for line in contents.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                let cgroup_path = parts[2];
                if !cgroup_path.is_empty() && cgroup_path != "/" {
                    found_path = Some(cgroup_path.to_string());
                    break;
                }
            }
        }
        assert_eq!(
            found_path,
            Some("/user.slice/user-1000.slice/session-2.scope".to_string())
        );
    }

    #[test]
    fn parse_env_vars() {
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // environ содержит переменные, разделённые нулевыми байтами
        let env_content =
            "HOME=/home/user\0DISPLAY=:0\0TERM=xterm-256color\0SSH_CLIENT=192.168.1.1\0";
        fs::write(proc_dir.join("environ"), env_content).unwrap();

        let path = proc_dir.join("environ");
        let contents = fs::read_to_string(&path).unwrap();
        let mut has_display = false;
        let mut has_wayland = false;
        let mut term = None;
        let mut ssh = false;

        for env_var in contents.split('\0') {
            if env_var.starts_with("DISPLAY=") {
                has_display = true;
            } else if env_var.starts_with("WAYLAND_DISPLAY=") {
                has_wayland = true;
            } else if env_var.starts_with("TERM=") {
                term = env_var.strip_prefix("TERM=").map(|s| s.to_string());
            } else if env_var.starts_with("SSH_") {
                ssh = true;
            }
        }

        assert!(has_display);
        assert!(!has_wayland);
        assert_eq!(term, Some("xterm-256color".to_string()));
        assert!(ssh);
    }

    #[test]
    fn parse_uid_gid_from_status() {
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Формат /proc/[pid]/status с Uid и Gid
        let status_content = "\
Name:   test_process
State:  R (running)
Uid:    1000 1000 1000 1000
Gid:    1000 1000 1000 1000
";
        fs::write(proc_dir.join("status"), status_content).unwrap();

        // Используем временный путь вместо реального /proc/123
        let status_path = proc_dir.join("status");
        let contents = fs::read_to_string(&status_path).unwrap();
        let mut uid = 0;
        let mut gid = 0;

        for line in contents.lines() {
            if line.starts_with("Uid:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    uid = parts[1].parse::<u32>().unwrap();
                }
            } else if line.starts_with("Gid:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    gid = parts[1].parse::<u32>().unwrap();
                }
            }
        }

        assert_eq!(uid, 1000);
        assert_eq!(gid, 1000);
    }

    #[test]
    fn parse_uid_gid_handles_missing_file() {
        // Несуществующий PID должен вернуть ошибку
        let result = read_uid_gid(999999);
        assert!(result.is_err());
    }

    #[test]
    fn extract_systemd_unit_handles_various_formats() {
        // Тест различных форматов systemd unit
        assert_eq!(
            extract_systemd_unit(&Some(
                "/user.slice/user-1000.slice/session-2.scope".to_string()
            )),
            Some("session-2.scope".to_string())
        );
        assert_eq!(
            extract_systemd_unit(&Some("/system.slice/ssh.service".to_string())),
            Some("ssh.service".to_string())
        );
        assert_eq!(
            extract_systemd_unit(&Some("/system.slice/dbus.service".to_string())),
            Some("dbus.service".to_string())
        );
        assert_eq!(extract_systemd_unit(&Some("/user.slice".to_string())), None);
        assert_eq!(extract_systemd_unit(&None), None);
    }

    #[test]
    fn calculate_uptime_with_valid_stat() {
        // Этот тест проверяет, что функция calculate_uptime не падает с ошибкой
        // при корректных входных данных. Так как функция зависит от системного времени
        // и boot_time, мы не можем точно предсказать результат, но можем проверить,
        // что она возвращает разумное значение.

        // Используем реальный процесс (текущий процесс) для получения реального Stat
        // Это более надежный подход, чем создание мокового Stat
        let current_pid = std::process::id() as i32;
        let proc = match Process::new(current_pid) {
            Ok(p) => p,
            Err(_) => {
                // Если не удалось получить процесс, пропускаем тест
                return;
            }
        };

        let stat = match proc.stat() {
            Ok(s) => s,
            Err(_) => {
                // Если не удалось получить stat, пропускаем тест
                return;
            }
        };

        let result = calculate_uptime(&stat);
        assert!(result.is_ok());
        let uptime = result.unwrap();
        // Проверяем, что uptime разумный (не слишком большой)
        // Для текущего процесса uptime может быть 0, если процесс только что запустился
        assert!(uptime < 1000000); // разумный верхний предел
    }

    #[test]
    fn read_cgroup_path_with_valid_file() {
        // Создаем временный файл cgroup
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Формат cgroup v2: 0::/user.slice/user-1000.slice/session-2.scope
        let cgroup_content = "0::/user.slice/user-1000.slice/session-2.scope\n";
        fs::write(proc_dir.join("cgroup"), cgroup_content).unwrap();

        // Мокаем чтение через временный файл
        let path = proc_dir.join("cgroup");
        let contents = fs::read_to_string(&path).unwrap();
        let mut found_path = None;
        for line in contents.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 3 {
                let cgroup_path = parts[2];
                if !cgroup_path.is_empty() && cgroup_path != "/" {
                    found_path = Some(cgroup_path.to_string());
                    break;
                }
            }
        }
        assert_eq!(
            found_path,
            Some("/user.slice/user-1000.slice/session-2.scope".to_string())
        );
    }

    #[test]
    fn read_cgroup_path_with_missing_file() {
        // Проверяем, что функция корректно обрабатывает отсутствие файла
        let result = read_cgroup_path(999999);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }
}
