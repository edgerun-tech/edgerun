use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_aurora_store_mainactivity;
pub mod com_aurora_store_data_receiver_deviceownerreceiver;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComAuroraStoreMainactivity => com_aurora_store_mainactivity::handle_event(event, state, capabilities),
        AppEvent::ComAuroraStoreDataReceiverDeviceownerreceiver => com_aurora_store_data_receiver_deviceownerreceiver::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.MAIN" => com_aurora_store_mainactivity::handle_event(event, state, capabilities),
            "android.intent.action.SEND" => com_aurora_store_mainactivity::handle_event(event, state, capabilities),
            "android.intent.action.SHOW_APP_INFO" => com_aurora_store_mainactivity::handle_event(event, state, capabilities),
            "android.intent.action.VIEW" => com_aurora_store_mainactivity::handle_event(event, state, capabilities),
            "android.app.action.DEVICE_ADMIN_ENABLED" => com_aurora_store_data_receiver_deviceownerreceiver::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
