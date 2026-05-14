use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_google_android_googlequicksearchbox_searchwidgetprovider;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComGoogleAndroidGooglequicksearchboxSearchwidgetprovider => com_google_android_googlequicksearchbox_searchwidgetprovider::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.appwidget.action.APPWIDGET_UPDATE" => com_google_android_googlequicksearchbox_searchwidgetprovider::handle_event(event, state, capabilities),
            "android.appwidget.action.APPWIDGET_UPDATE_OPTIONS" => com_google_android_googlequicksearchbox_searchwidgetprovider::handle_event(event, state, capabilities),
            "com.google.android.finsky.intent.action.UPDATE_DSE" => com_google_android_googlequicksearchbox_searchwidgetprovider::handle_event(event, state, capabilities),
            "com.google.android.finsky.intent.action.UPDATE_DSE_APP_STATE" => com_google_android_googlequicksearchbox_searchwidgetprovider::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
