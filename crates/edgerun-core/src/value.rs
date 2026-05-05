use crate::prelude::v1::*;

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

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Value::as_bool ----

    #[test]
    fn as_bool_returns_some_for_bool() {
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Bool(false).as_bool(), Some(false));
    }

    #[test]
    fn as_bool_returns_none_for_non_bool() {
        assert!(Value::Null.as_bool().is_none());
        assert!(Value::Int(0).as_bool().is_none());
        assert!(Value::String("true".into()).as_bool().is_none());
        assert!(Value::Seq(vec![]).as_bool().is_none());
        assert!(Value::Map(Default::default()).as_bool().is_none());
    }

    // ---- Value::as_i64 ----

    #[test]
    fn as_i64_returns_some_for_int() {
        assert_eq!(Value::Int(42).as_i64(), Some(42));
        assert_eq!(Value::Int(-1).as_i64(), Some(-1));
        assert_eq!(Value::Int(0).as_i64(), Some(0));
    }

    #[test]
    fn as_i64_returns_none_for_non_int() {
        assert!(Value::Null.as_i64().is_none());
        assert!(Value::Bool(true).as_i64().is_none());
        assert!(Value::String("42".into()).as_i64().is_none());
        assert!(Value::Seq(vec![]).as_i64().is_none());
        assert!(Value::Map(Default::default()).as_i64().is_none());
    }

    // ---- Value::as_str ----

    #[test]
    fn as_str_returns_some_for_string() {
        assert_eq!(Value::String("hello".into()).as_str(), Some("hello"));
        assert_eq!(Value::String("".into()).as_str(), Some(""));
    }

    #[test]
    fn as_str_returns_none_for_non_string() {
        assert!(Value::Null.as_str().is_none());
        assert!(Value::Bool(true).as_str().is_none());
        assert!(Value::Int(0).as_str().is_none());
        assert!(Value::Seq(vec![]).as_str().is_none());
        assert!(Value::Map(Default::default()).as_str().is_none());
    }

    // ---- Value::as_seq ----

    #[test]
    fn as_seq_returns_some_for_seq() {
        let v = Value::Seq(vec![Value::Int(1), Value::Int(2)]);
        assert_eq!(v.as_seq(), Some(&[Value::Int(1), Value::Int(2)][..]));
    }

    #[test]
    fn as_seq_returns_none_for_non_seq() {
        assert!(Value::Null.as_seq().is_none());
        assert!(Value::Bool(true).as_seq().is_none());
        assert!(Value::Int(0).as_seq().is_none());
        assert!(Value::String("".into()).as_seq().is_none());
        assert!(Value::Map(Default::default()).as_seq().is_none());
    }

    // ---- Value::as_map ----

    #[test]
    fn as_map_returns_some_for_map() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::Int(1));
        let v = Value::Map(m.clone());
        assert_eq!(v.as_map(), Some(&m));
    }

    #[test]
    fn as_map_returns_none_for_non_map() {
        assert!(Value::Null.as_map().is_none());
        assert!(Value::Bool(true).as_map().is_none());
        assert!(Value::Int(0).as_map().is_none());
        assert!(Value::String("".into()).as_map().is_none());
        assert!(Value::Seq(vec![]).as_map().is_none());
    }

    // ---- Constructor helpers ----

    #[test]
    fn ystr_creates_string_value() {
        assert_eq!(ystr("hello"), Value::String("hello".into()));
    }

    #[test]
    fn ybool_creates_bool_value() {
        assert_eq!(ybool(true), Value::Bool(true));
        assert_eq!(ybool(false), Value::Bool(false));
    }

    #[test]
    fn yi64_creates_int_value() {
        assert_eq!(yi64(42), Value::Int(42));
        assert_eq!(yi64(-1), Value::Int(-1));
    }

    #[test]
    fn ynull_creates_null_value() {
        assert_eq!(ynull(), Value::Null);
    }

    #[test]
    fn seq_creates_seq_value() {
        let s = seq([yi64(1), yi64(2), yi64(3)]);
        assert_eq!(
            s,
            Value::Seq(vec![Value::Int(1), Value::Int(2), Value::Int(3)])
        );
    }

    #[test]
    fn seq_empty() {
        let s = seq([] as [Value; 0]);
        assert_eq!(s, Value::Seq(vec![]));
    }

    #[test]
    fn mapping_creates_map_value() {
        let m = mapping([("a", yi64(1)), ("b", ystr("hello"))]);
        let mut expected = BTreeMap::new();
        expected.insert("a".into(), Value::Int(1));
        expected.insert("b".into(), Value::String("hello".into()));
        assert_eq!(m, Value::Map(expected));
    }

    #[test]
    fn mapping_empty() {
        let m: Value = mapping([] as [(String, Value); 0]);
        assert_eq!(m, Value::Map(BTreeMap::new()));
    }

    // ---- value_to_string ----

    #[test]
    fn value_to_string_null() {
        assert_eq!(value_to_string(&Value::Null), "<nil>");
    }

    #[test]
    fn value_to_string_bool() {
        assert_eq!(value_to_string(&Value::Bool(true)), "true");
        assert_eq!(value_to_string(&Value::Bool(false)), "false");
    }

    #[test]
    fn value_to_string_int() {
        assert_eq!(value_to_string(&Value::Int(42)), "42");
        assert_eq!(value_to_string(&Value::Int(-100)), "-100");
    }

    #[test]
    fn value_to_string_string() {
        assert_eq!(value_to_string(&Value::String("hello".into())), "hello");
    }

    #[test]
    fn value_to_string_seq() {
        let s = Value::Seq(vec![Value::Int(1), Value::Int(2)]);
        assert_eq!(value_to_string(&s), "[Int(1), Int(2)]");
    }

    #[test]
    fn value_to_string_map() {
        let mut m = BTreeMap::new();
        m.insert("a".into(), Value::Int(1));
        assert_eq!(value_to_string(&Value::Map(m)), r#"{"a": Int(1)}"#);
    }

    // ---- Clone / Debug / PartialEq / Eq ----

    #[test]
    fn value_clone() {
        let v = Value::Seq(vec![Value::Int(1), Value::Bool(true)]);
        let v2 = v.clone();
        assert_eq!(v, v2);
    }

    #[test]
    fn value_equality() {
        assert_eq!(Value::Null, Value::Null);
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_eq!(Value::String("x".into()), Value::String("x".into()));
        assert_eq!(
            Value::Seq(vec![Value::Int(1)]),
            Value::Seq(vec![Value::Int(1)])
        );
    }

    #[test]
    fn value_inequality() {
        assert_ne!(Value::Null, Value::Bool(true));
        assert_ne!(Value::Bool(true), Value::Bool(false));
        assert_ne!(Value::Int(0), Value::Int(1));
        assert_ne!(Value::String("a".into()), Value::String("b".into()));
        assert_ne!(Value::Seq(vec![]), Value::Seq(vec![Value::Int(1)]));
    }

    #[test]
    fn value_debug() {
        let v = Value::Int(42);
        let debug_str = format!("{:?}", v);
        assert!(debug_str.contains("42"));
    }
}
