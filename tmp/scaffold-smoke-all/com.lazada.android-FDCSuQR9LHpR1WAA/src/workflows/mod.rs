use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_lazada_activities_enteractivity;
pub mod com_lazada_android_videoproduction_biz_player_videoplayeractivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComLazadaActivitiesEnteractivity => com_lazada_activities_enteractivity::handle_event(event, state, capabilities),
        AppEvent::ComLazadaAndroidVideoproductionBizPlayerVideoplayeractivity => com_lazada_android_videoproduction_biz_player_videoplayeractivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.MAIN" => com_lazada_activities_enteractivity::handle_event(event, state, capabilities),
            "android.intent.action.VIEW" => com_lazada_activities_enteractivity::handle_event(event, state, capabilities),
            "com.lazada.wireless.action.navigator.INTERNAL_NAVIGATION" => com_lazada_activities_enteractivity::handle_event(event, state, capabilities),
            "com.lazada.wireless.action.navigator.INTERNAL_NAVIGATION" => com_lazada_android_videoproduction_biz_player_videoplayeractivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
