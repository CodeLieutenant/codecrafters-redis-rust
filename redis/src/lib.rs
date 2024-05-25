use std::future::Future;
use std::pin::Pin;

use crate::server::tcp::{Error, Server as InnerRedisServer};

include!(concat!(env!("OUT_DIR"), "/commands.rs"));

pub mod error;
mod parser;
mod resp;
pub(crate) mod server;
pub mod value;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Ping,
    Echo(Box<str>),
}

pub trait Server {
    fn run(&self) -> Pin<Box<dyn Future<Output=Result<(), Error>> + '_>>;
}

struct RedisServer(InnerRedisServer);

impl Server for RedisServer {
    fn run(&self) -> Pin<Box<dyn Future<Output=Result<(), Error>> + '_>> {
        Box::pin(self.0.start())
    }
}

pub async fn start_server(
    port: u16,
    connection_limit: usize,
) -> Result<Box<dyn Server>, error::Error> {
    let server = Box::new(RedisServer(
        InnerRedisServer::new(port, connection_limit).await?,
    ));

    Ok(server)
}
