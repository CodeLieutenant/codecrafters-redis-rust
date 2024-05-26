use std::rc::Rc;

use tracing::{error, info, instrument};

use crate::{value::Value, Command, CommandKeywords, COMMAND_KEYWORDS};

use crate::resp::parse as parse_input;

#[derive(Clone, Debug, PartialEq)]
pub struct Parser {
    pub(super) ast: Value,
}

unsafe impl Send for Parser {}

unsafe impl Sync for Parser {}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid command")]
    InvalidCommand,

    #[error("command does not exist")]
    NotExists,

    #[error("Invalid arguments given to the command {0}: {1}")]
    InvalidArguments(&'static str, &'static str),

    #[error("Failed to parse input: {0}")]
    ParseError(#[from] super::resp::Error),

    #[error("Invalid UTF8 Input: {0}")]
    Utf8(#[from] std::str::Utf8Error),
}

impl Parser {
    pub fn parse(input: impl AsRef<[u8]>) -> Result<Self, Error> {
        Ok(Self {
            ast: parse_input(input.as_ref())?,
        })
    }

    #[instrument]
    fn extract_params(values: Box<[Value]>) -> Result<Command, Error> {
        let command: &[u8] = match &values[0] {
            Value::SimpleString(command) | Value::BulkString(command) => command,
            _ => {
                return Err(Error::InvalidCommand);
            }
        };

        let command = std::str::from_utf8(command)?;
        let command = uncased::UncasedStr::new(command);
        let command = COMMAND_KEYWORDS.get(command).ok_or(Error::NotExists)?;

        match command {
            CommandKeywords::Ping => Ok(Command::Ping),
            CommandKeywords::Command => Ok(Command::Ping),
            CommandKeywords::Echo => {
                let values = &values[1..];

                if values.len() != 1 {
                    return Err(Error::InvalidArguments(
                        "echo",
                        "echo command required exactly 1 argument",
                    ));
                }

                let arg = match &values[0] {
                    Value::SimpleString(command) | Value::BulkString(command) => Rc::clone(command),
                    val => {
                        error!(
                            ty = val.value_type(),
                            command = "echo",
                            "argument to the command must be SimpleString or BulkString"
                        );
                        return Err(Error::InvalidArguments(
                            "echo",
                            "argument to the command must be SimpleString or BulkString",
                        ));
                    }
                };

                Ok(Command::Echo(arg))
            }
        }
    }

    #[instrument]
    pub fn command(self) -> Result<Command, Error> {
        match self.ast {
            Value::SimpleString(val) => {
                let s = std::str::from_utf8(&val)?;

                // Only command that can be sent as SimpleString is PING
                // Everything else must be sent as ARRAY
                if s.eq_ignore_ascii_case("ping") {
                    Ok(Command::Ping)
                } else {
                    Err(Error::InvalidCommand)
                }
            }
            Value::Array(val) => {
                if val.len() == 0 {
                    return Err(Error::InvalidCommand);
                }

                Self::extract_params(val)
            }

            _ => Err(Error::InvalidCommand),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{array, bulk_string, simple_string, Command};

    use super::*;

    #[test]
    fn test_parse_ping_command() {
        let parser = Parser {
            ast: simple_string!(b"PING"),
        };

        let result = parser.command();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Command::Ping);

        let parser = Parser {
            ast: array![simple_string!(b"PING")],
        };

        let result = parser.command();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Command::Ping);

        let parser = Parser {
            ast: array![bulk_string!(b"PING")],
        };

        let result = parser.command();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Command::Ping);
    }
}
