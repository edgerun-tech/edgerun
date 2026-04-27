//! FuturesUnordered - manage many futures

extern crate alloc;

use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub struct FuturesUnordered<F> {
    futures: Vec<F>,
}

impl<F> FuturesUnordered<F> {
    pub fn new() -> Self {
        Self { futures: Vec::new() }
    }
    
    pub fn push(&mut self, f: F) {
        self.futures.push(f);
    }
    
    pub fn len(&self) -> usize {
        self.futures.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.futures.is_empty()
    }
    
    pub fn clear(&mut self) {
        self.futures.clear();
    }
}

impl<F: Future> Future for FuturesUnordered<F> {
    type Output = Option<F::Output>;
    
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        for i in 0..self.futures.len() {
            match Pin::new(&mut self.futures[i]).poll(cx) {
                Poll::Ready(v) => {
                    self.futures.remove(i);
                    return Poll::Ready(Some(v));
                }
                Poll::Pending => {}
            }
        }
        Poll::Ready(None)
    }
}

impl<F> Default for FuturesUnordered<F> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn futures_unordered<F>() -> FuturesUnordered<F> {
    FuturesUnordered::new()
}