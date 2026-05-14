pub mod io {
    #[derive(Debug, Default)]
    pub struct Builder {
        reads: Vec<u8>,
    }

    impl Builder {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn read(&mut self, bytes: &[u8]) -> &mut Self {
            self.reads.extend_from_slice(bytes);
            self
        }

        pub fn build(self) -> std::io::Cursor<Vec<u8>> {
            std::io::Cursor::new(self.reads)
        }
    }
}
