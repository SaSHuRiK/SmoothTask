use std::fs;
use std::path::Path;

use anyhow::{bail, ensure, Context, Result};

/// Средние значения PSI для окна 10 и 60 секунд.
#[derive(Debug, Clone, PartialEq)]
pub struct PsiAverages {
    pub avg10: f32,
    pub avg60: f32,
}

/// PSI-метрики для ресурса (some/full).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PsiMetrics {
    pub some: Option<PsiAverages>,
    pub full: Option<PsiAverages>,
}

/// Снимок PSI по всем доступным ресурсам.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PressureSnapshot {
    pub cpu_some: Option<PsiAverages>,
    pub io_some: Option<PsiAverages>,
    pub mem_some: Option<PsiAverages>,
    pub mem_full: Option<PsiAverages>,
}

fn parse_psi_line(line: &str) -> Result<(&str, PsiAverages)> {
    let mut parts = line.split_whitespace();
    let record_type = parts
        .next()
        .ok_or_else(|| anyhow::anyhow!("missing PSI record type"))?;

    let mut avg10 = None;
    let mut avg60 = None;

    for part in parts {
        if let Some(value) = part.strip_prefix("avg10=") {
            avg10 = Some(value.parse::<f32>().with_context(|| {
                format!("failed to parse avg10 value: {value}")
            })?);
        } else if let Some(value) = part.strip_prefix("avg60=") {
            avg60 = Some(value.parse::<f32>().with_context(|| {
                format!("failed to parse avg60 value: {value}")
            })?);
        }
    }

    let avg10 = avg10.ok_or_else(|| anyhow::anyhow!("avg10 is missing in PSI line"))?;
    let avg60 = avg60.ok_or_else(|| anyhow::anyhow!("avg60 is missing in PSI line"))?;

    Ok((record_type, PsiAverages { avg10, avg60 }))
}

fn parse_psi(contents: &str) -> Result<PsiMetrics> {
    let mut metrics = PsiMetrics::default();

    for line in contents.lines().map(str::trim).filter(|l| !l.is_empty()) {
        let (record_type, averages) = parse_psi_line(line)?;
        match record_type {
            "some" => metrics.some = Some(averages),
            "full" => metrics.full = Some(averages),
            unknown => bail!("unknown PSI record type: {unknown}"),
        }
    }

    ensure!(
        metrics.some.is_some() || metrics.full.is_some(),
        "PSI metrics are empty"
    );

    Ok(metrics)
}

fn read_psi_file(path: &Path) -> Result<PsiMetrics> {
    let data = fs::read_to_string(path)
        .with_context(|| format!("failed to read PSI data from {:?}", path))?;
    parse_psi(&data)
}

/// Читает PSI-метрики из указанной директории (по умолчанию `/proc/pressure`).
pub fn read_pressure_snapshot(base: impl AsRef<Path>) -> Result<PressureSnapshot> {
    let base = base.as_ref();

    let cpu = read_psi_file(&base.join("cpu"))?;
    let io = read_psi_file(&base.join("io"))?;
    let memory = read_psi_file(&base.join("memory"))?;

    Ok(PressureSnapshot {
        cpu_some: cpu.some,
        io_some: io.some,
        mem_some: memory.some,
        mem_full: memory.full,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_some_only_line() {
        let data = "some avg10=0.10 avg60=0.20 avg300=0.30 total=10";
        let metrics = parse_psi(data).expect("parsed");

        assert_eq!(
            metrics,
            PsiMetrics {
                some: Some(PsiAverages {
                    avg10: 0.10,
                    avg60: 0.20,
                }),
                full: None,
            }
        );
    }

    #[test]
    fn parses_some_and_full() {
        let data = "some avg10=0.05 avg60=0.15 avg300=0.25 total=5\nfull avg10=0.50 avg60=0.75 avg300=0.95 total=9";
        let metrics = parse_psi(data).expect("parsed");

        assert_eq!(
            metrics,
            PsiMetrics {
                some: Some(PsiAverages {
                    avg10: 0.05,
                    avg60: 0.15,
                }),
                full: Some(PsiAverages {
                    avg10: 0.50,
                    avg60: 0.75,
                }),
            }
        );
    }

    #[test]
    fn errors_on_unknown_record_type() {
        let data = "maybe avg10=0.1 avg60=0.2";
        let err = parse_psi(data).unwrap_err();
        assert!(err.to_string().contains("unknown PSI record type"));
    }

    #[test]
    fn errors_on_missing_values() {
        let data = "some avg60=0.2";
        let err = parse_psi(data).unwrap_err();
        assert!(err.to_string().contains("avg10 is missing"));
    }

    #[test]
    fn reads_snapshot_from_temp_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let base = dir.path();

        fs::write(
            base.join("cpu"),
            "some avg10=0.10 avg60=0.20 avg300=0.30 total=1",
        )
        .expect("write cpu");
        fs::write(
            base.join("io"),
            "some avg10=0.01 avg60=0.02 avg300=0.03 total=2",
        )
        .expect("write io");
        fs::write(
            base.join("memory"),
            "some avg10=0.11 avg60=0.22 avg300=0.33 total=3\nfull avg10=0.44 avg60=0.55 avg300=0.66 total=4",
        )
        .expect("write memory");

        let snapshot = read_pressure_snapshot(base).expect("snapshot");

        assert_eq!(
            snapshot,
            PressureSnapshot {
                cpu_some: Some(PsiAverages {
                    avg10: 0.10,
                    avg60: 0.20,
                }),
                io_some: Some(PsiAverages {
                    avg10: 0.01,
                    avg60: 0.02,
                }),
                mem_some: Some(PsiAverages {
                    avg10: 0.11,
                    avg60: 0.22,
                }),
                mem_full: Some(PsiAverages {
                    avg10: 0.44,
                    avg60: 0.55,
                }),
            }
        );
    }
}

