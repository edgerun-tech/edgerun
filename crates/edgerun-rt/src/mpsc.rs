//! Bounded mpsc channel with lock-free backpressure.
//!
//! Design:
//! - Lock-free Treiber stack with tagged pointers (ABA-safe) as the pool
//! - Bounded = pool size (capacity)
//! - Direct handoff: when receiver frees capacity, it transfers to a blocked sender
//! - This prevents races where sender steals slot before receiver can process

use std::hint::spin_loop;
use std::mem::MaybeUninit;
use std::cell::UnsafeCell;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicUsize, AtomicPtr, Ordering};
use std::sync::Arc;

pub fn channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let cap = cap.max(1);
    
    let stub = Box::into_raw(Box::new(Node {
        slot_idx: usize::MAX,
        next: AtomicPtr::new(ptr::null_mut()),
    }));
    
    let inner = Arc::new(Inner {
        head: AtomicPtr::new(stub),
        tail: AtomicPtr::new(stub),
        pool: SlotPool::new(cap),
        closed: AtomicBool::new(false),
        sender_count: AtomicUsize::new(1),
        recv_waker: AtomicPtr::new(ptr::null_mut()),
        waiters_head: AtomicPtr::new(ptr::null_mut()),
        waiters_tail: AtomicPtr::new(ptr::null_mut()),
        waiter_count: AtomicUsize::new(0),
    });
    (
        Sender { inner: inner.clone() },
        Receiver { inner },
    )
}

struct Inner<T> {
    head: AtomicPtr<Node<T>>,
    tail: AtomicPtr<Node<T>>,
    pool: SlotPool<T>,
    closed: AtomicBool,
    sender_count: AtomicUsize,
    recv_waker: AtomicPtr<std::task::Waker>,
    waiters_head: AtomicPtr<SendWaiter<T>>,
    waiters_tail: AtomicPtr<SendWaiter<T>>,
    waiter_count: AtomicUsize,
}

impl<T> Inner<T> {
    fn has_waiters(&self) -> bool {
        !self.waiters_head.load(Ordering::Acquire).is_null()
    }

    fn wake_receiver(&self) {
        let waker = self.recv_waker.swap(ptr::null_mut(), Ordering::AcqRel);
        if !waker.is_null() {
            unsafe { (*waker).wake_by_ref() };
        }
    }

    fn try_handoff(&self, slot_idx: usize) -> bool {
        loop {
            let head = self.waiters_head.load(Ordering::Acquire);
            if head.is_null() {
                return false;
            }

            unsafe {
                let tail = self.waiters_tail.load(Ordering::Acquire);
                let next = (*head).next.load(Ordering::Acquire);
                
                if head == tail && next.is_null() {
                    return false;
                }
                
                let new_head = if next.is_null() { tail } else { next };
                
                if self.waiters_head.compare_exchange(head, new_head, Ordering::AcqRel, Ordering::Acquire).is_ok() {
                    self.waiter_count.fetch_sub(1, Ordering::AcqRel);
                    if next.is_null() {
                        self.waiters_tail.store(head, Ordering::Release);
                    }
                    (*head).slot_idx.store(slot_idx, Ordering::Release);
                    (*head).waker.wake_by_ref();
                    return true;
                }
            }
        }
    }

    fn enqueue_waiter(&self, waiter: *mut SendWaiter<T>) {
        unsafe {
            (*waiter).next.store(ptr::null_mut(), Ordering::Release);
        }
        loop {
            let tail = self.waiters_tail.load(Ordering::Acquire);
            if tail.is_null() {
                if self.waiters_head.compare_exchange(ptr::null_mut(), waiter, Ordering::AcqRel, Ordering::Acquire).is_ok() {
                    let _ = self.waiters_tail.compare_exchange(ptr::null_mut(), waiter, Ordering::Release, Ordering::Relaxed);
                    self.waiter_count.fetch_add(1, Ordering::Relaxed);
                    return;
                }
            } else {
                let next = unsafe { (*tail).next.load(Ordering::Acquire) };
                if next.is_null() {
                    if unsafe { (*tail).next.compare_exchange(ptr::null_mut(), waiter, Ordering::AcqRel, Ordering::Acquire).is_ok() } {
                        self.waiters_tail.store(waiter, Ordering::Release);
                        self.waiter_count.fetch_add(1, Ordering::Relaxed);
                        return;
                    }
                } else {
                    self.waiters_tail.store(next, Ordering::Release);
                }
            }
        }
    }
}

