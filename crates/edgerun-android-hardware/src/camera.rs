//! Android Camera capability via Camera2 NDK (`libcamera2_ndk.so`).

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityError, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};

#[cfg(all(feature = "android-real", target_os = "android"))]
mod real {
    use super::*;
    use once_cell::sync::OnceCell;
    use std::ffi::c_void;

    pub type ACameraManager = *mut c_void;
    pub type ACameraIdList = *mut c_void;
    pub const ACAMERA_ERROR_OK: i32 = 0;

    static LIB_CAMERA2: OnceCell<camera2::Camera2Fns> = OnceCell::new();

    mod camera2 {
        use super::*;
        pub type ACameraManagerCreate = unsafe extern "C" fn() -> ACameraManager;
        pub type ACameraManagerDelete = unsafe extern "C" fn(ACameraManager);
        pub type ACameraManagerGetCameraIdList =
            unsafe extern "C" fn(ACameraManager, *mut *mut ACameraIdList) -> i32;
        pub type ACameraManagerDeleteCameraIdList = unsafe extern "C" fn(*mut ACameraIdList);
        pub type ACameraIdListGetNumCameras = unsafe extern "C" fn(*const ACameraIdList) -> i32;
        pub type ACameraIdListGetCameraId =
            unsafe extern "C" fn(*const ACameraIdList, i32) -> *const i8;

        pub struct Camera2Fns {
            pub create: ACameraManagerCreate,
            pub delete: ACameraManagerDelete,
            pub get_camera_id_list: ACameraManagerGetCameraIdList,
            pub delete_camera_id_list: ACameraManagerDeleteCameraIdList,
            pub get_num_cameras: ACameraIdListGetNumCameras,
            pub get_camera_id: ACameraIdListGetCameraId,
        }

        impl Camera2Fns {
            pub fn load() -> Result<Self, CapabilityError> {
                unsafe {
                    let name = std::ffi::CString::new("libcamera2_ndk.so").unwrap();
                    let handle = libc::dlopen(name.as_ptr(), libc::RTLD_LAZY);
                    if handle.is_null() {
                        return Err(CapabilityError::Provider(
                            "libcamera2_ndk.so not found (requires Android 9.0+)".into(),
                        ));
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
                        create: sym(handle, "ACameraManager_create")?,
                        delete: sym(handle, "ACameraManager_delete")?,
                        get_camera_id_list: sym(handle, "ACameraManager_getCameraIdList")?,
                        delete_camera_id_list: sym(handle, "ACameraManager_deleteCameraIdList")?,
                        get_num_cameras: sym(handle, "ACameraIdList_getNumCameras")?,
                        get_camera_id: sym(handle, "ACameraIdList_getCameraId")?,
                    })
                }
            }
        }
    }

    fn ensure_loaded() -> Result<&'static camera2::Camera2Fns, CapabilityError> {
        LIB_CAMERA2.get_or_try_init(camera2::Camera2Fns::load)
    }

    unsafe fn cstr_to_string(ptr: *const i8) -> String {
        if ptr.is_null() {
            return String::new();
        }
        std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
    }

    pub struct Camera2Session {
        manager: ACameraManager,
        camera_ids: Vec<String>,
    }

    impl Camera2Session {
        pub fn open() -> Result<Self, CapabilityError> {
            let fns = ensure_loaded()?;
            unsafe {
                let manager = (fns.create)();
                if manager.is_null() {
                    return Err(CapabilityError::Provider(
                        "ACameraManager_create failed".into(),
                    ));
                }
                let mut id_list: *mut ACameraIdList = std::ptr::null_mut();
                let status = (fns.get_camera_id_list)(manager, &mut id_list);
                if status != ACAMERA_ERROR_OK || id_list.is_null() {
                    (fns.delete)(manager);
                    return Err(CapabilityError::Provider(format!(
                        "ACameraManager_getCameraIdList failed: {status}"
                    )));
                }
                let num = (fns.get_num_cameras)(id_list);
                let mut ids = Vec::new();
                for i in 0..num {
                    ids.push(cstr_to_string((fns.get_camera_id)(id_list, i)));
                }
                (fns.delete_camera_id_list)(id_list);
                Ok(Self {
                    manager,
                    camera_ids: ids,
                })
            }
        }
        pub fn available_cameras(&self) -> &[String] {
            &self.camera_ids
        }
    }

    impl Drop for Camera2Session {
        fn drop(&mut self) {
            if let Ok(fns) = ensure_loaded() {
                unsafe {
                    (fns.delete)(self.manager);
                }
            }
        }
    }

    pub struct AndroidCameraProvider {
        session: Option<Camera2Session>,
    }

    impl AndroidCameraProvider {
        pub fn new() -> Self {
            Self { session: None }
        }
        pub fn init(&mut self) -> Result<(), CapabilityError> {
            self.session = Some(Camera2Session::open()?);
            Ok(())
        }
    }

    impl CapabilityProvider for AndroidCameraProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "android-camera",
                "android",
                CapabilityRole::Input,
                &[CapabilityModality::Visual],
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
    pub struct Camera2Session;
    pub struct AndroidCameraProvider;
    impl Default for AndroidCameraProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl AndroidCameraProvider {
        pub fn new() -> Self {
            Self
        }
        pub fn init(&mut self) -> Result<(), CapabilityError> {
            Ok(())
        }
    }
    impl CapabilityProvider for AndroidCameraProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "android-camera-stub",
                "stub",
                CapabilityRole::Input,
                &[CapabilityModality::Visual],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query],
                Vec::new(),
            )
        }
    }
}

pub use real::{AndroidCameraProvider, Camera2Session};
