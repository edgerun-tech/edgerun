//! Small EdgeRun-owned async primitives used by Codex.

extern crate std;

use std::boxed::Box;
use std::collections::VecDeque;
use std::future::Future as StdFuture;
use std::marker::PhantomPinned;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::vec::Vec;

pub use std::future::Future;

pub trait Stream {
    type Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
}

impl<S> Stream for Pin<Box<S>>
where
    S: Stream + ?Sized,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.get_mut().as_mut().poll_next(cx)
    }
}

pub trait Sink<Item> {
    type Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;
    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error>;
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>>;
}

impl<Item, S> Sink<Item> for Pin<Box<S>>
where
    S: Sink<Item> + ?Sized,
{
    type Error = S::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut().as_mut().poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        self.get_mut().as_mut().start_send(item)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut().as_mut().poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.get_mut().as_mut().poll_close(cx)
    }
}

pub mod future {
    use super::*;

    pub type BoxFuture<'a, T> = Pin<Box<dyn StdFuture<Output = T> + std::marker::Send + 'a>>;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Either<L, R> {
        Left(L),
        Right(R),
    }

    pub fn ready<T>(value: T) -> Ready<T> {
        Ready { value: Some(value) }
    }

    pub struct Ready<T> {
        value: Option<T>,
    }

    impl<T> Unpin for Ready<T> {}

    impl<T> StdFuture for Ready<T> {
        type Output = T;

        fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
            Poll::Ready(self.value.take().expect("Ready polled after completion"))
        }
    }

    pub fn poll_fn<T, F>(f: F) -> PollFn<F>
    where
        F: FnMut(&mut Context<'_>) -> Poll<T>,
    {
        PollFn { f }
    }

    pub struct PollFn<F> {
        f: F,
    }

    impl<T, F> StdFuture for PollFn<F>
    where
        F: FnMut(&mut Context<'_>) -> Poll<T>,
    {
        type Output = T;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            // SAFETY: `f` is not moved out of `self`.
            let this = unsafe { self.as_mut().get_unchecked_mut() };
            (this.f)(cx)
        }
    }

    pub fn select<A, B>(future_a: A, future_b: B) -> Select<A, B>
    where
        A: StdFuture + Unpin,
        B: StdFuture + Unpin,
    {
        Select {
            future_a: Some(future_a),
            future_b: Some(future_b),
        }
    }

    pub struct Select<A, B> {
        future_a: Option<A>,
        future_b: Option<B>,
    }

    impl<A, B> StdFuture for Select<A, B>
    where
        A: StdFuture + Unpin,
        B: StdFuture + Unpin,
    {
        type Output = Either<(A::Output, B), (B::Output, A)>;

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if let Some(future_a) = self.future_a.as_mut() {
                if let Poll::Ready(output) = Pin::new(future_a).poll(cx) {
                    let future_b = self.future_b.take().expect("select missing second future");
                    self.future_a = None;
                    return Poll::Ready(Either::Left((output, future_b)));
                }
            }
            if let Some(future_b) = self.future_b.as_mut() {
                if let Poll::Ready(output) = Pin::new(future_b).poll(cx) {
                    let future_a = self.future_a.take().expect("select missing first future");
                    self.future_b = None;
                    return Poll::Ready(Either::Right((output, future_a)));
                }
            }
            Poll::Pending
        }
    }

    pub struct Shared<F>
    where
        F: StdFuture,
    {
        state: Arc<Mutex<SharedState<F>>>,
    }

    struct SharedState<F>
    where
        F: StdFuture,
    {
        future: Option<F>,
        output: Option<F::Output>,
        wakers: Vec<Waker>,
    }

    impl<F> Clone for Shared<F>
    where
        F: StdFuture,
    {
        fn clone(&self) -> Self {
            Self {
                state: Arc::clone(&self.state),
            }
        }
    }

    impl<F> Shared<F>
    where
        F: StdFuture,
    {
        pub(crate) fn new(future: F) -> Self {
            Self {
                state: Arc::new(Mutex::new(SharedState {
                    future: Some(future),
                    output: None,
                    wakers: Vec::new(),
                })),
            }
        }
    }

    impl<F> StdFuture for Shared<F>
    where
        F: StdFuture + Unpin,
        F::Output: Clone,
    {
        type Output = F::Output;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let mut state = self
                .state
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(output) = state.output.as_ref() {
                return Poll::Ready(output.clone());
            }
            let Some(future) = state.future.as_mut() else {
                state.wakers.push(cx.waker().clone());
                return Poll::Pending;
            };
            match Pin::new(future).poll(cx) {
                Poll::Ready(output) => {
                    state.future = None;
                    state.output = Some(output.clone());
                    for waker in state.wakers.drain(..) {
                        waker.wake();
                    }
                    Poll::Ready(output)
                }
                Poll::Pending => {
                    state.wakers.push(cx.waker().clone());
                    Poll::Pending
                }
            }
        }
    }
}

