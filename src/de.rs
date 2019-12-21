// de.rs
//
// Copyright (c) 2019  Douglas Lau
//
use crate::common::{Define, Separator};
use crate::error::{Error, ParseError, Result};
use crate::lines::DefIter;
use crate::parse::{self, Integer, Number};
use serde::de::{
    self, Deserialize, DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess,
    Visitor,
};
use std::io::Read;
use std::str;

/// Parsed text value
enum TextVal<'a> {
    /// Owned text value
    Owned(String),
    /// Borrowed text value
    Borrowed(&'a str),
}

/// Branch state
#[derive(Debug)]
enum BranchState {
    /// First field of branch
    First,
    /// Visiting fields of branch
    Visit,
    /// Clean up branch
    Cleanup,
}

/// Branch for stack
#[derive(Debug)]
struct Branch<'a> {
    /// Field names
    fields: &'static [&'static str],
    /// Flags for visited fields (same length as fields)
    visited: Vec<bool>,
    /// Branch state
    state: BranchState,
    /// Current key (should match one field)
    key: Option<&'a str>,
    /// List flag (applies to current key)
    list: bool,
    /// Substitute key
    substitute: Option<&'a str>,
}

impl<'a> Branch<'a> {
    /// Create a new Branch with fields
    fn with_fields(fields: &'static [&'static str]) -> Self {
        let visited = vec![false; fields.len()];
        Branch {
            fields,
            visited,
            state: BranchState::First,
            key: None,
            list: false,
            substitute: None,
        }
    }

    /// Create a new Branch
    fn new() -> Self {
        let mut branch = Branch::with_fields(&[]);
        branch.state = BranchState::Visit;
        branch
    }

    /// Get first field
    fn first_field(&self) -> Option<&'a str> {
        match self.fields.first() {
            Some(field) => Some(*field),
            _ => None,
        }
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
        self.visited.iter().any(|v| !v)
    }

    /// Cleanup state for one field
    fn cleanup_visit(&mut self) -> Option<&'static str> {
        if let BranchState::Cleanup = self.state {
            for i in 0..self.fields.len() {
                if !self.visited[i] {
                    self.visited[i] = true;
                    return Some(self.fields[i]);
                }
            }
        }
        None
    }

    /// Check if current field is substitute
    fn is_substitute(&self) -> bool {
        self.key.is_some() && self.key == self.substitute
    }
}

/// Iterator for key/value mappings
struct MappingIter<'a> {
    /// Define iterator
    defs: DefIter<'a>,
    /// Current define
    define: Option<Define<'a>>,
    /// Stack of nested branches
    stack: Vec<Branch<'a>>,
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
    fn new(input: &'a str) -> Self {
        let defs = DefIter::new(input);
        let define = None;
        let stack = vec![];
        MappingIter {
            defs,
            define,
            stack,
        }
    }

    /// Peek at next define
    fn peek(&mut self) -> Result<Option<Define<'a>>> {
        if let Some(e) = self.defs.error() {
            return Err(Error::FailedParse(e));
        }
        if self.define.is_none() {
            self.define = self.defs.next();
        }
        Ok(self.define)
    }

    /// Get the next define in a list
    fn next_list(&mut self) -> Option<Define<'a>> {
        if let Some(define) = self.define {
            match define.separator {
                Separator::Normal => {
                    let (d0, d1) = define.split_list();
                    self.define = d1;
                    Some(d0)
                }
                _ => {
                    self.define = None;
                    Some(define)
                }
            }
        } else {
            self.define
        }
    }

    /// Push branch onto the stack
    fn push_stack(&mut self, branch: Branch<'a>) {
        self.stack.push(branch)
    }

    /// Pop from the branch stack
    fn pop_stack(&mut self) {
        self.stack.pop();
    }

    /// Check if record is first field
    fn check_first_record(&mut self) -> bool {
        if let Some(branch) = self.stack.last_mut() {
            match branch.state {
                BranchState::First => {
                    branch.state = BranchState::Visit;
                    true
                }
                _ => false,
            }
        } else {
            false
        }
    }

    /// Check record substitute
    fn check_substitute(&mut self, first_record: bool) -> Result<()> {
        let indent = self.stack.len();
        if let Some(branch) = self.stack.last_mut() {
            if let Some(key) = branch.first_field() {
                if let Some(define) = self.define.take() {
                    if define.value.len() > 0 && indent > 0 {
                        if first_record {
                            return Err(Error::FailedParse(
                                ParseError::InvalidSubstitute,
                            ));
                        }
                        branch.substitute = Some(key);
                        self.define = Some(Define::new(
                            indent - 1,
                            key,
                            define.separator,
                            define.value,
                        ))
                    }
                }
            }
        }
        Ok(())
    }

    /// Check if cleanup is needed
    fn check_cleanup(&mut self) -> bool {
        if let Some(branch) = self.stack.last_mut() {
            branch.state = BranchState::Cleanup;
            branch.has_unvisited()
        } else {
            false
        }
    }

    /// Set the current key on stack
    fn set_key(&mut self, key: Option<&'a str>) {
        if let Some(branch) = self.stack.last_mut() {
            branch.visit(key)
        }
    }

    /// Set the top of stack to a list
    fn set_list(&mut self, list: bool) {
        if let Some(branch) = self.stack.last_mut() {
            branch.list = list;
            branch.state = BranchState::Visit;
        }
    }

    /// Check if the current define is a list
    fn is_list(&self) -> bool {
        match self.stack.last() {
            Some(branch) => branch.list,
            _ => false,
        }
    }

    /// Check indent nesting
    fn check_indent(&mut self) -> Result<bool> {
        match self.peek()? {
            Some(define) => Ok(define.check_indent(self.stack.len())),
            None => Ok(false),
        }
    }

    /// Check that key matches
    fn check_key(&mut self) -> Result<bool> {
        if let Some(branch) = self.stack.last() {
            if let Some(k) = branch.key {
                if let Some(define) = self.peek()? {
                    return Ok(define.key == k);
                }
            }
        }
        Ok(false)
    }

    /// Check if next item is appended
    fn is_append(&mut self) -> Result<bool> {
        Ok(self.check_indent()? && self.check_key()?)
    }

    /// Check if separator is a text append
    fn is_separator_text_append(&mut self) -> Result<bool> {
        match self.peek()? {
            Some(define) => Ok(define.separator == Separator::TextAppend),
            _ => Ok(false),
        }
    }

    /// Check if next item is text appended
    fn is_text_append(&mut self) -> Result<bool> {
        Ok(self.is_append()? && self.is_separator_text_append()?)
    }
}

