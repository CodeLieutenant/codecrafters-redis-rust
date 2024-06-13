use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use server::Server as InnerRedisServer;
pub(crate) use bytes::Buffer;

pub use database::{Database, Value as DatabaseValue};
pub use resp::Value;

mod redis_commands {
    include!(concat!(env!("OUT_DIR"), "/commands.rs"));
}

pub(crate) use crate::redis_commands::{COMMAND_KEYWORDS, CommandKeywords};

mod bytes;
mod macros;
mod database;

pub(crate) mod parser;
mod resp;
pub(crate) mod server;

#[derive(Debug, Clone, PartialEq)]
pub enum Command<'a> {
    Ping,
    Command,
    Echo(Cow<'a, str>),
    Get(Cow<'a, str>),
    Set {
        key: Cow<'a, [u8]>,
        value: &'a Value<'a>,
        expiration: Option<tokio::time::Duration>,
    },
}

pub trait Server {
    fn run(&self) -> Pin<Box<dyn Future<Output = Result<(), std::io::Error>> + '_>>;
}

struct RedisServer(InnerRedisServer, Arc<Database>);

impl Server for RedisServer {
    fn run(&self) -> Pin<Box<dyn Future<Output = Result<(), std::io::Error>> + '_>> {
        Box::pin(self.0.start(Arc::clone(&self.1)))
    }
}

pub async fn start_server(
    port: u16,
    connection_limit: usize,
    db: Arc<Database>,
) -> Result<Box<dyn Server>, std::io::Error> {
    let server = Box::new(RedisServer(
        InnerRedisServer::new(port, connection_limit).await?,
        db
    ));

    Ok(server)
}
