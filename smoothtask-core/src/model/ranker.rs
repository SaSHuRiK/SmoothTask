//! ML-ранкер для ранжирования AppGroup по приоритету.
//!
//! Этот модуль предоставляет интерфейс для ранжирования групп приложений
//! на основе их фич. В будущем здесь будет интеграция с ONNX/JSON моделями.

use crate::logging::snapshots::AppGroupRecord;
use crate::model::features::build_features;

/// Результат ранжирования для одной AppGroup.
#[derive(Debug, Clone)]
pub struct RankingResult {
    /// Score от ранкера (чем выше, тем важнее группа).
    pub score: f64,
    /// Ранг группы среди всех групп (1 = самый важный).
    pub rank: usize,
    /// Percentile score (0.0 - 1.0, где 1.0 = самый важный).
    pub percentile: f64,
}

/// Трейт для ранжирования AppGroup на основе их фич.
///
/// Трейт требует `Send + Sync`, так как ранкер используется в async контексте
/// и может быть перемещён между потоками.
pub trait Ranker: Send + Sync {
    /// Ранжировать список AppGroup на основе их фич из снапшота.
    ///
    /// # Аргументы
    ///
    /// * `app_groups` - список групп приложений для ранжирования
    /// * `snapshot` - полный снапшот системы (для построения фич)
    ///
    /// # Возвращает
    ///
    /// Маппинг app_group_id -> RankingResult с score, rank и percentile.
    fn rank(
        &self,
        app_groups: &[AppGroupRecord],
        snapshot: &crate::logging::snapshots::Snapshot,
    ) -> std::collections::HashMap<String, RankingResult>;
}

/// Заглушка ранкера для тестирования.
///
/// Возвращает фиксированные scores на основе простых правил:
/// - Фокусные группы получают высокий score
/// - GUI группы получают средний score
/// - Остальные получают низкий score
pub struct StubRanker;

impl StubRanker {
    /// Создать новый заглушку ранкера.
    pub fn new() -> Self {
        Self
    }
}

impl Default for StubRanker {
    fn default() -> Self {
        Self::new()
    }
}

impl Ranker for StubRanker {
    fn rank(
        &self,
        app_groups: &[AppGroupRecord],
        snapshot: &crate::logging::snapshots::Snapshot,
    ) -> std::collections::HashMap<String, RankingResult> {
        // Строим фичи для каждой группы и вычисляем простой score
        let mut scores: Vec<(String, f64)> = Vec::new();

        for app_group in app_groups {
            let _features = build_features(snapshot, app_group);

            // Простая эвристика для score:
            // - Базовая оценка: 0.5
            let mut score: f64 = 0.5;

            // - Фокусная группа: +0.4
            if app_group.is_focused_group {
                score += 0.4;
            }

            // - GUI группа: +0.2
            if app_group.has_gui_window {
                score += 0.2;
            }

            // - Высокий CPU usage (может быть важно): +0.1
            if let Some(cpu_share) = app_group.total_cpu_share {
                if cpu_share > 0.3 {
                    score += 0.1;
                }
            }

            // Ограничиваем score в диапазоне [0.0, 1.0]
            score = score.clamp(0.0, 1.0);

            scores.push((app_group.app_group_id.clone(), score));
        }

        // Сортируем по score (убывание)
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Вычисляем rank и percentile
        let total = scores.len();
        let mut results = std::collections::HashMap::new();

        for (rank_idx, (app_group_id, score)) in scores.iter().enumerate() {
            let rank = rank_idx + 1;
            // Percentile: 1.0 для самого важного, 0.0 для наименее важного
            let percentile = if total > 1 {
                1.0 - (rank_idx as f64) / ((total - 1) as f64)
            } else {
                1.0
            };

            results.insert(
                app_group_id.clone(),
                RankingResult {
                    score: *score,
                    rank,
                    percentile,
                },
            );
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::snapshots::{GlobalMetrics, ResponsivenessMetrics, Snapshot};
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
            processes: vec![],
            app_groups: vec![],
            responsiveness: ResponsivenessMetrics::default(),
        }
    }