pub mod stream {
    use super::*;

    pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = T> + std::marker::Send + 'a>>;

    pub fn iter<I>(iter: I) -> Iter<I::IntoIter>
    where
        I: IntoIterator,
    {
        Iter {
            iter: iter.into_iter(),
        }
    }

    pub struct Iter<I> {
        iter: I,
    }

    impl<I> Unpin for Iter<I> {}

    impl<I> Stream for Iter<I>
    where
        I: Iterator,
    {
        type Item = I::Item;

        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Ready(self.iter.next())
        }
    }

    pub fn pending<T>() -> Pending<T> {
        Pending {
            _marker: std::marker::PhantomData,
        }
    }

    pub struct Pending<T> {
        _marker: std::marker::PhantomData<T>,
    }

    impl<T> Unpin for Pending<T> {}

    impl<T> Stream for Pending<T> {
        type Item = T;

        fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            Poll::Pending
        }
    }

    pub fn once<F>(future: F) -> Once<F>
    where
        F: StdFuture,
    {
        Once {
            future: Some(Box::pin(future)),
        }
    }

    pub struct Once<F>
    where
        F: StdFuture,
    {
        future: Option<Pin<Box<F>>>,
    }

    impl<F> Unpin for Once<F> where F: StdFuture {}

    impl<F> Stream for Once<F>
    where
        F: StdFuture,
    {
        type Item = F::Output;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let Some(future) = self.future.as_mut() else {
                return Poll::Ready(None);
            };
            match future.as_mut().poll(cx) {
                Poll::Ready(output) => {
                    self.future = None;
                    Poll::Ready(Some(output))
                }
                Poll::Pending => Poll::Pending,
            }
        }
    }

    pub struct FuturesUnordered<F>
    where
        F: StdFuture,
    {
        futures: Vec<Pin<Box<F>>>,
    }

    impl<F> FuturesUnordered<F>
    where
        F: StdFuture,
    {
        pub fn new() -> Self {
            Self {
                futures: Vec::new(),
            }
        }

        pub fn push(&mut self, future: F) {
            self.futures.push(Box::pin(future));
        }
    }

    impl<F> Default for FuturesUnordered<F>
    where
        F: StdFuture,
    {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<F> FromIterator<F> for FuturesUnordered<F>
    where
        F: StdFuture,
    {
        fn from_iter<T: IntoIterator<Item = F>>(iter: T) -> Self {
            let mut futures = Self::new();
            for future in iter {
                futures.push(future);
            }
            futures
        }
    }

    impl<F> Unpin for FuturesUnordered<F> where F: StdFuture {}

    impl<F> Stream for FuturesUnordered<F>
    where
        F: StdFuture,
    {
        type Item = F::Output;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            if self.futures.is_empty() {
                return Poll::Ready(None);
            }
            let mut index = 0;
            while index < self.futures.len() {
                if let Poll::Ready(output) = self.futures[index].as_mut().poll(cx) {
                    drop(self.futures.swap_remove(index));
                    return Poll::Ready(Some(output));
                }
                index += 1;
            }
            Poll::Pending
        }
    }

    pub struct FuturesOrdered<F>
    where
        F: StdFuture,
    {
        futures: VecDeque<Pin<Box<F>>>,
    }

    impl<F> FuturesOrdered<F>
    where
        F: StdFuture,
    {
        pub fn new() -> Self {
            Self {
                futures: VecDeque::new(),
            }
        }

        pub fn push_back(&mut self, future: F) {
            self.futures.push_back(Box::pin(future));
        }

        pub fn push(&mut self, future: F) {
            self.push_back(future);
        }
    }

    impl<F> Default for FuturesOrdered<F>
    where
        F: StdFuture,
    {
        fn default() -> Self {
            Self::new()
        }
    }

    impl<F> Unpin for FuturesOrdered<F> where F: StdFuture {}

    impl<F> Stream for FuturesOrdered<F>
    where
        F: StdFuture,
    {
        type Item = F::Output;

        fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let Some(front) = self.futures.front_mut() else {
                return Poll::Ready(None);
            };
            match front.as_mut().poll(cx) {
                Poll::Ready(output) => {
                    self.futures.pop_front();
                    Poll::Ready(Some(output))
                }
                Poll::Pending => Poll::Pending,
            }
        }
    }
}

