use std::task::{Context, Waker};

static NOOP_WAKER: std::sync::LazyLock<Waker> = std::sync::LazyLock::new(|| {
    static VTABLE: std::task::RawWakerVTable =
        std::task::RawWakerVTable::new(clone_noop, wake_noop, wake_noop, drop_noop);
    const fn clone_noop(_: *const ()) -> std::task::RawWaker {
        std::task::RawWaker::new(std::ptr::null(), &VTABLE)
    }
    const fn wake_noop(_: *const ()) {}
    const fn drop_noop(_: *const ()) {}
    unsafe { Waker::from_raw(std::task::RawWaker::new(std::ptr::null(), &VTABLE)) }
});

pub(crate) fn noop_waker() -> Waker {
    NOOP_WAKER.clone()
}

pub(crate) fn cx() -> Context<'static> {
    Context::from_waker(&NOOP_WAKER)
}
