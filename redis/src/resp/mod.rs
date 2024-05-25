use nom::{Err as NomParseError, IResult, Parser as NomParser};
use nom::branch::alt;
use nom::bytes::streaming::take_until;
use nom::character::streaming::{char, i64 as i64_parser, line_ending};
use nom::combinator::{all_consuming, map, map_res};
use nom::error::{context, ParseError};
use nom::multi::fold_many_m_n;
use nom::sequence::{delimited, terminated};
use tracing::instrument;

use crate::value::Value;

type RespResult<'a> = IResult<&'a [u8], Value, nom::error::VerboseError<&'a [u8]>>;

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
    Utf8(#[from] std::str::Utf8Error),

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
    parse_simple('+', |val| Ok(Value::SimpleString(val.into()))).parse(input)
}

#[instrument]
#[inline]
fn parse_simple_error(input: &[u8]) -> RespResult {
    parse_simple('-', |val| {
        Ok(Value::Error(std::str::from_utf8(val)?.into()))
    })
        .parse(input)
}

#[instrument]
#[inline]
fn parse_bulk_string(input: &[u8]) -> RespResult {
    let (rest, result) = parse_length('$', OutOfRangeType::BulkString)(input)?;

    if result == -1i64 {
        return Ok((rest, Value::Null));
    }

    if result == 0i64 {
        return map(line_ending, |_| Value::BulkString(Default::default())).parse(rest);
    }

    map(terminated(take_until("\r"), line_ending), |val: &[u8]| {
        Value::BulkString(val.into())
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
pub(super) fn parse(input: &[u8]) -> Result<Value, Error> {
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
        assert_eq!(result, Ok(Value::BulkString(b"".to_vec().into())));
    }

    #[test]
    fn test_parse() {
        let input = b"+OK\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(Value::SimpleString(b"OK".to_vec().into())));
        //
        let input = b"$3\r\nfoo\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(Value::BulkString(b"foo".to_vec().into())));
        // //
        let input = b"-ERROR\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(Value::Error(Box::from("ERROR"))));
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
                    Value::BulkString(b"ECHO".to_vec().into()),
                    Value::BulkString(b"Hello".to_vec().into()),
                    Value::BulkString(b"World".to_vec().into()),
                    Value::SimpleString(b"Hello".to_vec().into()),
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
