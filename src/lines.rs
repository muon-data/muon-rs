// lines.rs
//
// Copyright (c) 2019  Douglas Lau
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
pub enum Line<'a> {
    /// Schema separator (:::)
    SchemaSeparator,
    /// Blank line
    Blank,
    /// Comment (starting with #)
    Comment(&'a str),
    /// Definition (key, separator, value)
    Definition(&'a str, Separator, &'a str),
}

/// Iterator for lines
///
/// If a parsing error happens, the [next](struct.LineIter.html#method.next)
/// method will return `None`.  Use [error](struct.LineIter.html#method.error)
/// to check for this.
pub struct LineIter<'a> {
    /// Input string
    input: &'a str,
    /// Parsing error
    error: Option<ParseError>,
}

/// Iterator for definitions
///
/// If a parsing error happens, the [next](struct.DefIter.html#method.next)
/// method will return `None`.  Use [error](struct.DefIter.html#method.error)
/// to check for this.
pub struct DefIter<'a> {
    /// Line iterator
    lines: LineIter<'a>,
    /// Number of spaces in one indent
    indent_spaces: Option<usize>,
    /// Parsed schema
    schema: Option<Schema<'a>>,
    /// Current definition (for append handling)
    define: Option<Define<'a>>,
    /// Parsing error
    error: Option<ParseError>,
}

impl State {
    /// Parse one character
    ///
    /// `offset` Byte offset for character
    /// `c` Character to parse
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
        match self {
            Error(_) | Comment | DefDone(_, _) => true,
            _ => false,
        }
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
        if line.len() == 0 {
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

impl<'a> LineIter<'a> {
    /// Create a new line iterator
    pub fn new(input: &'a str) -> Self {
        LineIter { input, error: None }
    }

    /// Get most recent parse error
    pub fn error(&self) -> Option<ParseError> {
        self.error
    }
}

impl<'a> Iterator for LineIter<'a> {
    type Item = Line<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Should keys be allowed to contain linefeeds?
        if let Some(lf) = self.input.find('\n') {
            let (line, remaining) = self.input.split_at(lf);
            self.input = &remaining[1..]; // trim linefeed
            match Line::new(line) {
                Ok(line) => {
                    self.error = None;
                    Some(line)
                }
                Err(e) => {
                    self.error = Some(e);
                    None
                }
            }
        } else if self.input.len() > 0 {
            self.error = Some(ParseError::MissingLinefeed);
            None
        } else {
            self.error = None;
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

impl<'a> DefIter<'a> {
    /// Create a new definition iterator
    pub fn new(input: &'a str) -> Self {
        let lines = LineIter::new(input);
        let indent_spaces = None;
        let schema = None;
        let define = None;
        let error = None;
        DefIter {
            lines,
            indent_spaces,
            schema,
            define,
            error,
        }
    }

    /// Get schema
    #[allow(dead_code)]
    pub fn schema(&self) -> Option<&Schema> {
        match &self.schema {
            Some(schema) => Some(schema),
            None => None,
        }
    }

    /// Get most recent parse error
    pub fn error(&self) -> Option<ParseError> {
        self.error
    }

    /// Set the indent spaces if needed
    fn set_indent_spaces(&mut self, key: &'a str) -> Result<(), ParseError> {
        if self.indent_spaces == None {
            match key_indent(key) {
                // Only 2, 3 or 4 space indents are valid
                Some(sp) if sp >= 2 && sp <= 4 => {
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
    type Item = Define<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.error = None;
        while let Some(ln) = self.lines.next() {
            match self.process_line(ln) {
                Ok(None) => (),
                Ok(Some(define)) => {
                    self.define = Some(define);
                    return Some(define);
                }
                Err(e) => {
                    self.error = Some(e);
                    return None;
                }
            }
        }
        self.error = self.lines.error();
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
        assert_eq!(li.next().unwrap(), Line::SchemaSeparator);
        assert_eq!(li.next().unwrap(), Line::Comment("# Comment"));
        assert_eq!(li.next().unwrap(), Line::SchemaSeparator);
        assert_eq!(li.next().unwrap(), Line::Blank);
        assert_eq!(
            li.next().unwrap(),
            Line::Definition("a", Separator::Normal, "value a")
        );
        assert_eq!(
            li.next().unwrap(),
            Line::Definition("b", Separator::TextValue, "value b")
        );
        assert_eq!(
            li.next().unwrap(),
            Line::Definition("c", Separator::TextAppend, "value c")
        );
    }

    #[test]
    fn invalid_li() {
        let a = ":value\nkey value\n\"key: value\"\na:value a\nb: value b";
        let mut li = LineIter::new(a);
        assert_eq!(li.next(), None);
        assert_eq!(li.error(), Some(ParseError::MissingKey));
        assert_eq!(li.next(), None);
        assert_eq!(li.error(), Some(ParseError::MissingSeparator));
        assert_eq!(li.next(), None);
        assert_eq!(li.error(), Some(ParseError::InvalidSeparator));
        assert_eq!(li.next(), None);
        assert_eq!(li.error(), Some(ParseError::InvalidSeparator));
        assert_eq!(li.next(), None);
        assert_eq!(li.error(), Some(ParseError::MissingLinefeed));
        assert_eq!(li.next(), None);
    }

    #[test]
    fn def_iter() {
        let a = ":::\na: text\nb: text\nc: record\n  d: list bool\n:::\na: value a\n# Comment\n:::\n\nb: value b\n\nc:\n : append\n  : bad\n";
        let mut di = DefIter::new(a);
        let d = di.next();
        assert_ne!(d, None);
        assert_eq!(
            d.unwrap(),
            Define::new(0, "a", Separator::Normal, "value a")
        );
        assert_eq!(di.next(), None);
        assert_eq!(di.error(), Some(ParseError::UnexpectedSchemaSeparator));
        assert_eq!(
            di.next().unwrap(),
            Define::new(0, "b", Separator::Normal, "value b")
        );
        assert_eq!(
            di.next().unwrap(),
            Define::new(0, "c", Separator::Normal, "")
        );
        assert_eq!(
            di.next().unwrap(),
            Define::new(0, "c", Separator::Normal, "append")
        );
        assert_eq!(di.next(), None);
        assert_eq!(di.error(), Some(ParseError::InvalidIndent));
    }

    #[test]
    fn def_iter2() {
        let a =
            "a:\n  b: 1\n  cc:=this\n  c:>test\n  d:\n   x: bad\n    e: 5.5\n  f: -9\n";
        let mut di = DefIter::new(a);
        assert_eq!(
            di.next().unwrap(),
            Define::new(0, "a", Separator::Normal, "")
        );
        assert_eq!(
            di.next().unwrap(),
            Define::new(1, "b", Separator::Normal, "1")
        );
        assert_eq!(
            di.next().unwrap(),
            Define::new(1, "cc", Separator::TextValue, "this")
        );
        assert_eq!(
            di.next().unwrap(),
            Define::new(1, "c", Separator::TextAppend, "test")
        );
        assert_eq!(
            di.next().unwrap(),
            Define::new(1, "d", Separator::Normal, "")
        );
        assert_eq!(di.next(), None);
        assert_eq!(di.error(), Some(ParseError::InvalidIndent));
        assert_eq!(
            di.next().unwrap(),
            Define::new(2, "e", Separator::Normal, "5.5")
        );
        assert_eq!(
            di.next().unwrap(),
            Define::new(1, "f", Separator::Normal, "-9")
        );
    }
}
