// de.rs
//
// Copyright (c) 2019  Douglas Lau
//
use crate::error::{Error, Result};
use crate::intparse::{self, Integer};
use crate::lines::{DefIter, Define, LineIter};
use lexical::FromLexical;
use serde::de::{
    self, Deserialize, DeserializeSeed, MapAccess, SeqAccess, Visitor,
};
use std::iter::Peekable;

/// Iterator for key/value mappings
struct MappingIter<'a> {
    /// Define iterator
    defs: Peekable<DefIter<'a>>,
    /// Stack of nested keys and list flags
    stack: Vec<(Option<&'a str>, bool)>,
    /// Current define (for list handling)
    define: Option<(Define<'a>, bool)>,
}

impl<'a> Iterator for MappingIter<'a> {
    type Item = Define<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.define.is_none() {
            self.define = self.defs.next();
        }
        if self.is_list() {
            self.next_list()
        } else if let Some((define, _)) = self.define {
            self.define = None;
            Some(define)
        } else {
            None
        }
    }
}

impl<'a> MappingIter<'a> {
    /// Create a new key/value mapping iterator
    fn new(iter: LineIter<'a>) -> Self {
        let defs = DefIter::new(iter).peekable();
        let stack = vec![];
        let define = None;
        MappingIter { defs, stack, define }
    }

    /// Peek at next define
    fn peek(&mut self) -> Option<Define<'a>> {
        if self.define.is_none() {
            self.define = self.defs.next();
        }
        if let Some((define, _)) = self.define {
            Some(define)
        } else {
            None
        }
    }

    /// Push onto the mapping stack
    fn push_stack(&mut self) {
        self.stack.push((None, false));
    }

    /// Pop from the mapping stack
    fn pop_stack(&mut self) {
        self.stack.pop();
    }

    /// Set the current key on stack
    fn set_key(&mut self, key: Option<&'a str>) {
        if let Some((_, list)) = self.stack.pop() {
            self.stack.push((key, list));
        }
    }

    /// Set the top of stack to a list
    fn set_list(&mut self, list: bool) {
        if let Some((key, _)) = self.stack.pop() {
            self.stack.push((key, list));
        }
    }

    /// Check if the current define is a list
    fn is_list(&self) -> bool {
        let ln = self.stack.len();
        if ln > 0 {
            let (_, list) = self.stack[ln - 1];
            list
        } else {
            false
        }
    }

    /// Get the next define in a list
    fn next_list(&mut self) -> Option<Define<'a>> {
        if let Some((define, double)) = self.define {
            let (d0, d1) = define.split_list();
            self.define = match d1 {
                Some(d) => Some((d, double)),
                None => None,
            };
            Some(d0)
        } else {
            None
        }
    }

    /// Check indent nesting
    fn check_indent(&mut self) -> bool {
        if let Some(Define::Valid(indent, _, _)) = self.peek() {
            self.stack.len() == indent + 1
        } else {
            false
        }
    }

    /// Check that key matches
    fn check_key(&mut self) -> bool {
        let ln = self.stack.len();
        if ln > 0 {
            if let (Some(k), _) = self.stack[ln - 1] {
                if let Some(Define::Valid(_, key, _)) = self.peek() {
                    return key == k;
                }
            }
        }
        true
    }
}

/// MuON deserializer
pub struct Deserializer<'de> {
    mappings: MappingIter<'de>,
}

impl<'de> Deserializer<'de> {
    fn from_str(input: &'de str) -> Self {
        let mappings = MappingIter::new(LineIter::new(input));
        Deserializer { mappings }
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
    /// Peek at current define
    fn peek_define(&mut self) -> Result<Define<'de>> {
        match self.mappings.peek() {
            Some(Define::Invalid(e, ln)) => {
                Err(Error::FailedParse(format!("{:?} {}", e, ln)))
            }
            Some(define) => Ok(define),
            None => Err(Error::Eof),
        }
    }

