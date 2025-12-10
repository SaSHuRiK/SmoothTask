use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub polling_interval_ms: u64,
    pub max_candidates: usize,
    pub dry_run_default: bool,
    #[serde(default = "default_policy_mode")]
    pub policy_mode: PolicyMode,

    pub thresholds: Thresholds,
    pub paths: Paths,
}

/// Режим работы Policy Engine.
///
/// Определяет, как Policy Engine вычисляет приоритеты для AppGroup:
///
/// - `rules-only`: Используются только жёсткие правила (guardrails) и семантические правила.
///   ML-ранкер не используется. Это режим по умолчанию и рекомендуется для начального использования.
///
/// - `hybrid`: Комбинация правил и ML-ранкера. Сначала применяются guardrails и семантические правила,
///   затем ML-ранкер используется для ранжирования групп внутри допустимых классов приоритетов.
///   Требует обученной модели CatBoostRanker.
///
/// # Примеры использования в конфигурации
///
/// ```yaml
/// # Режим только правил (по умолчанию)
/// policy_mode: rules-only
///
/// # Гибридный режим с ML-ранкером
/// policy_mode: hybrid
/// ```
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyMode {
    /// Только правила, без ML-ранкера.
    /// Используются guardrails и семантические правила для определения приоритетов.
    RulesOnly,
    /// Правила + ML-ранкер для определения приоритетов.
    /// Guardrails и семантические правила имеют приоритет, затем используется ML-ранкер
    /// для ранжирования групп внутри допустимых классов.
    Hybrid,
}

fn default_policy_mode() -> PolicyMode {
    PolicyMode::RulesOnly
}

#[derive(Debug, Deserialize, Clone)]
pub struct Thresholds {
    pub psi_cpu_some_high: f32,
    pub psi_io_some_high: f32,
    pub user_idle_timeout_sec: u64,
    pub interactive_build_grace_sec: u64,
    pub noisy_neighbour_cpu_share: f32,

    pub crit_interactive_percentile: f32,
    pub interactive_percentile: f32,
    pub normal_percentile: f32,
    pub background_percentile: f32,

    /// Порог для sched_latency_p99_ms (в миллисекундах) для определения bad_responsiveness.
    #[serde(default = "default_sched_latency_p99_threshold")]
    pub sched_latency_p99_threshold_ms: f64,

    /// Порог для ui_loop_p95_ms (в миллисекундах) для определения bad_responsiveness.
    /// По умолчанию 16.67 мс (60 FPS).
    #[serde(default = "default_ui_loop_p95_threshold")]
    pub ui_loop_p95_threshold_ms: f64,
}

fn default_sched_latency_p99_threshold() -> f64 {
    10.0 // 10 мс по умолчанию
}

fn default_ui_loop_p95_threshold() -> f64 {
    16.67 // 16.67 мс по умолчанию (60 FPS)
}

#[derive(Debug, Deserialize, Clone)]
pub struct Paths {
    pub snapshot_db_path: String,
    pub patterns_dir: String,
}

