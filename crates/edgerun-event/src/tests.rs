#[cfg(test)]
mod tests {
    use crate::queue::EventQueue;
    use crate::types::{Event, EventType, NetworkSubtype};

    #[test]
    fn test_event_queue_push_pop() {
        let mut queue: EventQueue<8> = EventQueue::new();
        assert!(queue.is_empty());

        assert!(queue.push(1, &[0x01, 0x00, 0x00, 0x00, 0x03]));
        assert_eq!(queue.len(), 1);

        let event = queue.pop().unwrap();
        assert_eq!(event.event_type, EventType::Network);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_event_queue_full() {
        let mut queue: EventQueue<2> = EventQueue::new();
        assert!(queue.push(1, &[0x01, 0x00, 0x00, 0x00, 0x03]));
        assert!(queue.push(3, &[0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]));
        assert!(!queue.push(1, &[0x02, 0x00, 0x00, 0x00, 0x01]));

        assert!(queue.is_full());
        assert_eq!(queue.len(), 2);
    }

    #[test]
    fn test_network_connected_event() {
        let event = Event::network_connected(42);
        assert_eq!(event.event_type, EventType::Network);
        assert_eq!(event.sock_id(), Some(42));
        assert_eq!(event.network_subtype(), Some(NetworkSubtype::Connected));
    }

    #[test]
    fn test_network_received_event() {
        let payload = b"hello world";
        let event = Event::network_received(7, payload).unwrap();
        assert_eq!(event.event_type, EventType::Network);
        assert_eq!(event.sock_id(), Some(7));
        assert_eq!(event.network_subtype(), Some(NetworkSubtype::Received));
        assert_eq!(event.network_payload(), Some(&payload[..]));
    }

    #[test]
    fn test_timer_fired_event() {
        let event = Event::timer_fired(12345);
        assert_eq!(event.event_type, EventType::Timer);
        assert_eq!(event.timer_id(), Some(12345));
    }

    #[test]
    fn test_event_roundtrip_bytes() {
        let original = Event::network_received(5, b"test data").unwrap();
        let bytes = original.to_bytes();
        let reconstructed = Event::from_bytes(&bytes[..original.total_len()]).unwrap();
        assert_eq!(reconstructed.event_type, original.event_type);
        assert_eq!(reconstructed.sock_id(), original.sock_id());
        assert_eq!(reconstructed.network_payload(), original.network_payload());
    }

    #[test]
    fn test_queue_fifo_order() {
        let mut queue: EventQueue<16> = EventQueue::new();

        for i in 0..5 {
            assert!(queue.push(1, &[i, 0, 0, 0, 1]));
        }

        for i in 0..5 {
            let event = queue.pop().unwrap();
            assert_eq!(event.data[0], i as u8);
        }

        assert!(queue.is_empty());
    }
}
