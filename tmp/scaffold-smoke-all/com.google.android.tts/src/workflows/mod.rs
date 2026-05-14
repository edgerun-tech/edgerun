use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_google_android_libraries_speech_modelmanager_languagepack_settings_settingsactivity;
pub mod com_google_android_libraries_speech_modelmanager_languagepack_settings_addlanguagesactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComGoogleAndroidLibrariesSpeechModelmanagerLanguagepackSettingsSettingsactivity => com_google_android_libraries_speech_modelmanager_languagepack_settings_settingsactivity::handle_event(event, state, capabilities),
        AppEvent::ComGoogleAndroidLibrariesSpeechModelmanagerLanguagepackSettingsAddlanguagesactivity => com_google_android_libraries_speech_modelmanager_languagepack_settings_addlanguagesactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "com.google.android.libraries.speech.modelmanager.languagepack.settings.SettingsActivity" => com_google_android_libraries_speech_modelmanager_languagepack_settings_settingsactivity::handle_event(event, state, capabilities),
            "com.google.android.libraries.speech.modelmanager.languagepack.settings.AddLanguagesActivity" => com_google_android_libraries_speech_modelmanager_languagepack_settings_addlanguagesactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
