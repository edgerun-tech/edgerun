#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "derive")]
pub use edgerun_error_derive::Error;

#[cfg(feature = "std")]
extern crate std;

#[cfg(feature = "std")]
use std::any::Any;
#[cfg(feature = "std")]
use std::boxed::Box;
#[cfg(feature = "std")]
use std::error::Error as StdError;
#[cfg(feature = "std")]
use std::fmt;
#[cfg(feature = "std")]
use std::string::String;

#[cfg(feature = "std")]
pub type Result<T, E = Error> = core::result::Result<T, E>;

#[cfg(not(feature = "std"))]
pub type Result<T, E> = core::result::Result<T, E>;

#[cfg(feature = "std")]
pub struct Error {
    inner: ErrorRepr,
}

#[cfg(feature = "std")]
enum ErrorRepr {
    Message(String),
    Source(Box<dyn StdError + Send + Sync + 'static>),
    Context { context: String, source: Box<Error> },
}

#[cfg(feature = "std")]
pub type Report = Error;

#[cfg(feature = "std")]
impl Error {
    pub fn msg(message: impl fmt::Display) -> Self {
        Self {
            inner: ErrorRepr::Message(message.to_string()),
        }
    }

    pub fn from_boxed(error: Box<dyn StdError + Send + Sync + 'static>) -> Self {
        Self {
            inner: ErrorRepr::Source(error),
        }
    }

    pub fn context(self, context: impl fmt::Display) -> Self {
        Self {
            inner: ErrorRepr::Context {
                context: context.to_string(),
                source: Box::new(self),
            },
        }
    }

    pub fn downcast_ref<T: StdError + Any>(&self) -> Option<&T> {
        if let Some(value) = (self as &dyn Any).downcast_ref::<T>() {
            return Some(value);
        }
        match &self.inner {
            ErrorRepr::Message(_) => None,
            ErrorRepr::Source(source) => {
                let source = source.as_ref() as &(dyn StdError + 'static);
                source.downcast_ref::<T>()
            }
            ErrorRepr::Context { source, .. } => source.downcast_ref::<T>(),
        }
    }

    pub fn chain(&self) -> Chain<'_> {
        Chain { next: Some(self) }
    }
}

#[cfg(feature = "std")]
impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

#[cfg(feature = "std")]
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.inner {
            ErrorRepr::Message(message) => f.write_str(message),
            ErrorRepr::Source(source) => fmt::Display::fmt(source, f),
            ErrorRepr::Context { context, source } => write!(f, "{context}: {source}"),
        }
    }
}

#[cfg(feature = "std")]
impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match &self.inner {
            ErrorRepr::Message(_) => None,
            ErrorRepr::Source(source) => Some(source.as_ref()),
            ErrorRepr::Context { source, .. } => Some(source.as_ref()),
        }
    }
}

#[cfg(feature = "std")]
impl From<String> for Error {
    fn from(value: String) -> Self {
        Self::msg(value)
    }
}

#[cfg(feature = "std")]
impl From<&str> for Error {
    fn from(value: &str) -> Self {
        Self::msg(value)
    }
}

#[cfg(feature = "std")]
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::from_boxed(Box::new(value))
    }
}

#[cfg(feature = "std")]
pub struct Chain<'a> {
    next: Option<&'a (dyn StdError + 'static)>,
}

#[cfg(feature = "std")]
impl<'a> Iterator for Chain<'a> {
    type Item = &'a (dyn StdError + 'static);

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.next.take()?;
        self.next = current.source();
        Some(current)
    }
}

#[cfg(feature = "std")]
pub trait Context<T> {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: fmt::Display;

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: fmt::Display,
        F: FnOnce() -> C;
}

#[cfg(feature = "std")]
impl<T, E> Context<T> for core::result::Result<T, E>
where
    E: StdError + Send + Sync + 'static,
{
    fn context<C>(self, context: C) -> Result<T>
    where
        C: fmt::Display,
    {
        self.map_err(|error| Error::from_boxed(Box::new(error)).context(context))
    }

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: fmt::Display,
        F: FnOnce() -> C,
    {
        self.map_err(|error| Error::from_boxed(Box::new(error)).context(f()))
    }
}

#[cfg(feature = "std")]
impl<T> Context<T> for Option<T> {
    fn context<C>(self, context: C) -> Result<T>
    where
        C: fmt::Display,
    {
        self.ok_or_else(|| Error::msg(context))
    }

    fn with_context<C, F>(self, f: F) -> Result<T>
    where
        C: fmt::Display,
        F: FnOnce() -> C,
    {
        self.ok_or_else(|| Error::msg(f()))
    }
}

#[cfg(feature = "std")]
pub fn anyhow(message: impl fmt::Display) -> Error {
    Error::msg(message)
}

#[macro_export]
macro_rules! anyhow {
    ($($arg:tt)*) => {
        $crate::Error::msg(format!($($arg)*))
    };
}

#[macro_export]
macro_rules! bail {
    ($($arg:tt)*) => {
        return Err($crate::anyhow!($($arg)*))
    };
}

#[macro_export]
macro_rules! ensure {
    ($condition:expr $(,)?) => {
        if !$condition {
            $crate::bail!("condition failed: {}", stringify!($condition));
        }
    };
    ($condition:expr, $($arg:tt)*) => {
        if !$condition {
            $crate::bail!($($arg)*);
        }
    };
}

pub mod anyhow_compat {
    pub use crate::Context;
    pub use crate::Error;
    pub use crate::Report;
    pub use crate::Result;
    pub use crate::anyhow;
    pub use crate::bail;
    pub use crate::ensure;
}
