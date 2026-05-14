use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_linecorp_line_settings_customappicon_customappiconsplashactivity_basic1splashactivity;
pub mod com_linecorp_line_settings_customappicon_customappiconsplashactivity_promotionhalloween1splashactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComLinecorpLineSettingsCustomappiconCustomappiconsplashactivityBasic1splashactivity => com_linecorp_line_settings_customappicon_customappiconsplashactivity_basic1splashactivity::handle_event(event, state, capabilities),
        AppEvent::ComLinecorpLineSettingsCustomappiconCustomappiconsplashactivityPromotionhalloween1splashactivity => com_linecorp_line_settings_customappicon_customappiconsplashactivity_promotionhalloween1splashactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.MAIN" => com_linecorp_line_settings_customappicon_customappiconsplashactivity_basic1splashactivity::handle_event(event, state, capabilities),
            "android.intent.action.MAIN" => com_linecorp_line_settings_customappicon_customappiconsplashactivity_promotionhalloween1splashactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
