// ser.rs
//
// Copyright (c) 2019  Douglas Lau
//
use crate::common::Separator;
use crate::error::{Error, Result};
use serde::ser::{self, Serialize};
use std::io::Write;

/// Item which can be serialized to a writer
trait Item {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()>;
}

impl Item for bool {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        if *self {
            write!(writer, "true")?;
        } else {
            write!(writer, "false")?;
        }
        Ok(())
    }
}

macro_rules! impl_item {
    () => {};
    ($i:ident $($more:ident)*) => {
        impl Item for $i {
            fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
                Ok(write!(writer, "{}", *self)?)
            }
        }
        impl_item!($($more)*);
    };
}

impl_item!(i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char);

impl Item for &str {
    fn write<W: Write>(&self, writer: &mut W) -> Result<()> {
        Ok(write!(writer, "{}", *self)?)
    }
}

/// Position of line output
#[derive(Clone, Copy, Debug)]
enum LinePos {
    /// Start of line
    Start,
    /// After one or more values
    AfterValue,
}

/// Dictionary for mapping stack
#[derive(Debug)]
struct Dict {
    /// Current key (one field in dict)
    key: Option<String>,
    /// Number of keys
    n_keys: u32,
    /// List flag (applies to current key)
    list: bool,
    /// Fields visited flag
    visited: bool,
}

/// MuON serializer
pub struct Serializer<W: Write> {
    /// Number of spaces per indent
    n_indent: usize,
    /// Writer for output
    writer: W,
    /// Stack of dict values
    stack: Vec<Dict>,
    /// Flag if current item is a key
    is_key: bool,
    /// Current output indent count
    indent: usize,
    /// Current line position
    line: LinePos,
    /// Current key / value separator
    separator: Separator,
}

impl<W: Write> Serializer<W> {
    /// Create a new MuON Serializer
    fn new(n_indent: usize, writer: W) -> Self {
        // Indents must be at least 1 space
        let n_indent = n_indent.max(1);
        Serializer {
            n_indent,
            writer,
            stack: vec![],
            is_key: false,
            indent: 0,
            line: LinePos::Start,
            separator: Separator::Normal,
        }
    }

    /// Push a new dict onto stack
    fn push_stack(&mut self) {
        self.stack.push(Dict {
            key: None,
            n_keys: 0,
            list: false,
            visited: false
        });
    }

    /// Pop a dict from stack
    fn pop_stack(&mut self) -> Result<()> {
        if let Some(dict) = self.stack.pop() {
            if !dict.visited {
                self.write_unvisited_key()?;
            }
            self.set_indent();
            self.write_linefeed()?;
        }
        Ok(())
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

    /// Set the current key
    fn set_key(&mut self, key: &str) {
        if let Some(dict) = self.stack.last_mut() {
            dict.key = Some(quoted_key(key));
            dict.n_keys += 1;
        }
    }

    /// Set the key to blank (for repeated keys)
    fn set_key_blank(&mut self) {
        if let Some(dict) = self.stack.last_mut() {
            if let Some(mut key) = dict.key.take() {
                let len = key.chars().count();
                key.clear();
                for _ in 0..len {
                    key.push(' ');
                }
                dict.key = Some(key);
            }
        }
    }

    /// Get the current nesting depth
    fn nesting(&self) -> usize {
        self.stack.len()
    }

    /// Set the output indent count
    fn set_indent(&mut self) {
        self.indent = self.nesting();
    }

    /// Check if line should be merged
    fn is_merge_line(&self) -> bool {
        match (self.line, self.separator) {
            (LinePos::AfterValue, Separator::Normal) => true,
            (_, _) => false,
        }
    }

    /// Serialize a key
    fn ser_key<T>(&mut self, t: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.is_key = true;
        t.serialize(&mut *self)?;
        self.is_key = false;
        Ok(())
    }

    /// Serialize a value
    fn ser_value<T>(&mut self, t: &T) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        Ok(t.serialize(&mut *self)?)
    }

