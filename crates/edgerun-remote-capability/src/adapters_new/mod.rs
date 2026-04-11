//! Additional remote adapters: display, NPU, fingerprint.

mod display;
mod npu;
mod fingerprint;

pub use display::DisplayRemoteAdapter;
pub use npu::NpuRemoteAdapter;
pub use fingerprint::FingerprintRemoteAdapter;
