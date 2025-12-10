//! Policy Engine — применение правил для определения приоритетов AppGroup.
//!
//! Policy Engine применяет жёсткие правила (guardrails) и семантические правила
//! для определения целевого класса приоритета для каждой AppGroup в снапшоте.

use crate::config::Config;
use crate::logging::snapshots::{AppGroupRecord, ProcessRecord, Snapshot};
use crate::policy::classes::PriorityClass;

/// Результат оценки политики для одной AppGroup.
#[derive(Debug, Clone)]
pub struct PolicyResult {
    /// Целевой класс приоритета для группы.
    pub priority_class: PriorityClass,
    /// Причина выбора приоритета (для логирования и отладки).
    pub reason: String,
}

/// Policy Engine для применения правил к снапшоту.
pub struct PolicyEngine {
    config: Config,
}

impl PolicyEngine {
    /// Создать новый Policy Engine с заданной конфигурацией.
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Оценить снапшот и определить приоритеты для всех AppGroup.
    ///
    /// # Аргументы
    ///
    /// * `snapshot` - снапшот системы с процессами и группами
    ///
    /// # Возвращает
    ///
    /// Маппинг app_group_id -> PolicyResult с приоритетом и причиной.
    pub fn evaluate_snapshot(
        &self,
        snapshot: &Snapshot,
    ) -> std::collections::HashMap<String, PolicyResult> {
        let mut results = std::collections::HashMap::new();

        for app_group in &snapshot.app_groups {
            let result = self.evaluate_app_group(app_group, snapshot);
            results.insert(app_group.app_group_id.clone(), result);
        }

        results
    }

    /// Оценить одну AppGroup и определить её приоритет.
    fn evaluate_app_group(&self, app_group: &AppGroupRecord, snapshot: &Snapshot) -> PolicyResult {
        // 1. Применяем жёсткие правила (guardrails)
        if let Some(guardrail_result) = self.apply_guardrails(app_group, snapshot) {
            return guardrail_result;
        }

        // 2. Применяем семантические правила
        if let Some(semantic_result) = self.apply_semantic_rules(app_group, snapshot) {
            return semantic_result;
        }

        // 3. Дефолтный приоритет (если правила не применились)
        PolicyResult {
            priority_class: PriorityClass::Normal,
            reason: "default: no rules matched".to_string(),
        }
    }

    /// Применить жёсткие правила (guardrails).
    ///
    /// Эти правила имеют наивысший приоритет и не могут быть переопределены.
    fn apply_guardrails(
        &self,
        app_group: &AppGroupRecord,
        snapshot: &Snapshot,
    ) -> Option<PolicyResult> {
        // Правило 1: Не трогать системные процессы
        if self.is_system_process_group(app_group, snapshot) {
            return Some(PolicyResult {
                priority_class: PriorityClass::Normal,
                reason: "guardrail: system process, leaving unchanged".to_string(),
            });
        }

        // Правило 2: Защита аудио
        if self.is_audio_client_with_xrun(app_group, snapshot) {
            return Some(PolicyResult {
                priority_class: PriorityClass::Interactive,
                reason: "guardrail: audio client with XRUN, protecting".to_string(),
            });
        }

        // Правило 3: Ограничение batch-групп (проверяем, что не превышаем лимит)
        // Это правило сложнее, так как требует знания о других группах
        // Пока пропускаем, можно добавить позже

        None
    }

    /// Применить семантические правила.
    ///
    /// Эти правила определяют приоритет на основе контекста и метрик.
    fn apply_semantic_rules(
        &self,
        app_group: &AppGroupRecord,
        snapshot: &Snapshot,
    ) -> Option<PolicyResult> {
        // Правило 1: Критически интерактивные процессы (фокус + аудио/игра)
        // Проверяем сначала более специфичные правила
        if app_group.is_focused_group
            && (self.has_audio_client(app_group, snapshot) || self.is_game(app_group, snapshot))
        {
            return Some(PolicyResult {
                priority_class: PriorityClass::CritInteractive,
                reason: "semantic: focused group with audio/game".to_string(),
            });
        }

        // Правило 2: Фокусный GUI-AppGroup всегда ≥ INTERACTIVE
        if app_group.is_focused_group && app_group.has_gui_window {
            return Some(PolicyResult {
                priority_class: PriorityClass::Interactive,
                reason: "semantic: focused GUI group".to_string(),
            });
        }

        // Правило 3: Активный терминал ≥ свернутым batch-процессам
        if self.is_active_terminal(app_group, snapshot) {
            return Some(PolicyResult {
                priority_class: PriorityClass::Interactive,
                reason: "semantic: active terminal with recent input".to_string(),
            });
        }

        // Правило 4: Updater/indexer при активном пользователе → максимум BACKGROUND/IDLE
        if snapshot.global.user_active && self.is_updater_or_indexer(app_group) {
            return Some(PolicyResult {
                priority_class: PriorityClass::Background,
                reason: "semantic: updater/indexer with active user".to_string(),
            });
        }

        // Правило 5: Noisy neighbour — если группа жрёт CPU, а отзывчивость падает
        if snapshot.responsiveness.bad_responsiveness
            && self.is_noisy_neighbour(app_group, snapshot)
        {
            return Some(PolicyResult {
                priority_class: PriorityClass::Background,
                reason: "semantic: noisy neighbour throttling".to_string(),
            });
        }

        None
    }

