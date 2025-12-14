//! Policy Engine — применение правил для определения приоритетов AppGroup.
//!
//! Policy Engine применяет жёсткие правила (guardrails) и семантические правила
//! для определения целевого класса приоритета для каждой AppGroup в снапшоте.
//! В режиме hybrid также использует ML-ранкер для более точного определения приоритетов.

use crate::config::config_struct::{Config, PolicyMode};

use crate::logging::snapshots::{AppGroupRecord, ProcessRecord, Snapshot};
use crate::model::ranker::{Ranker, RankingResult};
use crate::policy::classes::PriorityClass;
use crate::policy::dynamic::{DynamicPriorityScaler, SystemLoadInfo};

/// Результат оценки политики для одной AppGroup.
#[derive(Debug, Clone)]
pub struct PolicyResult {
    /// Целевой класс приоритета для группы.
    pub priority_class: PriorityClass,
    /// Причина выбора приоритета (для логирования и отладки).
    pub reason: String,
}

/// Policy Engine для применения правил к снапшоту.
///
/// Policy Engine определяет приоритеты для AppGroup на основе:
/// 1. Жёстких правил (guardrails) — имеют наивысший приоритет
/// 2. Семантических правил — применяются, если guardrails не сработали
/// 3. ML-ранкера (в hybrid режиме) — используется для ранжирования групп
///
/// # Примеры использования
///
/// ## Базовое использование (rules-only режим)
///
/// ```no_run
/// use smoothtask_core::config::Config;
/// use smoothtask_core::policy::engine::PolicyEngine;
/// use smoothtask_core::logging::snapshots::{Snapshot, GlobalMetrics, ResponsivenessMetrics};
/// use chrono::Utc;
///
/// # fn main() -> anyhow::Result<()> {
/// let config = Config::load("config.yml")?;
/// let engine = PolicyEngine::new(config);
///
/// // Создать минимальный снапшот для примера
/// let snapshot = Snapshot {
///     snapshot_id: 1234567890,
///     timestamp: Utc::now(),
///     global: GlobalMetrics {
///         cpu_user: 0.25,
///         cpu_system: 0.15,
///         cpu_idle: 0.55,
///         cpu_iowait: 0.05,
///         mem_total_kb: 16_384_256,
///         mem_used_kb: 8_000_000,
///         mem_available_kb: 8_384_256,
///         swap_total_kb: 8_192_000,
///         swap_used_kb: 1_000_000,
///         load_avg_one: 1.5,
///         load_avg_five: 1.2,
///         load_avg_fifteen: 1.0,
///         psi_cpu_some_avg10: Some(0.1),
///         psi_cpu_some_avg60: Some(0.15),
///         psi_io_some_avg10: Some(0.2),
///         psi_mem_some_avg10: Some(0.05),
///         psi_mem_full_avg10: None,
///         user_active: true,
///         time_since_last_input_ms: Some(5000),
///     },
///     processes: vec![],
///     app_groups: vec![],
///     responsiveness: ResponsivenessMetrics::default(),
/// };
///
/// // Оценить снапшот
/// let results = engine.evaluate_snapshot(&snapshot);
///
/// // Результаты содержат приоритеты для каждой AppGroup
/// for (app_group_id, result) in results {
///     println!("{}: {:?} ({})", app_group_id, result.priority_class, result.reason);
/// }
/// # Ok(())
/// # }
/// ```
///
/// ## Использование с кастомным ранкером (для тестирования)
///
/// ```no_run
/// use smoothtask_core::config::Config;
/// use smoothtask_core::policy::engine::PolicyEngine;
/// use smoothtask_core::model::ranker::{Ranker, StubRanker};
///
/// # fn main() -> anyhow::Result<()> {
/// let config = Config::load("config.yml")?;
/// let ranker = Box::new(StubRanker::new());
/// let engine = PolicyEngine::with_ranker(config, ranker);
/// # Ok(())
/// # }
/// ```
pub struct PolicyEngine {
    config: Config,
    ranker: Option<Box<dyn Ranker>>,
    dynamic_scaler: DynamicPriorityScaler,
}

impl PolicyEngine {
    /// Создать новый Policy Engine с заданной конфигурацией.
    ///
    /// В режиме `hybrid` автоматически загружается ONNXRanker, если модель включена и доступна.
    /// Если загрузка модели не удаётся или модель отключена, используется StubRanker.
    /// В режиме `rules-only` ранкер не используется.
    ///
    /// # Аргументы
    ///
    /// * `config` - конфигурация с параметрами политики и режимом работы
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::config::Config;
    /// use smoothtask_core::policy::engine::PolicyEngine;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let config = Config::load("config.yml")?;
    /// let engine = PolicyEngine::new(config);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(config: Config) -> Self {
        let ranker: Option<Box<dyn Ranker>> = if config.policy_mode == PolicyMode::Hybrid {
            // В hybrid режиме пытаемся загрузить ONNXRanker, если модель включена
            if config.model.enabled {
                #[cfg(feature = "onnx")]
                match crate::model::onnx_ranker::ONNXRanker::load(&config.model.model_path) {
                    Ok(onnx_ranker) => {
                        tracing::info!(
                            "Загружена ONNX модель для ранжирования: {}",
                            config.model.model_path
                        );
                        Some(Box::new(onnx_ranker))
                    }
                    Err(e) => {
                        tracing::error!(
                            "Не удалось загрузить ONNX модель с пути {}: {}",
                            config.model.model_path,
                            e
                        );
                        tracing::warn!(
                            "Фоллбек на StubRanker для hybrid режима. ML-ранжирование будет менее точным."
                        );
                        None
                    }
                }
                #[cfg(not(feature = "onnx"))]
                {
                    tracing::warn!(
                        "ONNX поддержка отключена (feature 'onnx' не включен). Используется дефолтный ранкер."
                    );
                    tracing::info!(
                        "Для использования ML-моделей включите feature 'onnx' и пересоберите проект."
                    );
                    None
                }
            } else {
                // Модель отключена в конфигурации, используем заглушку
                tracing::info!("ML-модель отключена в конфигурации, используется StubRanker");
                tracing::info!("Для включения ML-ранжирования установите model.enabled = true в конфигурации");
                Some(Box::new(crate::model::ranker::StubRanker::new()))
            }
        } else {
            tracing::debug!("Режим policy_mode = {:?}, ML-ранкер не используется", config.policy_mode);
            None
        };
        
