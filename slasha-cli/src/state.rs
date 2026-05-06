use crate::{http::ApiClient, output::OutputMode};

pub struct AppState {
    pub client: ApiClient,
    pub output_mode: OutputMode,
}
