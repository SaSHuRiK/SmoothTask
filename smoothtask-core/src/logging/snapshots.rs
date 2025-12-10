//! Сохранение снапшотов в SQLite для обучения.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Transaction};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::config::Thresholds;
use crate::metrics::system::SystemMetrics;

/// Идентификатор снапшота (timestamp в миллисекундах).
pub type SnapshotId = u64;

/// Глобальные метрики системы (упрощённая версия для снапшотов).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GlobalMetrics {
    pub cpu_user: f64,
    pub cpu_system: f64,
    pub cpu_idle: f64,
    pub cpu_iowait: f64,
    pub mem_total_kb: u64,
    pub mem_used_kb: u64,
    pub mem_available_kb: u64,
    pub swap_total_kb: u64,
    pub swap_used_kb: u64,
    pub load_avg_one: f64,
    pub load_avg_five: f64,
    pub load_avg_fifteen: f64,
    pub psi_cpu_some_avg10: Option<f64>,
    pub psi_cpu_some_avg60: Option<f64>,
    pub psi_io_some_avg10: Option<f64>,
    pub psi_mem_some_avg10: Option<f64>,
    pub psi_mem_full_avg10: Option<f64>,
    pub user_active: bool,
    pub time_since_last_input_ms: Option<u64>,
}

impl From<&SystemMetrics> for GlobalMetrics {
    fn from(metrics: &SystemMetrics) -> Self {
        GlobalMetrics {
            cpu_user: 0.0, // будет заполнено при вычислении дельты
            cpu_system: 0.0,
            cpu_idle: 0.0,
            cpu_iowait: 0.0,
            mem_total_kb: metrics.memory.mem_total_kb,
            mem_used_kb: metrics.memory.mem_used_kb(),
            mem_available_kb: metrics.memory.mem_available_kb,
            swap_total_kb: metrics.memory.swap_total_kb,
            swap_used_kb: metrics.memory.swap_used_kb(),
            load_avg_one: metrics.load_avg.one,
            load_avg_five: metrics.load_avg.five,
            load_avg_fifteen: metrics.load_avg.fifteen,
            psi_cpu_some_avg10: metrics.pressure.cpu.some.map(|p| p.avg10),
            psi_cpu_some_avg60: metrics.pressure.cpu.some.map(|p| p.avg60),
            psi_io_some_avg10: metrics.pressure.io.some.map(|p| p.avg10),
            psi_mem_some_avg10: metrics.pressure.memory.some.map(|p| p.avg10),
            psi_mem_full_avg10: metrics.pressure.memory.full.map(|p| p.avg10),
            user_active: false, // будет заполнено из метрик ввода
            time_since_last_input_ms: None,
        }
    }
}

/// Метрики отзывчивости системы.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResponsivenessMetrics {
    pub sched_latency_p95_ms: Option<f64>,
    pub sched_latency_p99_ms: Option<f64>,
    pub audio_xruns_delta: Option<u64>,
    pub ui_loop_p95_ms: Option<f64>,
    pub frame_jank_ratio: Option<f64>,
    pub bad_responsiveness: bool,
    pub responsiveness_score: Option<f64>,
}

