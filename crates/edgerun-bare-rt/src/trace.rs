//! Tracing stubs for no_std.

#![no_std]

// ===========================================================================
// Trace
// ===========================================================================

pub struct Span {
    id: u64,
    name: &'static str,
}

impl Span {
    pub fn new(name: &'static str) -> Self {
        Self { id: 0, name }
    }
    
    pub fn enter(&self) -> EnterGuard {
        EnterGuard { span: self }
    }
}

pub struct EnterGuard<'a> {
    span: &'a Span,
}

impl<'a> Drop for EnterGuard<'a> {
    fn drop(&mut self) {}
}

pub fn span(name: &'static str) -> Span {
    Span::new(name)
}