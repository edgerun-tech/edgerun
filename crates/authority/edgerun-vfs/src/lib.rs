#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod glob;
mod grep;
pub mod packet;
mod stats;
mod virtual_fs;

pub use grep::GrepMatch;
pub use packet::{
    DEFAULT_OBJECT_PACKET_BYTES, VFS_OBJECT_COMPRESSION_DEFLATE_RAW, VFS_OBJECT_COMPRESSION_NONE,
    VFS_OBJECT_SEAL_AES256_GCM, VFS_WIRE_ABI_VERSION, VfsFileRef, VfsObjectPacket,
    VfsObjectSealRequest, VfsObjectTransformRef, VfsObjectUnsealRequest, VfsPacketError,
    VfsRawDeflateWatAdapter, VfsRawDeflateWatUnavailable, VfsTreeManifest, VfsWireRecord,
    file_ref_and_packets_to_entry, file_to_packets, files_to_manifest, hash_hex, object_to_packets,
    packets_to_object, prepare_file_seal_request, prepare_object_seal_request,
    prepare_object_seal_request_stored_deflate, prepare_unseal_object_from_packets,
    sealed_object_to_packets, unsealed_file_payload_to_entry, unsealed_payload_to_object,
    unsealed_payload_to_object_with_deflate, vfs_from_file_packets, vfs_wire_record_bytes,
    vfs_wire_record_from_bytes,
};
pub use stats::MemoryStats;
pub use virtual_fs::{Changeset, FileContent, FileMeta, VirtualFileSystem};

pub type Path = str;
pub type PathBuf = alloc::string::String;
pub type SharedVFS = alloc::sync::Arc<sync::RwLock<VirtualFileSystem>>;

pub fn shared_vfs(vfs: VirtualFileSystem) -> SharedVFS {
    alloc::sync::Arc::new(sync::RwLock::new(vfs))
}

mod sync {
    use core::cell::UnsafeCell;
    use core::ops::{Deref, DerefMut};
    use core::sync::atomic::{AtomicUsize, Ordering};

    pub struct RwLock<T> {
        data: UnsafeCell<T>,
        state: AtomicUsize,
    }

    unsafe impl<T: Send> Send for RwLock<T> {}
    unsafe impl<T: Send> Sync for RwLock<T> {}

    impl<T> RwLock<T> {
        pub const fn new(data: T) -> Self {
            Self {
                data: UnsafeCell::new(data),
                state: AtomicUsize::new(0),
            }
        }

        pub fn read(&self) -> RwLockReadGuard<'_, T> {
            loop {
                let state = self.state.load(Ordering::Acquire);
                if state & 1 != 0 {
                    core::hint::spin_loop();
                    continue;
                }
                if self
                    .state
                    .compare_exchange_weak(state, state + 2, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    return RwLockReadGuard { lock: self };
                }
            }
        }

        pub fn write(&self) -> RwLockWriteGuard<'_, T> {
            while self
                .state
                .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
            {
                core::hint::spin_loop();
            }
            RwLockWriteGuard { lock: self }
        }
    }

    pub struct RwLockReadGuard<'a, T> {
        lock: &'a RwLock<T>,
    }

    impl<T> Deref for RwLockReadGuard<'_, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            unsafe { &*self.lock.data.get() }
        }
    }

    impl<T> Drop for RwLockReadGuard<'_, T> {
        fn drop(&mut self) {
            self.lock.state.fetch_sub(2, Ordering::Release);
        }
    }

    pub struct RwLockWriteGuard<'a, T> {
        lock: &'a RwLock<T>,
    }

    impl<T> Deref for RwLockWriteGuard<'_, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            unsafe { &*self.lock.data.get() }
        }
    }

    impl<T> DerefMut for RwLockWriteGuard<'_, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            unsafe { &mut *self.lock.data.get() }
        }
    }

    impl<T> Drop for RwLockWriteGuard<'_, T> {
        fn drop(&mut self) {
            self.lock.state.fetch_and(!1, Ordering::Release);
        }
    }
}
