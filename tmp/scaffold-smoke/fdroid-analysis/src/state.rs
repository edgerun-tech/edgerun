#[derive(Debug, Default)]
pub struct AppState {
    pub started_workflows: Vec<&'static str>,
}
