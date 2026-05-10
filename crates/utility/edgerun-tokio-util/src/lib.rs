//! Edgerun-owned compatibility surface for Tokio utility APIs.

extern crate std;

pub mod edgerun {
    pub use edgerun_tokio::edgerun::CancellationToken;
    pub use edgerun_tokio::edgerun::Cancelled;
}

pub mod sync {
    pub use edgerun_tokio::edgerun::{CancellationToken, Cancelled};
}

pub mod either {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Either<L, R> {
        Left(L),
        Right(R),
    }
}

pub mod task {
    use std::future::Future;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    pub struct AbortOnDropHandle<T> {
        inner: Option<edgerun_tokio::task::JoinHandle<T>>,
    }

    impl<T> AbortOnDropHandle<T> {
        pub fn new(handle: edgerun_tokio::task::JoinHandle<T>) -> Self {
            Self {
                inner: Some(handle),
            }
        }

        pub fn abort(&self) {
            if let Some(handle) = &self.inner {
                handle.abort();
            }
        }

        pub fn is_finished(&self) -> bool {
            self.inner
                .as_ref()
                .map(|handle| handle.is_finished())
                .unwrap_or(true)
        }
    }

    impl<T> Drop for AbortOnDropHandle<T> {
        fn drop(&mut self) {
            if let Some(handle) = &self.inner {
                handle.abort();
            }
        }
    }

    impl<T> Future for AbortOnDropHandle<T> {
        type Output = Result<T, edgerun_tokio::task::JoinError>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            let this = self.get_mut();
            match this.inner.as_mut() {
                Some(handle) => Pin::new(handle).poll(cx),
                None => panic!("polled AbortOnDropHandle after completion"),
            }
        }
    }
}

pub mod io {
    use std::io::Read;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use edgerun_futures::Stream;

    const DEFAULT_CHUNK_SIZE: usize = 8192;

    pub struct ReaderStream<R> {
        reader: R,
        done: bool,
        chunk_size: usize,
    }

    impl<R> ReaderStream<R> {
        pub fn new(reader: R) -> Self {
            Self {
                reader,
                done: false,
                chunk_size: DEFAULT_CHUNK_SIZE,
            }
        }

        pub fn with_capacity(reader: R, capacity: usize) -> Self {
            Self {
                reader,
                done: false,
                chunk_size: capacity.max(1),
            }
        }
    }

    impl<R: Read + Unpin> Stream for ReaderStream<R> {
        type Item = std::io::Result<edgerun_bytes::Bytes>;

        fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            if self.done {
                return Poll::Ready(None);
            }

            let mut buf = vec![0u8; self.chunk_size];
            match self.reader.read(&mut buf) {
                Ok(0) => {
                    self.done = true;
                    Poll::Ready(None)
                }
                Ok(n) => {
                    buf.truncate(n);
                    Poll::Ready(Some(Ok(edgerun_bytes::Bytes::from(buf))))
                }
                Err(error) => {
                    self.done = true;
                    Poll::Ready(Some(Err(error)))
                }
            }
        }
    }
}