impl ResponsivenessMetrics {
    /// Вычислить bad_responsiveness и responsiveness_score на основе метрик и порогов.
    ///
    /// Согласно документации, bad_responsiveness определяется как:
    /// - psi_cpu_some_avg10 > T_cpu
    /// - psi_io_some_avg10 > T_io
    /// - sched_p99 > T_sched
    /// - audio_xruns_global_recent > 0
    /// - dsp_load_global > T_dsp (пока не реализовано)
    /// - ui_loop_p95 > T_ui (пока не реализовано)
    ///
    /// responsiveness_score вычисляется как нормированная комбинация метрик (0.0 = плохо, 1.0 = хорошо).
    pub fn compute(&mut self, global: &GlobalMetrics, thresholds: &Thresholds) {
        // Вычисление bad_responsiveness
        let mut bad = false;

        // Проверка PSI CPU
        if let Some(psi_cpu) = global.psi_cpu_some_avg10 {
            if psi_cpu > thresholds.psi_cpu_some_high as f64 {
                bad = true;
            }
        }

        // Проверка PSI IO
        if let Some(psi_io) = global.psi_io_some_avg10 {
            if psi_io > thresholds.psi_io_some_high as f64 {
                bad = true;
            }
        }

        // Проверка scheduling latency
        if let Some(sched_p99) = self.sched_latency_p99_ms {
            if sched_p99 > thresholds.sched_latency_p99_threshold_ms {
                bad = true;
            }
        }

        // Проверка XRUN
        if let Some(xruns) = self.audio_xruns_delta {
            if xruns > 0 {
                bad = true;
            }
        }

        // Проверка UI latency (если есть)
        // TODO: добавить порог для ui_loop_p95_ms когда появится метрика
        if let Some(ui_p95) = self.ui_loop_p95_ms {
            // Временный порог 16.67 мс (60 FPS)
            if ui_p95 > 16.67 {
                bad = true;
            }
        }

        self.bad_responsiveness = bad;

        // Вычисление responsiveness_score
        // Score = 1.0 - нормализованная комбинация проблемных метрик
        // Чем больше проблем, тем ниже score
        let mut problem_score = 0.0;
        let mut weight_sum = 0.0;

        // PSI CPU (вес 0.3)
        if let Some(psi_cpu) = global.psi_cpu_some_avg10 {
            let normalized = (psi_cpu / thresholds.psi_cpu_some_high as f64).min(2.0);
            problem_score += normalized * 0.3;
            weight_sum += 0.3;
        }

        // PSI IO (вес 0.2)
        if let Some(psi_io) = global.psi_io_some_avg10 {
            let normalized = (psi_io / thresholds.psi_io_some_high as f64).min(2.0);
            problem_score += normalized * 0.2;
            weight_sum += 0.2;
        }

        // Scheduling latency (вес 0.3)
        if let Some(sched_p99) = self.sched_latency_p99_ms {
            let normalized = (sched_p99 / thresholds.sched_latency_p99_threshold_ms).min(2.0);
            problem_score += normalized * 0.3;
            weight_sum += 0.3;
        }

        // XRUN (вес 0.1, бинарный: есть/нет)
        if let Some(xruns) = self.audio_xruns_delta {
            if xruns > 0 {
                problem_score += 1.0 * 0.1;
            }
            weight_sum += 0.1;
        }

        // UI latency (вес 0.1, если есть)
        if let Some(ui_p95) = self.ui_loop_p95_ms {
            let normalized = (ui_p95 / 16.67).min(2.0);
            problem_score += normalized * 0.1;
            weight_sum += 0.1;
        }

        // Вычисляем финальный score: 1.0 - нормализованный problem_score
        if weight_sum > 0.0 {
            let normalized_problem = problem_score / weight_sum;
            self.responsiveness_score = Some((1.0 - normalized_problem.min(1.0)).max(0.0));
        } else {
            // Если нет доступных метрик, считаем score = 1.0 (хорошая отзывчивость)
            self.responsiveness_score = Some(1.0);
        }
    }
}

/// Запись о процессе в снапшоте.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRecord {
    pub pid: i32,
    pub ppid: i32,
    pub uid: u32,
    pub gid: u32,
    pub exe: Option<String>,
    pub cmdline: Option<String>,
    pub cgroup_path: Option<String>,
    pub systemd_unit: Option<String>,
    pub app_group_id: Option<String>,
    pub state: String,
    pub start_time: u64,
    pub uptime_sec: u64,
    pub tty_nr: i32,
    pub has_tty: bool,
    pub cpu_share_1s: Option<f64>,
    pub cpu_share_10s: Option<f64>,
    pub io_read_bytes: Option<u64>,
    pub io_write_bytes: Option<u64>,
    pub rss_mb: Option<u64>,
    pub swap_mb: Option<u64>,
    pub voluntary_ctx: Option<u64>,
    pub involuntary_ctx: Option<u64>,
    pub has_gui_window: bool,
    pub is_focused_window: bool,
    pub window_state: Option<String>,
    pub env_has_display: bool,
    pub env_has_wayland: bool,
    pub env_term: Option<String>,
    pub env_ssh: bool,
    pub is_audio_client: bool,
    pub has_active_stream: bool,
    pub process_type: Option<String>,
    pub tags: Vec<String>,
    pub nice: i32,
    pub ionice_class: Option<i32>,
    pub ionice_prio: Option<i32>,
    pub teacher_priority_class: Option<String>,
    pub teacher_score: Option<f64>,
}

