// de.rs
//
// Copyright (c) 2019  Douglas Lau
//
use crate::error::{Error, Result};
use crate::intparse::{self, Integer};
use serde::de::{
    self, Deserialize, DeserializeSeed, MapAccess, SeqAccess, Visitor,
};

/// MuON deserializer
pub struct Deserializer<'de> {
    input: &'de str,
    line: Option<Line<'de>>,
}

impl<'de> Deserializer<'de> {
    fn from_str(input: &'de str) -> Self {
        Deserializer { input, line: None }
    }
}

/// Create a MuON deserializer from a string slice
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    if deserializer.input.is_empty() {
        Ok(t)
    } else {
        Err(Error::TrailingCharacters)
    }
}

/// Line Types
enum Line<'a> {
    /// Schema separator (:::)
    SchemaSeparator,
    /// Blank
    Blank,
    /// Comment (starting with #)
    Comment(&'a str),
    /// Definition (key: value)
    Definition(&'a str, &'a str),
    /// Colon Definition (key:: value)
    DefDouble(&'a str, &'a str),
}

/// Line parsing states
enum LineState {
    /// Start state
    Start,
    /// Invalid line
    Invalid,
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

impl LineState {
    /// Update line state with next character
    fn parse(&self, i: usize, c: char) -> Self {
        use LineState::*;
        match self {
            Start => match c {
                ' ' => Start,
                '#' => Comment,
                ':' if i > 0 => KeyColon(i),
                ':' => Invalid,
                '"' => KeyQuotedOdd(false),
                _ => KeyNotQuoted,
            },
            KeyNotQuoted => match c {
                ':' => KeyColon(i),
                _ => KeyNotQuoted,
            },
            KeyQuotedOdd(b) => match c {
                '"' => KeyQuotedEven(*b),
                _ => KeyQuotedOdd(true),
            },
            KeyQuotedEven(b) => match c {
                '"' => KeyQuotedOdd(true), // doubled quote
                ':' if *b => KeyColon(i),
                _ => Invalid,
            },
            KeyColon(k) => match c {
                ' ' => Definition(*k),
                ':' => KeyDoubleColon(*k),
                _ => Invalid,
            },
            KeyDoubleColon(k) => match c {
                ' ' => DefDouble(*k),
                _ => Invalid,
            },
            _ => Invalid,
        }
    }

    /// Check if line state is done
    fn done<'a>(&self, s: &'a str) -> Option<Result<Line<'a>>> {
        use LineState::*;
        match self {
            Invalid => Some(Err(Error::InvalidLine(s.to_string()))),
            Comment => Some(Ok(Line::Comment(s))),
            Definition(k) => {
                let (key, value) = s.split_at(*k);
                let v = value.len().min(2); // colon and space
                let value = &value[v..];
                Some(Ok(Line::Definition(key, value)))
            }
            DefDouble(k) => {
                let (key, value) = s.split_at(*k);
                let v = value.len().min(3); // colons and space
                let value = &value[v..];
                Some(Ok(Line::DefDouble(key, value)))
            }
            _ => None,
        }
    }
}

impl<'a> Line<'a> {
    /// Parse one line
    fn parse(s: &'a str) -> Result<Self> {
        if s.len() == 0 {
            Ok(Line::Blank)
        } else if s == ":::" {
            Ok(Line::SchemaSeparator)
        } else {
            let mut p = LineState::Start;
            for (i, c) in s.char_indices() {
                p = p.parse(i, c);
                if let Some(r) = p.done(s) {
                    return r;
                }
            }
            // Check missing space after colon
            if let Some(r) = p.parse(s.len(), ' ').done(s) {
                r
            } else {
                Err(Error::InvalidLine(s.to_string()))
            }
        }
    }
}

