//! Output (display) management.

/// A display output (monitor).
#[derive(Debug, Clone)]
pub struct Output {
    /// wl_output object id.
    pub id: u32,
    /// Output name (e.g., "eDP-1").
    pub name: String,
    /// Description.
    pub description: String,
    /// Physical dimensions in mm.
    pub physical_width: i32,
    pub physical_height: i32,
    /// Current mode.
    pub mode: OutputMode,
    /// Available modes.
    pub modes: Vec<OutputMode>,
    /// Scale factor.
    pub scale: i32,
    /// Subpixel layout.
    pub subpixel: i32,
    /// Global position in the compositor space.
    pub x: i32,
    pub y: i32,
}

/// A display mode.
#[derive(Debug, Clone, Copy)]
pub struct OutputMode {
    pub width: i32,
    pub height: i32,
    pub refresh_mhz: i32,
    pub preferred: bool,
    pub current: bool,
}

impl Output {
    /// Create an output for a DRM connector.
    pub fn from_drm(
        id: u32,
        name: &str,
        width: u32,
        height: u32,
        refresh_mhz: u32,
        mm_width: u32,
        mm_height: u32,
        connector_type: &str,
    ) -> Self {
        let mode = OutputMode {
            width: width as i32,
            height: height as i32,
            refresh_mhz: refresh_mhz as i32,
            preferred: true,
            current: true,
        };
        Self {
            id,
            name: name.to_string(),
            description: format!("{connector_type} Output"),
            physical_width: mm_width as i32,
            physical_height: mm_height as i32,
            mode,
            modes: vec![mode],
            scale: 1,
            subpixel: 0, // unknown
            x: 0,
            y: 0,
        }
    }
}