struct SendWaiter<T> {
    value: Option<T>,
    slot_idx: AtomicUsize,
    waker: std::task::Waker,
    next: AtomicPtr<SendWaiter<T>>,
}

unsafe impl<T: Send> Send for Inner<T> {}
unsafe impl<T: Send> Sync for Inner<T> {}

struct Node<T> {
    slot_idx: usize,
    next: AtomicPtr<Node<T>>,
}

struct SlotPool<T> {
    slots: UnsafeCell<Box<[Slot<T>]>>,
    freelist: FreeList,
}

struct Slot<T> {
    value: MaybeUninit<T>,
    init: AtomicBool,
    next: AtomicUsize,
}

struct FreeList {
    head: AtomicUsize,
}

impl FreeList {
    fn new(cap: usize) -> Self {
        let head = if cap > 0 { 0 } else { usize::MAX };
        Self { head: AtomicUsize::new(head) }
    }

    fn pack(ptr: usize, tag: usize) -> usize {
        if ptr == usize::MAX {
            return usize::MAX;
        }
        ptr | (tag << 48)
    }

    fn unpack(v: usize) -> (usize, usize) {
        if v == usize::MAX {
            return (usize::MAX, 0);
        }
        (v & ((1 << 48) - 1), v >> 48)
    }

    pub fn pop<T>(&self, slots: &[Slot<T>]) -> Option<usize> {
        loop {
            let head = self.head.load(Ordering::Acquire);
            let (ptr, tag) = Self::unpack(head);

            if ptr == usize::MAX {
                return None;
            }

            let next = slots[ptr].next.load(Ordering::Acquire);
            let new = Self::pack(next, tag.wrapping_add(1));

            if self.head.compare_exchange(head, new, Ordering::AcqRel, Ordering::Acquire).is_ok() {
                return Some(ptr);
            }
        }
    }

    pub fn push<T>(&self, slots: &mut [Slot<T>], idx: usize) {
        loop {
            let head = self.head.load(Ordering::Acquire);
            let (ptr, tag) = Self::unpack(head);

            slots[idx].next.store(ptr, Ordering::Release);

            let new = Self::pack(idx, tag.wrapping_add(1));

            if self.head.compare_exchange(head, new, Ordering::AcqRel, Ordering::Acquire).is_ok() {
                return;
            }
        }
    }
}

impl<T> SlotPool<T> {
    fn new(cap: usize) -> Self {
        let mut slots = Vec::with_capacity(cap);
        for _ in 0..cap {
            slots.push(Slot {
                value: MaybeUninit::uninit(),
                init: AtomicBool::new(false),
                next: AtomicUsize::new(0),
            });
        }
        
        let freelist = FreeList::new(cap);
        for (i, slot) in slots.iter_mut().enumerate() {
            let next = if i + 1 < cap { i + 1 } else { usize::MAX };
            slot.next.store(next, Ordering::Relaxed);
        }
        Self {
            slots: UnsafeCell::new(slots.into_boxed_slice()),
            freelist,
        }
    }

    unsafe fn write(&self, idx: usize, value: T) {
        let slots = &mut *self.slots.get();
        slots[idx].value.as_mut_ptr().write(value);
        slots[idx].init.store(true, Ordering::Release);
    }

    unsafe fn read(&self, idx: usize) -> T {
        let slots = &*self.slots.get();
        let was_init = slots[idx].init.swap(false, Ordering::AcqRel);
        debug_assert!(was_init, "reading uninitialized slot");
        slots[idx].value.as_ptr().read()
    }

    fn acquire(&self) -> Option<usize> {
        unsafe { self.freelist.pop(&*self.slots.get()) }
    }

    fn release(&self, idx: usize) {
        unsafe { self.freelist.push(&mut *self.slots.get(), idx) }
    }
}

