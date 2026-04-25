//! Android display capability via `ANativeWindow` (NDK `libnative_window.so`).

use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityModality, CapabilityOperation,
    CapabilityProvider, CapabilityRole,
};

#[cfg(feature = "android-real")]
mod real {
    use super::*;
    use std::ffi::c_void;

    pub type ANativeWindow = *mut c_void;

    extern "C" {
        fn ANativeWindow_getWidth(window: *mut c_void) -> i32;
        fn ANativeWindow_getHeight(window: *mut c_void) -> i32;
        fn ANativeWindow_getFormat(window: *mut c_void) -> i32;
    }

    pub struct ANativeWindowSurface {
        window: ANativeWindow,
    }

    impl ANativeWindowSurface {
        pub unsafe fn from_raw(window: ANativeWindow) -> Self {
            Self { window }
        }
        pub fn width(&self) -> i32 {
            unsafe { ANativeWindow_getWidth(self.window) }
        }
        pub fn height(&self) -> i32 {
            unsafe { ANativeWindow_getHeight(self.window) }
        }
        pub fn format(&self) -> i32 {
            unsafe { ANativeWindow_getFormat(self.window) }
        }
    }

    pub struct AndroidDisplayProvider {
        surface: Option<ANativeWindowSurface>,
    }

    impl AndroidDisplayProvider {
        pub fn new() -> Self {
            Self { surface: None }
        }
        pub fn set_surface(&mut self, window: ANativeWindow) {
            self.surface = Some(unsafe { ANativeWindowSurface::from_raw(window) });
        }
        pub fn surface_info(&self) -> Option<String> {
            self.surface
                .as_ref()
                .map(|s| format!("{}x{} (format={})", s.width(), s.height(), s.format()))
        }
    }

    impl CapabilityProvider for AndroidDisplayProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "android-display",
                "android",
                CapabilityRole::Output,
                &[CapabilityModality::Display],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query],
                Vec::new(),
            )
        }
    }
}

#[cfg(not(feature = "android-real"))]
mod real {
    use super::*;
    pub struct ANativeWindowSurface;
    pub struct AndroidDisplayProvider;
    impl Default for AndroidDisplayProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl AndroidDisplayProvider {
        pub fn new() -> Self {
            Self
        }
        pub fn set_surface(&mut self, _w: *mut std::ffi::c_void) {}
        pub fn surface_info(&self) -> Option<String> {
            None
        }
    }
    impl CapabilityProvider for AndroidDisplayProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "android-display-stub",
                "stub",
                CapabilityRole::Output,
                &[CapabilityModality::Display],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query],
                Vec::new(),
            )
        }
    }
}

pub use real::{ANativeWindowSurface, AndroidDisplayProvider};
