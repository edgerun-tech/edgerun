#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComDeliveryheroConfigDashboardUiConfigdashboardactivity,
    ComDeliveryheroConfigDashboardUiExperimentationdashboardactivity,
    ComDeliveryheroPushServiceSpPushmessagingservice,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComDeliveryheroConfigDashboardUiConfigdashboardactivity,
    AppEvent::ComDeliveryheroConfigDashboardUiExperimentationdashboardactivity,
    AppEvent::ComDeliveryheroPushServiceSpPushmessagingservice,
    AppEvent::AndroidAction("com.deliveryhero.config.dashboard.CONFIG_DASHBOARD"),
    AppEvent::AndroidAction("com.deliveryhero.config.dashboard.EXPERIMENTATION"),
    AppEvent::AndroidAction("com.google.firebase.MESSAGING_EVENT"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComDeliveryheroConfigDashboardUiConfigdashboardactivity => "com.deliveryhero.config.dashboard.ui.ConfigDashboardActivity",
            AppEvent::ComDeliveryheroConfigDashboardUiExperimentationdashboardactivity => "com.deliveryhero.config.dashboard.ui.ExperimentationDashboardActivity",
            AppEvent::ComDeliveryheroPushServiceSpPushmessagingservice => "com.deliveryhero.push.service.sp.PushMessagingService",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
