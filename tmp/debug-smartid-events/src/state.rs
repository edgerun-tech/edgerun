#[derive(Debug, Default)]
pub struct AppState {
    pub started_workflows: Vec<&'static str>,
    pub handled_events: Vec<&'static str>,
}
