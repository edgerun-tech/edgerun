// SPDX-License-Identifier: Apache-2.0
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

pub type SharedWriter = Arc<Mutex<Box<dyn Write + Send>>>;
pub type SharedBytes = Arc<Mutex<Vec<u8>>>;

#[derive(Clone, Default)]
pub struct LockedBuf(pub SharedBytes);

impl Write for LockedBuf {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn capture_writer() -> (SharedBytes, SharedWriter) {
    let buf: SharedBytes = Arc::new(Mutex::new(Vec::new()));
    let writer: Box<dyn Write + Send> = Box::new(LockedBuf(buf.clone()));
    (buf, Arc::new(Mutex::new(writer)))
}
