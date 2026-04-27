//! Android Input capability via NDK `libinput.so`.

use alloc::format;
use alloc::vec::Vec;
use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityError, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};
use edgerun_input::InputEventRecord;

#[cfg(all(feature = "android-real", target_os = "android"))]
mod real {
    use super::*;
    use once_cell::sync::OnceCell;
    use std::ffi::c_int;

    #[repr(C)]
    pub struct AInputQueue {
        _private: [u8; 0],
    }
    #[repr(C)]
    pub struct AInputEvent {
        _private: [u8; 0],
    }

    pub const AINPUT_EVENT_TYPE_KEY: i32 = 1;
    pub const AINPUT_EVENT_TYPE_MOTION: i32 = 2;
    pub const AKEY_EVENT_ACTION_DOWN: i32 = 0;
    pub const AKEY_EVENT_ACTION_UP: i32 = 1;

    static LIB_INPUT: OnceCell<libinput::LibInputFns> = OnceCell::new();

    mod libinput {
        use super::*;
        use std::ffi::c_void;

        pub type AInputQueueGetEvent = unsafe extern "C" fn(*mut AInputQueue) -> *mut AInputEvent;
        pub type AInputQueueFinishEvent =
            unsafe extern "C" fn(*mut AInputQueue, *mut AInputEvent, c_int);
        pub type AInputEventGetType = unsafe extern "C" fn(*const AInputEvent) -> c_int;
        pub type AInputEventGetDeviceId = unsafe extern "C" fn(*const AInputEvent) -> c_int;
        pub type AKeyEventGetKeyCode = unsafe extern "C" fn(*const AInputEvent) -> c_int;
        pub type AKeyEventGetAction = unsafe extern "C" fn(*const AInputEvent) -> c_int;

        pub struct LibInputFns {
            pub get_event: AInputQueueGetEvent,
            pub finish_event: AInputQueueFinishEvent,
            pub get_type: AInputEventGetType,
            pub get_device_id: AInputEventGetDeviceId,
            pub get_keycode: AKeyEventGetKeyCode,
            pub get_action: AKeyEventGetAction,
        }

        impl LibInputFns {
            pub fn load() -> Result<Self, CapabilityError> {
                unsafe {
                    let name = std::ffi::CString::new("libinput.so").unwrap();
                    let handle = libc::dlopen(name.as_ptr(), libc::RTLD_LAZY);
                    if handle.is_null() {
                        return Err(CapabilityError::Provider("libinput.so not found".into()));
                    }
                    fn sym<T>(handle: *mut c_void, name: &str) -> Result<T, CapabilityError> {
                        let c_name = std::ffi::CString::new(name).unwrap();
                        let ptr = unsafe { libc::dlsym(handle, c_name.as_ptr()) };
                        if ptr.is_null() {
                            return Err(CapabilityError::Provider(format!("{name} not found")));
                        }
                        Ok(unsafe { std::mem::transmute_copy(&ptr) })
                    }
                    Ok(Self {
                        get_event: sym(handle, "AInputQueue_getEvent")?,
                        finish_event: sym(handle, "AInputQueue_finishEvent")?,
                        get_type: sym(handle, "AInputEvent_getType")?,
                        get_device_id: sym(handle, "AInputEvent_getDeviceId")?,
                        get_keycode: sym(handle, "AKeyEvent_getKeyCode")?,
                        get_action: sym(handle, "AKeyEvent_getAction")?,
                    })
                }
            }
        }
    }

    fn ensure_loaded() -> Result<&'static libinput::LibInputFns, CapabilityError> {
        LIB_INPUT.get_or_try_init(libinput::LibInputFns::load)
    }

    pub struct AInputDevice {
        queue: *mut AInputQueue,
    }
    unsafe impl Send for AInputDevice {}

    impl AInputDevice {
        pub unsafe fn from_raw(queue: *mut AInputQueue) -> Self {
            Self { queue }
        }

        pub fn poll_event(&self) -> Option<InputEventRecord> {
            if self.queue.is_null() {
                return None;
            }
            let fns = ensure_loaded().ok()?;
            unsafe {
                let event = (fns.get_event)(self.queue);
                if event.is_null() {
                    return None;
                }
                let event_type = (fns.get_type)(event);
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default();
                let record = if event_type == AINPUT_EVENT_TYPE_KEY {
                    InputEventRecord {
                        timestamp_sec: ts.as_secs() as i64,
                        timestamp_usec: ts.subsec_micros() as i64,
                        kind: edgerun_input::InputEventKind::Key,
                        code: (fns.get_keycode)(event) as u16,
                        value: if (fns.get_action)(event) == AKEY_EVENT_ACTION_DOWN {
                            1
                        } else {
                            0
                        },
                    }
                } else if event_type == AINPUT_EVENT_TYPE_MOTION {
                    InputEventRecord {
                        timestamp_sec: ts.as_secs() as i64,
                        timestamp_usec: ts.subsec_micros() as i64,
                        kind: edgerun_input::InputEventKind::RelativeMotion,
                        code: 0,
                        value: 0,
                    }
                } else {
                    return None;
                };
                (fns.finish_event)(self.queue, event, 1);
                Some(record)
            }
        }
    }

    pub struct AndroidInputProvider {
        device: Option<AInputDevice>,
    }
    impl AndroidInputProvider {
        pub fn new() -> Self {
            Self { device: None }
        }
        pub fn set_input_queue(&mut self, queue: *mut AInputQueue) {
            self.device = Some(unsafe { AInputDevice::from_raw(queue) });
        }
    }

    impl CapabilityProvider for AndroidInputProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "android-input",
                "android",
                CapabilityRole::Input,
                &[CapabilityModality::Touch],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query],
                Vec::new(),
            )
        }
    }
}

#[cfg(any(not(feature = "android-real"), not(target_os = "android")))]
mod real {
    use super::*;
    pub struct AInputDevice;
    pub struct AndroidInputProvider;
    impl Default for AndroidInputProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl AndroidInputProvider {
        pub fn new() -> Self {
            Self
        }
        pub fn set_input_queue(&mut self, _q: *mut std::ffi::c_void) {}
    }
    impl CapabilityProvider for AndroidInputProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "android-input-stub",
                "stub",
                CapabilityRole::Input,
                &[CapabilityModality::Touch],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query],
                Vec::new(),
            )
        }
    }
}

pub use real::{AInputDevice, AndroidInputProvider};
