// de.rs
//
// Copyright (c) 2019  Douglas Lau
//
use crate::error::{Error, Result};
use crate::parse::{self, Integer, Float};
use crate::lines::{DefIter, Define, LineIter};
use serde::de::{
    self, Deserialize, DeserializeSeed, MapAccess, SeqAccess, Visitor,
};
use std::iter::Peekable;

/// Dictionary for mapping stack
#[derive(Debug)]
struct Dict<'a> {
    /// Current key
    key: Option<&'a str>,
    /// List flag
    list: bool,
    /// Field names
    fields: &'static [&'static str],
    /// Flags for visited fields (same length as fields)
    visited: Vec<bool>,
    /// Flag to indicate finishing (cleaning up unvisited fields)
    finishing: bool,
}

/// Iterator for key/value mappings
struct MappingIter<'a> {
    /// Define iterator
    defs: Peekable<DefIter<'a>>,
    /// Stack of nested keys and list flags
    stack: Vec<Dict<'a>>,
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
    fn push_stack(&mut self, fields: &'static [&'static str]) {
        let visited = vec![false; fields.len()];
        let finishing = false;
        let d = Dict { key: None, list: false, fields, visited, finishing };
        self.stack.push(d);
    }

    /// Pop from the mapping stack
    fn pop_stack(&mut self) {
        self.stack.pop();
    }

    /// Set the current key on stack
    fn set_key(&mut self, key: Option<&'a str>) {
        if let Some(dict) = self.stack.last_mut() {
            dict.key = key;
            if let Some(f) = key {
                for i in 0..dict.fields.len() {
                    if dict.fields[i] == f {
                        dict.visited[i] = true
                    }
                }
            }
        }
    }

    /// Set the top of stack to a list
    fn set_list(&mut self, list: bool) {
        if let Some(dict) = self.stack.last_mut() {
            dict.list = list
        }
    }

    /// Check if the current define is a list
    fn is_list(&self) -> bool {
        if let Some(dict) = self.stack.last() {
            dict.list
        } else {
            false
        }
    }

    /// Get the next define in a list
    fn next_list(&mut self) -> Option<Define<'a>> {
        if let Some((define, double)) = self.define {
            if double {
                self.define = None;
                Some(define)
            } else {
                let (d0, d1) = define.split_list();
                self.define = match d1 {
                    Some(d) => Some((d, double)),
                    None => None,
                };
                Some(d0)
            }
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
        if let Some(dict) = self.stack.last() {
            if let Some(k) = dict.key {
                if let Some(Define::Valid(_, key, _)) = self.peek() {
                    return key == k
                }
            }
        }
        false
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
            None => Err(Error::UnexpectedEndOfInput),
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
            None => Err(Error::UnexpectedEndOfInput),
        }
    }

    /// Parse a text value
    fn parse_text(&mut self) -> Result<&'de str> {
        Ok(self.get_value()?)
    }

    /// Parse a char value
    fn parse_char(&mut self) -> Result<char> {
        let text = self.parse_text()?;
        if text.len() == 1 {
            if let Some(c) = text.chars().next() {
                return Ok(c)
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
        if let Some(v) = parse::int(value) {
            Ok(v)
        } else {
            Err(Error::FailedParse(format!("int: {}", value)))
        }
    }

    /// Parse a float value
    fn parse_float<T: Float>(&mut self) -> Result<T> {
        let value = self.get_value()?;
        if let Some(v) = parse::float(value) {
            Ok(v)
        } else {
            Err(Error::FailedParse(format!("float: {}", value)))
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
        visitor.visit_i8(self.parse_int()?)
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
        let val = self.parse_text()?;
        // FIXME: handle lists of strings
        if !self.mappings.is_list() && self.mappings.check_key() {
            let mut value = val.to_string();
            while self.mappings.check_key() {
                value.push('\n');
                value.push_str(self.parse_text()?);
            }
            visitor.visit_str(&value)
        } else {
            visitor.visit_borrowed_str(val)
        }
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
        Err(Error::UnsupportedType("bytes"))
    }

    fn deserialize_byte_buf<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        Err(Error::UnsupportedType("byte_buf"))
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(dict) = self.mappings.stack.last() {
            if dict.finishing {
                return visitor.visit_none()
            }
        }
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
        self.mappings.push_stack(&[]);
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
        // Get default value for struct
        if self.mappings.stack.len() > 0 {
            let _v = self.get_value()?;
            // FIXME: store default value somewhere
        }
        self.mappings.push_stack(fields);
        visitor.visit_map(self)
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
        Err(Error::UnsupportedType("enum"))
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        if let Some(dict) = self.mappings.stack.last_mut() {
            if dict.finishing {
                for i in 0..dict.fields.len() {
                    if !dict.visited[i] {
                        dict.visited[i] = true;
                        return visitor.visit_borrowed_str(dict.fields[i])
                    }
                }
            }
        }
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
            if let Some(dict) = self.mappings.stack.last_mut() {
                for i in 0..dict.fields.len() {
                    if !dict.visited[i] {
                        dict.finishing = true;
                        return seed.deserialize(&mut *self).map(Some)
                    }
                }
                dict.finishing = false;
            }
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
    fn integers() -> Result<(), Box<Error>> {
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
        ints: [i16; 3],
    }

    #[test]
    fn lists() -> Result<(), Box<Error>> {
        let b = "flags: false true true false\nvalues: Hello World\nints: 1 2 -5\n";
        let expected = B {
            flags: vec![false, true, true, false],
            values: vec!["Hello".to_string(), "World".to_string()],
            ints: [1, 2, -5],
        };
        assert_eq!(expected, from_str(b)?);
        let b = "flags: true true\nflags: false false\nints: 30 -25 0\n";
        let expected = B {
            flags: vec![true, true, false, false],
            values: vec![],
            ints: [30, -25, 0],
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
    fn floats() -> Result<(), Box<Error>> {
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

    #[derive(Deserialize, PartialEq, Debug)]
    struct D {
        struct_e: E,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct E {
        flag: bool,
    }

    #[test]
    fn nesting() -> Result<(), Box<Error>> {
        let d = "struct_e:\n  flag: false\n";
        let expected = D {
            struct_e: { E { flag: false } }
        };
        assert_eq!(expected, from_str(d)?);
        Ok(())
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct F {
        string: String,
    }

    #[test]
    fn string_append() -> Result<(), Box<Error>> {
        let f = "string: This is a long string\n      : for testing\n      : append definitions\n";
        let expected = F { string: "This is a long string\nfor testing\nappend definitions".to_string() };
        assert_eq!(expected, from_str(f)?);
        Ok(())
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct G {
        strings: Vec<String>,
    }

    #[test]
    fn string_list() -> Result<(), Box<Error>> {
        let g = "strings: one two\n       : three four\n       :: fifth item\n";
        let expected = G { strings: vec!["one".to_string(), "two".to_string(),
            "three".to_string(), "four".to_string(), "fifth item".to_string() ]};
        assert_eq!(expected, from_str(g)?);
        Ok(())
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct H {
        flag: Option<bool>,
        int: Option<i64>,
        float: Option<f32>,
    }

    #[test]
    fn options() -> Result<(), Box<Error>> {
        let h = "flag: false\n";
        let expected = H { flag: Some(false), int: None, float: None };
        assert_eq!(expected, from_str(h)?);
        let h = "int: 0xfab\n";
        let expected = H { flag: None, int: Some(0xFAB), float: None };
        assert_eq!(expected, from_str(h)?);
        let h = "float: -5e37\n";
        let expected = H { flag: None, int: None, float: Some(-5e37) };
        assert_eq!(expected, from_str(h)?);
        Ok(())
    }
}