/// Structure that can deserialize MuON into values.
pub struct Deserializer<'de> {
    mappings: MappingIter<'de>,
}

impl<'de> Deserializer<'de> {
    fn from_str(input: &'de str) -> Self {
        let mappings = MappingIter::new(input);
        Deserializer { mappings }
    }
}

/// Deserialize `T` from a string slice containing MuON
///
/// # Example
/// ```
/// # use serde_derive::Deserialize;
/// #[derive(Debug, Deserialize)]
/// struct Person {
///     name: String,
///     born: u32,
///     birthplace: Option<String>,
/// }
/// let muon = "name: Arthur Schopenhauer\nborn: 1788\nbirthplace: Danzig\n";
/// let person: Person = muon_rs::from_str(muon).unwrap();
/// println!("{:?}", person);
/// ```
///
/// # Errors
///
/// An error will be returned if the conversion cannot be performed.
/// This can occur for a number of reasons:
/// * The MuON data is malformed
/// * The structure of the MuON data does not match the structure of `T`
/// * A required field is missing
/// * A value is too big to fit within a primitive defined by `T`
pub fn from_str<'a, T>(s: &'a str) -> Result<T>
where
    T: Deserialize<'a>,
{
    let mut deserializer = Deserializer::from_str(s);
    let t = T::deserialize(&mut deserializer)?;
    Ok(t)
}

/// Deserialize `T` from a byte slice containing MuON
///
/// # Example
/// ```
/// # use serde_derive::Deserialize;
/// #[derive(Debug, Deserialize)]
/// struct Person {
///     name: String,
///     born: u32,
///     birthplace: Option<String>,
/// }
/// let muon = b"name: Arthur Schopenhauer\nborn: 1788\nbirthplace: Danzig\n";
/// let person: Person = muon_rs::from_slice(muon).unwrap();
/// println!("{:?}", person);
/// ```
///
/// # Errors
///
/// An error will be returned if the conversion cannot be performed.
/// This can occur for a number of reasons:
/// * The slice contains invalid UTF-8
/// * The MuON data is malformed
/// * The structure of the MuON data does not match the structure of `T`
/// * A required field is missing
/// * A value is too big to fit within a primitive defined by `T`
pub fn from_slice<'a, T>(v: &'a [u8]) -> Result<T>
where
    T: Deserialize<'a>,
{
    from_str(str::from_utf8(v)?)
}