    /// Serialize an item
    fn ser_item<I: Item>(&mut self, item: I) -> Result<()> {
        if self.is_key {
            return Err(Error::InvalidKey);
        }
        if self.is_merge_line() {
            write!(self.writer, " ")?;
        } else {
            self.write_keys()?;
        }
        item.write(&mut self.writer)?;
        self.line = LinePos::AfterValue;
        Ok(())
    }

    /// Write all necessary keys
    fn write_keys(&mut self) -> Result<()> {
        let n0 = self.indent.max(1) - 1;
        let n1 = self.nesting();
        for n in n0..n1 {
            self.write_key(n)?;
            match n1 - n {
                1 => (),
                2 if self.is_first_key() => {
                    self.visit_dict(n + 1);
                    break;
                }
                _ => write!(self.writer, ":\n")?,
            }
        }
        self.set_indent();
        self.set_key_blank();
        Ok(write!(self.writer, "{}", self.separator.as_str())?)
    }

    /// Check if the current key is the first within the dict
    fn is_first_key(&self) -> bool {
        match self.stack.last() {
            Some(dict) => dict.n_keys == 1,
            _ => false,
        }
    }

    /// Mark a dict as visited (key has been written)
    fn visit_dict(&mut self, n: usize) {
        if let Some(dict) = self.stack.iter_mut().nth(n) {
            dict.visited = true;
        }
    }

    /// Write a key
    fn write_key(&mut self, n: usize) -> Result<()> {
        self.write_linefeed()?;
        self.write_indent(n)?;
        if let Some(dict) = self.stack.iter_mut().nth(n) {
            if let Some(key) = &dict.key {
                write!(self.writer, "{}", key)?;
            }
            self.visit_dict(n);
        }
        Ok(())
    }

    /// Write key for unvisited dict
    fn write_unvisited_key(&mut self) -> Result<()> {
        let indent = self.nesting();
        if indent > 0 {
            self.write_key(indent - 1)?;
            write!(self.writer, ":\n")?;
        }
        Ok(())
    }

    /// Write a line feed if necessary
    fn write_linefeed(&mut self) -> Result<()> {
        if let LinePos::AfterValue = self.line {
            write!(self.writer, "\n")?;
            self.line = LinePos::Start;
        }
        Ok(())
    }

    /// Write an indentation
    fn write_indent(&mut self, n: usize) -> Result<()> {
        for _ in 0..n * self.n_indent {
            write!(self.writer, " ")?;
        }
        Ok(())
    }

    /// Write a text item
    fn write_text(&mut self, v: &str) -> Result<()> {
        if self.is_list() && v.contains(' ') {
            self.separator = Separator::TextValue;
        }
        for val in v.split('\n') {
            self.ser_item(val)?;
            match self.separator {
                Separator::Normal => (),
                _ => self.write_linefeed()?,
            }
            self.separator = Separator::TextAppend;
        }
        self.separator = Separator::Normal;
        Ok(())
    }
}

/// Create a quoted key
fn quoted_key(k: &str) -> String {
    if is_quoting_required(k) {
        let mut s = String::new();
        s.push('"');
        for c in k.chars() {
            s.push(c);
            if c == '"' {
                s.push(c);
            }
        }
        s.push('"');
        s
    } else {
        k.to_string()
    }
}

/// Check if quoting is required for a key
fn is_quoting_required(k: &str) -> bool {
    k.starts_with(' ')
        || k.starts_with('"')
        || k.starts_with('#')
        || k.contains(':')
}

