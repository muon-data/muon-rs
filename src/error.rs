// error.rs
//
// Copyright (c) 2019-2020  Douglas Lau
//
use std::fmt::{self, Display};
use std::io;
use std::str::{ParseBoolError, Utf8Error};

/// Parse errors
#[non_exhaustive]
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
    InvalidDefault,
    InvalidIndent,
    InvalidSeparator,
    InvalidSubstitute,
    InvalidType,
    MissingField,
    MissingKey,
    MissingLinefeed,
    MissingSeparator,
    UnexpectedKey,
    UnexpectedSchemaSeparator,
}

impl ParseError {
    fn description(self) -> &'static str {
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
            InvalidDefault => "invalid default",
            InvalidIndent => "invalid indent",
            InvalidSeparator => "invalid separator",
            InvalidSubstitute => "invalid substitute value",
            InvalidType => "invalid type",
            MissingField => "missing field",
            MissingKey => "missing key",
            MissingLinefeed => "missing line feed",
            MissingSeparator => "missing separator",
            UnexpectedKey => "unexpected key (not in schema)",
            UnexpectedSchemaSeparator => "unexpected schema separator",
        }
    }
}

impl From<ParseBoolError> for ParseError {
    fn from(_e: ParseBoolError) -> Self {
        ParseError::ExpectedBool
    }
}

impl Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(self.description())
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
    Utf8(Utf8Error),
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
        match self {
            Error::IO(e) => e.fmt(formatter),
            Error::Format(e) => e.fmt(formatter),
            Error::Utf8(e) => e.fmt(formatter),
            Error::FromUtf8(e) => e.fmt(formatter),
            Error::Serialize(msg) => formatter.write_str(msg),
            Error::Deserialize(msg) => formatter.write_str(msg),
            Error::UnsupportedType(msg) => formatter.write_str(msg),
            Error::InvalidKey => formatter.write_str("string keys only"),
            Error::FailedParse(e) => e.fmt(formatter),
        }
    }
}

impl std::error::Error for Error {}

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

impl From<Utf8Error> for Error {
    fn from(e: Utf8Error) -> Self {
        Error::Utf8(e)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(e: std::string::FromUtf8Error) -> Self {
        Error::FromUtf8(e)
    }
}
