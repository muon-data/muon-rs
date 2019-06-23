// lines.rs
//
// Copyright (c) 2019  Douglas Lau
//

/// Iterator for lines
pub struct LineIter<'a> {
    input: &'a str,
}

/// Parse errors
#[derive(Clone, Copy, Debug)]
pub enum ParseError {
    MissingKeyBeforeColon,
    MissingColon,
    MissingColonAfterQuote,
    MissingSpaceAfterColon,
    MissingLinefeed,
}

/// Line Types
pub enum Line<'a> {
    /// Invalid line (parse error)
    Invalid(ParseError, &'a str),
    /// Schema separator (:::)
    SchemaSeparator,
    /// Blank line
    Blank,
    /// Comment (starting with #)
    Comment(&'a str),
    /// Definition (key: value)
    Definition(&'a str, &'a str),
    /// Colon Definition (key:: value)
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
    /// Definition with colon at byte offset
    Definition(usize),
    /// Definition with double colon at byte offset
    DefDouble(usize),
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
                ' ' => Definition(off),
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
            Error(_) | Comment | Definition(_) | DefDouble(_) => true,
            _ => false,
        }
    }

    /// Convert state to a Line
    fn to_line<'a>(&self, line: &'a str) -> Line<'a> {
        use State::*;
        match self {
            Error(err) => Line::Invalid(*err, line),
            Comment => Line::Comment(line),
            Definition(off) => {
                let (key, value) = line.split_at(*off);
                let v = value.len().min(2); // colon and space
                let value = &value[v..];
                Line::Definition(key, value)
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
                    return state.to_line(line)
                }
            }
            // Check for missing space after colon
            state.parse_char(line.len(), ' ').to_line(line)
        }
    }

    /// Get the key for a definition
    pub fn key(&self) -> Option<&'a str> {
        match self {
            Line::Definition(key, _) => Some(key),
            Line::DefDouble(key, _) => Some(key),
            _ => None,
        }
    }

    /// Get the value for a definition
    pub fn value(&self) -> Option<&'a str> {
        match self {
            Line::Definition(_, value) => Some(value),
            Line::DefDouble(_, value) => Some(value),
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

#[cfg(test)]
mod test {
    use super::*;

}