    #[test]
    fn test_stub_ranker_basic() {
        let ranker = StubRanker::new();
        let mut snapshot = create_test_snapshot();

        let app_groups = vec![
            AppGroupRecord {
                app_group_id: "focused-gui".to_string(),
                root_pid: 1000,
                process_ids: vec![1000],
                app_name: Some("firefox".to_string()),
                total_cpu_share: Some(0.5),
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: Some(500),
                has_gui_window: true,
                is_focused_group: true,
                tags: vec!["browser".to_string()],
                priority_class: None,
            },
            AppGroupRecord {
                app_group_id: "background".to_string(),
                root_pid: 2000,
                process_ids: vec![2000],
                app_name: Some("updater".to_string()),
                total_cpu_share: Some(0.1),
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: Some(100),
                has_gui_window: false,
                is_focused_group: false,
                tags: vec!["updater".to_string()],
                priority_class: None,
            },
        ];

        snapshot.app_groups = app_groups.clone();

        let results = ranker.rank(&app_groups, &snapshot);

        // Проверяем, что результаты есть для всех групп
        assert_eq!(results.len(), 2);

        // Фокусная группа должна иметь более высокий score и rank = 1
        let focused_result = results.get("focused-gui").unwrap();
        let background_result = results.get("background").unwrap();

        assert!(focused_result.score > background_result.score);
        assert_eq!(focused_result.rank, 1);
        assert_eq!(background_result.rank, 2);
        assert!(focused_result.percentile > background_result.percentile);

        // Проверяем диапазоны
        assert!((0.0..=1.0).contains(&focused_result.score));
        assert!((0.0..=1.0).contains(&background_result.score));
        assert!((0.0..=1.0).contains(&focused_result.percentile));
        assert!((0.0..=1.0).contains(&background_result.percentile));
    }

    #[test]
    fn test_stub_ranker_single_group() {
        let ranker = StubRanker::new();
        let mut snapshot = create_test_snapshot();

        let app_groups = vec![AppGroupRecord {
            app_group_id: "single".to_string(),
            root_pid: 3000,
            process_ids: vec![3000],
            app_name: Some("app".to_string()),
            total_cpu_share: Some(0.2),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_rss_mb: Some(200),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
        }];

        snapshot.app_groups = app_groups.clone();

        let results = ranker.rank(&app_groups, &snapshot);

        assert_eq!(results.len(), 1);
        let result = results.get("single").unwrap();
        assert_eq!(result.rank, 1);
        assert_eq!(result.percentile, 1.0); // Единственная группа имеет percentile = 1.0
        assert!((0.0..=1.0).contains(&result.score));
    }

