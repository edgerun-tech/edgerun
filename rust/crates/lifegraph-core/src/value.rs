use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    String(String),
    Seq(Vec<Value>),
    Map(BTreeMap<String, Value>),
}

impl Value {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Int(v) => Some(*v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_seq(&self) -> Option<&[Value]> {
        match self {
            Self::Seq(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_map(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Self::Map(v) => Some(v),
            _ => None,
        }
    }
}

pub fn ystr(s: impl Into<String>) -> Value {
    Value::String(s.into())
}
pub fn ybool(v: bool) -> Value {
    Value::Bool(v)
}
pub fn yi64(v: i64) -> Value {
    Value::Int(v)
}
pub fn ynull() -> Value {
    Value::Null
}
pub fn seq(values: impl IntoIterator<Item = Value>) -> Value {
    Value::Seq(values.into_iter().collect())
}

pub fn mapping(entries: impl IntoIterator<Item = (impl Into<String>, Value)>) -> Value {
    let mut out = BTreeMap::new();
    for (k, v) in entries {
        out.insert(k.into(), v);
    }
    Value::Map(out)
}

pub fn value_to_string(v: &Value) -> String {
    match v {
        Value::Null => "<nil>".to_string(),
        Value::Bool(v) => v.to_string(),
        Value::Int(v) => v.to_string(),
        Value::String(v) => v.clone(),
        Value::Seq(v) => format!("{v:?}"),
        Value::Map(v) => format!("{v:?}"),
    }
}
