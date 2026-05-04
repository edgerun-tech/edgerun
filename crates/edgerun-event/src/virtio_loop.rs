use edgerun_virtio::{VirtBlk, VirtNet, VirtRng};

use crate::queue::EventQueue;
use crate::types::Event;

pub struct VirtioEventLoop<'a, const EQ: usize = 32> {
    pub net: Option<&'a mut VirtNet>,
    pub blk: Option<&'a mut VirtBlk>,
    pub rng: Option<&'a mut VirtRng>,
    queue: &'a mut EventQueue<EQ>,
    pub sock_id_counter: u32,
    pub disk_op_counter: u32,
    pub rx_buf: [u8; 2048],
    pub pending_rx: bool,
    pub rx_sock_id: u32,
}

impl<'a, const EQ: usize> VirtioEventLoop<'a, EQ> {
    pub fn new(queue: &'a mut EventQueue<EQ>) -> Self {
        Self {
            net: None,
            blk: None,
            rng: None,
            queue,
            sock_id_counter: 0,
            disk_op_counter: 0,
            rx_buf: [0u8; 2048],
            pending_rx: false,
            rx_sock_id: 0,
        }
    }

    pub fn poll_once(&mut self) -> bool {
        let mut had_event = false;

        if let Some(net) = self.net.as_deref_mut() {
            if let Some(len) = net.recv(&mut self.rx_buf) {
                if len == 0 {
                    if self.pending_rx {
                        had_event |=
                            crate::types::push_network_disconnected(self.queue, self.rx_sock_id);
                        self.pending_rx = false;
                    }
                } else {
                    if !self.pending_rx {
                        self.sock_id_counter = self.sock_id_counter.wrapping_add(1);
                        self.rx_sock_id = self.sock_id_counter;
                        self.pending_rx = true;
                        had_event |=
                            crate::types::push_network_connected(self.queue, self.rx_sock_id);
                    }
                    had_event |= crate::types::push_network_received(
                        self.queue,
                        self.rx_sock_id,
                        &self.rx_buf[..len],
                    );
                }
            }
        }

        if let Some(_blk) = self.blk.as_deref_mut() {
            // VirtBlk uses synchronous polling in its read_sector/write_sector.
            // The event loop can be used to signal completion for async operations
            // when the block device is extended with async APIs.
        }

        had_event
    }

    pub fn poll_many(&mut self, max_events: usize) -> usize {
        let mut count = 0;
        for _ in 0..max_events {
            if !self.poll_once() {
                break;
            }
            count += 1;
        }
        count
    }

    pub fn pop_event(&mut self) -> Option<Event> {
        self.queue.pop()
    }

    pub fn queue_len(&self) -> usize {
        self.queue.len()
    }

    pub fn queue_is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
