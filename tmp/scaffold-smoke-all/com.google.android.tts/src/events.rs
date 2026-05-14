#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComGoogleAndroidLibrariesSpeechModelmanagerLanguagepackSettingsSettingsactivity,
    ComGoogleAndroidLibrariesSpeechModelmanagerLanguagepackSettingsAddlanguagesactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComGoogleAndroidLibrariesSpeechModelmanagerLanguagepackSettingsSettingsactivity,
    AppEvent::ComGoogleAndroidLibrariesSpeechModelmanagerLanguagepackSettingsAddlanguagesactivity,
    AppEvent::AndroidAction("com.google.android.libraries.speech.modelmanager.languagepack.settings.SettingsActivity"),
    AppEvent::AndroidAction("com.google.android.libraries.speech.modelmanager.languagepack.settings.AddLanguagesActivity"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComGoogleAndroidLibrariesSpeechModelmanagerLanguagepackSettingsSettingsactivity => "com.google.android.libraries.speech.modelmanager.languagepack.settings.SettingsActivity",
            AppEvent::ComGoogleAndroidLibrariesSpeechModelmanagerLanguagepackSettingsAddlanguagesactivity => "com.google.android.libraries.speech.modelmanager.languagepack.settings.AddLanguagesActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
