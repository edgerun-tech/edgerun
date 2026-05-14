use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod net_openid_appauth_redirecturireceiveractivity;
pub mod com_ss_android_ugc_aweme_music_addtodsp_auth_redirecturireceiveractivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::NetOpenidAppauthRedirecturireceiveractivity => net_openid_appauth_redirecturireceiveractivity::handle_event(event, state, capabilities),
        AppEvent::ComSsAndroidUgcAwemeMusicAddtodspAuthRedirecturireceiveractivity => com_ss_android_ugc_aweme_music_addtodsp_auth_redirecturireceiveractivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.VIEW" => net_openid_appauth_redirecturireceiveractivity::handle_event(event, state, capabilities),
            "android.intent.action.VIEW" => com_ss_android_ugc_aweme_music_addtodsp_auth_redirecturireceiveractivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
