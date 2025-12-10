use anyhow::Result;
use clap::Parser;
use smoothtask_core::{config::Config, run_daemon};
use tokio::signal;
use tokio::sync::watch;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "smoothtaskd", about = "SmoothTask daemon")]
struct Args {
    /// Путь к конфигу
    #[arg(short, long, default_value = "/etc/smoothtask/smoothtask.yml")]
    config: String,

    /// Dry-run: считать приоритеты, но не применять
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let config = Config::load(&args.config)?;

    tracing::info!("Starting SmoothTask daemon (dry_run = {})", args.dry_run);

    // Создаём канал для graceful shutdown
    let (shutdown_tx, shutdown_rx) = watch::channel(());

    // Создаём задачу для обработки сигналов завершения
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        let _ = signal::ctrl_c().await;
        tracing::info!("Received SIGINT/SIGTERM, initiating graceful shutdown");
        let _ = shutdown_tx_clone.send(());
    });

    // Запускаем демон с каналом shutdown
    run_daemon(config, args.dry_run, shutdown_rx).await
}
