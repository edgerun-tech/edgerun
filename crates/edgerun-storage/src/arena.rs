// SPDX-License-Identifier: GPL-2.0-only
//! Arena allocation for zero-per-event memory allocation.
//!
//! Arena allocators provide:
//! - O(1) allocation (just bump pointer)
//! - No fragmentation
//! - Bulk deallocation
//! - Cache-friendly contiguous memory

use std::alloc::{alloc, dealloc, Layout};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Arena allocator for fixed-size chunks.
pub struct Arena {
    chunks: Vec<Chunk>,
    current_chunk: usize,
    chunk_size: usize,
    allocated: AtomicUsize,
}

/// A single chunk of memory in the arena.
struct Chunk {
    memory: NonNull<u8>,
    layout: Layout,
    used: usize,
}

impl Arena {
    /// Create a new arena with specified chunk size.
    pub fn new(chunk_size: usize) -> Self {
        let mut arena = Self {
            chunks: Vec::new(),
            current_chunk: 0,
            chunk_size,
            allocated: AtomicUsize::new(0),
        };
        arena.allocate_chunk();
        arena
    }

    /// Allocate memory from the arena.
    ///
    /// Returns a pointer to allocated memory or None if allocation fails.
    pub fn allocate(&mut self, size: usize, align: usize) -> Option<NonNull<u8>> {
        let layout = Layout::from_size_align(size, align).ok()?;

        // Try to allocate from current chunk
        if let Some(ptr) = self.allocate_in_current_chunk(layout) {
            self.allocated.fetch_add(size, Ordering::Relaxed);
            return Some(ptr);
        }

        // Current chunk is full, allocate new chunk
        self.allocate_chunk();
        self.current_chunk = self.chunks.len() - 1;

        // Try again
        if let Some(ptr) = self.allocate_in_current_chunk(layout) {
            self.allocated.fetch_add(size, Ordering::Relaxed);
            return Some(ptr);
        }

        None
    }

    /// Allocate a T from the arena.
    pub fn allocate_type<T>(&mut self) -> Option<NonNull<T>> {
        let layout = Layout::new::<T>();
        self.allocate(layout.size(), layout.align())
            .map(|ptr| ptr.cast())
    }

    /// Allocate a slice of T from the arena.
    pub fn allocate_slice<T>(&mut self, len: usize) -> Option<NonNull<T>> {
        let layout = Layout::array::<T>(len).ok()?;
        self.allocate(layout.size(), layout.align())
            .map(|ptr| ptr.cast())
    }

    /// Reset the arena, keeping chunks for reuse.
    pub fn reset(&mut self) {
        for chunk in &mut self.chunks {
            chunk.used = 0;
        }
        self.current_chunk = 0;
        self.allocated.store(0, Ordering::Relaxed);
    }

    /// Get total bytes allocated.
    pub fn bytes_allocated(&self) -> usize {
        self.allocated.load(Ordering::Relaxed)
    }

    /// Get total bytes reserved (including unused space in chunks).
    pub fn bytes_reserved(&self) -> usize {
        self.chunks.len() * self.chunk_size
    }

    fn allocate_chunk(&mut self) {
        let layout = Layout::from_size_align(self.chunk_size, 64).expect("Invalid chunk size");

        unsafe {
            let ptr = alloc(layout);
            if let Some(non_null) = NonNull::new(ptr) {
                self.chunks.push(Chunk {
                    memory: non_null,
                    layout,
                    used: 0,
                });
            }
        }
    }

    fn allocate_in_current_chunk(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let chunk = self.chunks.get_mut(self.current_chunk)?;

        // Align current position
        let align_mask = layout.align() - 1;
        let aligned_used = (chunk.used + align_mask) & !align_mask;

        if aligned_used + layout.size() <= self.chunk_size {
            let ptr = unsafe { NonNull::new_unchecked(chunk.memory.as_ptr().add(aligned_used)) };
            chunk.used = aligned_used + layout.size();
            Some(ptr)
        } else {
            None
        }
    }
}

impl Drop for Arena {
    fn drop(&mut self) {
        for chunk in &self.chunks {
            unsafe {
                dealloc(chunk.memory.as_ptr(), chunk.layout);
            }
        }
    }
}

