pub mod queue;
pub mod types;

#[cfg(feature = "event-virtio")]
pub mod virtio_loop;

#[cfg(test)]
mod tests;

pub use queue::EventQueue;
pub use types::{DiskSubtype, Event, EventType, NetworkSubtype, TimerSubtype};

#[cfg(feature = "event-virtio")]
pub use virtio_loop::VirtioEventLoop;
