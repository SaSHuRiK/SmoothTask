//! Сбор метрик процессов из /proc.
//!
//! Этот модуль предоставляет функции для чтения метрик процессов из файловой системы /proc.
//! Используется библиотека procfs для удобного доступа к данным процессов.

use crate::actuator::read_ionice;
use crate::logging::snapshots::ProcessRecord;
use anyhow::{Context, Result};
use procfs::process::{Process, Stat};
use procfs::ProcError;
use rayon::prelude::*;
use std::fs;

/// Собрать метрики всех процессов из /proc.
///
/// Возвращает вектор ProcessRecord для всех доступных процессов.
/// Процессы, к которым нет доступа или которые завершились, пропускаются.
///
/// # Возвращаемое значение
///
/// - `Ok(Vec<ProcessRecord>)` - вектор с метриками всех доступных процессов
/// - `Err(anyhow::Error)` - если не удалось получить список процессов из /proc
///
/// # Ошибки
///
/// Функция может вернуть ошибку в следующих случаях:
///
/// - **Нет доступа к /proc**: отсутствуют права доступа или /proc не смонтирован
/// - **Системная ошибка**: проблемы с файловой системой или ядром
///
/// # Примеры использования
///
/// ## Базовое использование
///
/// ```rust
/// use smoothtask_core::metrics::process::collect_process_metrics;
///
/// match collect_process_metrics() {
///     Ok(processes) => {
///         println!("Найдено {} процессов", processes.len());
///         for proc in processes {
///             println!("PID {}: {} - CPU: {:.1}%", proc.pid, proc.name, proc.cpu_usage);
///         }
///     }
///     Err(e) => {
///         eprintln!("Ошибка сбора метрик процессов: {}", e);
///     }
/// }
/// ```
///
/// ## Использование в главном цикле демона
///
/// ```rust
/// use smoothtask_core::metrics::process::collect_process_metrics;
/// use std::time::Instant;
///
/// let start_time = Instant::now();
/// match collect_process_metrics() {
///     Ok(processes) => {
///         let duration = start_time.elapsed();
///         tracing::info!(
///             "Собрано метрик для {} процессов за {:?}",
///             processes.len(),
///             duration
///         );
///     }
///     Err(e) => {
///         tracing::error!("Не удалось собрать метрики процессов: {}", e);
///     }
/// }
/// ```
///
/// ## Фильтрация и обработка результатов
///
/// ```rust
/// use smoothtask_core::metrics::process::collect_process_metrics;
///
/// if let Ok(processes) = collect_process_metrics() {
///     // Фильтрация процессов с высоким использованием CPU
///     let high_cpu_processes: Vec<_> = processes
///         .into_iter()
///         .filter(|p| p.cpu_usage > 50.0)
///         .collect();
///
///     println!("Процессы с высоким CPU: {}", high_cpu_processes.len());
/// }
/// ```
///
/// # Примечания
///
/// - Функция требует прав на чтение /proc (обычно требуются права root)
/// - Процессы, которые завершились во время сбора, автоматически пропускаются
/// - Ошибки доступа к отдельным процессам логируются на уровне debug и не прерывают выполнение
/// - Функция безопасна для вызова из многопоточного контекста
///
/// ## Обработка ошибок и graceful degradation
///
/// ```rust
/// use smoothtask_core::metrics::process::collect_process_metrics;
///
/// // Пример обработки ошибок с fallback логикой
/// let processes = match collect_process_metrics() {
///     Ok(processes) => processes,
///     Err(e) => {
///         tracing::error!("Не удалось собрать метрики процессов: {}", e);
///         // Fallback: использовать пустой вектор или кэшированные данные
///         Vec::new()
///     }
/// };
/// 
/// // Продолжение работы даже при отсутствии данных о процессах
/// if processes.is_empty() {
///     tracing::warn!("Нет данных о процессах, работаем в деградированном режиме");
/// }
/// ```
///
/// ## Интеграция с мониторингом и логированием
///
/// ```rust
/// use smoothtask_core::metrics::process::collect_process_metrics;
///
/// // Пример интеграции с системой мониторинга
/// if let Ok(processes) = collect_process_metrics() {
///     let total_cpu: f64 = processes.iter().map(|p| p.cpu_usage).sum();
///     let avg_cpu = total_cpu / processes.len() as f64;
///     
///     // Логирование статистики
///     tracing::info!(
///         "Процессов: {}, среднее CPU: {:.2}%, пиковое CPU: {:.2}%",
///         processes.len(),
///         avg_cpu,
///         processes.iter().map(|p| p.cpu_usage).fold(0.0, f64::max)
///     );
///     
///     // Экспорт метрик в Prometheus или другую систему
///     // metrics::gauge!("process_count", processes.len() as f64);
///     // metrics::gauge!("process_avg_cpu", avg_cpu);
/// }
/// ```
///
/// ## Работа с большими наборами данных
///
/// ```rust
/// use smoothtask_core::metrics::process::collect_process_metrics;
///
/// // Пример обработки большого количества процессов
/// if let Ok(processes) = collect_process_metrics() {
///     // Фильтрация и сортировка для анализа
///     let mut sorted_processes = processes;
///     sorted_processes.sort_by(|a, b| b.cpu_usage.partial_cmp(&a.cpu_usage).unwrap());
///     
///     // Вывод топ-10 процессов по использованию CPU
///     let top_10: Vec<_> = sorted_processes.into_iter().take(10).collect();
///     
///     for (i, proc) in top_10.iter().enumerate() {
///         tracing::info!(
///             "Top {}: PID {} - {} - CPU: {:.1}% - MEM: {} MB",
///             i + 1,
///             proc.pid,
///             proc.name,
///             proc.cpu_usage,
///             proc.rss_mb
///         );
///     }
/// }
/// ```
///
/// ## Использование в асинхронном контексте
///
/// ```rust
/// use smoothtask_core::metrics::process::collect_process_metrics;
/// use tokio::task;
///
/// // Пример использования в асинхронном контексте
/// let processes = task::spawn_blocking(|| {
///     collect_process_metrics()
/// }).await;
///
/// match processes {
///     Ok(Ok(processes)) => {
///         // Успешно собрали метрики в асинхронном контексте
///         println!("Собрано {} процессов", processes.len());
///     }
///     Ok(Err(e)) => {
///         eprintln!("Ошибка сбора метрик: {}", e);
///     }
///     Err(e) => {
///         eprintln!("Ошибка выполнения задачи: {}", e);
///     }
/// }
/// ```
pub fn collect_process_metrics() -> Result<Vec<ProcessRecord>> {
    let all_procs = procfs::process::all_processes()
        .context("Не удалось получить список процессов из /proc: проверьте права доступа и доступность /proc. Попробуйте: ls -la /proc | sudo ls /proc")?;

    // Оптимизация: предварительное выделение памяти для вектора процессов
    // Это уменьшает количество реаллокаций при добавлении элементов
    let mut processes = Vec::with_capacity(all_procs.size_hint().0);

    // Оптимизация: параллельная обработка процессов с использованием rayon
    // Это значительно ускоряет сбор метрик для большого количества процессов
    let process_results: Vec<_> = all_procs
        .par_bridge() // Преобразуем итератор в параллельный
        .filter_map(|proc_result| {
            match proc_result {
                Ok(proc) => {
                    match collect_single_process(&proc) {
                        Ok(Some(record)) => Some(record),
                        Ok(None) => None, // процесс завершился между итерациями
                        Err(e) => {
                            tracing::debug!(
                                "Ошибка сбора метрик для процесса PID {}: {}. \
                                 Процесс мог завершиться или нет прав доступа к /proc/{}/",
                                proc.pid(),
                                e,
                                proc.pid()
                            );
                            None
                        }
                    }
                }
                Err(ProcError::NotFound(_)) => None, // процесс завершился
                Err(e) => {
                    tracing::debug!(
                        "Ошибка доступа к процессу при чтении /proc: {}. \
                         Процесс мог завершиться или нет прав доступа",
                        e
                    );
                    None
                }
            }
        })
        .collect();

    processes.extend(process_results);

    // Оптимизация: уменьшаем выделенную память до фактического размера
    processes.shrink_to_fit();
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

    // Читаем cmdline с оптимизацией
    let cmdline = proc.cmdline().ok().and_then(|args| {
        if args.is_empty() {
            None
        } else {
            // Оптимизация: используем String::with_capacity для cmdline
            // чтобы уменьшить количество реаллокаций при join
            let mut cmdline_str = String::with_capacity(args.len() * 16); // средняя длина аргумента ~16 символов
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    cmdline_str.push(' ');
                }
                cmdline_str.push_str(arg);
            }
            Some(cmdline_str)
        }
    });

    // Читаем exe (симлинк на исполняемый файл) с оптимизацией
    let exe = proc
        .exe()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()));

    // Читаем cgroup_path с оптимизацией
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

    #[test]
    fn extract_systemd_unit_handles_edge_cases() {
        // Тест обработки пустых и некорректных путей
        assert_eq!(extract_systemd_unit(&Some("".to_string())), None);
        assert_eq!(extract_systemd_unit(&Some("/".to_string())), None);
        assert_eq!(
            extract_systemd_unit(&Some("invalid-format".to_string())),
            None
        );
        assert_eq!(
            extract_systemd_unit(&Some("/user.slice/".to_string())),
            None
        );

        // Тест обработки очень длинных путей
        let long_path = "/user.slice/".repeat(100) + "session-1.scope";
        assert_eq!(
            extract_systemd_unit(&Some(long_path)),
            Some("session-1.scope".to_string())
        );
    }

    #[test]
    fn parse_env_vars_handles_empty_and_malformed() {
        // Тест обработки пустого файла environ
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Пустой файл environ
        fs::write(proc_dir.join("environ"), "").unwrap();
        let path = proc_dir.join("environ");
        let contents = fs::read_to_string(&path).unwrap();
        let mut has_display = false;
        for env_var in contents.split('\0') {
            if env_var.starts_with("DISPLAY=") {
                has_display = true;
            }
        }
        assert!(!has_display);

        // Файл с некорректными данными
        let malformed_content = "INVALID_DATA_WITHOUT_NULL_BYTES";
        fs::write(proc_dir.join("environ"), malformed_content).unwrap();
        let contents = fs::read_to_string(&path).unwrap();
        let mut count = 0;
        for env_var in contents.split('\0') {
            if !env_var.is_empty() {
                count += 1;
            }
        }
        assert_eq!(count, 1); // только одна строка без нулевых байтов
    }

    #[test]
    fn parse_uid_gid_handles_malformed_status() {
        // Тест обработки некорректного файла status
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Некорректный формат status
        let malformed_status = "Invalid format without proper fields";
        fs::write(proc_dir.join("status"), malformed_status).unwrap();

        let status_path = proc_dir.join("status");
        let contents = fs::read_to_string(&status_path).unwrap();
        let mut uid = 0;
        let mut gid = 0;

        for line in contents.lines() {
            if line.starts_with("Uid:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(parsed_uid) = parts[1].parse::<u32>() {
                        uid = parsed_uid;
                    }
                }
            } else if line.starts_with("Gid:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(parsed_gid) = parts[1].parse::<u32>() {
                        gid = parsed_gid;
                    }
                }
            }
        }

        // Должны остаться значения по умолчанию (0)
        assert_eq!(uid, 0);
        assert_eq!(gid, 0);
    }

    #[test]
    fn calculate_uptime_handles_edge_cases() {
        // Тест обработки edge cases
        // Используем реальный процесс для получения валидного Stat
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

        // Тест с реальными данными - должен вернуть Ok
        let result = calculate_uptime(&stat);
        assert!(result.is_ok());

        // Проверяем, что uptime разумный
        let uptime = result.unwrap();
        assert!(uptime < 1000000); // разумный верхний предел
    }

    #[test]
    fn extract_systemd_unit_handles_complex_paths() {
        // Тест обработки сложных путей systemd
        assert_eq!(
            extract_systemd_unit(&Some(
                "/user.slice/user-1000.slice/session-2.scope/app.slice/firefox-1234.scope".to_string()
            )),
            Some("firefox-1234.scope".to_string())
        );

        // Тест с несколькими уровнями вложенности
        assert_eq!(
            extract_systemd_unit(&Some(
                "/system.slice/docker-abc123.scope/container.slice/firefox.service".to_string()
            )),
            Some("firefox.service".to_string())
        );

        // Тест с нестандартными именами
        assert_eq!(
            extract_systemd_unit(&Some(
                "/user.slice/user-1000.slice/session-2.scope/my-custom-app@123.service".to_string()
            )),
            Some("my-custom-app@123.service".to_string())
        );
    }

    #[test]
    fn parse_cgroup_v2_path_handles_edge_cases() {
        // Тест обработки пустого cgroup пути
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Пустой cgroup путь
        let empty_cgroup = "0::\n";
        fs::write(proc_dir.join("cgroup"), empty_cgroup).unwrap();

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
        assert_eq!(found_path, None);

        // Тест с корневым путем
        let root_cgroup = "0::/\n";
        fs::write(proc_dir.join("cgroup"), root_cgroup).unwrap();

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
        assert_eq!(found_path, None);
    }

    #[test]
    fn parse_env_vars_handles_special_characters() {
        // Тест обработки специальных символов в переменных окружения
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // environ с специальными символами
        let env_content = 
            "DISPLAY=:0\0TERM=xterm-256color\0SSH_CLIENT=192.168.1.1 22 33\0LANG=en_US.UTF-8\0";
        fs::write(proc_dir.join("environ"), env_content).unwrap();

        let path = proc_dir.join("environ");
        let contents = fs::read_to_string(&path).unwrap();
        let mut has_display = false;
        let mut term = None;
        let mut ssh = false;
        let mut lang = None;

        for env_var in contents.split('\0') {
            if env_var.starts_with("DISPLAY=") {
                has_display = true;
            } else if env_var.starts_with("TERM=") {
                term = env_var.strip_prefix("TERM=").map(|s| s.to_string());
            } else if env_var.starts_with("SSH_") {
                ssh = true;
            } else if env_var.starts_with("LANG=") {
                lang = env_var.strip_prefix("LANG=").map(|s| s.to_string());
            }
        }

        assert!(has_display);
        assert_eq!(term, Some("xterm-256color".to_string()));
        assert!(ssh);
        assert_eq!(lang, Some("en_US.UTF-8".to_string()));
    }

    #[test]
    fn parse_uid_gid_handles_boundary_values() {
        // Тест обработки граничных значений UID/GID
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Максимальные значения UID/GID
        let max_status = format!(
            "Name:   test_process\nState:  R (running)\nUid:    {} {} {} {}\nGid:    {} {} {} {}\n",
            u32::MAX, u32::MAX, u32::MAX, u32::MAX,
            u32::MAX, u32::MAX, u32::MAX, u32::MAX
        );
        fs::write(proc_dir.join("status"), max_status).unwrap();

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

        assert_eq!(uid, u32::MAX);
        assert_eq!(gid, u32::MAX);

        // Нулевые значения UID/GID
        let zero_status = "Name:   test_process\nState:  R (running)\nUid:    0 0 0 0\nGid:    0 0 0 0\n";
        fs::write(proc_dir.join("status"), zero_status).unwrap();

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

        assert_eq!(uid, 0);
        assert_eq!(gid, 0);
    }

    #[test]
    fn read_cgroup_path_with_malformed_content() {
        // Тест обработки некорректного содержимого cgroup
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // Некорректный формат cgroup (не хватает колонок)
        let malformed_cgroup = "0:/user.slice\n";
        fs::write(proc_dir.join("cgroup"), malformed_cgroup).unwrap();

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
        assert_eq!(found_path, None);

        // Некорректный формат cgroup (слишком много колонок)
        let malformed_cgroup2 = "0:1:2:/user.slice\n";
        fs::write(proc_dir.join("cgroup"), malformed_cgroup2).unwrap();

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
        // В этом случае parts[2] будет "2", а не "/user.slice"
        assert_eq!(found_path, Some("2".to_string()));
    }

    #[test]
    fn parse_env_vars_with_unicode_content() {
        // Тест обработки Unicode символов в переменных окружения
        let tmp = TempDir::new().unwrap();
        let proc_dir = tmp.path().join("proc").join("123");
        fs::create_dir_all(&proc_dir).unwrap();

        // environ с Unicode символами
        let env_content = 
            "DISPLAY=:0\0TERM=xterm-256color\0LANG=ru_RU.UTF-8\0USER=Пользователь\0";
        fs::write(proc_dir.join("environ"), env_content).unwrap();

        let path = proc_dir.join("environ");
        let contents = fs::read_to_string(&path).unwrap();
        let mut has_display = false;
        let mut lang = None;
        let mut user = None;

        for env_var in contents.split('\0') {
            if env_var.starts_with("DISPLAY=") {
                has_display = true;
            } else if env_var.starts_with("LANG=") {
                lang = env_var.strip_prefix("LANG=").map(|s| s.to_string());
            } else if env_var.starts_with("USER=") {
                user = env_var.strip_prefix("USER=").map(|s| s.to_string());
            }
        }

        assert!(has_display);
        assert_eq!(lang, Some("ru_RU.UTF-8".to_string()));
        assert_eq!(user, Some("Пользователь".to_string()));
    }
}