impl Config {
    /// Загружает конфигурацию из YAML файла.
    ///
    /// Функция читает файл по указанному пути, парсит YAML и валидирует конфигурацию.
    /// При ошибках чтения, парсинга или валидации возвращается `Result::Err` с описанием проблемы.
    ///
    /// # Примеры использования
    ///
    /// ## Базовое использование
    ///
    /// ```no_run
    /// use smoothtask_core::config::Config;
    ///
    /// let config = Config::load("configs/smoothtask.yml")?;
    /// println!("Polling interval: {} ms", config.polling_interval_ms);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// ## Обработка ошибок
    ///
    /// ```no_run
    /// use smoothtask_core::config::Config;
    ///
    /// match Config::load("configs/smoothtask.yml") {
    ///     Ok(config) => println!("Config loaded successfully"),
    ///     Err(e) => eprintln!("Failed to load config: {}", e),
    /// }
    /// ```
    ///
    /// ## Использование с переменной окружения
    ///
    /// ```no_run
    /// use smoothtask_core::config::Config;
    /// use std::env;
    ///
    /// let config_path = env::var("SMOOTHTASK_CONFIG")
    ///     .unwrap_or_else(|_| "configs/smoothtask.example.yml".to_string());
    /// let config = Config::load(&config_path)?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    ///
    /// # Ошибки
    ///
    /// Функция может вернуть ошибку в следующих случаях:
    ///
    /// - Файл не существует или недоступен для чтения
    /// - Файл содержит некорректный YAML
    /// - Конфигурация не проходит валидацию (некорректные значения полей)
    ///
    /// # Примеры ошибок
    ///
    /// ```no_run
    /// use smoothtask_core::config::Config;
    ///
    /// // Файл не существует
    /// let err = Config::load("/nonexistent/path.yml").unwrap_err();
    /// assert!(err.to_string().contains("failed to read config"));
    ///
    /// // Некорректный YAML
    /// // let err = Config::load("invalid.yaml").unwrap_err();
    /// // assert!(err.to_string().contains("failed to parse YAML"));
    ///
    /// // Валидация не прошла
    /// // let err = Config::load("invalid_config.yml").unwrap_err();
    /// // assert!(err.to_string().contains("must be"));
    /// ```
    pub fn load(path: &str) -> Result<Self> {
        let data = fs::read_to_string(path)
            .with_context(|| format!("failed to read config from {path}"))?;
        let cfg: Config = serde_yaml::from_str(&data)
            .with_context(|| format!("failed to parse YAML config at {path}"))?;
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        ensure!(
            self.polling_interval_ms >= 100,
            "polling_interval_ms must be >= 100 ms to prevent excessive system polling (got {})",
            self.polling_interval_ms
        );
        ensure!(
            self.polling_interval_ms <= 60000,
            "polling_interval_ms must be <= 60000 ms (1 minute) to ensure responsive system monitoring (got {})",
            self.polling_interval_ms
        );
        ensure!(
            self.max_candidates > 0,
            "max_candidates must be positive (got {})",
            self.max_candidates
        );
        ensure!(
            self.max_candidates <= 10000,
            "max_candidates must be <= 10000 to prevent excessive memory usage (got {})",
            self.max_candidates
        );

        self.thresholds.validate()?;
        self.paths.validate()?;

        Ok(())
    }
}

impl Thresholds {
    fn validate(&self) -> Result<()> {
        let percentiles = [
            (
                "crit_interactive_percentile",
                self.crit_interactive_percentile,
            ),
            ("interactive_percentile", self.interactive_percentile),
            ("normal_percentile", self.normal_percentile),
            ("background_percentile", self.background_percentile),
        ];

        for (name, value) in percentiles {
            ensure!(
                (0.0..=1.0).contains(&value),
                "thresholds.{name} must be in the [0, 1] range (got {value})"
            );
        }

        ensure!(
            self.background_percentile <= self.normal_percentile
                && self.normal_percentile <= self.interactive_percentile
                && self.interactive_percentile <= self.crit_interactive_percentile,
            "priority percentiles must be non-decreasing from background to critical"
        );

        ensure!(
            self.psi_cpu_some_high >= 0.0,
            "thresholds.psi_cpu_some_high must be >= 0.0 (got {})",
            self.psi_cpu_some_high
        );
        ensure!(
            self.psi_cpu_some_high <= 1.0,
            "thresholds.psi_cpu_some_high must be <= 1.0 (got {})",
            self.psi_cpu_some_high
        );
        ensure!(
            self.psi_io_some_high >= 0.0,
            "thresholds.psi_io_some_high must be >= 0.0 (got {})",
            self.psi_io_some_high
        );
        ensure!(
            self.psi_io_some_high <= 1.0,
            "thresholds.psi_io_some_high must be <= 1.0 (got {})",
            self.psi_io_some_high
        );
        ensure!(
            self.noisy_neighbour_cpu_share > 0.0,
            "thresholds.noisy_neighbour_cpu_share must be positive (got {})",
            self.noisy_neighbour_cpu_share
        );
        ensure!(
            self.noisy_neighbour_cpu_share <= 1.0,
            "thresholds.noisy_neighbour_cpu_share must be <= 1.0 (got {})",
            self.noisy_neighbour_cpu_share
        );
        ensure!(
            self.user_idle_timeout_sec > 0,
            "thresholds.user_idle_timeout_sec must be positive (got {})",
            self.user_idle_timeout_sec
        );
        ensure!(
            self.user_idle_timeout_sec <= 86400,
            "thresholds.user_idle_timeout_sec must be <= 86400 sec (24 hours) to ensure reasonable user activity tracking. Current value: {} sec. Please use a value between 1 and 86400 seconds.",
            self.user_idle_timeout_sec
        );
        ensure!(
            self.interactive_build_grace_sec > 0,
            "thresholds.interactive_build_grace_sec must be positive (got {})",
            self.interactive_build_grace_sec
        );
        ensure!(
            self.interactive_build_grace_sec <= 3600,
            "thresholds.interactive_build_grace_sec must be <= 3600 sec (1 hour) to ensure reasonable grace period for interactive builds. Current value: {} sec. Please use a value between 1 and 3600 seconds.",
            self.interactive_build_grace_sec
        );
        ensure!(
            self.sched_latency_p99_threshold_ms > 0.0,
            "thresholds.sched_latency_p99_threshold_ms must be positive (got {})",
            self.sched_latency_p99_threshold_ms
        );
        ensure!(
            self.sched_latency_p99_threshold_ms <= 1000.0,
            "thresholds.sched_latency_p99_threshold_ms must be <= 1000.0 ms (1 second) to ensure reasonable latency monitoring (got {})",
            self.sched_latency_p99_threshold_ms
        );
        ensure!(
            self.ui_loop_p95_threshold_ms > 0.0,
            "thresholds.ui_loop_p95_threshold_ms must be positive (got {})",
            self.ui_loop_p95_threshold_ms
        );
        ensure!(
            self.ui_loop_p95_threshold_ms <= 1000.0,
            "thresholds.ui_loop_p95_threshold_ms must be <= 1000.0 ms (1 second) to ensure reasonable UI latency monitoring (got {})",
            self.ui_loop_p95_threshold_ms
        );

        // Логическая валидация: P99 должен быть >= P95, так как P99 - это более высокий перцентиль
        ensure!(
            self.sched_latency_p99_threshold_ms >= self.ui_loop_p95_threshold_ms,
            "thresholds.sched_latency_p99_threshold_ms ({}) must be >= thresholds.ui_loop_p95_threshold_ms ({}) because P99 is a higher percentile than P95",
            self.sched_latency_p99_threshold_ms,
            self.ui_loop_p95_threshold_ms
        );

        Ok(())
    }
}

