// de.rs
//
// Copyright (c) 2019-2020  Douglas Lau
//
use crate::common::{Define, Separator};
use crate::error::{Error, ParseError, Result};
use crate::lines::DefIter;
use crate::parse::{self, Integer, Number};
use serde::de::{
    self, Deserialize, DeserializeOwned, DeserializeSeed, MapAccess, SeqAccess,
    Visitor,
};
use std::borrow::Cow;
use std::io::Read;
use std::str;

/// Branch state
#[derive(Copy, Clone, Debug, PartialEq)]
enum BranchState {
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
            state: BranchState::Visit,
            key: None,
            list: false,
            substitute: None,
        }
    }

    /// Create a new Branch
    fn new() -> Self {
        Branch::with_fields(&[])
    }

    /// Get first field
    fn first_field(&self) -> Option<&'a str> {
        self.fields.first().copied()
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

    /// Cleanup state for one field
    fn cleanup_visit(&mut self) -> Option<&'static str> {
        if self.state == BranchState::Cleanup {
            for i in 0..self.fields.len() {
                if !self.visited[i] {
                    self.visited[i] = true;
                    return Some(self.fields[i]);
                }
            }
        }
        None
    }

    /// Check whether all fields have been visited
    fn all_visited(&self) -> bool {
        self.visited.iter().all(|v| *v)
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

    /// Check record substitute
    fn check_substitute(&mut self) -> Result<()> {
        let indent = self.stack.len();
        if let Some(branch) = self.stack.last_mut() {
            if let Some(key) = branch.first_field() {
                if let Some(define) = self.define.take() {
                    if !define.value.is_empty() && indent > 0 {
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
        }
    }

    /// Check if the current define is a list
    fn is_list(&self) -> bool {
        match self.stack.last() {
            Some(branch) => branch.list,
            _ => false,
        }
    }

    /// Check whether indent nesting matches
    fn check_indent(&mut self) -> Result<bool> {
        match self.peek()? {
            Some(define) => Ok(define.check_indent(self.stack.len())),
            None => Ok(false),
        }
    }

    /// Get state of current branch
    fn branch_state(&self) -> BranchState {
        match &self.stack.last() {
            Some(branch) => branch.state,
            None => BranchState::Cleanup,
        }
    }

    /// Check whether the current branch is done
    fn check_branch_done(&mut self) -> Result<bool> {
        let done = !self.check_indent()?;
        if let Some(branch) = self.stack.last_mut() {
            if done {
                branch.state = BranchState::Cleanup;
            }
            Ok(branch.state == BranchState::Cleanup && branch.all_visited())
        } else {
            Ok(true)
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
    /// Create a Deserializer from a string slice
    fn new(input: &'de str) -> Self {
        let mappings = MappingIter::new(input);
        Deserializer { mappings }
    }
}

/// Deserialize `T` from a string slice containing MuON
///
/// # Example
/// ```
/// # use serde::Deserialize;
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
    let mut deserializer = Deserializer::new(s);
    let t = T::deserialize(&mut deserializer)?;
    Ok(t)
}

/// Deserialize `T` from a byte slice containing MuON
///
/// # Example
/// ```
/// # use serde::Deserialize;
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
/// # use serde::Deserialize;
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
        Ok(self.define_result(def)?.key)
    }

    /// Get the current value
    fn get_value(&mut self) -> Result<&'de str> {
        let def = self.mappings.next();
        Ok(self.define_result(def)?.value)
    }

    /// Parse a text value
    fn parse_text(&mut self) -> Result<Cow<'de, str>> {
        let mut val = self.get_value()?;
        let mut value = String::new();
        // Allocate a buffer if multiple lines of text
        while self.mappings.is_text_append()? {
            value.push_str(val);
            value.push('\n');
            val = self.get_value()?;
        }
        if value.is_empty() {
            Ok(Cow::Borrowed(val))
        } else {
            value.push_str(val);
            Ok(Cow::Owned(value))
        }
    }

    /// Parse a char (`text <=1 >=1`) value
    fn parse_char(&mut self) -> Result<char> {
        let val = self.get_value()?;
        // Check if char is newline
        if self.mappings.is_text_append()? && val.is_empty() {
            let val = self.get_value()?;
            // Make sure no more newlines and line is empty
            (!self.mappings.is_text_append()? && val.is_empty())
                .then_some(())
                .ok_or(Error::FailedParse(ParseError::ExpectedChar))?;
            return Ok('\n');
        }
        // Don't allow more than one line if not newline
        (!self.mappings.is_text_append()?)
            .then_some(())
            .ok_or(Error::FailedParse(ParseError::ExpectedChar))?;
        parse::char(val).ok_or(Error::FailedParse(ParseError::ExpectedChar))
    }

    /// Parse a bool value
    fn parse_bool(&mut self) -> Result<bool> {
        parse::bool(self.get_value()?)
            .ok_or(Error::FailedParse(ParseError::ExpectedBool))
    }

    /// Parse an int value
    fn parse_int<T: Integer>(&mut self) -> Result<T> {
        parse::int(self.get_value()?)
            .ok_or(Error::FailedParse(ParseError::ExpectedInt))
    }

    /// Parse a number value
    fn parse_number<T: Number>(&mut self) -> Result<T> {
        parse::number(self.get_value()?)
            .ok_or(Error::FailedParse(ParseError::ExpectedNumber))
    }
}

