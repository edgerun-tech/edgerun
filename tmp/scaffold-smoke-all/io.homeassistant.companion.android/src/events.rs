#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    IoHomeassistantCompanionAndroidMatterMattercommissioningactivity,
    IoHomeassistantCompanionAndroidLaunchLaunchactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::IoHomeassistantCompanionAndroidMatterMattercommissioningactivity,
    AppEvent::IoHomeassistantCompanionAndroidLaunchLaunchactivity,
    AppEvent::AndroidAction("com.google.android.gms.home.matter.ACTION_COMMISSION_DEVICE"),
    AppEvent::AndroidAction("android.intent.action.MAIN"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::IoHomeassistantCompanionAndroidMatterMattercommissioningactivity => "io.homeassistant.companion.android.matter.MatterCommissioningActivity",
            AppEvent::IoHomeassistantCompanionAndroidLaunchLaunchactivity => "io.homeassistant.companion.android.launch.LaunchActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
