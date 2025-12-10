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
    pub fn load(path: &str) -> anyhow::Result<Self> {
        let data = fs::read_to_string(path)?;
        let cfg: Config = serde_yaml::from_str(&data)?;
        Ok(cfg)
    }
}

