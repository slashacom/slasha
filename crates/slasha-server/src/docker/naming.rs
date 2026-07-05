use sha2::{Digest, Sha256};

pub fn image_name(app_slug: &str) -> String {
    format!("slasha/{}", app_slug)
}

pub fn image_tag(app_slug: &str, deployment_id: &str) -> String {
    format!("slasha/{}:{}", app_slug, deployment_id)
}

pub fn process_container_name(
    app_id: &str,
    deployment_id: &str,
    process_type: &str,
    index: u32,
) -> String {
    format!(
        "slasha-{}-{}-{}-{}",
        app_id, deployment_id, process_type, index
    )
}

pub fn release_container_name(app_id: &str, deployment_id: &str) -> String {
    format!("slasha-{}-{}-release", app_id, deployment_id)
}

pub fn service_container_name(service_id: &str) -> String {
    format!("slasha-svc-{}", service_id)
}

pub fn app_volume_prefix(app_id: &str) -> String {
    format!("slasha-app-vol-{}-", app_id)
}

pub fn app_volume_name(app_id: &str, mount_path: &str) -> String {
    let digest = Sha256::digest(mount_path.as_bytes());
    let short: String = digest
        .iter()
        .take(4)
        .map(|b| format!("{:02x}", b))
        .collect();
    format!("{}{}", app_volume_prefix(app_id), short)
}

pub fn service_volume_name(service_id: &str) -> String {
    format!("slasha-vol-{}", service_id)
}

pub fn app_network_name(app_id: &str) -> String {
    format!("slasha-{}", app_id)
}
