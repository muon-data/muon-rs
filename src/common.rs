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

/// Key / value definition
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Define<'a> {
    /// Indent count
    pub indent: usize,
    /// Key for definition
    pub key: &'a str,
    /// Key / value separator
    pub separator: Separator,
    /// Value for definition
    pub value: &'a str,
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

impl<'a> Define<'a> {
    /// Create a new definition
    pub fn new(
        indent: usize,
        key: &'a str,
        separator: Separator,
        value: &'a str,
    ) -> Self {
        Define {
            indent,
            key,
            separator,
            value,
        }
    }

    /// Split a definition for a list
    pub fn split_list(self) -> (Self, Option<Self>) {
        let v: Vec<&str> = self.value.splitn(2, ' ').collect();
        if v.len() == 1 {
            (self, None)
        } else {
            (
                Define::new(self.indent, self.key, self.separator, v[0]),
                Some(Define::new(self.indent, self.key, self.separator, v[1])),
            )
        }
    }

    /// Check indent nesting
    pub fn check_indent(&self, indent: usize) -> bool {
        indent == self.indent + 1
    }
}
