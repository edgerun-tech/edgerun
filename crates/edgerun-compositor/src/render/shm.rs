//! SHM buffer management — memory-map client SHM pools.

use std::collections::HashMap;
use std::io;
use std::os::fd::RawFd;

/// A SHM pool backed by a memory-mapped file descriptor.
pub struct ShmPool {
    pub id: u32,
    pub fd: RawFd,
    pub size: usize,
    pub mapping: *mut u8,
}

unsafe impl Send for ShmPool {}

impl ShmPool {
    /// Create a SHM pool by mmap'ing the client's fd.
    pub fn new(id: u32, fd: RawFd, size: i32) -> io::Result<Self> {
        let mut actual_size = size as usize;

        // If size is 0, get the actual fd size (client may resize later via wl_shm_pool.resize)
        if actual_size == 0 {
            let mut statbuf: libc::stat = unsafe { std::mem::zeroed() };
            if unsafe { libc::fstat(fd, &mut statbuf) } < 0 {
                return Err(io::Error::last_os_error());
            }
            actual_size = statbuf.st_size as usize;
            if actual_size == 0 {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, "SHM pool fd has zero size"));
            }
        }

        let ptr = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                actual_size,
                libc::PROT_READ,
                libc::MAP_PRIVATE,
                fd,
                0,
            )
        };

        if ptr == libc::MAP_FAILED {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            id,
            fd,
            size: actual_size,
            mapping: ptr as *mut u8,
        })
    }

    /// Read pixel data from the pool at the given offset.
    pub fn read(&self, offset: usize, len: usize) -> Option<&[u8]> {
        if offset + len > self.size {
            return None;
        }
        Some(unsafe { std::slice::from_raw_parts(self.mapping.add(offset), len) })
    }

    /// Get the full mapping as a slice.
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.mapping, self.size) }
    }
}

impl Drop for ShmPool {
    fn drop(&mut self) {
        if !self.mapping.is_null() {
            unsafe { libc::munmap(self.mapping as *mut libc::c_void, self.size) };
        }
    }
}

/// Manager for SHM pools.
pub struct ShmManager {
    pools: HashMap<u32, ShmPool>,
    /// Map client-side pool object id → internal pool id
    client_to_internal: HashMap<u32, u32>,
    /// Map fd → internal pool id for O(1) lookup in blit hot path.
    fd_to_internal: HashMap<RawFd, u32>,
    next_id: u32,
}

impl ShmManager {
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
            client_to_internal: HashMap::new(),
            fd_to_internal: HashMap::new(),
            next_id: 1,
        }
    }

    pub fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn create_pool(&mut self, client_pool_id: u32, fd: RawFd, size: i32) -> io::Result<u32> {
        let id = self.alloc_id();
        let pool = ShmPool::new(id, fd, size)?;
        self.fd_to_internal.insert(fd, id);
        self.pools.insert(id, pool);
        self.client_to_internal.insert(client_pool_id, id);
        Ok(id)
    }

    /// Look up pool fd by client-side pool object id.
    pub fn pool_fd_by_client_id(&self, client_pool_id: u32) -> Option<i32> {
        self.client_to_internal
            .get(&client_pool_id)
            .and_then(|&internal_id| self.pools.get(&internal_id))
            .map(|pool| pool.fd)
    }

    pub fn get_pool(&self, id: u32) -> Option<&ShmPool> {
        self.pools.get(&id)
    }

    pub fn get_pool_mut(&mut self, id: u32) -> Option<&mut ShmPool> {
        self.pools.get_mut(&id)
    }

    pub fn get_pool_by_client_id(&mut self, client_pool_id: u32) -> Option<&mut ShmPool> {
        self.client_to_internal.get(&client_pool_id)
            .and_then(|&internal_id| self.pools.get_mut(&internal_id))
    }

    pub fn destroy_pool(&mut self, id: u32) {
        if let Some(pool) = self.pools.remove(&id) {
            self.fd_to_internal.remove(&pool.fd);
        }
    }

    /// Look up pool by fd — O(1) via reverse index.
    pub fn get_pool_by_fd(&self, fd: RawFd) -> Option<&ShmPool> {
        self.fd_to_internal.get(&fd).and_then(|&internal_id| self.pools.get(&internal_id))
    }

    /// Iterate over all pools.
    pub fn pools(&self) -> impl Iterator<Item = (&u32, &ShmPool)> {
        self.pools.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_manager() {
        let mgr = ShmManager::new();
        assert_eq!(mgr.pools().count(), 0);
    }

    #[test]
    fn test_create_and_lookup_pool() {
        let mut mgr = ShmManager::new();
        // Create a temporary file for testing
        let fd = unsafe {
            libc::memfd_create(b"test_pool\0".as_ptr() as *const libc::c_char, 0)
        };
        assert!(fd >= 0);
        // Write some data
        unsafe {
            libc::ftruncate(fd, 4096);
        }

        let internal_id = mgr.create_pool(1, fd, 4096).unwrap();
        assert_eq!(internal_id, 1);

        // Lookup by client id
        assert!(mgr.pool_fd_by_client_id(1).is_some());
        assert!(mgr.pool_fd_by_client_id(99).is_none());

        // Lookup by fd
        assert!(mgr.get_pool_by_fd(fd).is_some());
        assert!(mgr.get_pool_by_fd(-1).is_none());

        // Cleanup
        mgr.destroy_pool(internal_id);
        assert_eq!(mgr.pools().count(), 0);
    }

    #[test]
    fn test_destroy_pool_removes_fd_index() {
        let mut mgr = ShmManager::new();
        let fd = unsafe {
            libc::memfd_create(b"test_pool2\0".as_ptr() as *const libc::c_char, 0)
        };
        assert!(fd >= 0);
        unsafe { libc::ftruncate(fd, 1024); }

        let id = mgr.create_pool(1, fd, 1024).unwrap();
        assert!(mgr.get_pool_by_fd(fd).is_some());

        mgr.destroy_pool(id);
        assert!(mgr.get_pool_by_fd(fd).is_none());

        unsafe { libc::close(fd); }
    }

    #[test]
    fn test_pool_read() {
        let pool = ShmPool::new(1, unsafe {
            let fd = libc::memfd_create(b"test_read\0".as_ptr() as *const libc::c_char, 0);
            libc::ftruncate(fd, 256);
            fd
        }, 256).unwrap();

        let data = pool.read(0, 10);
        assert!(data.is_some());
        assert_eq!(data.unwrap().len(), 10);

        // Read out of bounds
        let data = pool.read(250, 10);
        assert!(data.is_none());
    }
}
