//! Android Location capability.
//! Reads GPS data from sysfs (available on all Android devices via Linux kernel).

use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityModality,
    CapabilityOperation, CapabilityRole, CapabilityProvider,
};

#[cfg(feature = "android-real")]
mod real {
    use super::*;

    #[derive(Clone, Debug, Default)]
    pub struct Location {
        pub latitude: f64,
        pub longitude: f64,
        pub altitude: f64,
        pub accuracy_meters: f32,
        pub provider: String,
    }

    fn read_sysfs_prop(path: &str) -> Option<String> {
        std::fs::read_to_string(path).ok().map(|s| s.trim().to_string())
    }

    pub struct AndroidLocationProvider { last_location: Option<Location> }

    impl AndroidLocationProvider {
        pub fn new() -> Self { Self { last_location: None } }

        pub fn get_location(&mut self) -> Option<Location> {
            if let (Ok(lat_str), Ok(lon_str)) = (
                read_sysfs_prop("/sys/class/gps/latitude"),
                read_sysfs_prop("/sys/class/gps/longitude"),
            ) {
                if let (Ok(lat), Ok(lon)) = (lat_str.parse::<f64>(), lon_str.parse::<f64>()) {
                    let loc = Location {
                        latitude: lat, longitude: lon, altitude: 0.0,
                        accuracy_meters: 10.0, provider: "sysfs".into(),
                    };
                    self.last_location = Some(loc.clone());
                    return Some(loc);
                }
            }
            self.last_location.clone()
        }
    }

    impl CapabilityProvider for AndroidLocationProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor("android-location", "android", CapabilityRole::Input,
                &[CapabilityModality::Other],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query], Vec::new())
        }
    }
}

#[cfg(not(feature = "android-real"))]
mod real {
    use super::*;
    pub struct AndroidLocationProvider;
    impl Default for AndroidLocationProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl AndroidLocationProvider { pub fn new() -> Self { Self } }
    impl CapabilityProvider for AndroidLocationProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor("android-location-stub", "stub", CapabilityRole::Input,
                &[CapabilityModality::Other],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query], Vec::new())
        }
    }
}

pub use real::AndroidLocationProvider;
