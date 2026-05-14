mod capabilities;
mod events;
mod network;
mod state;
mod third_party_modules;
mod workflows;

fn main() {
    let mut app_state = state::AppState::default();
    let capabilities = capabilities::Capabilities::default();
    third_party_modules::configure_defaults();
    for event in events::SEED_EVENTS {
        workflows::dispatch(*event, &mut app_state, &capabilities);
    }
}
