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

mod de;
mod error;
mod parse;
mod lines;
mod ser;

pub use de::{from_slice, from_str, from_reader, Deserializer};
pub use error::{Error, Result};
pub use ser::{to_string, to_vec, to_writer, Serializer};