impl<'a, W: Write> ser::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.ser_item(v)
    }
    fn serialize_i8(self, v: i8) -> Result<()> {
        self.ser_item(v)
    }
    fn serialize_i16(self, v: i16) -> Result<()> {
        self.ser_item(v)
    }
    fn serialize_i32(self, v: i32) -> Result<()> {
        self.ser_item(v)
    }
    fn serialize_i64(self, v: i64) -> Result<()> {
        self.ser_item(v)
    }
    fn serialize_i128(self, v: i128) -> Result<()> {
        self.ser_item(v)
    }
    fn serialize_u8(self, v: u8) -> Result<()> {
        self.ser_item(v)
    }
    fn serialize_u16(self, v: u16) -> Result<()> {
        self.ser_item(v)
    }
    fn serialize_u32(self, v: u32) -> Result<()> {
        self.ser_item(v)
    }
    fn serialize_u64(self, v: u64) -> Result<()> {
        self.ser_item(v)
    }
    fn serialize_u128(self, v: u128) -> Result<()> {
        self.ser_item(v)
    }
    fn serialize_f32(self, v: f32) -> Result<()> {
        self.ser_item(v)
    }
    fn serialize_f64(self, v: f64) -> Result<()> {
        self.ser_item(v)
    }
    fn serialize_char(self, v: char) -> Result<()> {
        self.ser_item(v)
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        if self.is_key {
            self.set_key(v);
            self.write_linefeed()?;
        } else {
            self.write_text(v)?;
        }
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Err(Error::UnsupportedType("bytes"))
    }

    fn serialize_none(self) -> Result<()> {
        Ok(())
    }

    fn serialize_some<V>(self, value: &V) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
    ) -> Result<()> {
        self.serialize_str(variant)
    }

    fn serialize_newtype_struct<V>(
        self,
        _name: &'static str,
        value: &V,
    ) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<V>(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        value: &V,
    ) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        self.ser_key(variant)?;
        self.ser_value(value)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.set_list(true);
        Ok(self)
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(None)
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(None)
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.set_list(true);
        self.ser_key(variant)?;
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.push_stack();
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        self.push_stack();
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::UnsupportedType("struct variant"))
    }
}

impl<'a, W: Write> ser::SerializeSeq for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<V>(&mut self, value: &V) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        self.ser_value(value)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W: Write> ser::SerializeTuple for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<V>(&mut self, value: &V) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        self.ser_value(value)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W: Write> ser::SerializeTupleStruct for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, value: &V) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        self.ser_value(value)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W: Write> ser::SerializeTupleVariant for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, value: &V) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        self.ser_value(value)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a, W: Write> ser::SerializeMap for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<K>(&mut self, key: &K) -> Result<()>
    where
        K: ?Sized + Serialize,
    {
        self.ser_key(key)
    }

    fn serialize_value<V>(&mut self, value: &V) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        self.ser_value(value)
    }

    fn end(self) -> Result<()> {
        self.pop_stack()
    }
}

impl<'a, W: Write> ser::SerializeStruct for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, key: &'static str, value: &V) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        self.ser_key(key)?;
        self.ser_value(value)
    }

    fn end(self) -> Result<()> {
        self.pop_stack()
    }
}

impl<'a, W: Write> ser::SerializeStructVariant for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(
        &mut self,
        _key: &'static str,
        _value: &V,
    ) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        Err(Error::UnsupportedType("struct variant"))
    }
    fn end(self) -> Result<()> {
        Err(Error::UnsupportedType("struct variant"))
    }
}

/// Serialize a value to a string in MuON format
pub fn to_string<T>(value: &T) -> Result<String>
where
    T: Serialize,
{
    let mut serializer = Serializer::new(2, vec![]);
    value.serialize(&mut serializer)?;
    Ok(String::from_utf8(serializer.writer)?)
}

/// Serialize a value to a Vec of bytes in MuON format
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut serializer = Serializer::new(2, vec![]);
    value.serialize(&mut serializer)?;
    Ok(serializer.writer)
}

/// Serialize a value to a writer in MuON format
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<()>
where
    W: Write,
    T: Serialize,
{
    let mut serializer = Serializer::new(2, writer);
    value.serialize(&mut serializer)?;
    Ok(())
}

#[cfg(test)]
mod test {
    use super::{to_string, Error};
    use serde_derive::Serialize;
    use std::collections::HashMap;

