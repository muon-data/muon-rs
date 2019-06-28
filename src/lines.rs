// lines.rs
//
// Copyright (c) 2019  Douglas Lau
//

/// Iterator for lines
pub struct LineIter<'a> {
    input: &'a str,
}

/// Iterator for definitions
pub struct DefIter<'a> {
    /// Line iterator
    lines: LineIter<'a>,
    /// Number of spaces in one indent
    indent_spaces: Option<usize>,
    /// Current define (for append handling)
    define: Define<'a>,
}

/// Parse errors
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParseError {
    MissingKeyBeforeColon,
    MissingColon,
    MissingColonAfterQuote,
    MissingSpaceAfterColon,
    MissingLinefeed,
    InvalidSchemaSeparator,
    InvalidIndent,
}

/// Line Types
#[derive(Debug, PartialEq)]
pub enum Line<'a> {
    /// Invalid line (parse error)
    Invalid(ParseError, &'a str),
    /// Schema separator (:::)
    SchemaSeparator,
    /// Blank line
    Blank,
    /// Comment (starting with #)
    Comment(&'a str),
    /// Single colon definition (key: value)
    DefSingle(&'a str, &'a str),
    /// Double colon definition (key:: value)
    DefDouble(&'a str, &'a str),
}

/// Line parsing states
enum State {
    /// Error state
    Error(ParseError),
    /// Start state
    Start,
    /// Comment
    Comment,
    /// Key with no quoting
    KeyNotQuoted,
    /// Quoted key with even number of quotes (true means key not blank)
    KeyQuotedEven(bool),
    /// Quoted key with odd number of quotes (true means key not blank)
    KeyQuotedOdd(bool),
    /// Key with colon at byte offset
    KeyColon(usize),
    /// Key with double colon at byte offset
    KeyDoubleColon(usize),
    /// Definition with single colon at byte offset
    DefSingle(usize),
    /// Definition with double colon at byte offset
    DefDouble(usize),
}

/// Key/value definition
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Define<'a> {
    /// Invalid line or definition
    Invalid(ParseError, &'a str),
    /// Valid definition (indent count, key, value)
    Valid(usize, &'a str, &'a str),
}

impl State {
    /// Parse one character
    fn parse_char(self, i: usize, c: char) -> Self {
        use State::*;
        match self {
            Start => match c {
                ' ' => Start,
                '#' => Comment,
                ':' if i > 0 => KeyColon(i),
                ':' => Error(ParseError::MissingKeyBeforeColon),
                '"' => KeyQuotedOdd(false),
                _ => KeyNotQuoted,
            },
            KeyNotQuoted => match c {
                ':' => KeyColon(i),
                _ => KeyNotQuoted,
            },
            KeyQuotedOdd(b) => match c {
                '"' => KeyQuotedEven(b),
                _ => KeyQuotedOdd(true),
            },
            KeyQuotedEven(b) => match c {
                '"' => KeyQuotedOdd(true), // doubled quote
                ':' if b => KeyColon(i),
                _ => Error(ParseError::MissingColonAfterQuote),
            },
            KeyColon(off) => match c {
                ' ' => DefSingle(off),
                ':' => KeyDoubleColon(off),
                _ => Error(ParseError::MissingSpaceAfterColon),
            },
            KeyDoubleColon(off) => match c {
                ' ' => DefDouble(off),
                _ => Error(ParseError::MissingSpaceAfterColon),
            },
            _ => self,
        }
    }

    /// Check if line state is done
    fn is_done(&self) -> bool {
        use State::*;
        match self {
            Error(_) | Comment | DefSingle(_) | DefDouble(_) => true,
            _ => false,
        }
    }

    /// Convert state to a Line
    fn to_line<'a>(&self, line: &'a str) -> Line<'a> {
        use State::*;
        match self {
            Error(err) => Line::Invalid(*err, line),
            Comment => Line::Comment(line),
            DefSingle(off) => {
                let (key, value) = line.split_at(*off);
                let v = value.len().min(2); // colon and space
                let value = &value[v..];
                Line::DefSingle(key, value)
            }
            DefDouble(off) => {
                let (key, value) = line.split_at(*off);
                let v = value.len().min(3); // colons and space
                let value = &value[v..];
                Line::DefDouble(key, value)
            }
            _ => Line::Invalid(ParseError::MissingColon, line),
        }
    }
}