impl Paths {
    fn validate(&self) -> Result<()> {
        ensure!(
            !self.snapshot_db_path.trim().is_empty(),
            "snapshot_db_path must not be empty"
        );
        ensure!(
            !self.patterns_dir.trim().is_empty(),
            "patterns_dir must not be empty"
        );

        // Проверяем, что snapshot_db_path имеет расширение .sqlite или .db
        let snapshot_path = Path::new(&self.snapshot_db_path);
        if let Some(ext) = snapshot_path.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            ensure!(
                ext_str == "sqlite" || ext_str == "db",
                "snapshot_db_path must have .sqlite or .db extension (got {:?})",
                ext
            );
        } else {
            anyhow::bail!(
                "snapshot_db_path must have .sqlite or .db extension (got path without extension: {:?})",
                snapshot_path
            );
        }

        let snapshot_parent = Path::new(&self.snapshot_db_path)
            .parent()
            .unwrap_or_else(|| Path::new("."));
        ensure!(
            snapshot_parent.exists(),
            "snapshot_db_path parent directory must exist (got {:?})",
            snapshot_parent,
        );

        // Проверяем, что родительская директория доступна для записи
        // (пробуем создать временный файл для проверки прав доступа)
        #[cfg(unix)]
        {
            use std::fs::File;
            use std::io::Write;
            let test_file = snapshot_parent.join(".smoothtask_write_test");
            if let Ok(mut file) = File::create(&test_file) {
                // Если файл создан, пробуем записать в него
                if file.write_all(b"test").is_ok() {
                    // Удаляем тестовый файл
                    let _ = std::fs::remove_file(&test_file);
                } else {
                    anyhow::bail!(
                        "snapshot_db_path parent directory is not writable (got {:?})",
                        snapshot_parent
                    );
                }
            } else {
                anyhow::bail!(
                    "snapshot_db_path parent directory is not writable (got {:?})",
                    snapshot_parent
                );
            }
        }

        let patterns_dir = Path::new(&self.patterns_dir);
        ensure!(
            patterns_dir.is_dir(),
            "patterns_dir must point to an existing directory (got {:?})",
            patterns_dir,
        );

