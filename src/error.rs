// error.rs
//
// Copyright (c) 2019  Douglas Lau
//
use serde::{de, ser};
//use std;
use std::error::Error as _;
use std::fmt::{self, Display};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Message(String),

    // Serializer errors
    InvalidKey,
    UnsupportedType,

    // Deserializer errors
    Eof,
    InvalidLine(String),
    MissingLineFeed,
    TrailingCharacters,

    ExpectedBoolean,
    ExpectedChar,
    ExpectedInteger,
    ExpectedString,
    ExpectedMap,
    ExpectedEnum,
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Self {
        Error::Message(msg.to_string())
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(std::error::Error::description(self))
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Message(ref msg) => msg,
            Error::InvalidKey => "invalid key: string keys only",
            Error::UnsupportedType => "unsupported type",
            Error::Eof => "unexpected end of input",
            _ => unimplemented!(),
        }
    }
}

impl From<fmt::Error> for Error {
    fn from(e: fmt::Error) -> Self {
        Error::Message(e.description().to_string())
    }
}
