//! Simple executor and task system for bare-metal.

use crate::time::{Duration, Instant};

/// Waker that wakes a specific task.
#[derive(Clone)]
pub struct LocalWaker {
    index: usize,
}

/// Create a local waker for a task.
#[inline]
pub fn local_waker(index: usize) -> LocalWaker {
    LocalWaker { index }
}

impl LocalWaker {
    /// Wake the task.
    pub fn wake(&self) {
        // Will be handled by executor
        Executor::wake_task(self.index);
    }
}

use core::task::{RawWaker, RawWakerVTable, Waker};

/// Convert LocalWaker to std Waker for compatibility.
impl From<LocalWaker> for Waker {
    fn from(w: LocalWaker) -> Self {
        let ptr = Box::into_raw(Box::new(w.index));
        unsafe {
            Waker::new_unchecked(RawWaker::new(
                ptr as *const (),
                &VTABLE,
            ))
        }
    }
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(
    |ptr| {
        // clone - just increment ref (not implemented for local)
        let _ = ptr;
    },
    |ptr| {
        // wake - wake the task
        let idx = unsafe { *(ptr as *const usize) };
        Executor::wake_task(idx);
    },
    |ptr| {
        // wake_by_ref
        let idx = unsafe { *(ptr as *const usize) };
        Executor::wake_task(idx);
    },
    |_ptr| {
        // drop - nothing to do
    },
);

/// Task state.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TaskState {
    Idle,
    Ready,
    Running,
    Completed,
}

/// Task handle for spawning.
pub struct TaskId(usize);

/// Spawn result.
pub struct JoinHandle<T> {
    _phantom: core::marker::PhantomData<T>,
}

/// Spawn a future on the executor.
pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: core::future::Future,
{
    Executor::spawn(future)
}

/// Maximum number of tasks.
const MAX_TASKS: usize = 64;

/// Task storage - either heap or global static storage.
/// Using heapless::Vec for no_std compatibility would be ideal,
/// but we'll use a simple static array initially.
struct TaskStorage {
    future: Option<core::pin::Pin<Box<dyn core::future::Future<Output = ()>>>,
    state: TaskState,
    waker: Option<Waker>,
}

static mut TASKS: [TaskStorage; MAX_TASKS] = [TaskStorage {
    future: None,
    state: TaskState::Idle,
    waker: None,
}];
static mut READY_QUEUE: heapless::Vec<usize, MAX_TASKS> = heapless::Vec::new();
static mut CURRENT_TASK: usize = usize::MAX;

/// Main executor - needs to be in a trait or singleton for bare-metal.
pub struct Executor;

impl Executor {
    /// Spawn a future.
    pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
    where
        F: core::future::Future + 'static,
        F::Output: 'static,
    {
        // Find a free slot
        let index = unsafe {
            for (i, task) in TASKS.iter_mut().enumerate() {
                if task.state == TaskState::Idle && task.future.is_none() {
                    task.future = Some(Box::pin(future));
                    task.state = TaskState::Ready;
                    let _ = READY_QUEUE.push(i);
                    return JoinHandle {
                        _phantom: core::marker::PhantomData,
                    };
                }
            }
            usize::MAX // No space
        };

        JoinHandle {
            _phantom: core::marker::PhantomData,
        }
    }

    /// Wake a task by index.
    pub fn wake_task(index: usize) {
        unsafe {
            if index < MAX_TASKS && TASKS[index].state == TaskState::Running {
                TASKS[index].state = TaskState::Ready;
                let _ = READY_QUEUE.push(index);
            }
        }
    }

    /// Run the executor loop.
    pub fn run(&mut self) -> ! {
        loop {
            // Get next ready task
            let task_index = unsafe { READY_QUEUE.pop() };
            
            if let Some(idx) = task_index {
                unsafe {
                    if let Some(ref mut future) = TASKS[idx].future {
                        CURRENT_TASK = idx;
                        TASKS[idx].state = TaskState::Running;
                        
                        let w = Waker::from(local_waker(idx));
                        let mut ctx = core::task::Context::from_waker(&w);
                        
                        match future.as_mut().poll(&mut ctx) {
                            core::task::Poll::Ready(()) => {
                                TASKS[idx].state = TaskState::Completed;
                                TASKS[idx].future = None;
                            }
                            core::task::Poll::Pending => {
                                TASKS[idx].state = TaskState::Idle;
                            }
                        }
                    }
                }
            } else {
                // No work - idle
                crate::yield_idle();
            }
        }
    }
}