impl<'a> Line<'a> {
    /// Create line from input
    fn new(line: &'a str) -> Self {
        if line.len() == 0 {
            Line::Blank
        } else if line == ":::" {
            Line::SchemaSeparator
        } else {
            let mut state = State::Start;
            for (i, c) in line.char_indices() {
                state = state.parse_char(i, c);
                if state.is_done() {
                    return state.to_line(line);
                }
            }
            // Check for missing space after colon
            state.parse_char(line.len(), ' ').to_line(line)
        }
    }

    /// Get line indent, if any
    fn indent(&self) -> Option<usize> {
        if let Some(key) = self.key() {
            let i = key.chars().take_while(|c| *c == ' ').count();
            if i > 0 {
                Some(i)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get the key for a definition
    pub fn key(&self) -> Option<&'a str> {
        match self {
            Line::DefSingle(key, _) => Some(key),
            Line::DefDouble(key, _) => Some(key),
            _ => None,
        }
    }
}

impl<'a> LineIter<'a> {
    /// Create a new line iterator
    pub fn new(input: &'a str) -> Self {
        LineIter { input }
    }
}

impl<'a> Iterator for LineIter<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Should keys be allowed to contain linefeeds?
        if let Some(lf) = self.input.find('\n') {
            let (line, remaining) = self.input.split_at(lf);
            self.input = &remaining[1..]; // trim linefeed
            Some(Line::new(line))
        } else if self.input.len() > 0 {
            Some(Line::Invalid(ParseError::MissingLinefeed, self.input))
        } else {
            None
        }
    }
}

impl<'a> Define<'a> {
    /// Split a define for a list
    pub fn split_list(self) -> (Self, Option<Self>) {
        match self {
            Define::Valid(indent, key, value) => {
                let v: Vec<&str> = value.splitn(2, ' ').collect();
                if v.len() == 1 {
                    (self, None)
                } else {
                    (Define::Valid(indent, key, v[0]),
                     Some(Define::Valid(indent, key, v[1])))
                }
            }
            _ => (self, None),
        }
    }
}

impl<'a> DefIter<'a> {
    /// Create a new definition iterator
    pub fn new(lines: LineIter<'a>) -> Self {
        let indent_spaces = None;
        // this define is arbitrary
        let define = Define::Invalid(ParseError::InvalidIndent, "");
        DefIter {
            lines,
            indent_spaces,
            define,
        }
    }

    /// Set the indent spaces if needed
    fn set_indent_spaces(&mut self, line: &Line<'a>) {
        if self.indent_spaces == None {
            if let Some(indent) = line.indent() {
                self.indent_spaces = Some(indent);
            }
        }
    }

    /// Get the current key length (number of characters)
    fn key_len(&self) -> usize {
        match self.define {
            Define::Valid(indent, key, _) => {
                let i = indent * self.indent_spaces.unwrap_or(0);
                let k = key.chars().count();
                i + k
            }
            _ => 0,
        }
    }

    /// Get a define from key and value
    fn define(&self, key: &'a str, value: &'a str) -> Define<'a> {
        // Is key an "append" (all spaces)?
        if key.chars().all(|c| c == ' ') {
            if key.len() == self.key_len() {
                match self.define {
                    Define::Valid(indent, key, _) => {
                        Define::Valid(indent, key, value)
                    }
                    _ => Define::Invalid(ParseError::InvalidIndent, key),
                }
            } else {
                Define::Invalid(ParseError::InvalidIndent, key)
            }
        } else if let Some(indent) = self.indent_count(key) {
            let mut k = key;
            // trim leading spaces only (not all whitespace)
            while let Some(' ') = k.chars().next() {
                k = &k[1..];
            }
            Define::Valid(indent, k, value)
        } else {
            Define::Invalid(ParseError::InvalidIndent, key)
        }
    }

    /// Get indent count of a key
    fn indent_count(&self, key: &str) -> Option<usize> {
        let mut spaces = key.chars().take_while(|c| *c == ' ').count();
        let mut indent = 0;
        if let Some(indent_spaces) = self.indent_spaces {
            assert!(indent_spaces > 0);
            while spaces >= indent_spaces {
                spaces -= indent_spaces;
                indent += 1;
            }
        }
        if spaces == 0 {
            Some(indent)
        } else {
            // Invalid indent
            None
        }
    }
}

