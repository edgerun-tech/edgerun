use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.ss.android.ugc.aweme.music.addtodsp.auth.RedirectUriReceiverActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onresume(state, capabilities);
    onstart(state, capabilities);
    settheme(state, capabilities);
    onstop(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.ss.android.ugc.aweme.music.addtodsp.auth.RedirectUriReceiverActivity->onCreate
    // Static call sites: platform=0, internal=6
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onresume(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.ss.android.ugc.aweme.music.addtodsp.auth.RedirectUriReceiverActivity->onResume
    // Static call sites: platform=0, internal=3
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onstart(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.ss.android.ugc.aweme.music.addtodsp.auth.RedirectUriReceiverActivity->onStart
    // Static call sites: platform=0, internal=3
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn settheme(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.ss.android.ugc.aweme.music.addtodsp.auth.RedirectUriReceiverActivity->setTheme
    // Static call sites: platform=12, internal=10
    // Signal: activity_ui (2 call sites)
    // Signal: package_intents (2 call sites)
    // TODO: replace Android Intent behavior with typed Edgerun events
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onstop(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.ss.android.ugc.aweme.music.addtodsp.auth.RedirectUriReceiverActivity->onStop
    // Static call sites: platform=8, internal=2
    // Signal: activity_ui (8 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

