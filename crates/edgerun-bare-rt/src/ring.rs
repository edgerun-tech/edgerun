//! Ring buffer for packet queuing

extern crate alloc;

use alloc::vec::Vec;

pub struct RingBuffer {
    data: Vec<u8>,
    head: usize,
    tail: usize,
    capacity: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.next_power_of_two();
        Self {
            data: Vec::with_capacity(capacity),
            head: 0,
            tail: 0,
            capacity,
        }
    }

    pub fn push(&mut self, byte: u8) -> bool {
        if self.len() >= self.capacity - 1 {
            return false;
        }
        self.data.push(byte);
        self.tail = (self.tail + 1) & (self.capacity - 1);
        true
    }

    pub fn push_slice(&mut self, bytes: &[u8]) -> usize {
        let mut n = 0;
        for &b in bytes {
            if self.push(b) {
                n += 1;
            } else {
                break;
            }
        }
        n
    }

    pub fn pop(&mut self) -> Option<u8> {
        if self.head == self.tail {
            return None;
        }
        let byte = self.data[self.head];
        self.head = (self.head + 1) & (self.capacity - 1);
        Some(byte)
    }

    pub fn pop_slice(&mut self, buf: &mut [u8]) -> usize {
        let n = buf.len().min(self.len());
        for i in 0..n {
            if let Some(b) = self.pop() {
                buf[i] = b;
            } else {
                break;
            }
        }
        n
    }

    pub fn len(&self) -> usize {
        self.tail.wrapping_sub(self.head) & (self.capacity - 1)
    }

    pub fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity - 1
    }

    pub fn clear(&mut self) {
        self.head = 0;
        self.tail = 0;
    }
}
