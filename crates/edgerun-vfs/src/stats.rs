//! Memory statistics and monitoring

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

impl std::fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Memory Statistics:")?;
        writeln!(f, "  Files: {}", self.file_count)?;
        writeln!(f, "  Dirty files: {}", self.dirty_count)?;
        writeln!(f, "  Deleted files: {}", self.deleted_count)?;
        writeln!(f, "  Memory usage: {:.2} MB", self.memory_mb)?;
        writeln!(f, "  Avg file size: {} bytes", self.avg_file_size)?;
        Ok(())
    }
}
