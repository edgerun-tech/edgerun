mod capabilities;
mod network;
mod state;
mod third_party_modules;
mod workflows;

fn main() {
    let mut app_state = state::AppState::default();
    let capabilities = capabilities::Capabilities::default();
    third_party_modules::configure_defaults();
    workflows::org_fdroid_fdroid_panic_panicpreferencesactivity::register(&mut app_state, &capabilities);
    workflows::org_fdroid_fdroid_panic_panicresponderactivity::register(&mut app_state, &capabilities);
    workflows::org_fdroid_fdroid_panic_calculatoractivity::register(&mut app_state, &capabilities);
    workflows::org_fdroid_fdroid_views_repos_addrepoactivity::register(&mut app_state, &capabilities);
    workflows::org_fdroid_fdroid_views_appdetailsactivity::register(&mut app_state, &capabilities);
    workflows::org_fdroid_fdroid_views_main_mainactivity::register(&mut app_state, &capabilities);
    workflows::org_fdroid_fdroid_nearby_usbdeviceattachedreceiver::register(&mut app_state, &capabilities);
    workflows::org_fdroid_fdroid_nearby_usbdevicedetachedreceiver::register(&mut app_state, &capabilities);
}
