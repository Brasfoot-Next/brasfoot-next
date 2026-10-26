//! Reader of Java serialization streams (`ObjectOutputStream`), the format of the game's data
//! files: the team files (`teams/*.ban`, `e.t`), the league configurations
//! (`conf_ligas_nacionais/*.cfg`, `est.ArrayLigaType`) and the options (`options.bcf`).
//!
//! It turns a stream into a tree of [`Value`]s without running any game code. Objects keep their
//! serialized fields by name; classes with a `writeObject` keep what it wrote after the fields
//! (`ArrayList` writes its elements there, which [`Value::list`] reads).

use std::fmt;
use std::rc::Rc;

const STREAM_MAGIC: u16 = 0xaced;
const TC_NULL: u8 = 0x70;
const TC_REFERENCE: u8 = 0x71;
const TC_CLASSDESC: u8 = 0x72;
const TC_OBJECT: u8 = 0x73;
const TC_STRING: u8 = 0x74;
const TC_ARRAY: u8 = 0x75;
const TC_CLASS: u8 = 0x76;
const TC_BLOCKDATA: u8 = 0x77;
const TC_ENDBLOCKDATA: u8 = 0x78;
const TC_RESET: u8 = 0x79;
const TC_BLOCKDATALONG: u8 = 0x7a;
const TC_LONGSTRING: u8 = 0x7c;
const TC_ENUM: u8 = 0x7e;
const BASE_HANDLE: u32 = 0x7e0000;
const SC_WRITE_METHOD: u8 = 0x01;

#[derive(Debug, Clone)]
pub struct JavaSerError {
    pub message: String,
    pub position: usize,
}

impl fmt::Display for JavaSerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "java serialization: {} (at byte {})", self.message, self.position)
    }
}

impl std::error::Error for JavaSerError {}

/// A value of the stream. Objects and arrays are shared (`Rc`) as the stream's back references are.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Byte(i8),
    Char(u16),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    String(Rc<str>),
    Object(Rc<Object>),
    Array(Rc<Vec<Value>>),
    /// A class literal or an enum constant (its name).
    Class(Rc<str>),
    /// What a `writeObject` wrote outside objects (`TC_BLOCKDATA`).
    Block(Rc<Vec<u8>>),
}

/// An object: its class and its serialized fields, from the top superclass down, and what the
/// classes' `writeObject` methods wrote after them.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    pub class: Rc<str>,
    pub fields: Vec<(Rc<str>, Value)>,
    pub annotations: Vec<Value>,
}

impl Value {
    pub fn as_object(&self) -> Option<&Object> {
        if let Value::Object(o) = self { Some(o) } else { None }
    }

    pub fn as_int(&self) -> Option<i32> {
        if let Value::Int(i) = self { Some(*i) } else { None }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Value::Bool(b) = self { Some(*b) } else { None }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Value::String(s) = self { Some(s) } else { None }
    }

    /// The elements of a `java.util.ArrayList` (written by its `writeObject` after the size).
    pub fn list(&self) -> Vec<Value> {
        match self {
            Value::Object(o) if o.class.as_ref() == "java.util.ArrayList" => {
                o.annotations.iter().filter(|v| !matches!(v, Value::Block(_))).cloned().collect()
            }
            Value::Array(items) => items.as_ref().clone(),
            _ => Vec::new(),
        }
    }
}

impl Object {
    pub fn field(&self, name: &str) -> Option<&Value> {
        self.fields.iter().rev().find(|(n, _)| n.as_ref() == name).map(|(_, v)| v)
    }

    pub fn int(&self, name: &str) -> i32 {
        self.field(name).and_then(Value::as_int).unwrap_or(0)
    }

    pub fn bool(&self, name: &str) -> bool {
        self.field(name).and_then(Value::as_bool).unwrap_or(false)
    }

    /// A `String` field; `None` when null.
    pub fn string(&self, name: &str) -> Option<String> {
        self.field(name).and_then(Value::as_str).map(str::to_owned)
    }
}

#[derive(Debug)]
struct ClassDesc {
    name: Rc<str>,
    flags: u8,
    fields: Vec<(u8, Rc<str>)>,
    superclass: Option<Rc<ClassDesc>>,
}

#[derive(Debug, Clone)]
enum Handle {
    Value(Value),
    Desc(Rc<ClassDesc>),
    /// An object being read (a reference to it from inside itself is not supported).
    Pending,
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    handles: Vec<Handle>,
}

