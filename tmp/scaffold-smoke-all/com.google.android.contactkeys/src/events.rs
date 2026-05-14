#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComGoogleAndroidGmsContactkeysMainactivity,
    ComGoogleAndroidLibrariesPhenotypeRegistrationPhenotypemetadataholderservice,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComGoogleAndroidGmsContactkeysMainactivity,
    AppEvent::ComGoogleAndroidLibrariesPhenotypeRegistrationPhenotypemetadataholderservice,
    AppEvent::AndroidAction("android.intent.action.INSERT"),
    AppEvent::AndroidAction("android.intent.action.VIEW"),
    AppEvent::AndroidAction("com.google.android.libraries.phenotype.registration.PhenotypeMetadataHolderService"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComGoogleAndroidGmsContactkeysMainactivity => "com.google.android.gms.contactkeys.MainActivity",
            AppEvent::ComGoogleAndroidLibrariesPhenotypeRegistrationPhenotypemetadataholderservice => "com.google.android.libraries.phenotype.registration.PhenotypeMetadataHolderService",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
