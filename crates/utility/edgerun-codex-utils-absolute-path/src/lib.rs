pub mod test_support {
    use std::path::Path;
    use std::path::PathBuf;

    pub trait PathExt {
        fn abs(&self) -> PathBuf;
    }

    impl PathExt for Path {
        fn abs(&self) -> PathBuf {
            if self.is_absolute() {
                self.to_path_buf()
            } else {
                std::env::current_dir()
                    .expect("current directory")
                    .join(self)
            }
        }
    }

    pub trait PathBufExt {
        fn abs(self) -> PathBuf;
    }

    impl PathBufExt for PathBuf {
        fn abs(self) -> PathBuf {
            if self.is_absolute() {
                self
            } else {
                std::env::current_dir()
                    .expect("current directory")
                    .join(self)
            }
        }
    }
}
