use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_stagnationlab_sk_mainactivity;
pub mod com_huawei_hms_flutter_push_hms_flutterhmsmessageservice;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComStagnationlabSkMainactivity => com_stagnationlab_sk_mainactivity::handle_event(event, state, capabilities),
        AppEvent::ComHuaweiHmsFlutterPushHmsFlutterhmsmessageservice => com_huawei_hms_flutter_push_hms_flutterhmsmessageservice::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.MAIN" => com_stagnationlab_sk_mainactivity::handle_event(event, state, capabilities),
            "android.intent.action.VIEW" => com_stagnationlab_sk_mainactivity::handle_event(event, state, capabilities),
            "com.huawei.push.action.MESSAGING_EVENT" => com_huawei_hms_flutter_push_hms_flutterhmsmessageservice::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
