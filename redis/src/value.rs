use std::rc::Rc;

use tokio::io::AsyncWrite;
use tracing::instrument;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    NullArray,
    SimpleString(Rc<[u8]>),
    Error(Rc<str>),
    Integer(i64),
    BulkString(Rc<[u8]>),
    Array(Box<[Value]>),
}

impl Value {
    #[instrument]
    pub fn value_type(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::NullArray => "null_array",
            Value::SimpleString(_) => "simple_string",
            Value::Error(_) => "error",
            Value::Integer(_) => "integer",
            Value::BulkString(_) => "bulk_string",
            Value::Array(_) => "array",
        }
    }
}

#[cfg(test)]
mod tests {}
