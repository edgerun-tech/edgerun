use std::io;
use std::sync::Mutex;
use std::thread::JoinHandle;

pub(super) fn join_thread(handle: JoinHandle<()>, panic_message: &'static str) -> io::Result<()> {
    handle
        .join()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, panic_message))
}

pub(super) fn join_optional_thread(
    thread: &mut Option<JoinHandle<()>>,
    panic_message: &'static str,
) -> io::Result<()> {
    if let Some(handle) = thread.take() {
        join_thread(handle, panic_message)?;
    }
    Ok(())
}

pub(super) fn join_locked_optional_thread(
    thread: &Mutex<Option<JoinHandle<()>>>,
    poisoned_message: &'static str,
    panic_message: &'static str,
) -> io::Result<()> {
    let handle = thread.lock().expect(poisoned_message).take();
    if let Some(handle) = handle {
        join_thread(handle, panic_message)?;
    }
    Ok(())
}

pub(super) fn drain_joined_threads(
    threads: &Mutex<Vec<JoinHandle<()>>>,
    poisoned_message: &'static str,
    panic_message: &'static str,
) -> io::Result<()> {
    let handles = threads
        .lock()
        .expect(poisoned_message)
        .drain(..)
        .collect::<Vec<_>>();
    for handle in handles {
        join_thread(handle, panic_message)?;
    }
    Ok(())
}
