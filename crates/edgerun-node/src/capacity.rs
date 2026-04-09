/// Node capacity tracking — prevents overselling compute resources.
///
/// Tracks total hardware capacity and currently-allocated resources
/// across running workloads. New workloads are rejected if they would
/// exceed available capacity.

use std::sync::Mutex;

/// Hardware capacity discovered at node boot.
pub struct NodeCapacity {
    /// Total physical CPU cores
    pub total_cores: u32,
    /// Total physical memory in bytes
    pub total_memory_bytes: u64,
}

impl NodeCapacity {
    /// Discover the current machine's capacity.
    pub fn discover() -> Self {
        Self {
            total_cores: cpu_count(),
            total_memory_bytes: memory_bytes(),
        }
    }

    /// Available cores after reserving `reserved` for system use.
    pub fn available_cores(&self, reserved: u32) -> u32 {
        self.total_cores.saturating_sub(reserved)
    }
}

/// Current allocation state — protected by a Mutex to avoid TOCTOU races
/// when updating cores and Memory simultaneously.
struct AllocationState {
    allocated_cores: u32,
    allocated_memory: u64,
}

/// Runtime resource tracker — uses a Mutex to atomically update both
/// core and memory counters simultaneously, avoiding the TOCTOU race
/// that existed with two independent atomics.
pub struct ResourceTracker {
    total_cores: u32,
    total_memory_bytes: u64,
    state: Mutex<AllocationState>,
}

impl ResourceTracker {
    pub fn new(capacity: &NodeCapacity, reserved_cores: u32, reserved_memory: u64) -> Self {
        Self {
            total_cores: capacity.available_cores(reserved_cores),
            total_memory_bytes: capacity.total_memory_bytes.saturating_sub(reserved_memory),
            state: Mutex::new(AllocationState {
                allocated_cores: 0,
                allocated_memory: 0,
            }),
        }
    }

    /// Try to allocate resources. Returns true if successful.
    /// Atomic: both cores and memory are checked and updated under a single lock.
    pub fn try_allocate(&self, cores: u32, memory_bytes: u64) -> bool {
        let mut state = self.state.lock().unwrap();
        let cores_ok = cores <= self.total_cores.saturating_sub(state.allocated_cores);
        let mem_ok = memory_bytes <= self.total_memory_bytes.saturating_sub(state.allocated_memory);
        if cores_ok && mem_ok {
            state.allocated_cores += cores;
            state.allocated_memory += memory_bytes;
            true
        } else {
            false
        }
    }

    /// Release previously allocated resources.
    pub fn release(&self, cores: u32, memory_bytes: u64) {
        let mut state = self.state.lock().unwrap();
        state.allocated_cores = state.allocated_cores.saturating_sub(cores);
        state.allocated_memory = state.allocated_memory.saturating_sub(memory_bytes);
    }

    /// Available cores right now.
    pub fn available_cores(&self) -> u32 {
        let state = self.state.lock().unwrap();
        self.total_cores.saturating_sub(state.allocated_cores)
    }

    /// Available memory right now.
    pub fn available_memory(&self) -> u64 {
        let state = self.state.lock().unwrap();
        self.total_memory_bytes.saturating_sub(state.allocated_memory)
    }

    /// Current utilization as percentage (0-100).
    pub fn core_utilization_pct(&self) -> u32 {
        let state = self.state.lock().unwrap();
        if self.total_cores == 0 { return 0; }
        (state.allocated_cores * 100) / self.total_cores
    }
}

// ===========================================================================
// Platform-specific capacity discovery
// ===========================================================================

/// Get CPU core count from /proc/cpuinfo or sysconf.
fn cpu_count() -> u32 {
    // Try /proc/cpuinfo first
    if let Ok(content) = std::fs::read_to_string("/proc/cpuinfo") {
        let count = content.lines().filter(|l| l.starts_with("processor")).count();
        if count > 0 {
            return count as u32;
        }
    }

    // Fallback: sysconf
    let n = unsafe { libc::sysconf(libc::_SC_NPROCESSORS_ONLN) };
    if n > 0 {
        return n as u32;
    }

    // Last resort: 1 core
    1
}

/// Get total physical memory in bytes.
fn memory_bytes() -> u64 {
    // Try /proc/meminfo first
    if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                // Format: "MemTotal:       65818420 kB"
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(kb) = parts[1].parse::<u64>() {
                        return kb * 1024;
                    }
                }
            }
        }
    }

    // Fallback: sysconf
    let pages = unsafe { libc::sysconf(libc::_SC_PHYS_PAGES) };
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if pages > 0 && page_size > 0 {
        return (pages as u64) * (page_size as u64);
    }

    // Last resort: 1 GB
    1_073_741_824
}

/// Format bytes as human-readable string.
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut val = bytes as f64;
    let mut idx = 0;
    while val >= 1024.0 && idx < UNITS.len() - 1 {
        val /= 1024.0;
        idx += 1;
    }
    format!("{:.1} {}", val, UNITS[idx])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_discovery_returns_reasonable_values() {
        let cap = NodeCapacity::discover();
        assert!(cap.total_cores >= 1, "cpu count was 0");
        assert!(cap.total_cores <= 4096, "cpu count impossibly high: {}", cap.total_cores);
        assert!(cap.total_memory_bytes >= 1_000_000, "memory impossibly low");
    }

    #[test]
    fn tracker_allows_allocation_within_limits() {
        let cap = NodeCapacity { total_cores: 8, total_memory_bytes: 16_000_000_000 };
        let tracker = ResourceTracker::new(&cap, 1, 1_000_000_000);

        assert!(tracker.try_allocate(4, 8_000_000_000));
        assert_eq!(tracker.available_cores(), 3); // 8 - 1 reserved - 4 allocated

        tracker.release(4, 8_000_000_000);
        assert_eq!(tracker.available_cores(), 7);
    }

    #[test]
    fn tracker_rejects_oversold_cores() {
        let cap = NodeCapacity { total_cores: 4, total_memory_bytes: 8_000_000_000 };
        let tracker = ResourceTracker::new(&cap, 0, 0);

        assert!(tracker.try_allocate(3, 1_000_000_000));
        assert!(!tracker.try_allocate(2, 1_000_000_000)); // only 1 core left

        tracker.release(3, 1_000_000_000);
    }

    #[test]
    fn tracker_rejects_single_request_exceeding_total() {
        let cap = NodeCapacity { total_cores: 4, total_memory_bytes: 8_000_000_000 };
        let tracker = ResourceTracker::new(&cap, 0, 0);

        assert!(!tracker.try_allocate(10, 1_000_000_000));
        assert!(!tracker.try_allocate(2, 9_000_000_000));
    }

    #[test]
    fn tracker_utilization() {
        let cap = NodeCapacity { total_cores: 10, total_memory_bytes: 20_000_000_000 };
        let tracker = ResourceTracker::new(&cap, 0, 0);

        assert_eq!(tracker.core_utilization_pct(), 0);
        tracker.try_allocate(5, 5_000_000_000);
        assert_eq!(tracker.core_utilization_pct(), 50);
        tracker.release(5, 5_000_000_000);
        assert_eq!(tracker.core_utilization_pct(), 0);
    }
}
