// schema.rs
//
// Copyright (c) 2019  Douglas Lau
//
use crate::common::Define;
use crate::datetime::{Date, DateTime, Time};
use crate::error::ParseError;
use crate::parse;
use serde::{de::Visitor, Deserialize, Deserializer};
use std::fmt;
use std::str::FromStr;

/// Integer value enum
#[derive(Debug)]
pub enum IntValue {
    /// Unsigned integer value
    Unsigned(u128),
    /// Signed integer value
    Signed(i128),
}

impl FromStr for IntValue {
    type Err = ParseError;

    fn from_str(val: &str) -> Result<Self, Self::Err> {
        if let Some(vi) = parse::int(val) {
            return Ok(IntValue::Signed(vi));
        }
        if let Some(vu) = parse::int(val) {
            return Ok(IntValue::Unsigned(vu));
        }
        Err(ParseError::ExpectedInt)
    }
}

/// Number value enum
#[derive(Debug)]
pub enum NumValue {
    /// 32-bit number
    Num32(f32),
    /// 64-bit number
    Num64(f64),
}

impl FromStr for NumValue {
    type Err = ParseError;

    fn from_str(val: &str) -> Result<Self, Self::Err> {
        if let Some(v32) = parse::number(val) {
            return Ok(NumValue::Num32(v32));
        }
        if let Some(v64) = parse::number(val) {
            return Ok(NumValue::Num64(v64));
        }
        Err(ParseError::ExpectedNumber)
    }
}

/// A MuON value
#[derive(Debug)]
pub enum Value {
    /// Text value
    Text(String),
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(IntValue),
    /// Number value
    Number(NumValue),
    /// Date and time with offset
    DateTime(DateTime),
    /// Date with no time or offset
    Date(Date),
    /// Time with no date or offset
    Time(Time),
    /// Record value
    Record(Vec<(String, Value)>),
    /// Dictionary value
    Dictionary(Vec<(Value, Value)>),
    /// Any value
    Any(Box<Value>),
    /// Optional value
    Optional(Option<Box<Value>>),
    /// List of values
    List(Vec<Value>),
}

/// Visitor for deserializing a MuON Value
struct ValueVisitor;

impl<'de> Visitor<'de> for ValueVisitor {
    type Value = Value;
    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a MuON value")
    }
    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Bool(v))
    }
    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Int(IntValue::Signed(v.into())))
    }
    fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Int(IntValue::Signed(v)))
    }
    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Int(IntValue::Unsigned(v.into())))
    }
    fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Int(IntValue::Unsigned(v)))
    }
    fn visit_f32<E>(self, v: f32) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Number(NumValue::Num32(v)))
    }
    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Number(NumValue::Num64(v)))
    }
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Text(v.into()))
    }
    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Text(v.into()))
    }
    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Text(v))
    }
    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(Value::Optional(None))
    }
    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v = deserializer.deserialize_any(self)?;
        Ok(Value::Optional(Some(Box::new(v))))
    }
}

impl<'de> Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueVisitor {})
    }
}

/// Type modifier for a schema
#[derive(Debug)]
pub enum Modifier {
    /// Optional values may not be present
    Optional,
    /// List of values
    List,
}

/// Schema Node
#[derive(Debug)]
pub struct Node<'a> {
    /// Indent level
    indent: usize,
    /// Type name
    name: &'a str,
    /// Type modifier
    modifier: Option<Modifier>,
    /// Node type
    node_type: Type,
    /// Default value
    default: Option<Value>,
}

/// Schema Type
#[derive(Debug)]
pub enum Type {
    /// Text is a `String`
    Text,
    /// Boolean is a `bool`
    Bool,
    /// Integer is a signed or unsigned int
    Int,
    /// Number parses into `f64` or `f32`
    Number,
    /// Date-time parses into [DateTime](struct.DateTime.html)
    DateTime,
    /// Date parses into [Date](struct.Date.html)
    Date,
    /// Time parses into [Time](struct.Time.html)
    Time,
    /// Record parses into a struct or
    /// [Value::Record](enum.Value.html#variant.Record)
    Record,
    /// Dictionary parses into [HashMap](struct.HashMap.html) or
    /// [Value::Dictionary](enum.Value.html#variant.Dictionary)
    Dictionary,
    /// Any parses into [Value::Any](enum.Value.html#variant.Any)
    Any,
}

