use alloc::string::{String, ToString};

pub(crate) trait StringResultExt<T> {
    fn string_err(self) -> Result<T, String>;
}

impl<T, E: ToString> StringResultExt<T> for Result<T, E> {
    fn string_err(self) -> Result<T, String> {
        self.map_err(|error| error.to_string())
    }
}
