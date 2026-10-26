// Port of the read side of Kryo 4.0.2 as the game uses it (c.a.c(String, boolean)): a default
// `new Kryo()` with registrationRequired=false, so references on (MapReferenceResolver),
// unregistered classes written by name (DefaultClassResolver), FieldSerializer with
// optimizedGenerics off and var ints. Sources: Kryo.readClassAndObject/readObject/
// readObjectOrNull/readReferenceOrNull, ObjectField.read, CollectionSerializer.read,
// MapSerializer.read, DefaultArraySerializers, DefaultSerializers.

use std::collections::HashMap;
use std::rc::Rc;

use super::graph::{Calendar, ClassId, Graph, GraphClass, Node, NodeData, NodeId, Value};
use super::input::Input;
use super::schema::Schema;
use super::KryoError;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Primitive {
    Int,
    Long,
    Double,
    Float,
    Boolean,
    Short,
    Byte,
    Char,
}

#[derive(Clone, Copy, Debug)]
enum FieldPlan {
    Primitive(Primitive),
    /// Declared type not final: the class is written before the value.
    AnyObject,
    /// Declared type final: written without class, null through the reference marker.
    FinalObject(ClassId),
}

#[derive(Debug)]
enum SerializerKind {
    Fields(Vec<FieldPlan>),
    Collection,
    Map,
    String,
    /// A boxed primitive (`IntSerializer`, `LongSerializer`...).
    Boxed(Primitive),
    Calendar,
    Enum,
    PrimitiveArray(Primitive),
    StringArray,
    /// `ObjectArraySerializer`; `same_type` when the component class is final.
    ObjectArray { component: ClassId, same_type: bool },
    Unsupported(String),
}

struct ClassPlan {
    kind: Rc<SerializerKind>,
}

/// Classes Kryo registers in its constructor, by registration id (written as id + 2).
const REGISTERED: [&str; 9] = [
    "java.lang.Integer",
    "java.lang.String",
    "java.lang.Float",
    "java.lang.Boolean",
    "java.lang.Byte",
    "java.lang.Character",
    "java.lang.Short",
    "java.lang.Long",
    "java.lang.Double",
];

enum ReferenceRead {
    Null,
    Existing(Value),
    /// A new object; `Some(slot)` when it takes a reference id.
    New(Option<usize>),
}

pub struct Reader<'a> {
    input: Input<'a>,
    schema: &'a Schema,
    graph: Graph,
    class_ids: HashMap<String, ClassId>,
    plans: Vec<ClassPlan>,
    /// DefaultClassResolver's name ids, reset between roots.
    name_ids: HashMap<i32, ClassId>,
    /// MapReferenceResolver's read objects, reset between roots.
    references: Vec<Option<Value>>,
    string_ids: HashMap<String, u32>,
}

impl<'a> Reader<'a> {
    pub fn new(bytes: &'a [u8], schema: &'a Schema) -> Self {
        Reader {
            input: Input::new(bytes),
            schema,
            graph: Graph::default(),
            class_ids: HashMap::new(),
            plans: Vec::new(),
            name_ids: HashMap::new(),
            references: Vec::new(),
            string_ids: HashMap::new(),
        }
    }

    /// Reads `writeClassAndObject` roots until the end of the data.
    pub fn read_all(mut self) -> Result<Graph, KryoError> {
        while !self.input.is_at_end() {
            let root = self.read_class_and_object()?;
            self.graph.roots.push(root);
            // Kryo.reset() after every top-level call (autoReset)
            self.name_ids.clear();
            self.references.clear();
        }
        Ok(self.graph)
    }

