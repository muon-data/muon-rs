// lines.rs
//
// Copyright (c) 2019-2020  Douglas Lau
//
use crate::common::{Define, Separator};
use crate::error::ParseError;
use crate::schema::Schema;

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
    /// Definition with separator at byte offset
    DefDone(usize, Separator),
}

/// Line Types
#[derive(Debug, PartialEq)]
pub(crate) enum Line<'a> {
    /// Schema separator (:::)
    SchemaSeparator,
    /// Blank line
    Blank,
    /// Comment (starting with #)
    Comment(&'a str),
    /// Definition (key, separator, value)
    Definition(&'a str, Separator, &'a str),
}

impl State {
    /// Parse one character
    ///
    ///  - `offset` Byte offset for character
    ///  - `c` Character to parse
    fn parse_char(self, offset: usize, c: char) -> Self {
        use State::*;
        match self {
            Start => match c {
                ' ' => Start,
                '#' => Comment,
                ':' if offset > 0 => KeyColon(offset),
                ':' => Error(ParseError::MissingKey),
                '"' => KeyQuotedOdd(false),
                _ => KeyNotQuoted,
            },
            KeyNotQuoted => match c {
                ':' => KeyColon(offset),
                _ => KeyNotQuoted,
            },
            KeyQuotedOdd(b) => match c {
                '"' => KeyQuotedEven(b),
                _ => KeyQuotedOdd(true),
            },
            KeyQuotedEven(b) => match c {
                '"' => KeyQuotedOdd(true), // doubled quote
                ':' if b => KeyColon(offset),
                _ => Error(ParseError::InvalidSeparator),
            },
            KeyColon(off) => match c {
                ' ' => DefDone(off, Separator::Normal),
                '>' => DefDone(off, Separator::TextAppend),
                '=' => DefDone(off, Separator::TextValue),
                _ => Error(ParseError::InvalidSeparator),
            },
            _ => self,
        }
    }

    /// Check if line state is done
    fn is_done(&self) -> bool {
        use State::*;
        matches!(self, Error(_) | Comment | DefDone(_, _))
    }

    /// Convert state to a Line
    fn to_line<'a>(&self, line: &'a str) -> Result<Line<'a>, ParseError> {
        use State::*;
        match self {
            Comment => Ok(Line::Comment(line)),
            DefDone(off, separator) => {
                let (key, value) = line.split_at(*off);
                let v = value.len().min(separator.as_str().len());
                let value = &value[v..];
                Ok(Line::Definition(key, *separator, value))
            }
            Error(e) => Err(*e),
            _ => Err(ParseError::MissingSeparator),
        }
    }
}

impl<'a> Line<'a> {
    /// Create line from input
    fn new(line: &'a str) -> Result<Self, ParseError> {
        if line.is_empty() {
            Ok(Line::Blank)
        } else if line == ":::" {
            Ok(Line::SchemaSeparator)
        } else {
            let mut state = State::Start;
            for (offset, c) in line.char_indices() {
                state = state.parse_char(offset, c);
                if state.is_done() {
                    return state.to_line(line);
                }
            }
            // Check for missing space after colon
            state.parse_char(line.len(), ' ').to_line(line)
        }
    }
}

/// Iterator over lines
pub(crate) struct LineIter<'a> {
    /// Input string
    input: &'a str,
}

impl<'a> LineIter<'a> {
    /// Create a new line iterator
    pub(crate) fn new(input: &'a str) -> Self {
        LineIter { input }
    }
}

impl<'a> Iterator for LineIter<'a> {
    type Item = Result<Line<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        // Should keys be allowed to contain linefeeds?
        if let Some(lf) = self.input.find('\n') {
            let (line, remaining) = self.input.split_at(lf);
            self.input = &remaining[1..]; // trim linefeed
            Some(Line::new(line))
        } else if !self.input.is_empty() {
            Some(Err(ParseError::MissingLinefeed))
        } else {
            None
        }
    }
}

/// Get key indent, if any
fn key_indent(key: &str) -> Option<usize> {
    let i = key.chars().take_while(|c| *c == ' ').count();
    if i > 0 && i < key.chars().count() {
        Some(i)
    } else {
        None
    }
}