    #[derive(Serialize)]
    struct P {
        b: bool,
        ubyte: u8,
        byte: i8,
        ushort: u16,
        short: i16,
        uint: u32,
        int: i32,
        ulong: u64,
        long: i64,
        ull: u128,
        ll: i128,
        float: f32,
        double: f64,
        ninf: f32,
        nan: f64,
    }

    #[test]
    fn struct_p() -> Result<(), Box<Error>> {
        let s = P {
            b: true,
            ubyte: 255,
            byte: -128,
            ushort: 65535,
            short: -32768,
            uint: 0xFFFFFFFF,
            int: -12345678,
            ulong: 1234567890123456,
            long: -9876543210987654,
            ull: 12345678901234567890,
            ll: 23456789012345678901,
            float: -123.456,
            double: 789.012,
            ninf: -std::f32::INFINITY,
            nan: std::f64::NAN,
        };
        assert_eq!(
            to_string(&s)?,
            r#"b: true
ubyte: 255
byte: -128
ushort: 65535
short: -32768
uint: 4294967295
int: -12345678
ulong: 1234567890123456
long: -9876543210987654
ull: 12345678901234567890
ll: 23456789012345678901
float: -123.456
double: 789.012
ninf: -inf
nan: NaN
"#
        );
        Ok(())
    }

    #[derive(Serialize)]
    struct A {
        b: bool,
        int: u32,
        neg: i16,
        string_a: String,
    }

    #[test]
    fn struct_a() -> Result<(), Box<Error>> {
        let s = A {
            b: true,
            int: 1,
            neg: -15,
            string_a: "first second".to_string(),
        };
        assert_eq!(
            to_string(&s)?,
            r#"b: true
int: 1
neg: -15
string_a: first second
"#
        );
        Ok(())
    }

    #[derive(Serialize)]
    struct B {
        b: [bool; 3],
        string_b: Vec<&'static str>,
    }

    #[test]
    fn list_b() -> Result<(), Box<Error>> {
        let s = B {
            b: [false, true, false],
            string_b: vec!["first", "second"],
        };
        assert_eq!(
            to_string(&s)?,
            r#"b: false true false
string_b: first second
"#
        );
        Ok(())
    }

    #[derive(Serialize)]
    struct C {
        ints: Vec<i64>,
        string_c: Vec<&'static str>,
    }

    #[test]
    fn list_c() -> Result<(), Box<Error>> {
        let s = C {
            ints: vec![-1234567890123456, 55555],
            string_c: vec!["first item", "second", "third", "fourth item"],
        };
        assert_eq!(
            to_string(&s)?,
            r#"ints: -1234567890123456 55555
string_c:=first item
        : second third
        :=fourth item
"#
        );
        Ok(())
    }

    #[derive(Serialize)]
    struct D {
        num: u8,
        text_list: Vec<&'static str>,
    }
    #[test]
    fn text_list() -> Result<(), Box<Error>> {
        assert_eq!(
            to_string(&D {
                num: 15,
                text_list: vec![
                    "first item",
                    "second",
                    "third",
                    "fourth item",
                    "fifth\nitem",
                    "sixth",
                ],
            })?,
            "num: 15\ntext_list:=first item\n         : second third\n         :=fourth item\n         : fifth\n         :>item\n         : sixth\n"
        );
        assert_eq!(
            to_string(&D {
                num: 12,
                text_list: vec![],
            })?,
            "num: 12\n"
        );
        Ok(())
    }

    #[derive(Serialize)]
    struct E {
        flag: bool,
    }
    #[derive(Serialize)]
    struct F {
        struct_e: Vec<E>,
    }
    #[test]
    fn dict_list() -> Result<(), Box<Error>> {
        let s = F {
            struct_e: vec![
                E { flag: false },
                E { flag: false },
                E { flag: true },
                E { flag: false },
            ],
        };
        assert_eq!(
            to_string(&s)?,
            "struct_e: false\nstruct_e: false\nstruct_e: true\nstruct_e: false\n"
        );
        Ok(())
    }

