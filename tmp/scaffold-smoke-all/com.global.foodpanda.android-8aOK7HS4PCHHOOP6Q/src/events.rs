#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppEvent {
    ComDeliveryheroConfigDashboardUiConfigdashboardactivity,
    ComDeliveryheroConfigDashboardUiExperimentationdashboardactivity,
    AndroidAction(&'static str),
}

pub const SEED_EVENTS: &[AppEvent] = &[
    AppEvent::ComDeliveryheroConfigDashboardUiConfigdashboardactivity,
    AppEvent::ComDeliveryheroConfigDashboardUiExperimentationdashboardactivity,
    AppEvent::AndroidAction("com.deliveryhero.config.dashboard.CONFIG_DASHBOARD"),
    AppEvent::AndroidAction("com.deliveryhero.config.dashboard.EXPERIMENTATION"),
];

impl AppEvent {
    pub fn source(self) -> &'static str {
        match self {
            AppEvent::ComDeliveryheroConfigDashboardUiConfigdashboardactivity => "com.deliveryhero.config.dashboard.ui.ConfigDashboardActivity",
            AppEvent::ComDeliveryheroConfigDashboardUiExperimentationdashboardactivity => "com.deliveryhero.config.dashboard.ui.ExperimentationDashboardActivity",
            AppEvent::AndroidAction(action) => action,
        }
    }
}
