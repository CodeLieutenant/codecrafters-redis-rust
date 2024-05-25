use std::process::exit;

use redis::start_server;

#[tokio::main]
async fn main() {
    let server = start_server(6379, 1024).await;

    match server {
        Ok(server) => {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    println!("Receiving CTRL+C... Exiting...");
                },
                result = server.run() => {
                     if let Err(err) = result {
                        eprintln!("{:?}", err);
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
