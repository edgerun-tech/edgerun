pub mod font;
#[cfg(not(target_arch = "wasm32"))]
pub mod gpu;
pub mod logging;
pub mod render;
pub mod terminal;
pub mod text;

#[cfg(test)]
#[path = "tests/main_tests.rs"]
mod tests;
