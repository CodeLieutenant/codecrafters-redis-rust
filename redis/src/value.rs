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

    #[instrument]
    pub fn serialize(self, output: &mut Vec<u8>) {
        let mut buf = itoa::Buffer::new();

        match self {
            Value::Null => output.extend_from_slice(b"$-1\r\n"),
            Value::NullArray => output.extend_from_slice(b"*-1\r\n"),
            Value::SimpleString(val) => {
                output.reserve(val.len() + 3);
                output.push(b'+');
                output.extend_from_slice(&val);
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
                output.extend_from_slice(&val);
                output.extend_from_slice(b"\r\n");
            }
            Value::Array(array) => {
                let fmt = buf.format(array.len());
                output.reserve(fmt.len() + 3);

                output.push(b'*');
                output.extend_from_slice(fmt.as_bytes());
                output.extend_from_slice(b"\r\n");

                array.into_vec().drain(..).for_each(|value| value.serialize(output));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{array, error, bulk_string, integer, null, null_array, simple_string};
    use super::*;

    #[test]
    fn test_serialize() {
        let value = array!(
            null!(),
            null_array!(),
            integer!(100),
            bulk_string!(b"Hello World"),
            simple_string!(b"Hello World"),
            error!("SOME ERROR")
        );

        let mut output = Vec::new();
        let serialized = value.serialize(&mut output);
        let output = String::from_utf8(output).unwrap();

        assert_eq!(
            output.as_str(),
            "*6\r\n$-1\r\n*-1\r\n:100\r\n$11\r\nHello World\r\n+Hello World\r\n-SOME ERROR\r\n"
        );
    }
}
