// de.rs
//
// Copyright (c) 2019  Douglas Lau
//
use crate::common::Separator;
use crate::error::{Error, Result};
use crate::lines::{DefIter, Define, LineIter};
use crate::parse::{self, Float, Integer};
use serde::de::{
    self, Deserialize, DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess,
    Visitor,
};
use std::io::Read;
use std::str;

/// Dictionary mapping state
#[derive(Debug)]
enum DictState {
    /// Starting dict mapping
    Start,
    /// Visiting fields of mapping
    Visit,
    /// Clean up dict mapping
    Cleanup,
}

/// Dictionary for mapping stack
#[derive(Debug)]
struct Dict<'a> {
    /// Field names
    fields: &'static [&'static str],
    /// Flags for visited fields (same length as fields)
    visited: Vec<bool>,
    /// Dictionary mapping state
    state: DictState,
    /// Current key (should match one field)
    key: Option<&'a str>,
    /// List flag (applies to current key)
    list: bool,
}

impl<'a> Dict<'a> {
    /// Create a new Dict mapping
    fn new(is_root: bool, fields: &'static [&'static str]) -> Self {
        let visited = vec![false; fields.len()];
        // root dict does not have a Define, so doesn't need Start state
        let state = if is_root {
            DictState::Visit
        } else {
            DictState::Start
        };
        Dict {
            fields,
            visited,
            state,
            key: None,
            list: false,
        }
    }

    /// If in Start state, get first field
    fn start_first(&mut self) -> Option<&'a str> {
        let first = match (&self.state, self.fields.first()) {
            (DictState::Start, Some(field)) => Some(*field),
            _ => None,
        };
        self.state = DictState::Visit;
        first
    }

    /// Visit one field
    fn visit(&mut self, key: Option<&'a str>) {
        self.key = key;
        if let Some(f) = key {
            for i in 0..self.fields.len() {
                if self.fields[i] == f {
                    self.visited[i] = true
                }
            }
        }
    }

    /// Check for any unvisited fields
    fn has_unvisited(&self) -> bool {
        for i in 0..self.fields.len() {
            if !self.visited[i] {
                return true;
            }
        }
        false
    }

    /// Cleanup state for one field
    fn cleanup_visit(&mut self) -> Option<&'static str> {
        if let DictState::Cleanup = self.state {
            for i in 0..self.fields.len() {
                if !self.visited[i] {
                    self.visited[i] = true;
                    return Some(self.fields[i]);
                }
            }
        }
        None
    }
}

/// Iterator for key/value mappings
struct MappingIter<'a> {
    /// Define iterator
    defs: DefIter<'a>,
    /// Current define
    define: Option<Define<'a>>,
    /// Stack of nested dicts
    stack: Vec<Dict<'a>>,
}

impl<'a> Iterator for MappingIter<'a> {
    type Item = Define<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.define.is_none() {
            self.define = self.defs.next();
        }
        if self.is_list() {
            self.next_list()
        } else {
            self.define.take()
        }
    }
}

impl<'a> MappingIter<'a> {
    /// Create a new key/value mapping iterator
    fn new(iter: LineIter<'a>) -> Self {
        let defs = DefIter::new(iter);
        let define = None;
        let stack = vec![];
        MappingIter {
            defs,
            define,
            stack,
        }
    }

