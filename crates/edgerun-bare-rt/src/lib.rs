//! Minimal bare-metal async runtime for edgerun unikernel.
//! Multi-threaded executor with NO external dependencies.
//!
//! ## Design (embedded-style)
//! - Function-pointer based tasks (no Box<dyn Future>)
//! - Static task storage
//! - Polling via function pointers
//! - Spin-based synchronization

#![no_std]

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { unsafe { core::arch::asm!("hlt", options(noreturn)); } }
}

/// Halt CPU - fatal error.
#[inline(never)]
pub fn halt() -> ! { loop { unsafe { core::arch::asm!("hlt", options(noreturn)); } } }

/// CPU yield.
#[inline]
pub fn yield_cpu() { unsafe { core::arch::asm!("pause"); } }

// ===========================================================================
// Time
// ===========================================================================

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Duration { secs: u64, nanos: u32 }

impl Duration {
    pub const ZERO: Self = Self { secs: 0, nanos: 0 };
    pub const MAX: Self = Self { secs: u64::MAX, nanos: 999_999_999 };
    #[inline] pub const fn from_secs(s: u64) -> Self { Self { secs: s, nanos: 0 } }
    #[inline] pub const fn from_millis(ms: u64) -> Self { Self { secs: ms / 1000, nanos: ((ms % 1000) as u32) * 1_000_000 } }
    #[inline] pub const fn from_micros(us: u64) -> Self { Self { secs: us / 1_000_000, nanos: ((us % 1_000_000) as u32) * 1000 } }
    #[inline] pub const fn as_secs(&self) -> u64 { self.secs }
    #[inline] pub fn as_millis(&self) -> u64 { self.secs * 1000 + (self.nanos / 1_000_000) as u64 }
    #[inline] pub fn as_micros(&self) -> u64 { self.secs * 1_000_000 + (self.nanos / 1000) as u64 }
}

impl core::ops::Add for Duration {
    type Output = Self;
    #[inline] fn add(self, rhs: Self) -> Self::Output {
        let mut s = self.secs.saturating_add(rhs.secs);
        let mut n = self.nanos + rhs.nanos;
        if n >= 1_000_000_000 { n -= 1_000_000_000; s = s.saturating_add(1); }
        Self { secs: s, nanos: n }
    }
}

impl core::ops::Sub for Duration {
    type Output = Self;
    #[inline] fn sub(self, rhs: Self) -> Self::Output {
        let s = self.secs.saturating_sub(rhs.secs);
        let n = if self.nanos >= rhs.nanos { self.nanos - rhs.nanos } else { 1_000_000_000 - (rhs.nanos - self.nanos) };
        Self { secs: s, nanos: n }
    }
}

impl core::ops::AddAssign for Duration {
    #[inline] fn add_assign(&mut self, rhs: Self) { *self = *self + rhs; }
}

impl core::ops::SubAssign for Duration {
    #[inline] fn sub_assign(&mut self, rhs: Self) { *self = *self - rhs; }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Instant { ticks: u64 }

impl Instant {
    #[inline] pub const fn now() -> Self { Self { ticks: 0 } } // Override for hardware timer
    
    #[inline] pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        if self.ticks >= earlier.ticks {
            let diff = self.ticks - earlier.ticks;
            Some(Duration { secs: diff / 1_000_000, nanos: (diff % 1_000_000) as u32 * 1000 })
        } else { None }
    }
    
    #[inline] pub fn duration_since(&self, earlier: Instant) -> Duration {
        self.checked_duration_since(earlier).unwrap_or(Duration::ZERO)
    }
}

// ===========================================================================
// Atomics (std equivalent)
// ===========================================================================

pub type Ordering = core::sync::atomic::Ordering;

pub use core::sync::atomic::{AtomicU64, AtomicUsize, AtomicBool};

#[inline]
pub fn spin_wait() { for _ in 0..8 { unsafe { core::arch::asm!("pause"); } } }

// ===========================================================================
// Spin mutex
// ===========================================================================

pub struct SpinLock { inner: AtomicBool }

impl SpinLock {
    #[inline] pub const fn new() -> Self { Self { inner: AtomicBool::new(false) } }
    #[inline] pub fn lock(&self) { while self.inner.load(Acquire) { spin_wait(); } self.inner.store(true, Release); }
    #[inline] pub fn unlock(&self) { self.inner.store(false, Release); }
}

const Acquire: Ordering = Ordering::Acquire;
const Release: Ordering = Ordering::Release;
const Relaxed: Ordering = Ordering::Relaxed;
const AcqRel: Ordering = Ordering::AcqRel;

// ===========================================================================
// Task system - function pointer based
// ===========================================================================

pub const MAX_TASKS: usize = 64;

/// Poll function type.
pub type PollFn = Option<fn() -> bool>;

/// Task state.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskState { Idle, Ready, Running, Completed }

/// Task slot.
#[derive(Clone, Copy)]
pub struct Task {
    poll_fn: PollFn,
    state: TaskState,
}

impl Task {
    #[inline] pub const fn new() -> Self { Self { poll_fn: None, state: TaskState::Idle } }
}

/// All tasks - static storage.
static mut TASKS: [Task; MAX_TASKS] = [Task::new(); MAX_TASKS];

static mut READY_HEAD: AtomicUsize = AtomicUsize::new(0);
static mut READY_TAIL: AtomicUsize = AtomicUsize::new(0);
static mut READY_Q: [usize; MAX_TASKS] = [0; MAX_TASKS];
static mut TASK_COUNT: AtomicUsize = AtomicUsize::new(0);
static mut RUNNING: AtomicBool = AtomicBool::new(false);

/// Spawn a poll function.
#[inline]
pub fn spawn(f: fn() -> bool) -> usize {
    let idx = unsafe { TASK_COUNT.fetch_add(1, AcqRel) } % MAX_TASKS;
    unsafe {
        TASKS[idx].poll_fn = Some(f);
        TASKS[idx].state = TaskState::Ready;
        let t = READY_TAIL.fetch_add(1, AcqRel) % MAX_TASKS;
        READY_Q[t] = idx;
    }
    idx
}

/// Poll one task.
pub fn poll_one() -> bool {
    loop {
        let head = unsafe { READY_HEAD.load(Acquire) };
        let tail = unsafe { READY_TAIL.load(Acquire) };
        if head >= tail { return false; }
        
        let task_idx = unsafe { READY_Q[head % MAX_TASKS] };
        
        if unsafe { READY_HEAD.compare_exchange(head, head + 1, Acquire, Acquire).is_ok() } {
            unsafe {
                let task = &mut TASKS[task_idx];
                task.state = TaskState::Running;
                
                let ret = if let Some(f) = task.poll_fn { f() } else { false };
                
                if ret {
                    task.state = TaskState::Completed;
                } else {
                    task.state = TaskState::Ready;
                    let t = READY_TAIL.fetch_add(1, AcqRel) % MAX_TASKS;
                    READY_Q[t] = task_idx;
                }
            }
            return true;
        }
    }
}

/// Run all ready tasks.
pub fn run() {
    while poll_one() { }
}

/// Run until no tasks remain.
pub fn run_until_idle() {
    loop { if !poll_one() { break; } }
}

/// Check if tasks exist.
pub fn has_work() -> bool {
    unsafe { READY_HEAD.load(Acquire) < READY_TAIL.load(Acquire) }
}

/// Get/clear running state.
pub fn is_running() -> bool { unsafe { RUNNING.load(Acquire) } }

pub fn set_running(r: bool) { unsafe { RUNNING.store(r, Release); } }

// Re-exports - types already defined above