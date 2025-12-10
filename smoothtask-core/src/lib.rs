pub mod actuator;
pub mod classify;
pub mod config;
pub mod logging;
pub mod metrics;
pub mod model;
pub mod policy;

use anyhow::{Context, Result};
use chrono::Utc;
use config::Config;
use std::time::{Duration, Instant};
use tracing::{debug, error, info, warn};

use crate::actuator::{apply_priority_adjustments, plan_priority_changes, HysteresisTracker};
use crate::classify::{grouper::ProcessGrouper, rules::classify_all, rules::PatternDatabase};
use crate::logging::snapshots::{GlobalMetrics, ResponsivenessMetrics, Snapshot, SnapshotLogger};
use crate::metrics::audio::{AudioIntrospector, AudioMetrics, StaticAudioIntrospector};
use crate::metrics::audio_pipewire::PipeWireIntrospector;
use crate::metrics::input::InputTracker;
use crate::metrics::process::collect_process_metrics;
use crate::metrics::system::{collect_system_metrics, ProcPaths, SystemMetrics};
use crate::metrics::windows::{StaticWindowIntrospector, WindowIntrospector, X11Introspector};
use crate::policy::engine::PolicyEngine;

/// Главный цикл демона: опрос метрик, ранжирование, применение.
pub async fn run_daemon(config: Config, dry_run: bool) -> Result<()> {
    info!("Initializing SmoothTask daemon (dry_run = {})", dry_run);

    // Инициализация подсистем
    let mut snapshot_logger = if !config.paths.snapshot_db_path.is_empty() {
        Some(
            SnapshotLogger::new(&config.paths.snapshot_db_path)
                .context("Failed to initialize snapshot logger")?,
        )
    } else {
        None
    };

    let policy_engine = PolicyEngine::new(config.clone());
    let mut hysteresis = HysteresisTracker::new();

    // Инициализация интроспекторов
    // Пробуем использовать X11Introspector, если X-сервер доступен
    let window_introspector: Box<dyn WindowIntrospector> = {
        if X11Introspector::is_available() {
            match X11Introspector::new() {
                Ok(introspector) => {
                    info!("Using X11Introspector for window metrics");
                    Box::new(introspector)
                }
                Err(e) => {
                    warn!("X11 server available but failed to initialize X11Introspector: {}, falling back to StaticWindowIntrospector", e);
                    Box::new(StaticWindowIntrospector::new(Vec::new()))
                }
            }
        } else {
            warn!("X11 server not available, using StaticWindowIntrospector");
            Box::new(StaticWindowIntrospector::new(Vec::new()))
        }
    };

    // Инициализация PipeWire интроспектора с fallback на статический, если PipeWire недоступен
    let mut audio_introspector: Box<dyn AudioIntrospector> = {
        // Проверяем доступность pw-dump
        let pw_dump_available = std::process::Command::new("pw-dump")
            .arg("--version")
            .output()
            .is_ok();

        if pw_dump_available {
            info!("Using PipeWireIntrospector for audio metrics");
            Box::new(PipeWireIntrospector::new())
        } else {
            warn!("pw-dump not available, falling back to StaticAudioIntrospector");
            Box::new(StaticAudioIntrospector::empty())
        }
    };

    // Инициализация трекера активности пользователя
    let idle_threshold = Duration::from_secs(config.thresholds.user_idle_timeout_sec);
    let mut input_tracker = InputTracker::new(idle_threshold);

    // Загрузка базы паттернов для классификации
    let pattern_db = PatternDatabase::load(&config.paths.patterns_dir)
        .context("Failed to load pattern database")?;

    // Инициализация путей для чтения /proc
    let proc_paths = ProcPaths::default();

    // Состояние для вычисления дельт CPU
    let mut prev_cpu_times: Option<SystemMetrics> = None;

    info!("SmoothTask daemon started, entering main loop");

    let mut iteration = 0u64;
    loop {
        let loop_start = Instant::now();
        iteration += 1;

        debug!("Starting iteration {}", iteration);

        // Сбор снапшота
        let snapshot = match collect_snapshot(
            &proc_paths,
            window_introspector.as_ref(),
            &mut audio_introspector,
            &mut input_tracker,
            &mut prev_cpu_times,
            &config.thresholds,
        ) {
            Ok(snap) => snap,
            Err(e) => {
                error!("Failed to collect snapshot: {}", e);
                tokio::time::sleep(Duration::from_millis(config.polling_interval_ms)).await;
                continue;
            }
        };

        // Группировка процессов
        let mut processes = snapshot.processes.clone();
        let mut app_groups = ProcessGrouper::group_processes(&processes);

        // Классификация процессов и групп
        classify_all(&mut processes, &mut app_groups, &pattern_db);

        // Обновляем app_group_id в процессах на основе группировки
        for process in &mut processes {
            for app_group in &app_groups {
                if app_group.process_ids.contains(&process.pid) {
                    process.app_group_id = Some(app_group.app_group_id.clone());
                    break;
                }
            }
        }

        // Обновляем снапшот с классифицированными данными
        let snapshot = Snapshot {
            processes,
            app_groups,
            ..snapshot
        };

        // Применение политики
        let policy_results = policy_engine.evaluate_snapshot(&snapshot);

        // Планирование изменений приоритетов
        let adjustments = plan_priority_changes(&snapshot, &policy_results);

        if dry_run {
            debug!(
                "Dry-run: would apply {} priority adjustments",
                adjustments.len()
            );
            for adj in &adjustments {
                debug!(
                    "  PID {}: {} -> {} ({})",
                    adj.pid, adj.current_nice, adj.target_nice, adj.reason
                );
            }
        } else {
            // Применение изменений
            let apply_result = apply_priority_adjustments(&adjustments, &mut hysteresis);
            if apply_result.applied > 0 {
                info!(
                    "Applied {} priority adjustments ({} skipped due to hysteresis, {} errors)",
                    apply_result.applied, apply_result.skipped_hysteresis, apply_result.errors
                );
            }
            if apply_result.errors > 0 {
                warn!(
                    "Failed to apply {} priority adjustments",
                    apply_result.errors
                );
            }
        }

        // Логирование снапшота (опционально)
        if let Some(ref mut logger) = snapshot_logger {
            if let Err(e) = logger.log_snapshot(&snapshot) {
                warn!("Failed to log snapshot: {}", e);
            }
        }

        // Вычисляем время до следующей итерации
        let elapsed = loop_start.elapsed();
        let sleep_duration = if elapsed.as_millis() < config.polling_interval_ms as u128 {
            Duration::from_millis(config.polling_interval_ms) - elapsed
        } else {
            Duration::from_millis(0)
        };

        if sleep_duration.as_millis() > 0 {
            tokio::time::sleep(sleep_duration).await;
        } else {
            warn!(
                "Iteration {} took {}ms, longer than polling interval {}ms",
                iteration,
                elapsed.as_millis(),
                config.polling_interval_ms
            );
        }
    }
}

