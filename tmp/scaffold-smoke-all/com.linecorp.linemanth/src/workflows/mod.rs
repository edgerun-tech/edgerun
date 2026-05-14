use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_lineman_mart_feature_telemed_feature_videocall_telemedvideocallactivity;
pub mod com_linecorp_linemanth_android_feature_voip_presentation_voipactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComLinemanMartFeatureTelemedFeatureVideocallTelemedvideocallactivity => com_lineman_mart_feature_telemed_feature_videocall_telemedvideocallactivity::handle_event(event, state, capabilities),
        AppEvent::ComLinecorpLinemanthAndroidFeatureVoipPresentationVoipactivity => com_linecorp_linemanth_android_feature_voip_presentation_voipactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            _ => {}
        },
    }
}
