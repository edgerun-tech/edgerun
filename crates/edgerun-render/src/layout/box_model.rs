//! Box model resolution — generated from CSS Box proto.
//! DO NOT EDIT. Regenerate with: scripts/generate_renderer.py
#[derive(Debug, Clone, Copy)]
pub struct EdgeValues { pub top: f64, pub right: f64, pub bottom: f64, pub left: f64 }
impl EdgeValues {
    pub fn width(&self) -> f64 { self.left + self.right }
    pub fn height(&self) -> f64 { self.top + self.bottom }
    pub fn all_same(v: f64) -> Self { Self { top: v, right: v, bottom: v, left: v } }
    pub fn zero() -> Self { Self { top: 0.0, right: 0.0, bottom: 0.0, left: 0.0 } }
}

/// Does this element establish a new BFC?
pub fn establishes_bfc(display_outer: u8, overflow: u8, position: u8, contain: u8) -> bool {
    // display_outer: 2=flow-root, 4=flex, 5=grid, 3=table
    if matches!(display_outer, 2|3|4|5) { return true; }
    // overflow: 2=hidden, 4=scroll, 5=auto
    if matches!(overflow, 2|4|5) { return true; }
    // contain: 2=strict, 4=layout, 3=content
    if matches!(contain, 2|3|4) { return true; }
    position != 1 // not static
}

pub fn clips_overflow(o: u8) -> bool { matches!(o, 2|3) } // hidden or clip