pub struct Sender<T> {
    inner: Arc<Inner<T>>,
}

unsafe impl<T: Send> Send for Sender<T> {}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.inner.sender_count.fetch_add(1, Ordering::Relaxed);
        Sender { inner: self.inner.clone() }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        if self.inner.sender_count.fetch_sub(1, Ordering::AcqRel) == 1 {
            self.inner.closed.store(true, Ordering::Release);
        }
    }
}

impl<T> Sender<T> {
    pub fn try_send(&self, value: T) -> Result<(), SendError<T>> {
        if self.inner.closed.load(Ordering::Relaxed) {
            return Err(SendError(value));
        }

        let Some(slot_idx) = self.inner.pool.acquire() else {
            return Err(SendError(value));
        };
        
        unsafe {
            self.inner.pool.write(slot_idx, value);
        }

        let node = Box::into_raw(Box::new(Node {
            slot_idx,
            next: AtomicPtr::new(ptr::null_mut()),
        }));

        let prev = self.inner.tail.swap(node, Ordering::AcqRel);
        unsafe {
            (*prev).next.store(node, Ordering::Release);
        }

        self.inner.wake_receiver();

        Ok(())
    }

    pub fn send(&self, value: T) -> SendFut<T> {
        SendFut {
            inner: self.inner.clone(),
            value: Some(value),
            waiter_ptr: ptr::null_mut(),
        }
    }

    pub fn send_nowait(&self, value: T) -> Result<(), SendError<T>> {
        self.try_send(value)
    }

    pub fn blocking_send(&self, value: T) -> Result<(), SendError<T>> {
        if self.inner.closed.load(Ordering::Relaxed) {
            return Err(SendError(value));
        }

        loop {
            if let Some(slot_idx) = self.inner.pool.acquire() {
                unsafe {
                    self.inner.pool.write(slot_idx, value);
                }

                let node = Box::into_raw(Box::new(Node {
                    slot_idx,
                    next: AtomicPtr::new(ptr::null_mut()),
                }));

                let prev = self.inner.tail.swap(node, Ordering::AcqRel);
                unsafe {
                    (*prev).next.store(node, Ordering::Release);
                }

                self.inner.wake_receiver();
                return Ok(());
            }
            spin_loop();
        }
    }
}

pub struct SendFut<T> {
    inner: Arc<Inner<T>>,
    value: Option<T>,
    waiter_ptr: *mut SendWaiter<T>,
}

unsafe impl<T> Send for SendFut<T> {}

impl<T: Unpin> std::future::Future for SendFut<T> {
    type Output = Result<(), SendError<T>>;

    fn poll(mut self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let this = self.as_mut().get_mut();
        
        if this.value.is_none() {
            return std::task::Poll::Ready(Ok(()));
        }

        if this.inner.closed.load(Ordering::Relaxed) {
            return std::task::Poll::Ready(Err(SendError(this.value.take().unwrap())));
        }

        if !this.waiter_ptr.is_null() {
            unsafe {
                let assigned = (*this.waiter_ptr).slot_idx.load(Ordering::Acquire);
                if assigned != usize::MAX {
                    (*this.waiter_ptr).slot_idx.store(usize::MAX, Ordering::Release);
                    let slot_idx = assigned;
                    let value = (*this.waiter_ptr).value.take().unwrap();
                    this.inner.pool.write(slot_idx, value);

                    let node = Box::into_raw(Box::new(Node {
                        slot_idx,
                        next: AtomicPtr::new(ptr::null_mut()),
                    }));

                    let prev = this.inner.tail.swap(node, Ordering::AcqRel);
                    (*prev).next.store(node, Ordering::Release);
                    
                    this.inner.wake_receiver();
                    let waiter = Box::from_raw(this.waiter_ptr);
                    drop(waiter);
                    this.waiter_ptr = ptr::null_mut();
                    return std::task::Poll::Ready(Ok(()));
                }
                (*this.waiter_ptr).waker = cx.waker().clone();
            }
        }

        if !this.inner.has_waiters() {
            if let Some(slot_idx) = this.inner.pool.acquire() {
                unsafe {
                    this.inner.pool.write(slot_idx, this.value.take().unwrap());
                }

                let node = Box::into_raw(Box::new(Node {
                    slot_idx,
                    next: AtomicPtr::new(ptr::null_mut()),
                }));

                let prev = this.inner.tail.swap(node, Ordering::AcqRel);
                unsafe {
                    (*prev).next.store(node, Ordering::Release);
                }
                
                this.inner.wake_receiver();
                return std::task::Poll::Ready(Ok(()));
            }
        }

        if this.waiter_ptr.is_null() {
            let waiter = Box::into_raw(Box::new(SendWaiter {
                value: this.value.take(),
                slot_idx: AtomicUsize::new(usize::MAX),
                waker: cx.waker().clone(),
                next: AtomicPtr::new(ptr::null_mut()),
            }));
            this.waiter_ptr = waiter;
            this.inner.enqueue_waiter(waiter);
        }

        std::task::Poll::Pending
    }
}

