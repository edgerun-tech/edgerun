//! Edgerun-owned compatibility surface for Tokio-shaped APIs.

extern crate std;

use std::future::Future;

#[cfg(feature = "edgerun-runtime")]
pub mod edgerun {
    pub use edgerun_runtime::rt::*;
}

#[doc(hidden)]
pub use edgerun_futures as futures;

pub use task::spawn;

#[cfg(feature = "macros")]
pub use edgerun_tokio_macros::test;

#[macro_export]
macro_rules! pin {
    ($($x:ident),+ $(,)?) => {
        $(
            let mut $x = ::std::pin::pin!($x);
        )+
    };
}

#[macro_export]
macro_rules! join {
    ($($future:expr),+ $(,)?) => {
        ( $( $future.await ),+ )
    };
}

#[macro_export]
macro_rules! select {
    (biased; $($rest:tt)*) => {
        $crate::select! { $($rest)* }
    };
    ($binding:ident = $future:expr $(, if $cond:expr)? => $body:tt $(,)? $binding2:ident = $future2:expr $(, if $cond2:expr)? => $body2:tt $(,)? _ = $future3:expr $(, if $cond3:expr)? => $body3:tt $(,)?) => {
        {
            enum __EdgerunSelect<A, B, C> {
                First(A),
                Second(B),
                Third(C),
            }
            let selected = {
                let fut1 = async {
                    $(if !$cond { ::std::future::pending::<()>().await; })?
                    $future.await
                };
                let fut2 = async {
                    $(if !$cond2 { ::std::future::pending::<()>().await; })?
                    $future2.await
                };
                let fut3 = async {
                    $(if !$cond3 { ::std::future::pending::<()>().await; })?
                    $future3.await
                };
                let mut fut1 = ::std::boxed::Box::pin(fut1);
                let mut fut2 = ::std::boxed::Box::pin(fut2);
                let mut fut3 = ::std::boxed::Box::pin(fut3);
                $crate::futures::future::poll_fn(|cx| {
                    if let ::std::task::Poll::Ready(value) = ::std::future::Future::poll(fut1.as_mut(), cx) {
                        return ::std::task::Poll::Ready(__EdgerunSelect::First(value));
                    }
                    if let ::std::task::Poll::Ready(value) = ::std::future::Future::poll(fut2.as_mut(), cx) {
                        return ::std::task::Poll::Ready(__EdgerunSelect::Second(value));
                    }
                    if let ::std::task::Poll::Ready(value) = ::std::future::Future::poll(fut3.as_mut(), cx) {
                        return ::std::task::Poll::Ready(__EdgerunSelect::Third(value));
                    }
                    ::std::task::Poll::Pending
                }).await
            };
            match selected {
                __EdgerunSelect::First($binding) => $body,
                __EdgerunSelect::Second($binding2) => $body2,
                __EdgerunSelect::Third(_) => $body3,
            }
        }
    };
    (_ = $future:expr $(, if $cond:expr)? => $body:tt, $binding2:ident = $future2:expr $(, if $cond2:expr)? => $body2:expr $(,)?) => {
        {
            let selected = {
                let fut1 = async {
                    $(if !$cond { ::std::future::pending::<()>().await; })?
                    $future.await
                };
                let fut2 = async {
                    $(if !$cond2 { ::std::future::pending::<()>().await; })?
                    $future2.await
                };
                let fut1 = ::std::boxed::Box::pin(fut1);
                let fut2 = ::std::boxed::Box::pin(fut2);
                $crate::futures::future::select(fut1, fut2).await
            };
            match selected {
                $crate::futures::future::Either::Left((_, _)) => $body,
                $crate::futures::future::Either::Right(($binding2, _)) => $body2,
            }
        }
    };
    ($binding:ident = $future:expr $(, if $cond:expr)? => $body:tt $(,)? _ = $future2:expr $(, if $cond2:expr)? => $body2:tt $(,)?) => {
        {
            let selected = {
                let fut1 = async {
                    $(if !$cond { ::std::future::pending::<()>().await; })?
                    $future.await
                };
                let fut2 = async {
                    $(if !$cond2 { ::std::future::pending::<()>().await; })?
                    $future2.await
                };
                let fut1 = ::std::boxed::Box::pin(fut1);
                let fut2 = ::std::boxed::Box::pin(fut2);
                $crate::futures::future::select(fut1, fut2).await
            };
            match selected {
                $crate::futures::future::Either::Left(($binding, _)) => $body,
                $crate::futures::future::Either::Right((_, _)) => $body2,
            }
        }
    };
    (_ = $future:expr $(, if $cond:expr)? => $body:tt $(,)? $binding2:ident = $future2:expr $(, if $cond2:expr)? => $body2:tt $(,)?) => {
        {
            let selected = {
                let fut1 = async {
                    $(if !$cond { ::std::future::pending::<()>().await; })?
                    $future.await
                };
                let fut2 = async {
                    $(if !$cond2 { ::std::future::pending::<()>().await; })?
                    $future2.await
                };
                let fut1 = ::std::boxed::Box::pin(fut1);
                let fut2 = ::std::boxed::Box::pin(fut2);
                $crate::futures::future::select(fut1, fut2).await
            };
            match selected {
                $crate::futures::future::Either::Left((_, _)) => $body,
                $crate::futures::future::Either::Right(($binding2, _)) => $body2,
            }
        }
    };
    ($binding:ident = $future:expr $(, if $cond:expr)? => $body:tt $(,)? $binding2:ident = $future2:expr $(, if $cond2:expr)? => $body2:tt $(,)?) => {
        {
            let selected = {
                let fut1 = async {
                    $(if !$cond { ::std::future::pending::<()>().await; })?
                    $future.await
                };
                let fut2 = async {
                    $(if !$cond2 { ::std::future::pending::<()>().await; })?
                    $future2.await
                };
                let fut1 = ::std::boxed::Box::pin(fut1);
                let fut2 = ::std::boxed::Box::pin(fut2);
                $crate::futures::future::select(fut1, fut2).await
            };
            match selected {
                $crate::futures::future::Either::Left(($binding, _)) => $body,
                $crate::futures::future::Either::Right(($binding2, _)) => $body2,
            }
        }
    };
    (_ = $future:expr $(, if $cond:expr)? => $body:tt $(,)? _ = $future2:expr $(, if $cond2:expr)? => $body2:tt $(,)?) => {
        {
            let selected = {
                let fut1 = async {
                    $(if !$cond { ::std::future::pending::<()>().await; })?
                    $future.await
                };
                let fut2 = async {
                    $(if !$cond2 { ::std::future::pending::<()>().await; })?
                    $future2.await
                };
                let fut1 = ::std::boxed::Box::pin(fut1);
                let fut2 = ::std::boxed::Box::pin(fut2);
                $crate::futures::future::select(fut1, fut2).await
            };
            match selected {
                $crate::futures::future::Either::Left((_, _)) => $body,
                $crate::futures::future::Either::Right((_, _)) => $body2,
            }
        }
    };
    ($binding:pat = $future:expr $(, if $cond:expr)? => $body:expr, _ = $future2:expr $(, if $cond2:expr)? => $body2:expr $(,)?) => {
        {
            let fut1 = async {
                $(if !$cond { ::std::future::pending::<()>().await; })?
                let $binding = $future.await;
                $body
            };
            let fut2 = async {
                $(if !$cond2 { ::std::future::pending::<()>().await; })?
                let _ = $future2.await;
                $body2
            };
            let fut1 = ::std::boxed::Box::pin(fut1);
                let fut2 = ::std::boxed::Box::pin(fut2);
            match $crate::futures::future::select(fut1, fut2).await {
                $crate::futures::future::Either::Left((value, _)) => value,
                $crate::futures::future::Either::Right((value, _)) => value,
            }
        }
    };
    (_ = $future:expr $(, if $cond:expr)? => $body:expr, $binding2:pat = $future2:expr $(, if $cond2:expr)? => $body2:expr $(,)?) => {
        {
            let fut1 = async {
                $(if !$cond { ::std::future::pending::<()>().await; })?
                let _ = $future.await;
                $body
            };
            let fut2 = async {
                $(if !$cond2 { ::std::future::pending::<()>().await; })?
                let $binding2 = $future2.await;
                $body2
            };
            let fut1 = ::std::boxed::Box::pin(fut1);
                let fut2 = ::std::boxed::Box::pin(fut2);
            match $crate::futures::future::select(fut1, fut2).await {
                $crate::futures::future::Either::Left((value, _)) => value,
                $crate::futures::future::Either::Right((value, _)) => value,
            }
        }
    };
    ($binding:pat = $future:expr $(, if $cond:expr)? => $body:expr, $binding2:pat = $future2:expr $(, if $cond2:expr)? => $body2:expr $(,)?) => {
        {
            let fut1 = async {
                $(if !$cond { ::std::future::pending::<()>().await; })?
                let $binding = $future.await;
                $body
            };
            let fut2 = async {
                $(if !$cond2 { ::std::future::pending::<()>().await; })?
                let $binding2 = $future2.await;
                $body2
            };
            let fut1 = ::std::boxed::Box::pin(fut1);
                let fut2 = ::std::boxed::Box::pin(fut2);
            match $crate::futures::future::select(fut1, fut2).await {
                $crate::futures::future::Either::Left((value, _)) => value,
                $crate::futures::future::Either::Right((value, _)) => value,
            }
        }
    };
    (_ = $future:expr $(, if $cond:expr)? => $body:expr, _ = $future2:expr $(, if $cond2:expr)? => $body2:expr $(,)?) => {
        {
            let fut1 = async {
                $(if !$cond { ::std::future::pending::<()>().await; })?
                let _ = $future.await;
                $body
            };
            let fut2 = async {
                $(if !$cond2 { ::std::future::pending::<()>().await; })?
                let _ = $future2.await;
                $body2
            };
            let fut1 = ::std::boxed::Box::pin(fut1);
                let fut2 = ::std::boxed::Box::pin(fut2);
            match $crate::futures::future::select(fut1, fut2).await {
                $crate::futures::future::Either::Left((value, _)) => value,
                $crate::futures::future::Either::Right((value, _)) => value,
            }
        }
    };
    ($binding:pat = $future:expr => $body:expr $(,)?) => {
        {
            let $binding = $future.await;
            $body
        }
    };
    (_ = $future:expr => $body:expr $(,)?) => {
        {
            let _ = $future.await;
            $body
        }
    };
}

