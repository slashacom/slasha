use crate::http::ApiClient;

/// Command execution context for the Slasha CLI.
pub struct Context {
    /// HTTP API client configured for the target Slasha instance.
    pub api_client: ApiClient,
}
