//! Memory statistics and monitoring

use core::fmt;

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub file_count: usize,
    pub dirty_count: usize,
    pub deleted_count: usize,
    pub memory_bytes: usize,
    pub memory_mb: f64,
    pub avg_file_size: usize,
}

impl fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        core::writeln!(f, "Memory Statistics:")?;
        core::writeln!(f, "  Files: {}", self.file_count)?;
        core::writeln!(f, "  Dirty files: {}", self.dirty_count)?;
        core::writeln!(f, "  Deleted files: {}", self.deleted_count)?;
        core::writeln!(f, "  Memory usage: {:.2} MB", self.memory_mb)?;
        core::writeln!(f, "  Avg file size: {} bytes", self.avg_file_size)?;
        Ok(())
    }
}
