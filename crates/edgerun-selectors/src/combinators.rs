//! Selector combinators — from Selectors Level 4.
//! DO NOT EDIT. Regenerate with: scripts/generate_selector_dom.py
extern crate alloc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    ///   — E F matches F descendant of E
    Descendant,
    /// > — E > F matches F direct child of E
    Child,
    /// + — E + F matches F next sibling of E
    NextSibling,
    /// ~ — E ~ F matches F subsequent sibling of E
    SubsequentSibling,
    /// || — E || F matches F in column E
    Column,
}

impl Combinator {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '>' => Some(Combinator::Child),
            '+' => Some(Combinator::NextSibling),
            '~' => Some(Combinator::SubsequentSibling),
            _ => None,
        }
    }

    /// Check for the column combinator (||, two chars).
    pub fn try_from_str(s: &str) -> Option<Self> {
        if s == "||" { Some(Combinator::Column) } else { Self::from_char(s.chars().next()?) }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Descendant => " ",
            Self::Child => ">",
            Self::NextSibling => "+",
            Self::SubsequentSibling => "~",
            Self::Column => "||",
        }
    }
}
