use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_google_android_gms_accountsettings_ui_zeropartyentrypointactivity;
pub mod com_google_android_gms_accountsettings_ui_settingsloaderactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComGoogleAndroidGmsAccountsettingsUiZeropartyentrypointactivity => com_google_android_gms_accountsettings_ui_zeropartyentrypointactivity::handle_event(event, state, capabilities),
        AppEvent::ComGoogleAndroidGmsAccountsettingsUiSettingsloaderactivity => com_google_android_gms_accountsettings_ui_settingsloaderactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "com.android.settings.action.VIEW_ACCOUNT" => com_google_android_gms_accountsettings_ui_zeropartyentrypointactivity::handle_event(event, state, capabilities),
            "com.google.android.gms.accountsettings.VIEW_SETTINGS_0P" => com_google_android_gms_accountsettings_ui_zeropartyentrypointactivity::handle_event(event, state, capabilities),
            "com.google.android.gms.accountsettings.action.SAFETY_CENTER_SECURITY_CHECKUP" => com_google_android_gms_accountsettings_ui_zeropartyentrypointactivity::handle_event(event, state, capabilities),
            "com.google.android.gms.accountsettings.action.BROWSE_SETTINGS" => com_google_android_gms_accountsettings_ui_settingsloaderactivity::handle_event(event, state, capabilities),
            "com.google.android.gms.accountsettings.action.VIEW_SETTINGS" => com_google_android_gms_accountsettings_ui_settingsloaderactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