pub trait FutureExt: StdFuture + Sized {
    fn boxed<'a>(self) -> future::BoxFuture<'a, Self::Output>
    where
        Self: std::marker::Send + 'a,
    {
        Box::pin(self)
    }

    fn shared(self) -> future::Shared<Self>
    where
        Self: Unpin,
        Self::Output: Clone,
    {
        future::Shared::new(self)
    }

    fn now_or_never(mut self) -> Option<Self::Output>
    where
        Self: Unpin,
    {
        let waker = noop_waker();
        let mut cx = Context::from_waker(&waker);
        match Pin::new(&mut self).poll(&mut cx) {
            Poll::Ready(output) => Some(output),
            Poll::Pending => None,
        }
    }
}

impl<F> FutureExt for F where F: StdFuture + Sized {}

pub trait TryFutureExt: StdFuture + Sized {}

impl<F> TryFutureExt for F where F: StdFuture + Sized {}

pub trait StreamExt: Stream + Sized {
    fn next(&mut self) -> Next<'_, Self>
    where
        Self: Unpin,
    {
        Next { stream: self }
    }

    fn map<F, T>(self, f: F) -> Map<Self, F>
    where
        F: FnMut(Self::Item) -> T,
    {
        Map { stream: self, f }
    }

    fn map_err<F, T, E, U>(self, f: F) -> MapErr<Self, F>
    where
        Self: Stream<Item = Result<T, E>>,
        F: FnMut(E) -> U,
    {
        MapErr { stream: self, f }
    }

    fn chain<S>(self, other: S) -> Chain<Self, S>
    where
        S: Stream<Item = Self::Item>,
    {
        Chain {
            first: self,
            second: other,
            first_done: false,
        }
    }
}

impl<S> StreamExt for S where S: Stream + Sized {}

pub struct Next<'a, S>
where
    S: Stream + Unpin + ?Sized,
{
    stream: &'a mut S,
}

impl<S> StdFuture for Next<'_, S>
where
    S: Stream + Unpin + ?Sized,
{
    type Output = Option<S::Item>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.stream).poll_next(cx)
    }
}

pub struct Map<S, F> {
    stream: S,
    f: F,
}

impl<S, F> Unpin for Map<S, F> {}

impl<S, F, T> Stream for Map<S, F>
where
    S: Stream,
    F: FnMut(S::Item) -> T,
{
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // SAFETY: fields are not moved out of `self`.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        match unsafe { Pin::new_unchecked(&mut this.stream) }.poll_next(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(Some((this.f)(item))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct MapErr<S, F> {
    stream: S,
    f: F,
}

impl<S, F> Unpin for MapErr<S, F> {}

impl<S, F, T, E, U> Stream for MapErr<S, F>
where
    S: Stream<Item = Result<T, E>>,
    F: FnMut(E) -> U,
{
    type Item = Result<T, U>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // SAFETY: fields are not moved out of `self`.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        match unsafe { Pin::new_unchecked(&mut this.stream) }.poll_next(cx) {
            Poll::Ready(Some(Ok(item))) => Poll::Ready(Some(Ok(item))),
            Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err((this.f)(error)))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct Chain<A, B> {
    first: A,
    second: B,
    first_done: bool,
}

impl<A, B> Unpin for Chain<A, B> {}

