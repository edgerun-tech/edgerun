mod capabilities;
mod network;
mod state;
mod third_party_modules;
mod workflows;

fn main() {
    let mut app_state = state::AppState::default();
    let capabilities = capabilities::Capabilities::default();
    third_party_modules::configure_defaults();
    workflows::com_deliveryhero_config_dashboard_ui_configdashboardactivity::register(&mut app_state, &capabilities);
    workflows::com_deliveryhero_config_dashboard_ui_experimentationdashboardactivity::register(&mut app_state, &capabilities);
    workflows::com_deliveryhero_push_service_sp_pushmessagingservice::register(&mut app_state, &capabilities);
}
