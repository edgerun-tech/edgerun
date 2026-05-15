use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::channel::RouteBinding;
use crate::codec::ArchivedWorkPacketFrame;
use crate::protocol::WorkPacket;
use crate::std_runtime::framing::{
    read_work_packet_frame, write_encoded_work_packet, write_work_packet,
};
use crate::std_runtime::threading::{drain_joined_threads, join_locked_optional_thread};

const ACCEPT_POLL_MS: u64 = 10;
const CONNECTION_READ_TIMEOUT_MS: u64 = 250;

#[derive(Clone, Debug)]
pub(crate) struct TcpPacketServer {
    listen_addr: SocketAddr,
    inbox: Arc<Mutex<Vec<ArchivedWorkPacketFrame>>>,
    shutdown: Arc<AtomicBool>,
    accept_thread: Arc<Mutex<Option<JoinHandle<()>>>>,
    worker_threads: Arc<Mutex<Vec<JoinHandle<()>>>>,
}

impl TcpPacketServer {
    pub(crate) fn bind<A: ToSocketAddrs>(addr: A, inbox_name: &'static str) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let listen_addr = listener.local_addr()?;
        let inbox = Arc::new(Mutex::new(Vec::new()));
        let shutdown = Arc::new(AtomicBool::new(false));
        let worker_threads = Arc::new(Mutex::new(Vec::new()));
        let inbox_for_thread = Arc::clone(&inbox);
        let shutdown_for_thread = Arc::clone(&shutdown);
        let workers_for_thread = Arc::clone(&worker_threads);
        let accept_thread = thread::spawn(move || {
            while !shutdown_for_thread.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _peer)) => {
                        let _ = stream.set_read_timeout(Some(Duration::from_millis(
                            CONNECTION_READ_TIMEOUT_MS,
                        )));
                        let inbox = Arc::clone(&inbox_for_thread);
                        let shutdown = Arc::clone(&shutdown_for_thread);
                        let handle = thread::spawn(move || {
                            let _ = read_stream_into_inbox(stream, inbox, shutdown, inbox_name);
                        });
                        workers_for_thread
                            .lock()
                            .expect("tcp worker thread list poisoned")
                            .push(handle);
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(ACCEPT_POLL_MS));
                    }
                    Err(_) => break,
                }
            }
        });
        Ok(Self {
            listen_addr,
            inbox,
            shutdown,
            accept_thread: Arc::new(Mutex::new(Some(accept_thread))),
            worker_threads,
        })
    }

    pub(crate) fn listen_addr(&self) -> SocketAddr {
        self.listen_addr
    }

    pub(crate) fn drain_packet_frames(&self) -> Vec<ArchivedWorkPacketFrame> {
        self.inbox
            .lock()
            .expect("tcp packet server inbox poisoned")
            .drain(..)
            .collect()
    }

    pub(crate) fn drain_packets(&self) -> Vec<WorkPacket> {
        self.drain_packet_frames()
            .into_iter()
            .filter_map(|frame| frame.into_packet().ok())
            .collect()
    }

    pub(crate) fn shutdown(&self) -> io::Result<()> {
        self.shutdown.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.listen_addr);
        join_locked_optional_thread(
            &self.accept_thread,
            "tcp accept thread poisoned",
            "tcp accept thread panicked",
        )?;
        drain_joined_threads(
            &self.worker_threads,
            "tcp worker thread list poisoned",
            "tcp worker thread panicked",
        )
    }
}

pub(crate) fn send_packet_to_addr<A: ToSocketAddrs>(
    addr: A,
    packet: &WorkPacket,
) -> io::Result<()> {
    let mut stream = TcpStream::connect(addr)?;
    write_work_packet(&mut stream, packet)
}

pub(crate) fn send_encoded_packet_to_addr<A: ToSocketAddrs>(
    addr: A,
    bytes: &[u8],
) -> io::Result<()> {
    let mut stream = TcpStream::connect(addr)?;
    write_encoded_work_packet(&mut stream, bytes)
}

pub(crate) fn route_addr(route: &RouteBinding) -> String {
    String::from_utf8_lossy(&route.endpoint.address).into_owned()
}

pub(crate) fn send_encoded_packet_to_route(route: &RouteBinding, bytes: &[u8]) -> io::Result<()> {
    send_encoded_packet_to_addr(route_addr(route), bytes)
}

impl Drop for TcpPacketServer {
    fn drop(&mut self) {
        if Arc::strong_count(&self.accept_thread) == 1 {
            let _ = self.shutdown();
        }
    }
}

fn read_stream_into_inbox(
    mut stream: TcpStream,
    inbox: Arc<Mutex<Vec<ArchivedWorkPacketFrame>>>,
    shutdown: Arc<AtomicBool>,
    inbox_name: &'static str,
) -> io::Result<()> {
    if shutdown.load(Ordering::Acquire) {
        return Ok(());
    }
    let frame = read_work_packet_frame(&mut stream)?;
    if !shutdown.load(Ordering::Acquire) {
        inbox.lock().expect(inbox_name).push(frame);
    }
    Ok(())
}
