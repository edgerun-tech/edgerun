use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_tcl_bmmain_splashactivity;
pub mod com_tcl_bmloginui_ui_appflip_thappfliptclhomeactivity;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComTclBmmainSplashactivity => com_tcl_bmmain_splashactivity::handle_event(event, state, capabilities),
        AppEvent::ComTclBmloginuiUiAppflipThappfliptclhomeactivity => com_tcl_bmloginui_ui_appflip_thappfliptclhomeactivity::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "android.intent.action.MAIN" => com_tcl_bmmain_splashactivity::handle_event(event, state, capabilities),
            "android.intent.action.VIEW" => com_tcl_bmmain_splashactivity::handle_event(event, state, capabilities),
            "com.tcl.tclhome.appflip.obg.iot" => com_tcl_bmloginui_ui_appflip_thappfliptclhomeactivity::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