    /// Проверить, является ли группа системным процессом.
    fn is_system_process_group(&self, app_group: &AppGroupRecord, snapshot: &Snapshot) -> bool {
        // Находим процессы группы
        let group_processes: Vec<&ProcessRecord> = snapshot
            .processes
            .iter()
            .filter(|p| p.app_group_id.as_deref() == Some(app_group.app_group_id.as_str()))
            .collect();

        for process in group_processes {
            // Проверяем по exe
            if let Some(ref exe) = process.exe {
                let exe_lower = exe.to_lowercase();
                if exe_lower.contains("systemd")
                    || exe_lower.contains("journald")
                    || exe_lower.contains("udevd")
                    || exe_lower.contains("kernel")
                {
                    return true;
                }
            }

            // Проверяем по cgroup_path (системные cgroups)
            if let Some(ref cgroup) = process.cgroup_path {
                if cgroup.starts_with("/system.slice") || cgroup.starts_with("/sys/fs/cgroup") {
                    // Но не все системные процессы должны быть защищены
                    // Проверяем только критичные
                    if cgroup.contains("systemd") || cgroup.contains("kernel") {
                        return true;
                    }
                }
            }
        }

        false
    }

    /// Проверить, является ли группа аудио-клиентом с XRUN.
    fn is_audio_client_with_xrun(&self, app_group: &AppGroupRecord, snapshot: &Snapshot) -> bool {
        // Проверяем, есть ли XRUN события
        if snapshot.responsiveness.audio_xruns_delta.unwrap_or(0) == 0 {
            return false;
        }

        // Проверяем, есть ли в группе аудио-клиенты
        self.has_audio_client(app_group, snapshot)
    }

    /// Проверить, есть ли в группе аудио-клиенты.
    fn has_audio_client(&self, app_group: &AppGroupRecord, snapshot: &Snapshot) -> bool {
        snapshot
            .processes
            .iter()
            .filter(|p| p.app_group_id.as_deref() == Some(app_group.app_group_id.as_str()))
            .any(|p| p.is_audio_client && p.has_active_stream)
    }

    /// Проверить, является ли группа активным терминалом.
    fn is_active_terminal(&self, app_group: &AppGroupRecord, snapshot: &Snapshot) -> bool {
        // Проверяем, что пользователь активен
        if !snapshot.global.user_active {
            return false;
        }

        // Проверяем время с последнего ввода
        if let Some(time_since_input) = snapshot.global.time_since_last_input_ms {
            if time_since_input > self.config.thresholds.user_idle_timeout_sec * 1000 {
                return false;
            }
        }

        // Проверяем, есть ли в группе процессы с TTY
        snapshot
            .processes
            .iter()
            .filter(|p| p.app_group_id.as_deref() == Some(app_group.app_group_id.as_str()))
            .any(|p| p.has_tty && p.env_term.is_some())
    }

    /// Проверить, является ли группа updater'ом или indexer'ом.
    fn is_updater_or_indexer(&self, app_group: &AppGroupRecord) -> bool {
        // Проверяем по тегам
        app_group
            .tags
            .iter()
            .any(|tag| tag == "updater" || tag == "indexer" || tag == "maintenance")
    }

    /// Проверить, является ли группа "noisy neighbour".
    fn is_noisy_neighbour(&self, app_group: &AppGroupRecord, _snapshot: &Snapshot) -> bool {
        // Проверяем CPU usage группы
        if let Some(cpu_share) = app_group.total_cpu_share {
            if cpu_share > self.config.thresholds.noisy_neighbour_cpu_share as f64 {
                // Дополнительно проверяем, что группа не в фокусе
                if !app_group.is_focused_group {
                    return true;
                }
            }
        }

        false
    }

