use indexmap::IndexMap;
use std::cell::RefCell;
/// Basalt Value - Runtime value representation.
///
/// Conceptually, all values occupy 8-byte register slots. At the bytecode
/// level, each instruction encodes how to interpret its operands (the VM never
/// inspects a value to determine its type). However, for memory safety in the
/// Rust host, we use an enum to distinguish inline values from heap pointers.
/// This is an implementation detail of the Rust host, not a property of the
/// language - the Basalt programmer never sees runtime type tags.
use std::sync::Arc;

/// A value slot. Inline for primitives, heap-allocated for compound types.
#[derive(Clone, Debug)]
pub enum Value {
    /// 64-bit integer (used for all integer types, widened to i64 in registers)
    Int(i64),
    /// IEEE 754 double-precision float
    Float(f64),
    /// Boolean
    Bool(bool),
    /// Nil (unit value)
    Nil,
    /// Heap-allocated object (string, array, map, struct, enum, etc.)
    Heap(HeapRef),
}

/// Heap-allocated object types.
#[derive(Debug, Clone)]
pub enum HeapObject {
    String(String),
    Array(Vec<Value>),
    Map(IndexMap<MapKey, Value>),
    Tuple(Vec<Value>),
    Struct(StructObj),
    Enum(EnumObj),
    Error(Box<Value>),
    Range(i64, i64),
    Iterator(IterState),
    Closure(ClosureObj),
    /// A shared mutable cell for capture-by-reference closures.
    CaptureCell(Box<Value>),
}

#[derive(Debug, Clone)]
pub struct ClosureObj {
    pub func_idx: usize,
    pub captures: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct StructObj {
    pub type_name: String,
    pub fields: Vec<Value>,
}

#[derive(Debug, Clone)]
pub struct EnumObj {
    pub type_name: String,
    pub variant_index: u8,
    pub fields: Vec<Value>,
}

#[derive(Debug, Clone)]
pub enum IterState {
    Array {
        source: HeapRef,  // Arc ref to the original array
        index: usize,
    },
    Map {
        keys: Vec<MapKey>,     // keys snapshot (needed for stable iteration order)
        source: HeapRef,       // Arc ref to the original map
        index: usize,
    },
    String {
        chars: Vec<String>,  // strings are immutable value types, we need the char vec
        index: usize,
    },
    Range {
        current: i64,
        end: i64,
    },
}

/// Map keys (hashable types).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MapKey {
    Int(i64),
    Bool(bool),
    String(String),
}

/// A reference-counted pointer to a heap object.
pub type HeapRef = Arc<RefCell<HeapObject>>;

impl Value {
    pub fn int(n: i64) -> Value {
        Value::Int(n)
    }

    pub fn float(f: f64) -> Value {
        Value::Float(f)
    }

    pub fn bool(b: bool) -> Value {
        Value::Bool(b)
    }

    pub fn as_int(&self) -> i64 {
        match self {
            Value::Int(n) => *n,
            _ => panic!("expected Int, got {:?}", self.type_tag()),
        }
    }

    pub fn as_float(&self) -> f64 {
        match self {
            Value::Float(f) => *f,
            _ => panic!("expected Float, got {:?}", self.type_tag()),
        }
    }

