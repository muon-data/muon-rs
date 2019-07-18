// ser.rs
//
// Copyright (c) 2019  Douglas Lau
//
use crate::error::{Error, Result};
use serde::ser::{self, Serialize};
use std::fmt;

trait Item {
    fn write<W: fmt::Write>(&self, writer: &mut W) -> fmt::Result;
}

impl Item for bool {
    fn write<W: fmt::Write>(&self, writer: &mut W) -> fmt::Result {
        writer.write_str(if *self { "true" } else { "false" })
    }
}

macro_rules! item_itoa {
    ($tp:ty) => {
        impl Item for $tp {
            fn write<W: fmt::Write>(&self, writer: &mut W) -> fmt::Result {
                itoa::fmt(writer, *self)
            }
        }
    };
}

item_itoa!(i8);
item_itoa!(i16);
item_itoa!(i32);
item_itoa!(i64);
item_itoa!(i128);
item_itoa!(u8);
item_itoa!(u16);
item_itoa!(u32);
item_itoa!(u64);
item_itoa!(u128);

impl Item for f32 {
    fn write<W: fmt::Write>(&self, writer: &mut W) -> fmt::Result {
        writer.write_str(&*self.to_string())
    }
}

impl Item for f64 {
    fn write<W: fmt::Write>(&self, writer: &mut W) -> fmt::Result {
        writer.write_str(&*self.to_string())
    }
}

impl Item for char {
    fn write<W: fmt::Write>(&self, writer: &mut W) -> fmt::Result {
        writer.write_char(*self)
    }
}

impl Item for &str {
    fn write<W: fmt::Write>(&self, writer: &mut W) -> fmt::Result {
        writer.write_str(self)
    }
}

/// Key / Value enum for state
#[derive(Debug)]
enum KeyValue {
    Key,
    Value,
    ValueSeq,
}

/// Position of line output
#[derive(Debug)]
enum LinePos {
    /// Start of line
    Start,
    /// After one or more values
    AfterValue,
}

/// MuON serializer
pub struct Serializer {
    n_indent: usize,
    keys: Vec<Option<String>>,
    nesting: usize,
    line: LinePos,
    double_colon: bool,
    stack: Vec<KeyValue>,
    output: String,
}

impl Serializer {
    /// Create a new MuON Serializer
    fn new(n_indent: usize) -> Self {
        // Indents must be at least 1 space
        let n_indent = n_indent.max(1);
        Serializer {
            n_indent,
            keys: vec![],
            nesting: 0,
            line: LinePos::Start,
            double_colon: false,
            stack: vec![],
            output: String::new(),
        }
    }

    fn set_key(&mut self, key: &str) {
        if let Some(mut k) = self.keys.pop() {
            let key = quoted_key(key);
            k.replace(key);
            self.set_nesting();
            self.keys.push(k);
        }
        self.write_linefeed();
    }

    fn nesting(&self) -> usize {
        self.keys.len()
    }

    fn set_nesting(&mut self) {
        self.nesting = self.nesting();
    }

    fn set_double_colon(&mut self, double_colon: bool) -> bool {
        let r = self.double_colon && double_colon;
        self.double_colon = double_colon;
        r
    }

    fn key_len(&self) -> usize {
        let n = self.nesting();
        if n > 0 {
            if let Some(k) = &self.keys[n - 1] {
                k.len()
            } else {
                0
            }
        } else {
            0
        }
    }

    fn is_key(&self) -> bool {
        let n = self.stack.len();
        (n > 0)
            && match self.stack[n - 1] {
                KeyValue::Key => true,
                _ => false,
            }
    }

    fn is_sequence(&self) -> bool {
        let n = self.stack.len();
        (n > 0)
            && match self.stack[n - 1] {
                KeyValue::ValueSeq => true,
                _ => false,
            }
    }

    fn is_merge_line(&self) -> bool {
        match self.line {
            LinePos::Start => false,
            LinePos::AfterValue => !self.double_colon,
        }
    }

    fn ser_kv<T>(&mut self, t: &T, kv: KeyValue) -> Result<()>
    where
        T: ?Sized + Serialize,
    {
        self.stack.push(kv);
        t.serialize(&mut *self)?;
        self.stack.pop();
        Ok(())
    }

    fn write_linefeed(&mut self) {
        if let LinePos::AfterValue = self.line {
            self.output.push('\n');
            self.line = LinePos::Start;
        }
    }

    fn ser_item<I: Item>(&mut self, item: I) -> Result<()> {
        if self.is_key() {
            return Err(Error::InvalidKey);
        }
        if self.is_merge_line() {
            self.output.push(' ');
        } else {
            self.write_keys();
        }
        item.write(&mut self.output)?;
        self.line = LinePos::AfterValue;
        if self.double_colon {
            self.write_linefeed();
        }
        Ok(())
    }

    fn write_keys(&mut self) {
        if self.nesting == self.nesting() {
            self.write_blank_key();
        } else {
            let n0 = self.nesting.max(1) - 1;
            let n1 = self.nesting();
            for n in n0..n1 {
                self.write_key(n);
                if n + 1 < n1 {
                    self.output += ":\n";
                }
            }
            self.set_nesting();
        }
        self.write_colon();
    }

