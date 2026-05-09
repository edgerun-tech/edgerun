use std::sync::Arc;
use std::sync::LazyLock;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

use async_trait::async_trait;
use tokio::io;

use crate::absolute_path::AbsolutePathBuf;

const MAX_READ_FILE_BYTES: u64 = 512 * 1024 * 1024;

pub type FileSystemResult<T> = io::Result<T>;

#[derive(Clone, Copy, Debug)]
pub struct CreateDirectoryOptions {
    pub recursive: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct RemoveOptions {
    pub recursive: bool,
    pub force: bool,
}

#[derive(Clone, Debug)]
pub struct FileMetadata {
    pub is_directory: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    #[allow(dead_code)]
    pub created_at_ms: u64,
    #[allow(dead_code)]
    pub modified_at_ms: u64,
}

#[derive(Clone, Debug)]
pub struct FileSystemSandboxContext;

#[async_trait]
pub trait ExecutorFileSystem: Send + Sync {
    async fn read_file(
        &self,
        path: &AbsolutePathBuf,
        sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<Vec<u8>>;

    async fn write_file(
        &self,
        path: &AbsolutePathBuf,
        contents: Vec<u8>,
        sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<()>;

    async fn create_directory(
        &self,
        path: &AbsolutePathBuf,
        options: CreateDirectoryOptions,
        sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<()>;

    async fn get_metadata(
        &self,
        path: &AbsolutePathBuf,
        sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<FileMetadata>;

    async fn remove(
        &self,
        path: &AbsolutePathBuf,
        options: RemoveOptions,
        sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<()>;

    async fn read_file_text(
        &self,
        path: &AbsolutePathBuf,
        sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<String> {
        let bytes = self.read_file(path, sandbox).await?;
        String::from_utf8(bytes)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
    }
}

pub static LOCAL_FS: LazyLock<Arc<dyn ExecutorFileSystem>> =
    LazyLock::new(|| Arc::new(LocalFileSystem));

#[derive(Clone, Default)]
pub struct LocalFileSystem;

#[async_trait]
impl ExecutorFileSystem for LocalFileSystem {
    async fn read_file(
        &self,
        path: &AbsolutePathBuf,
        _sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<Vec<u8>> {
        let metadata = tokio::fs::metadata(path.as_path()).await?;
        if metadata.len() > MAX_READ_FILE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("file is too large to read: limit is {MAX_READ_FILE_BYTES} bytes"),
            ));
        }
        tokio::fs::read(path.as_path()).await
    }

    async fn write_file(
        &self,
        path: &AbsolutePathBuf,
        contents: Vec<u8>,
        _sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<()> {
        tokio::fs::write(path.as_path(), contents).await
    }

    async fn create_directory(
        &self,
        path: &AbsolutePathBuf,
        options: CreateDirectoryOptions,
        _sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<()> {
        if options.recursive {
            tokio::fs::create_dir_all(path.as_path()).await
        } else {
            tokio::fs::create_dir(path.as_path()).await
        }
    }

    async fn get_metadata(
        &self,
        path: &AbsolutePathBuf,
        _sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<FileMetadata> {
        let metadata = tokio::fs::metadata(path.as_path()).await?;
        let symlink_metadata = tokio::fs::symlink_metadata(path.as_path()).await?;
        Ok(FileMetadata {
            is_directory: metadata.is_dir(),
            is_file: metadata.is_file(),
            is_symlink: symlink_metadata.file_type().is_symlink(),
            created_at_ms: metadata.created().ok().map_or(0, system_time_to_unix_ms),
            modified_at_ms: metadata.modified().ok().map_or(0, system_time_to_unix_ms),
        })
    }

    async fn remove(
        &self,
        path: &AbsolutePathBuf,
        options: RemoveOptions,
        _sandbox: Option<&FileSystemSandboxContext>,
    ) -> FileSystemResult<()> {
        match tokio::fs::symlink_metadata(path.as_path()).await {
            Ok(metadata) => {
                if metadata.file_type().is_dir() {
                    if options.recursive {
                        tokio::fs::remove_dir_all(path.as_path()).await
                    } else {
                        tokio::fs::remove_dir(path.as_path()).await
                    }
                } else {
                    tokio::fs::remove_file(path.as_path()).await
                }
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound && options.force => Ok(()),
            Err(err) => Err(err),
        }
    }
}

fn system_time_to_unix_ms(time: SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or(0)
}
