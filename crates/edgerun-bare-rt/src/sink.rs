//! Sink trait for bare-metal async

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub trait Sink<Item> {
    type Error;
    
    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;
    
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;
    
    fn poll_start_push(self: Pin<&mut Self>, cx: &mut Context<'_>, item: Item) -> Poll<Result<(), Self::Error>>;
    
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;
}

pub struct VecSink<T> {
    buffer: Vec<T>,
}

impl<T> Unpin for VecSink<T> {}

impl<T> VecSink<T> {
    pub fn new() -> Self {
        Self { buffer: Vec::new() }
    }
    
    pub fn get_ref(&self) -> &Vec<T> {
        &self.buffer
    }
    
    pub fn into_inner(self) -> Vec<T> {
        self.buffer
    }
}

impl<T: Clone> Sink<T> for VecSink<T> {
    type Error = ();
    
    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
    
    fn poll_start_push(self: Pin<&mut Self>, _: &mut Context<'_>, item: T) -> Poll<Result<(), Self::Error>> {
        self.get_mut().buffer.push(item);
        Poll::Ready(Ok(()))
    }
    
    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }
}

pub fn vec_sink<T: Clone>() -> VecSink<T> {
    VecSink::new()
}