impl<'de, 'a> de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: Visitor<'de>,
    {
        self.deserialize_identifier(visitor)?;
        if let Some(schema) = self.mappings.defs.schema() {
            dbg!(&schema);
            dbg!(&self.mappings.stack);
            dbg!(&self.mappings.define);
            todo!("create Value from schema");
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
        if self.mappings.branch_state() == BranchState::Cleanup {
            return Err(Error::FailedParse(ParseError::MissingField));
        }
        match self.parse_text()? {
            Cow::Owned(val) => visitor.visit_string(val),
            Cow::Borrowed(val) => visitor.visit_str(val),
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
        if let Some(branch) = self.mappings.stack.last() {
            if branch.is_substitute() {
                return Err(Error::FailedParse(ParseError::InvalidSubstitute));
            }
        }
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
        self.mappings.next();
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
        self.mappings.push_stack(Branch::with_fields(fields));
        self.mappings.check_substitute()?;
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
        if self.mappings.check_branch_done()? {
            self.mappings.pop_stack();
            Ok(None)
        } else {
            seed.deserialize(&mut *self).map(Some)
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
    use serde::Deserialize;
    use std::collections::HashMap;

    #[derive(Deserialize, PartialEq, Debug)]
    struct Person {
        name: String,
        score: i32,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct People {
        person: Vec<Person>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Strings {
        strings: Vec<String>,
    }

    #[derive(Deserialize, PartialEq, Debug)]
    struct Wrapper {
        int: i64,
    }

    #[test]
    fn integers() -> Result<(), Box<Error>> {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Data {
            b: bool,
            uint: u32,
            int: i32,
        }

        let data = "b: false\nuint: 7\nint: -5\n";
        let expected = Data {
            b: false,
            uint: 7,
            int: -5,
        };
        assert_eq!(expected, from_str(data)?);
        let data = "b: true\nuint: xF00D\nint: b1111_0000_1111\n";
        let expected = Data {
            b: true,
            uint: 0xF00D,
            int: 0xF0F,
        };
        assert_eq!(expected, from_str(data)?);
        Ok(())
    }

    #[test]
    fn lists() -> Result<(), Box<Error>> {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Data {
            flags: Vec<bool>,
            values: Vec<String>,
            ints: [i16; 3],
        }

        let data =
            "flags: false true true false\nvalues: Hello World\nints: 1 2 -5\n";
        let expected = Data {
            flags: [false, true, true, false].to_vec(),
            values: ["Hello".to_string(), "World".to_string()].to_vec(),
            ints: [1, 2, -5],
        };
        assert_eq!(expected, from_str(data)?);
        let data = "flags: true true\nflags: false false\nints: 30 -25 0\n";
        let expected = Data {
            flags: [true, true, false, false].to_vec(),
            values: Vec::new(),
            ints: [30, -25, 0],
        };
        assert_eq!(expected, from_str(data)?);
        Ok(())
    }

    #[test]
    fn numbers() -> Result<(), Box<Error>> {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Data {
            float: f32,
            double: f64,
        }

        let data = "float: +3.1415927\ndouble: -123.456789e0\n";
        let expected = Data {
            float: std::f32::consts::PI,
            double: -123.456789,
        };
        assert_eq!(expected, from_str(data)?);
        let data = "float: 1e15\ndouble: inf\n";
        let expected = Data {
            float: 1e15,
            double: std::f64::INFINITY,
        };
        assert_eq!(expected, from_str(data)?);
        let data = "float: 8_765.432\ndouble: -inf\n";
        let expected = Data {
            float: 8_765.432,
            double: std::f64::NEG_INFINITY,
        };
        assert_eq!(expected, from_str(data)?);
        let data = "float: 123_.456\ndouble: 1.0\n";
        assert!(from_str::<Data>(data).is_err());
        let data = "float: _123.456\ndouble: 1.0\n";
        assert!(from_str::<Data>(data).is_err());
        let data = "float: 123.456_\ndouble: 1.0\n";
        assert!(from_str::<Data>(data).is_err());
        let data = "float: 123.456\ndouble: 1__0.0\n";
        assert!(from_str::<Data>(data).is_err());
        let data = "float: .123_456\ndouble: 1.0\n";
        assert!(from_str::<Data>(data).is_err());
        Ok(())
    }

    #[test]
    fn nesting() -> Result<(), Box<Error>> {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Nested {
            wrapper: Wrapper,
            flag: bool,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct Data {
            nested: Nested,
        }

        let expected = Data {
            nested: Nested {
                wrapper: Wrapper { int: 321 },
                flag: false,
            },
        };
        let data = "nested:\n  wrapper:\n    int: 321\n  flag: false\n";
        assert_eq!(expected, from_str(data)?);
        let data = "nested:\n  flag: false\n  wrapper:\n    int: 321\n";
        assert_eq!(expected, from_str(data)?);
        match from_str::<Nested>("wrapper: 223344\n  int: 55\n").unwrap_err() {
            Error::Deserialize(_) => Ok(()),
            r => panic!("bad error {r:?}"),
        }
    }

    #[test]
    fn char_append() -> Result<(), Box<Error>> {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Data {
            char: char,
        }

        let data = "char: ç\n";
        let expected = Data { char: 'ç' };
        assert_eq!(expected, from_str(data)?);
        let data = "char:\n    :>\n";
        let expected = Data { char: '\n' };
        assert_eq!(expected, from_str(data)?);
        let data = "char: ç\n    :>append some junk\n";
        match from_str::<Data>(data).unwrap_err() {
            Error::FailedParse(ParseError::ExpectedChar) => Ok(()),
            r => panic!("bad result: {r:?}"),
        }
    }

    #[test]
    fn text_append() -> Result<(), Box<Error>> {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Data {
            string: String,
        }

        let data = "string: This is a long string\n      :>for testing\n      \
                    :>append definitions\n";
        let expected = Data {
            string: "This is a long string\nfor testing\nappend definitions"
                .to_string(),
        };
        assert_eq!(expected, from_str(data)?);
        match from_str::<Data>("string: test\njunk: stuff\n").unwrap_err() {
            Error::FailedParse(ParseError::UnexpectedKey) => Ok(()),
            r => panic!("bad result: {r:?}"),
        }
    }

    #[test]
    fn text_list() -> Result<(), Box<Error>> {
        let data =
            "strings: first second third\n       :>item\n       : fourth\
                    \n       :=fifth item\n       : sixth\n";
        let expected = Strings {
            strings: Vec::from([
                "first".to_string(),
                "second".to_string(),
                "third\nitem".to_string(),
                "fourth".to_string(),
                "fifth item".to_string(),
                "sixth".to_string(),
            ]),
        };
        assert_eq!(expected, from_str::<Strings>(data)?);
        Ok(())
    }

    #[test]
    fn options() -> Result<(), Box<Error>> {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Data {
            flag: Option<bool>,
            int: Option<i64>,
            float: Option<f32>,
        }

        let data = "flag: false\n";
        let expected = Data {
            flag: Some(false),
            int: None,
            float: None,
        };
        assert_eq!(expected, from_str(data)?);
        let data = "int: xfab\n";
        let expected = Data {
            flag: None,
            int: Some(0xFAB),
            float: None,
        };
        assert_eq!(expected, from_str(data)?);
        let data = "float: -5e37\n";
        let expected = Data {
            flag: None,
            int: None,
            float: Some(-5e37),
        };
        assert_eq!(expected, from_str(data)?);
        Ok(())
    }

    #[test]
    fn record_list() -> Result<(), Box<Error>> {
        let data = "person:\n   name: Genghis Khan\n   score: 500\n\
                    person:\n   name: Josef Stalin\n   score: 250\n\
                    person:\n   name: Dudley Do-Right\n   score: 800\n";
        let expected = People {
            person: Vec::from([
                Person {
                    name: "Genghis Khan".to_string(),
                    score: 500,
                },
                Person {
                    name: "Josef Stalin".to_string(),
                    score: 250,
                },
                Person {
                    name: "Dudley Do-Right".to_string(),
                    score: 800,
                },
            ]),
        };
        assert_eq!(expected, from_str(data)?);
        Ok(())
    }

    #[test]
    fn record_bad() -> Result<(), Box<Error>> {
        let people = "person:\n  score: 500\n\
                      person:\n  name: Josef Stalin\n  score: 250\n";
        match from_str::<People>(people).unwrap_err() {
            Error::FailedParse(ParseError::MissingField) => Ok(()),
            r => panic!("bad error {r:?}"),
        }
    }

    #[test]
    fn datetime() -> Result<(), Box<Error>> {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Data {
            name: String,
            date: Date,
            time: Time,
            datetime: DateTime,
        }

        let date = "2019-08-07".parse().map_err(Error::FailedParse)?;
        let time = "12:34:56.789".parse().map_err(Error::FailedParse)?;
        let datetime = "1999-12-31T23:59:59.999-00:00"
            .parse()
            .map_err(Error::FailedParse)?;
        let expected = Data {
            name: "one day".to_string(),
            date,
            time,
            datetime,
        };
        let data = "name: one day\n\
                    date: 2019-08-07\n\
                    time: 12:34:56.789\n\
                    datetime: 1999-12-31T23:59:59.999-00:00\n";
        assert_eq!(expected, from_str(data)?);
        Ok(())
    }

    #[test]
    fn record_substitute() -> Result<(), Box<Error>> {
        let people = People {
            person: Vec::from([
                Person {
                    name: "Immanuel Kant".to_string(),
                    score: 600,
                },
                Person {
                    name: "Arthur Schopenhauer".to_string(),
                    score: 225,
                },
                Person {
                    name: "René Descartes".to_string(),
                    score: 400,
                },
            ]),
        };
        let data = "person: Immanuel Kant\n  score: 600\n\
                    person: Arthur Schopenhauer\n  score: 225\n\
                    person: René Descartes\n  score: 400\n";
        assert_eq!(people, from_str(data)?);
        Ok(())
    }

    #[test]
    fn record_optional() -> Result<(), Box<Error>> {
        #[derive(Deserialize, PartialEq, Debug)]
        struct Thing {
            name: Option<String>,
            id: u32,
        }

        #[derive(Deserialize, PartialEq, Debug)]
        struct Data {
            thing: Vec<Thing>,
        }

        let data = Data {
            thing: Vec::from([
                Thing {
                    name: Some("X".to_string()),
                    id: 1,
                },
                Thing { name: None, id: 2 },
            ]),
        };
        assert_eq!(
            data,
            from_str("thing:\n  name: X\n  id: 1\nthing:\n  id: 2\n")?,
        );
        let data = "thing: X\n  id: 1\nthing:\n  id: 2\n";
        match from_str::<Data>(data).unwrap_err() {
            Error::FailedParse(ParseError::InvalidSubstitute) => Ok(()),
            r => panic!("bad error {r:?}"),
        }
    }

    #[test]
    fn substitute_record() -> Result<(), Box<Error>> {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Data {
            name: Wrapper,
            other: u32,
        }

        let data = Data {
            name: Wrapper { int: 999 },
            other: 15,
        };
        assert_eq!(data, from_str("name: 999\nother: 15\n")?);
        assert_eq!(data, from_str("name:\n  int: 999\nother: 15\n")?);
        Ok(())
    }

    #[test]
    fn no_substitute_optional() -> Result<(), Box<Error>> {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Group {
            label: String,
        }

        #[derive(Debug, Deserialize, PartialEq)]
        struct Data {
            group: Option<Group>,
        }

        let data = Data {
            group: Some(Group {
                label: String::from("group label"),
            }),
        };
        assert_eq!(data, from_str("group: group label\n")?,);
        Ok(())
    }

    #[test]
    fn no_substitute_list() -> Result<(), Box<Error>> {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Data {
            chan: Vec<Strings>,
        }

        match from_str::<Data>("chan: first second\n").unwrap_err() {
            Error::FailedParse(ParseError::InvalidSubstitute) => Ok(()),
            r => panic!("bad error {r:?}"),
        }
    }

    #[test]
    #[ignore]
    fn hashmap_dict() -> Result<(), Box<Error>> {
        #[derive(Debug, Deserialize, PartialEq)]
        struct Data {
            dict: HashMap<String, String>,
        }

        let mut data = Data {
            dict: HashMap::new(),
        };
        assert_eq!(data, from_str("dict:\n")?);
        data.dict.insert("key".to_string(), "value".to_string());
        assert_eq!(data, from_str("dict:\n  key: value\n")?);
        Ok(())
    }
}
