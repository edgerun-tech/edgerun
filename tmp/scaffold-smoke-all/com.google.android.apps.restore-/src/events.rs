#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComGoogleAndroidAppsRestoreBackupApiserviceBackupoptinapiendpointservice,
    ComGoogleAndroidAppsRestoreBackupExternalstorageApiserviceExternalstoragebackupapiendpointservice,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComGoogleAndroidAppsRestoreBackupApiserviceBackupoptinapiendpointservice,
    AppEvent::ComGoogleAndroidAppsRestoreBackupExternalstorageApiserviceExternalstoragebackupapiendpointservice,
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComGoogleAndroidAppsRestoreBackupApiserviceBackupoptinapiendpointservice => "com.google.android.apps.restore.backup.apiservice.BackupOptInApiEndpointService",
            AppEvent::ComGoogleAndroidAppsRestoreBackupExternalstorageApiserviceExternalstoragebackupapiendpointservice => "com.google.android.apps.restore.backup.externalstorage.apiservice.ExternalStorageBackupApiEndpointService",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
