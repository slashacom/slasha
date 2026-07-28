use std::collections::HashMap;

use slasha_db::{app::App, deployment::Deployment, models::app_scale::ProcessType};

/// Docker label denoting that a resource is managed by Slasha.
pub const LABEL_MANAGED: &str = "slasha.managed";

/// Docker label identifying the associated application ID.
pub const LABEL_APP_ID: &str = "slasha.app_id";

/// Docker label identifying the application slug.
pub const LABEL_APP_SLUG: &str = "slasha.app_slug";

/// Docker label identifying the deployment ID.
pub const LABEL_DEPLOYMENT_ID: &str = "slasha.deployment_id";

/// Docker label identifying the service ID.
pub const LABEL_SERVICE_ID: &str = "slasha.service_id";

/// Docker label identifying the process type (e.g. `web`, `worker`).
pub const LABEL_PROCESS_TYPE: &str = "slasha.process_type";

/// Docker label identifying the 0-indexed process instance number.
pub const LABEL_INSTANCE_INDEX: &str = "slasha.instance_index";

/// Docker label identifying the exposed internal container port.
pub const LABEL_CONTAINER_PORT: &str = "slasha.container_port";

/// Docker label identifying the infrastructure role (e.g. `proxy`).
pub const LABEL_ROLE: &str = "slasha.role";

/// Docker label identifying the cron job ID.
pub const LABEL_CRON_JOB_ID: &str = "slasha.cron_job_id";

/// Docker label identifying the cron run ID.
pub const LABEL_CRON_RUN_ID: &str = "slasha.cron_run_id";

/// Constructs the map of Docker labels attached to process containers.
///
/// # Arguments
///
/// * `app` - Target application model ([`App`]).
/// * `deployment` - Target deployment model ([`Deployment`]).
/// * `process_type` - Process type enum ([`ProcessType`]).
/// * `instance_index` - Replica instance index reference.
/// * `container_port` - Optional internal HTTP exposure port.
///
/// # Returns
///
/// A [`HashMap`] of label key-value strings.
pub fn process_container_labels(
    app: &App,
    deployment: &Deployment,
    process_type: &ProcessType,
    instance_index: &u32,
    container_port: Option<u16>,
) -> HashMap<String, String> {
    let mut labels = HashMap::new();
    labels.insert(LABEL_MANAGED.to_string(), "true".to_string());
    labels.insert(LABEL_APP_ID.to_string(), app.id.clone());
    labels.insert(LABEL_DEPLOYMENT_ID.to_string(), deployment.id.clone());
    labels.insert(LABEL_APP_SLUG.to_string(), app.slug.clone());

    if let Some(container_port) = container_port {
        labels.insert(LABEL_CONTAINER_PORT.to_string(), container_port.to_string());
    }

    labels.insert(LABEL_PROCESS_TYPE.to_string(), process_type.to_string());
    labels.insert(LABEL_INSTANCE_INDEX.to_string(), instance_index.to_string());

    labels
}

/// Constructs the map of Docker labels attached to service containers.
///
/// # Arguments
///
/// * `app` - Target application model ([`App`]).
/// * `service_id` - Database service ID string.
///
/// # Returns
///
/// A [`HashMap`] of label key-value strings.
pub fn service_container_labels(app: &App, service_id: &str) -> HashMap<String, String> {
    let mut labels = HashMap::new();
    labels.insert(LABEL_MANAGED.to_string(), "true".to_string());
    labels.insert(LABEL_APP_ID.to_string(), app.id.clone());
    labels.insert(LABEL_SERVICE_ID.to_string(), service_id.to_string());

    labels
}

/// Constructs the map of Docker labels attached to cron job containers.
///
/// # Arguments
///
/// * `app` - Target application model ([`App`]).
/// * `cron_job_id` - Cron job ID string.
/// * `cron_run_id` - Cron run ID string.
///
/// # Returns
///
/// A [`HashMap`] of label key-value strings.
pub fn cron_container_labels(
    app: &App,
    cron_job_id: &str,
    cron_run_id: &str,
) -> HashMap<String, String> {
    let mut labels = HashMap::new();
    labels.insert(LABEL_MANAGED.to_string(), "true".to_string());
    labels.insert(LABEL_APP_ID.to_string(), app.id.clone());
    labels.insert(LABEL_APP_SLUG.to_string(), app.slug.clone());
    labels.insert(LABEL_CRON_JOB_ID.to_string(), cron_job_id.to_string());
    labels.insert(LABEL_CRON_RUN_ID.to_string(), cron_run_id.to_string());
    labels.insert(LABEL_PROCESS_TYPE.to_string(), "cron".to_string());

    labels
}
