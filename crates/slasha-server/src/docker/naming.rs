use sha2::{Digest, Sha256};
use slasha_db::models::app_scale::ProcessType;

/// Generates the container name for an application process replica.
///
/// # Arguments
///
/// * `app_id` - Application ID string.
/// * `deployment_id` - Deployment ID string.
/// * `process_type` - Process type enum ([`ProcessType`]).
/// * `index` - Replica instance index.
///
/// # Returns
///
/// Formatted container name string.
pub fn process_container_name(
    app_id: &str,
    deployment_id: &str,
    process_type: &ProcessType,
    index: u32,
) -> String {
    format!(
        "slasha-app-ctr-{}-{}-{}-{}",
        app_id,
        deployment_id,
        process_type.to_string().to_lowercase(),
        index
    )
}

/// Generates the container name for a database service.
///
/// # Arguments
///
/// * `service_id` - Service ID string.
///
/// # Returns
///
/// Formatted container name string.
pub fn service_container_name(service_id: &str) -> String {
    format!("slasha-svc-ctr-{}", service_id)
}

/// Generates the volume prefix for an application's persistent volumes.
///
/// # Arguments
///
/// * `app_id` - Application ID string.
///
/// # Returns
///
/// Formatted volume prefix string.
pub fn app_volume_prefix(app_id: &str) -> String {
    format!("slasha-app-vol-{}-", app_id)
}

/// Generates the unique volume name for an application's mounted path.
///
/// # Arguments
///
/// * `app_id` - Application ID string.
/// * `mount_path` - Target container mount path.
///
/// # Returns
///
/// Formatted volume name string.
pub fn app_volume_name(app_id: &str, mount_path: &str) -> String {
    let digest = Sha256::digest(mount_path.as_bytes());
    let short: String = digest
        .iter()
        .take(4)
        .map(|b| format!("{:02x}", b))
        .collect();
    format!("{}{}", app_volume_prefix(app_id), short)
}

/// Generates the volume name for a database service.
///
/// # Arguments
///
/// * `service_id` - Service ID string.
///
/// # Returns
///
/// Formatted volume name string.
pub fn service_volume_name(service_id: &str) -> String {
    format!("slasha-svc-vol-{}", service_id)
}

/// Generates the bridge network name for an application.
///
/// # Arguments
///
/// * `app_id` - Application ID string.
///
/// # Returns
///
/// Formatted network name string.
pub fn app_network_name(app_id: &str) -> String {
    format!("slasha-app-net-{}", app_id)
}

/// Generates the container name for an ephemeral cron job run.
///
/// # Arguments
///
/// * `run_id` - Cron run ID string.
///
/// # Returns
///
/// Formatted container name string.
pub fn cron_container_name(run_id: &str) -> String {
    format!("slasha-cron-{}", run_id)
}
