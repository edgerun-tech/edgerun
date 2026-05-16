//! Stream trait and utilities for bare-metal async

use core::marker::PhantomData;
use core::pin::Pin;
use core::task::{Context, Poll};

pub trait Stream {
    type Item;
    
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
}

pub struct Ready<T> {
    value: Option<T>,
}

impl<T> Unpin for Ready<T> {}

impl<T> Ready<T> {
    pub fn new(value: T) -> Self {
        Self { value: Some(value) }
    }
}

impl<T> Stream for Ready<T> {
    type Item = T;
    
    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        Poll::Ready(this.value.take())
    }
}

pub struct Empty<T> {
    _marker: PhantomData<T>,
}

impl<T> Stream for Empty<T> {
    type Item = T;
    
    fn poll_next(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(None)
    }
}

pub fn ready<T>(value: T) -> Ready<T> {
    Ready::new(value)
}

pub fn empty<T>() -> Empty<T> {
    Empty { _marker: PhantomData }
}