    fn class_id(&mut self, name: &str) -> Result<ClassId, KryoError> {
        if let Some(&id) = self.class_ids.get(name) {
            return Ok(id);
        }
        let schema = self.schema.classes.get(name).ok_or_else(|| self.input.error(format!("class not in schema: {name}")))?;
        let id = self.plans.len() as ClassId;
        // reserve the id first: field plans may refer back to this class
        self.class_ids.insert(name.to_string(), id);
        self.plans.push(ClassPlan { kind: Rc::new(SerializerKind::Unsupported(String::new())) });
        self.graph.classes.push(GraphClass {
            name: name.to_string(),
            fields: schema.fields.iter().map(|f| f.name.clone()).collect(),
        });
        let kind = match schema.serializer.as_str() {
            "field" => {
                let mut plan = Vec::with_capacity(schema.fields.len());
                for field in &schema.fields {
                    plan.push(match field.kind.as_str() {
                        "object" => match &field.value_class {
                            Some(value_class) => FieldPlan::FinalObject(self.class_id(value_class)?),
                            None => FieldPlan::AnyObject,
                        },
                        primitive => FieldPlan::Primitive(primitive_named(primitive).ok_or_else(|| {
                            self.input.error(format!("unknown primitive {primitive} in {name}"))
                        })?),
                    });
                }
                SerializerKind::Fields(plan)
            }
            "collection" => SerializerKind::Collection,
            "map" => SerializerKind::Map,
            "string" => SerializerKind::String,
            "calendar" => SerializerKind::Calendar,
            "enum" => SerializerKind::Enum,
            "string-array" => SerializerKind::StringArray,
            "object-array" => {
                let component = schema.component.clone().unwrap_or_default();
                // ObjectArraySerializer tests Modifier.isFinal on the component class itself, and
                // array classes are always final.
                let same_type = component.starts_with('[')
                    || self.schema.classes.get(&component).is_some_and(|c| c.is_final);
                SerializerKind::ObjectArray { component: self.class_id(&component)?, same_type }
            }
            other => {
                if let Some(primitive) = other.strip_suffix("-array").and_then(primitive_named) {
                    SerializerKind::PrimitiveArray(primitive)
                } else if let Some(primitive) = primitive_named(other) {
                    SerializerKind::Boxed(primitive)
                } else {
                    SerializerKind::Unsupported(other.to_string())
                }
            }
        };
        self.plans[id as usize].kind = Rc::new(kind);
        Ok(id)
    }

    /// `DefaultClassResolver.readClass`.
    fn read_class(&mut self) -> Result<Option<ClassId>, KryoError> {
        match self.input.read_var_int(true)? {
            0 => Ok(None),
            1 => {
                let name_id = self.input.read_var_int(true)?;
                if let Some(&class) = self.name_ids.get(&name_id) {
                    return Ok(Some(class));
                }
                let name = self.input.read_string()?.ok_or_else(|| self.input.error("null class name"))?;
                let class = self.class_id(&name)?;
                self.name_ids.insert(name_id, class);
                Ok(Some(class))
            }
            id => {
                let name = REGISTERED
                    .get((id - 2) as usize)
                    .ok_or_else(|| self.input.error(format!("unregistered class id {}", id - 2)))?;
                Ok(Some(self.class_id(name)?))
            }
        }
    }

    fn uses_references(&self, class: ClassId) -> bool {
        // MapReferenceResolver.useReferences: everything but the primitive wrappers
        !matches!(*self.plans[class as usize].kind, SerializerKind::Boxed(_))
    }

    /// `Kryo.readReferenceOrNull`.
    fn read_reference_or_null(&mut self, class: ClassId, may_be_null: bool) -> Result<ReferenceRead, KryoError> {
        let references_supported = self.uses_references(class);
        let id;
        if may_be_null {
            id = self.input.read_var_int(true)?;
            if id == 0 {
                return Ok(ReferenceRead::Null);
            }
            if !references_supported {
                return Ok(ReferenceRead::New(None));
            }
        } else {
            if !references_supported {
                return Ok(ReferenceRead::New(None));
            }
            id = self.input.read_var_int(true)?;
        }
        if id == 1 {
            self.references.push(None);
            return Ok(ReferenceRead::New(Some(self.references.len() - 1)));
        }
        let index = (id - 2) as usize;
        match self.references.get(index) {
            Some(Some(value)) => Ok(ReferenceRead::Existing(*value)),
            _ => Err(self.input.error(format!("reference {index} to an object not read yet"))),
        }
    }

    pub fn read_class_and_object(&mut self) -> Result<Value, KryoError> {
        match self.read_class()? {
            None => Ok(Value::Null),
            Some(class) => self.read_object(class),
        }
    }

    /// `Kryo.readObject` (never null).
    fn read_object(&mut self, class: ClassId) -> Result<Value, KryoError> {
        match self.read_reference_or_null(class, false)? {
            ReferenceRead::Null => Ok(Value::Null),
            ReferenceRead::Existing(value) => Ok(value),
            ReferenceRead::New(slot) => self.read_new(class, slot),
        }
    }