/// Iterator for definitions
///
/// If a parsing error happens, the [`DefIter::next()`] method will return
/// `None`.  Use [`DefIter::error()`] to check for this.
pub(crate) struct DefIter<'a> {
    /// Line iterator
    lines: LineIter<'a>,
    /// Number of spaces in one indent
    indent_spaces: Option<usize>,
    /// Parsed schema
    schema: Option<Schema<'a>>,
    /// Current definition (for append handling)
    define: Option<Define<'a>>,
}

impl<'a> DefIter<'a> {
    /// Create a new definition iterator
    pub(crate) fn new(input: &'a str) -> Self {
        let lines = LineIter::new(input);
        let indent_spaces = None;
        let schema = None;
        let define = None;
        DefIter {
            lines,
            indent_spaces,
            schema,
            define,
        }
    }

    /// Get schema
    #[allow(dead_code)]
    pub(crate) fn schema(&self) -> Option<&Schema> {
        self.schema.as_ref()
    }

    /// Set the indent spaces if needed
    fn set_indent_spaces(&mut self, key: &'a str) -> Result<(), ParseError> {
        if self.indent_spaces.is_none() {
            match key_indent(key) {
                // Only 2, 3 or 4 space indents are valid
                Some(sp) if (2..=4).contains(&sp) => {
                    self.indent_spaces = Some(sp);
                    Ok(())
                }
                Some(_) => Err(ParseError::InvalidIndent),
                None => Ok(()),
            }
        } else {
            Ok(())
        }
    }

    /// Get the current key length (number of characters)
    fn key_len(&self) -> usize {
        if let Some(define) = self.define {
            let i = define.indent * self.indent_spaces.unwrap_or(0);
            let k = define.key.chars().count();
            i + k
        } else {
            0
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

    /// Make a definition from key and value
    fn make_define(
        &self,
        key: &'a str,
        separator: Separator,
        value: &'a str,
    ) -> Result<Define<'a>, ParseError> {
        // is key blank? (all spaces)
        if key.chars().all(|c| c == ' ') {
            if key.len() == self.key_len() {
                if let Some(define) = self.define {
                    return Ok(Define::new(
                        define.indent,
                        define.key,
                        separator,
                        value,
                    ));
                }
            }
        } else if let Some(indent) = self.indent_count(key) {
            let mut k = key;
            // trim leading spaces only (not all whitespace)
            while let Some(' ') = k.chars().next() {
                k = &k[1..];
            }
            return Ok(Define::new(indent, k, separator, value));
        }
        Err(ParseError::InvalidIndent)
    }

    /// Process a define
    fn process_define(
        &mut self,
        key: &'a str,
        separator: Separator,
        value: &'a str,
    ) -> Result<Option<Define<'a>>, ParseError> {
        self.set_indent_spaces(key)?;
        let def = self.make_define(key, separator, value)?;
        if let (None, Some(schema)) = (&self.define, &mut self.schema) {
            if schema.add_define(def)? {
                return Ok(None);
            }
        }
        Ok(Some(def))
    }

    /// Process a schema separator
    fn process_schema(&mut self) -> Result<Option<Define<'a>>, ParseError> {
        match (&self.define, &mut self.schema) {
            (None, None) => {
                self.schema = Some(Schema::new());
                Ok(None)
            }
            (None, Some(schema)) => {
                if schema.finish() {
                    Err(ParseError::UnexpectedSchemaSeparator)
                } else {
                    Ok(None)
                }
            }
            (_, _) => Err(ParseError::UnexpectedSchemaSeparator),
        }
    }

    /// Process a line
    fn process_line(
        &mut self,
        ln: Line<'a>,
    ) -> Result<Option<Define<'a>>, ParseError> {
        match ln {
            Line::SchemaSeparator => self.process_schema(),
            Line::Blank | Line::Comment(_) => Ok(None),
            Line::Definition(key, separator, value) => {
                self.process_define(key, separator, value)
            }
        }
    }
}