/// Собрать полный снапшот системы.
fn collect_snapshot(
    proc_paths: &ProcPaths,
    window_introspector: &dyn WindowIntrospector,
    audio_introspector: &mut Box<dyn AudioIntrospector>,
    input_tracker: &mut InputTracker,
    prev_cpu_times: &mut Option<SystemMetrics>,
    thresholds: &crate::config::Thresholds,
) -> Result<Snapshot> {
    let now = Instant::now();
    let timestamp = Utc::now();
    let snapshot_id = timestamp.timestamp_millis() as u64;

    // Сбор системных метрик
    let system_metrics =
        collect_system_metrics(proc_paths).context("Failed to collect system metrics")?;

    // Вычисление дельт CPU
    let cpu_usage = if let Some(ref prev) = prev_cpu_times {
        prev.cpu_times.delta(&system_metrics.cpu_times)
    } else {
        None
    };
    *prev_cpu_times = Some(system_metrics.clone());

    // Сбор метрик процессов
    let mut processes = collect_process_metrics().context("Failed to collect process metrics")?;

    // Сбор метрик окон
    let pid_to_window =
        crate::metrics::windows::build_pid_to_window_map(window_introspector).unwrap_or_default();

    // Обновление информации об окнах в процессах
    for process in &mut processes {
        if let Some(window) = pid_to_window.get(&(process.pid as u32)) {
            process.has_gui_window = true;
            process.is_focused_window = window.is_focused();
            process.window_state = Some(format!("{:?}", window.state));
        }
    }

    // Сбор метрик аудио
    let audio_metrics = audio_introspector.audio_metrics().unwrap_or_else(|_| {
        use std::time::SystemTime;
        AudioMetrics::empty(SystemTime::now(), SystemTime::now())
    });
    let audio_clients = audio_introspector.clients().unwrap_or_default();
    let audio_client_pids: std::collections::HashSet<u32> =
        audio_clients.iter().map(|c| c.pid).collect();

    // Обновление информации об аудио-клиентах в процессах
    for process in &mut processes {
        if audio_client_pids.contains(&(process.pid as u32)) {
            process.is_audio_client = true;
            process.has_active_stream = true;
        }
    }

    // Сбор метрик ввода (обновляем трекер для чтения новых событий из evdev, если доступно)
    let input_metrics = input_tracker.update(now);

    // Построение GlobalMetrics
    let global = GlobalMetrics {
        cpu_user: cpu_usage.map(|u| u.user).unwrap_or(0.0),
        cpu_system: cpu_usage.map(|u| u.system).unwrap_or(0.0),
        cpu_idle: cpu_usage.map(|u| u.idle).unwrap_or(0.0),
        cpu_iowait: cpu_usage.map(|u| u.iowait).unwrap_or(0.0),
        mem_total_kb: system_metrics.memory.mem_total_kb,
        mem_used_kb: system_metrics.memory.mem_used_kb(),
        mem_available_kb: system_metrics.memory.mem_available_kb,
        swap_total_kb: system_metrics.memory.swap_total_kb,
        swap_used_kb: system_metrics.memory.swap_used_kb(),
        load_avg_one: system_metrics.load_avg.one,
        load_avg_five: system_metrics.load_avg.five,
        load_avg_fifteen: system_metrics.load_avg.fifteen,
        psi_cpu_some_avg10: system_metrics.pressure.cpu.some.map(|p| p.avg10),
        psi_cpu_some_avg60: system_metrics.pressure.cpu.some.map(|p| p.avg60),
        psi_io_some_avg10: system_metrics.pressure.io.some.map(|p| p.avg10),
        psi_mem_some_avg10: system_metrics.pressure.memory.some.map(|p| p.avg10),
        psi_mem_full_avg10: system_metrics.pressure.memory.full.map(|p| p.avg10),
        user_active: input_metrics.user_active,
        time_since_last_input_ms: input_metrics.time_since_last_input_ms,
    };

    // Построение ResponsivenessMetrics
    let mut responsiveness = ResponsivenessMetrics {
        audio_xruns_delta: Some(audio_metrics.xrun_count as u64),
        ..Default::default()
    };

    // Вычисление bad_responsiveness и responsiveness_score
    responsiveness.compute(&global, thresholds);

    // Пока app_groups будут пустыми, они заполнятся после группировки
    Ok(Snapshot {
        snapshot_id,
        timestamp,
        global,
        processes,
        app_groups: Vec::new(),
        responsiveness,
    })
}