        // Проверяем, что директория patterns_dir доступна для чтения
        #[cfg(unix)]
        {
            use std::fs::read_dir;
            if read_dir(patterns_dir).is_err() {
                anyhow::bail!("patterns_dir is not readable (got {:?})", patterns_dir);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_config(contents: &str) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().expect("tempfile");
        file.write_all(contents.as_bytes())
            .expect("write temp config");
        file
    }

    fn build_config(snapshot_db_path: &str, patterns_dir: &str) -> String {
        format!(
            r#"
polling_interval_ms: 100
max_candidates: 5
dry_run_default: true

paths:
  snapshot_db_path: "{snapshot_db_path}"
  patterns_dir: "{patterns_dir}"

thresholds:
  psi_cpu_some_high: 0.2
  psi_io_some_high: 0.2
  user_idle_timeout_sec: 1
  interactive_build_grace_sec: 1
  noisy_neighbour_cpu_share: 0.5

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.7
  normal_percentile: 0.5
  background_percentile: 0.3
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#
        )
    }

    #[test]
    fn loads_valid_config() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("data").join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");

        assert_eq!(cfg.polling_interval_ms, 500);
        assert_eq!(cfg.max_candidates, 150);
        assert!(!cfg.dry_run_default);
        assert_eq!(cfg.policy_mode, PolicyMode::RulesOnly);
        assert_eq!(
            cfg.paths.snapshot_db_path,
            snapshot_db_path.display().to_string()
        );
        assert_eq!(cfg.paths.patterns_dir, patterns_dir.display().to_string());
        assert!((cfg.thresholds.crit_interactive_percentile - 0.9).abs() < f32::EPSILON);
    }