/// Thread-local arena allocator with object pooling.
pub struct ObjectPool<T> {
    arena: Arena,
    free_list: Vec<NonNull<T>>,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> ObjectPool<T> {
    /// Create a new object pool.
    pub fn new(capacity: usize) -> Self {
        let chunk_size = std::mem::size_of::<T>() * capacity + 1024;
        Self {
            arena: Arena::new(chunk_size),
            free_list: Vec::with_capacity(capacity),
            _phantom: std::marker::PhantomData,
        }
    }

    /// Acquire an object from the pool.
    pub fn acquire(&mut self) -> Option<PoolPtr<'_, T>> {
        // Try to reuse from free list
        if let Some(ptr) = self.free_list.pop() {
            return Some(PoolPtr { ptr, pool: self });
        }

        // Allocate new object from arena
        let layout = std::alloc::Layout::new::<T>();
        self.arena
            .allocate(layout.size(), layout.align())
            .map(|ptr| PoolPtr {
                ptr: ptr.cast(),
                pool: self,
            })
    }

    /// Release an object back to the pool.
    fn release(&mut self, ptr: NonNull<T>) {
        self.free_list.push(ptr);
    }

    /// Reset the pool.
    pub fn reset(&mut self) {
        self.arena.reset();
        self.free_list.clear();
    }

    /// Get pool statistics.
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            free_objects: self.free_list.len(),
            total_allocated: self.arena.bytes_allocated() / std::mem::size_of::<T>(),
            bytes_reserved: self.arena.bytes_reserved(),
        }
    }
}

/// Smart pointer for pooled objects.
pub struct PoolPtr<'a, T> {
    ptr: NonNull<T>,
    pool: &'a mut ObjectPool<T>,
}

impl<'a, T> std::ops::Deref for PoolPtr<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.ptr.as_ref() }
    }
}

impl<'a, T> std::ops::DerefMut for PoolPtr<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.ptr.as_mut() }
    }
}

impl<'a, T> Drop for PoolPtr<'a, T> {
    fn drop(&mut self) {
        self.pool.release(self.ptr);
    }
}

/// Pool statistics.
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub free_objects: usize,
    pub total_allocated: usize,
    pub bytes_reserved: usize,
}

/// Event arena optimized for event allocation.
pub struct EventArena {
    data_arena: Arena,
    event_arena: Arena,
}

impl EventArena {
    /// Create a new event arena.
    pub fn new() -> Self {
        Self {
            data_arena: Arena::new(4 * 1024 * 1024), // 4MB data chunks
            event_arena: Arena::new(1024 * 1024),    // 1MB event chunks
        }
    }

    /// Allocate event payload data.
    pub fn allocate_payload(&mut self, size: usize) -> Option<NonNull<u8>> {
        self.data_arena.allocate(size, 1)
    }

    /// Get memory usage statistics.
    pub fn stats(&self) -> ArenaStats {
        ArenaStats {
            data_bytes: self.data_arena.bytes_allocated(),
            data_reserved: self.data_arena.bytes_reserved(),
            event_bytes: self.event_arena.bytes_allocated(),
            event_reserved: self.event_arena.bytes_reserved(),
        }
    }

    /// Reset both arenas.
    pub fn reset(&mut self) {
        self.data_arena.reset();
        self.event_arena.reset();
    }
}

/// Arena statistics.
#[derive(Debug, Clone)]
pub struct ArenaStats {
    pub data_bytes: usize,
    pub data_reserved: usize,
    pub event_bytes: usize,
    pub event_reserved: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arena_allocation() {
        let mut arena = Arena::new(1024);

        // Allocate some memory
        let ptr1 = arena.allocate(100, 8).unwrap();
        let ptr2 = arena.allocate(200, 8).unwrap();

        assert_ne!(ptr1.as_ptr(), ptr2.as_ptr());
        assert!(arena.bytes_allocated() >= 300);
    }

    #[test]
    fn test_object_pool() {
        let mut pool: ObjectPool<u64> = ObjectPool::new(100);

        // Acquire and release objects
        {
            let mut obj = pool.acquire().unwrap();
            *obj = 42;
        } // Released back to pool

        let stats = pool.stats();
        assert_eq!(stats.free_objects, 1);
    }

    #[test]
    fn test_arena_reset() {
        let mut arena = Arena::new(1024);

        arena.allocate(100, 8).unwrap();
        let allocated_before = arena.bytes_allocated();
        assert!(allocated_before > 0);

        arena.reset();
        assert_eq!(arena.bytes_allocated(), 0);
    }
}
