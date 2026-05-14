use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_google_android_partnersetup_bootreceiver;
pub mod com_google_android_partnersetup_phonestatereceiver;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComGoogleAndroidPartnersetupBootreceiver => com_google_android_partnersetup_bootreceiver::handle_event(event, state, capabilities),
        AppEvent::ComGoogleAndroidPartnersetupPhonestatereceiver => com_google_android_partnersetup_phonestatereceiver::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.BOOT_COMPLETED" => com_google_android_partnersetup_bootreceiver::handle_event(event, state, capabilities),
            "android.intent.action.MY_PACKAGE_REPLACED" => com_google_android_partnersetup_bootreceiver::handle_event(event, state, capabilities),
            "android.intent.action.SIM_STATE_CHANGED" => com_google_android_partnersetup_phonestatereceiver::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
