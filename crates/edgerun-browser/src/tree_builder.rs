
//! HTML Tree Builder Insertion Modes — WHATWG §13.2.6.
//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py
extern crate alloc;
use alloc::vec::Vec;


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertionMode {
    Initial,
    BeforeHtml,
    BeforeHead,
    InHead,
    InHeadNoscript,
    AfterHead,
    InBody,
    Text,
    InTable,
    InTableText,
    InCaption,
    InRow,
    InCell,
    InColgroup,
    InTableBody,
    AfterBody,
    InFrameset,
    AfterFrameset,
    AfterAfterBody,
    AfterAfterFrameset,
}

#[derive(Debug, Clone)]
pub enum InsertionAction {
    InsertNode,
    Ignore,
    FosterParent,
    Reprocess(InsertionMode),
    ChangeMode(InsertionMode),
    InScope { tag: &'static str, result: bool },
}

pub struct TreeBuilderState {
    pub insertion_mode: InsertionMode,
    pub original_insertion_mode: Option<InsertionMode>,
    pub frameset_ok: bool,
    pub head_element: Option<usize>, // index into open_elements
    pub form_element: Option<usize>,
    pub template_insertion_modes: Vec<InsertionMode>,
}

impl TreeBuilderState {
    pub fn new() -> Self {
        Self {
            insertion_mode: InsertionMode::Initial,
            original_insertion_mode: None,
            frameset_ok: true,
            head_element: None,
            form_element: None,
            template_insertion_modes: Vec::new(),
        }
    }
}