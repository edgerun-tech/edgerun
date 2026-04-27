//! Android audio capability via AAudio (NDK `libaaudio.so`).

use alloc::format;
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

    pub type AAUDIOStream = *mut c_void;
    pub type AAUDIOStreamBuilder = *mut c_void;

    pub const AAUDIO_FORMAT_PCM_I16: i32 = 1;
    pub const AAUDIO_DIRECTION_OUTPUT: i32 = 0;
    pub const AAUDIO_DIRECTION_INPUT: i32 = 1;
    pub const AAUDIO_PERFORMANCE_MODE_LOW_LATENCY: i32 = 12;

    static LIB_AAUDIO: OnceCell<aaudio::AaudioFns> = OnceCell::new();

    mod aaudio {
        use super::*;
        macro_rules! fn_ptr {
            ($name:ident, $($args:tt)*) => { pub type $name = unsafe extern "C" fn $($args)*; }
        }
        fn_ptr!(CreateBuilder, (*mut AAUDIOStreamBuilder) -> i32);
        fn_ptr!(SetDirection, (AAUDIOStreamBuilder, i32) -> i32);
        fn_ptr!(SetFormat, (AAUDIOStreamBuilder, i32) -> i32);
        fn_ptr!(SetSampleRate, (AAUDIOStreamBuilder, i32) -> i32);
        fn_ptr!(SetChannelCount, (AAUDIOStreamBuilder, i32) -> i32);
        fn_ptr!(SetPerformance, (AAUDIOStreamBuilder, i32) -> i32);
        fn_ptr!(OpenStream, (AAUDIOStreamBuilder, *mut AAUDIOStream) -> i32);
        fn_ptr!(DeleteBuilder, (AAUDIOStreamBuilder) -> i32);
        fn_ptr!(Read, (AAUDIOStream, *mut c_void, i32, i64) -> i32);
        fn_ptr!(Write, (AAUDIOStream, *const c_void, i32, i64) -> i32);
        fn_ptr!(Close, (AAUDIOStream) -> i32);

        pub struct AaudioFns {
            pub create_builder: CreateBuilder,
            pub set_direction: SetDirection,
            pub set_format: SetFormat,
            pub set_sample_rate: SetSampleRate,
            pub set_channel_count: SetChannelCount,
            pub set_performance: SetPerformance,
            pub open_stream: OpenStream,
            pub delete_builder: DeleteBuilder,
            pub read: Read,
            pub write: Write,
            pub close: Close,
        }

        impl AaudioFns {
            pub fn load() -> Result<Self, CapabilityError> {
                unsafe {
                    let name = std::ffi::CString::new("libaaudio.so").unwrap();
                    let handle = libc::dlopen(name.as_ptr(), libc::RTLD_LAZY);
                    if handle.is_null() {
                        return Err(CapabilityError::Provider(
                            "libaaudio.so not found (requires Android 8.0+)".into(),
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
                        create_builder: sym(handle, "AAudio_createStreamBuilder")?,
                        set_direction: sym(handle, "AAudioStreamBuilder_setDirection")?,
                        set_format: sym(handle, "AAudioStreamBuilder_setFormat")?,
                        set_sample_rate: sym(handle, "AAudioStreamBuilder_setSampleRate")?,
                        set_channel_count: sym(handle, "AAudioStreamBuilder_setChannelCount")?,
                        set_performance: sym(handle, "AAudioStreamBuilder_setPerformanceMode")?,
                        open_stream: sym(handle, "AAudioStreamBuilder_openStream")?,
                        delete_builder: sym(handle, "AAudioStreamBuilder_delete")?,
                        read: sym(handle, "AAudioStream_read")?,
                        write: sym(handle, "AAudioStream_write")?,
                        close: sym(handle, "AAudioStream_close")?,
                    })
                }
            }
        }
    }

    fn ensure_loaded() -> Result<&'static aaudio::AaudioFns, CapabilityError> {
        LIB_AAUDIO.get_or_try_init(aaudio::AaudioFns::load)
    }

    pub struct AAudioStream {
        stream: AAUDIOStream,
    }

    impl AAudioStream {
        pub fn open(
            direction: i32,
            sample_rate: i32,
            channels: i32,
        ) -> Result<Self, CapabilityError> {
            let fns = ensure_loaded()?;
            unsafe {
                let mut builder: AAUDIOStreamBuilder = std::ptr::null_mut();
                if (fns.create_builder)(&mut builder) != 0 {
                    return Err(CapabilityError::Provider(
                        "AAudio_createStreamBuilder failed".into(),
                    ));
                }
                (fns.set_direction)(builder, direction);
                (fns.set_format)(builder, AAUDIO_FORMAT_PCM_I16);
                (fns.set_sample_rate)(builder, sample_rate);
                (fns.set_channel_count)(builder, channels);
                (fns.set_performance)(builder, AAUDIO_PERFORMANCE_MODE_LOW_LATENCY);
                let mut stream: AAUDIOStream = std::ptr::null_mut();
                let result = (fns.open_stream)(builder, &mut stream);
                (fns.delete_builder)(builder);
                if result != 0 || stream.is_null() {
                    return Err(CapabilityError::Provider(format!(
                        "AAudioStreamBuilder_openStream failed: {result}"
                    )));
                }
                Ok(Self { stream })
            }
        }

        pub fn read(&self, buffer: &mut [i16], frames: i32) -> Result<i32, CapabilityError> {
            let fns = ensure_loaded()?;
            unsafe {
                let result = (fns.read)(
                    self.stream,
                    buffer.as_mut_ptr() as *mut c_void,
                    frames,
                    1_000_000_000,
                );
                if result < 0 {
                    Err(CapabilityError::Provider(format!(
                        "AAudioStream_read failed: {result}"
                    )))
                } else {
                    Ok(result)
                }
            }
        }

        pub fn write(&self, buffer: &[i16], frames: i32) -> Result<i32, CapabilityError> {
            let fns = ensure_loaded()?;
            unsafe {
                let result = (fns.write)(
                    self.stream,
                    buffer.as_ptr() as *const c_void,
                    frames,
                    1_000_000_000,
                );
                if result < 0 {
                    Err(CapabilityError::Provider(format!(
                        "AAudioStream_write failed: {result}"
                    )))
                } else {
                    Ok(result)
                }
            }
        }
    }

    impl Drop for AAudioStream {
        fn drop(&mut self) {
            if let Ok(fns) = ensure_loaded() {
                unsafe {
                    (fns.close)(self.stream);
                }
            }
        }
    }

    pub struct AndroidAudioInputProvider {
        stream: Option<AAudioStream>,
    }
    impl AndroidAudioInputProvider {
        pub fn new() -> Self {
            Self { stream: None }
        }
        pub fn start_capture(
            &mut self,
            sample_rate: i32,
            channels: i32,
        ) -> Result<(), CapabilityError> {
            self.stream = Some(AAudioStream::open(
                AAUDIO_DIRECTION_INPUT,
                sample_rate,
                channels,
            )?);
            Ok(())
        }
    }
    impl CapabilityProvider for AndroidAudioInputProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "android-microphone",
                "android",
                CapabilityRole::Input,
                &[CapabilityModality::Auditory],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query],
                Vec::new(),
            )
        }
    }

    pub struct AndroidAudioOutputProvider {
        stream: Option<AAudioStream>,
    }
    impl AndroidAudioOutputProvider {
        pub fn new() -> Self {
            Self { stream: None }
        }
        pub fn start_playback(
            &mut self,
            sample_rate: i32,
            channels: i32,
        ) -> Result<(), CapabilityError> {
            self.stream = Some(AAudioStream::open(
                AAUDIO_DIRECTION_OUTPUT,
                sample_rate,
                channels,
            )?);
            Ok(())
        }
    }
    impl CapabilityProvider for AndroidAudioOutputProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "android-speaker",
                "android",
                CapabilityRole::Output,
                &[CapabilityModality::Auditory],
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
    pub struct AAudioStream;
    pub struct AndroidAudioInputProvider;
    pub struct AndroidAudioOutputProvider;
    impl Default for AndroidAudioInputProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl AndroidAudioInputProvider {
        pub fn new() -> Self {
            Self
        }
        pub fn start_capture(&mut self, _sr: i32, _ch: i32) -> Result<(), CapabilityError> {
            Ok(())
        }
    }
    impl Default for AndroidAudioOutputProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl AndroidAudioOutputProvider {
        pub fn new() -> Self {
            Self
        }
        pub fn start_playback(&mut self, _sr: i32, _ch: i32) -> Result<(), CapabilityError> {
            Ok(())
        }
    }
    impl CapabilityProvider for AndroidAudioInputProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "android-microphone-stub",
                "stub",
                CapabilityRole::Input,
                &[CapabilityModality::Auditory],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query],
                Vec::new(),
            )
        }
    }
    impl CapabilityProvider for AndroidAudioOutputProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "android-speaker-stub",
                "stub",
                CapabilityRole::Output,
                &[CapabilityModality::Auditory],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query],
                Vec::new(),
            )
        }
    }
}

pub use real::{AAudioStream, AndroidAudioInputProvider, AndroidAudioOutputProvider};
