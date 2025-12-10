//! Построение фич для ML-ранкера.
//!
//! Этот модуль преобразует Snapshot и AppGroupRecord в вектор фич,
//! совместимый с CatBoostRanker (аналогично Python-версии в `smoothtask_trainer.features`).

use crate::logging::snapshots::{AppGroupRecord, ProcessRecord, Snapshot};

/// Вектор фич для одного AppGroup.
///
/// Содержит числовые, булевые и категориальные фичи в том же порядке,
/// что и в Python-версии `build_feature_matrix`.
#[derive(Debug, Clone)]
pub struct FeatureVector {
    /// Числовые фичи (в порядке _NUMERIC_COLS из Python).
    pub numeric: Vec<f64>,
    /// Булевые фичи (в порядке _BOOL_COLS из Python).
    pub bool: Vec<i32>,
    /// Категориальные фичи (в порядке _CAT_COLS из Python).
    pub categorical: Vec<String>,
    /// Индексы категориальных фичей в общем векторе (numeric + bool + categorical).
    pub cat_feature_indices: Vec<usize>,
}

impl FeatureVector {
    /// Получить общее количество фич (numeric + bool + categorical).
    pub fn total_features(&self) -> usize {
        self.numeric.len() + self.bool.len() + self.categorical.len()
    }
}

