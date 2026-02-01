//! Data models used throughout the application.

/// Represents a value in the system
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// An integer value
    Int(i64),
    /// A floating-point value
    Float(f64),
    /// A text value
    Text(String),
    /// A list of values
    List(Vec<Value>),
    /// A null/missing value
    Null,
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(n) => write!(f, "{}", n),
            Value::Text(s) => write!(f, "{}", s),
            Value::List(vs) => {
                write!(f, "[")?;
                for (i, v) in vs.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Null => write!(f, "null"),
        }
    }
}

/// A record with named fields
pub struct Record {
    pub fields: std::collections::HashMap<String, Value>,
}

impl Record {
    /// Create a new empty record
    pub fn new() -> Self {
        Record {
            fields: std::collections::HashMap::new(),
        }
    }

    /// Get a field value by name
    pub fn get(&self, name: &str) -> Option<&Value> {
        self.fields.get(name)
    }

    /// Set a field value
    pub fn set(&mut self, name: String, value: Value) {
        self.fields.insert(name, value);
    }
}

/// Convert from a string to a Value
impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::Text(s)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Int(n)
    }
}
