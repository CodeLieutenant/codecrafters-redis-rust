use bytes::BufMut;

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


impl RedisValue {
    pub fn serialize(self, output: &mut Vec<u8>) {
        let mut buf = itoa::Buffer::new();


        match self {
            RedisValue::Null => output.extend_from_slice(b"$-1\r\n"),
            RedisValue::NullArray => output.extend_from_slice(b"*-1\r\n"),
            RedisValue::SimpleString(val) => {
                output.reserve(val.len() + 3);
                output.push(b'+');
                output.extend_from_slice(&val);
                output.extend_from_slice(b"\r\n");
            }
            RedisValue::Error(val) => {
                output.reserve(val.len() + 3);
                output.push(b'-');
                output.extend_from_slice(val.into());
                output.extend_from_slice(b"\r\n");
            }
            RedisValue::Integer(val) => {
                output.reserve(val.len() + 3);
                output.push(b':');
                output.extend_from_slice(buf.format(val).into());
                output.extend_from_slice(b"\r\n");
            }
            RedisValue::BulkString(val) => {
                let fmt = buf.format(val);
                output.reserve(val.len() + fmt.len() + 5);

                output.push(b'$');
                output.extend_from_slice(fmt.into());
                output.extend_from_slice(b"\r\n");
                output.extend_from_slice(&val);
                output.extend_from_slice(b"\r\n");
            }
            RedisValue::Array(_) => {}
        }
    }
}