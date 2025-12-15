mod systemd;

use anyhow::{Context, Result};
use clap::Parser;
use smoothtask_core::config::auto_reload::ConfigAutoReload;
use smoothtask_core::config::config_struct::Config;
use smoothtask_core::run_daemon;
use tokio::{signal, sync::watch};
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

    // Создаём систему автоматической перезагрузки конфигурации
    let config_auto_reload = ConfigAutoReload::new(&args.config, config.clone())
        .context("Failed to initialize config auto-reload system")?;
    
    // Запускаем мониторинг изменений конфигурационного файла
    let _watch_handle = config_auto_reload.start_watching();
    
    // Запускаем задачу автоматической перезагрузки конфигурации
    let _auto_reload_handle = config_auto_reload.start_auto_reload();

    // Создаём канал для graceful shutdown (false = работаем, true = shutdown)
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    // Создаём задачу для обработки сигналов завершения
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = signal::ctrl_c().await {
            tracing::error!("Error waiting for Ctrl-C signal: {}", e);
            // Уведомляем systemd о критической ошибке
            systemd::notify_error(1, Some(&format!("Signal handling error: {}", e)));
        }
        tracing::info!("Received SIGINT/SIGTERM, initiating graceful shutdown");
        if shutdown_tx_clone.send(true).is_err() {
            tracing::error!("Failed to send shutdown signal");
        }
    });

    // Запускаем демон с каналом shutdown и callback для systemd notify
    let on_ready = Box::new(|| {
        if let Err(e) = systemd::notify_ready() {
            tracing::debug!(
                "Failed to notify systemd (not running under systemd?): {}",
                e
            );
        } else {
            tracing::info!("Notified systemd: READY=1");
        }
    });
    let on_status_update = Box::new(|status: &str| {
        tracing::debug!("Updating systemd status: {}", status);
        systemd::notify_status(status);
    });
    run_daemon(
        config,
        Some(config_auto_reload),
        Some(args.config.clone()),
        args.dry_run,
        shutdown_rx,
        Some(on_ready),
        Some(on_status_update),
    )
    .await?;

    Ok(())
}
