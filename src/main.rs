use std::process::exit;
use tracing::{error, info};

use tracing_subscriber::{filter::EnvFilter, fmt::layer as fmt_layer, prelude::*, registry};

use redis::start_server;

#[tokio::main]
async fn main() {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());

    let stdout_layer = fmt_layer()
        .with_ansi(true)
        .with_level(true)
        .with_thread_names(false)
        .with_target(false)
        .with_writer(non_blocking);

    registry().with(env_filter).with(stdout_layer).init();

    let server = start_server(6379, 1024).await;

    match server {
        Ok(server) => {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    info!("Receiving CTRL+C... Exiting...");
                },
                result = server.run() => {
                     if let Err(err) = result {
                        error!("{:?}", err);
                    }
                }
            }
        }

        Err(err) => {
            eprintln!("{:?}", err);
            exit(1);
        }
    }
}