pub mod runtime {
    use super::*;

    pub type Runtime = edgerun_runtime::rt::Runtime;

    #[derive(Clone, Copy)]
    pub struct Handle(edgerun_runtime::rt::RuntimeHandle);

    pub struct Builder {
        inner: edgerun_runtime::rt::Builder,
    }

    impl Builder {
        pub fn new_current_thread() -> Self {
            Self {
                inner: edgerun_runtime::rt::Builder::new_multi_thread(),
            }
        }

        pub fn new_multi_thread() -> Self {
            Self {
                inner: edgerun_runtime::rt::Builder::new_multi_thread(),
            }
        }

        pub fn worker_threads(&mut self, threads: usize) -> &mut Self {
            self.inner.worker_threads(threads);
            self
        }

        pub fn max_blocking_threads(&mut self, threads: usize) -> &mut Self {
            self.inner.max_blocking_threads(threads);
            self
        }

        pub fn enable_all(&mut self) -> &mut Self {
            self.inner.enable_all();
            self
        }

        pub fn build(&self) -> Result<Runtime, edgerun_runtime::rt::Error> {
            self.inner.build()
        }
    }

    impl Handle {
        pub fn current() -> Self {
            Self(edgerun_runtime::rt::RuntimeHandle)
        }

        pub fn spawn<F>(&self, future: F) -> edgerun_runtime::rt::JoinHandle<F::Output>
        where
            F: Future + Send + 'static,
            F::Output: Send + 'static,
        {
            self.0.spawn(future)
        }

