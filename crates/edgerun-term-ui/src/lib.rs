use thiserror::Error;

#[derive(Debug, Error)]
pub enum TermUiError {
	#[error("IO error: {0}")]
	Io(#[from] std::io::Error),
	#[error("Tempfile error: {0}")]
	Tempfile(#[from] tempfile::PersistError),
	#[error("Other error: {0}")]
	Other(String),
}
pub mod app_render;
pub mod debug;
pub mod input;
pub mod overlay;
pub mod suggest;
pub mod widgets;
