use nom::{Err as NomParseError, IResult, Parser as NomParser};
use nom::branch::alt;
use nom::bytes::streaming::take_until;
use nom::character::streaming::{char, i64 as i64_parser, line_ending};
use nom::combinator::{all_consuming, map, map_res};
use nom::error::{context, ParseError};
use nom::multi::fold_many_m_n;
use nom::sequence::{delimited, terminated};
use tracing::instrument;

use crate::command::{RedisCommand, RedisValue};

pub const RESP_MAX_SIZE: usize = 512 * 1024 * 1024;

#[derive(Clone, Debug)]
pub struct Parser {
    ast: RedisValue,
}

impl Parser {
    pub fn command(&self) -> Result<RedisCommand, ParserError> {
        Ok(RedisCommand::Echo("".into()))
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum OutOfRangeType {
    Array,
    BulkString,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ParserError {
    #[error("Number out of range {0:?}: {1} (valid values: -1, >= 0 < 512MiB)")]
    OutOfRange(OutOfRangeType, i64),

    #[error("Failed to parse input: {0}")]
    ParseError(#[from] nom::error::VerboseError<String>),

    #[error("String must be UTF8: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),

    #[error("needs more input")]
    Incomplete,
}

#[instrument]
#[inline]
fn parse_simple_string(
    input: &[u8],
) -> IResult<&[u8], RedisValue, nom::error::VerboseError<&[u8]>> {
    map_res(
        delimited(char('+'), take_until("\r"), line_ending),
        |val: &[u8]| Result::<RedisValue, ParserError>::Ok(RedisValue::SimpleString(val.into())),
    )
        .parse(input)
}

#[instrument]
#[inline]
fn parse_simple_error(input: &[u8]) -> IResult<&[u8], RedisValue, nom::error::VerboseError<&[u8]>> {
    map_res(
        context("simple_error_delimited", |input| {
            delimited(char('-'), take_until("\r"), line_ending).parse(input)
        }),
        |val: &[u8]| {
            Result::<RedisValue, ParserError>::Ok(RedisValue::Error(
                std::str::from_utf8(val)?.into(),
            ))
        },
    )
        .parse(input)
}

#[instrument]
#[inline]
fn parse_bulk_string(input: &[u8]) -> IResult<&[u8], RedisValue, nom::error::VerboseError<&[u8]>> {
    let (rest, result) = parse_length('$', OutOfRangeType::BulkString)(input)?;

    if result == -1i64 {
        return Ok((rest, RedisValue::Null));
    }

    if result == 0i64 {
        return map(line_ending, |_| RedisValue::BulkString(Default::default())).parse(rest);
    }

    map(terminated(take_until("\r"), line_ending), |val: &[u8]| {
        RedisValue::BulkString(val.into())
    })
        .parse(rest)
}

#[instrument]
#[inline]
fn parse_integer(input: &[u8]) -> IResult<&[u8], RedisValue, nom::error::VerboseError<&[u8]>> {
    map(delimited(char(':'), i64_parser, line_ending), |val: i64| {
        RedisValue::Integer(val)
    })
        .parse(input)
}

#[inline]
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
                val => Err(ParserError::OutOfRange(out_of_range_type, val)),
            },
        )
            .parse(input)
    }
}

#[inline]
fn parse_any(input: &[u8]) -> IResult<&[u8], RedisValue, nom::error::VerboseError<&[u8]>> {
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
fn parse_array(input: &[u8]) -> IResult<&[u8], RedisValue, nom::error::VerboseError<&[u8]>> {
    let (rest, result) = parse_length('*', OutOfRangeType::Array)(input)?;

    if result == -1i64 {
        return Ok((rest, RedisValue::NullArray));
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

    Ok((rest, RedisValue::Array(value.into())))
}

#[inline]
pub fn parse(input: &[u8]) -> Result<Parser, ParserError> {
    match all_consuming(parse_any).parse(input) {
        Ok((&[], redis_type)) => Ok(Parser { ast: redis_type }),
        Ok((rest, _)) => Err(ParserError::ParseError(
            nom::error::VerboseError::from_error_kind(
                std::str::from_utf8(rest)?.to_string(),
                nom::error::ErrorKind::Fail,
            ),
        )),
        Err(NomParseError::Incomplete(size)) => {
            println!("{size:?}");
            Err(ParserError::Incomplete)
        }
        Err(err) => Err(ParserError::ParseError(
            nom::error::VerboseError::from_error_kind(err.to_string(), nom::error::ErrorKind::Fail),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_nulls() {
        let input = b"$-1\r\n";

        let result = parse(input);
        assert_eq!(result, Ok(RedisValue::Null));

        let input = b"*-1\r\n";

        let result = parse(input);
        assert_eq!(result, Ok(RedisValue::NullArray));
    }

    #[test]
    fn test_parse_empty_array() {
        let input = b"*0\r\n";

        let result = parse(input);
        assert_eq!(result, Ok(RedisValue::Array(vec![].into())));
    }

    #[test]
    fn test_parse_empty_string() {
        let input = b"$0\r\n\r\n";

        let result = parse(input);
        assert_eq!(result, Ok(RedisValue::BulkString(b"".to_vec().into())));
    }

    #[test]
    fn test_parse() {
        let input = b"+OK\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(RedisValue::SimpleString(b"OK".to_vec().into())));
        //
        let input = b"$3\r\nfoo\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(RedisValue::BulkString(b"foo".to_vec().into())));
        // //
        let input = b"-ERROR\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(RedisValue::Error(Box::from("ERROR"))));
        //
        let input = b":123\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(RedisValue::Integer(123)));
        //
        let input = b":-123\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(RedisValue::Integer(-123)));

        let input = b"*1\r\n:523\r\n";
        let result = parse(input);
        assert_eq!(
            result,
            Ok(RedisValue::Array(vec![RedisValue::Integer(523)].into()))
        );

        let input = b"*4\r\n$4\r\nECHO\r\n$5\r\nHello\r\n$5\r\nWorld\r\n+Hello\r\n";
        let result = parse(input);
        assert_eq!(
            result,
            Ok(RedisValue::Array(
                vec![
                    RedisValue::BulkString(b"ECHO".to_vec().into()),
                    RedisValue::BulkString(b"Hello".to_vec().into()),
                    RedisValue::BulkString(b"World".to_vec().into()),
                    RedisValue::SimpleString(b"Hello".to_vec().into()),
                ]
                    .into()
            ))
        );
    }

    #[test]
    fn test_not_enough_data() {
        let input = b":123";
        let result = parse(input);
        assert_eq!(result, Err(ParserError::Incomplete));
    }
}
