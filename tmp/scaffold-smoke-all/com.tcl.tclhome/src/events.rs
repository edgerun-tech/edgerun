#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComTclBmmainSplashactivity,
    ComTclBmloginuiUiAppflipThappfliptclhomeactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComTclBmmainSplashactivity,
    AppEvent::ComTclBmloginuiUiAppflipThappfliptclhomeactivity,
    AppEvent::AndroidAction("android.intent.action.MAIN"),
    AppEvent::AndroidAction("android.intent.action.VIEW"),
    AppEvent::AndroidAction("com.tcl.tclhome.appflip.obg.iot"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComTclBmmainSplashactivity => "com.tcl.bmmain.SplashActivity",
            AppEvent::ComTclBmloginuiUiAppflipThappfliptclhomeactivity => "com.tcl.bmloginui.ui.appflip.THAppFlipTCLHomeActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
