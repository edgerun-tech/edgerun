use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_grab_partner_sdk_wrapper_deeplink_deeplinkactivity;
pub mod com_grab_gkyc_sdk_features_basic_ui_basicactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComGrabPartnerSdkWrapperDeeplinkDeeplinkactivity => com_grab_partner_sdk_wrapper_deeplink_deeplinkactivity::handle_event(event, state, capabilities),
        AppEvent::ComGrabGkycSdkFeaturesBasicUiBasicactivity => com_grab_gkyc_sdk_features_basic_ui_basicactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.VIEW" => com_grab_partner_sdk_wrapper_deeplink_deeplinkactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
