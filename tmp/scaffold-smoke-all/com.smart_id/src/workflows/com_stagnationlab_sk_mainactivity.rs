use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.stagnationlab.sk.MainActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onnewintent(state, capabilities);
    onactivityresult(state, capabilities);
    onresume(state, capabilities);
    onstart(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.stagnationlab.sk.MainActivity->onCreate
    // Static call sites: platform=819, internal=285
    // Signal: activity_ui (116 call sites)
    // Signal: package_intents (84 call sites)
    // Signal: work_background (32 call sites)
    // Signal: camera_media_capture (11 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    state.capability_intents.push(capabilities.background_tasks.request("schedule_or_handle_background_work", SOURCE_CLASS));
    state.capability_intents.push(capabilities.camera_media.request("capture_or_process_media", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: request location only when this workflow runs
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
    // TODO: wrap camera/media access in explicit user sessions
}

fn onnewintent(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.stagnationlab.sk.MainActivity->onNewIntent
    // Static call sites: platform=8, internal=16
    // Signal: package_intents (4 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onactivityresult(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.stagnationlab.sk.MainActivity->onActivityResult
    // Static call sites: platform=2, internal=5
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onresume(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.stagnationlab.sk.MainActivity->onResume
    // Static call sites: platform=0, internal=5
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onstart(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.stagnationlab.sk.MainActivity->onStart
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