impl<'a> Iterator for DefIter<'a> {
    type Item = Result<Define<'a>, ParseError>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(ln) = self.lines.next() {
            match ln.and_then(|ln| self.process_line(ln)) {
                Ok(None) => (),
                Ok(Some(define)) => {
                    self.define = Some(define);
                    return Some(Ok(define));
                }
                Err(e) => return Some(Err(e)),
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
        let a = ":::\n# Comment\n:::\n\na: value a\nb:=value b\nc:>value c\n";
        let mut li = LineIter::new(a);
        assert_eq!(li.next().unwrap(), Ok(Line::SchemaSeparator));
        assert_eq!(li.next().unwrap(), Ok(Line::Comment("# Comment")));
        assert_eq!(li.next().unwrap(), Ok(Line::SchemaSeparator));
        assert_eq!(li.next().unwrap(), Ok(Line::Blank));
        assert_eq!(
            li.next().unwrap(),
            Ok(Line::Definition("a", Separator::Normal, "value a")),
        );
        assert_eq!(
            li.next().unwrap(),
            Ok(Line::Definition("b", Separator::TextValue, "value b")),
        );
        assert_eq!(
            li.next().unwrap(),
            Ok(Line::Definition("c", Separator::TextAppend, "value c")),
        );
    }

    #[test]
    fn invalid_li() {
        let a = ":value\nkey value\n\"key: value\"\na:value a\nb: value b";
        let mut li = LineIter::new(a);
        assert_eq!(li.next(), Some(Err(ParseError::MissingKey)));
        assert_eq!(li.next(), Some(Err(ParseError::MissingSeparator)));
        assert_eq!(li.next(), Some(Err(ParseError::InvalidSeparator)));
        assert_eq!(li.next(), Some(Err(ParseError::InvalidSeparator)));
        assert_eq!(li.next(), Some(Err(ParseError::MissingLinefeed)));
        assert_eq!(li.next(), Some(Err(ParseError::MissingLinefeed)));
    }

    #[test]
    fn def_iter() {
        let a = ":::\na: text\nb: text\nc: record\n  d: list bool\n:::\na: value a\n# Comment\n:::\n\nb: value b\n\nc:\n : append\n  : bad\n";
        let mut di = DefIter::new(a);
        let d = di.next();
        assert_ne!(d, None);
        assert_eq!(
            d.unwrap(),
            Ok(Define::new(0, "a", Separator::Normal, "value a")),
        );
        assert_eq!(di.next(), Some(Err(ParseError::UnexpectedSchemaSeparator)));
        assert_eq!(
            di.next().unwrap(),
            Ok(Define::new(0, "b", Separator::Normal, "value b")),
        );
        assert_eq!(
            di.next().unwrap(),
            Ok(Define::new(0, "c", Separator::Normal, "")),
        );
        assert_eq!(
            di.next().unwrap(),
            Ok(Define::new(0, "c", Separator::Normal, "append")),
        );
        assert_eq!(di.next(), Some(Err(ParseError::InvalidIndent)));
    }

    #[test]
    fn def_iter2() {
        let a =
            "a:\n  b: 1\n  cc:=this\n  c:>test\n  d:\n   x: bad\n    e: 5.5\n  f: -9\n";
        let mut di = DefIter::new(a);
        assert_eq!(
            di.next().unwrap(),
            Ok(Define::new(0, "a", Separator::Normal, "")),
        );
        assert_eq!(
            di.next().unwrap(),
            Ok(Define::new(1, "b", Separator::Normal, "1")),
        );
        assert_eq!(
            di.next().unwrap(),
            Ok(Define::new(1, "cc", Separator::TextValue, "this")),
        );
        assert_eq!(
            di.next().unwrap(),
            Ok(Define::new(1, "c", Separator::TextAppend, "test")),
        );
        assert_eq!(
            di.next().unwrap(),
            Ok(Define::new(1, "d", Separator::Normal, "")),
        );
        assert_eq!(di.next(), Some(Err(ParseError::InvalidIndent)));
        assert_eq!(
            di.next().unwrap(),
            Ok(Define::new(2, "e", Separator::Normal, "5.5")),
        );
        assert_eq!(
            di.next().unwrap(),
            Ok(Define::new(1, "f", Separator::Normal, "-9")),
        );
    }
}
