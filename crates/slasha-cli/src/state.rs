use crate::{http::ApiClient, output::OutputMode};

pub struct AppState {
    pub api_client: ApiClient,
    pub output_mode: OutputMode,
}