    /// Peek the current key
    fn peek_key(&mut self) -> Result<&'de str> {
        match self.peek_define()? {
            Define::Valid(_, k, _) => Ok(k),
            _ => unreachable!(),
        }
    }

    /// Get the current value
    fn get_value(&mut self) -> Result<&'de str> {
        match self.mappings.next() {
            Some(Define::Invalid(e, ln)) => {
                Err(Error::FailedParse(format!("{:?} {}", e, ln)))
            }
            Some(Define::Valid(_, _, v)) => Ok(v),
            None => Err(Error::Eof),
        }
    }

    /// Parse a text value
    fn parse_text(&mut self) -> Result<&'de str> {
        // FIXME: if next line is an "append", build a temp String
        Ok(self.get_value()?)
    }

    /// Parse a char value
    fn parse_char(&mut self) -> Result<char> {
        let text = self.parse_text()?;
        if text.len() == 1 {
            if let Some(c) = text.chars().next() {
                return Ok(c);
            }
        }
        Err(Error::FailedParse(format!("char: {}", text)))
    }

    /// Parse a bool value
    fn parse_bool(&mut self) -> Result<bool> {
        let value = self.get_value()?;
        match value {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(Error::FailedParse(format!("bool: {}", value))),
        }
    }

    /// Parse an int value
    fn parse_int<T: Integer>(&mut self) -> Result<T> {
        let value = self.get_value()?;
        if let Some(v) = intparse::from_str(value) {
            Ok(v)
        } else {
            Err(Error::FailedParse(format!("int: {}", value)))
        }
    }

    /// Parse a float value
    fn parse_float<T: FromLexical>(&mut self) -> Result<T> {
        let value = self.get_value()?;
        let res = lexical::try_parse(value);
        match res {
            Ok(v) => Ok(v),
            Err(e) => parse_filtered_float(value, e),
        }
    }
}

/// Parse a float after filtering out underscores
fn parse_filtered_float<T: FromLexical>(value: &str, e: lexical::Error)
    -> Result<T>
{
    if let lexical::ErrorKind::InvalidDigit(_) = e.kind() {
        if let Ok(v) = lexical::try_parse(filter_float(value)) {
            return Ok(v)
        }
    }
    Err(Error::FailedParse(format!("float: {}", value)))
}

/// Filter a float, removing valid underscores
fn filter_float(value: &str) -> String {
    let mut val = String::with_capacity(value.len());
    for v in value.split('_') {
        let before = val.as_bytes();
        let len = before.len();
        // Check character before underscore is a decimal digit
        if len > 0 && !char::from(before[len-1]).is_digit(10) {
            val.push('_')
        }
        let after = v.as_bytes();
        let vlen = v.len();
        // Allow starting with decimal point
        if len == 0 && vlen > 0 && char::from(after[0]) == '.' {
            val.push('0')
        }
        // Check character after underscore is a decimal digit
        if vlen == 0 || !char::from(after[0]).is_digit(10) {
            val.push('_')
        }
        val.push_str(v);
    }
    val
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

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_any(visitor)
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

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f32(self.parse_float()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.parse_float()?)
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
        self.mappings.set_list(true);
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
        self.mappings.push_stack();
        visitor.visit_map(self)
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
        unimplemented!()
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        let key = self.peek_key()?;
        self.mappings.set_key(Some(key));
        visitor.visit_borrowed_str(key)
    }
}

impl<'de> SeqAccess<'de> for Deserializer<'de> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: DeserializeSeed<'de>,
    {
        if self.mappings.check_indent() && self.mappings.check_key() {
            seed.deserialize(&mut *self).map(Some)
        } else {
            self.mappings.set_list(false);
            self.mappings.set_key(None);
            Ok(None)
        }
    }
}

impl<'de> MapAccess<'de> for Deserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: DeserializeSeed<'de>,
    {
        if self.mappings.check_indent() {
            seed.deserialize(&mut *self).map(Some)
        } else {
            self.mappings.pop_stack();
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
        let a = "b: true\nuint: 0xF00D\nint: 0b1111_0000_1111\n";
        let expected = A {
            b: true,
            uint: 0xF00D,
            int: 0xF0F,
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

    #[derive(Deserialize, PartialEq, Debug)]
    struct C {
        float: f32,
        double: f64,
    }

    #[test]
    fn struct_c() -> Result<(), Box<Error>> {
        let c = "float: +3.14159\ndouble: -123.456789e0\n";
        let expected = C {
            float: 3.14159,
            double: -123.456789,
        };
        assert_eq!(expected, from_str(c)?);
        let c = "float: 1e15\ndouble: inf\n";
        let expected = C {
            float: 1e15,
            double: std::f64::INFINITY,
        };
        assert_eq!(expected, from_str(c)?);
        let c = "float: 8_765.432_1\ndouble: -inf\n";
        let expected = C {
            float: 8_765.432_1,
            double: std::f64::NEG_INFINITY,
        };
        assert_eq!(expected, from_str(c)?);
        let c = "float: 123_.456\ndouble: 1.0\n";
        assert!(from_str::<C>(c).is_err());
        let c = "float: _123.456\ndouble: 1.0\n";
        assert!(from_str::<C>(c).is_err());
        let c = "float: 123.456_\ndouble: 1.0\n";
        assert!(from_str::<C>(c).is_err());
        let c = "float: 123.456\ndouble: 1__0.0\n";
        assert!(from_str::<C>(c).is_err());
        let c = "float: .123_456\ndouble: 1.0\n";
        assert!(from_str::<C>(c).is_err());
        Ok(())
    }
}