impl<'a> Iterator for DefIter<'a> {
    type Item = (Define<'a>, bool);

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(ln) = self.lines.next() {
            self.set_indent_spaces(&ln);
            match ln {
                Line::SchemaSeparator => {
                    return Some((Define::Invalid(
                        ParseError::InvalidSchemaSeparator,
                        ":::",
                    ), false))
                }
                Line::Invalid(err, line) => {
                    return Some((Define::Invalid(err, line), false))
                }
                Line::DefSingle(key, value) => {
                    self.define = self.define(key, value);
                    return Some((self.define, false));
                }
                Line::DefDouble(key, value) => {
                    self.define = self.define(key, value);
                    return Some((self.define, true));
                }
                _ => (),
            }
        }
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn valid_li() {
        let a = ":::\n# Comment\n:::\n\na: value a\nb:: value b\n";
        let mut li = LineIter::new(a);
        assert_eq!(li.next().unwrap(), Line::SchemaSeparator);
        assert_eq!(li.next().unwrap(), Line::Comment("# Comment"));
        assert_eq!(li.next().unwrap(), Line::SchemaSeparator);
        assert_eq!(li.next().unwrap(), Line::Blank);
        assert_eq!(li.next().unwrap(), Line::DefSingle("a", "value a"));
        assert_eq!(li.next().unwrap(), Line::DefDouble("b", "value b"));
    }

    #[test]
    fn invalid_li() {
        let a = ":value\nkey value\n\"key: value\"\na:value a\nb: value b";
        let mut li = LineIter::new(a);
        assert_eq!(
            li.next().unwrap(),
            Line::Invalid(ParseError::MissingKeyBeforeColon, ":value")
        );
        assert_eq!(
            li.next().unwrap(),
            Line::Invalid(ParseError::MissingColon, "key value")
        );
        assert_eq!(
            li.next().unwrap(),
            Line::Invalid(ParseError::MissingColonAfterQuote, "\"key: value\"")
        );
        assert_eq!(
            li.next().unwrap(),
            Line::Invalid(ParseError::MissingSpaceAfterColon, "a:value a")
        );
        assert_eq!(
            li.next().unwrap(),
            Line::Invalid(ParseError::MissingLinefeed, "b: value b")
        );
    }

    #[test]
    fn def_iter() {
        let a = ":::\na: value a\n# Comment\n\nb: value b\n\nc:\n : append\n  : bad\n";
        let li = LineIter::new(a);
        let mut di = DefIter::new(li);
        assert_eq!(
            di.next().unwrap(),
            (Define::Invalid(ParseError::InvalidSchemaSeparator, ":::"), false)
        );
        assert_eq!(di.next().unwrap(), (Define::Valid(0, "a", "value a"), false));
        assert_eq!(di.next().unwrap(), (Define::Valid(0, "b", "value b"), false));
        assert_eq!(di.next().unwrap(), (Define::Valid(0, "c", ""), false));
        assert_eq!(di.next().unwrap(), (Define::Valid(0, "c", "append"), false));
        assert_eq!(
            di.next().unwrap(),
            (Define::Invalid(ParseError::InvalidIndent, "  "), false)
        );
    }

    #[test]
    fn def_iter2() {
        let a =
            "a:\n  b: 1\n  c:: test\n  d:\n   x: bad\n    e: 5.5\n  f: -9\n";
        let li = LineIter::new(a);
        let mut di = DefIter::new(li);
        assert_eq!(di.next().unwrap(), (Define::Valid(0, "a", ""), false));
        assert_eq!(di.next().unwrap(), (Define::Valid(1, "b", "1"), false));
        assert_eq!(di.next().unwrap(), (Define::Valid(1, "c", "test"), true));
        assert_eq!(di.next().unwrap(), (Define::Valid(1, "d", ""), false));
        assert_eq!(
            di.next().unwrap(),
            (Define::Invalid(ParseError::InvalidIndent, "   x"), false)
        );
        assert_eq!(di.next().unwrap(), (Define::Valid(2, "e", "5.5"), false));
        assert_eq!(di.next().unwrap(), (Define::Valid(1, "f", "-9"), false));
    }
}
