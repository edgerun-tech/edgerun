//! Signal/interrupt handling for bare-metal

use core::sync::atomic::{AtomicBool, Ordering};

pub struct Signal {
    raised: AtomicBool,
}

impl Signal {
    pub const fn new() -> Self {
        Self { raised: AtomicBool::new(false) }
    }
    
    pub fn raised(&self) -> bool {
        self.raised.load(Ordering::Acquire)
    }
    
    pub fn raise(&self) {
        self.raised.store(true, Ordering::Release);
    }
    
    pub fn clear(&self) {
        self.raised.store(false, Ordering::Release);
    }
}

pub struct SignalHandler<F> {
    signal: Signal,
    handler: F,
}

impl<F> SignalHandler<F> {
    pub fn new(signal: Signal, handler: F) -> Self {
        Self { signal, handler }
    }
    
    pub fn poll(&mut self) where F: FnMut() {
        if self.signal.raised() {
            (self.handler)();
            self.signal.clear();
        }
    }
}

pub fn ctrl_c() -> Signal { Signal::new() }
pub fn alarm() -> Signal { Signal::new() }
pub fn usr1() -> Signal { Signal::new() }
pub fn usr2() -> Signal { Signal::new() }