impl<'de> Deserializer<'de> {
    /// Get the next line
    fn next_line(&mut self) -> Result<Option<Line<'de>>> {
        if let Some(lf) = self.input.find('\n') {
            let (line, remaining) = self.input.split_at(lf);
            self.input = &remaining[1..];
            Ok(Some(Line::parse(line)?))
        } else if self.input.len() > 0 {
            Err(Error::TrailingCharacters)
        } else {
            Ok(None)
        }
    }

    /// Update to next definition
    fn next_def(&mut self) -> Result<Option<()>> {
        self.line = None;
        while let Some(line) = self.next_line()? {
            self.line = Some(line);
            if let Some(_) = self.definition() {
                return Ok(Some(()));
            }
        }
        Ok(None)
    }

    /// Get the current definition
    fn definition(&self) -> Option<&Line<'de>> {
        if let Some(line) = &self.line {
            match line {
                Line::Definition(_, _) | Line::DefDouble(_, _) => Some(line),
                _ => None,
            }
        } else {
            None
        }
    }

    /// Get the current key
    fn curr_key(&self) -> Result<&'de str> {
        if let Some(d) = self.definition() {
            match d {
                Line::Definition(k, _) => return Ok(k),
                Line::DefDouble(k, _) => return Ok(k),
                _ => (),
            }
        }
        Err(Error::Eof)
    }

    /// Get the current value
    fn curr_value(&self) -> Result<&'de str> {
        if let Some(d) = self.definition() {
            match d {
                Line::Definition(_, v) => return Ok(v),
                Line::DefDouble(_, v) => return Ok(v),
                _ => (),
            }
        }
        Err(Error::Eof)
    }

    fn parse_text(&mut self) -> Result<&'de str> {
        Ok(self.curr_value()?)
    }

    fn parse_char(&mut self) -> Result<char> {
        let text = self.parse_text()?;
        if text.len() == 1 {
            if let Some(c) = text.chars().next() {
                return Ok(c);
            }
        }
        Err(Error::ExpectedChar)
    }

    fn parse_bool(&mut self) -> Result<bool> {
        let value = self.curr_value()?;
        match value {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(Error::ExpectedBoolean),
        }
    }

    fn parse_int<T: Integer>(&mut self) -> Result<T> {
        let value = self.curr_value()?;
        if let Some(v) = intparse::from_str(value) {
            Ok(v)
        } else {
            Err(Error::ExpectedInteger)
        }
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // FIXME: use schema to know what types to return
        unimplemented!();
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_bool(self.parse_bool()?)
    }

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // i8 does not impl From<u8>, so use this as workaround
        let v: i16 = self.parse_int()?;
        visitor.visit_i8(v as i8)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i16(self.parse_int()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i32(self.parse_int()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.parse_int()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u8(self.parse_int()?)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u16(self.parse_int()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u32(self.parse_int()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(self.parse_int()?)
    }

    fn deserialize_f32<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_f64<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_char(self.parse_char()?)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // FIXME: if next line is an "append", build a temp String
        visitor.visit_borrowed_str(self.parse_text()?)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_bytes<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        unimplemented!()
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if self.input.starts_with("null") {
            self.input = &self.input["null".len()..];
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // FIXME: is this a regular list or a list of dicts?
        //        that needs to be in the state somehow...

        let value = visitor.visit_seq(&mut self)?;
        Ok(value)
    }

    fn deserialize_tuple<V>(self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        _len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // FIXME: make note of indent level
        let value = visitor.visit_map(&mut self)?;
        Ok(value)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_map(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        _visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::ExpectedEnum)
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.curr_key()?)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }
}

impl<'de> SeqAccess<'de> for Deserializer<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self).map(Some)
    }
}

impl<'de> MapAccess<'de> for Deserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if let Some(_) = self.next_def()? {
            seed.deserialize(&mut *self).map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self)
    }
}

#[cfg(test)]
mod test {
    use super::{from_str, Error};
    use serde_derive::Deserialize;

    #[derive(Deserialize, PartialEq, Debug)]
    struct A {
        b: bool,
        uint: u32,
        int: i32,
    }

    #[test]
    fn struct_a() -> Result<(), Box<Error>> {
        let a = "b: false\nuint: 7\nint: -5\n";
        let expected = A {
            b: false,
            uint: 7,
            int: -5,
        };
        assert_eq!(expected, from_str(a)?);
        Ok(())
    }
}
