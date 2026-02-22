pub mod font;
#[cfg(not(target_arch = "wasm32"))]
pub mod gpu;
pub mod logging;
pub mod render;
pub mod terminal;
#[cfg(any(test, feature = "test-util"))]
pub mod test_support;
pub mod text;

#[cfg(test)]
#[path = "tests/main_tests.rs"]
mod tests;
