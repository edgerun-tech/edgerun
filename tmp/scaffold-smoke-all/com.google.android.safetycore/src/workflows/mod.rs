use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_google_android_libraries_phenotype_registration_phenotypemetadataholderservice;
pub mod com_google_android_apps_safetycore_service_classificationapiservice;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComGoogleAndroidLibrariesPhenotypeRegistrationPhenotypemetadataholderservice => com_google_android_libraries_phenotype_registration_phenotypemetadataholderservice::handle_event(event, state, capabilities),
        AppEvent::ComGoogleAndroidAppsSafetycoreServiceClassificationapiservice => com_google_android_apps_safetycore_service_classificationapiservice::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "com.google.android.libraries.phenotype.registration.PhenotypeMetadataHolderService" => com_google_android_libraries_phenotype_registration_phenotypemetadataholderservice::handle_event(event, state, capabilities),
            "com.google.android.apps.safetycore.classification.BIND" => com_google_android_apps_safetycore_service_classificationapiservice::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
