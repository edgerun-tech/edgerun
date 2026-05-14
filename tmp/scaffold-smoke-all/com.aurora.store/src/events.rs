#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComAuroraStoreMainactivity,
    ComAuroraStoreDataReceiverDeviceownerreceiver,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComAuroraStoreMainactivity,
    AppEvent::ComAuroraStoreDataReceiverDeviceownerreceiver,
    AppEvent::AndroidAction("android.intent.action.MAIN"),
    AppEvent::AndroidAction("android.intent.action.SEND"),
    AppEvent::AndroidAction("android.intent.action.SHOW_APP_INFO"),
    AppEvent::AndroidAction("android.intent.action.VIEW"),
    AppEvent::AndroidAction("android.app.action.DEVICE_ADMIN_ENABLED"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComAuroraStoreMainactivity => "com.aurora.store.MainActivity",
            AppEvent::ComAuroraStoreDataReceiverDeviceownerreceiver => "com.aurora.store.data.receiver.DeviceOwnerReceiver",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