    #[test]
    fn loads_config_with_hybrid_mode() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false
policy_mode: hybrid

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.to_str().unwrap(),
            patterns_dir.to_str().unwrap()
        ));

        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");
        assert_eq!(cfg.policy_mode, PolicyMode::Hybrid);
    }

    #[test]
    fn rejects_invalid_percentile_order() {
        let file = write_temp_config(
            r#"
polling_interval_ms: 100
max_candidates: 1
dry_run_default: true

paths:
  snapshot_db_path: "/tmp/db"
  patterns_dir: "/tmp/patterns"

thresholds:
  psi_cpu_some_high: 0.2
  psi_io_some_high: 0.2
  user_idle_timeout_sec: 1
  interactive_build_grace_sec: 1
  noisy_neighbour_cpu_share: 0.5

  crit_interactive_percentile: 0.4
  interactive_percentile: 0.7
  normal_percentile: 0.2
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
        );

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("priority percentiles must be non-decreasing"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn errors_include_path_when_file_is_missing() {
        let missing_path = "/non/existent/smoothtask.yml";
        let err = Config::load(missing_path).unwrap_err();
        let message = err.to_string();

        assert!(message.contains(missing_path), "message was: {message}");
        assert!(
            message.contains("failed to read config"),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn rejects_missing_snapshot_parent_directory() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let missing_parent = temp_dir.path().join("no_such").join("snapshots.sqlite");

        let file = write_temp_config(&build_config(
            missing_parent.to_str().unwrap(),
            temp_dir.path().to_str().unwrap(),
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("snapshot_db_path parent directory must exist"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_missing_patterns_directory() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let missing_patterns = temp_dir.path().join("patterns");

        let file = write_temp_config(&build_config(
            temp_dir.path().join("snapshots.sqlite").to_str().unwrap(),
            missing_patterns.to_str().unwrap(),
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("patterns_dir must point to an existing directory"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_polling_interval_too_small() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 50
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("polling_interval_ms must be >= 100"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_polling_interval_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 70000
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("polling_interval_ms must be <= 60000"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_max_candidates_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 20000
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string().contains("max_candidates must be <= 10000"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_snapshot_db_path_without_extension() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots"); // без расширения
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("snapshot_db_path must have .sqlite or .db extension"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_snapshot_db_path_with_wrong_extension() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.txt"); // неправильное расширение
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("snapshot_db_path must have .sqlite or .db extension"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn accepts_snapshot_db_path_with_db_extension() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.db"); // .db расширение
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        // Должен загрузиться без ошибок
        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");
        assert_eq!(
            cfg.paths.snapshot_db_path,
            snapshot_db_path.display().to_string()
        );
    }

    #[test]
    fn rejects_sched_latency_p99_threshold_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 2000.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("sched_latency_p99_threshold_ms must be <= 1000.0"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_ui_loop_p95_threshold_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 1000.0
  ui_loop_p95_threshold_ms: 2000.0
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("ui_loop_p95_threshold_ms must be <= 1000.0"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn accepts_valid_latency_thresholds() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        // Должен загрузиться без ошибок
        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");
        assert!((cfg.thresholds.sched_latency_p99_threshold_ms - 20.0).abs() < f64::EPSILON);
        assert!((cfg.thresholds.ui_loop_p95_threshold_ms - 16.67).abs() < f64::EPSILON);
    }

    #[test]
    #[cfg(unix)]
    fn validates_snapshot_db_path_parent_is_writable() {
        // Тест проверяет, что валидация проверяет права на запись в родительскую директорию
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        // Должен загрузиться без ошибок (временная директория доступна для записи)
        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");
        assert_eq!(
            cfg.paths.snapshot_db_path,
            snapshot_db_path.display().to_string()
        );
    }

    #[test]
    #[cfg(unix)]
    fn validates_patterns_dir_is_readable() {
        // Тест проверяет, что валидация проверяет права на чтение директории patterns_dir
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        // Должен загрузиться без ошибок (временная директория доступна для чтения)
        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");
        assert_eq!(cfg.paths.patterns_dir, patterns_dir.display().to_string());
    }

    // Edge cases для polling_interval_ms
    #[test]
    fn rejects_polling_interval_zero() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 0
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("polling_interval_ms must be >= 100"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn accepts_polling_interval_minimum() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 100
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");
        assert_eq!(cfg.polling_interval_ms, 100);
    }

    #[test]
    fn accepts_polling_interval_maximum() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 60000
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");
        assert_eq!(cfg.polling_interval_ms, 60000);
    }

    // Edge cases для max_candidates
    #[test]
    fn rejects_max_candidates_zero() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 0
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string().contains("max_candidates must be positive"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn accepts_max_candidates_minimum() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 1
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");
        assert_eq!(cfg.max_candidates, 1);
    }

    #[test]
    fn accepts_max_candidates_maximum() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 10000
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");
        assert_eq!(cfg.max_candidates, 10000);
    }

    // Edge cases для percentiles
    #[test]
    fn rejects_percentile_negative() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: -0.1
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("crit_interactive_percentile must be in the [0, 1] range"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_percentile_greater_than_one() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 1.1
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("crit_interactive_percentile must be in the [0, 1] range"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn accepts_percentile_boundary_zero() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 1.0
  interactive_percentile: 1.0
  normal_percentile: 1.0
  background_percentile: 1.0
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");
        assert!((cfg.thresholds.crit_interactive_percentile - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn rejects_psi_cpu_some_high_negative() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: -0.1
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string().contains("psi_cpu_some_high must be >= 0.0"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_psi_io_some_high_negative() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: -0.1
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string().contains("psi_io_some_high must be >= 0.0"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_noisy_neighbour_cpu_share_greater_than_one() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 1.1

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("noisy_neighbour_cpu_share must be <= 1.0"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_noisy_neighbour_cpu_share_zero_or_negative() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        // Тест для нуля
        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.0

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("noisy_neighbour_cpu_share must be positive"),
            "unexpected error: {err:?}"
        );

        // Тест для отрицательного значения
        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: -0.1

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("noisy_neighbour_cpu_share must be positive"),
            "unexpected error: {err:?}"
        );
    }

    // Edge cases для latency thresholds
    #[test]
    fn rejects_sched_latency_p99_threshold_zero() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 0.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("sched_latency_p99_threshold_ms must be positive"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn accepts_sched_latency_p99_threshold_maximum() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 1000.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");
        assert!((cfg.thresholds.sched_latency_p99_threshold_ms - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn rejects_ui_loop_p95_threshold_zero() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 10.0
  ui_loop_p95_threshold_ms: 0.0
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("ui_loop_p95_threshold_ms must be positive"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn accepts_ui_loop_p95_threshold_maximum() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 1000.0
  ui_loop_p95_threshold_ms: 1000.0
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");
        assert!((cfg.thresholds.ui_loop_p95_threshold_ms - 1000.0).abs() < f64::EPSILON);
    }

    // Edge cases для timeouts
    #[test]
    fn rejects_user_idle_timeout_zero() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 0
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("user_idle_timeout_sec must be positive"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_interactive_build_grace_sec_zero() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 0
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("interactive_build_grace_sec must be positive"),
            "unexpected error: {err:?}"
        );
    }

    // Edge cases для путей с пробелами
    #[test]
    fn rejects_snapshot_db_path_with_only_spaces() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "   "
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("snapshot_db_path must not be empty"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_patterns_dir_with_only_spaces() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "   "

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string().contains("patterns_dir must not be empty"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_user_idle_timeout_sec_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 100000
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("user_idle_timeout_sec must be <= 86400"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn rejects_interactive_build_grace_sec_too_large() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10000
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            err.to_string()
                .contains("interactive_build_grace_sec must be <= 3600"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn accepts_valid_timeout_values() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 3600
  interactive_build_grace_sec: 1800
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        // Должен успешно загрузиться с валидными значениями
        let cfg = Config::load(file.path().to_str().unwrap()).expect("config should load");
        assert_eq!(cfg.thresholds.user_idle_timeout_sec, 3600);
        assert_eq!(cfg.thresholds.interactive_build_grace_sec, 1800);
    }

    // Валидация логического соотношения между latency thresholds
    #[test]
    fn rejects_sched_latency_p99_threshold_less_than_ui_loop_p95_threshold() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 10.0
  ui_loop_p95_threshold_ms: 20.0
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let err = Config::load(file.path().to_str().unwrap()).unwrap_err();
        let err_msg = err.to_string();
        assert!(
            err_msg.contains("sched_latency_p99_threshold_ms")
                && err_msg.contains("must be >=")
                && err_msg.contains("ui_loop_p95_threshold_ms"),
            "unexpected error: {err:?}"
        );
    }

    #[test]
    fn accepts_sched_latency_p99_threshold_equal_to_ui_loop_p95_threshold() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 16.67
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        // Должен успешно загрузиться, когда пороги равны
        let cfg = Config::load(file.path().to_str().unwrap()).expect("config should load");
        assert!((cfg.thresholds.sched_latency_p99_threshold_ms - 16.67).abs() < f64::EPSILON);
        assert!((cfg.thresholds.ui_loop_p95_threshold_ms - 16.67).abs() < f64::EPSILON);
    }

    #[test]
    fn accepts_sched_latency_p99_threshold_greater_than_ui_loop_p95_threshold() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        // Должен успешно загрузиться, когда P99 > P95
        let cfg = Config::load(file.path().to_str().unwrap()).expect("config should load");
        assert!((cfg.thresholds.sched_latency_p99_threshold_ms - 20.0).abs() < f64::EPSILON);
        assert!((cfg.thresholds.ui_loop_p95_threshold_ms - 16.67).abs() < f64::EPSILON);
    }

    #[test]
    fn policy_mode_defaults_to_rules_only() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");
        // policy_mode не указан, должен быть дефолтным (RulesOnly)
        assert_eq!(cfg.policy_mode, PolicyMode::RulesOnly);
    }

    #[test]
    fn policy_mode_accepts_rules_only() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let snapshot_db_path = temp_dir.path().join("snapshots.sqlite");
        std::fs::create_dir_all(snapshot_db_path.parent().unwrap()).expect("snapshot dir");
        let patterns_dir = temp_dir.path().join("patterns");
        std::fs::create_dir_all(&patterns_dir).expect("patterns dir");

        let file = write_temp_config(&format!(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false
policy_mode: rules-only

paths:
  snapshot_db_path: "{}"
  patterns_dir: "{}"

thresholds:
  psi_cpu_some_high: 0.6
  psi_io_some_high: 0.4
  user_idle_timeout_sec: 120
  interactive_build_grace_sec: 10
  noisy_neighbour_cpu_share: 0.7

  crit_interactive_percentile: 0.9
  interactive_percentile: 0.6
  normal_percentile: 0.3
  background_percentile: 0.1
  sched_latency_p99_threshold_ms: 20.0
  ui_loop_p95_threshold_ms: 16.67
        "#,
            snapshot_db_path.display(),
            patterns_dir.display()
        ));

        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");
        assert_eq!(cfg.policy_mode, PolicyMode::RulesOnly);
    }
}
