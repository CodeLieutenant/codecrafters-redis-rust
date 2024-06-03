use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;

use crate::server::tcp::{Error, Server as InnerRedisServer};

mod redis_commands {
    include!(concat!(env!("OUT_DIR"), "/commands.rs"));
}

mod bytes;
pub mod error;
mod macros;
pub(crate) mod parser;
mod resp;
pub(crate) mod server;
pub mod value;

#[derive(Debug, Clone, PartialEq)]
pub enum Command<'a> {
    Ping,
    Command,
    Echo(Cow<'a, str>),
    Get(Cow<'a, str>),
    Set { key: Cow<'a, str> },
}

// Safety -> This technically is not true
// as Rc is not Send + Sync, but Command is handle at most in one thread,
// even if it crosses thread boundaries, there is no concurrent access on Command
unsafe impl<'a> Send for Command<'a> {}

unsafe impl<'a> Sync for Command<'a> {}

pub trait Server {
    fn run(&self) -> Pin<Box<dyn Future<Output = Result<(), Error>> + '_>>;
}

struct RedisServer(InnerRedisServer);

impl Server for RedisServer {
    fn run(&self) -> Pin<Box<dyn Future<Output = Result<(), Error>> + '_>> {
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