/// Deserialize `T` from a reader IO stream containing MuON
///
/// This may call many short reads, so wrapping the reader with
/// [`std::io::BufReader`].
///
/// [`std::io::BufReader`]: https://doc.rust-lang.org/std/io/struct.BufReader.html
///
/// # Example
/// ```
/// # use serde_derive::Deserialize;
/// # use std::fs::File;
/// #[derive(Debug, Deserialize)]
/// struct BookList {
///     book: Vec<Book>,
/// }
///
/// #[derive(Debug, Deserialize)]
/// struct Book {
///     title: String,
///     author: String,
///     year: Option<i16>,
///     character: Vec<Character>,
/// }
///
/// #[derive(Debug, Deserialize)]
/// struct Character {
///     name: String,
///     location: Option<String>,
/// }
///
/// # fn main() -> Result<(), muon_rs::Error> {
/// let muon = File::open("tests/books.muon")?;
/// let books: BookList = muon_rs::from_reader(muon)?;
/// println!("{:?}", books);
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// An error will be returned if the conversion cannot be performed.
/// This can occur for a number of reasons:
/// * An IO error is encountered from reader
/// * The MuON data is malformed
/// * The structure of the MuON data does not match the structure of `T`
/// * A required field is missing
/// * A value is too big to fit within a primitive defined by `T`
pub fn from_reader<R, T>(mut reader: R) -> Result<T>
where
    R: Read,
    T: DeserializeOwned,
{
    // FIXME: this should be optimized
    let mut s = String::new();
    reader.read_to_string(&mut s)?;
    from_str(&s)
}

// FIXME: add a from_value function