impl<'a> Reader<'a> {
    fn error<T>(&self, message: impl Into<String>) -> Result<T, JavaSerError> {
        Err(JavaSerError { message: message.into(), position: self.pos })
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], JavaSerError> {
        if self.pos + n > self.bytes.len() {
            return self.error("unexpected end of stream");
        }
        let s = &self.bytes[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn u8(&mut self) -> Result<u8, JavaSerError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, JavaSerError> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn i32(&mut self) -> Result<i32, JavaSerError> {
        let b = self.take(4)?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn i64(&mut self) -> Result<i64, JavaSerError> {
        let b = self.take(8)?;
        Ok(i64::from_be_bytes(b.try_into().unwrap()))
    }

    /// Java's modified UTF-8.
    fn utf(&mut self, len: usize) -> Result<Rc<str>, JavaSerError> {
        let b = self.take(len)?;
        let mut units = Vec::with_capacity(len);
        let mut i = 0;
        while i < b.len() {
            let c = b[i] as u32;
            let unit = if c < 0x80 {
                i += 1;
                c
            } else if c & 0xe0 == 0xc0 && i + 1 < b.len() {
                let u = ((c & 0x1f) << 6) | (b[i + 1] as u32 & 0x3f);
                i += 2;
                u
            } else if c & 0xf0 == 0xe0 && i + 2 < b.len() {
                let u = ((c & 0x0f) << 12) | ((b[i + 1] as u32 & 0x3f) << 6) | (b[i + 2] as u32 & 0x3f);
                i += 3;
                u
            } else {
                return self.error("bad modified UTF-8");
            };
            units.push(unit as u16);
        }
        Ok(String::from_utf16_lossy(&units).into())
    }

    fn new_handle(&mut self, h: Handle) -> usize {
        self.handles.push(h);
        self.handles.len() - 1
    }

    fn handle(&mut self) -> Result<Handle, JavaSerError> {
        let n = self.i32()? as u32;
        let Some(i) = n.checked_sub(BASE_HANDLE) else { return self.error("bad handle") };
        match self.handles.get(i as usize) {
            Some(Handle::Pending) => self.error("a reference to an object from inside itself"),
            Some(h) => Ok(h.clone()),
            None => self.error(format!("unknown handle {n:#x}")),
        }
    }

    fn class_desc(&mut self) -> Result<Option<Rc<ClassDesc>>, JavaSerError> {
        match self.u8()? {
            TC_NULL => Ok(None),
            TC_REFERENCE => match self.handle()? {
                Handle::Desc(d) => Ok(Some(d)),
                _ => self.error("a reference that is not a class description"),
            },
            TC_CLASSDESC => {
                let len = self.u16()? as usize;
                let name = self.utf(len)?;
                let _uid = self.i64()?;
                let slot = self.new_handle(Handle::Pending);
                let flags = self.u8()?;
                let count = self.u16()?;
                let mut fields = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    let code = self.u8()?;
                    let len = self.u16()? as usize;
                    let field = self.utf(len)?;
                    if code == b'L' || code == b'[' {
                        // the field's class name, a string object
                        self.value()?;
                    }
                    fields.push((code, field));
                }
                self.skip_annotations()?;
                let superclass = self.class_desc()?;
                let desc = Rc::new(ClassDesc { name, flags, fields, superclass });
                self.handles[slot] = Handle::Desc(desc.clone());
                Ok(Some(desc))
            }
            other => self.error(format!("expected a class description, found {other:#x}")),
        }
    }

    fn skip_annotations(&mut self) -> Result<(), JavaSerError> {
        loop {
            if self.bytes.get(self.pos) == Some(&TC_ENDBLOCKDATA) {
                self.pos += 1;
                return Ok(());
            }
            self.value()?;
        }
    }

    fn annotations(&mut self) -> Result<Vec<Value>, JavaSerError> {
        let mut out = Vec::new();
        loop {
            if self.bytes.get(self.pos) == Some(&TC_ENDBLOCKDATA) {
                self.pos += 1;
                return Ok(out);
            }
            out.push(self.value()?);
        }
    }

    fn primitive(&mut self, code: u8) -> Result<Value, JavaSerError> {
        Ok(match code {
            b'Z' => Value::Bool(self.u8()? != 0),
            b'B' => Value::Byte(self.u8()? as i8),
            b'C' => Value::Char(self.u16()?),
            b'S' => Value::Short(self.u16()? as i16),
            b'I' => Value::Int(self.i32()?),
            b'J' => Value::Long(self.i64()?),
            b'F' => Value::Float(f32::from_bits(self.i32()? as u32)),
            b'D' => Value::Double(f64::from_bits(self.i64()? as u64)),
            b'L' | b'[' => self.value()?,
            other => return self.error(format!("unknown field type {:?}", other as char)),
        })
    }

    fn value(&mut self) -> Result<Value, JavaSerError> {
        let tag = self.u8()?;
        match tag {
            TC_NULL => Ok(Value::Null),
            TC_REFERENCE => match self.handle()? {
                Handle::Value(v) => Ok(v),
                _ => self.error("a reference that is not a value"),
            },
            TC_STRING => {
                let len = self.u16()? as usize;
                let s = Value::String(self.utf(len)?);
                self.new_handle(Handle::Value(s.clone()));
                Ok(s)
            }
            TC_LONGSTRING => {
                let len = self.i64()? as usize;
                let s = Value::String(self.utf(len)?);
                self.new_handle(Handle::Value(s.clone()));
                Ok(s)
            }
            TC_CLASS => {
                let desc = self.class_desc()?;
                let v = Value::Class(desc.map(|d| d.name.clone()).unwrap_or_else(|| "".into()));
                self.new_handle(Handle::Value(v.clone()));
                Ok(v)
            }
            TC_ENUM => {
                let _desc = self.class_desc()?;
                let slot = self.new_handle(Handle::Pending);
                let name = match self.value()? {
                    Value::String(s) => s,
                    _ => return self.error("an enum constant without its name"),
                };
                let v = Value::Class(name);
                self.handles[slot] = Handle::Value(v.clone());
                Ok(v)
            }
            TC_ARRAY => {
                let desc = self.class_desc()?.ok_or_else(|| JavaSerError { message: "array without class".into(), position: self.pos })?;
                let slot = self.new_handle(Handle::Pending);
                let len = self.i32()? as usize;
                let element = desc.name.as_bytes().get(1).copied().unwrap_or(b'L');
                let mut items = Vec::with_capacity(len.min(1 << 20));
                for _ in 0..len {
                    items.push(self.primitive(element)?);
                }
                let v = Value::Array(Rc::new(items));
                self.handles[slot] = Handle::Value(v.clone());
                Ok(v)
            }
            TC_OBJECT => {
                let desc = self.class_desc()?.ok_or_else(|| JavaSerError { message: "object without class".into(), position: self.pos })?;
                let slot = self.new_handle(Handle::Pending);
                let mut chain = Vec::new();
                let mut d = Some(desc.clone());
                while let Some(c) = d {
                    d = c.superclass.clone();
                    chain.push(c);
                }
                let mut fields = Vec::new();
                let mut annotations = Vec::new();
                for c in chain.iter().rev() {
                    for (code, name) in &c.fields {
                        fields.push((name.clone(), self.primitive(*code)?));
                    }
                    if c.flags & SC_WRITE_METHOD != 0 {
                        annotations.extend(self.annotations()?);
                    }
                }
                let v = Value::Object(Rc::new(Object { class: desc.name.clone(), fields, annotations }));
                self.handles[slot] = Handle::Value(v.clone());
                Ok(v)
            }
            TC_BLOCKDATA => {
                let len = self.u8()? as usize;
                Ok(Value::Block(Rc::new(self.take(len)?.to_vec())))
            }
            TC_BLOCKDATALONG => {
                let len = self.i32()? as usize;
                Ok(Value::Block(Rc::new(self.take(len)?.to_vec())))
            }
            TC_RESET => {
                self.handles.clear();
                self.value()
            }
            other => self.error(format!("unknown tag {other:#x}")),
        }
    }
}

/// Reads the first object of a stream.
pub fn read(bytes: &[u8]) -> Result<Value, JavaSerError> {
    let mut r = Reader { bytes, pos: 0, handles: Vec::new() };
    if r.u16()? != STREAM_MAGIC {
        return r.error("not a Java serialization stream");
    }
    let _version = r.u16()?;
    r.value()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `new java.util.ArrayList<>(List.of("Fulano", "Fulano"))` written by `ObjectOutputStream`:
    /// the second string is a back reference.
    #[test]
    fn reads_an_array_list_with_a_back_reference() {
        let bytes: &[u8] = &[
            0xac, 0xed, 0x00, 0x05, 0x73, 0x72, 0x00, 0x13, b'j', b'a', b'v', b'a', b'.', b'u', b't', b'i', b'l', b'.', b'A', b'r', b'r',
            b'a', b'y', b'L', b'i', b's', b't', 0x78, 0x81, 0xd2, 0x1d, 0x99, 0xc7, 0x61, 0x9d, 0x03, 0x00, 0x01, 0x49, 0x00, 0x04, b's',
            b'i', b'z', b'e', 0x78, 0x70, 0x00, 0x00, 0x00, 0x02, 0x77, 0x04, 0x00, 0x00, 0x00, 0x02, 0x74, 0x00, 0x06, b'F', b'u', b'l',
            b'a', b'n', b'o', 0x71, 0x00, 0x7e, 0x00, 0x02, 0x78,
        ];
        let v = read(bytes).unwrap();
        let items = v.list();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].as_str(), Some("Fulano"));
        assert_eq!(items[1].as_str(), Some("Fulano"));
        assert_eq!(v.as_object().unwrap().int("size"), 2);
    }
}
