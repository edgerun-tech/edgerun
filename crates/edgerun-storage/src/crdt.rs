// SPDX-License-Identifier: GPL-2.0-only
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;

use crate::event::{ActorId, CrdtType};

#[derive(Error, Debug)]
pub enum CrdtError {
    #[error("Type mismatch: expected {expected:?}, got {actual:?}")]
    TypeMismatch {
        expected: CrdtType,
        actual: CrdtType,
    },
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Invalid operation")]
    InvalidOperation,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct OrSet {
    elements: HashMap<String, HashSet<ActorId>>,
    tombstones: HashMap<String, HashSet<ActorId>>,
}

impl OrSet {
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
            tombstones: HashMap::new(),
        }
    }

    pub fn add(&mut self, element: String, actor: ActorId) {
        self.elements
            .entry(element.clone())
            .or_insert_with(HashSet::new)
            .insert(actor);
        self.tombstones.remove(&element);
    }

    pub fn remove(&mut self, element: &str, actor: ActorId) {
        self.tombstones
            .entry(element.to_string())
            .or_insert_with(HashSet::new)
            .insert(actor);
        self.elements.remove(element);
    }

    pub fn contains(&self, element: &str) -> bool {
        if let Some(actors) = self.elements.get(element) {
            if let Some(tombstones) = self.tombstones.get(element) {
                actors.len() > tombstones.len()
            } else {
                !actors.is_empty()
            }
        } else {
            false
        }
    }

    pub fn elements(&self) -> Vec<String> {
        self.elements
            .keys()
            .filter(|e| self.contains(*e))
            .cloned()
            .collect()
    }

    pub fn merge(&mut self, other: &OrSet) {
        for (element, actors) in &other.elements {
            let entry = self
                .elements
                .entry(element.clone())
                .or_insert_with(HashSet::new);
            for actor in actors {
                entry.insert(actor.clone());
            }
        }

        for (element, actors) in &other.tombstones {
            let entry = self
                .tombstones
                .entry(element.clone())
                .or_insert_with(HashSet::new);
            for actor in actors {
                entry.insert(actor.clone());
            }
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct LwwRegister {
    value: Vec<u8>,
    timestamp: i64,
    actor_id: ActorId,
}

impl LwwRegister {
    pub fn new() -> Self {
        Self {
            value: Vec::new(),
            timestamp: 0,
            actor_id: ActorId::new(),
        }
    }

    pub fn set(&mut self, value: Vec<u8>, timestamp: i64, actor: ActorId) {
        if timestamp > self.timestamp || (timestamp == self.timestamp && actor.0 > self.actor_id.0)
        {
            self.value = value;
            self.timestamp = timestamp;
            self.actor_id = actor;
        }
    }

    pub fn get(&self) -> &[u8] {
        &self.value
    }

    pub fn merge(&mut self, other: &LwwRegister) {
        if other.timestamp > self.timestamp
            || (other.timestamp == self.timestamp && other.actor_id.0 > self.actor_id.0)
        {
            self.value = other.value.clone();
            self.timestamp = other.timestamp;
            self.actor_id = other.actor_id.clone();
        }
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct PnCounter {
    positive: HashMap<ActorId, i64>,
    negative: HashMap<ActorId, i64>,
}

impl PnCounter {
    pub fn new() -> Self {
        Self {
            positive: HashMap::new(),
            negative: HashMap::new(),
        }
    }

    pub fn increment(&mut self, actor: ActorId, amount: i64) {
        let entry = self.positive.entry(actor).or_insert(0);
        *entry += amount;
    }

    pub fn decrement(&mut self, actor: ActorId, amount: i64) {
        let entry = self.negative.entry(actor).or_insert(0);
        *entry += amount;
    }

    pub fn value(&self) -> i64 {
        let pos: i64 = self.positive.values().sum();
        let neg: i64 = self.negative.values().sum();
        pos - neg
    }

    pub fn merge(&mut self, other: &PnCounter) {
        for (actor, &val) in &other.positive {
            let entry = self.positive.entry(actor.clone()).or_insert(0);
            *entry = (*entry).max(val);
        }

        for (actor, &val) in &other.negative {
            let entry = self.negative.entry(actor.clone()).or_insert(0);
            *entry = (*entry).max(val);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_or_set_add_remove() {
        let mut or_set = OrSet::new();

        let actor = ActorId::new();

        or_set.add("hello".to_string(), actor.clone());
        assert!(or_set.contains("hello"));

        or_set.remove("hello", actor);
        assert!(!or_set.contains("hello"));
    }

    #[test]
    fn test_or_set_merge() {
        let mut or_set1 = OrSet::new();
        let actor1 = ActorId::new();
        or_set1.add("a".to_string(), actor1);

        let mut or_set2 = OrSet::new();
        let actor2 = ActorId::new();
        or_set2.add("b".to_string(), actor2);

        or_set1.merge(&or_set2);

        assert!(or_set1.contains("a"));
        assert!(or_set1.contains("b"));
    }

    #[test]
    fn test_or_set_elements() {
        let mut or_set = OrSet::new();

        let actor = ActorId::new();
        or_set.add("a".to_string(), actor.clone());
        or_set.add("b".to_string(), actor);

        let elements = or_set.elements();
        assert_eq!(elements.len(), 2);
    }

    #[test]
    fn test_lww_register() {
        let mut register = LwwRegister::new();

        let actor1 = ActorId::from_bytes(*b"1111111111111111");
        let actor2 = ActorId::from_bytes(*b"2222222222222222");

        register.set(b"value1".to_vec(), 100, actor1);
        assert_eq!(register.get(), b"value1");

        register.set(b"value2".to_vec(), 50, actor2.clone());
        assert_eq!(register.get(), b"value1");

        register.set(b"value3".to_vec(), 150, actor2);
        assert_eq!(register.get(), b"value3");
    }

    #[test]
    fn test_lww_register_merge() {
        let mut reg1 = LwwRegister::new();
        reg1.set(
            b"v1".to_vec(),
            100,
            ActorId::from_bytes(*b"1111111111111111"),
        );

        let mut reg2 = LwwRegister::new();
        reg2.set(
            b"v2".to_vec(),
            200,
            ActorId::from_bytes(*b"2222222222222222"),
        );

        reg1.merge(&reg2);

        assert_eq!(reg1.get(), b"v2");
    }

    #[test]
    fn test_pn_counter() {
        let mut counter = PnCounter::new();

        let actor = ActorId::new();

        counter.increment(actor.clone(), 10);
        counter.increment(actor.clone(), 5);
        counter.decrement(actor, 3);

        assert_eq!(counter.value(), 12);
    }

    #[test]
    fn test_pn_counter_merge() {
        let mut counter1 = PnCounter::new();
        let actor1 = ActorId::new();
        counter1.increment(actor1, 10);

        let mut counter2 = PnCounter::new();
        let actor2 = ActorId::new();
        counter2.increment(actor2, 20);

        counter1.merge(&counter2);

        assert_eq!(counter1.value(), 30);
    }

    #[test]
    fn test_or_set_contains_not_present() {
        let or_set = OrSet::new();
        assert!(!or_set.contains("nonexistent"));
    }

    #[test]
    fn test_or_set_remove() {
        let mut or_set = OrSet::new();

        let actor = ActorId::new();
        or_set.add("test".to_string(), actor.clone());
        assert!(or_set.contains("test"));

        or_set.remove("test", actor);
        assert!(!or_set.contains("test"));
    }

    #[test]
    fn test_lww_register_timestamp_tiebreaker() {
        let mut register = LwwRegister::new();

        let actor1 = ActorId::from_bytes(*b"AAAAAAAAAAAAAAA0");
        let actor2 = ActorId::from_bytes(*b"BBBBBBBBBBBBBBB1");

        register.set(b"v1".to_vec(), 100, actor1);
        register.set(b"v2".to_vec(), 100, actor2);

        assert_eq!(register.get(), b"v2");
    }

    #[test]
    fn test_pn_counter_decrement() {
        let mut counter = PnCounter::new();

        let actor = ActorId::new();
        counter.increment(actor.clone(), 10);
        counter.decrement(actor, 5);

        assert_eq!(counter.value(), 5);
    }
}
