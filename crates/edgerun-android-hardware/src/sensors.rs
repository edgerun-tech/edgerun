//! Android sensors via `libsensor.so` (NDK `ASensorManager`).

use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityError, CapabilityModality,
    CapabilityOperation, CapabilityRole, CapabilityProvider,
};

#[cfg(feature = "android-real")]
mod real {
    use super::*;
    use once_cell::sync::OnceCell;
    use std::ffi::c_void;

    pub type ASensorManager = *mut c_void;
    pub type ASensor = *const c_void;

    pub const ASENSOR_TYPE_ACCELEROMETER: i32 = 1;
    pub const ASENSOR_TYPE_GYROSCOPE: i32 = 4;
    pub const ASENSOR_TYPE_LIGHT: i32 = 5;
    pub const ASENSOR_TYPE_PROXIMITY: i32 = 8;
    pub const ASENSOR_TYPE_MAGNETIC_FIELD: i32 = 2;
    pub const ASENSOR_TYPE_PRESSURE: i32 = 6;
    pub const ASENSOR_TYPE_GRAVITY: i32 = 9;
    pub const ASENSOR_TYPE_LINEAR_ACCELERATION: i32 = 10;

    static LIB_SENSOR: OnceCell<sensor::SensorFns> = OnceCell::new();

    mod sensor {
        use super::*;
        pub type GetInstance = unsafe extern "C" fn() -> ASensorManager;
        pub type GetDefaultSensor = unsafe extern "C" fn(ASensorManager, i32) -> ASensor;
        pub type GetName = unsafe extern "C" fn(ASensor) -> *const i8;
        pub type GetVendor = unsafe extern "C" fn(ASensor) -> *const i8;
        pub type GetResolution = unsafe extern "C" fn(ASensor) -> f32;

        pub struct SensorFns {
            pub get_instance: GetInstance,
            pub get_default_sensor: GetDefaultSensor,
            pub get_name: GetName,
            pub get_vendor: GetVendor,
            pub get_resolution: GetResolution,
        }

        pub fn load() -> Result<Self, CapabilityError> {
            unsafe {
                let name = std::ffi::CString::new("libsensor.so").unwrap();
                let handle = libc::dlopen(name.as_ptr(), libc::RTLD_LAZY);
                if handle.is_null() {
                    return Err(CapabilityError::Provider("libsensor.so not found".into()));
                }
                fn sym<T>(handle: *mut c_void, name: &str) -> Result<T, CapabilityError> {
                    let c_name = std::ffi::CString::new(name).unwrap();
                    let ptr = libc::dlsym(handle, c_name.as_ptr());
                    if ptr.is_null() {
                        return Err(CapabilityError::Provider(format!("{name} not found")));
                    }
                    Ok(std::mem::transmute(ptr))
                }
                Ok(Self {
                    get_instance: sym(handle, "ASensorManager_getInstance")?,
                    get_default_sensor: sym(handle, "ASensorManager_getDefaultSensor")?,
                    get_name: sym(handle, "ASensor_getName")?,
                    get_vendor: sym(handle, "ASensor_getVendor")?,
                    get_resolution: sym(handle, "ASensor_getResolution")?,
                })
            }
        }
    }

    fn ensure_loaded() -> Result<&'static sensor::SensorFns, CapabilityError> {
        LIB_SENSOR.get_or_try_init(sensor::SensorFns::load)
    }

    pub struct ASensorManagerWrapper { manager: ASensorManager }

    impl ASensorManagerWrapper {
        pub fn new() -> Result<Self, CapabilityError> {
            let fns = ensure_loaded()?;
            unsafe {
                let manager = (fns.get_instance)();
                if manager.is_null() {
                    return Err(CapabilityError::Provider("ASensorManager_getInstance returned null".into()));
                }
                Ok(Self { manager })
            }
        }

        pub fn get_sensor(&self, sensor_type: i32) -> Option<String> {
            let fns = ensure_loaded().ok()?;
            unsafe {
                let sensor = (fns.get_default_sensor)(self.manager, sensor_type);
                if sensor.is_null() { return None; }
                let name = std::ffi::CStr::from_ptr((fns.get_name)(sensor)).to_string_lossy().into_owned();
                let vendor = std::ffi::CStr::from_ptr((fns.get_vendor)(sensor)).to_string_lossy().into_owned();
                let res = (fns.get_resolution)(sensor);
                Some(format!("{name} ({vendor}, res={res:.2})"))
            }
        }

        pub fn list_sensors(&self) -> Vec<String> {
            [ASENSOR_TYPE_ACCELEROMETER, ASENSOR_TYPE_GYROSCOPE, ASENSOR_TYPE_LIGHT,
             ASENSOR_TYPE_PROXIMITY, ASENSOR_TYPE_MAGNETIC_FIELD, ASENSOR_TYPE_PRESSURE,
             ASENSOR_TYPE_GRAVITY, ASENSOR_TYPE_LINEAR_ACCELERATION]
                .iter().filter_map(|&t| self.get_sensor(t)).collect()
        }
    }

    pub struct AndroidSensorProvider { manager: Option<ASensorManagerWrapper> }

    impl AndroidSensorProvider {
        pub fn new() -> Self { Self { manager: None } }
    }

    impl CapabilityProvider for AndroidSensorProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor("android-sensors", "android", CapabilityRole::Input,
                &[CapabilityModality::Other],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query], Vec::new())
        }
    }
}

#[cfg(not(feature = "android-real"))]
mod real {
    use super::*;
    pub struct ASensorManager;
    pub struct ASensorManagerWrapper;
    pub struct AndroidSensorProvider;
    impl AndroidSensorProvider { pub fn new() -> Self { Self } }
    impl CapabilityProvider for AndroidSensorProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor("android-sensors-stub", "stub", CapabilityRole::Input,
                &[CapabilityModality::Other],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query], Vec::new())
        }
    }
}

pub use real::{AndroidSensorProvider, ASensorManagerWrapper};
