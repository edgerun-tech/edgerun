mod capabilities;
mod network;
mod state;
mod third_party_modules;
mod workflows;

fn main() {
    let mut app_state = state::AppState::default();
    let capabilities = capabilities::Capabilities::default();
    third_party_modules::configure_defaults();
    workflows::com_thingclips_social_amazon_activity_triplealexaaccountlinkactivity::register(&mut app_state, &capabilities);
    workflows::com_thingclips_social_amazon_activity_alexaauthactivity::register(&mut app_state, &capabilities);
    workflows::com_thingclips_smart_speech_activity_assisantmainactivity::register(&mut app_state, &capabilities);
}
