#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComGoogleAndroidGmsAccountsettingsUiZeropartyentrypointactivity,
    ComGoogleAndroidGmsAccountsettingsUiSettingsloaderactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComGoogleAndroidGmsAccountsettingsUiZeropartyentrypointactivity,
    AppEvent::ComGoogleAndroidGmsAccountsettingsUiSettingsloaderactivity,
    AppEvent::AndroidAction("com.android.settings.action.VIEW_ACCOUNT"),
    AppEvent::AndroidAction("com.google.android.gms.accountsettings.VIEW_SETTINGS_0P"),
    AppEvent::AndroidAction("com.google.android.gms.accountsettings.action.SAFETY_CENTER_SECURITY_CHECKUP"),
    AppEvent::AndroidAction("com.google.android.gms.accountsettings.action.BROWSE_SETTINGS"),
    AppEvent::AndroidAction("com.google.android.gms.accountsettings.action.VIEW_SETTINGS"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComGoogleAndroidGmsAccountsettingsUiZeropartyentrypointactivity => "com.google.android.gms.accountsettings.ui.ZeroPartyEntryPointActivity",
            AppEvent::ComGoogleAndroidGmsAccountsettingsUiSettingsloaderactivity => "com.google.android.gms.accountsettings.ui.SettingsLoaderActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