    #[test]
    fn test_stub_ranker_empty_list() {
        let ranker = StubRanker::new();
        let snapshot = create_test_snapshot();

        let app_groups = vec![];

        let results = ranker.rank(&app_groups, &snapshot);

        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_stub_ranker_identical_scores() {
        let ranker = StubRanker::new();
        let mut snapshot = create_test_snapshot();

        // Создаём две группы с одинаковыми характеристиками (должны получить одинаковый score)
        let app_groups = vec![
            AppGroupRecord {
                app_group_id: "group1".to_string(),
                root_pid: 1000,
                process_ids: vec![1000],
                app_name: Some("app1".to_string()),
                total_cpu_share: Some(0.2),
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: Some(100),
                has_gui_window: false,
                is_focused_group: false,
                tags: vec![],
                priority_class: None,
            },
            AppGroupRecord {
                app_group_id: "group2".to_string(),
                root_pid: 2000,
                process_ids: vec![2000],
                app_name: Some("app2".to_string()),
                total_cpu_share: Some(0.2),
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: Some(100),
                has_gui_window: false,
                is_focused_group: false,
                tags: vec![],
                priority_class: None,
            },
        ];

        snapshot.app_groups = app_groups.clone();

        let results = ranker.rank(&app_groups, &snapshot);

        assert_eq!(results.len(), 2);
        let result1 = results.get("group1").unwrap();
        let result2 = results.get("group2").unwrap();

        // Одинаковые характеристики должны дать одинаковый score
        assert_eq!(result1.score, result2.score);

        // Обе группы должны иметь валидные rank и percentile
        assert!(result1.rank >= 1 && result1.rank <= 2);
        assert!(result2.rank >= 1 && result2.rank <= 2);
        assert!((0.0..=1.0).contains(&result1.percentile));
        assert!((0.0..=1.0).contains(&result2.percentile));
    }

    #[test]
    fn test_stub_ranker_boundary_values() {
        let ranker = StubRanker::new();
        let mut snapshot = create_test_snapshot();

        // Группа с максимальным score (focused + GUI + высокий CPU)
        let app_groups = vec![
            AppGroupRecord {
                app_group_id: "max_score".to_string(),
                root_pid: 1000,
                process_ids: vec![1000],
                app_name: Some("app".to_string()),
                total_cpu_share: Some(0.5), // > 0.3 для бонуса
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: Some(500),
                has_gui_window: true,
                is_focused_group: true,
                tags: vec![],
                priority_class: None,
            },
            // Группа с минимальным score (ничего особенного)
            AppGroupRecord {
                app_group_id: "min_score".to_string(),
                root_pid: 2000,
                process_ids: vec![2000],
                app_name: Some("app".to_string()),
                total_cpu_share: Some(0.1), // < 0.3, без бонуса
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: Some(50),
                has_gui_window: false,
                is_focused_group: false,
                tags: vec![],
                priority_class: None,
            },
        ];

        snapshot.app_groups = app_groups.clone();

        let results = ranker.rank(&app_groups, &snapshot);

        assert_eq!(results.len(), 2);
        let max_result = results.get("max_score").unwrap();
        let min_result = results.get("min_score").unwrap();

        // Максимальный score должен быть выше минимального
        assert!(max_result.score > min_result.score);
        assert_eq!(max_result.rank, 1);
        assert_eq!(min_result.rank, 2);

        // Проверяем, что scores в допустимом диапазоне [0.0, 1.0]
        assert!((0.0..=1.0).contains(&max_result.score));
        assert!((0.0..=1.0).contains(&min_result.score));

        // Percentile для максимального score должен быть выше
        assert!(max_result.percentile > min_result.percentile);
    }

    #[test]
    fn test_stub_ranker_multiple_groups() {
        let ranker = StubRanker::new();
        let mut snapshot = create_test_snapshot();

        // Создаём несколько групп с разными характеристиками
        let app_groups = vec![
            AppGroupRecord {
                app_group_id: "focused".to_string(),
                root_pid: 1000,
                process_ids: vec![1000],
                app_name: Some("focused".to_string()),
                total_cpu_share: Some(0.2),
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: Some(200),
                has_gui_window: true,
                is_focused_group: true,
                tags: vec![],
                priority_class: None,
            },
            AppGroupRecord {
                app_group_id: "gui".to_string(),
                root_pid: 2000,
                process_ids: vec![2000],
                app_name: Some("gui".to_string()),
                total_cpu_share: Some(0.2),
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: Some(200),
                has_gui_window: true,
                is_focused_group: false,
                tags: vec![],
                priority_class: None,
            },
            AppGroupRecord {
                app_group_id: "background".to_string(),
                root_pid: 3000,
                process_ids: vec![3000],
                app_name: Some("background".to_string()),
                total_cpu_share: Some(0.1),
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: Some(100),
                has_gui_window: false,
                is_focused_group: false,
                tags: vec![],
                priority_class: None,
            },
        ];

        snapshot.app_groups = app_groups.clone();

        let results = ranker.rank(&app_groups, &snapshot);

        assert_eq!(results.len(), 3);

        let focused_result = results.get("focused").unwrap();
        let gui_result = results.get("gui").unwrap();
        let background_result = results.get("background").unwrap();

        // Фокусная группа должна иметь самый высокий score
        assert!(focused_result.score > gui_result.score);
        assert!(focused_result.score > background_result.score);

        // GUI группа должна иметь более высокий score, чем фоновая
        assert!(gui_result.score > background_result.score);

        // Проверяем ранги
        assert_eq!(focused_result.rank, 1);
        assert_eq!(gui_result.rank, 2);
        assert_eq!(background_result.rank, 3);

        // Проверяем percentile (должны быть упорядочены)
        assert!(focused_result.percentile > gui_result.percentile);
        assert!(gui_result.percentile > background_result.percentile);

        // Все percentile должны быть в диапазоне [0.0, 1.0]
        assert!((0.0..=1.0).contains(&focused_result.percentile));
        assert!((0.0..=1.0).contains(&gui_result.percentile));
        assert!((0.0..=1.0).contains(&background_result.percentile));
    }
}
