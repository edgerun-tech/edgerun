use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_revolut_ui_login_pin_loginactivity;
pub mod com_revolut_feature_app_launcher_impl_ui_launcheractivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComRevolutUiLoginPinLoginactivity => com_revolut_ui_login_pin_loginactivity::handle_event(event, state, capabilities),
        AppEvent::ComRevolutFeatureAppLauncherImplUiLauncheractivity => com_revolut_feature_app_launcher_impl_ui_launcheractivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "OPEN_MAIN_ACTIVITY" => com_revolut_ui_login_pin_loginactivity::handle_event(event, state, capabilities),
            "android.intent.action.MAIN" => com_revolut_ui_login_pin_loginactivity::handle_event(event, state, capabilities),
            "android.intent.action.VIEW" => com_revolut_ui_login_pin_loginactivity::handle_event(event, state, capabilities),
            "com.revolut.category.CHAT_MESSAGE" => com_revolut_ui_login_pin_loginactivity::handle_event(event, state, capabilities),
            "com.revolut.revolut.a2a" => com_revolut_ui_login_pin_loginactivity::handle_event(event, state, capabilities),
            "com.revolut.revolut.action.IN_APP_VERIFICATION" => com_revolut_ui_login_pin_loginactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
