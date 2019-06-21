// lib.rs      muon crate.
//
// Copyright (c) 2019  Douglas Lau
//

#![forbid(unsafe_code)]

mod de;
mod error;
mod intparse;
mod ser;

pub use de::{from_str, Deserializer};
pub use error::{Error, Result};
pub use ser::{to_string, Serializer};
