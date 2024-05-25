use crate::{Command, value::Value};

use super::error::Error;
use super::resp::parse as parse_input;

#[derive(Clone, Debug, PartialEq)]
pub struct Parser {
    pub(super) ast: Value,
}


impl Parser {
    pub fn parse(input: impl AsRef<[u8]>) -> Result<Self, Error> {
        Ok(Self {
            ast: parse_input(input.as_ref())?,
        })
    }

    pub fn command(&self) -> Result<Command, Error> {
        match self.ast {
            Value::SimpleString(_) => Ok(Command::Ping),
            Value::Array(_) => Ok(Command::Ping),
            _ => Err(Error::InvalidCommand),
        }
    }
}

#[cfg(test)]
mod tests {}
