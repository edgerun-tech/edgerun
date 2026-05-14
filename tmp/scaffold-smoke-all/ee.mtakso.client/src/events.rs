#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    EeMtaksoClientNewbaseRidehailingmapactivity,
    EeMtaksoClientNewbaseVoipVoiptrampolineactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::EeMtaksoClientNewbaseRidehailingmapactivity,
    AppEvent::EeMtaksoClientNewbaseVoipVoiptrampolineactivity,
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::EeMtaksoClientNewbaseRidehailingmapactivity => "ee.mtakso.client.newbase.RideHailingMapActivity",
            AppEvent::EeMtaksoClientNewbaseVoipVoiptrampolineactivity => "ee.mtakso.client.newbase.voip.VoipTrampolineActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
