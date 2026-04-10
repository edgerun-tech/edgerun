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
    next_id: u32,
}

impl ShmManager {
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
            client_to_internal: HashMap::new(),
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
        self.pools.remove(&id);
    }

    /// Iterate over all pools.
    pub fn pools(&self) -> impl Iterator<Item = (&u32, &ShmPool)> {
        self.pools.iter()
    }
}
