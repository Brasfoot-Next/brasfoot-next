//! A Java object graph as read from a Kryo stream, independent of what the classes mean.
//! The importer walks it to build the game model.

pub type ClassId = u32;
pub type NodeId = u32;
pub type StrId = u32;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Byte(i8),
    Short(i16),
    Char(u16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    Str(StrId),
    Enum { class: ClassId, ordinal: u32 },
    Node(NodeId),
}

#[derive(Debug)]
pub struct Node {
    pub class: ClassId,
    pub data: NodeData,
}

#[derive(Debug)]
pub enum NodeData {
    /// A FieldSerializer object: values in the class's serialized field order.
    Object(Vec<Value>),
    /// Collections, object arrays and string arrays.
    Items(Vec<Value>),
    Map(Vec<(Value, Value)>),
    Ints(Vec<i32>),
    Longs(Vec<i64>),
    Doubles(Vec<f64>),
    Floats(Vec<f32>),
    Booleans(Vec<bool>),
    Shorts(Vec<i16>),
    Bytes(Vec<i8>),
    Chars(Vec<u16>),
    Calendar(Calendar),
}

/// `java.util.Calendar` as Kryo's CalendarSerializer writes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Calendar {
    pub time_zone: String,
    pub millis: i64,
    pub lenient: bool,
    pub first_day_of_week: i32,
    pub minimal_days_in_first_week: i32,
    pub gregorian_change: i64,
}

#[derive(Debug)]
pub struct GraphClass {
    pub name: String,
    /// Serialized field names, in the order of `NodeData::Object` values.
    pub fields: Vec<String>,
}

#[derive(Debug, Default)]
pub struct Graph {
    pub classes: Vec<GraphClass>,
    pub strings: Vec<String>,
    pub nodes: Vec<Node>,
    /// The top-level objects, one per `writeClassAndObject` call.
    pub roots: Vec<Value>,
}

impl Graph {
    pub fn node(&self, id: NodeId) -> &Node {
        &self.nodes[id as usize]
    }

    pub fn class_name(&self, class: ClassId) -> &str {
        &self.classes[class as usize].name
    }

    pub fn string(&self, id: StrId) -> &str {
        &self.strings[id as usize]
    }

    /// The value of a named field of an object node, if the node is an object with that field.
    pub fn field(&self, node: NodeId, name: &str) -> Option<Value> {
        let node = self.node(node);
        let NodeData::Object(values) = &node.data else { return None };
        let index = self.classes[node.class as usize].fields.iter().position(|f| f == name)?;
        values.get(index).copied()
    }
}
