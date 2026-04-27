use core::future::Future;
use core::marker::Unpin;

pub async fn block_on<F: Future + Unpin>(mut f: F) -> F::Output {
    loop {
        let waker = unsafe { edgerun_platform::waker::make_ipi_waker(edgerun_platform::this_cpu()) };
        let mut cx = core::task::Context::from_waker(&waker);
        match core::pin::Pin::new(&mut f).poll(&mut cx) {
            core::task::Poll::Ready(v) => return v,
            core::task::Poll::Pending => unsafe { edgerun_platform::yield_cpu(); },
        }
    }
}