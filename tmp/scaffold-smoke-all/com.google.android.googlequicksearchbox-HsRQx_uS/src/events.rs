#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComGoogleAndroidGooglequicksearchboxSearchwidgetprovider,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComGoogleAndroidGooglequicksearchboxSearchwidgetprovider,
    AppEvent::AndroidAction("android.appwidget.action.APPWIDGET_UPDATE"),
    AppEvent::AndroidAction("android.appwidget.action.APPWIDGET_UPDATE_OPTIONS"),
    AppEvent::AndroidAction("com.google.android.finsky.intent.action.UPDATE_DSE"),
    AppEvent::AndroidAction("com.google.android.finsky.intent.action.UPDATE_DSE_APP_STATE"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComGoogleAndroidGooglequicksearchboxSearchwidgetprovider => "com.google.android.googlequicksearchbox.SearchWidgetProvider",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
