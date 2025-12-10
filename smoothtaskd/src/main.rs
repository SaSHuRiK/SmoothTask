use anyhow::Result;
use clap::Parser;
use smoothtask_core::{config::Config, run_daemon};
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

    run_daemon(config, args.dry_run).await
}

