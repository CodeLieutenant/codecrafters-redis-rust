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

#[macro_export] macro_rules! integer {
     ($data: expr) => {{
         let data: i64 = { $data };
         Value::Integer(data)
     }};
 }
#[macro_export] macro_rules! simple_string {
        ($data: expr) => {{
            let bytes: &[u8] = { $data };
            let rc: std::rc::Rc<[u8]> = std::rc::Rc::from(bytes);
            Value::SimpleString(rc)
        }}
    }

#[macro_export] macro_rules! simple_string_rc {
        ($rc: expr) => {{
            let rc: std::rc::Rc<[u8]> = std::rc::Rc::clone($rc);
            Value::SimpleString(rc)
        }}
    }

#[macro_export] macro_rules! error {
        ($data: expr) => {{
            let bytes: &str = { $data };
            let rc: std::rc::Rc<str> = std::rc::Rc::from(bytes);
            Value::Error(rc)
        }}
    }
#[macro_export] macro_rules! error_rc {
        ($rc: expr) => {{
            let rc: std::rc::Rc<str> = std::rc::Rc::clone($c);
            Value::Error(rc)
        }}
    }

#[macro_export] macro_rules! bulk_string {
        ($data: expr) => {{
            let bytes: &[u8] = { $data };
            let rc: std::rc::Rc<[u8]> = std::rc::Rc::from(bytes);
            Value::BulkString(rc)
        }}
    }

#[macro_export] macro_rules! bulk_string_rc {
        ($rc: expr) => {{
            let rc: std::rc::Rc<[u8]> = { std::rc::Rc::clone($rc) };
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