pub struct Receiver<T> {
    inner: Arc<Inner<T>>,
}

unsafe impl<T: Send> Send for Receiver<T> {}

impl<T> Receiver<T> {
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        if self.inner.closed.load(Ordering::Acquire) {
            if let Some(slot_idx) = self.pop_node() {
                let val = unsafe {
                    let val = self.inner.pool.read(slot_idx);
                    self.inner.pool.release(slot_idx);
                    val
                };
                return Ok(val);
            }
            return Err(TryRecvError::Disconnected);
        }

        self.pop_node()
            .map(|slot_idx| unsafe {
                let val = self.inner.pool.read(slot_idx);
                // Try direct handoff to waiter first, then release if none
                if !self.try_handoff(slot_idx) {
                    self.inner.pool.release(slot_idx);
                }
                val
            })
            .ok_or(TryRecvError::Empty)
    }

    fn try_handoff(&self, slot_idx: usize) -> bool {
        self.inner.try_handoff(slot_idx)
    }

    pub fn blocking_recv(&mut self) -> Option<T> {
        loop {
            if let Ok(val) = self.try_recv() {
                return Some(val);
            }
            if self.inner.closed.load(Ordering::Acquire) && self.inner.head.load(Ordering::Acquire) == self.inner.tail.load(Ordering::Acquire) {
                return None;
            }
            spin_loop();
        }
    }

    pub fn recv(&self) -> RecvFut<'_, T> {
        RecvFut { inner: &self.inner }
    }

    fn pop_node(&self) -> Option<usize> {
        let head = self.inner.head.load(Ordering::Acquire);
        
        unsafe {
            let next = (*head).next.load(Ordering::Acquire);
            
            if next.is_null() {
                return None;
            }
            
            let slot_idx = (*next).slot_idx;
            if slot_idx == usize::MAX {
                self.inner.head.store(next, Ordering::Release);
                let next_next = (*next).next.load(Ordering::Acquire);
                if next_next.is_null() {
                    return None;
                }
                let slot_idx = (*next_next).slot_idx;
                self.inner.head.store(next_next, Ordering::Release);
                drop(Box::from_raw(next));
                return Some(slot_idx);
            }
            
            self.inner.head.store(next, Ordering::Release);
            drop(Box::from_raw(head));
            Some(slot_idx)
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.inner.closed.store(true, Ordering::Release);
        while let Some(slot_idx) = self.pop_node() {
            unsafe {
                let slots = &*self.inner.pool.slots.get();
                if slots[slot_idx].init.swap(false, Ordering::AcqRel) {
                    slots[slot_idx].value.as_ptr().read();
                }
            }
            self.inner.pool.release(slot_idx);
        }
    }
}

pub struct RecvFut<'a, T> {
    inner: &'a Inner<T>,
}

unsafe impl<T: Send> Send for RecvFut<'_, T> {}