impl<'de> Deserializer<'de> {
    /// Parse a define into a result
    fn define_result(
        &self,
        define: Option<Define<'de>>,
    ) -> Result<Define<'de>> {
        match define {
            Some(define) => Ok(define),
            None => match self.mappings.defs.error() {
                Some(e) => Err(Error::FailedParse(e)),
                None => Err(Error::FailedParse(ParseError::ExpectedMore)),
            },
        }
    }

    /// Peek the current key
    fn peek_key(&mut self) -> Result<&'de str> {
        let def = self.mappings.peek()?;
        match self.define_result(def)? {
            define => Ok(define.key),
        }
    }

    /// Get the current value
    fn get_value(&mut self) -> Result<&'de str> {
        let def = self.mappings.next();
        match self.define_result(def)? {
            define => Ok(define.value),
        }
    }

    /// Parse a text value
    fn parse_text(&mut self) -> Result<TextVal<'de>> {
        let val = self.get_value()?;
        if self.mappings.is_text_append()? {
            let mut value = val.to_string();
            while self.mappings.is_text_append()? {
                value.push('\n');
                value.push_str(self.get_value()?);
            }
            Ok(TextVal::Owned(value))
        } else {
            Ok(TextVal::Borrowed(val))
        }
    }

    /// Parse a char value
    fn parse_char(&mut self) -> Result<char> {
        let text = self.get_value()?;
        let mut chars = text.chars();
        if let Some(c) = chars.next() {
            if chars.next().is_none() {
                return Ok(c);
            }
        }
        Err(Error::FailedParse(ParseError::ExpectedChar))
    }

    /// Parse a bool value
    fn parse_bool(&mut self) -> Result<bool> {
        let value = self.get_value()?;
        match value {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err(Error::FailedParse(ParseError::ExpectedBool)),
        }
    }

    /// Parse an int value
    fn parse_int<T: Integer>(&mut self) -> Result<T> {
        let value = self.get_value()?;
        if let Some(v) = parse::int(value) {
            Ok(v)
        } else {
            Err(Error::FailedParse(ParseError::ExpectedInt))
        }
    }

    /// Parse a number value
    fn parse_number<T: Number>(&mut self) -> Result<T> {
        let value = self.get_value()?;
        if let Some(v) = parse::number(value) {
            Ok(v)
        } else {
            Err(Error::FailedParse(ParseError::ExpectedNumber))
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
        if let Some(branch) = self.mappings.stack.last() {
            dbg!(&branch.key);
        }
        Err(Error::FailedParse(ParseError::UnexpectedKey))
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
        visitor.visit_f32(self.parse_number()?)
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        visitor.visit_f64(self.parse_number()?)
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
        match self.parse_text()? {
            TextVal::Owned(val) => visitor.visit_str(&val),
            TextVal::Borrowed(val) => visitor.visit_borrowed_str(&val),
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
        if let Some(branch) = self.mappings.stack.last() {
            if let BranchState::Cleanup = branch.state {
                return visitor.visit_none();
            }
            if branch.is_substitute() {
                return Err(Error::FailedParse(ParseError::InvalidSubstitute));
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
        self.mappings.push_stack(Branch::new());
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
        let first_record = self.mappings.check_first_record();
        self.mappings.push_stack(Branch::with_fields(fields));
        self.mappings.check_substitute(first_record)?;
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
        if let Some(branch) = self.mappings.stack.last_mut() {
            if let Some(field) = branch.cleanup_visit() {
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
        if self.mappings.is_append()? {
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
        if self.mappings.check_indent()? || self.mappings.check_cleanup() {
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
    use super::super::datetime::*;
    use super::{from_str, Error, ParseError};
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
        let a = "b: true\nuint: xF00D\nint: b1111_0000_1111\n";
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
    fn numbers() -> Result<(), Box<Error>> {
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
        assert_eq!(
            D {
                struct_e: {
                    E {
                        struct_f: F { int: 987654321 },
                        flag: false,
                    }
                },
            },
            from_str(
                "struct_e:\n  struct_f:\n    int: 987_654_321\n  flag: false\n"
            )?
        );
        assert_eq!(
            D {
                struct_e: {
                    E {
                        struct_f: F { int: -123456 },
                        flag: true,
                    }
                },
            },
            from_str(
                "struct_e:\n  flag: true\n  struct_f:\n    int: -12_34_56\n"
            )?
        );
        match from_str::<E>("struct_f: 223344\n  int: 55\n") {
            Err(Error::FailedParse(ParseError::InvalidSubstitute)) => (),
            r => panic!("bad result {:?}", r),
        }
        Ok(())
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct G {
        string: String,
    }

    #[test]
    fn text_append() -> Result<(), Box<Error>> {
        let g = "string: This is a long string\n      :>for testing\n      :>append definitions\n";
        let expected = G {
            string: "This is a long string\nfor testing\nappend definitions"
                .to_string(),
        };
        assert_eq!(expected, from_str(g)?);
        match from_str::<G>("string: test\njunk: stuff\n") {
            Err(Error::FailedParse(ParseError::UnexpectedKey)) => (),
            r => panic!("bad result {:?}", r),
        }
        Ok(())
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct H {
        strings: Vec<String>,
    }

    #[test]
    fn text_list() -> Result<(), Box<Error>> {
        let h = "strings: first second third\n       :>item\n       : fourth\n       :=fifth item\n       : sixth\n";
        assert_eq!(
            H {
                strings: vec![
                    "first".to_string(),
                    "second".to_string(),
                    "third\nitem".to_string(),
                    "fourth".to_string(),
                    "fifth item".to_string(),
                    "sixth".to_string(),
                ],
            },
            from_str::<H>(h)?
        );
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
        let i = "int: xfab\n";
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
    fn record_list() -> Result<(), Box<Error>> {
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

    #[derive(Deserialize, PartialEq, Debug)]
    struct M {
        name: String,
        date: Date,
        time: Time,
        datetime: DateTime,
    }
    #[test]
    fn datetime() -> Result<(), Box<Error>> {
        let date = "2019-08-07".parse().map_err(|e| Error::FailedParse(e))?;
        let time = "12:34:56.789".parse().map_err(|e| Error::FailedParse(e))?;
        let datetime = "1999-12-31T23:59:59.999-00:00"
            .parse()
            .map_err(|e| Error::FailedParse(e))?;
        assert_eq!(
            M { name: "one day".to_string(), date, time, datetime },
            from_str("name: one day\ndate: 2019-08-07\ntime: 12:34:56.789\ndatetime: 1999-12-31T23:59:59.999-00:00\n")?
        );
        Ok(())
    }

    #[test]
    fn record_substitute() -> Result<(), Box<Error>> {
        assert_eq!(
            J {
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
            },
            from_str("person: Immanuel Kant\n  score: 600\nperson: Arthur Schopenhauer\n  score: 225\nperson: René Descartes\n  score: 400\n")?
        );
        Ok(())
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct N {
        name: Option<String>,
        id: u32,
    }
    #[derive(Deserialize, PartialEq, Debug)]
    struct O {
        thing: Vec<N>,
    }
    #[test]
    fn record_optional() -> Result<(), Box<Error>> {
        assert_eq!(
            O {
                thing: vec![
                    N {
                        name: Some("X".to_string()),
                        id: 1
                    },
                    N { name: None, id: 2 },
                ]
            },
            from_str("thing:\n  name: X\n  id: 1\nthing:\n  id: 2\n")?
        );
        match from_str::<O>("thing: X\n  id: 1\nthing:\n  id: 2\n") {
            Err(Error::FailedParse(ParseError::InvalidSubstitute)) => (),
            r => panic!("bad result {:?}", r),
        };
        Ok(())
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct R {
        name: F,
        other: u32,
    }
    #[test]
    fn substitute_record() -> Result<(), Box<Error>> {
        assert_eq!(
            R {
                name: F { int: 999 },
                other: 15,
            },
            from_str("name:\n  int: 999\nother: 15\n")?
        );
        match from_str::<R>("name: 999\nother: 15\n") {
            Err(Error::FailedParse(ParseError::InvalidSubstitute)) => (),
            r => panic!("bad result {:?}", r),
        };
        Ok(())
    }
}