/// Full schema
#[derive(Debug)]
pub struct Schema<'a> {
    /// List of all nodes
    nodes: Vec<Node<'a>>,
    /// Flag indicating reading finished
    finished: bool,
}

impl Modifier {
    /// Create a type modifier from start of a string slice
    fn from_str_start(val: &str) -> (Option<Self>, &str) {
        let v: Vec<&str> = val.splitn(2, ' ').collect();
        if v.len() > 1 {
            match v[0] {
                "optional" => (Some(Modifier::Optional), v[1]),
                "list" => (Some(Modifier::List), v[1]),
                _ => (None, val),
            }
        } else {
            (None, val)
        }
    }
}

impl FromStr for Type {
    type Err = ParseError;

    fn from_str(val: &str) -> Result<Self, Self::Err> {
        match val {
            "text" => Ok(Type::Text),
            "bool" => Ok(Type::Bool),
            "int" => Ok(Type::Int),
            "number" => Ok(Type::Number),
            "datetime" => Ok(Type::DateTime),
            "date" => Ok(Type::Date),
            "time" => Ok(Type::Time),
            "record" => Ok(Type::Record),
            "dictionary" => Ok(Type::Dictionary),
            "any" => Ok(Type::Any),
            _ => Err(ParseError::InvalidType),
        }
    }
}

impl Type {
    /// Parse a value by type
    fn parse_value(
        &self,
        modifier: &Option<Modifier>,
        v: &str,
    ) -> Result<Value, ParseError> {
        match (modifier, self) {
            (None, Type::Text) => Ok(Value::Text(String::from(v))),
            (None, Type::Bool) => Ok(Value::Bool(v.parse()?)),
            (None, Type::Int) => Ok(Value::Int(v.parse()?)),
            (None, Type::Number) => Ok(Value::Number(v.parse()?)),
            (None, Type::DateTime) => Ok(Value::DateTime(v.parse()?)),
            (None, Type::Date) => Ok(Value::Date(v.parse()?)),
            (None, Type::Time) => Ok(Value::Time(v.parse()?)),
            _ => Err(ParseError::InvalidDefault),
        }
    }
}

impl<'a> Node<'a> {
    /// Create a schema node from a definition
    fn from_define(define: Define<'a>) -> Result<Self, ParseError> {
        let indent = define.indent;
        let name = define.key;
        let value = define.value;
        let (modifier, value) = Modifier::from_str_start(value);
        let mut v = value.splitn(2, ' ');
        if let Some(tp) = v.next() {
            let node_type: Type = tp.parse()?;
            let default = v
                .next()
                .map(|dflt| node_type.parse_value(&modifier, dflt))
                .transpose()?;
            Ok(Node {
                indent,
                name,
                modifier,
                node_type,
                default,
            })
        } else {
            Err(ParseError::InvalidType)
        }
    }

    /// Check if node indent is valid with previous node
    fn is_indent_valid(&self, prev: Option<&Self>) -> bool {
        match prev {
            None => self.indent == 0,
            Some(prev) => {
                self.indent <= prev.indent
                    || match prev.node_type {
                        Type::Record | Type::Dictionary | Type::Any => {
                            self.indent == prev.indent + 1
                        }
                        _ => false,
                    }
            }
        }
    }
}

impl<'a> Schema<'a> {
    /// Create a new schema
    pub fn new() -> Self {
        let nodes = vec![];
        let finished = false;
        Schema { nodes, finished }
    }

    /// Add node
    fn add_node(&mut self, node: Node<'a>) -> Result<(), ParseError> {
        if node.is_indent_valid(self.nodes.last()) {
            self.nodes.push(node);
            Ok(())
        } else {
            Err(ParseError::InvalidIndent)
        }
    }

    /// Add a define
    pub fn add_define(&mut self, def: Define<'a>) -> Result<bool, ParseError> {
        if self.finished {
            Ok(false)
        } else {
            self.add_node(Node::from_define(def)?)?;
            Ok(true)
        }
    }

    /// Finish the schema
    pub fn finish(&mut self) -> bool {
        if self.finished {
            true
        } else {
            self.finished = true;
            false
        }
    }
}
