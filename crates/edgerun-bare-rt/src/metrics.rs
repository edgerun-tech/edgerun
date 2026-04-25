//! Runtime metrics stub.


pub struct RuntimeMetrics;

impl RuntimeMetrics {
    pub fn new() -> Self { Self }
    pub fn worker_count(&self) -> usize { 0 }
    pub fn active_tasks(&self) -> usize { 0 }
    pub fn completed_tasks(&self) -> u64 { 0 }
    pub fn panic_count(&self) -> u64 { 0 }
}

pub struct TaskMetrics;

impl TaskMetrics {
    pub fn new(_name: &str) -> Self { Self }
    pub fn id(&self) -> u64 { 0 }
    pub fn name(&self) -> &str { "" }
    pub fn latency(&self) -> u64 { 0 }
}