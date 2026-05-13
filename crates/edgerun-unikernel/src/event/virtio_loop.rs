use edgerun_virtio::{VirtBlk, VirtNet, VirtRng, VirtioError};

use crate::event::queue::EventQueue;
use crate::event::types::Event;

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
    net_error_count: u32,
    last_net_error: Option<VirtioError>,
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
            net_error_count: 0,
            last_net_error: None,
        }
    }

    pub fn poll_once(&mut self) -> bool {
        let mut had_event = false;

        let net_recv = if let Some(net) = self.net.as_deref_mut() {
            Some(net.try_recv(&mut self.rx_buf))
        } else {
            None
        };

        if let Some(result) = net_recv {
            match result {
                Ok(Some(len)) => {
                    had_event |= self.handle_net_rx(len);
                }
                Ok(None) => {}
                Err(error) => {
                    had_event |= self.record_net_error(error);
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

    fn handle_net_rx(&mut self, len: usize) -> bool {
        if len == 0 {
            if self.pending_rx {
                let pushed =
                    crate::event::types::push_network_disconnected(self.queue, self.rx_sock_id);
                self.pending_rx = false;
                return pushed;
            }
            return false;
        }

        let mut had_event = false;
        if !self.pending_rx {
            self.sock_id_counter = self.sock_id_counter.wrapping_add(1);
            self.rx_sock_id = self.sock_id_counter;
            self.pending_rx = true;
            had_event |= crate::event::types::push_network_connected(self.queue, self.rx_sock_id);
        }
        had_event |= crate::event::types::push_network_received(
            self.queue,
            self.rx_sock_id,
            &self.rx_buf[..len],
        );
        had_event
    }

    fn record_net_error(&mut self, error: VirtioError) -> bool {
        self.net_error_count = self.net_error_count.wrapping_add(1);
        self.last_net_error = Some(error);
        crate::event::types::push_network_error(self.queue, self.rx_sock_id)
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

    pub fn net_error_count(&self) -> u32 {
        self.net_error_count
    }

    pub fn last_net_error(&self) -> Option<VirtioError> {
        self.last_net_error
    }
}

#[cfg(test)]
mod tests {
    use edgerun_virtio::VirtioError;

    use crate::event::queue::EventQueue;
    use crate::event::types::NetworkSubtype;

    use super::VirtioEventLoop;

    #[test]
    fn net_errors_are_reported_as_events() {
        let mut queue = EventQueue::<4>::new();
        let mut event_loop = VirtioEventLoop::new(&mut queue);

        assert!(event_loop.record_net_error(VirtioError::DeviceTimeout));

        assert_eq!(event_loop.net_error_count(), 1);
        assert_eq!(
            event_loop.last_net_error(),
            Some(VirtioError::DeviceTimeout)
        );

        let event = event_loop.pop_event().unwrap();
        assert_eq!(event.sock_id(), Some(0));
        assert_eq!(event.network_subtype(), Some(NetworkSubtype::Error));
    }

    #[test]
    fn net_receive_opens_session_before_payload() {
        let mut queue = EventQueue::<4>::new();
        let mut event_loop = VirtioEventLoop::new(&mut queue);
        event_loop.rx_buf[..3].copy_from_slice(b"abc");

        assert!(event_loop.handle_net_rx(3));

        let connected = event_loop.pop_event().unwrap();
        assert_eq!(connected.sock_id(), Some(1));
        assert_eq!(connected.network_subtype(), Some(NetworkSubtype::Connected));
        let received = event_loop.pop_event().unwrap();
        assert_eq!(received.sock_id(), Some(1));
        assert_eq!(received.network_subtype(), Some(NetworkSubtype::Received));
        assert_eq!(received.network_payload(), Some(&b"abc"[..]));
    }
}
