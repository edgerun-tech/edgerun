//! Selector specificity — from Selectors Level 4.
//! DO NOT EDIT. Regenerate with: scripts/generate_selector_dom.py
extern crate alloc;

/// Specificity triple (A, B, C) as defined in Selectors Level 4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash)]
pub struct Specificity(pub u32, pub u32, pub u32);

impl Specificity {
    /// A: Count of ID selectors
    /// B: Classes, attributes, pseudo-classes
    /// C: Type selectors and pseudo-elements

    pub fn add_id(&mut self) { self.0 += 1; }
    pub fn add_class(&mut self) { self.1 += 1; }
    pub fn add_type(&mut self) { self.2 += 1; }

    /// Check if this specificity is greater than another.
    pub fn gt(&self, other: &Self) -> bool {
        self.0 > other.0 || (self.0 == other.0 && (self.1 > other.1 || (self.1 == other.1 && self.2 > other.2)))
    }
}