use std::ops::Deref;
use std::path::Path;
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct AbsolutePathBuf(PathBuf);

impl AbsolutePathBuf {
    pub fn current_dir() -> std::io::Result<Self> {
        Self::from_absolute_path(std::env::current_dir()?).map_err(std::io::Error::other)
    }

    pub fn from_absolute_path(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        if path.is_absolute() {
            Ok(Self(path))
        } else {
            Err(format!("path is not absolute: {}", path.display()))
        }
    }

    pub fn resolve_path_against_base(path: &Path, base: &Self) -> Self {
        if path.is_absolute() {
            Self(path.to_path_buf())
        } else {
            Self(base.0.join(path))
        }
    }

    pub fn as_path(&self) -> &Path {
        self.0.as_path()
    }

    pub fn to_path_buf(&self) -> PathBuf {
        self.0.clone()
    }

    pub fn into_path_buf(self) -> PathBuf {
        self.0
    }

    pub fn display(&self) -> std::path::Display<'_> {
        self.0.display()
    }

    pub fn join(&self, path: &Path) -> Self {
        Self::resolve_path_against_base(path, self)
    }

    pub fn parent(&self) -> Option<Self> {
        self.0.parent().map(|path| Self(path.to_path_buf()))
    }

    pub fn file_name(&self) -> Option<&std::ffi::OsStr> {
        self.0.file_name()
    }
}

impl AsRef<Path> for AbsolutePathBuf {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl Deref for AbsolutePathBuf {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        self.as_path()
    }
}
