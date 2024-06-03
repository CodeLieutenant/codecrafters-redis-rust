#[macro_export]
macro_rules! null {
    () => {
        Value::Null
    };
}

#[macro_export]
macro_rules! null_array {
    () => {
        Value::NullArray
    };
}

#[macro_export]
macro_rules! integer {
    ($data: expr) => {{
        let data: i64 = { $data };
        Value::Integer(data)
    }};
}
#[macro_export]
macro_rules! simple_string {
    ($data: expr) => {{
        Value::SimpleString(std::borrow::Cow::Borrowed($data))
    }};
}

#[macro_export]
macro_rules! error {
    ($data: expr) => {
        Value::Error(std::borrow::Cow::Borrowed($data))
    };
}

#[macro_export]
macro_rules! bulk_string {
    ($data: expr) => {
        Value::BulkString(std::borrow::Cow::Borrowed($data))
    };
}

#[macro_export]
macro_rules! array {
    [$($data:expr),+] => {{
             let bytes: &[Value] = &[$($data),+];
             let b: Box<[Value]> = Box::from(bytes);
             Value::Array(b)
    }}
}
#[macro_export]
macro_rules! array_box {
    [$($data:expr),+] => {{
        let bytes: &[Value] = &[$($data),+];
        let b: Box<[Value]> = Box::from(bytes);
        b
    }}
}
