use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod io_homeassistant_companion_android_matter_mattercommissioningactivity;
pub mod io_homeassistant_companion_android_launch_launchactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::IoHomeassistantCompanionAndroidMatterMattercommissioningactivity => io_homeassistant_companion_android_matter_mattercommissioningactivity::handle_event(event, state, capabilities),
        AppEvent::IoHomeassistantCompanionAndroidLaunchLaunchactivity => io_homeassistant_companion_android_launch_launchactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "com.google.android.gms.home.matter.ACTION_COMMISSION_DEVICE" => io_homeassistant_companion_android_matter_mattercommissioningactivity::handle_event(event, state, capabilities),
            "android.intent.action.MAIN" => io_homeassistant_companion_android_launch_launchactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
