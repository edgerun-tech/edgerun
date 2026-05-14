use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_google_android_gms_contactkeys_mainactivity;
pub mod com_google_android_libraries_phenotype_registration_phenotypemetadataholderservice;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComGoogleAndroidGmsContactkeysMainactivity => com_google_android_gms_contactkeys_mainactivity::handle_event(event, state, capabilities),
        AppEvent::ComGoogleAndroidLibrariesPhenotypeRegistrationPhenotypemetadataholderservice => com_google_android_libraries_phenotype_registration_phenotypemetadataholderservice::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.INSERT" => com_google_android_gms_contactkeys_mainactivity::handle_event(event, state, capabilities),
            "android.intent.action.VIEW" => com_google_android_gms_contactkeys_mainactivity::handle_event(event, state, capabilities),
            "com.google.android.libraries.phenotype.registration.PhenotypeMetadataHolderService" => com_google_android_libraries_phenotype_registration_phenotypemetadataholderservice::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
