//! Планирование и применение приоритетов (nice/ionice/cgroups).
//!
//! Этот модуль вычисляет, какие процессы требуют обновления приоритетов, и
//! применяет их через системные вызовы (setpriority, ioprio_set) и cgroups v2.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::Result;
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
    /// Текущий ionice (class, level), если известен.
    pub current_ionice: Option<(i32, i32)>,
    /// Целевой ionice.
    pub target_ionice: IoNiceParams,
    /// Причина из PolicyEngine (для логирования/трассировки).
    pub reason: String,
}

/// Построить список изменений приоритетов на основе результатов политики.
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
        let current_ionice = process.ionice_class.zip(process.ionice_prio);

        if needs_change(process.nice, current_ionice, params) {
            adjustments.push(PriorityAdjustment {
                pid: process.pid,
                app_group_id: app_group_id.clone(),
                target_class: policy.priority_class,
                current_nice: process.nice,
                target_nice: params.nice.nice,
                current_ionice,
                target_ionice: params.ionice,
                reason: policy.reason.clone(),
            });
        }
    }

    adjustments
}

fn needs_change(
    current_nice: i32,
    current_ionice: Option<(i32, i32)>,
    target: PriorityParams,
) -> bool {
    if current_nice != target.nice.nice {
        return true;
    }

    match current_ionice {
        Some((class, level)) => class != target.ionice.class || level != target.ionice.level,
        None => true,
    }
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

/// Применить изменение cgroup v2 для процесса или группы процессов.
///
/// Эта функция устанавливает cpu.weight для cgroup процесса.
/// Для полноценной реализации требуется работа с cgroups-rs и создание/управление cgroups.
/// Пока что это заглушка, которая логирует намерение.
fn apply_cgroup(_pid: i32, _cgroup_params: CgroupParams, _cgroup_path: Option<&str>) -> Result<()> {
    // TODO: Реализовать полноценное управление cgroups v2 через cgroups-rs
    // Для этого нужно:
    // 1. Определить cgroup процесса (из /proc/[pid]/cgroup)
    // 2. Создать или использовать существующий cgroup для AppGroup
    // 3. Установить cpu.weight через cgroups-rs
    // 4. Переместить процесс в нужный cgroup (если требуется)

    debug!(
        pid = _pid,
        cpu_weight = _cgroup_params.cpu_weight,
        cgroup_path = ?_cgroup_path,
        "Cgroup adjustment requested (not yet implemented)"
    );

    // Пока что просто возвращаем Ok, так как полная реализация требует больше работы
    // и может быть добавлена в отдельной задаче
    Ok(())
}

/// Результат применения изменений приоритетов.
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
/// Применяет nice и ionice для каждого процесса из списка adjustments,
/// учитывая гистерезис для предотвращения частых изменений.
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
                "Skipping change due to hysteresis"
            );
            result.skipped_hysteresis += 1;
            continue;
        }

        // Применяем nice
        if let Err(e) = apply_nice(adj.pid, adj.target_nice) {
            warn!(
                pid = adj.pid,
                error = %e,
                "Failed to apply nice"
            );
            result.errors += 1;
            continue;
        }

        // Применяем ionice
        if let Err(e) = apply_ionice(adj.pid, adj.target_ionice.class, adj.target_ionice.level) {
            warn!(
                pid = adj.pid,
                error = %e,
                "Failed to apply ionice"
            );
            result.errors += 1;
            continue;
        }

        // Применяем cgroup (пока что только логируем)
        let cgroup_params = adj.target_class.params().cgroup;
        if let Err(e) = apply_cgroup(adj.pid, cgroup_params, None) {
            warn!(
                pid = adj.pid,
                error = %e,
                "Failed to apply cgroup"
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
        assert_eq!(adj.target_ionice, PriorityClass::Interactive.ionice());
        assert_eq!(adj.current_nice, 0);
        assert_eq!(adj.current_ionice, None);
    }

    #[test]
    fn skips_when_priorities_already_match() {
        let mut process = base_process("app1", 1);
        process.nice = PriorityClass::Background.nice();
        let ionice = PriorityClass::Background.ionice();
        process.ionice_class = Some(ionice.class);
        process.ionice_prio = Some(ionice.level);

        let snapshot = make_snapshot(vec![process], vec![app_group("app1")]);

        let mut policy_results = HashMap::new();
        policy_results.insert(
            "app1".to_string(),
            make_policy_result(PriorityClass::Background, "batch task"),
        );

        let adjustments = plan_priority_changes(&snapshot, &policy_results);
        assert!(adjustments.is_empty());
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
    fn class_order_correct() {
        assert_eq!(class_order(PriorityClass::CritInteractive), 5);
        assert_eq!(class_order(PriorityClass::Interactive), 4);
        assert_eq!(class_order(PriorityClass::Normal), 3);
        assert_eq!(class_order(PriorityClass::Background), 2);
        assert_eq!(class_order(PriorityClass::Idle), 1);
    }
}
