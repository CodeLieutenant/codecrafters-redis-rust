#[derive(Debug, Clone, PartialEq)]
pub enum RedisCommand {
    Ping,
    Echo(Box<str>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum RedisValue {
    Null,
    NullArray,
    SimpleString(Box<[u8]>),
    Error(Box<str>),
    Integer(i64),
    BulkString(Box<[u8]>),
    Array(Box<[RedisValue]>),
}
