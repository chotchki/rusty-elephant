//! Top Level of the sql parsing engine

mod common;
mod create;
mod insert;
mod select;

use self::select::parse_select;

use super::objects::ParseTree;
use create::parse_create_table;
use insert::parse_insert;
use nom::branch::alt;
use nom::bytes::complete::tag;
use nom::combinator::{all_consuming, complete, opt};
use nom::error::{convert_error, ContextError, ParseError, VerboseError};
use nom::sequence::tuple;
use nom::Finish;
use nom::IResult;
use thiserror::Error;

pub struct SqlParser {}

impl SqlParser {
    pub fn parse(input: &str) -> Result<ParseTree, SqlParserError> {
        match SqlParser::nom_parse::<VerboseError<&str>>(input).finish() {
            Ok((_, cmd)) => Ok(cmd),
            Err(e) => Err(SqlParserError::ParseError(convert_error(input, e))),
        }
    }

    fn nom_parse<'a, E: ParseError<&'a str> + ContextError<&'a str>>(
        input: &'a str,
    ) -> IResult<&'a str, ParseTree, E> {
        //TODO Had to remove all consuming since it was throwing EOF issues
        let (input, (result, _)) = complete(tuple((
            alt((parse_create_table, parse_insert, parse_select)),
            opt(tag(";")),
        )))(input)?;
        Ok((input, result))
    }
}

#[derive(Debug, Error)]
pub enum SqlParserError {
    #[error("SQL Parse Error {0}")]
    ParseError(String),
    #[error("Got an incomplete on {0} which shouldn't be possible")]
    Incomplete(String),
}
