use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.onesignal.NotificationOpenedActivityHMS";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onnewintent(state, capabilities);
    a(state, capabilities);
    init(state, capabilities);
    b(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.onesignal.NotificationOpenedActivityHMS->onCreate
    // Static call sites: platform=1, internal=1
    // Signal: activity_ui (1 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onnewintent(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.onesignal.NotificationOpenedActivityHMS->onNewIntent
    // Static call sites: platform=1, internal=1
    // Signal: activity_ui (1 call sites)
    // Signal: package_intents (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn a(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.onesignal.NotificationOpenedActivityHMS->a
    // Static call sites: platform=2, internal=1
    // Signal: activity_ui (2 call sites)
    // Signal: package_intents (1 call sites)
    state.capability_intents.push(capabilities.app_events.request("send_or_receive_app_event", SOURCE_CLASS));
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.onesignal.NotificationOpenedActivityHMS-><init>
    // Static call sites: platform=1, internal=0
    // Signal: activity_ui (1 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn b(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.onesignal.NotificationOpenedActivityHMS->b
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

