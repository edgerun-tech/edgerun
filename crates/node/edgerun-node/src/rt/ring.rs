//! Ring buffer for packet queuing

use alloc::vec;
use alloc::vec::Vec;

pub struct RingBuffer {
    data: Vec<u8>,
    head: usize,
    tail: usize,
    capacity: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        let capacity = match capacity.max(2).checked_next_power_of_two() {
            Some(capacity) => capacity,
            None => 1 << (usize::BITS as usize - 1),
        };
        Self {
            data: vec![0; capacity],
            head: 0,
            tail: 0,
            capacity,
        }
    }

    pub fn push(&mut self, byte: u8) -> bool {
        if self.len() >= self.capacity - 1 {
            return false;
        }
        self.data[self.tail] = byte;
        self.tail = (self.tail + 1) & (self.capacity - 1);
        true
    }

    pub fn push_slice(&mut self, bytes: &[u8]) -> usize {
        let free = (self.capacity - 1).saturating_sub(self.len());
        let count = free.min(bytes.len());

        if count == 0 {
            return 0;
        }

        let first = (self.capacity - self.tail).min(count);
        let second = count - first;

        self.data[self.tail..self.tail + first].copy_from_slice(&bytes[..first]);
        if second > 0 {
            self.data[..second].copy_from_slice(&bytes[first..first + second]);
        }

        self.tail = (self.tail + count) & (self.capacity - 1);
        count
    }
}

impl RingBuffer {
    pub fn pop_slice(&mut self, buf: &mut [u8]) -> usize {
        let count = self.len().min(buf.len());
        if count == 0 {
            return 0;
        }

        let first = (self.capacity - self.head).min(count);
        let second = count - first;

        buf[..first].copy_from_slice(&self.data[self.head..self.head + first]);
        if second > 0 {
            buf[first..count].copy_from_slice(&self.data[..second]);
        }

        self.head = (self.head + count) & (self.capacity - 1);
        count
    }
    pub fn pop(&mut self) -> Option<u8> {
        if self.head == self.tail {
            return None;
        }
        let byte = self.data[self.head];
        self.head = (self.head + 1) & (self.capacity - 1);
        Some(byte)
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
