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
#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PolicyMode {
    /// Только правила, без ML-ранкера.
    RulesOnly,
    /// Правила + ML-ранкер для определения приоритетов.
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
}

#[derive(Debug, Deserialize, Clone)]
pub struct Paths {
    pub snapshot_db_path: String,
    pub patterns_dir: String,
}

impl Config {
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
            self.polling_interval_ms > 0,
            "polling_interval_ms must be positive"
        );
        ensure!(self.max_candidates > 0, "max_candidates must be positive");

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
                "{name} must be in the [0, 1] range (got {value})"
            );
        }

        ensure!(
            self.background_percentile <= self.normal_percentile
                && self.normal_percentile <= self.interactive_percentile
                && self.interactive_percentile <= self.crit_interactive_percentile,
            "priority percentiles must be non-decreasing from background to critical"
        );

        ensure!(
            (0.0..=1.0).contains(&self.psi_cpu_some_high)
                && (0.0..=1.0).contains(&self.psi_io_some_high),
            "PSI thresholds must be in the [0, 1] range"
        );
        ensure!(
            (0.0..=1.0).contains(&self.noisy_neighbour_cpu_share),
            "noisy_neighbour_cpu_share must be in the [0, 1] range"
        );
        ensure!(
            self.user_idle_timeout_sec > 0,
            "user_idle_timeout_sec must be positive"
        );
        ensure!(
            self.interactive_build_grace_sec > 0,
            "interactive_build_grace_sec must be positive"
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

        let snapshot_parent = Path::new(&self.snapshot_db_path)
            .parent()
            .unwrap_or_else(|| Path::new("."));
        ensure!(
            snapshot_parent.exists(),
            "snapshot_db_path parent directory must exist (got {:?})",
            snapshot_parent,
        );

        let patterns_dir = Path::new(&self.patterns_dir);
        ensure!(
            patterns_dir.is_dir(),
            "patterns_dir must point to an existing directory (got {:?})",
            patterns_dir,
        );

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
polling_interval_ms: 10
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
        let file = write_temp_config(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false
policy_mode: hybrid

paths:
  snapshot_db_path: "/var/lib/smoothtask/snapshots.sqlite"
  patterns_dir: "/etc/smoothtask/patterns"

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
        "#,
        );

        let cfg = Config::load(file.path().to_str().unwrap()).expect("config loads");
        assert_eq!(cfg.policy_mode, PolicyMode::Hybrid);
    }

    #[test]
    fn rejects_invalid_percentile_order() {
        let file = write_temp_config(
            r#"
polling_interval_ms: 10
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
            err
                .to_string()
                .contains("patterns_dir must point to an existing directory"),
            "unexpected error: {err:?}"
        );
    }
}
