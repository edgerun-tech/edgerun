//! edgerun-browser — Browser engine core types from web specs.
//!
//! Generated from WHATWG HTML, W3C CSS, and ECMA-262 specifications.
#![cfg_attr(not(test), no_std)]

pub mod element_registry;
pub mod tokenizer_states;
pub mod tree_builder;
pub mod property_registry;
pub mod value_types;
pub mod at_rule_registry;
pub mod js_object_registry;
pub mod js_abstract_ops;
pub mod js_globals;
pub mod dom_hierarchy;
