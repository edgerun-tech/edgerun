use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.google.android.apps.restore.backup.externalstorage.apiservice.ExternalStorageBackupApiEndpointService";
pub const KIND: &str = "service";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    onbind(state, capabilities);
    oncreate(state, capabilities);
    ondestroy(state, capabilities);
    init(state, capabilities);
}

fn onbind(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.restore.backup.externalstorage.apiservice.ExternalStorageBackupApiEndpointService->onBind
    // Static call sites: platform=5, internal=16
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.restore.backup.externalstorage.apiservice.ExternalStorageBackupApiEndpointService->onCreate
    // Static call sites: platform=5, internal=15
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn ondestroy(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.restore.backup.externalstorage.apiservice.ExternalStorageBackupApiEndpointService->onDestroy
    // Static call sites: platform=2, internal=5
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.restore.backup.externalstorage.apiservice.ExternalStorageBackupApiEndpointService-><init>
    // Static call sites: platform=0, internal=3
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