    /// Peek at next define
    fn peek(&mut self) -> Option<Define<'a>> {
        if self.define.is_none() {
            self.define = self.defs.next();
        }
        self.define
    }

    /// Get the next define in a list
    fn next_list(&mut self) -> Option<Define<'a>> {
        if let Some(define) = self.define {
            match define {
                Define::Valid(_, _, separator, _) => {
                    match separator {
                        Separator::DoubleColon => {
                            self.define = None;
                            Some(define)
                        }
                        Separator::DoubleColonAppend => {
                            // FIXME
                            self.define = None;
                            Some(define)
                        }
                        _ => {
                            let (d0, d1) = define.split_list();
                            self.define = d1;
                            Some(d0)
                        }
                    }
                }
                _ => Some(define),
            }
        } else {
            self.define
        }
    }

    /// Push onto the mapping stack
    fn push_stack(&mut self, fields: &'static [&'static str]) {
        let is_root = self.stack.len() == 0;
        self.stack.push(Dict::new(is_root, fields))
    }

    /// Pop from the mapping stack
    fn pop_stack(&mut self) {
        self.stack.pop();
    }

    /// Check if dict is in Start state
    fn check_start(&mut self) {
        let indent = self.stack.len();
        if let Some(dict) = self.stack.last_mut() {
            if let Some(key) = dict.start_first() {
                if let Some(Define::Valid(_, _, separator, v)) =
                    self.define.take()
                {
                    if v.len() > 0 && indent > 0 {
                        self.define =
                            Some(Define::Valid(indent - 1, key, separator, v))
                    }
                }
            }
        }
    }

    /// Check if cleanup is needed
    fn check_cleanup(&mut self) -> bool {
        if let Some(dict) = self.stack.last_mut() {
            dict.state = DictState::Cleanup;
            dict.has_unvisited()
        } else {
            false
        }
    }

    /// Set the current key on stack
    fn set_key(&mut self, key: Option<&'a str>) {
        if let Some(dict) = self.stack.last_mut() {
            dict.visit(key)
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
        match self.stack.last() {
            Some(dict) => dict.list,
            _ => false,
        }
    }

    /// Check indent nesting
    fn check_indent(&mut self) -> bool {
        if let Some(Define::Valid(indent, _, _, _)) = self.peek() {
            self.stack.len() == indent + 1
        } else {
            false
        }
    }

    /// Check that key matches
    fn check_key(&mut self) -> bool {
        if let Some(dict) = self.stack.last() {
            if let Some(k) = dict.key {
                if let Some(Define::Valid(_, key, _, _)) = self.peek() {
                    return key == k;
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

/// Deserialize a MuON document from a string slice
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    Ok(t)
}

/// Deserialize a MuON document from a byte slice
pub fn from_slice<'a, T>(v: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    from_str(str::from_utf8(v)?)
}

/// Deserialize a MuON document from a reader
pub fn from_reader<R, T>(mut reader: R) -> Result<T>
where
    R: Read,
    T: DeserializeOwned,
{
    let mut s = String::new();
    reader.read_to_string(&mut s)?;
    from_str(&s)
}

impl<'de> Deserializer<'de> {
    /// Parse a define into a result
    fn define_result(define: Option<Define>) -> Result<Define> {
        match define {
            Some(Define::Invalid(e, ln)) => {
                Err(Error::FailedParse(format!("{:?} {}", e, ln)))
            }
            Some(define) => Ok(define),
            None => Err(Error::UnexpectedEndOfInput),
        }
    }

    /// Peek the current key
    fn peek_key(&mut self) -> Result<&'de str> {
        match Deserializer::define_result(self.mappings.peek())? {
            Define::Valid(_, k, _, _) => Ok(k),
            _ => unreachable!(),
        }
    }

    /// Get the current value
    fn get_value(&mut self) -> Result<&'de str> {
        match Deserializer::define_result(self.mappings.next())? {
            Define::Valid(_, _, _, v) => Ok(v),
            _ => unreachable!(),
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
        Err(Error::UnsupportedType("Schema support not implemented"))
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
            if let DictState::Cleanup = dict.state {
                return visitor.visit_none();
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
            if let Some(field) = dict.cleanup_visit() {
                return visitor.visit_borrowed_str(field);
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
        self.mappings.check_start();
        if self.mappings.check_indent() || self.mappings.check_cleanup() {
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
        let b =
            "flags: false true true false\nvalues: Hello World\nints: 1 2 -5\n";
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
        struct_f: F,
        flag: bool,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct F {
        int: i64,
    }

    #[test]
    fn nesting() -> Result<(), Box<Error>> {
        let d = "struct_e:\n  struct_f:\n    int: 987_654_321\n  flag: false\n";
        let expected = D {
            struct_e: {
                E {
                    struct_f: F { int: 987654321 },
                    flag: false,
                }
            },
        };
        assert_eq!(expected, from_str(d)?);
        let d = "struct_e:\n  flag: true\n  struct_f:\n    int: -12_34_56\n";
        let expected = D {
            struct_e: {
                E {
                    struct_f: F { int: -123456 },
                    flag: true,
                }
            },
        };
        assert_eq!(expected, from_str(d)?);
        Ok(())
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct G {
        string: String,
    }

    #[test]
    fn string_append() -> Result<(), Box<Error>> {
        let g = "string: This is a long string\n      : for testing\n      : append definitions\n";
        let expected = G {
            string: "This is a long string\nfor testing\nappend definitions"
                .to_string(),
        };
        assert_eq!(expected, from_str(g)?);
        Ok(())
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct H {
        strings: Vec<String>,
    }

    #[test]
    fn string_list() -> Result<(), Box<Error>> {
        let h = "strings: one two\n       : three four\n       :: fifth item\n       : sixth\n";
        let expected = H {
            strings: vec![
                "one".to_string(),
                "two".to_string(),
                "three".to_string(),
                "four".to_string(),
                "fifth item".to_string(),
                "sixth".to_string(),
            ],
        };
        assert_eq!(expected, from_str(h)?);
        Ok(())
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct I {
        flag: Option<bool>,
        int: Option<i64>,
        float: Option<f32>,
    }

    #[test]
    fn options() -> Result<(), Box<Error>> {
        let i = "flag: false\n";
        let expected = I {
            flag: Some(false),
            int: None,
            float: None,
        };
        assert_eq!(expected, from_str(i)?);
        let i = "int: 0xfab\n";
        let expected = I {
            flag: None,
            int: Some(0xFAB),
            float: None,
        };
        assert_eq!(expected, from_str(i)?);
        let i = "float: -5e37\n";
        let expected = I {
            flag: None,
            int: None,
            float: Some(-5e37),
        };
        assert_eq!(expected, from_str(i)?);
        Ok(())
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct J {
        person: Vec<K>,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct K {
        name: String,
        score: i32,
    }

    #[test]
    fn dict_list() -> Result<(), Box<Error>> {
        let j = "person:\n   name: Genghis Khan\n   score: 500\nperson:\n   name: Josef Stalin\n   score: 250\nperson:\n   name: Dudley Do-Right\n   score: 800\n";
        let expected = J {
            person: vec![
                K {
                    name: "Genghis Khan".to_string(),
                    score: 500,
                },
                K {
                    name: "Josef Stalin".to_string(),
                    score: 250,
                },
                K {
                    name: "Dudley Do-Right".to_string(),
                    score: 800,
                },
            ],
        };
        assert_eq!(expected, from_str(j)?);
        Ok(())
    }

    #[test]
    fn dict_default() -> Result<(), Box<Error>> {
        let j = "person: Immanuel Kant\n  score: 600\nperson: Arthur Schopenhauer\n  score: 225\nperson: René Descartes\n  score: 400\n";
        let expected = J {
            person: vec![
                K {
                    name: "Immanuel Kant".to_string(),
                    score: 600,
                },
                K {
                    name: "Arthur Schopenhauer".to_string(),
                    score: 225,
                },
                K {
                    name: "René Descartes".to_string(),
                    score: 400,
                },
            ],
        };
        assert_eq!(expected, from_str(j)?);
        Ok(())
    }
}
