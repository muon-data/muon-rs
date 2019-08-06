// lib.rs      muon crate.
//
// Copyright (c) 2019  Douglas Lau
//
#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! ## muon-rs
//!
//! A Rust library for the [MuON](https://github.com/muon-data/muon) data
//! format, using [serde](https://serde.rs).

mod common;
mod de;
mod error;
mod lines;
mod parse;
mod schema;
mod ser;

pub use de::{from_reader, from_slice, from_str, Deserializer};
pub use error::{Error, Result};
pub use ser::{to_string, to_vec, to_writer, Serializer};
