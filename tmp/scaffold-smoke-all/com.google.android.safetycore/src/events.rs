#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComGoogleAndroidLibrariesPhenotypeRegistrationPhenotypemetadataholderservice,
    ComGoogleAndroidAppsSafetycoreServiceClassificationapiservice,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComGoogleAndroidLibrariesPhenotypeRegistrationPhenotypemetadataholderservice,
    AppEvent::ComGoogleAndroidAppsSafetycoreServiceClassificationapiservice,
    AppEvent::AndroidAction("com.google.android.libraries.phenotype.registration.PhenotypeMetadataHolderService"),
    AppEvent::AndroidAction("com.google.android.apps.safetycore.classification.BIND"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComGoogleAndroidLibrariesPhenotypeRegistrationPhenotypemetadataholderservice => "com.google.android.libraries.phenotype.registration.PhenotypeMetadataHolderService",
            AppEvent::ComGoogleAndroidAppsSafetycoreServiceClassificationapiservice => "com.google.android.apps.safetycore.service.ClassificationApiService",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
