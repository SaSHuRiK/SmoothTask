pub mod actuator;
pub mod classify;
pub mod config;
pub mod logging;
pub mod metrics;
pub mod model;
pub mod policy;

use anyhow::Result;
use config::Config;

/// Главный цикл демона: опрос метрик, ранжирование, применение.
pub async fn run_daemon(_config: Config, dry_run: bool) -> Result<()> {
    // TODO:
    // 1. инициализация подсистем (cgroups, БД, model-инференс)
    // 2. основной loop:
    //    - metrics::collect_snapshot()
    //    - classify::apply_rules(...)
    //    - policy::evaluate_snapshot(...)
    //    - actuator::apply_changes(...)
    //    - logging::snapshots::maybe_log(...)
    loop {
        // временный заглушечный loop
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        tracing::debug!("SmoothTask tick (stub)");
        if dry_run {
            // в будущем сюда можно добавить отладочный вывод
        }
    }
}
