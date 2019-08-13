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
//! [`serde::Deserialize`]: https://docs.serde.rs/serde/trait.Deserialize.html
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
//! # use serde_derive::{Deserialize, Serialize};
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
//! [`serde::Serialize`]: https://docs.serde.rs/serde/trait.Serialize.html
//!
//! ### Example
//!
//! ```rust
//! # use serde_derive::Serialize;
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
//! <table>
//!   <tr>
//!     <th>MuON Type</th>
//!     <th>Rust Types</th>
//!   </tr>
//!   <tr>
//!     <td>text</td>
//!     <td><a href="https://doc.rust-lang.org/std/string/struct.String.html">
//!         String</a>
//!     </td>
//!   </tr>
//!   <tr>
//!     <td>bool</td>
//!     <td><a href="https://doc.rust-lang.org/std/primitive.bool.html">bool</a>
//!     </td>
//!   </tr>
//!   <tr>
//!     <td>int</td>
//!     <td><a href="https://doc.rust-lang.org/std/primitive.i8.html">i8</a>
//!        <a href="https://doc.rust-lang.org/std/primitive.i16.html">i16</a>
//!        <a href="https://doc.rust-lang.org/std/primitive.i32.html">i32</a>
//!        <a href="https://doc.rust-lang.org/std/primitive.i64.html">i64</a>
//!        <a href="https://doc.rust-lang.org/std/primitive.i128.html">i128</a>
//!        <a href="https://doc.rust-lang.org/std/primitive.u8.html">u8</a>
//!        <a href="https://doc.rust-lang.org/std/primitive.u16.html">u16</a>
//!        <a href="https://doc.rust-lang.org/std/primitive.u32.html">u32</a>
//!        <a href="https://doc.rust-lang.org/std/primitive.u64.html">u64</a>
//!        <a href="https://doc.rust-lang.org/std/primitive.u128.html">u128</a>
//!     </td>
//!   </tr>
//!   <tr>
//!     <td>number</td>
//!     <td><a href="https://doc.rust-lang.org/std/primitive.f32.html">f32</a>
//!        <a href="https://doc.rust-lang.org/std/primitive.f64.html">f64</a>
//!     </td>
//!   </tr>
//!   <tr>
//!     <td>datetime</td>
//!     <td><a href="struct.DateTime.html">DateTime</a></td>
//!   </tr>
//!   <tr>
//!     <td>date</td>
//!     <td><a href="struct.Date.html">Date</a></td>
//!   </tr>
//!   <tr>
//!     <td>time</td>
//!     <td><a href="struct.Time.html">Time</a></td>
//!   </tr>
//!   <tr>
//!     <td>record</td>
//!     <td>struct implementing
//!         <a href="https://docs.serde.rs/serde/trait.Deserialize.html">
//!         Deserialize</a>
//!     </td>
//!   </tr>
//!   <tr>
//!     <td>dictionary</td>
//!     <td><a href="https://doc.rust-lang.org/std/collections/struct.HashMap.html">
//!         HashMap</a>
//!     </td>
//!   </tr>
//!   <tr>
//!     <td>any</td>
//!     <td><a href="enum.Value.html">Value</a></td>
//!   </tr>
//! </table>
//!
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
pub use schema::Value;
pub use ser::{to_string, to_vec, to_writer, Serializer};