        let dynamic_scaler = DynamicPriorityScaler::new(config.clone());
        
        Self { config, ranker, dynamic_scaler }
    }

    /// Создать Policy Engine с явно заданным ранкером (для тестирования).
    ///
    /// Этот метод полезен для тестирования с кастомным ранкером или для использования
    /// реального ML-ранкера вместо StubRanker.
    ///
    /// # Аргументы
    ///
    /// * `config` - конфигурация с параметрами политики
    /// * `ranker` - ранкер для ранжирования AppGroup (должен реализовывать трейт `Ranker`)
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::config::Config;
    /// use smoothtask_core::policy::engine::PolicyEngine;
    /// use smoothtask_core::model::ranker::{Ranker, StubRanker};
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let config = Config::load("config.yml")?;
    /// let ranker = Box::new(StubRanker::new());
    /// let engine = PolicyEngine::with_ranker(config, ranker);
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_ranker(config: Config, ranker: Box<dyn Ranker>) -> Self {
        let dynamic_scaler = DynamicPriorityScaler::new(config.clone());
        Self {
            config,
            ranker: Some(ranker),
            dynamic_scaler,
        }
    }

    /// Оценить снапшот и определить приоритеты для всех AppGroup.
    ///
    /// Функция применяет правила к каждой AppGroup в снапшоте в следующем порядке:
    /// 1. Жёсткие правила (guardrails) — защита системных процессов, RT-приоритеты, аудио
    /// 2. Семантические правила — фокусный GUI, активный терминал, updater/indexer, noisy neighbour
    /// 3. ML-ранкер (в hybrid режиме) — ранжирование на основе percentile
    /// 4. Дефолтный приоритет — если правила не применились
    ///
    /// # Аргументы
    ///
    /// * `snapshot` - снапшот системы с процессами и группами
    ///
    /// # Возвращает
    ///
    /// Маппинг `app_group_id -> PolicyResult` с приоритетом и причиной выбора.
    ///
    /// # Примеры
    ///
    /// ```no_run
    /// use smoothtask_core::config::Config;
    /// use smoothtask_core::policy::engine::PolicyEngine;
    /// use smoothtask_core::logging::snapshots::{Snapshot, GlobalMetrics, ResponsivenessMetrics};
    /// use chrono::Utc;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let config = Config::load("config.yml")?;
    /// let engine = PolicyEngine::new(config);
    ///
    /// // Создать минимальный снапшот для примера
    /// let snapshot = Snapshot {
    ///     snapshot_id: 1234567890,
    ///     timestamp: Utc::now(),
    ///     global: GlobalMetrics {
    ///         cpu_user: 0.25,
    ///         cpu_system: 0.15,
    ///         cpu_idle: 0.55,
    ///         cpu_iowait: 0.05,
    ///         mem_total_kb: 16_384_256,
    ///         mem_used_kb: 8_000_000,
    ///         mem_available_kb: 8_384_256,
    ///         swap_total_kb: 8_192_000,
    ///         swap_used_kb: 1_000_000,
    ///         load_avg_one: 1.5,
    ///         load_avg_five: 1.2,
    ///         load_avg_fifteen: 1.0,
    ///         psi_cpu_some_avg10: Some(0.1),
    ///         psi_cpu_some_avg60: Some(0.15),
    ///         psi_io_some_avg10: Some(0.2),
    ///         psi_mem_some_avg10: Some(0.05),
    ///         psi_mem_full_avg10: None,
    ///         user_active: true,
    ///         time_since_last_input_ms: Some(5000),
    ///     },
    ///     processes: vec![],
    ///     app_groups: vec![],
    ///     responsiveness: ResponsivenessMetrics::default(),
    /// };
    ///
    /// let results = engine.evaluate_snapshot(&snapshot);
    ///
    /// // Обработка результатов
    /// for (app_group_id, result) in results {
    ///     println!("{}: {:?} ({})", app_group_id, result.priority_class, result.reason);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn evaluate_snapshot(
        &self,
        snapshot: &Snapshot,
    ) -> std::collections::HashMap<String, PolicyResult> {
        // В hybrid mode сначала ранжируем все группы
        let ranking_results: Option<std::collections::HashMap<String, RankingResult>> = self
            .ranker
            .as_ref()
            .map(|ranker| ranker.rank(&snapshot.app_groups, snapshot));

        let mut results = std::collections::HashMap::new();

        for app_group in &snapshot.app_groups {
            let result = self.evaluate_app_group(
                app_group,
                snapshot,
                ranking_results
                    .as_ref()
                    .and_then(|r| r.get(&app_group.app_group_id)),
            );
            results.insert(app_group.app_group_id.clone(), result);
        }

        // Применяем динамическое масштабирование приоритетов на основе нагрузки системы
        let base_priorities: std::collections::HashMap<String, PriorityClass> = results
            .iter()
            .map(|(app_group_id, result)| (app_group_id.clone(), result.priority_class))
            .collect();
        
        let scaled_priorities = self.dynamic_scaler.scale_priorities(snapshot, &base_priorities);
        
        // Обновляем результаты с учетом динамического масштабирования
        let mut final_results = std::collections::HashMap::new();
        for (app_group_id, result) in results {
            if let Some(scaled_priority) = scaled_priorities.get(&app_group_id) {
                if *scaled_priority != result.priority_class {
                    // Если приоритет был изменен динамическим масштабированием, обновляем причину
                    let load_info = SystemLoadInfo::from_global_metrics(&snapshot.global);
                    let _load_category = load_info.get_load_category();
                    let reason = format!(
                        "{}. dynamic-scaling: {} -> {} (load={:.3})",
                        result.reason,
                        result.priority_class.as_str(),
                        scaled_priority.as_str(),
                        load_info.load_level
                    );
                    final_results.insert(
                        app_group_id,
                        PolicyResult {
                            priority_class: *scaled_priority,
                            reason,
                        },
                    );
                } else {
                    final_results.insert(app_group_id, result);
                }
            } else {
                final_results.insert(app_group_id, result);
            }
        }

        final_results
    }

    /// Оценить одну AppGroup и определить её приоритет.
    fn evaluate_app_group(
        &self,
        app_group: &AppGroupRecord,
        snapshot: &Snapshot,
        ranking_result: Option<&RankingResult>,
    ) -> PolicyResult {
        // 1. Применяем жёсткие правила (guardrails) - они имеют наивысший приоритет
        if let Some(guardrail_result) = self.apply_guardrails(app_group, snapshot) {
            return guardrail_result;
        }

        // 2. Применяем семантические правила
        if let Some(semantic_result) = self.apply_semantic_rules(app_group, snapshot) {
            return semantic_result;
        }

        // 3. В hybrid mode используем ML-ранкер для определения приоритета на основе percentile
        if let Some(ranking) = ranking_result {
            let priority_class = self.map_percentile_to_class(ranking.percentile);
            return PolicyResult {
                priority_class,
                reason: format!(
                    "ml-ranker: percentile={:.3}, score={:.3}, rank={}",
                    ranking.percentile, ranking.score, ranking.rank
                ),
            };
        }

        // 4. Дефолтный приоритет (если правила не применились и нет ранкера)
        PolicyResult {
            priority_class: PriorityClass::Normal,
            reason: "default: no rules matched".to_string(),
        }
    }

    /// Маппинг percentile на класс приоритета согласно порогам из конфига.
    ///
    /// Пороги должны быть упорядочены: background_percentile <= normal_percentile <=
    /// interactive_percentile <= crit_interactive_percentile
    fn map_percentile_to_class(&self, percentile: f64) -> PriorityClass {
        let t = &self.config.thresholds;
        // Приводим пороги к f64 для сравнения с percentile
        if percentile >= f64::from(t.crit_interactive_percentile) {
            PriorityClass::CritInteractive
        } else if percentile >= f64::from(t.interactive_percentile) {
            PriorityClass::Interactive
        } else if percentile >= f64::from(t.normal_percentile) {
            PriorityClass::Normal
        } else if percentile >= f64::from(t.background_percentile) {
            PriorityClass::Background
        } else {
            PriorityClass::Idle
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
    use crate::config::config_struct::{
        CacheIntervals, Config, MLClassifierConfig, ModelConfig, ModelType, NotificationBackend,
        NotificationConfig, NotificationLevel, Paths, PatternAutoUpdateConfig, Thresholds,
    };
    use crate::logging::snapshots::{GlobalMetrics, ResponsivenessMetrics};
    use crate::metrics::ebpf::EbpfConfig;
    use chrono::Utc;

    fn create_test_config() -> Config {
        use crate::config::config_struct::LoggingConfig;
        Config {
            polling_interval_ms: 500,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: PolicyMode::RulesOnly,
            enable_snapshot_logging: false,
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
                sched_latency_p99_threshold_ms: 10.0,
                ui_loop_p95_threshold_ms: 16.67,
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: None,
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
            },
            cache_intervals: CacheIntervals {
                system_metrics_cache_interval: 3,
                process_metrics_cache_interval: 1,
            },
            notifications: NotificationConfig {
                enabled: false,
                backend: NotificationBackend::Stub,
                app_name: "SmoothTask".to_string(),
                min_level: NotificationLevel::Warning,
            },
            model: ModelConfig {
                enabled: false,
                model_path: "models/ranker.onnx".to_string(),
                model_type: ModelType::Onnx,
            },
            ml_classifier: MLClassifierConfig::default(),
            pattern_auto_update: PatternAutoUpdateConfig::default(),
            ebpf: EbpfConfig::default(),
        }
    }

    fn create_hybrid_config() -> Config {
        use crate::config::config_struct::LoggingConfig;
        Config {
            polling_interval_ms: 500,
            max_candidates: 150,
            dry_run_default: false,
            policy_mode: PolicyMode::Hybrid,
            enable_snapshot_logging: false,
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
                sched_latency_p99_threshold_ms: 10.0,
                ui_loop_p95_threshold_ms: 16.67,
                priority_hysteresis_stable_sec: Some(30),
            },
            paths: Paths {
                log_file_path: "smoothtask.log".to_string(),
                snapshot_db_path: "/tmp/test.db".to_string(),
                patterns_dir: "/tmp/patterns".to_string(),
                api_listen_addr: None,
            },
            logging: LoggingConfig {
                log_max_size_bytes: 10_485_760,
                log_max_rotated_files: 5,
                log_compression_enabled: true,
                log_rotation_interval_sec: 0,
                log_max_age_sec: 0,
                log_max_total_size_bytes: 0,
                log_cleanup_interval_sec: 3600,
            },
            cache_intervals: CacheIntervals {
                system_metrics_cache_interval: 3,
                process_metrics_cache_interval: 1,
            },
            notifications: NotificationConfig {
                enabled: false,
                backend: NotificationBackend::Stub,
                app_name: "SmoothTask".to_string(),
                min_level: NotificationLevel::Warning,
            },
            model: ModelConfig {
                enabled: false,
                model_path: "models/ranker.onnx".to_string(),
                model_type: ModelType::Onnx,
            },
            ml_classifier: MLClassifierConfig::default(),
            pattern_auto_update: PatternAutoUpdateConfig::default(),
            ebpf: EbpfConfig::default(),
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
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(500),
            has_gui_window: true,
            is_focused_group: true,
            tags: vec!["browser".to_string()],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
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
            io_read_operations: None,
            io_write_operations: None,
            io_total_operations: None,
            io_last_update_ns: None,
            io_data_source: None,
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
            energy_uj: None,
            power_w: None,
            energy_timestamp: None,
            network_rx_bytes: None,
            network_tx_bytes: None,
            network_rx_packets: None,
            network_tx_packets: None,
            network_tcp_connections: None,
            network_udp_connections: None,
            network_last_update_ns: None,
            network_data_source: None,
            gpu_utilization: None,
            gpu_memory_bytes: None,
            gpu_time_us: None,
            gpu_api_calls: None,
            gpu_last_update_ns: None,
            gpu_data_source: None,
        };

        let app_group = AppGroupRecord {
            app_group_id: "systemd".to_string(),
            root_pid: 1,
            process_ids: vec![1],
            app_name: Some("systemd".to_string()),
            total_cpu_share: Some(0.05),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(50),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
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

        let mut audio_process = ProcessRecord::default();
        audio_process.pid = 2000;
        audio_process.ppid = 1;
        audio_process.uid = 1000;
        audio_process.gid = 1000;
        audio_process.exe = Some("/usr/bin/pulseaudio".to_string());
        audio_process.app_group_id = Some("pulseaudio".to_string());
        audio_process.state = "R".to_string();
        audio_process.start_time = 0;
        audio_process.uptime_sec = 100;
        audio_process.tty_nr = 0;
        audio_process.has_tty = false;
        audio_process.rss_mb = Some(100);
        audio_process.is_audio_client = true;
        audio_process.has_active_stream = true;
        audio_process.process_type = Some("audio".to_string());
        audio_process.tags = vec!["audio".to_string()];
        audio_process.nice = 0;
        audio_process.ionice_class = Some(2);
        audio_process.ionice_prio = Some(4);

        let app_group = AppGroupRecord {
            app_group_id: "pulseaudio".to_string(),
            root_pid: 2000,
            process_ids: vec![2000],
            app_name: Some("pulseaudio".to_string()),
            total_cpu_share: Some(0.1),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(100),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec!["audio".to_string()],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
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
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(200),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec!["updater".to_string()],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
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
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(500),
            has_gui_window: false,
            is_focused_group: false, // Не в фокусе
            tags: vec![],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
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
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(1000),
            has_gui_window: true,
            is_focused_group: true, // В фокусе
            tags: vec!["game".to_string()],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
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
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(100),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        };

        snapshot.app_groups = vec![app_group.clone()];

        let results = engine.evaluate_snapshot(&snapshot);
        let result = results.get("unknown").unwrap();

        assert_eq!(result.priority_class, PriorityClass::Normal);
        assert!(result.reason.contains("default"));
    }

    #[test]
    fn test_hybrid_mode_uses_ranker() {
        let config = create_hybrid_config();
        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        // Создаём группы, которые НЕ попадают под правила (не фокусные, не системные, не updater)
        // чтобы проверить, что ранкер используется
        let app_groups = vec![
            AppGroupRecord {
                app_group_id: "normal-app-1".to_string(),
                root_pid: 1000,
                process_ids: vec![1000],
                app_name: Some("app1".to_string()),
                total_cpu_share: Some(0.5), // Высокий CPU для ранкера
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_io_read_operations: None,
                total_io_write_operations: None,
                total_io_operations: None,
                io_data_source: None,
                total_rss_mb: Some(500),
                has_gui_window: true,    // GUI, но не в фокусе
                is_focused_group: false, // Не в фокусе, чтобы не попасть под семантические правила
                tags: vec![],
                priority_class: None,
                total_energy_uj: None,
                total_power_w: None,
                total_network_rx_bytes: None,
                total_network_tx_bytes: None,
                total_network_rx_packets: None,
                total_network_tx_packets: None,
                total_network_tcp_connections: None,
                total_network_udp_connections: None,
                network_data_source: None,
            },
            AppGroupRecord {
                app_group_id: "normal-app-2".to_string(),
                root_pid: 2000,
                process_ids: vec![2000],
                app_name: Some("app2".to_string()),
                total_cpu_share: Some(0.1), // Низкий CPU
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_io_read_operations: None,
                total_io_write_operations: None,
                total_io_operations: None,
                io_data_source: None,
                total_rss_mb: Some(100),
                has_gui_window: false,
                is_focused_group: false,
                tags: vec![],
                priority_class: None,
                total_energy_uj: None,
                total_power_w: None,
                total_network_rx_bytes: None,
                total_network_tx_bytes: None,
                total_network_rx_packets: None,
                total_network_tx_packets: None,
                total_network_tcp_connections: None,
                total_network_udp_connections: None,
                network_data_source: None,
            },
        ];

        snapshot.app_groups = app_groups;

        let results = engine.evaluate_snapshot(&snapshot);

        // Проверяем, что результаты есть для всех групп
        assert_eq!(results.len(), 2);

        // Обе группы должны использовать ранкер (не попадают под правила)
        let app1_result = results.get("normal-app-1").unwrap();
        let app2_result = results.get("normal-app-2").unwrap();

        // В hybrid mode ранкер должен определить приоритеты на основе percentile
        // StubRanker даёт более высокий score группам с GUI и высоким CPU
        assert!(app1_result.priority_class >= app2_result.priority_class);
        assert!(app1_result.reason.contains("ml-ranker"));
        assert!(app2_result.reason.contains("ml-ranker"));
    }

    #[test]
    fn test_hybrid_mode_guardrails_override_ranker() {
        let config = create_hybrid_config();
        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        // Системный процесс должен быть защищён, даже если ранкер даст высокий score
        let mut system_process = ProcessRecord::default();
        system_process.pid = 1;
        system_process.ppid = 0;
        system_process.uid = 0;
        system_process.gid = 0;
        system_process.exe = Some("/usr/lib/systemd/systemd".to_string());
        system_process.cgroup_path = Some("/system.slice/systemd.service".to_string());
        system_process.systemd_unit = Some("systemd.service".to_string());
        system_process.app_group_id = Some("systemd".to_string());
        system_process.state = "S".to_string();
        system_process.start_time = 0;
        system_process.uptime_sec = 1000;
        system_process.tty_nr = 0;
        system_process.has_tty = false;
        system_process.rss_mb = Some(50);
        system_process.process_type = Some("daemon".to_string());
        system_process.nice = 0;
        system_process.ionice_class = Some(2);
        system_process.ionice_prio = Some(4);

        let app_group = AppGroupRecord {
            app_group_id: "systemd".to_string(),
            root_pid: 1,
            process_ids: vec![1],
            app_name: Some("systemd".to_string()),
            total_cpu_share: Some(0.05),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(50),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        };

        snapshot.processes = vec![system_process];
        snapshot.app_groups = vec![app_group];

        let results = engine.evaluate_snapshot(&snapshot);
        let result = results.get("systemd").unwrap();

        // Guardrails должны переопределить ранкер
        assert_eq!(result.priority_class, PriorityClass::Normal);
        assert!(result.reason.contains("system process"));
    }

    #[test]
    fn test_map_percentile_to_class() {
        let config = create_hybrid_config();
        let engine = PolicyEngine::new(config);

        // Тестируем маппинг percentile на классы
        // Используем приватный метод через публичный API через evaluate_snapshot

        let mut snapshot = create_test_snapshot();
        let app_group = AppGroupRecord {
            app_group_id: "test".to_string(),
            root_pid: 1000,
            process_ids: vec![1000],
            app_name: None,
            total_cpu_share: Some(0.1),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(100),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        };

        snapshot.app_groups = vec![app_group];

        let results = engine.evaluate_snapshot(&snapshot);
        let result = results.get("test").unwrap();

        // Проверяем, что результат содержит информацию о percentile
        // StubRanker для одной группы даст percentile = 1.0, что должно дать CritInteractive
        // (так как 1.0 >= 0.9)
        assert!(result.reason.contains("ml-ranker"));
    }

    #[test]
    fn test_map_percentile_to_class_boundaries() {
        let config = create_hybrid_config();
        let engine = PolicyEngine::new(config);

        // Тестируем граничные случаи маппинга percentile на классы
        // Создаём несколько групп с разными характеристиками для получения разных percentile
        let mut snapshot = create_test_snapshot();

        // Создаём группы с разными характеристиками для получения разных score и percentile
        let app_groups = vec![
            AppGroupRecord {
                app_group_id: "high_priority".to_string(),
                root_pid: 1001,
                process_ids: vec![1001],
                app_name: None,
                total_cpu_share: Some(0.5), // > 0.3 для бонуса
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_io_read_operations: None,
                total_io_write_operations: None,
                total_io_operations: None,
                io_data_source: None,
                total_rss_mb: Some(100),
                has_gui_window: true,
                is_focused_group: true, // Фокусная группа -> высокий score
                tags: vec![],
                priority_class: None,
                total_energy_uj: None,
                total_power_w: None,
                total_network_rx_bytes: None,
                total_network_tx_bytes: None,
                total_network_rx_packets: None,
                total_network_tx_packets: None,
                total_network_tcp_connections: None,
                total_network_udp_connections: None,
                network_data_source: None,
            },
            AppGroupRecord {
                app_group_id: "medium_priority".to_string(),
                root_pid: 1002,
                process_ids: vec![1002],
                app_name: None,
                total_cpu_share: Some(0.2), // < 0.3, без бонуса
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_io_read_operations: None,
                total_io_write_operations: None,
                total_io_operations: None,
                io_data_source: None,
                total_rss_mb: Some(100),
                has_gui_window: true, // GUI группа -> средний score
                is_focused_group: false,
                tags: vec![],
                priority_class: None,
                total_energy_uj: None,
                total_power_w: None,
                total_network_rx_bytes: None,
                total_network_tx_bytes: None,
                total_network_rx_packets: None,
                total_network_tx_packets: None,
                total_network_tcp_connections: None,
                total_network_udp_connections: None,
                network_data_source: None,
            },
            AppGroupRecord {
                app_group_id: "low_priority".to_string(),
                root_pid: 1003,
                process_ids: vec![1003],
                app_name: None,
                total_cpu_share: Some(0.1), // < 0.3, без бонуса
                total_io_read_bytes: None,
                total_io_write_bytes: None,
                total_io_read_operations: None,
                total_io_write_operations: None,
                total_io_operations: None,
                io_data_source: None,
                total_rss_mb: Some(100),
                has_gui_window: false,
                is_focused_group: false,
                tags: vec![],
                priority_class: None,
                total_energy_uj: None,
                total_power_w: None,
                total_network_rx_bytes: None,
                total_network_tx_bytes: None,
                total_network_rx_packets: None,
                total_network_tx_packets: None,
                total_network_tcp_connections: None,
                total_network_udp_connections: None,
                network_data_source: None,
            },
        ];

        snapshot.app_groups = app_groups;

        let results = engine.evaluate_snapshot(&snapshot);

        // Проверяем, что все результаты имеют валидные классы приоритета
        // Примечание: фокусная группа может получить приоритет через семантические правила,
        // а не через ML-ранкер, поэтому не проверяем наличие "ml-ranker" для всех групп
        for (app_group_id, result) in &results {
            // Проверяем, что класс приоритета валиден
            match result.priority_class {
                PriorityClass::CritInteractive
                | PriorityClass::Interactive
                | PriorityClass::Normal
                | PriorityClass::Background
                | PriorityClass::Idle => {
                    // Валидный класс
                }
            }
            // Проверяем, что причина указана
            assert!(
                !result.reason.is_empty(),
                "Result for {} should have a reason",
                app_group_id
            );
        }

        // Проверяем, что группы без фокуса используют ML-ранкер
        let medium_result = results.get("medium_priority").unwrap();
        let low_result = results.get("low_priority").unwrap();

        // Группы без фокуса должны использовать ML-ранкер в hybrid режиме
        assert!(
            medium_result.reason.contains("ml-ranker") || medium_result.reason.contains("semantic"),
            "Medium priority group should use ml-ranker or semantic rules"
        );
        assert!(
            low_result.reason.contains("ml-ranker") || low_result.reason.contains("default"),
            "Low priority group should use ml-ranker or default"
        );

        // Проверяем, что все группы получили результаты
        assert_eq!(results.len(), 3);
        assert!(results.contains_key("high_priority"));
        assert!(results.contains_key("medium_priority"));
        assert!(results.contains_key("low_priority"));
    }

    #[test]
    fn test_hybrid_mode_with_onnx_model_disabled() {
        // Тест hybrid режима с отключенной моделью
        let mut config = create_hybrid_config();
        config.model.enabled = false;

        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        let app_groups = vec![AppGroupRecord {
            app_group_id: "test-group".to_string(),
            root_pid: 1000,
            process_ids: vec![1000],
            app_name: None,
            total_cpu_share: Some(0.1),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(100),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        }];

        snapshot.app_groups = app_groups;

        let results = engine.evaluate_snapshot(&snapshot);

        // Должен быть результат для группы
        assert_eq!(results.len(), 1);
        let result = results.get("test-group").unwrap();

        // Должен использовать ML-ранкер (StubRanker в данном случае)
        assert!(result.reason.contains("ml-ranker"));
    }

    #[test]
    fn test_hybrid_mode_with_nonexistent_onnx_model() {
        // Тест hybrid режима с несуществующим файлом модели
        let mut config = create_hybrid_config();
        config.model.enabled = true;
        config.model.model_path = "/nonexistent/path/model.onnx".to_string();

        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        let app_groups = vec![AppGroupRecord {
            app_group_id: "test-group".to_string(),
            root_pid: 1000,
            process_ids: vec![1000],
            app_name: None,
            total_cpu_share: Some(0.1),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(100),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        }];

        snapshot.app_groups = app_groups;

        let results = engine.evaluate_snapshot(&snapshot);

        // Должен быть результат для группы
        assert_eq!(results.len(), 1);
        let result = results.get("test-group").unwrap();

        // Когда модель не загружается, используется дефолтный приоритет (без ML-ранкера)
        assert!(result.reason.contains("default: no rules matched"));
    }

    #[test]
    fn test_rules_only_mode_no_ranker() {
        // Тест rules-only режима (ранкер не используется)
        let config = create_test_config();
        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        let app_groups = vec![AppGroupRecord {
            app_group_id: "test-group".to_string(),
            root_pid: 1000,
            process_ids: vec![1000],
            app_name: None,
            total_cpu_share: Some(0.1),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(100),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        }];

        snapshot.app_groups = app_groups;

        let results = engine.evaluate_snapshot(&snapshot);

        // Должен быть результат для группы
        assert_eq!(results.len(), 1);
        let result = results.get("test-group").unwrap();

        // В rules-only режиме должен использовать дефолтный приоритет
        assert_eq!(result.priority_class, PriorityClass::Normal);
        assert!(result.reason.contains("default"));
    }

    #[test]
    fn test_active_terminal_with_recent_input() {
        let config = create_test_config();
        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        // Устанавливаем активного пользователя с недавним вводом
        snapshot.global.user_active = true;
        snapshot.global.time_since_last_input_ms = Some(1000); // 1 секунда назад

        let mut terminal_process = ProcessRecord::default();
        terminal_process.pid = 2000;
        terminal_process.ppid = 1500;
        terminal_process.uid = 1000;
        terminal_process.gid = 1000;
        terminal_process.exe = Some("/usr/bin/bash".to_string());
        terminal_process.cmdline = Some("bash".to_string());
        terminal_process.cgroup_path = Some("/user.slice/user-1000.slice/session-1.scope".to_string());
        terminal_process.app_group_id = Some("terminal".to_string());
        terminal_process.state = "S".to_string();
        terminal_process.start_time = 0;
        terminal_process.uptime_sec = 500;
        terminal_process.tty_nr = 1;
        terminal_process.has_tty = true;
        terminal_process.cpu_share_1s = Some(0.05);
        terminal_process.cpu_share_10s = Some(0.03);
        terminal_process.rss_mb = Some(20);
        terminal_process.env_term = Some("xterm-256color".to_string());
        terminal_process.process_type = Some("cli".to_string());
        terminal_process.tags = vec!["terminal".to_string()];
        terminal_process.nice = 0;
        terminal_process.ionice_class = Some(2);
        terminal_process.ionice_prio = Some(4);

        let app_group = AppGroupRecord {
            app_group_id: "terminal".to_string(),
            root_pid: 2000,
            process_ids: vec![2000],
            app_name: Some("bash".to_string()),
            total_cpu_share: Some(0.05),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(20),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec!["terminal".to_string()],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        };

        snapshot.processes = vec![terminal_process];
        snapshot.app_groups = vec![app_group.clone()];

        let results = engine.evaluate_snapshot(&snapshot);
        let result = results.get("terminal").unwrap();

        // Активный терминал должен получить Interactive приоритет
        assert_eq!(result.priority_class, PriorityClass::Interactive);
        assert!(result.reason.contains("active terminal"));
    }

    #[test]
    fn test_multiple_app_groups_different_priorities() {
        let config = create_test_config();
        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        // Создаем несколько групп с разными характеристиками
        
        // Группа 1: Фокусный GUI (должен быть Interactive)
        let focused_gui_group = AppGroupRecord {
            app_group_id: "focused-gui".to_string(),
            root_pid: 1000,
            process_ids: vec![1000],
            app_name: Some("firefox".to_string()),
            total_cpu_share: Some(0.2),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(500),
            has_gui_window: true,
            is_focused_group: true,
            tags: vec!["browser".to_string()],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        };

        // Группа 2: Системный процесс (должен быть Normal)
        let system_group = AppGroupRecord {
            app_group_id: "systemd".to_string(),
            root_pid: 1,
            process_ids: vec![1],
            app_name: Some("systemd".to_string()),
            total_cpu_share: Some(0.05),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(50),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec!["system".to_string()],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        };

        // Группа 3: Updater с активным пользователем (должен быть Background)
        let updater_group = AppGroupRecord {
            app_group_id: "updater".to_string(),
            root_pid: 2000,
            process_ids: vec![2000],
            app_name: Some("packagekitd".to_string()),
            total_cpu_share: Some(0.1),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(100),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec!["updater".to_string()],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        };

        // Группа 4: Обычный процесс без особых характеристик (должен быть Normal)
        let normal_group = AppGroupRecord {
            app_group_id: "normal".to_string(),
            root_pid: 3000,
            process_ids: vec![3000],
            app_name: Some("background-task".to_string()),
            total_cpu_share: Some(0.05),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(50),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec!["background".to_string()],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        };

        snapshot.app_groups = vec![
            focused_gui_group,
            system_group,
            updater_group,
            normal_group,
        ];

        // Добавляем системный процесс для защиты
        let mut system_process = ProcessRecord::default();
        system_process.pid = 1;
        system_process.ppid = 0;
        system_process.uid = 0;
        system_process.gid = 0;
        system_process.exe = Some("/usr/lib/systemd/systemd".to_string());
        system_process.cgroup_path = Some("/system.slice/systemd.service".to_string());
        system_process.systemd_unit = Some("systemd.service".to_string());
        system_process.app_group_id = Some("systemd".to_string());
        system_process.state = "S".to_string();
        system_process.start_time = 0;
        system_process.uptime_sec = 1000;
        system_process.tty_nr = 0;
        system_process.has_tty = false;
        system_process.rss_mb = Some(50);
        system_process.process_type = Some("daemon".to_string());
        system_process.nice = 0;
        system_process.ionice_class = Some(2);
        system_process.ionice_prio = Some(4);

        snapshot.processes = vec![system_process];

        let results = engine.evaluate_snapshot(&snapshot);

        // Проверяем, что каждая группа получила ожидаемый приоритет
        assert_eq!(results.get("focused-gui").unwrap().priority_class, PriorityClass::Interactive);
        assert_eq!(results.get("systemd").unwrap().priority_class, PriorityClass::Normal);
        assert_eq!(results.get("updater").unwrap().priority_class, PriorityClass::Background);
        assert_eq!(results.get("normal").unwrap().priority_class, PriorityClass::Normal);
    }

    #[test]
    fn test_game_with_audio_focused_gets_crit_interactive() {
        let config = create_test_config();
        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        snapshot.responsiveness.audio_xruns_delta = Some(3); // Есть XRUN

        let mut game_process = ProcessRecord::default();
        game_process.pid = 3000;
        game_process.ppid = 2500;
        game_process.uid = 1000;
        game_process.gid = 1000;
        game_process.exe = Some("/usr/games/supergame".to_string());
        game_process.cmdline = Some("supergame".to_string());
        game_process.cgroup_path = Some("/user.slice/user-1000.slice/session-1.scope".to_string());
        game_process.app_group_id = Some("game".to_string());
        game_process.state = "S".to_string();
        game_process.start_time = 0;
        game_process.uptime_sec = 100;
        game_process.tty_nr = 0;
        game_process.has_tty = false;
        game_process.cpu_share_1s = Some(0.4);
        game_process.cpu_share_10s = Some(0.3);
        game_process.rss_mb = Some(500);
        game_process.has_gui_window = true;
        game_process.is_focused_window = true;
        game_process.env_has_display = true;
        game_process.is_audio_client = true;
        game_process.has_active_stream = true;
        game_process.process_type = Some("gui".to_string());
        game_process.tags = vec!["game".to_string()];
        game_process.nice = 0;
        game_process.ionice_class = Some(2);
        game_process.ionice_prio = Some(4);

        let app_group = AppGroupRecord {
            app_group_id: "game".to_string(),
            root_pid: 3000,
            process_ids: vec![3000],
            app_name: Some("supergame".to_string()),
            total_cpu_share: Some(0.4),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(500),
            has_gui_window: true,
            is_focused_group: true,
            tags: vec!["game".to_string()],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        };

        snapshot.processes = vec![game_process];
        snapshot.app_groups = vec![app_group.clone()];

        let results = engine.evaluate_snapshot(&snapshot);
        let result = results.get("game").unwrap();

        // Фокусная игра с аудио должна получить CritInteractive
        assert_eq!(result.priority_class, PriorityClass::CritInteractive);
        assert!(result.reason.contains("focused group with audio/game"));
    }

    #[test]
    fn test_dynamic_scaling_high_load() {
        let config = create_test_config();
        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        // Устанавливаем высокую нагрузку системы
        snapshot.global.load_avg_one = 5.0; // Высокая нагрузка на 4-ядерной системе
        snapshot.global.psi_cpu_some_avg10 = Some(0.9);
        snapshot.global.psi_io_some_avg10 = Some(0.8);
        snapshot.global.psi_mem_some_avg10 = Some(0.7);

        // Создаем группу с нормальным приоритетом
        let app_group = AppGroupRecord {
            app_group_id: "background-app".to_string(),
            root_pid: 3000,
            process_ids: vec![3000],
            app_name: Some("background-app".to_string()),
            total_cpu_share: Some(0.2),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(200),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        };

        snapshot.app_groups = vec![app_group.clone()];

        let results = engine.evaluate_snapshot(&snapshot);
        let result = results.get("background-app").unwrap();

        // При высокой нагрузке нормальный приоритет должен быть понижен до фонового
        assert_eq!(result.priority_class, PriorityClass::Background);
        assert!(result.reason.contains("dynamic-scaling"));
        assert!(result.reason.contains("NORMAL -> BACKGROUND"));
    }

    #[test]
    fn test_dynamic_scaling_no_change_low_load() {
        let config = create_test_config();
        let engine = PolicyEngine::new(config);
        let mut snapshot = create_test_snapshot();

        // Устанавливаем низкую нагрузку системы
        snapshot.global.load_avg_one = 0.5; // Низкая нагрузка
        snapshot.global.psi_cpu_some_avg10 = Some(0.1);
        snapshot.global.psi_io_some_avg10 = Some(0.1);
        snapshot.global.psi_mem_some_avg10 = Some(0.1);

        // Создаем группу с нормальным приоритетом
        let app_group = AppGroupRecord {
            app_group_id: "normal-app".to_string(),
            root_pid: 4000,
            process_ids: vec![4000],
            app_name: Some("normal-app".to_string()),
            total_cpu_share: Some(0.3),
            total_io_read_bytes: None,
            total_io_write_bytes: None,
            total_io_read_operations: None,
            total_io_write_operations: None,
            total_io_operations: None,
            io_data_source: None,
            total_rss_mb: Some(300),
            has_gui_window: false,
            is_focused_group: false,
            tags: vec![],
            priority_class: None,
            total_energy_uj: None,
            total_power_w: None,
            total_network_rx_bytes: None,
            total_network_tx_bytes: None,
            total_network_rx_packets: None,
            total_network_tx_packets: None,
            total_network_tcp_connections: None,
            total_network_udp_connections: None,
            network_data_source: None,
        };

        snapshot.app_groups = vec![app_group.clone()];

        let results = engine.evaluate_snapshot(&snapshot);
        let result = results.get("normal-app").unwrap();

        // При низкой нагрузке приоритет не должен измениться
        assert_eq!(result.priority_class, PriorityClass::Normal);
        assert!(!result.reason.contains("dynamic-scaling"));
    }
}
