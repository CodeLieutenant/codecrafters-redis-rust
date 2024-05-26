use std::rc::Rc;

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

pub(crate) mod macros {
    #[macro_export] macro_rules! null {
        () => {
            Value::Null
        }
    }

    #[macro_export] macro_rules! null_array {
        () => {
            Value::NullArray
        }
    }

    #[macro_export] macro_rules! simple_string {
        ($data: expr) => {{
            let bytes: &[u8] = { $data };
            let rc: Rc<[u8]> = std::rc::Rc::from(bytes);
            Value::SimpleString(rc)
        }}
    }

    #[macro_export] macro_rules! bulk_string {
        ($data: expr) => {{
            let bytes: &[u8] = { $data };
            let rc: Rc<[u8]> = std::rc::Rc::from(bytes);
            Value::BulkString(rc)
        }}
    }

    #[macro_export] macro_rules! array {
        [$($data:expr),+] => {{
                 let bytes: &[Value] = &[$($data),+];
                 let b: Box<[Value]> = Box::from(bytes);
                 Value::Array(b)
        }}
    }
}

#[cfg(test)]
mod tests {}
