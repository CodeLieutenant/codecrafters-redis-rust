use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use crate::server::tcp::{Error, Server as InnerRedisServer};

include!(concat!(env!("OUT_DIR"), "/commands.rs"));

pub mod error;
mod resp;
pub(crate) mod server;
pub mod value;
pub(crate) mod parser;
mod macros;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Ping,
    Echo(Rc<[u8]>),
}

// Safety -> This technically is not true
// as Rc is not Send + Sync, but Command is handle at most in one thread,
// even if it crosses thread boundaries, there is no concurrent access on Command
unsafe impl Send for Command {}

unsafe impl Sync for Command {}

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
