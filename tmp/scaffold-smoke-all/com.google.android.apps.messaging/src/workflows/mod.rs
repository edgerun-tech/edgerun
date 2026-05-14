use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_google_android_apps_messaging_ui_conversation_launchconversationactivity;
pub mod com_google_android_apps_messaging_main_mainactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComGoogleAndroidAppsMessagingUiConversationLaunchconversationactivity => com_google_android_apps_messaging_ui_conversation_launchconversationactivity::handle_event(event, state, capabilities),
        AppEvent::ComGoogleAndroidAppsMessagingMainMainactivity => com_google_android_apps_messaging_main_mainactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.SENDTO" => com_google_android_apps_messaging_ui_conversation_launchconversationactivity::handle_event(event, state, capabilities),
            "android.intent.action.VIEW" => com_google_android_apps_messaging_ui_conversation_launchconversationactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
