use std::borrow::Cow;
use std::str::Utf8Error;

use nom::branch::alt;
use nom::bytes::streaming::take_until;
use nom::character::streaming::{char, i64 as i64_parser, line_ending};
use nom::combinator::{all_consuming, map, map_res};
use nom::error::{context, ParseError};
use nom::multi::fold_many_m_n;
use nom::sequence::{delimited, terminated};
use nom::{Err as NomParseError, IResult, Parser as NomParser};
use tracing::instrument;
use crate::Value;

type RespResult<'a> = IResult<&'a [u8], Value<'a>, nom::error::VerboseError<&'a [u8]>>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum OutOfRangeType {
    Array,
    BulkString,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum Error {
    #[error("Number out of range {0:?}: {1} (valid values: -1, >= 0 < 512MiB)")]
    OutOfRange(OutOfRangeType, i64),

    #[error("Failed to parse input: {0}")]
    Parse(#[from] nom::error::VerboseError<String>),

    #[error("String must be UTF8: {0}")]
    Utf8(#[from] Utf8Error),

    #[error("needs more input")]
    Incomplete,
}

const RESP_MAX_SIZE: usize = 512 * 1024 * 1024;

#[instrument]
#[inline]
fn parse_simple<'a>(
    indicator: char,
    cb: fn(&'a [u8]) -> Result<Value, Error>,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], Value, nom::error::VerboseError<&'a [u8]>> {
    move |input| {
        map_res(
            delimited(char(indicator), take_until("\r"), line_ending),
            cb,
        )
            .parse(input)
    }
}

#[instrument]
#[inline]
fn parse_simple_string(input: &[u8]) -> RespResult {
    parse_simple('+', |val| {
        Ok(if val.len() == 0 {
            Value::SimpleString(EMTPY_STR)
        } else {
            Value::SimpleString(Cow::Borrowed(std::str::from_utf8(val)?))
        })
    })
        .parse(input)
}

#[instrument]
#[inline]
fn parse_simple_error(input: &[u8]) -> RespResult {
   parse_simple('-', |val| {
        Ok(Value::Error(std::str::from_utf8(val)?.into()))
    })
        .parse(input)
}

pub(crate) const EMTPY_STR: Cow<'static, str> = Cow::Owned(String::new());
pub(crate) const EMTPY_BYTES: Cow<'static, [u8]> = Cow::Borrowed(&[]);

#[instrument]
#[inline]
fn parse_bulk_string(input: &[u8]) -> RespResult {
    let (rest, result) = parse_length('$', OutOfRangeType::BulkString)(input)?;

    if result == -1i64 {
        return Ok((rest, Value::Null));
    }

    if result == 0i64 {
        return map(line_ending, |_| Value::BulkString(EMTPY_BYTES)).parse(rest);
    }

    map_res(terminated(take_until("\r"), line_ending), |val: &[u8]| {
        Ok::<Value, Utf8Error>(Value::BulkString(Cow::Borrowed(val)))
    })
        .parse(rest)
}

#[instrument]
#[inline]
fn parse_integer(input: &[u8]) -> RespResult {
    map(delimited(char(':'), i64_parser, line_ending), |val: i64| {
        Value::Integer(val)
    })
        .parse(input)
}

#[inline]
#[instrument]
fn parse_length<'a>(
    delimiter: char,
    out_of_range_type: OutOfRangeType,
) -> impl FnMut(&'a [u8]) -> IResult<&'a [u8], i64, nom::error::VerboseError<&'a [u8]>> {
    move |input| {
        map_res(
            context(
                "length_parser",
                delimited(char(delimiter), i64_parser, line_ending),
            ),
            |len| match len {
                len if len >= -1 && len < RESP_MAX_SIZE as i64 => Ok(len),
                val => Err(Error::OutOfRange(out_of_range_type, val)),
            },
        )
            .parse(input)
    }
}

#[inline]
#[instrument]
fn parse_any(input: &[u8]) -> RespResult {
    context(
        "parse_any",
        alt((
            context("simple_string", parse_simple_string),
            context("array", parse_array),
            context("simple_error", parse_simple_error),
            context("bulk_string", parse_bulk_string),
            context("integer", parse_integer),
        )),
    )
        .parse(input)
}

#[instrument]
#[inline]
fn parse_array(input: &[u8]) -> RespResult {
    let (rest, result) = parse_length('*', OutOfRangeType::Array)(input)?;

    if result == -1i64 {
        return Ok((rest, Value::NullArray));
    }

    let (rest, value) = fold_many_m_n(
        result as usize,
        result as usize,
        parse_any,
        move || Vec::with_capacity(result as usize),
        |mut acc, item| {
            acc.push(item);
            acc
        },
    )(rest)?;

    Ok((rest, Value::Array(value.into())))
}

#[inline]
#[instrument]
pub fn parse(input: &[u8]) -> Result<Value, Error> {
    match all_consuming(parse_any).parse(input) {
        Ok((&[], redis_type)) => Ok(redis_type),
        Ok((rest, _)) => Err(Error::Parse(nom::error::VerboseError::from_error_kind(
            std::str::from_utf8(rest)?.to_string(),
            nom::error::ErrorKind::Fail,
        ))),
        Err(NomParseError::Incomplete(_)) => Err(Error::Incomplete),
        Err(err) => Err(Error::Parse(nom::error::VerboseError::from_error_kind(
            err.to_string(),
            nom::error::ErrorKind::Fail,
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! cow_str {
        ($data: expr) => {{
            let item: &str = { $data };

            std::borrow::Cow::Borrowed(item)
        }};
    }
    macro_rules! cow_bytes {
        ($data: expr) => {{
            let item: &[u8] = { $data };

            std::borrow::Cow::Borrowed(item)
        }};
    }

    #[test]
    fn test_parse_nulls() {
        let input = b"$-1\r\n";

        let result = parse(input);
        assert_eq!(result, Ok(Value::Null));

        let input = b"*-1\r\n";

        let result = parse(input);
        assert_eq!(result, Ok(Value::NullArray));
    }

    #[test]
    fn test_parse_empty_array() {
        let input = b"*0\r\n";

        let result = parse(input);
        assert_eq!(result, Ok(Value::Array(vec![].into())));
    }

    #[test]
    fn test_parse_empty_string() {
        let input = b"$0\r\n\r\n";

        let result = parse(input);
        assert_eq!(result, Ok(Value::BulkString(cow_bytes!(b""))));
    }

    #[test]
    fn test_parse() {
        let input = b"+OK\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(Value::SimpleString(cow_str!("OK"))));
        //
        let input = b"$3\r\nfoo\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(Value::BulkString(cow_bytes!(b"foo"))));
        // //
        let input = b"-ERROR\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(Value::Error(cow_str!("ERROR"))));
        //
        let input = b":123\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(Value::Integer(123)));
        //
        let input = b":-123\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(Value::Integer(-123)));

        let input = b"*1\r\n:523\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(Value::Array(vec![Value::Integer(523)].into())));

        let input = b"*4\r\n$4\r\nECHO\r\n$5\r\nHello\r\n$5\r\nWorld\r\n+Hello\r\n";
        let result = parse(input);
        assert_eq!(
            result,
            Ok(Value::Array(
                vec![
                    Value::BulkString(cow_bytes!(b"ECHO")),
                    Value::BulkString(cow_bytes!(b"Hello")),
                    Value::BulkString(cow_bytes!(b"World")),
                    Value::SimpleString(cow_str!("Hello")),
                ]
                    .into()
            ))
        );
    }

    #[test]
    fn test_not_enough_data() {
        let input = b":123";
        let result = parse(input);
        assert_eq!(result, Err(Error::Incomplete));
    }
}