impl<A, B> Stream for Chain<A, B>
where
    A: Stream,
    B: Stream<Item = A::Item>,
{
    type Item = A::Item;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // SAFETY: fields are not moved out of `self`.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        if !this.first_done {
            match unsafe { Pin::new_unchecked(&mut this.first) }.poll_next(cx) {
                Poll::Ready(Some(item)) => return Poll::Ready(Some(item)),
                Poll::Ready(None) => this.first_done = true,
                Poll::Pending => return Poll::Pending,
            }
        }
        unsafe { Pin::new_unchecked(&mut this.second) }.poll_next(cx)
    }
}

pub trait TryStreamExt: Stream + Sized {
    fn try_next(&mut self) -> TryNext<'_, Self>
    where
        Self: Unpin,
    {
        TryNext { stream: self }
    }
}

impl<S> TryStreamExt for S where S: Stream + Sized {}

pub struct TryNext<'a, S>
where
    S: Stream + Unpin + ?Sized,
{
    stream: &'a mut S,
}

impl<S, T, E> StdFuture for TryNext<'_, S>
where
    S: Stream<Item = Result<T, E>> + Unpin + ?Sized,
{
    type Output = Result<Option<T>, E>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut *self.stream).poll_next(cx) {
            Poll::Ready(Some(Ok(item))) => Poll::Ready(Ok(Some(item))),
            Poll::Ready(Some(Err(error))) => Poll::Ready(Err(error)),
            Poll::Ready(None) => Poll::Ready(Ok(None)),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub trait SinkExt<Item>: Sink<Item> + Sized {
    fn send(&mut self, item: Item) -> Send<'_, Self, Item>
    where
        Self: Unpin,
    {
        Send {
            sink: self,
            item: Some(item),
            state: SendState::Ready,
        }
    }

    fn close(&mut self) -> Close<'_, Self, Item>
    where
        Self: Unpin,
    {
        Close {
            sink: self,
            _item: std::marker::PhantomData,
        }
    }
}

impl<S, Item> SinkExt<Item> for S where S: Sink<Item> + Sized {}

enum SendState {
    Ready,
    Flush,
}

pub struct Send<'a, S, Item>
where
    S: Sink<Item> + Unpin + ?Sized,
{
    sink: &'a mut S,
    item: Option<Item>,
    state: SendState,
}

impl<S, Item> StdFuture for Send<'_, S, Item>
where
    S: Sink<Item> + Unpin + ?Sized,
{
    type Output = Result<(), S::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            // SAFETY: `Send` is Unpin in practice through its `S: Unpin` bound and we do not move
            // the sink behind the stored mutable reference.
            let this = unsafe { self.as_mut().get_unchecked_mut() };
            match this.state {
                SendState::Ready => match Pin::new(&mut *this.sink).poll_ready(cx) {
                    Poll::Ready(Ok(())) => {
                        let item = this.item.take().expect("send item missing");
                        Pin::new(&mut *this.sink).start_send(item)?;
                        this.state = SendState::Flush;
                    }
                    Poll::Ready(Err(error)) => return Poll::Ready(Err(error)),
                    Poll::Pending => return Poll::Pending,
                },
                SendState::Flush => return Pin::new(&mut *this.sink).poll_flush(cx),
            }
        }
    }
}

pub struct Close<'a, S, Item>
where
    S: Sink<Item> + Unpin + ?Sized,
{
    sink: &'a mut S,
    _item: std::marker::PhantomData<Item>,
}

impl<S, Item> StdFuture for Close<'_, S, Item>
where
    S: Sink<Item> + Unpin + ?Sized,
{
    type Output = Result<(), S::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: `Close` does not move the sink behind the stored mutable reference.
        let this = unsafe { self.as_mut().get_unchecked_mut() };
        Pin::new(&mut *this.sink).poll_close(cx)
    }
}

pub mod prelude {
    pub use crate::FutureExt;
    pub use crate::SinkExt;
    pub use crate::StreamExt;
    pub use crate::TryFutureExt;
    pub use crate::TryStreamExt;
}

fn noop_waker() -> Waker {
    const VTABLE: RawWakerVTable = RawWakerVTable::new(
        |_| RawWaker::new(std::ptr::null(), &VTABLE),
        |_| {},
        |_| {},
        |_| {},
    );
    // SAFETY: the no-op vtable never dereferences the null data pointer.
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) }
}

#[allow(dead_code)]
struct NotUnpin(PhantomPinned);
