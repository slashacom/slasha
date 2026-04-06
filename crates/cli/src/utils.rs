use crate::config::Config;

pub fn build_git_remote_url(slug: &str) -> String {
    let config = Config::load().unwrap();
    let base_url = config
        .base_url
        .unwrap_or_else(|| "http://localhost:3000".into());
    format!("{}/git/{}", base_url, slug)
}