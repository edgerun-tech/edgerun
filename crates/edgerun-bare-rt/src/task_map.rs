//! Task map - track spawned tasks by ID.


extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub struct TaskMap {
    tasks: Vec<Arc<TaskEntry>>,
    len: AtomicUsize,
}

pub struct TaskEntry {
    id: u64,
    name: &'static str,
}

impl TaskMap {
    pub fn new() -> Self {
        Self {
            tasks: Vec::new(),
            len: AtomicUsize::new(0),
        }
    }

    pub fn insert(&mut self, name: &'static str) -> u64 {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        self.tasks.push(Arc::new(TaskEntry { id, name }));
        self.len.fetch_add(1, Ordering::Relaxed);
        id
    }

    pub fn remove(&mut self, id: u64) {
        self.tasks.retain(|t| t.id != id);
        self.len.fetch_sub(1, Ordering::Relaxed);
    }

    pub fn len(&self) -> usize {
        self.len.load(Ordering::Relaxed)
    }
}

impl Default for TaskMap {
    fn default() -> Self {
        Self::new()
    }
}