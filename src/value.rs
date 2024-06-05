use serde::Serializer;
use std::borrow::Cow;
use std::error::Error;
use std::fmt::{Debug, Formatter};

use tracing::instrument;

#[derive(Clone, PartialEq)]
pub enum Value<'a> {
    Null,
    NullArray,
    SimpleString(Cow<'a, str>),
    Error(Cow<'a, str>),
    Integer(i64),
    BulkString(Cow<'a, str>),
    Array(Box<[Value<'a>]>),
}

pub(crate) const OK: &[u8] = b"+OK\r\n";
pub(crate) const PONG: &[u8] = b"+PONG\r\n";

impl<'a> Debug for Value<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => f.write_str("NULL"),
            Value::NullArray => f.write_str("NULL ARRAY"),
            Value::SimpleString(data) => {
                f.write_str("SIMPLE STRING(")?;
                f.write_str(std::str::from_utf8(data.as_bytes()).unwrap())?;
                f.write_str(")")
            }
            Value::Error(err) => f.write_str(std::str::from_utf8(err.as_bytes()).unwrap()),
            Value::Integer(val) => {
                f.write_str("INTEGER(")?;
                f.serialize_i64(*val)?;
                f.write_str(")")
            }
            Value::BulkString(data) => {
                f.write_str("BULK STRING(")?;
                f.write_str(std::str::from_utf8(data.as_bytes()).unwrap())?;
                f.write_str(")")
            }
            Value::Array(array) => {
                f.write_str("ARRAY[")?;

                for item in array.iter() {
                    item.fmt(f)?;
                    f.write_str(", ")?;
                }

                f.write_str("]")
            }
        }
    }
}

impl<'a> From<&(dyn Error + Send + Sync)> for Value<'a> {
    fn from(value: &(dyn Error + Send + Sync)) -> Self {
        Self::Error(value.to_string().into())
    }
}

impl<'a> Value<'a> {
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

    #[instrument]
    pub fn serialize(self, output: &mut Vec<u8>) {
        let mut buf = itoa::Buffer::new();

        match self {
            Value::Null => output.extend_from_slice(b"$-1\r\n"),
            Value::NullArray => output.extend_from_slice(b"*-1\r\n"),
            Value::SimpleString(val) => {
                output.reserve(val.len() + 3);
                output.push(b'+');
                output.extend_from_slice(val.as_bytes());
                output.extend_from_slice(b"\r\n");
            }
            Value::Error(val) => {
                output.reserve(val.len() + 3);
                output.push(b'-');
                output.extend_from_slice(val.as_bytes());
                output.extend_from_slice(b"\r\n");
            }
            Value::Integer(val) => {
                let data = buf.format(val);
                output.reserve(data.len() + 3);
                output.push(b':');
                output.extend_from_slice(data.as_bytes());
                output.extend_from_slice(b"\r\n");
            }
            Value::BulkString(val) => {
                let fmt = buf.format(val.len());
                output.reserve(val.len() + fmt.len() + 5);

                output.push(b'$');
                output.extend_from_slice(fmt.as_bytes());
                output.extend_from_slice(b"\r\n");
                output.extend_from_slice(val.as_bytes());
                output.extend_from_slice(b"\r\n");
            }
            Value::Array(array) => {
                let fmt = buf.format(array.len());
                output.reserve(fmt.len() + 3);

                output.push(b'*');
                output.extend_from_slice(fmt.as_bytes());
                output.extend_from_slice(b"\r\n");

                array
                    .into_vec()
                    .drain(..)
                    .for_each(|value| value.serialize(output));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{array, bulk_string, error, integer, null, null_array, simple_string};

    #[test]
    fn test_serialize() {
        let value = array!(
            null!(),
            null_array!(),
            integer!(100),
            bulk_string!("Hello World"),
            simple_string!("Hello World"),
            error!("SOME ERROR")
        );

        let mut output = Vec::new();
        value.serialize(&mut output);
        let output = String::from_utf8(output).unwrap();

        assert_eq!(
            output.as_str(),
            "*6\r\n$-1\r\n*-1\r\n:100\r\n$11\r\nHello World\r\n+Hello World\r\n-SOME ERROR\r\n"
        );
    }
}
