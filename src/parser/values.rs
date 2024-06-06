use std::borrow::Cow;
use std::cell::Cell;
use tracing::{error, instrument};
use uncased::UncasedStr;
use crate::Value;

#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("Invalid Type: {0}")]
    InvalidType(&'static str),

    #[error("Invalid number")]
    InvalidNumber,

    #[error("Invalid UTF8 Input: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Not enough arguments")]
    OutOfBounds,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Values<'a> {
    values: Box<[Value<'a>]>,
    idx: Cell<isize>,
}

impl<'a> Values<'a> {
    #[inline]
    #[instrument]
    pub(crate) fn new(values: Box<[Value<'a>]>) -> Self {
        Self {
            values,
            idx: Cell::new(-1),
        }
    }

    #[inline]
    #[instrument]
    pub(crate) fn get_number(&self) -> Result<i64, Error> {
        let arg = match self.next()? {
            Value::SimpleString(command) => {
                command.parse().map_err(|_| Error::InvalidNumber)?
            }
            Value::BulkString(command) => {
                std::str::from_utf8(command)?.parse().map_err(|_| Error::InvalidNumber)?
            },
            Value::Integer(i) => *i,
            value => {
                error!(
                    ty = value.value_type(),
                    "argument to the command must be SimpleString, BulkString or Integer"
                );
                return Err(Error::InvalidType(
                    "argument to the command must be  SimpleString, BulkString or Integer",
                ));
            }
        };

        Ok(arg)
    }

    #[inline]
    #[instrument]
    pub(crate) fn get_array(&self) -> Result<&[Value], Error> {
        Ok(&[])
    }

    #[inline]
    #[instrument]
    pub(crate) fn get_string(&self) -> Result<Cow<'_, str>, Error> {
        match self.next()? {
            Value::SimpleString(command) => Ok(Cow::<'_, str>::from(command as &str)),
            Value::BulkString(command) => Ok(Cow::Borrowed(std::str::from_utf8(command)?)),
            value => {
                error!(
                    ty = value.value_type(),
                    "argument to the command must be SimpleString or BulkString"
                );
                Err(Error::InvalidType(
                    "argument to the command must be SimpleString or BulkString",
                ))
            }
        }
    }

    #[inline]
    #[instrument]
    pub(crate) fn get_bytes(&self) -> Result<Cow<'_, [u8]>, Error> {
        match self.next()? {
            Value::BulkString(command) => Ok(Cow::Borrowed(command)),
            value => {
                error!(
                    ty = value.value_type(),
                    "argument to the command must be BulkString"
                );
                Err(Error::InvalidType(
                    "argument to the command must be BulkString",
                ))
            }
        }
    }

    #[inline]
    #[instrument]
    pub(crate) fn get_uncased_string(&self) -> Result<&UncasedStr, Error> {
        match self.next()? {
            Value::SimpleString(command) => Ok(UncasedStr::new(command)),
            Value::BulkString(command) => Ok(UncasedStr::new(std::str::from_utf8(command)?)),
            value => {
                error!(
                    ty = value.value_type(),
                    "argument to the command must be SimpleString or BulkString"
                );
                Err(Error::InvalidType(
                    "argument to the command must be SimpleString or BulkString",
                ))
            }
        }
    }

    #[inline]
    pub(crate) fn next(&self) -> Result<&Value, Error> {
        self.idx.replace(self.idx.get() + 1);
        self.check_bounds()?;
        Ok(&self.values[self.idx.get() as usize])
    }
}

impl<'a> Values<'a> {
    #[inline]
    fn check_bounds(&self) -> Result<(), Error> {
        if self.idx.get() >= self.values.len() as isize {
            Err(Error::OutOfBounds)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {}
