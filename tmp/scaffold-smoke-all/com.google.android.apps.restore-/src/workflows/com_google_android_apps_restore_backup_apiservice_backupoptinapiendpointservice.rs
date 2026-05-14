use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.google.android.apps.restore.backup.apiservice.BackupOptInApiEndpointService";
pub const KIND: &str = "service";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onbind(state, capabilities);
    ondestroy(state, capabilities);
    init(state, capabilities);
    a(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.restore.backup.apiservice.BackupOptInApiEndpointService->onCreate
    // Static call sites: platform=6, internal=21
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onbind(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.restore.backup.apiservice.BackupOptInApiEndpointService->onBind
    // Static call sites: platform=2, internal=7
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn ondestroy(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.restore.backup.apiservice.BackupOptInApiEndpointService->onDestroy
    // Static call sites: platform=2, internal=5
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.restore.backup.apiservice.BackupOptInApiEndpointService-><init>
    // Static call sites: platform=0, internal=3
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn a(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.restore.backup.apiservice.BackupOptInApiEndpointService->a
    // Static call sites: platform=2, internal=0
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

