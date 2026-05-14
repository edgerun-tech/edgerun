use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_twitter_app_settings_settingsrootcompatactivity;
pub mod com_twitter_android_authorizeappactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComTwitterAppSettingsSettingsrootcompatactivity => com_twitter_app_settings_settingsrootcompatactivity::handle_event(event, state, capabilities),
        AppEvent::ComTwitterAndroidAuthorizeappactivity => com_twitter_android_authorizeappactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.MAIN" => com_twitter_app_settings_settingsrootcompatactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