    #[derive(Serialize)]
    struct G {
        option_a: Option<bool>,
        option_b: Option<u32>,
    }
    #[test]
    fn optional() -> Result<(), Box<Error>> {
        assert_eq!(
            to_string(&G {
                option_a: None,
                option_b: Some(37),
            })?,
            "option_b: 37\n"
        );
        assert_eq!(
            to_string(&G {
                option_a: Some(false),
                option_b: None,
            })?,
            "option_a: false\n"
        );
        Ok(())
    }
    #[derive(Serialize)]
    struct H {
        list_g: Vec<G>,
    }
    #[test]
    fn list_optional() -> Result<(), Box<Error>> {
        assert_eq!(
            to_string(&H {
                list_g: vec![
                    G { option_a: None, option_b: None },
                    G { option_a: None, option_b: Some(55) },
                    G { option_a: Some(true), option_b: None },
                    G { option_a: Some(false), option_b: Some(99) },
                    G { option_a: None, option_b: None },
                ],
            })?,
            "list_g:\nlist_g:\n  option_b: 55\nlist_g: true\nlist_g: false\n  option_b: 99\nlist_g:\n"
        );
        Ok(())
    }

    #[derive(Serialize)]
    struct I {
        txt: String,
    }
    #[derive(Serialize)]
    struct J {
        option_a: Option<i32>,
        option_b: Vec<I>,
    }
    #[derive(Serialize)]
    struct K {
        list_j: Vec<J>,
    }
    #[test]
    fn list_vec() -> Result<(), Box<Error>> {
        assert_eq!(
            to_string(&K {
                list_j: vec![
                    J {
                        option_a: Some(99),
                        option_b: vec![I { txt: "test".to_string() }]
                    },
                    J {
                        option_a: None,
                        option_b: vec![I { txt: "abc".to_string() }]
                    },
                    J {
                        option_a: Some(77),
                        option_b: vec![I { txt: "xyz".to_string() }]
                    },
                    J {
                        option_a: None,
                        option_b: vec![]
                    },
                ],
            })?,
            "list_j: 99\n  option_b: test\nlist_j:\n  option_b: abc\nlist_j: 77\n  option_b: xyz\nlist_j:\n"
        );
        Ok(())
    }
    #[derive(Serialize)]
    struct L {
        list_i: Vec<I>,
    }
    #[derive(Serialize)]
    struct M {
        dict_l: L,
    }
    #[test]
    fn list_dict() -> Result<(), Box<Error>> {
        assert_eq!(
            to_string(&M {
                dict_l: L {
                    list_i: vec![
                        I { txt: "abc".to_string() },
                        I { txt: "def".to_string() },
                        I { txt: "ghi".to_string() },
                        I { txt: "xyz".to_string() },
                    ],
                }
            })?,
            "dict_l:\n  list_i: abc\n  list_i: def\n  list_i: ghi\n  list_i: xyz\n"
        );
        Ok(())
    }

    #[test]
    fn map_quoted_keys() -> Result<(), Box<Error>> {
        let mut m = HashMap::new();
        m.insert("\"quoted\" key".to_string(), "\"quoted\" value".to_string());
        assert_eq!(
            to_string(&m)?,
            "\"\"\"quoted\"\" key\": \"quoted\" value\n"
        );

        let mut m = HashMap::new();
        m.insert(" spacey key".to_string(), "value".to_string());
        assert_eq!(to_string(&m)?, "\" spacey key\": value\n");

        let mut m = HashMap::new();
        m.insert("# commenty key".to_string(), "value".to_string());
        assert_eq!(to_string(&m)?, "\"# commenty key\": value\n");

        let mut m = HashMap::new();
        m.insert("colon: key".to_string(), "value".to_string());
        assert_eq!(to_string(&m)?, "\"colon: key\": value\n");

        let mut m = HashMap::new();
        m.insert("\ttabby key".to_string(), "value".to_string());
        assert_eq!(to_string(&m)?, "\ttabby key: value\n");
        Ok(())
    }
}
