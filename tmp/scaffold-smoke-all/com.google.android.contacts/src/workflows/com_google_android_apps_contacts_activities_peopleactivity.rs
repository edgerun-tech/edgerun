use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.google.android.apps.contacts.activities.PeopleActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onactivityresult(state, capabilities);
    onnewintent(state, capabilities);
    onresume(state, capabilities);
    onstart(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.contacts.activities.PeopleActivity->onCreate
    // Static call sites: platform=0, internal=4
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onactivityresult(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.contacts.activities.PeopleActivity->onActivityResult
    // Static call sites: platform=0, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onnewintent(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.contacts.activities.PeopleActivity->onNewIntent
    // Static call sites: platform=0, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onresume(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.contacts.activities.PeopleActivity->onResume
    // Static call sites: platform=0, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onstart(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.apps.contacts.activities.PeopleActivity->onStart
    // Static call sites: platform=0, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

