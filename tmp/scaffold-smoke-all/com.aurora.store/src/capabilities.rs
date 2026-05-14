#[derive(Debug, Default)]
pub struct Capabilities {
    pub accounts_identity: AccountsIdentityCapability,
    pub network: NetworkCapability,
    pub location: LocationCapability,
    pub notifications: NotificationCapability,
    pub storage: StorageCapability,
    pub local_state: LocalStateCapability,
    pub camera_media: CameraMediaCapability,
    pub bluetooth_nearby: BluetoothNearbyCapability,
    pub secrets_crypto: SecretsCryptoCapability,
    pub contacts_calendar: ContactsCalendarCapability,
    pub app_events: AppEventsCapability,
    pub sensors: SensorsCapability,
    pub background_tasks: BackgroundTasksCapability,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CapabilityIntent {
    pub capability: &'static str,
    pub operation: &'static str,
    pub source: &'static str,
}

#[derive(Debug, Default)] pub struct AccountsIdentityCapability;
#[derive(Debug, Default)] pub struct NetworkCapability;
#[derive(Debug, Default)] pub struct LocationCapability;
#[derive(Debug, Default)] pub struct NotificationCapability;
#[derive(Debug, Default)] pub struct StorageCapability;
#[derive(Debug, Default)] pub struct LocalStateCapability;
#[derive(Debug, Default)] pub struct CameraMediaCapability;
#[derive(Debug, Default)] pub struct BluetoothNearbyCapability;
#[derive(Debug, Default)] pub struct SecretsCryptoCapability;
#[derive(Debug, Default)] pub struct ContactsCalendarCapability;
#[derive(Debug, Default)] pub struct AppEventsCapability;
#[derive(Debug, Default)] pub struct SensorsCapability;
#[derive(Debug, Default)] pub struct SmsTelephonyCapability;
#[derive(Debug, Default)] pub struct BackgroundTasksCapability;

impl AccountsIdentityCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent("accounts_identity", operation, source) } }
impl NetworkCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent("network_web", operation, source) } }
impl LocationCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent("location", operation, source) } }
impl NotificationCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent("notifications", operation, source) } }
impl StorageCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent("files_storage", operation, source) } }
impl LocalStateCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent("database_preferences", operation, source) } }
impl CameraMediaCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent("camera_media_capture", operation, source) } }
impl BluetoothNearbyCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent("bluetooth_nearby", operation, source) } }
impl SecretsCryptoCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent("crypto_security", operation, source) } }
impl ContactsCalendarCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent("contacts_calendar", operation, source) } }
impl AppEventsCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent("package_intents", operation, source) } }
impl SensorsCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent("sensors", operation, source) } }
impl SmsTelephonyCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent("sms_telephony", operation, source) } }
impl BackgroundTasksCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent("work_background", operation, source) } }

fn intent(capability: &'static str, operation: &'static str, source: &'static str) -> CapabilityIntent {
    CapabilityIntent { capability, operation, source }
}
