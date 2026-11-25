//! The types a param or an action argument can have.

use serde::Serialize;
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ValueType {
    String,
    Int,
    Number,
    Bool,
    List,
    Object,
    Duration,
    Any,
}

impl ValueType {
    pub const ALL: &'static [ValueType] = &[
        ValueType::String,
        ValueType::Int,
        ValueType::Number,
        ValueType::Bool,
        ValueType::List,
        ValueType::Object,
        ValueType::Duration,
        ValueType::Any,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ValueType::String => "string",
            ValueType::Int => "int",
            ValueType::Number => "number",
            ValueType::Bool => "bool",
            ValueType::List => "list",
            ValueType::Object => "object",
            ValueType::Duration => "duration",
            ValueType::Any => "any",
        }
    }

    pub fn parse(name: &str) -> Option<ValueType> {
        Self::ALL.iter().copied().find(|ty| ty.name() == name)
    }

    /// Whether a literal value fits. A string with `${ }` holes is checked
    /// when it is filled in, not here.
    pub fn accepts(self, value: &Value) -> bool {
        match self {
            ValueType::String => value.is_string(),
            ValueType::Int => value.is_i64() || value.is_u64(),
            ValueType::Number => value.is_number(),
            ValueType::Bool => value.is_boolean(),
            ValueType::List => value.is_array(),
            ValueType::Object => value.is_object(),
            ValueType::Duration => value.as_str().is_some_and(|text| crate::duration::parse(text).is_ok()),
            ValueType::Any => true,
        }
    }

    /// The type a literal implies.
    pub fn of(value: &Value) -> ValueType {
        match value {
            Value::String(_) => ValueType::String,
            Value::Bool(_) => ValueType::Bool,
            Value::Number(n) if n.is_f64() => ValueType::Number,
            Value::Number(_) => ValueType::Int,
            Value::Array(_) => ValueType::List,
            Value::Object(_) => ValueType::Object,
            Value::Null => ValueType::Any,
        }
    }
}
