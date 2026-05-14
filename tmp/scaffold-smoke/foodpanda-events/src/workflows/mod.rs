use crate::capabilities::Capabilities;
use crate::events::AppEvent;
use crate::state::AppState;

pub mod com_deliveryhero_config_dashboard_ui_configdashboardactivity;
pub mod com_deliveryhero_config_dashboard_ui_experimentationdashboardactivity;
pub mod com_deliveryhero_push_service_sp_pushmessagingservice;

pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {
    match event {
        AppEvent::ComDeliveryheroConfigDashboardUiConfigdashboardactivity => com_deliveryhero_config_dashboard_ui_configdashboardactivity::handle_event(event, state, capabilities),
        AppEvent::ComDeliveryheroConfigDashboardUiExperimentationdashboardactivity => com_deliveryhero_config_dashboard_ui_experimentationdashboardactivity::handle_event(event, state, capabilities),
        AppEvent::ComDeliveryheroPushServiceSpPushmessagingservice => com_deliveryhero_push_service_sp_pushmessagingservice::handle_event(event, state, capabilities),
        AppEvent::AndroidAction(action) => match action {
            "com.deliveryhero.config.dashboard.CONFIG_DASHBOARD" => com_deliveryhero_config_dashboard_ui_configdashboardactivity::handle_event(event, state, capabilities),
            "com.deliveryhero.config.dashboard.EXPERIMENTATION" => com_deliveryhero_config_dashboard_ui_experimentationdashboardactivity::handle_event(event, state, capabilities),
            "com.google.firebase.MESSAGING_EVENT" => com_deliveryhero_push_service_sp_pushmessagingservice::handle_event(event, state, capabilities),
            _ => {}
        },
    }
}
