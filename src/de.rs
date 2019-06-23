// de.rs
//
// Copyright (c) 2019  Douglas Lau
//
use crate::error::{Error, Result};
use crate::intparse::{self, Integer};
use crate::lines::{Line, LineIter};
use serde::de::{
    self, Deserialize, DeserializeSeed, MapAccess, SeqAccess, Visitor,
};

/// MuON deserializer
pub struct Deserializer<'de> {
    iter: LineIter<'de>,
    line: Option<Line<'de>>,
    keys: Vec<&'static [&'static str]>,
}

impl<'de> Deserializer<'de> {
    fn from_str(input: &'de str) -> Self {
        let iter = LineIter::new(input);
        let keys = vec![];
        Deserializer { iter, line: None, keys }
    }
}

/// Create a MuON deserializer from a string slice
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    Ok(t)
}

impl<'de> Deserializer<'de> {

    /// Update to next definition
    fn next_def(&mut self) -> Result<Option<()>> {
        while let Some(line) = self.iter.next() {
            if let Line::Invalid(e, ln) = line {
                return Err(Error::ParsingError(format!("{:?}: {}", e, ln)));
            }
            if let Some(_) = line.key() {
                self.line = Some(line);
                return Ok(Some(()));
            }
        }
        Ok(None)
    }

    /// Get the current key
    fn curr_key(&self) -> Result<&'de str> {
        if let Some(line) = &self.line {
            if let Some(key) = line.key() {
                return Ok(key);
            }
        }
        Err(Error::Eof)
    }

    /// Get the current value
    fn curr_value(&self) -> Result<&'de str> {
        if let Some(line) = &self.line {
            if let Some(value) = line.value() {
                return Ok(value);
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
        // FIXME
        visitor.visit_some(self)
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

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // FIXME: is this a regular list or a list of dicts?
        //        that needs to be in the state somehow...

        visitor.visit_seq(self)
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

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        // FIXME: make note of indent level
        visitor.visit_map(self)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.keys.push(fields);
        let r = self.deserialize_map(visitor);
        self.keys.pop();
        r
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
        // FIXME: if no more elements in list return None
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

    #[derive(Deserialize, PartialEq, Debug)]
    struct B {
        flags: Vec<bool>,
        values: Vec<String>,
    }

    #[test]
    fn struct_b() -> Result<(), Box<Error>> {
        let b = "flags: false true true false\nvalues: Hello World\n";
        let expected = B {
            flags: vec![false, true, true, false],
            values: vec!["Hello".to_string(), "World".to_string()],
        };
        assert_eq!(expected, from_str(b)?);
        Ok(())
    }
}
