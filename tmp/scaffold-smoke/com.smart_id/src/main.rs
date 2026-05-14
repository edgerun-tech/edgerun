mod capabilities;
mod network;
mod state;
mod third_party_modules;
mod workflows;

fn main() {
    let mut app_state = state::AppState::default();
    let capabilities = capabilities::Capabilities::default();
    third_party_modules::configure_defaults();
    workflows::com_stagnationlab_sk_mainactivity::register(&mut app_state, &capabilities);
    workflows::io_flutter_plugins_firebase_messaging_flutterfirebasemessagingservice::register(&mut app_state, &capabilities);
    workflows::com_huawei_hms_flutter_push_hms_flutterhmsmessageservice::register(&mut app_state, &capabilities);
}