        pub fn spawn_blocking<F, R>(&self, f: F) -> edgerun_runtime::rt::JoinHandle<R>
        where
            F: FnOnce() -> R + Send + 'static,
            R: Send + 'static,
        {
            self.0.spawn_blocking(f)
        }
    }
}

pub mod task {
    use super::*;

    pub type JoinError = edgerun_runtime::rt::JoinError;
    pub type JoinHandle<T> = edgerun_runtime::rt::JoinHandle<T>;
    pub type JoinSet<T> = edgerun_runtime::rt::JoinSet<T>;

    pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        edgerun_runtime::rt::spawn(future)
    }

    pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        edgerun_runtime::rt::spawn_blocking(f)
    }

    pub async fn yield_now() {
        edgerun_runtime::rt::yieldnow().await;
    }
}

pub mod time {
    pub use edgerun_runtime::rt::{
        Duration, Elapsed, Instant, Sleep, SleepUntil, Timeout, TimeoutAt, sleep_until, timeout,
        timeout_at,
    };

    pub fn sleep(duration: Duration) -> Sleep {
        Sleep::after(duration)
    }

    pub mod error {
        pub type Elapsed = edgerun_runtime::rt::Elapsed;
    }
}

pub mod io {
    pub use std::io::{Error, ErrorKind, Result};