    pub fn as_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            _ => panic!("expected Bool, got {:?}", self.type_tag()),
        }
    }

    pub fn type_tag(&self) -> &'static str {
        match self {
            Value::Int(_) => "Int",
            Value::Float(_) => "Float",
            Value::Bool(_) => "Bool",
            Value::Nil => "Nil",
            Value::Heap(_) => "Heap",
        }
    }

    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }

    pub fn as_heap_ref(&self) -> Option<&HeapRef> {
        match self {
            Value::Heap(href) => Some(href),
            _ => None,
        }
    }

    pub fn string(s: String) -> Value {
        Value::Heap(Arc::new(RefCell::new(HeapObject::String(s))))
    }

    pub fn array(vals: Vec<Value>) -> Value {
        Value::Heap(Arc::new(RefCell::new(HeapObject::Array(vals))))
    }

    pub fn map(entries: IndexMap<MapKey, Value>) -> Value {
        Value::Heap(Arc::new(RefCell::new(HeapObject::Map(entries))))
    }

    pub fn tuple(vals: Vec<Value>) -> Value {
        Value::Heap(Arc::new(RefCell::new(HeapObject::Tuple(vals))))
    }

    pub fn new_struct(type_name: String, fields: Vec<Value>) -> Value {
        Value::Heap(Arc::new(RefCell::new(HeapObject::Struct(StructObj {
            type_name,
            fields,
        }))))
    }

    pub fn new_enum(type_name: String, variant_index: u8, fields: Vec<Value>) -> Value {
        Value::Heap(Arc::new(RefCell::new(HeapObject::Enum(EnumObj {
            type_name,
            variant_index,
            fields,
        }))))
    }

    pub fn error(val: Value) -> Value {
        Value::Heap(Arc::new(RefCell::new(HeapObject::Error(Box::new(val)))))
    }

    pub fn closure(func_idx: usize, captures: Vec<Value>) -> Value {
        Value::Heap(Arc::new(RefCell::new(HeapObject::Closure(ClosureObj {
            func_idx,
            captures,
        }))))
    }

    pub fn range(start: i64, end: i64) -> Value {
        Value::Heap(Arc::new(RefCell::new(HeapObject::Range(start, end))))
    }

    pub fn is_error(&self) -> bool {
        if let Value::Heap(href) = self {
            matches!(&*href.borrow(), HeapObject::Error(_))
        } else {
            false
        }
    }

    /// Deep structural equality.
    pub fn deep_eq(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Heap(a), Value::Heap(b)) => {
                // Same reference?
                if Arc::ptr_eq(a, b) {
                    return true;
                }
                heap_obj_eq(&a.borrow(), &b.borrow())
            }
            _ => false,
        }
    }

    /// Convert value to display string.
    pub fn display_as_string(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => format_float(*f),
            Value::Bool(b) => {
                if *b {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            Value::Nil => "nil".to_string(),
            Value::Heap(href) => {
                let obj = href.borrow();
                match &*obj {
                    HeapObject::String(s) => s.clone(),
                    HeapObject::Array(vals) => {
                        let parts: Vec<String> =
                            vals.iter().map(|v| v.display_as_string()).collect();
                        format!("[{}]", parts.join(", "))
                    }
                    HeapObject::Map(entries) => {
                        let parts: Vec<String> = entries
                            .iter()
                            .map(|(k, v)| {
                                format!("{}: {}", format_map_key(k), v.display_as_string())
                            })
                            .collect();
                        format!("{{{}}}", parts.join(", "))
                    }
                    HeapObject::Tuple(vals) => {
                        let parts: Vec<String> =
                            vals.iter().map(|v| v.display_as_string()).collect();
                        format!("({})", parts.join(", "))
                    }
                    HeapObject::Struct(s) => format!("{} {{ ... }}", s.type_name),
                    HeapObject::Enum(e) => format!("{}::variant_{}", e.type_name, e.variant_index),
                    HeapObject::Error(inner) => format!("Error({})", inner.display_as_string()),
                    HeapObject::Range(s, e) => format!("{}..{}", s, e),
                    HeapObject::Closure(c) => format!("<closure func_{}>", c.func_idx),
                    HeapObject::CaptureCell(val) => val.display_as_string(),
                    HeapObject::Iterator(_) => "<iterator>".to_string(),
                }
            }
        }
    }
}

pub fn format_float(f: f64) -> String {
    if f == f.floor() && f.is_finite() && f.abs() < 1e15 {
        format!("{:.1}", f)
    } else {
        format!("{}", f)
    }
}

fn format_map_key(key: &MapKey) -> String {
    match key {
        MapKey::Int(n) => n.to_string(),
        MapKey::Bool(b) => b.to_string(),
        MapKey::String(s) => format!("\"{}\"", s),
    }
}

fn heap_obj_eq(a: &HeapObject, b: &HeapObject) -> bool {
    match (a, b) {
        (HeapObject::String(a), HeapObject::String(b)) => a == b,
        (HeapObject::Array(a), HeapObject::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.deep_eq(y))
        }
        (HeapObject::Map(a), HeapObject::Map(b)) => {
            if a.len() != b.len() {
                return false;
            }
            for (k, v) in a.iter() {
                match b.get(k) {
                    Some(bv) => {
                        if !v.deep_eq(bv) {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            true
        }
        (HeapObject::Tuple(a), HeapObject::Tuple(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.deep_eq(y))
        }
        (HeapObject::Struct(a), HeapObject::Struct(b)) => {
            a.type_name == b.type_name
                && a.fields.len() == b.fields.len()
                && a.fields
                    .iter()
                    .zip(b.fields.iter())
                    .all(|(x, y)| x.deep_eq(y))
        }
        (HeapObject::Enum(a), HeapObject::Enum(b)) => {
            a.type_name == b.type_name
                && a.variant_index == b.variant_index
                && a.fields
                    .iter()
                    .zip(b.fields.iter())
                    .all(|(x, y)| x.deep_eq(y))
        }
        _ => false,
    }
}

pub fn map_key_to_value(key: &MapKey) -> Value {
    match key {
        MapKey::Int(n) => Value::int(*n),
        MapKey::Bool(b) => Value::bool(*b),
        MapKey::String(s) => Value::string(s.clone()),
    }
}
