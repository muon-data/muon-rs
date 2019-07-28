// common.rs
//
// Copyright (c) 2019  Douglas Lau
//

/// Key / value separator type
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Separator {
    /// Normal `: ` separator
    Normal,
    /// Text append `:>` separator
    TextAppend,
    /// Text value `:=` separator
    TextValue,
}

impl Separator {
    /// Get the separator as a string slice
    pub fn as_str(&self) -> &'static str {
        match self {
            Separator::Normal => ": ",
            Separator::TextAppend => ":>",
            Separator::TextValue => ":=",
        }
    }
}
