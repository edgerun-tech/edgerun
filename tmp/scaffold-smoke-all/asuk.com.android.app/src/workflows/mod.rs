use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_onesignal_notificationopenedactivityhms;
pub mod com_onesignal_notificationopenedreceiver;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComOnesignalNotificationopenedactivityhms => com_onesignal_notificationopenedactivityhms::handle_event(event, state, capabilities),
        AppEvent::ComOnesignalNotificationopenedreceiver => com_onesignal_notificationopenedreceiver::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.VIEW" => com_onesignal_notificationopenedactivityhms::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
