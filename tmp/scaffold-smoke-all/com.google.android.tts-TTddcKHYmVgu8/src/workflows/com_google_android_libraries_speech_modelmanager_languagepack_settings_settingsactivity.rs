use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub const SOURCE_CLASS: &str = "com.google.android.libraries.speech.modelmanager.languagepack.settings.SettingsActivity";
pub const KIND: &str = "activity";

pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    state.handled_events.push(event.source());
    state.started_workflows.push(SOURCE_CLASS);
    oncreate(state, capabilities);
    onactivityresult(state, capabilities);
    onnewintent(state, capabilities);
    onrequestpermissionsresult(state, capabilities);
    onresume(state, capabilities);
}

fn oncreate(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.libraries.speech.modelmanager.languagepack.settings.SettingsActivity->onCreate
    // Static call sites: platform=6, internal=44
    // Signal: activity_ui (1 call sites)
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onactivityresult(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.libraries.speech.modelmanager.languagepack.settings.SettingsActivity->onActivityResult
    // Static call sites: platform=1, internal=4
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onnewintent(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.libraries.speech.modelmanager.languagepack.settings.SettingsActivity->onNewIntent
    // Static call sites: platform=1, internal=4
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onrequestpermissionsresult(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.libraries.speech.modelmanager.languagepack.settings.SettingsActivity->onRequestPermissionsResult
    // Static call sites: platform=1, internal=4
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

fn onresume(state: &mut AppState, capabilities: &Capabilities) {
    let _ = state;
    let _ = capabilities;
    // Source method: com.google.android.libraries.speech.modelmanager.languagepack.settings.SettingsActivity->onResume
    // Static call sites: platform=1, internal=4
    // TODO: translate this lifecycle/body method into explicit Edgerun control flow
}

