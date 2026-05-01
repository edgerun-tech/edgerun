use crate::types::Event;

pub struct EventQueue<const N: usize = 32> {
    slots: [Option<Event>; N],
    head: usize,
    tail: usize,
    len: usize,
}

impl<const N: usize> EventQueue<N> {
    pub const fn new() -> Self {
        const NONE_SLOT: Option<Event> = None;
        Self {
            slots: [NONE_SLOT; N],
            head: 0,
            tail: 0,
            len: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn is_full(&self) -> bool {
        self.len == N
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        N
    }

    pub fn push(&mut self, event_type: u8, data: &[u8]) -> bool {
        if self.is_full() {
            return false;
        }
        let event = match Event::new(
            match Event::from_u8(event_type) {
                Some(t) => t,
                None => return false,
            },
            data,
        ) {
            Some(e) => e,
            None => return false,
        };
        self.slots[self.tail] = Some(event);
        self.tail = (self.tail + 1) % N;
        self.len += 1;
        true
    }

    pub fn push_event(&mut self, event: Event) -> bool {
        if self.is_full() {
            return false;
        }
        self.slots[self.tail] = Some(event);
        self.tail = (self.tail + 1) % N;
        self.len += 1;
        true
    }

    pub fn pop(&mut self) -> Option<Event> {
        if self.is_empty() {
            return None;
        }
        let event = self.slots[self.head].take()?;
        self.head = (self.head + 1) % N;
        self.len -= 1;
        Some(event)
    }

    pub fn peek(&self) -> Option<&Event> {
        if self.is_empty() {
            return None;
        }
        self.slots[self.head].as_ref()
    }

    pub fn clear(&mut self) {
        for slot in self.slots.iter_mut() {
            *slot = None;
        }
        self.head = 0;
        self.tail = 0;
        self.len = 0;
    }
}

impl<const N: usize> Default for EventQueue<N> {
    fn default() -> Self {
        Self::new()
    }
}