    pub use edgerun_runtime::rt::{
        AsyncBufRead, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader, Cursor,
        IoError, copy, copy_bidirectional,
    };
}

pub mod fs {
    use std::io::{Read, Write};
    use std::path::Path;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use edgerun_runtime::rt::io::{AsyncRead, AsyncWrite, IoError, Result as IoResult};

    fn map_io_error(error: std::io::Error) -> IoError {
        match error.kind() {
            std::io::ErrorKind::UnexpectedEof => IoError::UnexpectedEof,
            std::io::ErrorKind::WriteZero => IoError::WriteZero,
            _ => IoError::Other("filesystem operation failed"),
        }
    }

    pub async fn metadata(path: impl AsRef<Path>) -> std::io::Result<std::fs::Metadata> {
        std::fs::metadata(path)
    }

    pub async fn symlink_metadata(path: impl AsRef<Path>) -> std::io::Result<std::fs::Metadata> {
        std::fs::symlink_metadata(path)
    }

    pub async fn read(path: impl AsRef<Path>) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    pub async fn read_to_string(path: impl AsRef<Path>) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }

    pub async fn write(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> std::io::Result<()> {
        std::fs::write(path, contents)
    }

    pub async fn create_dir(path: impl AsRef<Path>) -> std::io::Result<()> {
        std::fs::create_dir(path)
    }

    pub async fn create_dir_all(path: impl AsRef<Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    pub async fn remove_file(path: impl AsRef<Path>) -> std::io::Result<()> {
        std::fs::remove_file(path)
    }

    pub async fn remove_dir(path: impl AsRef<Path>) -> std::io::Result<()> {
        std::fs::remove_dir(path)
    }

    pub async fn remove_dir_all(path: impl AsRef<Path>) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }

    pub async fn rename(from: impl AsRef<Path>, to: impl AsRef<Path>) -> std::io::Result<()> {
        std::fs::rename(from, to)
    }

    pub async fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> std::io::Result<u64> {
        std::fs::copy(from, to)
    }

    pub struct File {
        inner: std::fs::File,
    }

    impl File {
        pub async fn open(path: impl AsRef<Path>) -> std::io::Result<Self> {
            std::fs::File::open(path).map(|inner| Self { inner })
        }

        pub async fn create(path: impl AsRef<Path>) -> std::io::Result<Self> {
            std::fs::File::create(path).map(|inner| Self { inner })
        }

        pub async fn sync_all(&self) -> std::io::Result<()> {
            self.inner.sync_all()
        }
    }

    impl std::io::Read for File {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            self.inner.read(buf)
        }
    }

    impl std::io::Write for File {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.inner.write(buf)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.inner.flush()
        }
    }

    impl AsyncRead for File {
        fn poll_read(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<IoResult<usize>> {
            Poll::Ready(self.inner.read(buf).map_err(map_io_error))
        }
    }

    impl AsyncWrite for File {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<IoResult<usize>> {
            Poll::Ready(self.inner.write(buf).map_err(map_io_error))
        }

        fn poll_flush(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<IoResult<()>> {
            Poll::Ready(self.inner.flush().map_err(map_io_error))
        }
    }

    pub struct OpenOptions {
        inner: std::fs::OpenOptions,
    }

    impl OpenOptions {
        pub fn new() -> Self {
            Self {
                inner: std::fs::OpenOptions::new(),
            }
        }

        pub fn read(&mut self, read: bool) -> &mut Self {
            self.inner.read(read);
            self
        }

        pub fn write(&mut self, write: bool) -> &mut Self {
            self.inner.write(write);
            self
        }

        pub fn append(&mut self, append: bool) -> &mut Self {
            self.inner.append(append);
            self
        }

        pub fn truncate(&mut self, truncate: bool) -> &mut Self {
            self.inner.truncate(truncate);
            self
        }

        pub fn create(&mut self, create: bool) -> &mut Self {
            self.inner.create(create);
            self
        }

        pub fn create_new(&mut self, create_new: bool) -> &mut Self {
            self.inner.create_new(create_new);
            self
        }

        pub async fn open(&self, path: impl AsRef<Path>) -> std::io::Result<File> {
            self.inner.open(path).map(|inner| File { inner })
        }
    }
}

pub mod net {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, ToSocketAddrs};
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll};

    use edgerun_runtime::rt::{AsyncRead, AsyncWrite};

    pub struct TcpStream {
        inner: Arc<edgerun_runtime::rt::AsyncTcpStream>,
    }

    impl TcpStream {
        pub async fn connect<A: ToString>(addr: A) -> edgerun_runtime::rt::io::Result<Self> {
            edgerun_runtime::rt::ConnectFuture::new(addr)
                .await
                .map(|inner| Self { inner })
        }

        pub fn local_addr(&self) -> edgerun_runtime::rt::io::Result<SocketAddr> {
            self.inner.local_addr()
        }

        pub fn peer_addr(&self) -> edgerun_runtime::rt::io::Result<SocketAddr> {
            self.inner.peer_addr()
        }
    }

    impl edgerun_runtime::rt::AsyncRead for TcpStream {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut [u8],
        ) -> Poll<edgerun_runtime::rt::io::Result<usize>> {
            Pin::new(&mut self.inner).poll_read(cx, buf)
        }
    }

    impl edgerun_runtime::rt::AsyncWrite for TcpStream {
        fn poll_write(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<edgerun_runtime::rt::io::Result<usize>> {
            Pin::new(&mut self.inner).poll_write(cx, buf)
        }

        fn poll_flush(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<edgerun_runtime::rt::io::Result<()>> {
            Pin::new(&mut self.inner).poll_flush(cx)
        }

        fn poll_shutdown(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
        ) -> Poll<edgerun_runtime::rt::io::Result<()>> {
            Pin::new(&mut self.inner).poll_shutdown(cx)
        }
    }

    fn map_edgerun_io(error: edgerun_runtime::rt::IoError) -> std::io::Error {
        match error {
            edgerun_runtime::rt::IoError::UnexpectedEof => {
                std::io::Error::from(std::io::ErrorKind::UnexpectedEof)
            }
            edgerun_runtime::rt::IoError::WriteZero => {
                std::io::Error::from(std::io::ErrorKind::WriteZero)
            }
            edgerun_runtime::rt::IoError::Other(message) => std::io::Error::other(message),
        }
    }

    impl Read for TcpStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let waker = edgerun_runtime::rt::noop_waker();
            let mut cx = Context::from_waker(&waker);
            loop {
                match Pin::new(&mut *self).poll_read(&mut cx, buf) {
                    Poll::Ready(result) => return result.map_err(map_edgerun_io),
                    Poll::Pending => {
                        edgerun_runtime::rt::run_queue();
                        std::thread::yield_now();
                    }
                }
            }
        }
    }

    impl Write for TcpStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let waker = edgerun_runtime::rt::noop_waker();
            let mut cx = Context::from_waker(&waker);
            loop {
                match Pin::new(&mut *self).poll_write(&mut cx, buf) {
                    Poll::Ready(result) => return result.map_err(map_edgerun_io),
                    Poll::Pending => {
                        edgerun_runtime::rt::run_queue();
                        std::thread::yield_now();
                    }
                }
            }
        }

        fn flush(&mut self) -> std::io::Result<()> {
            let waker = edgerun_runtime::rt::noop_waker();
            let mut cx = Context::from_waker(&waker);
            loop {
                match Pin::new(&mut *self).poll_flush(&mut cx) {
                    Poll::Ready(result) => return result.map_err(map_edgerun_io),
                    Poll::Pending => {
                        edgerun_runtime::rt::run_queue();
                        std::thread::yield_now();
                    }
                }
            }
        }
    }

    pub struct TcpListener {
        inner: edgerun_runtime::rt::AsyncTcpListener,
    }

    impl TcpListener {
        pub async fn bind<A: ToSocketAddrs>(addr: A) -> edgerun_runtime::rt::io::Result<Self> {
            edgerun_runtime::rt::AsyncTcpListener::bind(addr).map(|inner| Self { inner })
        }

        pub fn local_addr(&self) -> edgerun_runtime::rt::io::Result<SocketAddr> {
            self.inner.local_addr()
        }

        pub async fn accept(&self) -> edgerun_runtime::rt::io::Result<(TcpStream, SocketAddr)> {
            self.inner
                .accept()
                .await
                .map(|(inner, addr)| (TcpStream { inner }, addr))
        }
    }
}

