use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.revolut.feature.app_launcher.impl.ui.LauncherActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    t1(state, capabilities);
    init(state, capabilities);
    j1(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.revolut.feature.app_launcher.impl.ui.LauncherActivity->onCreate
    // Static call sites: platform=0, internal=5
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn t1(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.revolut.feature.app_launcher.impl.ui.LauncherActivity->t1
    // Static call sites: platform=0, internal=2
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn init(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.revolut.feature.app_launcher.impl.ui.LauncherActivity-><init>
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn j1(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.revolut.feature.app_launcher.impl.ui.LauncherActivity->J1
    // Static call sites: platform=0, internal=1
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

