use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub enum RedisCommand {
    Ping,
    Echo(Arc<str>),
}
