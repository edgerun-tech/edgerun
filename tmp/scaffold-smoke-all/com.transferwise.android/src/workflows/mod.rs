use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_wise_deeplink_deeplinkproxyactivity;
pub mod com_wise_notifications_presentation_preferences_notificationpreferencesactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComWiseDeeplinkDeeplinkproxyactivity => com_wise_deeplink_deeplinkproxyactivity::handle_event(event, state, capabilities),
        AppEvent::ComWiseNotificationsPresentationPreferencesNotificationpreferencesactivity => com_wise_notifications_presentation_preferences_notificationpreferencesactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.VIEW" => com_wise_deeplink_deeplinkproxyactivity::handle_event(event, state, capabilities),
            "android.intent.action.MAIN" => com_wise_notifications_presentation_preferences_notificationpreferencesactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
