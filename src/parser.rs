use nom::branch::alt;
use nom::bytes::complete::{take_until, take_while};
use nom::character::complete::{char, i64 as i64_parser, line_ending};
use nom::combinator::map;
use nom::error::{context, ParseError};
use nom::multi::many0;
use nom::sequence::{delimited, preceded, terminated, Tuple};
use nom::{IResult, Parser};

#[derive(Debug, Clone, PartialEq)]
pub enum RedisType {
    SimpleString(String),
    Error(String),
    Integer(i64),
    BulkString(String),
    Array(Vec<RedisType>),
}

fn parse_simple_string(input: &str) -> IResult<&str, RedisType, nom::error::VerboseError<&str>> {
    map(
        delimited(char('+'), take_until("\r\n"), line_ending),
        |val: &str| RedisType::SimpleString(val.to_string()),
    )
    .parse(input)
}

fn parse_simple_error(input: &str) -> IResult<&str, RedisType, nom::error::VerboseError<&str>> {
    map(
        delimited(char('-'), take_until("\r\n"), line_ending),
        |val: &str| RedisType::Error(val.to_string()),
    )
    .parse(input)
}

fn parse_bulk_string<'a>(
    input: &'a str,
) -> IResult<&str, RedisType, nom::error::VerboseError<&str>> {
    map(
        context("bulk_string_with_length", |val: &'a str| {
            (
                delimited(
                    char('$'),
                    terminated(take_while(|c| c != '\r'), char('\r')),
                    line_ending,
                ),
                terminated(take_while(|c| c != '\r'), line_ending),
            )
                .parse(val)
        }),
        |(_len, val)| RedisType::BulkString(val.to_string()),
    )
    .parse(input)
}

fn parse_integer(input: &str) -> IResult<&str, RedisType, nom::error::VerboseError<&str>> {
    map(delimited(char(':'), i64_parser, line_ending), |val: i64| {
        RedisType::Integer(val)
    })
    .parse(input)
}

fn parse_array<'a>(input: &'a str) -> IResult<&str, RedisType, nom::error::VerboseError<&str>> {
    map(
        context("array_parser", |val: &'a str| {
            (
                context("array_length_parser", |val| {
                    (
                        preceded(char('*'), i64_parser),
                        terminated(take_while(|c| c != '\r'), char('\r')),
                        line_ending,
                    )
                        .parse(val)
                }),
                context("array_parser_inner", |val| {
                    (many0(alt((
                        context("simple_string", parse_simple_string),
                        context("simple_error", parse_simple_error),
                        context("bulk_string", parse_bulk_string),
                        context("integer", parse_integer),
                    ))),)
                        .parse(val)
                }),
            )
                .parse(val)
        }),
        |(_, v)| RedisType::Array(v.0),
    )
    .parse(input)
}

pub fn parse(input: &str) -> Result<RedisType, nom::error::VerboseError<String>> {
    let result = alt((
        (context("array", parse_array)),
        (context("simple_string", parse_simple_string)),
        (context("simple_error", parse_simple_error)),
        (context("bulk_string", parse_bulk_string)),
        (context("integer", parse_integer)),
    ))
    .parse(input);

    match result {
        Ok(("", redis_type)) => Ok(redis_type),
        Ok((rest, _)) => Err(nom::error::VerboseError::from_error_kind(
            rest.to_string(),
            nom::error::ErrorKind::Fail,
        )),
        Err(err) => Err(nom::error::VerboseError::from_error_kind(
            err.to_string(),
            nom::error::ErrorKind::Fail,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_string() {
        let input = "+OK\r\n";
        let result = parse_simple_string(input);
        assert_eq!(result, Ok(("", RedisType::SimpleString("OK".to_string()))));

        let input = "+PING\r\n";
        let result = parse_simple_string(input);
        assert_eq!(
            result,
            Ok(("", RedisType::SimpleString("PING".to_string())))
        );
    }

    #[test]
    fn test_simple_error() {
        let input = "-ERROR\r\n";
        let result = parse_simple_error(input);
        assert_eq!(result, Ok(("", RedisType::Error("ERROR".to_string()))));
    }

    #[test]
    fn test_bulk_string() {
        let input = "$3\r\nfoo\r\n";
        let result = parse_bulk_string(input);
        assert_eq!(result, Ok(("", RedisType::BulkString("foo".to_string()))));
    }

    #[test]
    fn test_parse() {
        let input = "+OK\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(RedisType::SimpleString("OK".to_string())));

        let input = "$3\r\nfoo\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(RedisType::BulkString("foo".to_string())));

        let input = "-ERROR\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(RedisType::Error("ERROR".to_string())));

        let input = ":123\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(RedisType::Integer(123)));

        let input = ":-123\r\n";
        let result = parse(input);
        assert_eq!(result, Ok(RedisType::Integer(-123)));

        let input = "*3\r\n:523\r\n+hello\r\n$5\r\nhello\r\n";
        let result = parse(input);
        assert_eq!(
            result,
            Ok(RedisType::Array(vec![
                RedisType::Integer(523),
                RedisType::SimpleString("hello".to_string()),
                RedisType::BulkString("hello".to_string())
            ]))
        );

        let input = "*3\r\n$4\r\nECHO\r\n$5\r\nHello\r\n$5\r\nWorld\r\n";
        let result = parse(input);
        assert_eq!(
            result,
            Ok(RedisType::Array(vec![
                RedisType::BulkString("ECHO".to_string()),
                RedisType::BulkString("Hello".to_string()),
                RedisType::BulkString("World".to_string())
            ]))
        );
    }

    #[test]
    fn test_integer() {
        let input = ":123\r\n";
        let result = parse_integer(input);
        assert_eq!(result, Ok(("", RedisType::Integer(123))));

        let input = ":-123\r\n";
        let result = parse_integer(input);
        assert_eq!(result, Ok(("", RedisType::Integer(-123))));
    }
}
