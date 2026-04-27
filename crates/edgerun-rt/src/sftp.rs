//! SFTP client

extern crate alloc;

use alloc::vec::Vec;
use core::cell::RefCell;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

macro_rules! ready_future {
    ($name:ident, $output:ty) => {
        pub struct $name {
            result: Option<$output>,
        }

        impl Future for $name {
            type Output = $output;

            fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
                Poll::Ready(self.result.take().unwrap_or(Err(())))
            }
        }
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SftpFileType {
    Regular,
    Directory,
    Symlink,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SftpFileAttr {
    pub ftype: SftpFileType,
    pub size: u64,
    pub permissions: u32,
    pub atime: u32,
    pub mtime: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SftpEntry {
    pub name: Vec<u8>,
    pub attr: SftpFileAttr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SftpFile {
    path: Vec<u8>,
    data: Vec<u8>,
    attr: SftpFileAttr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SftpSymlink {
    link: Vec<u8>,
    target: Vec<u8>,
}

#[derive(Default)]
struct SftpState {
    connected: bool,
    authenticated: bool,
    files: Vec<SftpFile>,
    dirs: Vec<Vec<u8>>,
    handles: Vec<Vec<u8>>,
    symlinks: Vec<SftpSymlink>,
}

pub struct SftpClient {
    state: RefCell<SftpState>,
}

impl SftpClient {
    pub fn new() -> Self {
        let mut state = SftpState::default();
        state.dirs.push(b"/".to_vec());
        Self {
            state: RefCell::new(state),
        }
    }

    pub fn connect(&self, host: &[u8], port: u16) -> SftpConnectFuture {
        let result = if !host.is_empty() && port != 0 {
            self.state.borrow_mut().connected = true;
            Ok(())
        } else {
            Err(())
        };
        SftpConnectFuture {
            result: Some(result),
        }
    }

    pub fn login(&self, user: &[u8], pass: &[u8]) -> SftpLoginFuture {
        let result = if self.state.borrow().connected && !user.is_empty() && !pass.is_empty() {
            self.state.borrow_mut().authenticated = true;
            Ok(())
        } else {
            Err(())
        };
        SftpLoginFuture {
            result: Some(result),
        }
    }

    pub fn open(&self, path: &[u8], read: bool) -> SftpOpenFuture {
        let path = normalize_path(path);
        let mut state = self.state.borrow_mut();
        let result = if !state.authenticated || path == b"/" {
            Err(())
        } else if find_file(&state.files, &path).is_some() {
            if !state.handles.iter().any(|handle| handle == &path) {
                state.handles.push(path.clone());
            }
            Ok(path)
        } else if read {
            Err(())
        } else {
            state.files.push(SftpFile {
                path: path.clone(),
                data: Vec::new(),
                attr: default_attr(SftpFileType::Regular, 0),
            });
            state.handles.push(path.clone());
            Ok(path)
        };
        SftpOpenFuture {
            result: Some(result),
        }
    }

    pub fn read(&self, handle: &[u8], offset: u64, len: usize) -> SftpReadFuture {
        let state = self.state.borrow();
        let result = if !state.handles.iter().any(|stored| stored.as_slice() == handle) {
            Err(())
        } else {
            find_file(&state.files, handle)
                .map(|file| {
                    let start = (offset as usize).min(file.data.len());
                    let end = start.saturating_add(len).min(file.data.len());
                    file.data[start..end].to_vec()
                })
                .ok_or(())
        };
        SftpReadFuture {
            result: Some(result),
        }
    }

    pub fn write(&self, handle: &[u8], offset: u64, data: &[u8]) -> SftpWriteFuture {
        let mut state = self.state.borrow_mut();
        let result = if !state.handles.iter().any(|stored| stored.as_slice() == handle) {
            Err(())
        } else {
            find_file_mut(&mut state.files, handle)
                .map(|file| {
                    let offset = offset as usize;
                    if file.data.len() < offset {
                        file.data.resize(offset, 0);
                    }
                    let end = offset.saturating_add(data.len());
                    if file.data.len() < end {
                        file.data.resize(end, 0);
                    }
                    file.data[offset..end].copy_from_slice(data);
                    file.attr.size = file.data.len() as u64;
                    data.len()
                })
                .ok_or(())
        };
        SftpWriteFuture {
            result: Some(result),
        }
    }

    pub fn close(&self, handle: &[u8]) -> SftpCloseFuture {
        let mut state = self.state.borrow_mut();
        let before = state.handles.len();
        state.handles.retain(|stored| stored.as_slice() != handle);
        SftpCloseFuture {
            result: Some(if before == state.handles.len() {
                Err(())
            } else {
                Ok(())
            }),
        }
    }

    pub fn readdir(&self, path: &[u8]) -> SftpReaddirFuture {
        let path = normalize_path(path);
        let state = self.state.borrow();
        let mut entries = Vec::new();

        for dir in &state.dirs {
            if parent_path(dir).as_slice() == path.as_slice() && dir.as_slice() != path.as_slice() {
                entries.push(SftpEntry {
                    name: basename(dir),
                    attr: default_attr(SftpFileType::Directory, 0),
                });
            }
        }
        for file in &state.files {
            if parent_path(&file.path).as_slice() == path.as_slice() {
                entries.push(SftpEntry {
                    name: basename(&file.path),
                    attr: file.attr.clone(),
                });
            }
        }
        for link in &state.symlinks {
            if parent_path(&link.link).as_slice() == path.as_slice() {
                entries.push(SftpEntry {
                    name: basename(&link.link),
                    attr: default_attr(SftpFileType::Symlink, link.target.len() as u64),
                });
            }
        }

        SftpReaddirFuture {
            result: Some(if state.dirs.iter().any(|dir| dir == &path) {
                Ok(entries)
            } else {
                Err(())
            }),
        }
    }

    pub fn mkdir(&self, path: &[u8]) -> SftpMkdirFuture {
        let path = normalize_path(path);
        let mut state = self.state.borrow_mut();
        let result = if path == b"/" || state.dirs.iter().any(|dir| dir == &path) {
            Err(())
        } else {
            state.dirs.push(path);
            Ok(())
        };
        SftpMkdirFuture {
            result: Some(result),
        }
    }

    pub fn rmdir(&self, path: &[u8]) -> SftpRmdirFuture {
        let path = normalize_path(path);
        let mut state = self.state.borrow_mut();
        let has_children = state
            .files
            .iter()
            .any(|file| parent_path(&file.path) == path)
            || state
                .dirs
                .iter()
                .any(|dir| parent_path(dir) == path && dir != &path);
        let result = if path == b"/" || has_children {
            Err(())
        } else {
            let before = state.dirs.len();
            state.dirs.retain(|dir| dir != &path);
            if before == state.dirs.len() {
                Err(())
            } else {
                Ok(())
            }
        };
        SftpRmdirFuture {
            result: Some(result),
        }
    }

    pub fn remove(&self, path: &[u8]) -> SftpRemoveFuture {
        let path = normalize_path(path);
        let mut state = self.state.borrow_mut();
        let before = state.files.len();
        state.files.retain(|file| file.path != path);
        SftpRemoveFuture {
            result: Some(if before == state.files.len() {
                Err(())
            } else {
                Ok(())
            }),
        }
    }

    pub fn rename(&self, old: &[u8], new: &[u8]) -> SftpRenameFuture {
        let old = normalize_path(old);
        let new = normalize_path(new);
        let mut state = self.state.borrow_mut();
        let result = if let Some(file) = find_file_mut(&mut state.files, &old) {
            file.path = new.clone();
            for handle in &mut state.handles {
                if handle == &old {
                    *handle = new.clone();
                }
            }
            Ok(())
        } else if let Some(dir) = state.dirs.iter_mut().find(|dir| **dir == old) {
            *dir = new;
            Ok(())
        } else {
            Err(())
        };
        SftpRenameFuture {
            result: Some(result),
        }
    }

    pub fn stat(&self, path: &[u8]) -> SftpStatFuture {
        SftpStatFuture {
            result: Some(self.stat_path(path, true)),
        }
    }

    pub fn lstat(&self, path: &[u8]) -> SftpStatFuture {
        SftpStatFuture {
            result: Some(self.stat_path(path, false)),
        }
    }

    pub fn fstat(&self, handle: &[u8]) -> SftpStatFuture {
        let state = self.state.borrow();
        let result = if state.handles.iter().any(|stored| stored.as_slice() == handle) {
            find_file(&state.files, handle)
                .map(|file| file.attr.clone())
                .ok_or(())
        } else {
            Err(())
        };
        SftpStatFuture {
            result: Some(result),
        }
    }

    pub fn setstat(&self, path: &[u8], attr: &SftpFileAttr) -> SftpSetstatFuture {
        let path = normalize_path(path);
        let mut state = self.state.borrow_mut();
        let result = find_file_mut(&mut state.files, &path)
            .map(|file| {
                file.attr = attr.clone();
            })
            .ok_or(());
        SftpSetstatFuture {
            result: Some(result),
        }
    }

    pub fn symlink(&self, target: &[u8], link: &[u8]) -> SftpSymlinkFuture {
        let target = normalize_path(target);
        let link = normalize_path(link);
        let mut state = self.state.borrow_mut();
        state.symlinks.retain(|stored| stored.link != link);
        state.symlinks.push(SftpSymlink { link, target });
        SftpSymlinkFuture {
            result: Some(Ok(())),
        }
    }

    pub fn readlink(&self, path: &[u8]) -> SftpReadlinkFuture {
        let path = normalize_path(path);
        let state = self.state.borrow();
        SftpReadlinkFuture {
            result: Some(
                state
                    .symlinks
                    .iter()
                    .find(|link| link.link == path)
                    .map(|link| link.target.clone())
                    .ok_or(()),
            ),
        }
    }

    pub fn realpath(&self, path: &[u8]) -> SftpRealpathFuture {
        SftpRealpathFuture {
            result: Some(Ok(normalize_path(path))),
        }
    }

    pub fn quit(&self) -> SftpQuitFuture {
        let mut state = self.state.borrow_mut();
        state.connected = false;
        state.authenticated = false;
        state.handles.clear();
        SftpQuitFuture {
            result: Some(Ok(())),
        }
    }

    fn stat_path(&self, path: &[u8], follow_symlink: bool) -> Result<SftpFileAttr, ()> {
        let path = normalize_path(path);
        let state = self.state.borrow();
        if let Some(file) = find_file(&state.files, &path) {
            return Ok(file.attr.clone());
        }
        if state.dirs.iter().any(|dir| dir == &path) {
            return Ok(default_attr(SftpFileType::Directory, 0));
        }
        if let Some(link) = state.symlinks.iter().find(|link| link.link == path) {
            if follow_symlink {
                return self.stat_path(&link.target, false);
            }
            return Ok(default_attr(SftpFileType::Symlink, link.target.len() as u64));
        }
        Err(())
    }
}

impl Default for SftpClient {
    fn default() -> Self {
        Self::new()
    }
}

ready_future!(SftpConnectFuture, Result<(), ()>);
ready_future!(SftpLoginFuture, Result<(), ()>);
ready_future!(SftpOpenFuture, Result<Vec<u8>, ()>);
ready_future!(SftpReadFuture, Result<Vec<u8>, ()>);
ready_future!(SftpWriteFuture, Result<usize, ()>);
ready_future!(SftpCloseFuture, Result<(), ()>);
ready_future!(SftpReaddirFuture, Result<Vec<SftpEntry>, ()>);
ready_future!(SftpMkdirFuture, Result<(), ()>);
ready_future!(SftpRmdirFuture, Result<(), ()>);
ready_future!(SftpRemoveFuture, Result<(), ()>);
ready_future!(SftpRenameFuture, Result<(), ()>);
ready_future!(SftpStatFuture, Result<SftpFileAttr, ()>);
ready_future!(SftpSetstatFuture, Result<(), ()>);
ready_future!(SftpSymlinkFuture, Result<(), ()>);
ready_future!(SftpReadlinkFuture, Result<Vec<u8>, ()>);
ready_future!(SftpRealpathFuture, Result<Vec<u8>, ()>);
ready_future!(SftpQuitFuture, Result<(), ()>);

fn default_attr(ftype: SftpFileType, size: u64) -> SftpFileAttr {
    SftpFileAttr {
        ftype,
        size,
        permissions: match ftype {
            SftpFileType::Directory => 0o755,
            _ => 0o644,
        },
        atime: 0,
        mtime: 0,
    }
}

fn find_file<'a>(files: &'a [SftpFile], path: &[u8]) -> Option<&'a SftpFile> {
    files.iter().find(|file| file.path.as_slice() == path)
}

fn find_file_mut<'a>(files: &'a mut [SftpFile], path: &[u8]) -> Option<&'a mut SftpFile> {
    files.iter_mut().find(|file| file.path.as_slice() == path)
}

fn normalize_path(path: &[u8]) -> Vec<u8> {
    if path.is_empty() || path == b"/" {
        return b"/".to_vec();
    }

    let mut normalized = Vec::with_capacity(path.len() + 1);
    if path[0] != b'/' {
        normalized.push(b'/');
    }
    normalized.extend_from_slice(path);

    while normalized.len() > 1 && normalized.last() == Some(&b'/') {
        normalized.pop();
    }

    normalized
}

fn parent_path(path: &[u8]) -> Vec<u8> {
    let path = normalize_path(path);
    match path.iter().rposition(|byte| *byte == b'/') {
        Some(0) | None => b"/".to_vec(),
        Some(index) => path[..index].to_vec(),
    }
}

fn basename(path: &[u8]) -> Vec<u8> {
    let path = normalize_path(path);
    match path.iter().rposition(|byte| *byte == b'/') {
        Some(index) => path[index + 1..].to_vec(),
        None => path,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block_on;

    #[test]
    fn sftp_session_manages_files_dirs_and_links() {
        let client = SftpClient::new();

        block_on(client.connect(b"localhost", 22)).unwrap();
        block_on(client.login(b"user", b"pass")).unwrap();
        block_on(client.mkdir(b"/docs")).unwrap();

        let handle = block_on(client.open(b"/docs/readme.txt", false)).unwrap();
        assert_eq!(block_on(client.write(&handle, 0, b"hello")).unwrap(), 5);
        assert_eq!(block_on(client.read(&handle, 1, 3)).unwrap(), b"ell");
        assert_eq!(block_on(client.fstat(&handle)).unwrap().size, 5);
        block_on(client.close(&handle)).unwrap();

        let entries = block_on(client.readdir(b"/docs")).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, b"readme.txt");

        block_on(client.symlink(b"/docs/readme.txt", b"/docs/current")).unwrap();
        assert_eq!(block_on(client.readlink(b"/docs/current")).unwrap(), b"/docs/readme.txt");
        assert_eq!(block_on(client.lstat(b"/docs/current")).unwrap().ftype, SftpFileType::Symlink);

        block_on(client.rename(b"/docs/readme.txt", b"/docs/notes.txt")).unwrap();
        assert_eq!(block_on(client.stat(b"/docs/notes.txt")).unwrap().size, 5);
        block_on(client.remove(b"/docs/notes.txt")).unwrap();
        block_on(client.rmdir(b"/docs")).unwrap_err();
    }
}
