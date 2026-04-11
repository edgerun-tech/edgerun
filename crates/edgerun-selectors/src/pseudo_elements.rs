//! Pseudo-element selectors — generated from Selectors Level 4.
//! DO NOT EDIT. Regenerate with: scripts/generate_selector_dom.py
extern crate alloc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PseudoElement {
    /// ::before
    Before,
    /// ::marker
    Marker,
    /// ::after
    After,
    /// ::first-line
    FirstLine,
    /// ::slotted
    Slotted,
    /// ::lang
    Lang,
    /// ::column
    Column,
    /// ::first-letter
    FirstLetter,
    /// ::shadow
    Shadow,
    /// ::placeholder
    Placeholder,
    /// ::selection
    Selection,
    /// ::backdrop
    Backdrop,
    /// ::file-selector-button
    FileSelectorButton,
    /// ::page
    Page,
    /// ::part
    Part,
}

impl PseudoElement {
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "::before" => Some(PseudoElement::Before),
            "::marker" => Some(PseudoElement::Marker),
            "::after" => Some(PseudoElement::After),
            "::first-line" => Some(PseudoElement::FirstLine),
            "::slotted" => Some(PseudoElement::Slotted),
            "::lang" => Some(PseudoElement::Lang),
            "::column" => Some(PseudoElement::Column),
            "::first-letter" => Some(PseudoElement::FirstLetter),
            "::shadow" => Some(PseudoElement::Shadow),
            "::placeholder" => Some(PseudoElement::Placeholder),
            "::selection" => Some(PseudoElement::Selection),
            "::backdrop" => Some(PseudoElement::Backdrop),
            "::file-selector-button" => Some(PseudoElement::FileSelectorButton),
            "::page" => Some(PseudoElement::Page),
            "::part" => Some(PseudoElement::Part),
            _ => None,
        }
    }
}