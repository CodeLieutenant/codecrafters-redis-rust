#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    String(Box<str>),
    Bytes(Box<[u8]>),
    Integer(i64),
    Null,
}

impl<'a> TryFrom<&crate::Value<'a>> for Value {
    type Error = &'static str;

    fn try_from(value: &crate::Value<'a>) -> Result<Self, Self::Error> {
        match value {
            crate::Value::Null => Ok(Value::Null),
            crate::Value::SimpleString(val) => Ok(Value::String(val.to_string().into_boxed_str())),
            crate::Value::Integer(val) => Ok(Value::Integer(*val)),
            crate::Value::BulkString(val) => Ok(Value::Bytes(val.to_vec().into_boxed_slice())),
            _ => Err("invalid value"),
        }
    }
}

impl TryFrom<&str> for Value {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Ok(Value::String(value.to_string().into_boxed_str()))
    }
}

impl TryFrom<&[u8]> for Value {
    type Error = &'static str;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        Ok(Value::Bytes(value.to_vec().into_boxed_slice()))
    }
}

impl TryFrom<i64> for Value {
    type Error = &'static str;

    fn try_from(value: i64) -> Result<Self, Self::Error> {
        Ok(Value::Integer(value))
    }
}