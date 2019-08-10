// error.rs
//
// Copyright (c) 2019  Douglas Lau
//
use std::error::Error as _;
use std::fmt::{self, Display};
use std::io;
use std::str;

/// Parse errors
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParseError {
    ExpectedBool,
    ExpectedMore,
    ExpectedChar,
    ExpectedDate,
    ExpectedDateTime,
    ExpectedInt,
    ExpectedNumber,
    ExpectedTime,
    ExpectedTimeOffset,
    InvalidIndent,
    InvalidSeparator,
    InvalidType,
    MissingKey,
    MissingLinefeed,
    MissingSeparator,
    UnexpectedKey,
    UnexpectedSchemaSeparator,
}

impl ParseError {
    fn description(&self) -> &'static str {
        use ParseError::*;
        match self {
            ExpectedBool => "expected bool",
            ExpectedMore => "expected more input data",
            ExpectedChar => "expected char",
            ExpectedDate => "expected date",
            ExpectedDateTime => "expected datetime",
            ExpectedInt => "expected int",
            ExpectedNumber => "expected number",
            ExpectedTime => "expected time",
            ExpectedTimeOffset => "expected time offset",
            InvalidIndent => "invalid indent",
            InvalidSeparator => "invalid separator",
            InvalidType => "invalid type",
            MissingKey => "missing key",
            MissingLinefeed => "missing line feed",
            MissingSeparator => "missing separator",
            UnexpectedKey => "unexpected key",
            UnexpectedSchemaSeparator => "unexpected schema separator",
        }
    }
}

/// Errors which can occur when serializing and deserializing MuON data.
#[derive(Debug)]
pub enum Error {
    /// I/O errors
    IO(io::Error),
    /// Formatting error while serializing
    Format(fmt::Error),
    /// Invalid UTF-8 while deserializing
    Utf8(str::Utf8Error),
    /// Invalid UTF-8 while serializing
    FromUtf8(std::string::FromUtf8Error),
    /// Serializing error from serde
    Serialize(String),
    /// Deserializing error from serde
    Deserialize(String),
    /// Unsupported type error
    UnsupportedType(&'static str),
    /// Invalid key
    InvalidKey,
    /// Failed parse while deserializing
    FailedParse(ParseError),
}

/// MuON result type
pub type Result<T> = std::result::Result<T, Error>;

impl serde::ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Serialize(msg.to_string())
    }
}

impl serde::de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Deserialize(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.description())
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::IO(ref e) => e.description(),
            Error::Format(ref e) => e.description(),
            Error::Utf8(ref e) => e.description(),
            Error::FromUtf8(ref e) => e.description(),
            Error::Serialize(ref msg) => msg,
            Error::Deserialize(ref msg) => msg,
            Error::UnsupportedType(ref msg) => msg,
            Error::InvalidKey => "string keys only",
            Error::FailedParse(ref e) => e.description(),
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IO(e)
    }
}

impl From<fmt::Error> for Error {
    fn from(e: fmt::Error) -> Self {
        Error::Format(e)
    }
}

impl From<str::Utf8Error> for Error {
    fn from(e: str::Utf8Error) -> Self {
        Error::Utf8(e)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Error::FromUtf8(e)
    }
}
