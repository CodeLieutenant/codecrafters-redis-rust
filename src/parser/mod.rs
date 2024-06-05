mod error;
mod values;

use bytes::BytesMut;
use tracing::{error, instrument};

use crate::parser::values::{Error as ValueError, Values};
use crate::redis_commands::{CommandKeywords, COMMAND_KEYWORDS};
use crate::resp::parse as parse_input;
use crate::{value::Value, Command};

#[derive(Clone, Debug, PartialEq)]
pub struct Parser<'a> {
    pub(super) ast: Values<'a>,
}

unsafe impl<'a> Send for Parser<'a> {}

unsafe impl<'a> Sync for Parser<'a> {}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid command")]
    InvalidInput,

    #[error("command does not exist")]
    NotExists,

    #[error("Command argument does not exist")]
    InvalidCommandArgument,

    #[error("Invalid arguments given to the command: {0}")]
    InvalidArguments(&'static str),

    #[error("Failed to parse input: {0}")]
    Parse(#[from] super::resp::Error),

    #[error(transparent)]
    Value(#[from] ValueError),
}

// fn parse_get_command(values: Box<[Value]>) -> Result<Command, Error> {
//     let values = &values[1..];
//
//     if values.len() == 0 {
//         return Err(Error::InvalidArguments("get command requires 1 argument"));
//     }
//
//     let key = match &values[0] {
//         Value::SimpleString(command) | Value::BulkString(command) => Rc::clone(command),
//         _ => {
//             return Err(Error::InvalidArguments(
//                 "argument to the command must be SimpleString or BulkString",
//             ));
//         }
//     };
//
//     Ok(Command::Get(key))
// }
//
// fn parse_set_command(values: Box<[Value]>) -> Result<Command, Error> {
//     let values = &values[1..];
//
//     if values.len() == 0 {
//         return Err(Error::InvalidArguments(
//             "set command requires at least 1 argument",
//         ));
//     }
//
//     let key = get_key(&values[0])?;
//
//     if values.len() > 1 {
//         let flags = SET_PARAMS
//             .get(uncased_str(&values[1])?.as_uncased_str())
//             .ok_or(Error::NotExists)?;
//     }
//
//     Ok(Command::Set { key })
// }

impl<'a> Parser<'a> {
    pub fn parse(input: &'a BytesMut) -> Result<Self, Error> {
        let values = match parse_input(input)? {
            Value::Array(val) => Values::new(val),
            _ => return Err(Error::InvalidInput),
        };

        Ok(Self { ast: values })
    }

    #[instrument]
    pub fn command(&mut self) -> Result<Command, Error> {
        let command = COMMAND_KEYWORDS
            .get(self.ast.get_uncased_string()?)
            .ok_or(Error::NotExists)?;

        match command {
            CommandKeywords::Ping => Ok(Command::Ping),
            CommandKeywords::Command => Ok(Command::Command),
            CommandKeywords::Echo => Ok(Command::Echo(self.ast.get_string()?)),
            CommandKeywords::Get => Ok(Command::Get(self.ast.get_string()?)),
            CommandKeywords::Set => Ok(Command::Set {
                key: self.ast.get_string()?,
                value: self.ast.get_string()?,
                expiration_ms: -1,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{array_box, simple_string, Command};

    use super::*;

    #[test]
    fn test_parse_ping_command() {
        let mut parser = Parser {
            ast: Values::new(array_box![simple_string!("PING")]),
        };

        let result = parser.command();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Command::Ping);
        //
        // let parser = Parser {
        //     ast: array![bulk_string!(b"PING")],
        // };
        //
        // let result = parser.command();
        //
        // assert!(result.is_ok());
        // assert_eq!(result.unwrap(), Command::Ping);
    }
}
