mod error;
mod values;

use bytes::BytesMut;
use tokio::time::Duration;
use tracing::{error, instrument};

pub use values::Error as ValueError;

use crate::redis_commands::{SetParams, SET_PARAMS};
use crate::resp::parse as parse_input;
use crate::{Command, CommandKeywords, Value, COMMAND_KEYWORDS};
use values::Values;

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

    // #[error("Invalid arguments given to the command: {0}")]
    // InvalidArguments(&'static str),
    #[error("Failed to parse input: {0}")]
    Parse(#[from] super::resp::Error),

    #[error(transparent)]
    Value(#[from] ValueError),
}

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
            CommandKeywords::Set => {
                let key = self.ast.get_bytes()?;
                let value = self.ast.next()?;

                let expiration_ms = match self.ast.get_uncased_string() {
                    Ok(val) => {
                        let param = SET_PARAMS.get(val).ok_or(Error::InvalidCommandArgument)?;

                        Some(match param {
                            SetParams::EX => Duration::from_secs(self.ast.get_number()? as u64),
                            SetParams::PX => Duration::from_millis(self.ast.get_number()? as u64),
                        })
                    }

                    Err(ValueError::OutOfBounds) => None,
                    Err(err) => return Err(Error::Value(err)),
                };

                Ok(Command::Set {
                    key,
                    value,
                    expiration: expiration_ms,
                })
            }
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
    }
}
