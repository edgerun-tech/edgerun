use crate::prelude::*;

#[allow(unused_imports)]
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::string::String;
#[allow(unused_imports)]
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::vec::Vec;

use crate::JsonValue;
#[cfg(any(not(feature = "std"), target_os = "none"))]
use alloc::borrow::Cow;
#[cfg(all(feature = "std", not(target_os = "none")))]
use std::borrow::Cow;

#[derive(Clone, Debug, PartialEq)]
pub enum BorrowedJsonValue<'a> {
    Null,
    Bool(bool),
    Number(crate::JsonNumber),
    String(Cow<'a, str>),
    Array(Vec<BorrowedJsonValue<'a>>),
    Object(Vec<(Cow<'a, str>, BorrowedJsonValue<'a>)>),
}

impl BorrowedJsonValue<'_> {
    pub fn into_owned(self) -> JsonValue {
        match self {
            Self::Null => JsonValue::Null,
            Self::Bool(v) => JsonValue::Bool(v),
            Self::Number(n) => JsonValue::Number(n),
            Self::String(s) => JsonValue::String(s.into_owned()),
            Self::Array(v) => {
                JsonValue::array_from_iter(v.into_iter().map(BorrowedJsonValue::into_owned))
            }
            Self::Object(e) => JsonValue::object_from_iter(
                e.into_iter().map(|(k, v)| (k.into_owned(), v.into_owned())),
            ),
        }
    }
}
