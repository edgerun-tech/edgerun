// SPDX-License-Identifier: GPL-2.0-only
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BlockError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Not aligned: {0}")]
    NotAligned(usize),
    #[error("Out of bounds: offset {0} size {1}")]
    OutOfBounds(u64, u64),
    #[error("Device not ready")]
    NotReady,
}

pub trait BlockDevice: Send + Sync {
    fn read_at(&self, offset: u64, len: usize) -> Result<Vec<u8>, BlockError>;
    fn write_at(&self, offset: u64, data: &[u8]) -> Result<(), BlockError>;
    fn flush(&self) -> Result<(), BlockError>;
    fn size(&self) -> u64;
    fn is_direct(&self) -> bool;
}

pub struct FileBlockDevice {
    #[allow(dead_code)]
    path: PathBuf,
    file: std::sync::Mutex<std::fs::File>,
    size: u64,
    direct: bool,
}

impl FileBlockDevice {
    fn lock_file(&self) -> std::sync::MutexGuard<'_, std::fs::File> {
        // If a previous panic poisoned the lock, recover the inner file instead of crashing.
        match self.file.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl FileBlockDevice {
    pub fn new(path: PathBuf, direct: bool) -> Result<Self, BlockError> {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .append(false)
            .open(&path)?;

        let size = file.metadata()?.len();

        Ok(Self {
            path,
            file: std::sync::Mutex::new(file),
            size,
            direct,
        })
    }

    pub fn set_size(&mut self, size: u64) -> Result<(), BlockError> {
        {
            let file = self.lock_file();
            file.set_len(size)?;
        }
        self.size = size;
        Ok(())
    }
}

impl BlockDevice for FileBlockDevice {
    fn read_at(&self, offset: u64, len: usize) -> Result<Vec<u8>, BlockError> {
        if offset + len as u64 > self.size {
            return Err(BlockError::OutOfBounds(offset, self.size));
        }

        let mut file = self.lock_file();
        let mut buf = vec![0u8; len];
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn write_at(&self, offset: u64, data: &[u8]) -> Result<(), BlockError> {
        if offset + data.len() as u64 > self.size {
            return Err(BlockError::OutOfBounds(offset, self.size));
        }

        let mut file = self.lock_file();
        file.seek(SeekFrom::Start(offset))?;
        file.write_all(data)?;
        Ok(())
    }

    fn flush(&self) -> Result<(), BlockError> {
        let mut file = self.lock_file();
        file.flush()?;
        Ok(())
    }

    fn size(&self) -> u64 {
        self.size
    }

    fn is_direct(&self) -> bool {
        self.direct
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_file_block_device_basic() -> Result<(), BlockError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.bin");

        let mut device = FileBlockDevice::new(path.clone(), false)?;
        device.set_size(1024)?;

        let data = b"hello world";
        device.write_at(0, data)?;
        device.flush()?;

        let read_data = device.read_at(0, 11)?;
        assert_eq!(&read_data, data);

        Ok(())
    }

    #[test]
    fn test_file_block_device_out_of_bounds() -> Result<(), BlockError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.bin");

        let mut device = FileBlockDevice::new(path, false)?;
        device.set_size(10)?;

        let result = device.read_at(5, 10);
        assert!(matches!(result, Err(BlockError::OutOfBounds(5, 10))));

        Ok(())
    }

    #[test]
    fn test_file_block_device_write_read() -> Result<(), BlockError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.bin");

        let mut device = FileBlockDevice::new(path, false)?;
        device.set_size(4096)?;

        for i in 0..100 {
            let offset = (i * 40) as u64;
            let data = format!("test data {i} ");
            device.write_at(offset, data.as_bytes())?;
        }
        device.flush()?;

        for i in 0..100 {
            let offset = (i * 40) as u64;
            let expected = format!("test data {i} ");
            let read = device.read_at(offset, expected.len())?;
            assert_eq!(&read, expected.as_bytes());
        }

        Ok(())
    }

    #[test]
    fn test_file_block_device_size() -> Result<(), BlockError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.bin");

        let mut device = FileBlockDevice::new(path, false)?;
        device.set_size(2048)?;

        assert_eq!(device.size(), 2048);

        Ok(())
    }

    #[test]
    fn test_file_block_device_is_direct() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.bin");

        let device = FileBlockDevice::new(path, true).unwrap();

        assert!(device.is_direct());
    }

    #[test]
    fn test_file_block_device_read_empty() -> Result<(), BlockError> {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("test.bin");

        let mut device = FileBlockDevice::new(path, false)?;
        device.set_size(0)?;

        let result = device.read_at(0, 100);
        assert!(matches!(result, Err(BlockError::OutOfBounds(0, 0))));

        Ok(())
    }
}
