use std::process::exit;

mod command;
mod parser;
mod server;
mod handler;

#[tokio::main]
async fn main() {
    let server = server::RedisServer::new(6379, 1024).await;

    match server {
        Ok(server) => {
            tokio::select! {
                result = tokio::signal::ctrl_c() => {
                    if let Err(err) = result {
                        eprintln!("{:?}", err);
                    }
                },
                _ = server.run() => {
                    println!("Receiving CTRL+C... Exiting...");
                }
            }
        }

        Err(err) => {
            eprintln!("{:?}", err);
            exit(1);
        }
    }
}
