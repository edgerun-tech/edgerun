use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.wise.notifications.presentation.preferences.NotificationPreferencesActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    h1(state, capabilities);
    j1(state, capabilities);
    f1(state, capabilities);
    g1(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.wise.notifications.presentation.preferences.NotificationPreferencesActivity->onCreate
    // Static call sites: platform=0, internal=6
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn h1(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.wise.notifications.presentation.preferences.NotificationPreferencesActivity->h1
    // Static call sites: platform=0, internal=25
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn j1(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.wise.notifications.presentation.preferences.NotificationPreferencesActivity->j1
    // Static call sites: platform=0, internal=20
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn f1(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.wise.notifications.presentation.preferences.NotificationPreferencesActivity->f1
    // Static call sites: platform=0, internal=14
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn g1(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.wise.notifications.presentation.preferences.NotificationPreferencesActivity->g1
    // Static call sites: platform=0, internal=13
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

