//! FuturesOrdered - ordered futures by key

extern crate alloc;

use alloc::vec::Vec;
use core::cmp::Ordering;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

pub struct FuturesOrdered<F> {
    futures: Vec<(usize, F)>,
    next_key: usize,
}

impl<F> FuturesOrdered<F> {
    pub fn new() -> Self {
        Self { futures: Vec::new(), next_key: 0 }
    }
    
    pub fn push(&mut self, f: F) -> usize {
        let key = self.next_key;
        self.next_key += 1;
        self.futures.push((key, f));
        self.futures.sort_by(|a, b| a.0.cmp(&b.0));
        key
    }
    
    pub fn len(&self) -> usize {
        self.futures.len()
    }
    
    pub fn is_empty(&self) -> bool {
        self.futures.is_empty()
    }
}

impl<F: Future> Future for FuturesOrdered<F> {
    type Output = Option<F::Output>;
    
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        for (_, ref mut f) in self.futures.iter_mut() {
            match Pin::new(f).poll(cx) {
                Poll::Ready(v) => return Poll::Ready(Some(v)),
                Poll::Pending => {}
            }
        }
        Poll::Ready(None)
    }
}

impl<F> Default for FuturesOrdered<F> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn futures_ordered<F>() -> FuturesOrdered<F> {
    FuturesOrdered::new()
}