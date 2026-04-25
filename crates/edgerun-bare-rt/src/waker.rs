//! Waker implementations for bare-metal.
/// 
/// Provides waker primitives for async/await.

use crate::executor::TaskId;

/// Create a waker that wakes a specific task.
pub fn local_waker(task_id: TaskId) -> LocalWaker {
    LocalWaker(task_id)
}

/// Waker that wakes a specific task by ID.
#[derive(Clone)]
pub struct LocalWaker(TaskId);

impl LocalWaker {
    /// Wake the task.
    pub fn wake(&self) {
        // Task waking implemented in executor
    }
}

impl core::task::Wake for LocalWaker {
    fn wake(self: &LocalWaker) {
        // Forward to executor
        self.wake();
    }
    
    fn wake_by_ref(self: &LocalWaker) {
        // Forward to executor
        self.wake();
    }
}