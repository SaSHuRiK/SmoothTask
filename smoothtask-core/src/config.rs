use anyhow::{ensure, Result};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub polling_interval_ms: u64,
    pub max_candidates: usize,
    pub dry_run_default: bool,

    pub thresholds: Thresholds,
    pub paths: Paths,
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
        let data = fs::read_to_string(path)?;
        let cfg: Config = serde_yaml::from_str(&data)?;
        cfg.validate()?;
        Ok(cfg)
    }

    fn validate(&self) -> Result<()> {
        ensure!(self.polling_interval_ms > 0, "polling_interval_ms must be positive");
        ensure!(self.max_candidates > 0, "max_candidates must be positive");

        self.thresholds.validate()?;
        self.paths.validate()?;

        Ok(())
    }
}

impl Thresholds {
    fn validate(&self) -> Result<()> {
        let percentiles = [
            ("crit_interactive_percentile", self.crit_interactive_percentile),
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

    #[test]
    fn loads_valid_config() {
        let file = write_temp_config(
            r#"
polling_interval_ms: 500
max_candidates: 150
dry_run_default: false

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

        assert_eq!(cfg.polling_interval_ms, 500);
        assert_eq!(cfg.max_candidates, 150);
        assert!(!cfg.dry_run_default);
        assert_eq!(
            cfg.paths.snapshot_db_path,
            "/var/lib/smoothtask/snapshots.sqlite"
        );
        assert_eq!(cfg.paths.patterns_dir, "/etc/smoothtask/patterns");
        assert!((cfg.thresholds.crit_interactive_percentile - 0.9).abs() < f32::EPSILON);
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
            err
                .to_string()
                .contains("priority percentiles must be non-decreasing"),
            "unexpected error: {err:?}"
        );
    }
}