impl<T> std::future::Future for RecvFut<'_, T> {
    type Output = Option<T>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let this = self.get_mut();
        
        let try_pop = || -> Option<usize> {
            let head = this.inner.head.load(Ordering::Acquire);
            unsafe {
                let next = (*head).next.load(Ordering::Acquire);
                if next.is_null() {
                    return None;
                }
                let slot_idx = (*next).slot_idx;
                if slot_idx == usize::MAX {
                    this.inner.head.store(next, Ordering::Release);
                    let next_next = (*next).next.load(Ordering::Acquire);
                    if next_next.is_null() {
                        return None;
                    }
                    let slot_idx = (*next_next).slot_idx;
                    this.inner.head.store(next_next, Ordering::Release);
                    drop(Box::from_raw(next));
                    return Some(slot_idx);
                }
                this.inner.head.store(next, Ordering::Release);
                drop(Box::from_raw(head));
                Some(slot_idx)
            }
        };
        
        if this.inner.closed.load(Ordering::Acquire) {
            if let Some(slot_idx) = try_pop() {
                let val = unsafe {
                    let slots = &*this.inner.pool.slots.get();
                    let was_init = slots[slot_idx].init.swap(false, Ordering::AcqRel);
                    debug_assert!(was_init, "reading uninitialized slot");
                    slots[slot_idx].value.as_ptr().read()
                };
                this.inner.pool.release(slot_idx);
                return std::task::Poll::Ready(Some(val));
            }
            return std::task::Poll::Ready(None);
        }

        if let Some(slot_idx) = try_pop() {
            let val = unsafe {
                let slots = &*this.inner.pool.slots.get();
                let was_init = slots[slot_idx].init.swap(false, Ordering::AcqRel);
                debug_assert!(was_init, "reading uninitialized slot");
                slots[slot_idx].value.as_ptr().read()
            };
            if !this.inner.try_handoff(slot_idx) {
                this.inner.pool.release(slot_idx);
            }
            return std::task::Poll::Ready(Some(val));
        }

        let old_waker = this.inner.recv_waker.swap(Box::into_raw(Box::new(cx.waker().clone())), Ordering::AcqRel);
        if !old_waker.is_null() { drop(unsafe { Box::from_raw(old_waker) }); }
        
        if this.inner.closed.load(Ordering::Acquire) || try_pop().is_some() {
            let waker = this.inner.recv_waker.swap(ptr::null_mut(), Ordering::AcqRel);
            if !waker.is_null() { drop(unsafe { Box::from_raw(waker) }); }
            if let Some(slot_idx) = try_pop() {
                let val = unsafe {
                    let slots = &*this.inner.pool.slots.get();
                    let was_init = slots[slot_idx].init.swap(false, Ordering::AcqRel);
                    debug_assert!(was_init, "reading uninitialized slot");
                    slots[slot_idx].value.as_ptr().read()
                };
                if !this.inner.try_handoff(slot_idx) {
                    this.inner.pool.release(slot_idx);
                }
                return std::task::Poll::Ready(Some(val));
            }
            return std::task::Poll::Ready(None);
        }

        std::task::Poll::Pending
    }
}

#[derive(Debug)]
pub struct SendError<T>(pub T);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryRecvError {
    Empty,
    Disconnected,
}

impl<T> std::fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "send error")
    }
}

impl std::fmt::Display for TryRecvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TryRecvError::Empty => write!(f, "channel empty"),
            TryRecvError::Disconnected => write!(f, "channel disconnected"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpsc_send_recv() {
        let (tx, rx) = channel::<i32>(10);
        tx.send_nowait(42).unwrap();
        assert_eq!(rx.try_recv(), Ok(42));
    }

    #[test]
    fn mpsc_backpressure() {
        let (tx, rx) = channel::<i32>(2);
        tx.send_nowait(1).unwrap();
        tx.send_nowait(2).unwrap();
        assert!(tx.send_nowait(3).is_err());
        assert_eq!(rx.try_recv(), Ok(1));
        assert_eq!(rx.try_recv(), Ok(2));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
    }

    #[test]
    fn mpsc_close() {
        let (tx, rx) = channel::<i32>(10);
        tx.send_nowait(1).unwrap();
        tx.send_nowait(2).unwrap();
        drop(tx);
        assert_eq!(rx.try_recv(), Ok(1));
        assert_eq!(rx.try_recv(), Ok(2));
        assert_eq!(rx.try_recv(), Err(TryRecvError::Disconnected));
    }
}
