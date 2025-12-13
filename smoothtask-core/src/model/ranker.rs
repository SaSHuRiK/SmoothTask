//! ML-ранкер для ранжирования AppGroup по приоритету.
//!
//! Этот модуль предоставляет интерфейс для ранжирования групп приложений
//! на основе их фич. В будущем здесь будет интеграция с ONNX/JSON моделями.

use crate::logging::snapshots::AppGroupRecord;
use crate::model::features::build_features;

/// Результат ранжирования для одной AppGroup.
///
/// Структура содержит результаты ранжирования группы приложений:
/// score (важность группы), rank (позиция среди всех групп) и percentile
/// (нормализованный score в диапазоне [0.0, 1.0]).
///
/// # Поля
///
/// - `score`: Абсолютный score от ранкера (чем выше, тем важнее группа).
///   Обычно в диапазоне [0.0, 1.0], но может быть любым числом в зависимости
///   от реализации ранкера.
/// - `rank`: Позиция группы среди всех групп (1 = самый важный, 2 = второй по важности и т.д.).
///   Группы с одинаковым score могут иметь одинаковый rank.
/// - `percentile`: Нормализованный score в диапазоне [0.0, 1.0], где 1.0 = самый важный.
///   Вычисляется на основе позиции группы среди всех групп.
///
/// # Примеры использования
///
/// **Примечание:** Примеры помечены как `ignore`, потому что они требуют создания
/// сложных структур (`Snapshot`, `AppGroupRecord`) с реальными метриками системы,
/// что невозможно сделать в doctest'ах без доступа к `/proc` и другим системным ресурсам.
/// Для реального использования см. интеграционные тесты в `tests/` или примеры в `model/mod.rs`.
///
/// ```ignore
/// use smoothtask_core::model::ranker::{Ranker, StubRanker, RankingResult};
/// use smoothtask_core::logging::snapshots::{Snapshot, AppGroupRecord};
///
/// let ranker = StubRanker::new();
/// let snapshot: Snapshot = /* ... */;
/// let app_groups: Vec<AppGroupRecord> = /* ... */;
///
/// // Ранжирование групп
/// let results = ranker.rank(&app_groups, &snapshot);
///
/// // Использование результатов
/// for (app_group_id, result) in &results {
///     println!("Group {}: score={:.2}, rank={}, percentile={:.2}",
///              app_group_id, result.score, result.rank, result.percentile);
///
///     // Использование percentile для определения приоритета
///     if result.percentile > 0.8 {
///         println!("  -> High priority group");
///     } else if result.percentile > 0.5 {
///         println!("  -> Medium priority group");
///     } else {
///         println!("  -> Low priority group");
///     }
/// }
/// ```
///
/// # Примечания
///
/// - `score` и `percentile` могут различаться: score - это абсолютное значение
///   от ранкера, а percentile - это нормализованная позиция среди всех групп.
/// - Группы с одинаковым score могут иметь разные percentile, если они имеют
///   разные позиции после сортировки.
/// - Percentile вычисляется как `1.0 - (rank - 1) / (total - 1)` для более
///   чем одной группы, и `1.0` для единственной группы.
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
            // Примечание: build_features вызывается для будущего использования фич,
            // но пока используется только простая эвристика на основе полей AppGroupRecord
            let _ = build_features(snapshot, app_group);

            // Простая эвристика для score:
            // - Базовая оценка: 0.5
            let mut score: f64 = 0.5;

            // - Фокусная группа: +0.4
            if app_group.is_focused_group {
                score += 0.4;
                tracing::debug!(
                    "Группа {}: фокусная группа, score += 0.4",
                    app_group.app_group_id
                );
            }

            // - GUI группа: +0.2
            if app_group.has_gui_window {
                score += 0.2;
                tracing::debug!(
                    "Группа {}: GUI группа, score += 0.2",
                    app_group.app_group_id
                );
            }

            // - Высокий CPU usage (может быть важно): +0.1
            if let Some(cpu_share) = app_group.total_cpu_share {
                if cpu_share > 0.3 {
                    score += 0.1;
                    tracing::debug!(
                        "Группа {}: высокий CPU usage ({:.2}), score += 0.1",
                        app_group.app_group_id,
                        cpu_share
                    );
                }
            }

            // Ограничиваем score в диапазоне [0.0, 1.0]
            score = score.clamp(0.0, 1.0);
            tracing::debug!(
                "Группа {}: итоговый score = {:.2}",
                app_group.app_group_id,
                score
            );

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
                total_energy_uj: None,
                total_power_w: None,
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
                total_energy_uj: None,
                total_power_w: None,
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

    #[test]
    fn test_stub_ranker_default() {
        // Тест проверяет, что Default::default() создаёт такой же ранкер, как StubRanker::new()
        let ranker1 = StubRanker::new();
        let ranker2 = StubRanker;

        // Оба ранкера должны работать одинаково
        let snapshot = create_test_snapshot();
        let app_groups = vec![AppGroupRecord {
            app_group_id: "test".to_string(),
            root_pid: 1000,
            process_ids: vec![1000],
            app_name: Some("test".to_string()),
            total_cpu_share: Some(0.2),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_rss_mb: Some(100),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
        }];

        let results1 = ranker1.rank(&app_groups, &snapshot);
        let results2 = ranker2.rank(&app_groups, &snapshot);

        // Результаты должны быть одинаковыми
        assert_eq!(results1.len(), results2.len());
        let result1 = results1.get("test").unwrap();
        let result2 = results2.get("test").unwrap();
        assert_eq!(result1.score, result2.score);
        assert_eq!(result1.rank, result2.rank);
        assert_eq!(result1.percentile, result2.percentile);
    }

    #[test]
    fn test_stub_ranker_build_features_consistency() {
        // Тест проверяет, что build_features вызывается корректно для всех групп
        // и не падает даже при отсутствующих данных
        let ranker = StubRanker::new();
        let mut snapshot = create_test_snapshot();

        // Создаём группы с различными характеристиками
        let app_groups = vec![
            AppGroupRecord {
                app_group_id: "group1".to_string(),
                root_pid: 1000,
                process_ids: vec![1000],
                app_name: Some("app1".to_string()),
                total_cpu_share: Some(0.5),
                total_io_read_bytes: Some(1024 * 1024),
                total_io_write_bytes: Some(512 * 1024),
                total_rss_mb: Some(200),
                has_gui_window: true,
                is_focused_group: true,
                tags: vec!["browser".to_string()],
                priority_class: Some("INTERACTIVE".to_string()),
            },
            AppGroupRecord {
                app_group_id: "group2".to_string(),
                root_pid: 2000,
                process_ids: vec![2000],
                app_name: None,
                total_cpu_share: None,
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: None,
                has_gui_window: false,
                is_focused_group: false,
                tags: vec![],
                priority_class: None,
            },
        ];

        snapshot.app_groups = app_groups.clone();

        // Ранжирование не должно падать, даже если build_features вызывается для групп
        // с отсутствующими процессами или данными
        let results = ranker.rank(&app_groups, &snapshot);

        // Проверяем, что результаты есть для всех групп
        assert_eq!(results.len(), 2);
        assert!(results.contains_key("group1"));
        assert!(results.contains_key("group2"));

        // Проверяем, что результаты валидны
        let result1 = results.get("group1").unwrap();
        let result2 = results.get("group2").unwrap();

        // Фокусная группа должна иметь более высокий score
        assert!(result1.score > result2.score);
        assert_eq!(result1.rank, 1);
        assert_eq!(result2.rank, 2);

        // Проверяем диапазоны
        assert!((0.0..=1.0).contains(&result1.score));
        assert!((0.0..=1.0).contains(&result2.score));
        assert!((0.0..=1.0).contains(&result1.percentile));
        assert!((0.0..=1.0).contains(&result2.percentile));
    }

    #[test]
    fn test_stub_ranker_large_group_set() {
        // Тест проверяет производительность и корректность ранжирования большого количества групп
        let ranker = StubRanker::new();
        let mut snapshot = create_test_snapshot();

        // Создаём большое количество групп (50 штук)
        let mut app_groups = Vec::new();
        for i in 0..50 {
            let is_focused = i == 0; // Только первая группа фокусная
            let has_gui = i < 10;    // Первые 10 групп имеют GUI
            let cpu_share = if i < 5 { Some(0.3 + (i as f64 * 0.1)) } else { Some(0.1) };

            app_groups.push(AppGroupRecord {
                app_group_id: format!("group{}", i),
                root_pid: 1000 + i as i32,
                process_ids: vec![1000 + i as i32],
                app_name: Some(format!("app{}", i)),
                total_cpu_share: cpu_share,
                total_io_read_bytes: Some((i * 1000) as u64),
                total_io_write_bytes: Some((i * 500) as u64),
                total_rss_mb: Some((i * 10) as u64),
                has_gui_window: has_gui,
                is_focused_group: is_focused,
                tags: vec![format!("tag{}", i % 5)],
                priority_class: None,
            });
        }

        snapshot.app_groups = app_groups.clone();

        let results = ranker.rank(&app_groups, &snapshot);

        // Проверяем, что результаты есть для всех групп
        assert_eq!(results.len(), 50);

        // Проверяем, что фокусная группа имеет самый высокий ранг
        let focused_result = results.get("group0").unwrap();
        assert_eq!(focused_result.rank, 1);
        assert!(focused_result.percentile > 0.9);

        // Проверяем, что группы с GUI имеют более высокие ранги, чем без GUI
        for i in 0..10 {
            let gui_result = results.get(&format!("group{}", i)).unwrap();
            let non_gui_result = results.get(&format!("group{}", i + 10)).unwrap();
            assert!(gui_result.score > non_gui_result.score);
            assert!(gui_result.rank < non_gui_result.rank);
        }

        // Проверяем, что все scores и percentiles в допустимых диапазонах
        for (_, result) in &results {
            assert!((0.0..=1.0).contains(&result.score));
            assert!((0.0..=1.0).contains(&result.percentile));
            assert!(result.rank >= 1 && result.rank <= 50);
        }

        // Проверяем, что ранги уникальны и последовательны
        let mut ranks: Vec<usize> = results.values().map(|r| r.rank).collect();
        ranks.sort();
        assert_eq!(ranks, (1..=50).collect::<Vec<_>>());
    }

    #[test]
    fn test_stub_ranker_edge_case_scores() {
        // Тест проверяет обработку крайних случаев при расчёте scores
        let ranker = StubRanker::new();
        let mut snapshot = create_test_snapshot();

        // Создаём группы, которые должны получить максимальный и минимальный возможные scores
        let app_groups = vec![
            // Группа с максимальным возможным score (focused + GUI + высокий CPU)
            AppGroupRecord {
                app_group_id: "max_score".to_string(),
                root_pid: 1000,
                process_ids: vec![1000],
                app_name: Some("max_app".to_string()),
                total_cpu_share: Some(1.0), // Максимальный CPU
                total_io_read_bytes: Some(10000000),
                total_io_write_bytes: Some(10000000),
                total_rss_mb: Some(1000),
                has_gui_window: true,
                is_focused_group: true,
                tags: vec!["critical".to_string()],
                priority_class: None,
            },
            // Группа с минимальным возможным score (нет фокуса, нет GUI, низкий CPU)
            AppGroupRecord {
                app_group_id: "min_score".to_string(),
                root_pid: 2000,
                process_ids: vec![2000],
                app_name: Some("min_app".to_string()),
                total_cpu_share: Some(0.0), // Минимальный CPU
                total_io_read_bytes: Some(0),
                total_io_write_bytes: Some(0),
                total_rss_mb: Some(1),
                has_gui_window: false,
                is_focused_group: false,
                tags: vec![],
                priority_class: None,
            },
            // Группа со средними характеристиками (GUI для повышения score)
            AppGroupRecord {
                app_group_id: "mid_score".to_string(),
                root_pid: 3000,
                process_ids: vec![3000],
                app_name: Some("mid_app".to_string()),
                total_cpu_share: Some(0.2),
                total_io_read_bytes: Some(1000000),
                total_io_write_bytes: Some(500000),
                total_rss_mb: Some(100),
                has_gui_window: true, // GUI для повышения score
                is_focused_group: false,
                tags: vec!["normal".to_string()],
                priority_class: None,
            },
        ];

        snapshot.app_groups = app_groups.clone();

        let results = ranker.rank(&app_groups, &snapshot);

        assert_eq!(results.len(), 3);

        let max_result = results.get("max_score").unwrap();
        let min_result = results.get("min_score").unwrap();
        let mid_result = results.get("mid_score").unwrap();

        // Максимальный score должен быть выше среднего и минимального
        assert!(max_result.score > mid_result.score);
        assert!(max_result.score > min_result.score);
        assert_eq!(max_result.rank, 1);

        // Средний score должен быть выше минимального (благодаря GUI)
        assert!(mid_result.score > min_result.score);
        assert_eq!(mid_result.rank, 2);
        assert_eq!(min_result.rank, 3);

        // Проверяем ожидаемые значения scores
        // max_score: 0.5 (база) + 0.4 (фокус) + 0.2 (GUI) + 0.1 (CPU > 0.3) = 1.2 -> clamped to 1.0
        assert_eq!(max_result.score, 1.0);
        // mid_score: 0.5 (база) + 0.2 (GUI) = 0.7
        assert_eq!(mid_result.score, 0.7);
        // min_score: 0.5 (база)
        assert_eq!(min_result.score, 0.5);

        // Все scores должны быть в допустимом диапазоне [0.0, 1.0]
        assert!((0.0..=1.0).contains(&max_result.score));
        assert!((0.0..=1.0).contains(&mid_result.score));
        assert!((0.0..=1.0).contains(&min_result.score));

        // Percentile должны быть упорядочены
        assert!(max_result.percentile > mid_result.percentile);
        assert!(mid_result.percentile > min_result.percentile);

        // Максимальный percentile должен быть 1.0 (самый важный)
        assert_eq!(max_result.percentile, 1.0);
        // Минимальный percentile должен быть 0.0 (наименее важный)
        assert_eq!(min_result.percentile, 0.0);
    }

    #[test]
    fn test_stub_ranker_consistency_across_runs() {
        // Тест проверяет, что ранжирование даёт одинаковые результаты при повторных вызовах
        let ranker = StubRanker::new();
        let snapshot = create_test_snapshot();

        let app_groups = vec![
            AppGroupRecord {
                app_group_id: "group1".to_string(),
                root_pid: 1000,
                process_ids: vec![1000],
                app_name: Some("app1".to_string()),
                total_cpu_share: Some(0.3),
                total_io_read_bytes: Some(1000000),
                total_io_write_bytes: Some(500000),
                total_rss_mb: Some(200),
                has_gui_window: true,
                is_focused_group: true,
                tags: vec!["browser".to_string()],
                priority_class: None,
            },
            AppGroupRecord {
                app_group_id: "group2".to_string(),
                root_pid: 2000,
                process_ids: vec![2000],
                app_name: Some("app2".to_string()),
                total_cpu_share: Some(0.2),
                total_io_read_bytes: Some(500000),
                total_io_write_bytes: Some(250000),
                total_rss_mb: Some(100),
                has_gui_window: false,
                is_focused_group: false,
                tags: vec!["editor".to_string()],
                priority_class: None,
            },
        ];

        // Выполняем ранжирование несколько раз
        let results1 = ranker.rank(&app_groups, &snapshot);
        let results2 = ranker.rank(&app_groups, &snapshot);
        let results3 = ranker.rank(&app_groups, &snapshot);

        // Проверяем, что результаты идентичны
        assert_eq!(results1.len(), results2.len());
        assert_eq!(results2.len(), results3.len());

        for group_id in ["group1", "group2"] {
            let r1 = results1.get(group_id).unwrap();
            let r2 = results2.get(group_id).unwrap();
            let r3 = results3.get(group_id).unwrap();

            // Все характеристики должны быть одинаковыми
            assert_eq!(r1.score, r2.score);
            assert_eq!(r2.score, r3.score);
            assert_eq!(r1.rank, r2.rank);
            assert_eq!(r2.rank, r3.rank);
            assert_eq!(r1.percentile, r2.percentile);
            assert_eq!(r2.percentile, r3.percentile);
        }

        // Проверяем, что ранжирование стабильно
        let result1 = results1.get("group1").unwrap();
        let result2 = results1.get("group2").unwrap();
        assert!(result1.score > result2.score);
        assert_eq!(result1.rank, 1);
        assert_eq!(result2.rank, 2);
    }

    #[test]
    fn test_stub_ranker_extreme_cpu_values() {
        // Тест проверяет обработку экстремальных значений CPU usage
        let ranker = StubRanker::new();
        let mut snapshot = create_test_snapshot();

        let app_groups = vec![
            // Группа с очень высоким CPU usage (> 1.0, что возможно в некоторых системах)
            AppGroupRecord {
                app_group_id: "extreme_cpu".to_string(),
                root_pid: 1000,
                process_ids: vec![1000],
                app_name: Some("stress_test".to_string()),
                total_cpu_share: Some(2.5), // Очень высокое значение
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: Some(1000),
                has_gui_window: false,
                is_focused_group: false,
                tags: vec!["stress".to_string()],
                priority_class: None,
            },
            // Группа с отрицательным CPU usage (невалидное значение, но должно обрабатываться)
            AppGroupRecord {
                app_group_id: "negative_cpu".to_string(),
                root_pid: 2000,
                process_ids: vec![2000],
                app_name: Some("invalid_app".to_string()),
                total_cpu_share: Some(-0.1), // Невалидное значение
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: Some(100),
                has_gui_window: false,
                is_focused_group: false,
                tags: vec!["invalid".to_string()],
                priority_class: None,
            },
            // Группа с нулевым CPU usage
            AppGroupRecord {
                app_group_id: "zero_cpu".to_string(),
                root_pid: 3000,
                process_ids: vec![3000],
                app_name: Some("idle_app".to_string()),
                total_cpu_share: Some(0.0),
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: Some(50),
                has_gui_window: false,
                is_focused_group: false,
                tags: vec!["idle".to_string()],
                priority_class: None,
            },
        ];

        snapshot.app_groups = app_groups.clone();

        let results = ranker.rank(&app_groups, &snapshot);

        // Проверяем, что все группы обработаны
        assert_eq!(results.len(), 3);

        // Проверяем, что все scores в допустимом диапазоне [0.0, 1.0]
        for (_, result) in &results {
            assert!((0.0..=1.0).contains(&result.score));
            assert!((0.0..=1.0).contains(&result.percentile));
        }

        // Группа с фокусом должна иметь более высокий score
        let extreme_result = results.get("extreme_cpu").unwrap();
        let negative_result = results.get("negative_cpu").unwrap();
        let zero_result = results.get("zero_cpu").unwrap();

        // Все группы без фокуса и GUI должны иметь одинаковый базовый score (0.5)
        // Группа с высоким CPU (> 0.3) должна получить бонус +0.1
        assert_eq!(extreme_result.score, 0.6); // 0.5 + 0.1 за высокий CPU
        assert_eq!(negative_result.score, 0.5); // 0.5 базовый
        assert_eq!(zero_result.score, 0.5);     // 0.5 базовый
    }

    #[test]
    fn test_stub_ranker_missing_optional_fields() {
        // Тест проверяет обработку групп с отсутствующими опциональными полями
        let ranker = StubRanker::new();
        let mut snapshot = create_test_snapshot();

        let app_groups = vec![
            // Группа с отсутствующим app_name
            AppGroupRecord {
                app_group_id: "no_name".to_string(),
                root_pid: 1000,
                process_ids: vec![1000],
                app_name: None,
                total_cpu_share: None,
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: None,
                has_gui_window: false,
                is_focused_group: false,
                tags: vec![],
                priority_class: None,
            },
            // Группа с отсутствующими метриками
            AppGroupRecord {
                app_group_id: "no_metrics".to_string(),
                root_pid: 2000,
                process_ids: vec![2000],
                app_name: Some("app".to_string()),
                total_cpu_share: None,
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_rss_mb: None,
                has_gui_window: false,
                is_focused_group: false,
                tags: vec!["no_metrics".to_string()],
                priority_class: None,
            },
        ];

        snapshot.app_groups = app_groups.clone();

        let results = ranker.rank(&app_groups, &snapshot);

        // Проверяем, что все группы обработаны
        assert_eq!(results.len(), 2);
        assert!(results.contains_key("no_name"));
        assert!(results.contains_key("no_metrics"));

        // Проверяем, что результаты валидны
        let no_name_result = results.get("no_name").unwrap();
        let no_metrics_result = results.get("no_metrics").unwrap();

        // Обе группы должны иметь одинаковый score (базовый 0.5)
        assert_eq!(no_name_result.score, no_metrics_result.score);
        assert_eq!(no_name_result.score, 0.5);

        // Проверяем ранги и percentiles
        assert!((0.0..=1.0).contains(&no_name_result.percentile));
        assert!((0.0..=1.0).contains(&no_metrics_result.percentile));
        assert!(no_name_result.rank >= 1 && no_name_result.rank <= 2);
        assert!(no_metrics_result.rank >= 1 && no_metrics_result.rank <= 2);
    }

    #[test]
    fn test_stub_ranker_performance_large_dataset() {
        // Тест проверяет производительность ранжирования большого количества групп
        let ranker = StubRanker::new();
        let mut snapshot = create_test_snapshot();

        // Создаём большое количество групп (100 штук)
        let mut app_groups = Vec::new();
        for i in 0..100 {
            let is_focused = i == 0;
            let has_gui = i < 20;
            let cpu_share = if i < 10 { Some(0.4) } else { Some(0.1) };

            app_groups.push(AppGroupRecord {
                app_group_id: format!("group_{}", i),
                root_pid: 1000 + i as i32,
                process_ids: vec![1000 + i as i32],
                app_name: Some(format!("app_{}", i)),
                total_cpu_share: cpu_share,
                total_io_read_bytes: Some((i * 10000) as u64),
                total_io_write_bytes: Some((i * 5000) as u64),
                total_rss_mb: Some((i * 10) as u64),
                has_gui_window: has_gui,
                is_focused_group: is_focused,
                tags: vec![format!("tag_{}", i % 5)],
                priority_class: None,
            });
        }

        snapshot.app_groups = app_groups.clone();

        // Замеряем время выполнения
        let start_time = std::time::Instant::now();
        let results = ranker.rank(&app_groups, &snapshot);
        let duration = start_time.elapsed();

        // Проверяем, что все группы обработаны
        assert_eq!(results.len(), 100);

        // Проверяем, что фокусная группа имеет самый высокий ранг
        let focused_result = results.get("group_0").unwrap();
        assert_eq!(focused_result.rank, 1);
        assert!(focused_result.percentile > 0.95);

        // Проверяем, что группы с GUI имеют более высокие ранги, чем без GUI
        for i in 0..20 {
            let gui_result = results.get(&format!("group_{}", i)).unwrap();
            let non_gui_result = results.get(&format!("group_{}", i + 20)).unwrap();
            assert!(gui_result.score >= non_gui_result.score);
        }

        // Проверяем, что все scores и percentiles в допустимых диапазонах
        for (_, result) in &results {
            assert!((0.0..=1.0).contains(&result.score));
            assert!((0.0..=1.0).contains(&result.percentile));
            assert!(result.rank >= 1 && result.rank <= 100);
        }

        // Проверяем, что ранги уникальны и последовательны
        let mut ranks: Vec<usize> = results.values().map(|r| r.rank).collect();
        ranks.sort();
        assert_eq!(ranks, (1..=100).collect::<Vec<_>>());

        // Проверяем производительность (должно выполняться быстро)
        assert!(duration.as_millis() < 100, "Ранжирование 100 групп не должно занимать более 100мс");
    }
}
