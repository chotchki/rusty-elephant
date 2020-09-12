use std::io;

//See here for error types: https://doc.rust-lang.org/stable/rust-by-example/error/multiple_error_types/define_error_type.html

#[derive(Debug)]
pub enum ParserError {
    IoError(io::Error),
    FormatError()
}

impl From<io::Error> for ParserError {
    fn from(error: io::Error) -> Self {
        ParserError::IoError(error)
    }
}

impl From<String> for ParserError {
    fn from(string: String) -> Self {
        ParserError::FormatError()
    }
}