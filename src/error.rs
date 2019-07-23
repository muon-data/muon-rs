// error.rs
//
// Copyright (c) 2019  Douglas Lau
//
use std::error::Error as _;
use std::fmt::{self, Display};
use std::io;
use std::str;

/// Errors which can occur when serializing and deserializing MuON data.
#[derive(Debug)]
pub enum Error {
    /// I/O errors
    IO(io::Error),
    /// Formatting error while serializing
    Format(fmt::Error),
    /// Invalid UTF-8 while deserializing
    Utf8(str::Utf8Error),
    /// Serializing error from serde
    Serialize(String),
    /// Deserializing error from serde
    Deserialize(String),
    /// Unsupported type error
    UnsupportedType(&'static str),
    /// Invalid key
    InvalidKey,
    /// Unexpected end of input while deserializing
    UnexpectedEndOfInput,
    /// Failed parse while deserializing
    FailedParse(String),
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
            Error::Serialize(ref msg) => msg,
            Error::Deserialize(ref msg) => msg,
            Error::UnsupportedType(ref msg) => msg,
            Error::InvalidKey => "string keys only",
            Error::UnexpectedEndOfInput => "unexpected end of input",
            Error::FailedParse(ref msg) => msg,
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
