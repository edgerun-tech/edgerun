use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.instagram.mainactivity.LauncherActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onresume(state, capabilities);
    onstart(state, capabilities);
    ondestroy(state, capabilities);
    onpause(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.instagram.mainactivity.LauncherActivity->onCreate
    // Static call sites: platform=28, internal=100
    // Signal: package_intents (12 call sites)
    // Signal: activity_ui (6 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onresume(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.instagram.mainactivity.LauncherActivity->onResume
    // Static call sites: platform=9, internal=32
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onstart(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.instagram.mainactivity.LauncherActivity->onStart
    // Static call sites: platform=9, internal=32
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn ondestroy(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.instagram.mainactivity.LauncherActivity->onDestroy
    // Static call sites: platform=9, internal=32
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onpause(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.instagram.mainactivity.LauncherActivity->onPause
    // Static call sites: platform=9, internal=32
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

