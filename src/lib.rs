// lib.rs      muon crate.
//
// Copyright (c) 2019  Douglas Lau
//
#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! # MuON
//!
//! Serialize / deserialize [MuON](https://github.com/muon-data/muon.git)
//! documents in Rust.

mod common;
mod de;
mod error;
mod lines;
mod parse;
mod ser;

pub use de::{from_reader, from_slice, from_str, Deserializer};
pub use error::{Error, Result};
pub use ser::{to_string, to_vec, to_writer, Serializer};
