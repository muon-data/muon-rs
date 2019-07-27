// common.rs
//
// Copyright (c) 2019  Douglas Lau
//

/// Key / value separator type
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Separator {
    /// Single colon separator
    SingleColon,
    /// Double colon separator (non-breaking string)
    DoubleColon,
    /// Double colon append separator (non-breaking string)
    DoubleColonAppend,
}

impl Separator {
    /// Get the separator as a string slice
    pub fn as_str(&self) -> &'static str {
        match self {
            Separator::SingleColon => ": ",
            Separator::DoubleColon => "::",
            Separator::DoubleColonAppend => ":: ",
        }
    }
}