    /// `Kryo.readObjectOrNull`.
    fn read_object_or_null(&mut self, class: ClassId) -> Result<Value, KryoError> {
        match self.read_reference_or_null(class, true)? {
            ReferenceRead::Null => Ok(Value::Null),
            ReferenceRead::Existing(value) => Ok(value),
            ReferenceRead::New(slot) => self.read_new(class, slot),
        }
    }

    fn new_node(&mut self, class: ClassId, slot: Option<usize>) -> NodeId {
        let id = self.graph.nodes.len() as NodeId;
        self.graph.nodes.push(Node { class, data: NodeData::Items(Vec::new()) });
        // FieldSerializer, CollectionSerializer, MapSerializer and ObjectArraySerializer call
        // kryo.reference(object) before reading their contents, so cycles resolve.
        self.set_reference(slot, Value::Node(id));
        id
    }

    fn set_reference(&mut self, slot: Option<usize>, value: Value) {
        if let Some(slot) = slot {
            self.references[slot] = Some(value);
        }
    }

    fn intern(&mut self, s: String) -> Value {
        if let Some(&id) = self.string_ids.get(&s) {
            return Value::Str(id);
        }
        let id = self.graph.strings.len() as u32;
        self.string_ids.insert(s.clone(), id);
        self.graph.strings.push(s);
        Value::Str(id)
    }

    /// The serializer's `read` for a new object of `class`.
    fn read_new(&mut self, class: ClassId, slot: Option<usize>) -> Result<Value, KryoError> {
        let kind = Rc::clone(&self.plans[class as usize].kind);
        self.read_with(class, slot, &kind)
    }

    fn read_with(&mut self, class: ClassId, slot: Option<usize>, kind: &SerializerKind) -> Result<Value, KryoError> {
        let value = match kind {
            SerializerKind::Fields(plan) => {
                let id = self.new_node(class, slot);
                let mut values = Vec::with_capacity(plan.len());
                for field in plan {
                    values.push(match *field {
                        FieldPlan::Primitive(p) => self.read_primitive(p)?,
                        FieldPlan::FinalObject(value_class) => self.read_object_or_null(value_class)?,
                        FieldPlan::AnyObject => self.read_class_and_object()?,
                    });
                }
                self.graph.nodes[id as usize].data = NodeData::Object(values);
                return Ok(Value::Node(id));
            }
            SerializerKind::Collection => {
                let id = self.new_node(class, slot);
                let length = self.input.read_var_int(true)? as usize;
                let mut items = Vec::with_capacity(length.min(1 << 20));
                for _ in 0..length {
                    items.push(self.read_class_and_object()?);
                }
                self.graph.nodes[id as usize].data = NodeData::Items(items);
                return Ok(Value::Node(id));
            }
            SerializerKind::Map => {
                let length = self.input.read_var_int(true)? as usize;
                let id = self.new_node(class, slot);
                let mut entries = Vec::with_capacity(length.min(1 << 20));
                for _ in 0..length {
                    let key = self.read_class_and_object()?;
                    let value = self.read_class_and_object()?;
                    entries.push((key, value));
                }
                self.graph.nodes[id as usize].data = NodeData::Map(entries);
                return Ok(Value::Node(id));
            }
            SerializerKind::ObjectArray { component, same_type } => {
                let length = self.input.read_var_int(true)?;
                if length == 0 {
                    return Ok(Value::Null);
                }
                let id = self.new_node(class, slot);
                let mut items = Vec::with_capacity((length - 1) as usize);
                for _ in 1..length {
                    items.push(if *same_type {
                        self.read_object_or_null(*component)?
                    } else {
                        self.read_class_and_object()?
                    });
                }
                self.graph.nodes[id as usize].data = NodeData::Items(items);
                return Ok(Value::Node(id));
            }
            SerializerKind::String => match self.input.read_string()? {
                Some(s) => self.intern(s),
                None => Value::Null,
            },
            SerializerKind::Boxed(p) => self.read_boxed(*p)?,
            SerializerKind::Enum => {
                let ordinal = self.input.read_var_int(true)?;
                if ordinal == 0 { Value::Null } else { Value::Enum { class, ordinal: (ordinal - 1) as u32 } }
            }
            SerializerKind::Calendar => {
                let time_zone = self.input.read_string()?.unwrap_or_default();
                let calendar = Calendar {
                    time_zone,
                    millis: self.input.read_var_long(true)?,
                    lenient: self.input.read_boolean()?,
                    first_day_of_week: self.input.read_var_int(true)?,
                    minimal_days_in_first_week: self.input.read_var_int(true)?,
                    gregorian_change: self.input.read_var_long(false)?,
                };
                self.push_node(class, NodeData::Calendar(calendar))
            }
            SerializerKind::StringArray => {
                let length = self.input.read_var_int(true)?;
                if length == 0 {
                    Value::Null
                } else {
                    let string_class = self.class_id("java.lang.String")?;
                    let mut items = Vec::with_capacity((length - 1) as usize);
                    for _ in 1..length {
                        items.push(self.read_object_or_null(string_class)?);
                    }
                    self.push_node(class, NodeData::Items(items))
                }
            }
            SerializerKind::PrimitiveArray(p) => {
                let length = self.input.read_var_int(true)?;
                if length == 0 {
                    Value::Null
                } else {
                    let data = self.read_primitive_array(*p, (length - 1) as usize)?;
                    self.push_node(class, data)
                }
            }
            SerializerKind::Unsupported(serializer) => {
                let name = self.graph.class_name(class).to_string();
                return Err(self.input.error(format!("no reader for {name} (serializer {serializer})")));
            }
        };
        // serializers that do not call kryo.reference themselves: Kryo references the result
        if value != Value::Null {
            self.set_reference(slot, value);
        }
        Ok(value)
    }

