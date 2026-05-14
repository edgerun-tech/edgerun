use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_google_android_apps_restore_backup_apiservice_backupoptinapiendpointservice;
pub mod com_google_android_apps_restore_backup_externalstorage_apiservice_externalstoragebackupapiendpointservice;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComGoogleAndroidAppsRestoreBackupApiserviceBackupoptinapiendpointservice => com_google_android_apps_restore_backup_apiservice_backupoptinapiendpointservice::handle_event(event, state, capabilities),
        AppEvent::ComGoogleAndroidAppsRestoreBackupExternalstorageApiserviceExternalstoragebackupapiendpointservice => com_google_android_apps_restore_backup_externalstorage_apiservice_externalstoragebackupapiendpointservice::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            _ => {}
        },
    }
}