pub mod sync {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex as StdMutex};
    use std::task::{Context, Poll, Waker};

    pub use edgerun_runtime::rt::{Notify, Semaphore};
    pub type SemaphorePermit<'a> = edgerun_runtime::rt::SemaphoreGuard<'a>;

    pub type TryLockError = ();

    pub struct Mutex<T> {
        inner: edgerun_runtime::rt::AsyncMutex<T>,
    }

    impl<T: Default> Default for Mutex<T> {
        fn default() -> Self {
            Self::new(T::default())
        }
    }

    impl<T> std::fmt::Debug for Mutex<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("Mutex").finish_non_exhaustive()
        }
    }

    impl<T> Mutex<T> {
        pub fn new(value: T) -> Self {
            Self {
                inner: edgerun_runtime::rt::AsyncMutex::new(value),
            }
        }

        pub fn lock(&self) -> edgerun_runtime::rt::AsyncMutexLock<'_, T> {
            self.inner.lock()
        }
    }

    pub struct RwLock<T> {
        inner: edgerun_runtime::rt::RwLock<T>,
    }

    impl<T: Default> Default for RwLock<T> {
        fn default() -> Self {
            Self::new(T::default())
        }
    }

    impl<T> std::fmt::Debug for RwLock<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("RwLock").finish_non_exhaustive()
        }
    }

    impl<T> RwLock<T> {
        pub fn new(value: T) -> Self {
            Self {
                inner: edgerun_runtime::rt::RwLock::new(value),
            }
        }

        pub async fn read(&self) -> edgerun_runtime::rt::RwLockReadGuard<'_, T> {
            self.inner.read()
        }

        pub async fn write(&self) -> edgerun_runtime::rt::RwLockWriteGuard<'_, T> {
            self.inner.write()
        }

        pub fn try_read(
            &self,
        ) -> Result<edgerun_runtime::rt::RwLockReadGuard<'_, T>, TryLockError> {
            self.inner.try_read().ok_or(())
        }

        pub fn try_write(
            &self,
        ) -> Result<edgerun_runtime::rt::RwLockWriteGuard<'_, T>, TryLockError> {
            self.inner.try_write().ok_or(())
        }
    }

    pub mod mpsc {
        use std::future::Future;
        use std::pin::Pin;
        use std::task::{Context, Poll};

        pub use edgerun_runtime::rt::mpsc::{SendError, TryRecvError, TrySendError};

        pub struct Sender<T> {
            inner: edgerun_runtime::rt::mpsc::Sender<T>,
        }

        impl<T> Clone for Sender<T> {
            fn clone(&self) -> Self {
                Self {
                    inner: self.inner.clone(),
                }
            }
        }

        pub struct Receiver<T> {
            inner: edgerun_runtime::rt::mpsc::Receiver<T>,
        }

        impl<T> Sender<T> {
            pub fn send(&self, value: T) -> edgerun_runtime::rt::mpsc::Send<T> {
                self.inner.send(value)
            }

            pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
                self.inner.try_send(value)
            }

            pub fn is_closed(&self) -> bool {
                self.inner.is_closed()
            }
        }

        impl<T> Receiver<T> {
            pub fn recv(&mut self) -> edgerun_runtime::rt::mpsc::Recv<'_, T> {
                self.inner.recv()
            }

            pub fn try_recv(&self) -> Result<T, TryRecvError> {
                self.inner.try_recv()
            }

            pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
                Pin::new(&mut self.inner.recv()).poll(cx)
            }

            pub fn close(&self) {
                self.inner.close();
            }
        }

        pub struct UnboundedSender<T> {
            inner: edgerun_runtime::rt::mpsc::Sender<T>,
        }

        impl<T> Clone for UnboundedSender<T> {
            fn clone(&self) -> Self {
                Self {
                    inner: self.inner.clone(),
                }
            }
        }

        pub struct UnboundedReceiver<T> {
            inner: edgerun_runtime::rt::mpsc::Receiver<T>,
        }

        impl<T> UnboundedSender<T> {
            pub fn send(&self, value: T) -> Result<(), SendError<T>> {
                self.inner.send_nowait(value)
            }

            pub fn is_closed(&self) -> bool {
                self.inner.is_closed()
            }
        }

        impl<T> UnboundedReceiver<T> {
            pub fn recv(&mut self) -> edgerun_runtime::rt::mpsc::Recv<'_, T> {
                self.inner.recv()
            }

            pub fn try_recv(&self) -> Result<T, TryRecvError> {
                self.inner.try_recv()
            }

            pub fn poll_recv(&mut self, cx: &mut Context<'_>) -> Poll<Option<T>> {
                Pin::new(&mut self.inner.recv()).poll(cx)
            }

            pub fn close(&self) {
                self.inner.close();
            }
        }

        pub fn channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
            let (tx, rx) = edgerun_runtime::rt::mpsc::channel(cap);
            (Sender { inner: tx }, Receiver { inner: rx })
        }

        pub fn unbounded_channel<T>() -> (UnboundedSender<T>, UnboundedReceiver<T>) {
            let (tx, rx) = edgerun_runtime::rt::mpsc::channel(0);
            (
                UnboundedSender { inner: tx },
                UnboundedReceiver { inner: rx },
            )
        }
    }

    pub mod oneshot {
        use super::*;

        pub mod error {
            pub type TryRecvError = super::TryRecvError;
            pub type RecvError = super::RecvError;
        }

        pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
            let inner = Arc::new(Inner {
                value: StdMutex::new(None),
                closed: StdMutex::new(false),
                waker: StdMutex::new(None),
            });
            (
                Sender {
                    inner: Some(inner.clone()),
                },
                Receiver { inner },
            )
        }

        struct Inner<T> {
            value: StdMutex<Option<T>>,
            closed: StdMutex<bool>,
            waker: StdMutex<Option<Waker>>,
        }

        pub struct Sender<T> {
            inner: Option<Arc<Inner<T>>>,
        }

        impl<T> Sender<T> {
            pub fn send(mut self, value: T) -> Result<(), T> {
                let Some(inner) = self.inner.take() else {
                    return Err(value);
                };
                if *inner.closed.lock().expect("oneshot poisoned") {
                    return Err(value);
                }
                *inner.value.lock().expect("oneshot poisoned") = Some(value);
                if let Some(waker) = inner.waker.lock().expect("oneshot poisoned").take() {
                    waker.wake();
                }
                Ok(())
            }
        }

        impl<T> Drop for Sender<T> {
            fn drop(&mut self) {
                if let Some(inner) = self.inner.take() {
                    *inner.closed.lock().expect("oneshot poisoned") = true;
                    if let Some(waker) = inner.waker.lock().expect("oneshot poisoned").take() {
                        waker.wake();
                    }
                }
            }
        }

        pub struct Receiver<T> {
            inner: Arc<Inner<T>>,
        }

        impl<T> Receiver<T> {
            pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
                if let Some(value) = self.inner.value.lock().expect("oneshot poisoned").take() {
                    Ok(value)
                } else if *self.inner.closed.lock().expect("oneshot poisoned") {
                    Err(TryRecvError::Closed)
                } else {
                    Err(TryRecvError::Empty)
                }
            }
        }

        impl<T> Future for Receiver<T> {
            type Output = Result<T, RecvError>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let this = self.get_mut();
                if let Some(value) = this.inner.value.lock().expect("oneshot poisoned").take() {
                    return Poll::Ready(Ok(value));
                }
                if *this.inner.closed.lock().expect("oneshot poisoned") {
                    return Poll::Ready(Err(RecvError));
                }
                *this.inner.waker.lock().expect("oneshot poisoned") = Some(cx.waker().clone());
                Poll::Pending
            }
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum TryRecvError {
            Empty,
            Closed,
        }

        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub struct RecvError;

        impl std::fmt::Display for RecvError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("oneshot channel closed")
            }
        }

        impl std::error::Error for RecvError {}
    }

    pub mod watch {
        pub use edgerun_runtime::rt::watch::{Receiver, Sender};

        pub fn channel<T: Clone>(value: T) -> (Sender<T>, Receiver<T>) {
            edgerun_runtime::rt::watch::watch(value)
        }
    }

    pub mod broadcast {
        pub mod error {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum RecvError {
                Closed,
                Lagged(u64),
            }
        }

        pub struct Sender<T: Clone + 'static> {
            inner: edgerun_runtime::rt::broadcast::Publisher<T>,
        }

        pub struct Receiver<T: Clone + 'static> {
            inner: edgerun_runtime::rt::broadcast::Subscriber<T>,
        }

        pub fn channel<T: Clone + 'static>(cap: usize) -> (Sender<T>, Receiver<T>) {
            let (publisher, subscriber) = edgerun_runtime::rt::broadcast::broadcast(cap);
            (Sender { inner: publisher }, Receiver { inner: subscriber })
        }

        impl<T: Clone + 'static> Sender<T> {
            pub fn send(&self, value: T) -> Result<usize, error::RecvError> {
                self.inner.send(value);
                Ok(1)
            }

            pub fn subscribe(&self) -> Receiver<T> {
                let (_tx, rx) = channel(1);
                rx
            }
        }

        impl<T: Clone + 'static> Receiver<T> {
            pub async fn recv(&mut self) -> Result<T, error::RecvError> {
                Ok((&mut self.inner).await)
            }
        }
    }

    pub struct Barrier {
        target: usize,
        state: StdMutex<BarrierState>,
    }

    struct BarrierState {
        count: usize,
        waiters: Vec<Waker>,
    }

    impl Barrier {
        pub fn new(target: usize) -> Self {
            Self {
                target,
                state: StdMutex::new(BarrierState {
                    count: 0,
                    waiters: Vec::new(),
                }),
            }
        }

        pub fn wait(&self) -> BarrierWait<'_> {
            BarrierWait {
                barrier: self,
                registered: false,
            }
        }
    }

    pub struct BarrierWait<'a> {
        barrier: &'a Barrier,
        registered: bool,
    }

    impl Future for BarrierWait<'_> {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if self.barrier.target <= 1 {
                return Poll::Ready(());
            }
            let mut state = self.barrier.state.lock().expect("barrier poisoned");
            if !self.registered {
                state.count += 1;
                self.registered = true;
            }
            if state.count >= self.barrier.target {
                let waiters = std::mem::take(&mut state.waiters);
                for waker in waiters {
                    waker.wake();
                }
                Poll::Ready(())
            } else {
                state.waiters.push(cx.waker().clone());
                Poll::Pending
            }
        }
    }

    pub type OnceCell<T> = edgerun_runtime::rt::OnceCell<T>;
}
