// error.rs
//
// Copyright (c) 2019  Douglas Lau
//
use std::error::Error as _;
use std::fmt::{self, Display};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Format(fmt::Error),
    Serialize(String),
    Deserialize(String),
    UnsupportedType(&'static str),
    InvalidKey,
    UnexpectedEndOfInput,
    FailedParse(String),
}

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
            Error::Format(ref e) => e.description(),
            Error::Serialize(ref msg) => msg,
            Error::Deserialize(ref msg) => msg,
            Error::UnsupportedType(ref msg) => msg,
            Error::InvalidKey => "string keys only",
            Error::UnexpectedEndOfInput => "unexpected end of input",
            Error::FailedParse(ref msg) => msg,
        }
    }
}

impl From<fmt::Error> for Error {
    fn from(e: fmt::Error) -> Self {
        Error::Format(e)
    }
}