/// Запись о группе приложений в снапшоте.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppGroupRecord {
    pub app_group_id: String,
    pub root_pid: i32,
    pub process_ids: Vec<i32>,
    pub app_name: Option<String>,
    pub total_cpu_share: Option<f64>,
    pub total_io_read_bytes: Option<u64>,
    pub total_io_write_bytes: Option<u64>,
    pub total_rss_mb: Option<u64>,
    pub has_gui_window: bool,
    pub is_focused_group: bool,
    pub tags: Vec<String>,
    pub priority_class: Option<String>,
}

/// Полный снапшот системы.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub snapshot_id: SnapshotId,
    pub timestamp: DateTime<Utc>,
    pub global: GlobalMetrics,
    pub processes: Vec<ProcessRecord>,
    pub app_groups: Vec<AppGroupRecord>,
    pub responsiveness: ResponsivenessMetrics,
}

/// Менеджер для записи снапшотов в SQLite.
pub struct SnapshotLogger {
    conn: Connection,
}

impl SnapshotLogger {
    /// Создать новый логгер и инициализировать схему БД.
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path.as_ref())
            .with_context(|| format!("Не удалось открыть БД: {}", db_path.as_ref().display()))?;

        let logger = SnapshotLogger { conn };
        logger.init_schema()?;
        Ok(logger)
    }

    /// Инициализировать схему БД (создать таблицы, если их нет).
    fn init_schema(&self) -> Result<()> {
        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS snapshots (
                snapshot_id INTEGER PRIMARY KEY,
                timestamp TEXT NOT NULL,
                cpu_user REAL,
                cpu_system REAL,
                cpu_idle REAL,
                cpu_iowait REAL,
                mem_total_kb INTEGER,
                mem_used_kb INTEGER,
                mem_available_kb INTEGER,
                swap_total_kb INTEGER,
                swap_used_kb INTEGER,
                load_avg_one REAL,
                load_avg_five REAL,
                load_avg_fifteen REAL,
                psi_cpu_some_avg10 REAL,
                psi_cpu_some_avg60 REAL,
                psi_io_some_avg10 REAL,
                psi_mem_some_avg10 REAL,
                psi_mem_full_avg10 REAL,
                user_active INTEGER,
                time_since_last_input_ms INTEGER,
                sched_latency_p95_ms REAL,
                sched_latency_p99_ms REAL,
                audio_xruns_delta INTEGER,
                ui_loop_p95_ms REAL,
                frame_jank_ratio REAL,
                bad_responsiveness INTEGER,
                responsiveness_score REAL
            )
            "#,
            [],
        )?;

        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS processes (
                snapshot_id INTEGER NOT NULL,
                pid INTEGER NOT NULL,
                ppid INTEGER,
                uid INTEGER,
                gid INTEGER,
                exe TEXT,
                cmdline TEXT,
                cgroup_path TEXT,
                systemd_unit TEXT,
                app_group_id TEXT,
                state TEXT,
                start_time INTEGER,
                uptime_sec INTEGER,
                tty_nr INTEGER,
                has_tty INTEGER,
                cpu_share_1s REAL,
                cpu_share_10s REAL,
                io_read_bytes INTEGER,
                io_write_bytes INTEGER,
                rss_mb INTEGER,
                swap_mb INTEGER,
                voluntary_ctx INTEGER,
                involuntary_ctx INTEGER,
                has_gui_window INTEGER,
                is_focused_window INTEGER,
                window_state TEXT,
                env_has_display INTEGER,
                env_has_wayland INTEGER,
                env_term TEXT,
                env_ssh INTEGER,
                is_audio_client INTEGER,
                has_active_stream INTEGER,
                process_type TEXT,
                tags TEXT,
                nice INTEGER,
                ionice_class INTEGER,
                ionice_prio INTEGER,
                teacher_priority_class TEXT,
                teacher_score REAL,
                PRIMARY KEY (snapshot_id, pid),
                FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
            )
            "#,
            [],
        )?;

        self.conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS app_groups (
                snapshot_id INTEGER NOT NULL,
                app_group_id TEXT NOT NULL,
                root_pid INTEGER,
                process_ids TEXT,
                app_name TEXT,
                total_cpu_share REAL,
                total_io_read_bytes INTEGER,
                total_io_write_bytes INTEGER,
                total_rss_mb INTEGER,
                has_gui_window INTEGER,
                is_focused_group INTEGER,
                tags TEXT,
                priority_class TEXT,
                PRIMARY KEY (snapshot_id, app_group_id),
                FOREIGN KEY (snapshot_id) REFERENCES snapshots(snapshot_id)
            )
            "#,
            [],
        )?;

        // Индексы для ускорения запросов
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp ON snapshots(timestamp)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_processes_pid ON processes(pid)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_processes_app_group ON processes(app_group_id)",
            [],
        )?;

        Ok(())
    }

    /// Записать снапшот в БД.
    pub fn log_snapshot(&mut self, snapshot: &Snapshot) -> Result<()> {
        let tx = self.conn.transaction()?;
        Self::insert_snapshot(&tx, snapshot)?;
        Self::insert_processes(&tx, snapshot)?;
        Self::insert_app_groups(&tx, snapshot)?;
        tx.commit()?;
        Ok(())
    }

    fn insert_snapshot(tx: &Transaction, snapshot: &Snapshot) -> Result<()> {
        let g = &snapshot.global;
        let r = &snapshot.responsiveness;

        tx.execute(
            r#"
            INSERT INTO snapshots (
                snapshot_id, timestamp,
                cpu_user, cpu_system, cpu_idle, cpu_iowait,
                mem_total_kb, mem_used_kb, mem_available_kb,
                swap_total_kb, swap_used_kb,
                load_avg_one, load_avg_five, load_avg_fifteen,
                psi_cpu_some_avg10, psi_cpu_some_avg60,
                psi_io_some_avg10,
                psi_mem_some_avg10, psi_mem_full_avg10,
                user_active, time_since_last_input_ms,
                sched_latency_p95_ms, sched_latency_p99_ms,
                audio_xruns_delta,
                ui_loop_p95_ms, frame_jank_ratio,
                bad_responsiveness, responsiveness_score
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                snapshot.snapshot_id as i64,
                snapshot.timestamp.to_rfc3339(),
                g.cpu_user,
                g.cpu_system,
                g.cpu_idle,
                g.cpu_iowait,
                g.mem_total_kb as i64,
                g.mem_used_kb as i64,
                g.mem_available_kb as i64,
                g.swap_total_kb as i64,
                g.swap_used_kb as i64,
                g.load_avg_one,
                g.load_avg_five,
                g.load_avg_fifteen,
                g.psi_cpu_some_avg10,
                g.psi_cpu_some_avg60,
                g.psi_io_some_avg10,
                g.psi_mem_some_avg10,
                g.psi_mem_full_avg10,
                g.user_active as i32,
                g.time_since_last_input_ms.map(|v| v as i64),
                r.sched_latency_p95_ms,
                r.sched_latency_p99_ms,
                r.audio_xruns_delta.map(|v| v as i64),
                r.ui_loop_p95_ms,
                r.frame_jank_ratio,
                r.bad_responsiveness as i32,
                r.responsiveness_score,
            ],
        )?;
        Ok(())
    }

    fn insert_processes(tx: &Transaction, snapshot: &Snapshot) -> Result<()> {
        for proc in &snapshot.processes {
            let tags_json = serde_json::to_string(&proc.tags)
                .context("Не удалось сериализовать tags процесса")?;

            tx.execute(
                r#"
                INSERT INTO processes (
                    snapshot_id, pid, ppid, uid, gid,
                    exe, cmdline, cgroup_path, systemd_unit, app_group_id,
                    state, start_time, uptime_sec,
                    tty_nr, has_tty,
                    cpu_share_1s, cpu_share_10s,
                    io_read_bytes, io_write_bytes,
                    rss_mb, swap_mb,
                    voluntary_ctx, involuntary_ctx,
                    has_gui_window, is_focused_window, window_state,
                    env_has_display, env_has_wayland, env_term, env_ssh,
                    is_audio_client, has_active_stream,
                    process_type, tags,
                    nice, ionice_class, ionice_prio,
                    teacher_priority_class, teacher_score
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    snapshot.snapshot_id as i64,
                    proc.pid,
                    proc.ppid,
                    proc.uid as i64,
                    proc.gid as i64,
                    proc.exe,
                    proc.cmdline,
                    proc.cgroup_path,
                    proc.systemd_unit,
                    proc.app_group_id,
                    proc.state,
                    proc.start_time as i64,
                    proc.uptime_sec as i64,
                    proc.tty_nr,
                    proc.has_tty as i32,
                    proc.cpu_share_1s,
                    proc.cpu_share_10s,
                    proc.io_read_bytes.map(|v| v as i64),
                    proc.io_write_bytes.map(|v| v as i64),
                    proc.rss_mb.map(|v| v as i64),
                    proc.swap_mb.map(|v| v as i64),
                    proc.voluntary_ctx.map(|v| v as i64),
                    proc.involuntary_ctx.map(|v| v as i64),
                    proc.has_gui_window as i32,
                    proc.is_focused_window as i32,
                    proc.window_state,
                    proc.env_has_display as i32,
                    proc.env_has_wayland as i32,
                    proc.env_term,
                    proc.env_ssh as i32,
                    proc.is_audio_client as i32,
                    proc.has_active_stream as i32,
                    proc.process_type,
                    tags_json,
                    proc.nice,
                    proc.ionice_class,
                    proc.ionice_prio,
                    proc.teacher_priority_class,
                    proc.teacher_score,
                ],
            )?;
        }
        Ok(())
    }

    fn insert_app_groups(tx: &Transaction, snapshot: &Snapshot) -> Result<()> {
        for group in &snapshot.app_groups {
            let process_ids_json = serde_json::to_string(&group.process_ids)
                .context("Не удалось сериализовать process_ids группы")?;
            let tags_json = serde_json::to_string(&group.tags)
                .context("Не удалось сериализовать tags группы")?;

            tx.execute(
                r#"
                INSERT INTO app_groups (
                    snapshot_id, app_group_id, root_pid, process_ids,
                    app_name,
                    total_cpu_share,
                    total_io_read_bytes, total_io_write_bytes,
                    total_rss_mb,
                    has_gui_window, is_focused_group,
                    tags, priority_class
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                params![
                    snapshot.snapshot_id as i64,
                    group.app_group_id,
                    group.root_pid,
                    process_ids_json,
                    group.app_name,
                    group.total_cpu_share,
                    group.total_io_read_bytes.map(|v| v as i64),
                    group.total_io_write_bytes.map(|v| v as i64),
                    group.total_rss_mb.map(|v| v as i64),
                    group.has_gui_window as i32,
                    group.is_focused_group as i32,
                    tags_json,
                    group.priority_class,
                ],
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Thresholds;
    use tempfile::NamedTempFile;

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
            processes: vec![ProcessRecord {
                pid: 1234,
                ppid: 1,
                uid: 1000,
                gid: 1000,
                exe: Some("/usr/bin/test".to_string()),
                cmdline: Some("test --flag".to_string()),
                cgroup_path: Some("/user.slice/user-1000.slice".to_string()),
                systemd_unit: None,
                app_group_id: Some("test-app".to_string()),
                state: "R".to_string(),
                start_time: 1000000,
                uptime_sec: 3600,
                tty_nr: 0,
                has_tty: false,
                cpu_share_1s: Some(0.1),
                cpu_share_10s: Some(0.08),
                io_read_bytes: Some(1024 * 1024),
                io_write_bytes: Some(512 * 1024),
                rss_mb: Some(100),
                swap_mb: None,
                voluntary_ctx: Some(1000),
                involuntary_ctx: Some(50),
                has_gui_window: false,
                is_focused_window: false,
                window_state: None,
                env_has_display: false,
                env_has_wayland: false,
                env_term: None,
                env_ssh: false,
                is_audio_client: false,
                has_active_stream: false,
                process_type: Some("cli_interactive".to_string()),
                tags: vec!["terminal".to_string()],
                nice: 0,
                ionice_class: Some(2),
                ionice_prio: Some(4),
                teacher_priority_class: Some("INTERACTIVE".to_string()),
                teacher_score: Some(0.75),
            }],
            app_groups: vec![AppGroupRecord {
                app_group_id: "test-app".to_string(),
                root_pid: 1234,
                process_ids: vec![1234, 1235],
                app_name: Some("test".to_string()),
                total_cpu_share: Some(0.15),
                total_io_read_bytes: Some(2 * 1024 * 1024),
                total_io_write_bytes: Some(1024 * 1024),
                total_rss_mb: Some(200),
                has_gui_window: false,
                is_focused_group: false,
                tags: vec!["terminal".to_string()],
                priority_class: Some("INTERACTIVE".to_string()),
            }],
            responsiveness: ResponsivenessMetrics {
                sched_latency_p95_ms: Some(5.0),
                sched_latency_p99_ms: Some(10.0),
                audio_xruns_delta: Some(0),
                ui_loop_p95_ms: None,
                frame_jank_ratio: None,
                bad_responsiveness: false,
                responsiveness_score: Some(0.9),
            },
        }
    }

    #[test]
    fn test_snapshot_logger_create_and_insert() {
        let tmp_file = NamedTempFile::new().expect("temp file");
        let db_path = tmp_file.path();

        let mut logger = SnapshotLogger::new(db_path).expect("logger created");
        let snapshot = create_test_snapshot();

        logger.log_snapshot(&snapshot).expect("snapshot logged");

        // Проверяем, что данные записались
        let conn = Connection::open(db_path).expect("reopen db");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
            .expect("count snapshots");
        assert_eq!(count, 1);

        let proc_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM processes", [], |row| row.get(0))
            .expect("count processes");
        assert_eq!(proc_count, 1);

        let group_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM app_groups", [], |row| row.get(0))
            .expect("count groups");
        assert_eq!(group_count, 1);
    }

    #[test]
    fn test_snapshot_logger_multiple_snapshots() {
        let tmp_file = NamedTempFile::new().expect("temp file");
        let db_path = tmp_file.path();

        let mut logger = SnapshotLogger::new(db_path).expect("logger created");

        let mut snapshot1 = create_test_snapshot();
        snapshot1.snapshot_id = 1000;
        logger.log_snapshot(&snapshot1).expect("snapshot 1 logged");

        let mut snapshot2 = create_test_snapshot();
        snapshot2.snapshot_id = 2000;
        logger.log_snapshot(&snapshot2).expect("snapshot 2 logged");

        let conn = Connection::open(db_path).expect("reopen db");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM snapshots", [], |row| row.get(0))
            .expect("count snapshots");
        assert_eq!(count, 2);
    }

    fn create_test_thresholds() -> Thresholds {
        Thresholds {
            psi_cpu_some_high: 0.6,
            psi_io_some_high: 0.4,
            user_idle_timeout_sec: 120,
            interactive_build_grace_sec: 10,
            noisy_neighbour_cpu_share: 0.7,
            crit_interactive_percentile: 0.9,
            interactive_percentile: 0.6,
            normal_percentile: 0.3,
            background_percentile: 0.1,
            sched_latency_p99_threshold_ms: 10.0,
        }
    }

    #[test]
    fn test_compute_responsiveness_good_conditions() {
        let thresholds = create_test_thresholds();
        let global = GlobalMetrics {
            psi_cpu_some_avg10: Some(0.1), // 0.1 / 0.6 = 0.167
            psi_io_some_avg10: Some(0.2),  // 0.2 / 0.4 = 0.5
            ..Default::default()
        };
        let mut responsiveness = ResponsivenessMetrics {
            sched_latency_p99_ms: Some(5.0), // 5.0 / 10.0 = 0.5
            audio_xruns_delta: Some(0),      // нет XRUN
            ..Default::default()
        };

        responsiveness.compute(&global, &thresholds);

        assert!(!responsiveness.bad_responsiveness);
        assert!(responsiveness.responsiveness_score.is_some());
        let score = responsiveness.responsiveness_score.unwrap();
        // При хороших условиях score должен быть > 0.5 (все метрики ниже порогов)
        assert!(
            score > 0.5,
            "score should be reasonable for good conditions, got {}",
            score
        );
    }

    #[test]
    fn test_compute_responsiveness_psi_cpu_high() {
        let thresholds = create_test_thresholds();
        let global = GlobalMetrics {
            psi_cpu_some_avg10: Some(0.8), // Выше порога 0.6
            psi_io_some_avg10: Some(0.2),
            ..Default::default()
        };
        let mut responsiveness = ResponsivenessMetrics {
            sched_latency_p99_ms: Some(5.0),
            audio_xruns_delta: Some(0),
            ..Default::default()
        };

        responsiveness.compute(&global, &thresholds);

        assert!(
            responsiveness.bad_responsiveness,
            "should detect bad responsiveness due to high PSI CPU"
        );
        assert!(responsiveness.responsiveness_score.is_some());
        let score = responsiveness.responsiveness_score.unwrap();
        assert!(
            score < 0.8,
            "score should be lower due to high PSI CPU, got {}",
            score
        );
    }

    #[test]
    fn test_compute_responsiveness_psi_io_high() {
        let thresholds = create_test_thresholds();
        let global = GlobalMetrics {
            psi_cpu_some_avg10: Some(0.1),
            psi_io_some_avg10: Some(0.5), // Выше порога 0.4
            ..Default::default()
        };
        let mut responsiveness = ResponsivenessMetrics {
            sched_latency_p99_ms: Some(5.0),
            audio_xruns_delta: Some(0),
            ..Default::default()
        };

        responsiveness.compute(&global, &thresholds);

        assert!(
            responsiveness.bad_responsiveness,
            "should detect bad responsiveness due to high PSI IO"
        );
    }

    #[test]
    fn test_compute_responsiveness_sched_latency_high() {
        let thresholds = create_test_thresholds();
        let global = GlobalMetrics {
            psi_cpu_some_avg10: Some(0.1),
            psi_io_some_avg10: Some(0.2),
            ..Default::default()
        };
        let mut responsiveness = ResponsivenessMetrics {
            sched_latency_p99_ms: Some(15.0), // Выше порога 10.0
            audio_xruns_delta: Some(0),
            ..Default::default()
        };

        responsiveness.compute(&global, &thresholds);

        assert!(
            responsiveness.bad_responsiveness,
            "should detect bad responsiveness due to high scheduling latency"
        );
    }

    #[test]
    fn test_compute_responsiveness_audio_xruns() {
        let thresholds = create_test_thresholds();
        let global = GlobalMetrics {
            psi_cpu_some_avg10: Some(0.1),
            psi_io_some_avg10: Some(0.2),
            ..Default::default()
        };
        let mut responsiveness = ResponsivenessMetrics {
            sched_latency_p99_ms: Some(5.0),
            audio_xruns_delta: Some(1), // Есть XRUN
            ..Default::default()
        };

        responsiveness.compute(&global, &thresholds);

        assert!(
            responsiveness.bad_responsiveness,
            "should detect bad responsiveness due to audio XRUN"
        );
    }

    #[test]
    fn test_compute_responsiveness_multiple_problems() {
        let thresholds = create_test_thresholds();
        let global = GlobalMetrics {
            psi_cpu_some_avg10: Some(0.8), // Выше порога
            psi_io_some_avg10: Some(0.5),  // Выше порога
            ..Default::default()
        };
        let mut responsiveness = ResponsivenessMetrics {
            sched_latency_p99_ms: Some(15.0), // Выше порога
            audio_xruns_delta: Some(2),       // Есть XRUN
            ..Default::default()
        };

        responsiveness.compute(&global, &thresholds);

        assert!(
            responsiveness.bad_responsiveness,
            "should detect bad responsiveness with multiple problems"
        );
        assert!(responsiveness.responsiveness_score.is_some());
        let score = responsiveness.responsiveness_score.unwrap();
        assert!(
            score < 0.5,
            "score should be very low with multiple problems, got {}",
            score
        );
    }

    #[test]
    fn test_compute_responsiveness_no_metrics() {
        let thresholds = create_test_thresholds();
        let global = GlobalMetrics {
            psi_cpu_some_avg10: None,
            psi_io_some_avg10: None,
            ..Default::default()
        };
        let mut responsiveness = ResponsivenessMetrics {
            sched_latency_p99_ms: None,
            audio_xruns_delta: None,
            ..Default::default()
        };

        responsiveness.compute(&global, &thresholds);

        // Без метрик считаем, что отзывчивость хорошая
        assert!(!responsiveness.bad_responsiveness);
        assert_eq!(responsiveness.responsiveness_score, Some(1.0));
    }

    #[test]
    fn test_compute_responsiveness_score_range() {
        let thresholds = create_test_thresholds();
        let global = GlobalMetrics {
            psi_cpu_some_avg10: Some(0.3),
            psi_io_some_avg10: Some(0.2),
            ..Default::default()
        };
        let mut responsiveness = ResponsivenessMetrics {
            sched_latency_p99_ms: Some(7.0),
            audio_xruns_delta: Some(0),
            ..Default::default()
        };

        responsiveness.compute(&global, &thresholds);

        assert!(responsiveness.responsiveness_score.is_some());
        let score = responsiveness.responsiveness_score.unwrap();
        assert!(
            (0.0..=1.0).contains(&score),
            "score should be in [0, 1] range, got {}",
            score
        );
    }
}
