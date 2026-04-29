//! BLE transport stub for `no_std` bring-up and offline emulation.
//!
//! The stub intentionally has deterministic behavior and bounded memory so it can
//! be used from unit-like checks and on bare-metal bootstrap flows before real
//! ESP32-S3 radio glue is available.

use core::cell::UnsafeCell;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::esp32s3_ble::{BleLinkTransport, HCI_ACL_PAYLOAD_LEN};

#[derive(Clone, Copy)]
struct Frame {
    used: bool,
    len: usize,
    bytes: [u8; HCI_ACL_PAYLOAD_LEN],
}

impl Frame {
    const fn new() -> Self {
        Self {
            used: false,
            len: 0,
            bytes: [0u8; HCI_ACL_PAYLOAD_LEN],
        }
    }
}

struct FrameQueue<const N: usize> {
    frames: [Frame; N],
    head: usize,
    tail: usize,
}

impl<const N: usize> FrameQueue<N> {
    const fn new() -> Self {
        Self {
            frames: [Frame::new(); N],
            head: 0,
            tail: 0,
        }
    }

    fn clear(&mut self) {
        let mut i = 0;
        while i < N {
            self.frames[i].used = false;
            self.frames[i].len = 0;
            i += 1;
        }
        self.head = 0;
        self.tail = 0;
    }

    fn push(&mut self, input: &[u8]) -> bool {
        if input.len() > HCI_ACL_PAYLOAD_LEN || self.frames[self.head].used {
            return false;
        }
        let frame = &mut self.frames[self.head];
        frame.len = input.len();
        frame.bytes[..input.len()].copy_from_slice(input);
        frame.used = true;
        self.head = (self.head + 1) % N;
        true
    }

    fn pop(&mut self, out: &mut [u8]) -> Option<usize> {
        if !self.frames[self.tail].used {
            return None;
        }
        let frame = &mut self.frames[self.tail];
        if out.len() < frame.len {
            return None;
        }
        out[..frame.len].copy_from_slice(&frame.bytes[..frame.len]);
        let len = frame.len;
        frame.used = false;
        frame.len = 0;
        self.tail = (self.tail + 1) % N;
        Some(len)
    }
}

#[derive(Clone, Copy)]
struct RawCounters {
    tx: u32,
    rx: u32,
}

impl RawCounters {
    const fn new() -> Self {
        Self { tx: 0, rx: 0 }
    }
}

pub struct NoBleRadio<const RX: usize = 8, const TX: usize = 8> {
    tx: UnsafeCell<FrameQueue<TX>>,
    rx: UnsafeCell<FrameQueue<RX>>,
    dropped_tx: AtomicBool,
    dropped_rx: AtomicBool,
    counters: UnsafeCell<RawCounters>,
}

unsafe impl<const RX: usize, const TX: usize> Send for NoBleRadio<RX, TX> {}
unsafe impl<const RX: usize, const TX: usize> Sync for NoBleRadio<RX, TX> {}

impl<const RX: usize, const TX: usize> NoBleRadio<RX, TX> {
    pub const fn new() -> Self {
        Self {
            tx: UnsafeCell::new(FrameQueue::new()),
            rx: UnsafeCell::new(FrameQueue::new()),
            dropped_tx: AtomicBool::new(false),
            dropped_rx: AtomicBool::new(false),
            counters: UnsafeCell::new(RawCounters::new()),
        }
    }

    pub fn clear(&self) {
        unsafe {
            (&mut *self.tx.get()).clear();
            (&mut *self.rx.get()).clear();
        }
        self.dropped_tx.store(false, Ordering::SeqCst);
        self.dropped_rx.store(false, Ordering::SeqCst);
        unsafe { &mut *self.counters.get() }.tx = 0;
        unsafe { &mut *self.counters.get() }.rx = 0;
    }

    pub fn tx_dropped(&self) -> bool {
        self.dropped_tx.load(Ordering::SeqCst)
    }

    pub fn rx_dropped(&self) -> bool {
        self.dropped_rx.load(Ordering::SeqCst)
    }

    pub fn tx_count(&self) -> u32 {
        unsafe { (*self.counters.get()).tx }
    }

    pub fn rx_count(&self) -> u32 {
        unsafe { (*self.counters.get()).rx }
    }

    pub fn inject_rx(&self, frame: &[u8]) -> bool {
        let result = unsafe { (&mut *self.rx.get()).push(frame) };
        if !result {
            self.dropped_rx.store(true, Ordering::SeqCst);
        }
        result
    }

    pub fn pop_tx(&self, out: &mut [u8]) -> Option<usize> {
        unsafe { (&mut *self.tx.get()).pop(out) }
    }
}

impl<const RX: usize, const TX: usize> Default for NoBleRadio<RX, TX> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const RX: usize, const TX: usize> BleLinkTransport for NoBleRadio<RX, TX> {
    fn tx(&mut self, packet: &[u8]) -> bool {
        if unsafe { (&mut *self.tx.get()).push(packet) } {
            unsafe { (&mut *self.counters.get()).tx = (*self.counters.get()).tx.wrapping_add(1) };
            true
        } else {
            self.dropped_tx.store(true, Ordering::SeqCst);
            false
        }
    }

    fn rx(&mut self, out: &mut [u8]) -> Option<usize> {
        let result = unsafe { (&mut *self.rx.get()).pop(out) };
        if let Some(len) = result {
            unsafe { (&mut *self.counters.get()).rx = (*self.counters.get()).rx.wrapping_add(1) };
            Some(len)
        } else {
            None
        }
    }
}
