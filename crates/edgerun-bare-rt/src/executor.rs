//! Runtime executor - simple task queue stub

#![no_std]

extern crate alloc;

pub fn run() {}

pub fn spawn_tasks_and_run() {
    loop {
        run();
    }
}