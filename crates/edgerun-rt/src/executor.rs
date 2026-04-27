//! Runtime - task executor stub

pub fn spawn<F>(_future: F)
where
    F: Future<Output = ()> + 'static,
{
}

pub fn run_queue() {}
pub fn pending() -> usize {
    0
}
pub fn runs() -> u32 {
    0
}

use core::future::Future;