/// Построить вектор фич для AppGroup из снапшота.
///
/// # Аргументы
///
/// * `snapshot` - полный снапшот системы
/// * `app_group` - группа приложений, для которой строятся фичи
///
/// # Возвращает
///
/// `FeatureVector` с нормализованными фичами и дефолтами для отсутствующих значений.
pub fn build_features(snapshot: &Snapshot, app_group: &AppGroupRecord) -> FeatureVector {
    // Находим процессы группы
    let group_processes: Vec<&ProcessRecord> = snapshot
        .processes
        .iter()
        .filter(|p| {
            p.app_group_id
                .as_deref()
                .map_or(false, |id| id == app_group.app_group_id)
        })
        .collect();

    // Берем первый процесс как представителя (для процессных фич)
    let representative_process = group_processes.first();

    // Числовые фичи (в порядке _NUMERIC_COLS)
    let mut numeric = Vec::new();

    // Процессные метрики
    numeric.push(
        representative_process
            .and_then(|p| p.cpu_share_1s)
            .unwrap_or(0.0),
    );
    numeric.push(
        representative_process
            .and_then(|p| p.cpu_share_10s)
            .unwrap_or(0.0),
    );
    numeric.push(
        representative_process
            .and_then(|p| p.io_read_bytes.map(|v| v as f64))
            .unwrap_or(0.0),
    );
    numeric.push(
        representative_process
            .and_then(|p| p.io_write_bytes.map(|v| v as f64))
            .unwrap_or(0.0),
    );
    numeric.push(
        representative_process
            .and_then(|p| p.rss_mb.map(|v| v as f64))
            .unwrap_or(0.0),
    );
    numeric.push(
        representative_process
            .and_then(|p| p.swap_mb.map(|v| v as f64))
            .unwrap_or(0.0),
    );
    numeric.push(
        representative_process
            .and_then(|p| p.voluntary_ctx.map(|v| v as f64))
            .unwrap_or(0.0),
    );
    numeric.push(
        representative_process
            .and_then(|p| p.involuntary_ctx.map(|v| v as f64))
            .unwrap_or(0.0),
    );
    numeric.push(representative_process.map(|p| p.nice as f64).unwrap_or(0.0));
    numeric.push(
        representative_process
            .and_then(|p| p.ionice_class.map(|v| v as f64))
            .unwrap_or(0.0),
    );
    numeric.push(
        representative_process
            .and_then(|p| p.ionice_prio.map(|v| v as f64))
            .unwrap_or(0.0),
    );

    // Глобальные метрики
    let g = &snapshot.global;
    numeric.push(g.load_avg_one);
    numeric.push(g.load_avg_five);
    numeric.push(g.load_avg_fifteen);
    numeric.push(g.mem_used_kb as f64);
    numeric.push(g.mem_available_kb as f64);
    numeric.push(g.mem_total_kb as f64);
    numeric.push(g.swap_total_kb as f64);
    numeric.push(g.swap_used_kb as f64);
    numeric.push(g.time_since_last_input_ms.map(|v| v as f64).unwrap_or(0.0));
    numeric.push(g.cpu_user);
    numeric.push(g.cpu_system);
    numeric.push(g.cpu_idle);
    numeric.push(g.cpu_iowait);
    numeric.push(g.psi_cpu_some_avg10.unwrap_or(0.0));
    numeric.push(g.psi_cpu_some_avg60.unwrap_or(0.0));
    numeric.push(g.psi_io_some_avg10.unwrap_or(0.0));
    numeric.push(g.psi_mem_some_avg10.unwrap_or(0.0));
    numeric.push(g.psi_mem_full_avg10.unwrap_or(0.0));

    // Групповые метрики
    numeric.push(app_group.total_cpu_share.unwrap_or(0.0));
    numeric.push(
        app_group
            .total_io_read_bytes
            .map(|v| v as f64)
            .unwrap_or(0.0),
    );
    numeric.push(
        app_group
            .total_io_write_bytes
            .map(|v| v as f64)
            .unwrap_or(0.0),
    );
    numeric.push(app_group.total_rss_mb.map(|v| v as f64).unwrap_or(0.0));

    // Булевые фичи (в порядке _BOOL_COLS)
    let mut bool_features = Vec::new();
    bool_features.push(if snapshot.global.user_active { 1 } else { 0 });
    bool_features.push(if snapshot.responsiveness.bad_responsiveness {
        1
    } else {
        0
    });
    bool_features.push(
        if representative_process.map(|p| p.has_tty).unwrap_or(false) {
            1
        } else {
            0
        },
    );
    bool_features.push(
        if representative_process
            .map(|p| p.has_gui_window)
            .unwrap_or(false)
        {
            1
        } else {
            0
        },
    );
    bool_features.push(
        if representative_process
            .map(|p| p.is_focused_window)
            .unwrap_or(false)
        {
            1
        } else {
            0
        },
    );
    bool_features.push(
        if representative_process
            .map(|p| p.env_has_display)
            .unwrap_or(false)
        {
            1
        } else {
            0
        },
    );
    bool_features.push(
        if representative_process
            .map(|p| p.env_has_wayland)
            .unwrap_or(false)
        {
            1
        } else {
            0
        },
    );
    bool_features.push(
        if representative_process.map(|p| p.env_ssh).unwrap_or(false) {
            1
        } else {
            0
        },
    );
    bool_features.push(
        if representative_process
            .map(|p| p.is_audio_client)
            .unwrap_or(false)
        {
            1
        } else {
            0
        },
    );
    bool_features.push(
        if representative_process
            .map(|p| p.has_active_stream)
            .unwrap_or(false)
        {
            1
        } else {
            0
        },
    );
    bool_features.push(if app_group.has_gui_window { 1 } else { 0 });
    bool_features.push(if app_group.is_focused_group { 1 } else { 0 });

    // Категориальные фичи (в порядке _CAT_COLS)
    let mut categorical = Vec::new();
    let numeric_count = numeric.len();
    let bool_count = bool_features.len();

    // process_type
    categorical.push(
        representative_process
            .and_then(|p| p.process_type.clone())
            .unwrap_or_else(|| "unknown".to_string()),
    );

    // app_name
    categorical.push(
        app_group
            .app_name
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
    );

    // priority_class
    categorical.push(
        app_group
            .priority_class
            .clone()
            .unwrap_or_else(|| "unknown".to_string()),
    );

    // teacher_priority_class
    categorical.push(
        representative_process
            .and_then(|p| p.teacher_priority_class.clone())
            .unwrap_or_else(|| "unknown".to_string()),
    );

    // env_term
    categorical.push(
        representative_process
            .and_then(|p| p.env_term.clone())
            .unwrap_or_else(|| "unknown".to_string()),
    );

    // tags_joined
    let tags_joined = if !app_group.tags.is_empty() {
        let mut sorted_tags = app_group.tags.clone();
        sorted_tags.sort();
        sorted_tags.join("|")
    } else {
        "unknown".to_string()
    };
    categorical.push(tags_joined);

    // Вычисляем индексы категориальных фичей
    let cat_feature_indices: Vec<usize> = (0..categorical.len())
        .map(|i| numeric_count + bool_count + i)
        .collect();

    FeatureVector {
        numeric,
        bool: bool_features,
        categorical,
        cat_feature_indices,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::snapshots::{GlobalMetrics, ResponsivenessMetrics};
    use chrono::Utc;

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
                has_tty: true,
                cpu_share_1s: Some(0.1),
                cpu_share_10s: Some(0.08),
                io_read_bytes: Some(1024 * 1024),
                io_write_bytes: Some(512 * 1024),
                rss_mb: Some(100),
                swap_mb: None,
                voluntary_ctx: Some(1000),
                involuntary_ctx: Some(50),
                has_gui_window: true,
                is_focused_window: true,
                window_state: None,
                env_has_display: true,
                env_has_wayland: false,
                env_term: Some("xterm-256color".to_string()),
                env_ssh: false,
                is_audio_client: false,
                has_active_stream: false,
                process_type: Some("gui_interactive".to_string()),
                tags: vec!["browser".to_string()],
                nice: 0,
                ionice_class: Some(2),
                ionice_prio: Some(4),
                teacher_priority_class: Some("INTERACTIVE".to_string()),
                teacher_score: Some(0.75),
            }],
            app_groups: vec![],
            responsiveness: ResponsivenessMetrics {
                sched_latency_p95_ms: Some(5.0),
                sched_latency_p99_ms: Some(10.0),
                audio_xruns_delta: None,
                ui_loop_p95_ms: None,
                frame_jank_ratio: None,
                bad_responsiveness: false,
                responsiveness_score: Some(0.9),
            },
        }
    }

    #[test]
    fn test_build_features_basic() {
        let snapshot = create_test_snapshot();
        let app_group = AppGroupRecord {
            app_group_id: "test-app".to_string(),
            root_pid: 1234,
            process_ids: vec![1234],
            app_name: Some("test".to_string()),
            total_cpu_share: Some(0.15),
            total_io_read_bytes: Some(2 * 1024 * 1024),
            total_io_write_bytes: Some(1024 * 1024),
            total_rss_mb: Some(200),
            has_gui_window: true,
            is_focused_group: true,
            tags: vec!["browser".to_string(), "media".to_string()],
            priority_class: Some("INTERACTIVE".to_string()),
        };

        let features = build_features(&snapshot, &app_group);

        // Проверяем количество фич
        assert_eq!(features.numeric.len(), 33); // 11 процессных + 18 глобальных + 4 групповых
        assert_eq!(features.bool.len(), 12);
        assert_eq!(features.categorical.len(), 6);
        assert_eq!(features.total_features(), 51);

        // Проверяем некоторые числовые фичи
        assert_eq!(features.numeric[0], 0.1); // cpu_share_1s
        assert_eq!(features.numeric[1], 0.08); // cpu_share_10s
        assert_eq!(features.numeric[11], 1.5); // load_avg_one

        // Проверяем булевые фичи
        assert_eq!(features.bool[0], 1); // user_active
        assert_eq!(features.bool[1], 0); // bad_responsiveness
        assert_eq!(features.bool[2], 1); // has_tty
        assert_eq!(features.bool[3], 1); // has_gui_window
        assert_eq!(features.bool[4], 1); // is_focused_window

        // Проверяем категориальные фичи
        assert_eq!(features.categorical[0], "gui_interactive"); // process_type
        assert_eq!(features.categorical[1], "test"); // app_name
        assert_eq!(features.categorical[2], "INTERACTIVE"); // priority_class
        assert_eq!(features.categorical[5], "browser|media"); // tags_joined (отсортированы)

        // Проверяем индексы категориальных фичей
        assert_eq!(features.cat_feature_indices.len(), 6);
        assert_eq!(features.cat_feature_indices[0], 33 + 12); // первый категориальный
    }

    #[test]
    fn test_build_features_defaults() {
        let snapshot = create_test_snapshot();
        let app_group = AppGroupRecord {
            app_group_id: "empty-app".to_string(),
            root_pid: 9999,
            process_ids: vec![9999],
            app_name: None,
            total_cpu_share: None,
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_rss_mb: None,
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
        };

        let features = build_features(&snapshot, &app_group);

        // Процесс не найден, должны быть дефолты
        assert_eq!(features.numeric[0], 0.0); // cpu_share_1s
        assert_eq!(features.numeric[1], 0.0); // cpu_share_10s

        // Категориальные фичи должны быть "unknown"
        assert_eq!(features.categorical[0], "unknown"); // process_type
        assert_eq!(features.categorical[1], "unknown"); // app_name
        assert_eq!(features.categorical[2], "unknown"); // priority_class
        assert_eq!(features.categorical[5], "unknown"); // tags_joined

        // Булевые фичи должны быть 0
        assert_eq!(features.bool[2], 0); // has_tty
        assert_eq!(features.bool[3], 0); // has_gui_window
    }

    #[test]
    fn test_build_features_cat_indices() {
        let snapshot = create_test_snapshot();
        let app_group = AppGroupRecord {
            app_group_id: "test-app".to_string(),
            root_pid: 1234,
            process_ids: vec![1234],
            app_name: Some("test".to_string()),
            total_cpu_share: Some(0.15),
            total_io_read_bytes: Some(2 * 1024 * 1024),
            total_io_write_bytes: Some(1024 * 1024),
            total_rss_mb: Some(200),
            has_gui_window: true,
            is_focused_group: true,
            tags: vec!["browser".to_string()],
            priority_class: Some("INTERACTIVE".to_string()),
        };

        let features = build_features(&snapshot, &app_group);

        // Проверяем, что индексы категориальных фичей корректны
        let numeric_count = features.numeric.len();
        let bool_count = features.bool.len();
        let expected_first_cat_idx = numeric_count + bool_count;

        assert_eq!(features.cat_feature_indices[0], expected_first_cat_idx);
        assert_eq!(
            features.cat_feature_indices[features.cat_feature_indices.len() - 1],
            expected_first_cat_idx + features.categorical.len() - 1
        );

        // Проверяем, что все индексы последовательны
        for (i, &idx) in features.cat_feature_indices.iter().enumerate() {
            assert_eq!(idx, expected_first_cat_idx + i);
        }
    }
}