    fn write_blank_key(&mut self) {
        self.write_linefeed();
        self.write_indent(self.nesting);
        for _ in 0..self.key_len() {
            self.output.push(' ');
        }
    }

    fn write_indent(&mut self, n: usize) {
        for _ in self.n_indent..n * self.n_indent {
            self.output.push(' ');
        }
    }

    fn write_colon(&mut self) {
        if self.double_colon {
            self.output += ":: ";
        } else {
            self.output += ": ";
        }
    }

    fn write_key(&mut self, n: usize) {
        self.write_linefeed();
        self.write_indent(n + 1);
        let k = &self.keys[n];
        if let Some(key) = k {
            self.output += &key;
        }
    }
}

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

fn is_quoting_required(k: &str) -> bool {
    k.starts_with(' ')
        || k.starts_with('"')
        || k.starts_with('#')
        || k.contains(':')
}

impl<'a> ser::Serializer for &'a mut Serializer {
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
        if self.is_key() {
            self.set_key(v);
        } else {
            if self.set_double_colon(
                self.is_sequence() && (v.contains(' ') || v.contains('\n')),
            ) {
                // Blank line needed between two double-colon values
                self.write_blank_key();
                self.output += ":\n";
                self.line = LinePos::Start;
            }
            for val in v.split('\n') {
                self.ser_item(val)?;
            }
        }
        Ok(())
    }

    fn serialize_bytes(self, _v: &[u8]) -> Result<()> {
        Ok(())
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
        self.ser_kv(variant, KeyValue::Key)?;
        self.ser_kv(value, KeyValue::Value)
    }

    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
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
        self.ser_kv(variant, KeyValue::Key)?;
        Ok(self)
    }

    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.keys.push(None);
        Ok(self)
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct> {
        self.keys.push(None);
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

impl<'a> ser::SerializeSeq for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<V>(&mut self, value: &V) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        self.ser_kv(value, KeyValue::ValueSeq)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTuple for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_element<V>(&mut self, value: &V) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        self.ser_kv(value, KeyValue::ValueSeq)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, value: &V) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        self.ser_kv(value, KeyValue::ValueSeq)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeTupleVariant for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, value: &V) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        self.ser_kv(value, KeyValue::ValueSeq)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<'a> ser::SerializeMap for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_key<K>(&mut self, key: &K) -> Result<()>
    where
        K: ?Sized + Serialize,
    {
        self.ser_kv(key, KeyValue::Key)
    }

    fn serialize_value<V>(&mut self, value: &V) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        self.ser_kv(value, KeyValue::Value)
    }

    fn end(self) -> Result<()> {
        self.keys.pop();
        Ok(())
    }
}

impl<'a> ser::SerializeStruct for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    fn serialize_field<V>(&mut self, key: &'static str, value: &V) -> Result<()>
    where
        V: ?Sized + Serialize,
    {
        self.ser_kv(key, KeyValue::Key)?;
        self.ser_kv(value, KeyValue::Value)
    }

    fn end(self) -> Result<()> {
        self.keys.pop();
        self.write_linefeed();
        self.set_nesting();
        Ok(())
    }
}

impl<'a> ser::SerializeStructVariant for &'a mut Serializer {
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
    let mut serializer = Serializer::new(2);
    value.serialize(&mut serializer)?;
    serializer.write_linefeed();
    Ok(serializer.output)
}

// FIXME: add to_bytes, to_writer

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
    fn struct_b() -> Result<(), Box<Error>> {
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
    fn struct_c() -> Result<(), Box<Error>> {
        let s = C {
            ints: vec![-1234567890123456, 55555],
            string_c: vec!["first item", "second", "third", "fourth item"],
        };
        assert_eq!(
            to_string(&s)?,
            r#"ints: -1234567890123456 55555
string_c:: first item
        : second third
        :: fourth item
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
    fn struct_d() -> Result<(), Box<Error>> {
        let s = D {
            num: 15,
            text_list: vec![
                "first item",
                "second",
                "third",
                "fourth item",
                "fifth\nitem",
                "sixth",
            ],
        };
        assert_eq!(
            to_string(&s)?,
            r#"num: 15
text_list:: first item
         : second third
         :: fourth item
         :
         :: fifth
         :: item
         : sixth
"#
        );

        let s = D {
            num: 12,
            text_list: vec![],
        };
        assert_eq!(to_string(&s)?, "num: 12\n");

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
    fn struct_f() -> Result<(), Box<Error>> {
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
            r#"struct_e:
  flag: false
struct_e:
  flag: false
struct_e:
  flag: true
struct_e:
  flag: false
"#
        );
        Ok(())
    }

    #[derive(Serialize)]
    struct G {
        option_a: Option<bool>,
        option_b: Option<u32>,
    }
    #[test]
    fn struct_g() -> Result<(), Box<Error>> {
        let s = G {
            option_a: None,
            option_b: Some(37),
        };
        assert_eq!(to_string(&s)?, "option_b: 37\n");

        let s = G {
            option_a: Some(false),
            option_b: None,
        };
        assert_eq!(to_string(&s)?, "option_a: false\n");

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