    /// Проверить, является ли группа игрой.
    fn is_game(&self, app_group: &AppGroupRecord, _snapshot: &Snapshot) -> bool {
        app_group.tags.iter().any(|tag| tag == "game")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Paths, Thresholds};
    use crate::logging::snapshots::{GlobalMetrics, ResponsivenessMetrics};
    use chrono::Utc;

    fn create_test_config() -> Config {
        Config {
            polling_interval_ms: 500,
            max_candidates: 150,
            dry_run_default: false,
            thresholds: Thresholds {
                psi_cpu_some_high: 0.6,
                psi_io_some_high: 0.4,
                user_idle_timeout_sec: 120,
                interactive_build_grace_sec: 10,
                noisy_neighbour_cpu_share: 0.7,
                crit_interactive_percentile: 0.9,
                interactive_percentile: 0.6,
                normal_percentile: 0.3,
                background_percentile: 0.1,
            },
            paths: Paths {
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
            },
        }
    }

    fn create_test_snapshot() -> Snapshot {
        Snapshot {
            snapshot_id: 1234567890,
            timestamp: Utc::now(),
            global: GlobalMetrics {
                cpu_user: 0.25,
                cpu_system: 0.15,
                cpu_idle: 0.55,
                cpu_iowait: 0.05,
                mem_total_kb: 16_384_256,
                mem_used_kb: 8_000_000,
                mem_available_kb: 8_384_256,
                swap_total_kb: 8_192_000,
                swap_used_kb: 1_000_000,
                load_avg_one: 1.5,
                load_avg_five: 1.2,
                load_avg_fifteen: 1.0,
                psi_cpu_some_avg10: Some(0.1),
                psi_cpu_some_avg60: Some(0.15),
                psi_io_some_avg10: Some(0.2),
                psi_mem_some_avg10: Some(0.05),
                psi_mem_full_avg10: None,
                user_active: true,
                time_since_last_input_ms: Some(5000),
            },
            processes: vec![],
            app_groups: vec![],
            responsiveness: ResponsivenessMetrics::default(),
        }
    }

    #[test]
    fn test_focused_gui_group_gets_interactive() {
        let config = create_test_config();
        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        let app_group = AppGroupRecord {
            app_group_id: "firefox".to_string(),
            root_pid: 1000,
            process_ids: vec![1000],
            app_name: Some("firefox".to_string()),
            total_cpu_share: Some(0.1),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_rss_mb: Some(500),
            has_gui_window: true,
            is_focused_group: true,
            tags: vec!["browser".to_string()],
            priority_class: None,
        };

        snapshot.app_groups = vec![app_group.clone()];

        let results = engine.evaluate_snapshot(&snapshot);
        let result = results.get("firefox").unwrap();

        assert_eq!(result.priority_class, PriorityClass::Interactive);
        assert!(result.reason.contains("focused GUI"));
    }

    #[test]
    fn test_system_process_protected() {
        let config = create_test_config();
        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        let system_process = ProcessRecord {
            pid: 1,
            ppid: 0,
            uid: 0,
            gid: 0,
            exe: Some("/usr/lib/systemd/systemd".to_string()),
            cmdline: None,
            cgroup_path: Some("/system.slice/systemd.service".to_string()),
            systemd_unit: Some("systemd.service".to_string()),
            app_group_id: Some("systemd".to_string()),
            state: "S".to_string(),
            start_time: 0,
            uptime_sec: 1000,
            tty_nr: 0,
            has_tty: false,
            cpu_share_1s: None,
            cpu_share_10s: None,
            io_read_bytes: None,
            io_write_bytes: None,
            rss_mb: Some(50),
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
            process_type: Some("daemon".to_string()),
            tags: vec![],
            nice: 0,
            ionice_class: Some(2),
            ionice_prio: Some(4),
            teacher_priority_class: None,
            teacher_score: None,
        };

        let app_group = AppGroupRecord {
            app_group_id: "systemd".to_string(),
            root_pid: 1,
            process_ids: vec![1],
            app_name: Some("systemd".to_string()),
            total_cpu_share: Some(0.05),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_rss_mb: Some(50),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
        };

        snapshot.processes = vec![system_process];
        snapshot.app_groups = vec![app_group.clone()];

        let results = engine.evaluate_snapshot(&snapshot);
        let result = results.get("systemd").unwrap();

        assert_eq!(result.priority_class, PriorityClass::Normal);
        assert!(result.reason.contains("system process"));
    }

