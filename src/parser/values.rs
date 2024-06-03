use crate::value::Value;
use std::borrow::Cow;
use tracing::error;
use uncased::UncasedStr;

#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("Invalid Type: {0}")]
    InvalidType(&'static str),

    #[error("Invalid UTF8 Input: {0}")]
    Utf8(#[from] std::str::Utf8Error),

    #[error("Not enough arguments")]
    OutOfBounds,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Values<'a> {
    values: Box<[Value<'a>]>,
    idx: isize,
}

impl<'a> Values<'a> {
    #[inline]
    pub(crate) fn new(values: Box<[Value<'a>]>) -> Self {
        Self { values, idx: -1 }
    }

    #[inline]
    pub(crate) fn get_number(&mut self) -> Result<i64, Error> {
        let arg = match self.next()? {
            Value::SimpleString(_command) | Value::BulkString(_command) => 0,
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
    pub(crate) fn get_array(&mut self) -> Result<&[Value], Error> {
        Ok(&[])
    }

    #[inline]
    pub(crate) fn get_string(&mut self) -> Result<Cow<'_, str>, Error> {
        match self.next()? {
            Value::SimpleString(command) | Value::BulkString(command) => {
                Ok(Cow::<'_, str>::from(command as &str))
            }
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
    pub(crate) fn get_uncased_string(&mut self) -> Result<&UncasedStr, Error> {
        match self.next()? {
            Value::SimpleString(command) | Value::BulkString(command) => {
                Ok(UncasedStr::new(command))
            }
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
}

impl<'a> Values<'a> {
    #[inline]
    fn next(&mut self) -> Result<&Value, Error> {
        self.check_bounds()?;
        self.idx += 1;
        Ok(&self.values[self.idx as usize])
    }

    #[inline]
    fn check_bounds(&self) -> Result<(), Error> {
        if self.idx >= self.values.len() as isize {
            Err(Error::OutOfBounds)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {}
