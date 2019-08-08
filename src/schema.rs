// schema.rs
//
// Copyright (c) 2019  Douglas Lau
//
use crate::common::Define;
use crate::datetime::{Date, DateTime, Time};
use crate::error::ParseError;
use std::str::FromStr;

/// Representation of any valid MuON value
#[derive(Debug)]
#[allow(dead_code)]
pub enum Value {
    /// Text value
    Text(String),
    /// Boolean value
    Bool(bool),
//    Int(Integer),
//    Float(Float),
    /// Date and time with offset
    DateTime(DateTime),
    /// Date with no time or offset
    Date(Date),
    /// Time with no date or offset
    Time(Time),
//    Dict(Map<String, Value>),
    /// Optional value
    Optional(Option<Box<Value>>),
    /// List of values
    List(Vec<Value>),
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
    /// Text type
    Text,
    /// Boolean type
    Bool,
    /// Integer type
    Int,
    /// Float type
    Float,
    /// Date-time type
    DateTime,
    /// Date type
    Date,
    /// Time type
    Time,
    /// Dict type
    Dict,
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
                _ => (None, v[0]),
            } 
        } else {
            (None, v[0])
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
            "float" => Ok(Type::Float),
            "datetime" => Ok(Type::DateTime),
            "date" => Ok(Type::Date),
            "time" => Ok(Type::Time),
            "dict" => Ok(Type::Dict),
            _ => Err(ParseError::InvalidType),
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
        let v: Vec<&str> = value.splitn(2, ' ').collect();
        if v.len() > 0 {
            let node_type = v[0].parse()?;
            // FIXME: parse default value
            let default = None;
            Ok(Node { indent, name, modifier, node_type, default })
        } else {
            Err(ParseError::InvalidType)
        }
    }

    /// Check if node indent is valid with previous node
    fn is_indent_valid(&self, prev: Option<&Self>) -> bool {
        match prev {
            None => self.indent == 0,
            Some(prev) => {
                self.indent <= prev.indent ||
                match prev.node_type {
                    Type::Dict => self.indent == prev.indent + 1,
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