    fn push_node(&mut self, class: ClassId, data: NodeData) -> Value {
        let id = self.graph.nodes.len() as NodeId;
        self.graph.nodes.push(Node { class, data });
        Value::Node(id)
    }

    /// Primitive fields (UnsafeCacheFields, var ints enabled).
    fn read_primitive(&mut self, p: Primitive) -> Result<Value, KryoError> {
        let input = &mut self.input;
        Ok(match p {
            Primitive::Int => Value::Int(input.read_var_int(false)?),
            Primitive::Long => Value::Long(input.read_var_long(false)?),
            Primitive::Double => Value::Double(input.read_double()?),
            Primitive::Float => Value::Float(input.read_float()?),
            Primitive::Boolean => Value::Bool(input.read_boolean()?),
            Primitive::Short => Value::Short(input.read_short()?),
            Primitive::Byte => Value::Byte(input.read_byte()?),
            Primitive::Char => Value::Char(input.read_char()?),
        })
    }

    /// Boxed values (DefaultSerializers): the same encodings as fields.
    fn read_boxed(&mut self, p: Primitive) -> Result<Value, KryoError> {
        self.read_primitive(p)
    }

    fn read_primitive_array(&mut self, p: Primitive, length: usize) -> Result<NodeData, KryoError> {
        let input = &mut self.input;
        Ok(match p {
            Primitive::Int => NodeData::Ints((0..length).map(|_| input.read_var_int(false)).collect::<Result<_, _>>()?),
            Primitive::Long => NodeData::Longs((0..length).map(|_| input.read_var_long(false)).collect::<Result<_, _>>()?),
            Primitive::Double => NodeData::Doubles((0..length).map(|_| input.read_double()).collect::<Result<_, _>>()?),
            Primitive::Float => NodeData::Floats((0..length).map(|_| input.read_float()).collect::<Result<_, _>>()?),
            Primitive::Boolean => NodeData::Booleans((0..length).map(|_| input.read_boolean()).collect::<Result<_, _>>()?),
            Primitive::Short => NodeData::Shorts((0..length).map(|_| input.read_short()).collect::<Result<_, _>>()?),
            Primitive::Byte => NodeData::Bytes((0..length).map(|_| input.read_byte()).collect::<Result<_, _>>()?),
            Primitive::Char => NodeData::Chars((0..length).map(|_| input.read_char()).collect::<Result<_, _>>()?),
        })
    }
}

fn primitive_named(name: &str) -> Option<Primitive> {
    Some(match name {
        "int" => Primitive::Int,
        "long" => Primitive::Long,
        "double" => Primitive::Double,
        "float" => Primitive::Float,
        "boolean" => Primitive::Boolean,
        "short" => Primitive::Short,
        "byte" => Primitive::Byte,
        "char" => Primitive::Char,
        _ => return None,
    })
}