    #[test]
    fn test_audio_client_with_xrun_protected() {
        let config = create_test_config();
        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        snapshot.responsiveness.audio_xruns_delta = Some(5);

        let audio_process = ProcessRecord {
            pid: 2000,
            ppid: 1,
            uid: 1000,
            gid: 1000,
            exe: Some("/usr/bin/pulseaudio".to_string()),
            cmdline: None,
            cgroup_path: None,
            systemd_unit: None,
            app_group_id: Some("pulseaudio".to_string()),
            state: "R".to_string(),
            start_time: 0,
            uptime_sec: 100,
            tty_nr: 0,
            has_tty: false,
            cpu_share_1s: None,
            cpu_share_10s: None,
            io_read_bytes: None,
            io_write_bytes: None,
            rss_mb: Some(100),
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
            is_audio_client: true,
            has_active_stream: true,
            process_type: Some("audio".to_string()),
            tags: vec!["audio".to_string()],
            nice: 0,
            ionice_class: Some(2),
            ionice_prio: Some(4),
            teacher_priority_class: None,
            teacher_score: None,
        };

        let app_group = AppGroupRecord {
            app_group_id: "pulseaudio".to_string(),
            root_pid: 2000,
            process_ids: vec![2000],
            app_name: Some("pulseaudio".to_string()),
            total_cpu_share: Some(0.1),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_rss_mb: Some(100),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec!["audio".to_string()],
            priority_class: None,
        };

        snapshot.processes = vec![audio_process];
        snapshot.app_groups = vec![app_group.clone()];

        let results = engine.evaluate_snapshot(&snapshot);
        let result = results.get("pulseaudio").unwrap();

        assert_eq!(result.priority_class, PriorityClass::Interactive);
        assert!(result.reason.contains("audio client with XRUN"));
    }

    #[test]
    fn test_updater_with_active_user_gets_background() {
        let config = create_test_config();
        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        snapshot.global.user_active = true;

        let app_group = AppGroupRecord {
            app_group_id: "updater".to_string(),
            root_pid: 3000,
            process_ids: vec![3000],
            app_name: Some("updater".to_string()),
            total_cpu_share: Some(0.2),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_rss_mb: Some(200),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec!["updater".to_string()],
            priority_class: None,
        };

        snapshot.app_groups = vec![app_group.clone()];

        let results = engine.evaluate_snapshot(&snapshot);
        let result = results.get("updater").unwrap();

        assert_eq!(result.priority_class, PriorityClass::Background);
        assert!(result.reason.contains("updater/indexer"));
    }

    #[test]
    fn test_noisy_neighbour_throttled() {
        let config = create_test_config();
        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        snapshot.responsiveness.bad_responsiveness = true;

        let app_group = AppGroupRecord {
            app_group_id: "noisy".to_string(),
            root_pid: 4000,
            process_ids: vec![4000],
            app_name: Some("noisy-app".to_string()),
            total_cpu_share: Some(0.8), // Высокий CPU usage
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_rss_mb: Some(500),
            has_gui_window: false,
            is_focused_group: false, // Не в фокусе
            tags: vec![],
            priority_class: None,
        };

        snapshot.app_groups = vec![app_group.clone()];

        let results = engine.evaluate_snapshot(&snapshot);
        let result = results.get("noisy").unwrap();

        assert_eq!(result.priority_class, PriorityClass::Background);
        assert!(result.reason.contains("noisy neighbour"));
    }

    #[test]
    fn test_crit_interactive_for_focused_game() {
        let config = create_test_config();
        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        let app_group = AppGroupRecord {
            app_group_id: "game".to_string(),
            root_pid: 5000,
            process_ids: vec![5000],
            app_name: Some("game".to_string()),
            total_cpu_share: Some(0.5),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_rss_mb: Some(1000),
            has_gui_window: true,
            is_focused_group: true, // В фокусе
            tags: vec!["game".to_string()],
            priority_class: None,
        };

        snapshot.app_groups = vec![app_group.clone()];

        let results = engine.evaluate_snapshot(&snapshot);
        let result = results.get("game").unwrap();

        assert_eq!(result.priority_class, PriorityClass::CritInteractive);
        assert!(result.reason.contains("focused group with audio/game"));
    }

    #[test]
    fn test_default_priority_when_no_rules_match() {
        let config = create_test_config();
        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        let app_group = AppGroupRecord {
            app_group_id: "unknown".to_string(),
            root_pid: 6000,
            process_ids: vec![6000],
            app_name: None,
            total_cpu_share: Some(0.1),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_rss_mb: Some(100),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
        };

        snapshot.app_groups = vec![app_group.clone()];

        let results = engine.evaluate_snapshot(&snapshot);
        let result = results.get("unknown").unwrap();

        assert_eq!(result.priority_class, PriorityClass::Normal);
        assert!(result.reason.contains("default"));
    }
}
