use lifegraph_capabilities::{CapabilityDescriptor, CapabilityError, CapabilityProvider};
use lifegraph_input::{
    default_input_descriptor, validate_event_read_request, InputDevice, InputDeviceInfo,
    InputDeviceKind, InputEventKind, InputEventRecord,
};
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};

const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_REL: u16 = 0x02;
const EV_ABS: u16 = 0x03;
const EV_MSC: u16 = 0x04;
const EV_SW: u16 = 0x05;

const KEY_A: usize = 30;
const BTN_MOUSE: usize = 0x110;
const BTN_TOUCH: usize = 0x14a;
const BTN_TOOL_PEN: usize = 0x140;
const BTN_GAMEPAD: usize = 0x130;
const REL_X: usize = 0x00;
const REL_Y: usize = 0x01;
const ABS_X: usize = 0x00;
const ABS_Y: usize = 0x01;
const ABS_MT_POSITION_X: usize = 0x35;
const ABS_MT_POSITION_Y: usize = 0x36;
const SW_LID: usize = 0x00;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct TimeVal {
    tv_sec: i64,
    tv_usec: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct LinuxInputEvent {
    time: TimeVal,
    type_: u16,
    code: u16,
    value: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvdevDeviceInfo {
    pub event_node: String,
    pub sysfs_path: PathBuf,
    pub device_name: String,
    pub physical_path: Option<String>,
    pub unique_id: Option<String>,
    pub bus_type: Option<u16>,
    pub vendor_id: Option<u16>,
    pub product_id: Option<u16>,
    pub version: Option<u16>,
    pub modalias: Option<String>,
    pub kind: InputDeviceKind,
    pub capabilities_ev: Vec<u64>,
    pub capabilities_key: Vec<u64>,
    pub capabilities_rel: Vec<u64>,
    pub capabilities_abs: Vec<u64>,
    pub capabilities_sw: Vec<u64>,
}

#[derive(Debug)]
pub struct EvdevInputBackend {
    pub info: EvdevDeviceInfo,
    file: File,
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

fn parse_uevent_map(path: &Path) -> Vec<(String, String)> {
    let Some(raw) = fs::read_to_string(path).ok() else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

fn parse_product_id(value: &str) -> (Option<u16>, Option<u16>, Option<u16>, Option<u16>) {
    let mut parts = value.split('/');
    let bus = parts.next().and_then(|v| u16::from_str_radix(v, 16).ok());
    let vendor = parts.next().and_then(|v| u16::from_str_radix(v, 16).ok());
    let product = parts.next().and_then(|v| u16::from_str_radix(v, 16).ok());
    let version = parts.next().and_then(|v| u16::from_str_radix(v, 16).ok());
    (bus, vendor, product, version)
}

fn parse_hex_bitmap(text: &str) -> Vec<u64> {
    text.split_whitespace()
        .filter_map(|chunk| u64::from_str_radix(chunk, 16).ok())
        .collect()
}

#[cfg(test)]
fn bitmap_with_bit(bit: usize) -> Vec<u64> {
    let mut out = vec![0u64; (bit / 64) + 1];
    out[bit / 64] |= 1u64 << (bit % 64);
    out
}

fn bitmap_contains(words: &[u64], bit: usize) -> bool {
    let word_index = bit / 64;
    if word_index >= words.len() {
        return false;
    }
    let bit_index = bit % 64;
    (words[word_index] & (1u64 << bit_index)) != 0
}

fn classify_device(keys: &[u64], rel: &[u64], abs: &[u64], sw: &[u64]) -> InputDeviceKind {
    if bitmap_contains(keys, BTN_TOUCH)
        || bitmap_contains(abs, ABS_MT_POSITION_X)
        || bitmap_contains(abs, ABS_MT_POSITION_Y)
    {
        return InputDeviceKind::Touch;
    }
    if bitmap_contains(keys, BTN_TOOL_PEN) {
        return InputDeviceKind::Pen;
    }
    if bitmap_contains(keys, BTN_MOUSE)
        || bitmap_contains(rel, REL_X)
        || bitmap_contains(rel, REL_Y)
    {
        return InputDeviceKind::Pointer;
    }
    if bitmap_contains(keys, BTN_GAMEPAD) {
        return InputDeviceKind::Gamepad;
    }
    if bitmap_contains(sw, SW_LID) {
        return InputDeviceKind::Switch;
    }
    if bitmap_contains(keys, KEY_A) || bitmap_contains(abs, ABS_X) || bitmap_contains(abs, ABS_Y) {
        return InputDeviceKind::Keyboard;
    }
    InputDeviceKind::Other
}

fn event_kind(ty: u16) -> InputEventKind {
    match ty {
        EV_SYN => InputEventKind::Synchronization,
        EV_KEY => InputEventKind::Key,
        EV_REL => InputEventKind::RelativeMotion,
        EV_ABS => InputEventKind::AbsoluteMotion,
        EV_SW => InputEventKind::Switch,
        EV_MSC => InputEventKind::Misc,
        other => InputEventKind::Other(other),
    }
}

pub fn discover_evdev_devices() -> Result<Vec<EvdevDeviceInfo>, CapabilityError> {
    discover_evdev_devices_in(Path::new("/sys/class/input"))
}

pub fn discover_evdev_devices_in(root: &Path) -> Result<Vec<EvdevDeviceInfo>, CapabilityError> {
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(root).map_err(|e| CapabilityError::Provider(e.to_string()))? {
        let entry = entry.map_err(|e| CapabilityError::Provider(e.to_string()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("event") {
            continue;
        }
        let sysfs_path = entry.path();
        let device_path =
            fs::canonicalize(sysfs_path.join("device")).unwrap_or(sysfs_path.join("device"));
        let device_name = read_trimmed(&device_path.join("name")).unwrap_or_else(|| name.clone());
        let physical_path = read_trimmed(&device_path.join("phys"));
        let unique_id = read_trimmed(&device_path.join("uniq")).filter(|s| !s.is_empty());
        let uevent = parse_uevent_map(&device_path.join("uevent"));
        let mut bus_type = None;
        let mut vendor_id = None;
        let mut product_id = None;
        let mut version = None;
        let mut modalias = None;
        for (key, value) in &uevent {
            match key.as_str() {
                "PRODUCT" => {
                    let (bus, vendor, product, ver) = parse_product_id(value);
                    bus_type = bus;
                    vendor_id = vendor;
                    product_id = product;
                    version = ver;
                }
                "MODALIAS" => modalias = Some(value.clone()),
                _ => {}
            }
        }
        let capabilities_ev = read_trimmed(&device_path.join("capabilities/ev"))
            .map(|s| parse_hex_bitmap(&s))
            .unwrap_or_default();
        let capabilities_key = read_trimmed(&device_path.join("capabilities/key"))
            .map(|s| parse_hex_bitmap(&s))
            .unwrap_or_default();
        let capabilities_rel = read_trimmed(&device_path.join("capabilities/rel"))
            .map(|s| parse_hex_bitmap(&s))
            .unwrap_or_default();
        let capabilities_abs = read_trimmed(&device_path.join("capabilities/abs"))
            .map(|s| parse_hex_bitmap(&s))
            .unwrap_or_default();
        let capabilities_sw = read_trimmed(&device_path.join("capabilities/sw"))
            .map(|s| parse_hex_bitmap(&s))
            .unwrap_or_default();
        let kind = classify_device(
            &capabilities_key,
            &capabilities_rel,
            &capabilities_abs,
            &capabilities_sw,
        );
        out.push(EvdevDeviceInfo {
            event_node: name,
            sysfs_path,
            device_name,
            physical_path,
            unique_id,
            bus_type,
            vendor_id,
            product_id,
            version,
            modalias,
            kind,
            capabilities_ev,
            capabilities_key,
            capabilities_rel,
            capabilities_abs,
            capabilities_sw,
        });
    }
    out.sort_by(|a, b| a.event_node.cmp(&b.event_node));
    Ok(out)
}

impl EvdevInputBackend {
    pub fn open(info: EvdevDeviceInfo) -> Result<Self, CapabilityError> {
        let path = PathBuf::from("/dev/input").join(&info.event_node);
        let file = OpenOptions::new()
            .read(true)
            .open(&path)
            .map_err(|e| CapabilityError::Provider(format!("open {}: {e}", path.display())))?;
        Ok(Self { info, file })
    }
}

impl CapabilityProvider for EvdevInputBackend {
    fn descriptor(&self) -> CapabilityDescriptor {
        default_input_descriptor("evdev-kernel", &self.info.event_node)
    }
}

impl InputDevice for EvdevInputBackend {
    fn input_info(&self) -> Result<InputDeviceInfo, CapabilityError> {
        Ok(InputDeviceInfo {
            provider: "evdev-kernel".into(),
            instance_id: self.info.event_node.clone(),
            display_name: self.info.device_name.clone(),
            kind: self.info.kind,
            event_node: format!("/dev/input/{}", self.info.event_node),
            physical_path: self.info.physical_path.clone(),
            unique_id: self.info.unique_id.clone(),
        })
    }

    fn read_events(&mut self, max_events: usize) -> Result<Vec<InputEventRecord>, CapabilityError> {
        validate_event_read_request(max_events)?;
        let event_size = std::mem::size_of::<LinuxInputEvent>();
        let mut buf = vec![0u8; event_size * max_events];
        let bytes_read = self.file.read(&mut buf).map_err(|e| {
            CapabilityError::Provider(format!("read evdev fd {}: {e}", self.file.as_raw_fd()))
        })?;
        if bytes_read % event_size != 0 {
            return Err(CapabilityError::Provider(
                "evdev read returned partial input_event records".into(),
            ));
        }
        let mut out = Vec::new();
        for chunk in buf[..bytes_read].chunks_exact(event_size) {
            let event = unsafe { (chunk.as_ptr() as *const LinuxInputEvent).read_unaligned() };
            out.push(InputEventRecord {
                timestamp_sec: event.time.tv_sec,
                timestamp_usec: event.time.tv_usec,
                kind: event_kind(event.type_),
                code: event.code,
                value: event.value,
            });
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("{name}-{unique}"));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn parse_product_id_splits_fields() {
        assert_eq!(
            parse_product_id("0003/046D/085E/0111"),
            (Some(0x0003), Some(0x046d), Some(0x085e), Some(0x0111))
        );
    }

    #[test]
    fn bitmap_parse_and_contains_work() {
        let bits = parse_hex_bitmap("3 0 10");
        assert!(bitmap_contains(&bits, 0));
        assert!(bitmap_contains(&bits, 1));
        assert!(!bitmap_contains(&bits, 2));
    }

    #[test]
    fn classify_pointer_and_touch() {
        assert_eq!(
            classify_device(&bitmap_with_bit(BTN_MOUSE), &[0b11], &[], &[]),
            InputDeviceKind::Pointer
        );
        assert_eq!(
            classify_device(&bitmap_with_bit(BTN_TOUCH), &[], &[], &[]),
            InputDeviceKind::Touch
        );
    }

    #[test]
    fn discover_devices_from_sysfs_layout() {
        let root = temp_root("evdev-input");
        let event = root.join("event0");
        fs::create_dir_all(event.join("device/capabilities")).unwrap();
        fs::write(event.join("device/name"), "Test Keyboard\n").unwrap();
        fs::write(event.join("device/phys"), "usb-test/input0\n").unwrap();
        fs::write(event.join("device/capabilities/ev"), "3\n").unwrap();
        fs::write(
            event.join("device/capabilities/key"),
            format!("{:x}\n", 1u64 << KEY_A),
        )
        .unwrap();
        let devices = discover_evdev_devices_in(&root).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].device_name, "Test Keyboard");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn event_kind_maps_common_types() {
        assert_eq!(event_kind(EV_KEY), InputEventKind::Key);
        assert_eq!(event_kind(EV_REL), InputEventKind::RelativeMotion);
    }
}
