// lib.rs      muon crate.
//
// Copyright (c) 2019  Douglas Lau
//
#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! # muon-rs
//!
//! A Rust library for the [MuON](https://github.com/muon-data/muon) data
//! format, using [serde](https://serde.rs).
//!
//! ## Deserializing
//!
//! The easiest way to deserialize data is to derive [`serde::Deserialize`] on
//! a struct.  Then use one of the [`from_`](index.html#functions) functions.
//!
//! ### Example
//!
//! MuON file:
//! ```muon
//! book: Pale Fire
//!   author: Vladimir Nabokov
//!   year: 1962
//!   character: John Shade
//!     location: New Wye
//!   character: Charles Kinbote
//!     location: Zembla
//! book: The Curious Incident of the Dog in the Night-Time
//!   author: Mark Haddon
//!   year: 2003
//!   character: Christopher Boone
//!     location: Swindon
//!   character: Siobhan
//! ```
//!
//! Rust code:
//! ```rust
//! # use serde::{Deserialize, Serialize};
//! # use std::fs::File;
//! #[derive(Debug, Deserialize, Serialize)]
//! struct BookList {
//!     book: Vec<Book>,
//! }
//!
//! #[derive(Debug, Deserialize, Serialize)]
//! struct Book {
//!     title: String,
//!     author: String,
//!     year: Option<i16>,
//!     character: Vec<Character>,
//! }
//!
//! #[derive(Debug, Deserialize, Serialize)]
//! struct Character {
//!     name: String,
//!     location: Option<String>,
//! }
//!
//! # fn main() -> Result<(), muon_rs::Error> {
//! let muon = File::open("tests/books.muon")?;
//! let books: BookList = muon_rs::from_reader(muon)?;
//! println!("{:?}", books);
//! # Ok(())
//! # }
//! ```
//!
//! ## Serializing
//!
//! Deriving [`serde::Serialize`] on a struct is just as easy.  The
//! [`to_`](index.html#functions) functions are used to serialize MuON data.
//!
//! ### Example
//!
//! ```rust
//! # use serde::Serialize;
//! # use std::fs::File;
//! # #[derive(Debug, Serialize)]
//! # struct BookList {
//! #     book: Vec<Book>,
//! # }
//! # #[derive(Debug, Serialize)]
//! # struct Book {
//! #     title: String,
//! #     author: String,
//! #     year: Option<i16>,
//! #     character: Vec<Character>,
//! # }
//! # #[derive(Debug, Serialize)]
//! # struct Character {
//! #     name: String,
//! #     location: Option<String>,
//! # }
//! # fn main() -> Result<(), muon_rs::Error> {
//! let books = BookList {
//!     book: vec![
//!         Book {
//!             title: "Flight".to_string(),
//!             author: "Sherman Alexie".to_string(),
//!             year: Some(2007),
//!             character: vec![
//!                 Character {
//!                     name: "Zits".to_string(),
//!                     location: Some("Seattle".to_string()),
//!                 },
//!                 Character {
//!                     name: "Justice".to_string(),
//!                     location: None,
//!                 },
//!             ],
//!         },
//!     ],
//! };
//! let muon = muon_rs::to_string(&books)?;
//! println!("{:?}", muon);
//! # Ok(())
//! # }
//! ```
//!
//! ## Types
//!
//! MuON types can be mapped to different Rust types.
//!
//! | MuON Type      | Rust Types                                              |
//! |----------------|---------------------------------------------------------|
//! | `text`         | [`String`]                                              |
//! | `text <=1 >=1` | [`char`]                                                |
//! | `bool`         | [`bool`]                                                |
//! | `int`          | [`i8`] [`i16`] [`i32`] [`i64`] [`i128`] [`isize`] [`u8`] [`u16`] [`u32`] [`u64`] [`u128`] [`usize`] |
//! | `number`       | [`f32`] [`f64`]                                         |
//! | `datetime`     | [`DateTime`]                                            |
//! | `date`         | [`Date`]                                                |
//! | `time`         | [`Time`]                                                |
//! | `record`       | struct implementing [`Deserialize`](serde::Deserialize) |
//! | `dictionary`   | [`HashMap`] [`BTreeMap`]                                |
//! | `any`          | [`Value`]                                               |
//!
//! [`HashMap`]: std::collections::HashMap
//! [`BTreeMap`]: std::collections::BTreeMap

mod common;
mod datetime;
mod de;
mod error;
mod lines;
mod parse;
mod schema;
mod ser;

pub use datetime::{Date, DateTime, Time, TimeOffset};
pub use de::{from_reader, from_slice, from_str, Deserializer};
pub use error::{Error, Result};
pub use schema::{IntValue, NumValue, Value};
pub use ser::{to_string, to_vec, to_writer, Serializer};
