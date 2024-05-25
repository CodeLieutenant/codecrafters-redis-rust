#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    NullArray,
    SimpleString(Box<[u8]>),
    Error(Box<str>),
    Integer(i64),
    BulkString(Box<[u8]>),
    Array(Box<[Value]>),
}

impl Value {
    pub fn serialize(&self) {}
}

#[cfg(test)]
mod tests {}
