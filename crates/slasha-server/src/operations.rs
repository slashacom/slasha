use std::sync::Arc;

use dashmap::{DashMap, mapref::entry::Entry};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ResourceKey {
    App(Arc<str>),
    Service(Arc<str>),
}

impl ResourceKey {
    /// Constructs a [`ResourceKey::App`] for an application ID string.
    ///
    /// # Arguments
    ///
    /// * `id` - Application ID string.
    ///
    /// # Returns
    ///
    /// A [`ResourceKey::App`] variant.
    pub fn app(id: impl AsRef<str>) -> Self {
        Self::App(Arc::from(id.as_ref()))
    }

    /// Constructs a [`ResourceKey::Service`] for a service ID string.
    ///
    /// # Arguments
    ///
    /// * `id` - Service ID string.
    ///
    /// # Returns
    ///
    /// A [`ResourceKey::Service`] variant.
    pub fn service(id: impl AsRef<str>) -> Self {
        Self::Service(Arc::from(id.as_ref()))
    }

    /// Returns the static string representation of the resource type.
    ///
    /// # Returns
    ///
    /// Resource type display name string.
    pub fn resource_string(&self) -> &'static str {
        match self {
            Self::App(_) => "App",
            Self::Service(_) => "Service",
        }
    }
}

#[derive(Clone)]
pub enum AppOperation {
    Deploying {
        deployment_id: String,
        cancel_token: CancellationToken,
    },
    Migrating,
    Scaling,
    Purging,
    RepoSyncing,
    Stopping,
    Restarting,
    Deleting,
}

#[derive(Clone)]
pub enum ServiceOperation {
    Provisioning,
    Stopping,
    Restarting,
    Deleting,
    BackingUp,
}

#[derive(Clone)]
pub enum ActiveOperation {
    App(AppOperation),
    Service(ServiceOperation),
}

impl ActiveOperation {
    /// Returns the static status string for the active operation.
    ///
    /// # Returns
    ///
    /// Operation status string.
    pub fn status_str(&self) -> &'static str {
        match self {
            Self::App(AppOperation::Deploying { .. }) => "deploying",
            Self::App(AppOperation::Migrating) => "migrating",
            Self::App(AppOperation::Scaling) => "scaling",
            Self::App(AppOperation::Purging) => "purging",
            Self::App(AppOperation::RepoSyncing) => "syncing",
            Self::App(AppOperation::Stopping) => "stopping",
            Self::App(AppOperation::Restarting) => "restarting",
            Self::App(AppOperation::Deleting) => "deleting",

            Self::Service(ServiceOperation::Provisioning) => "provisioning",
            Self::Service(ServiceOperation::Stopping) => "stopping",
            Self::Service(ServiceOperation::Restarting) => "restarting",
            Self::Service(ServiceOperation::Deleting) => "deleting",
            Self::Service(ServiceOperation::BackingUp) => "backing_up",
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum OperationError {
    #[error("{resource} is currently {status}")]
    Busy {
        resource: &'static str,
        status: &'static str,
    },
}

/// Registry tracking active app and service operations to enforce concurrency locks.
#[derive(Clone, Default)]
pub struct OperationRegistry {
    active: Arc<DashMap<ResourceKey, ActiveOperation>>,
}

impl OperationRegistry {
    /// Creates a new empty [`OperationRegistry`].
    ///
    /// # Returns
    ///
    /// A new [`OperationRegistry`] instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Attempts to acquire an operation lock for an application.
    ///
    /// # Arguments
    ///
    /// * `app_id` - Target application ID string.
    /// * `operation` - Application operation variant ([`AppOperation`]).
    ///
    /// # Returns
    ///
    /// A [`Result`] containing an [`OperationGuard`] or an [`OperationError::Busy`].
    pub fn try_acquire_app(
        &self,
        app_id: impl AsRef<str>,
        operation: AppOperation,
    ) -> Result<OperationGuard, OperationError> {
        self.try_acquire(ResourceKey::app(app_id), ActiveOperation::App(operation))
    }

    /// Attempts to acquire an operation lock for a service.
    ///
    /// # Arguments
    ///
    /// * `service_id` - Target service ID string.
    /// * `operation` - Service operation variant ([`ServiceOperation`]).
    ///
    /// # Returns
    ///
    /// A [`Result`] containing an [`OperationGuard`] or an [`OperationError::Busy`].
    pub fn try_acquire_service(
        &self,
        service_id: impl AsRef<str>,
        operation: ServiceOperation,
    ) -> Result<OperationGuard, OperationError> {
        self.try_acquire(
            ResourceKey::service(service_id),
            ActiveOperation::Service(operation),
        )
    }

    fn try_acquire(
        &self,
        key: ResourceKey,
        op: ActiveOperation,
    ) -> Result<OperationGuard, OperationError> {
        match self.active.entry(key.clone()) {
            Entry::Occupied(existing) => Err(OperationError::Busy {
                resource: key.resource_string(),
                status: existing.get().status_str(),
            }),
            Entry::Vacant(slot) => {
                slot.insert(op.clone());
                Ok(OperationGuard {
                    registry: self.active.clone(),
                    key,
                    op,
                })
            }
        }
    }

    /// Returns the current status string of a resource if it is busy.
    ///
    /// # Arguments
    ///
    /// * `key` - Target resource key ([`ResourceKey`]).
    ///
    /// # Returns
    ///
    /// Option containing the active operation status string.
    pub fn status_of(&self, key: &ResourceKey) -> Option<&'static str> {
        self.active.get(key).map(|op| op.status_str())
    }

    /// Returns a reference entry to the active operation for a resource key.
    ///
    /// # Arguments
    ///
    /// * `key` - Target resource key ([`ResourceKey`]).
    ///
    /// # Returns
    ///
    /// Option containing DashMap reference entry.
    pub fn get_operation(
        &self,
        key: &ResourceKey,
    ) -> Option<dashmap::mapref::one::Ref<'_, ResourceKey, ActiveOperation>> {
        self.active.get(key)
    }

    /// Ensures that a resource key has no active ongoing operations.
    ///
    /// # Arguments
    ///
    /// * `key` - Target resource key ([`ResourceKey`]).
    ///
    /// # Returns
    ///
    /// Result returning `()` if idle or [`OperationError::Busy`] if occupied.
    pub fn ensure_idle(&self, key: &ResourceKey) -> Result<(), OperationError> {
        if let Some(op) = self.active.get(key) {
            Err(OperationError::Busy {
                resource: key.resource_string(),
                status: op.value().status_str(),
            })
        } else {
            Ok(())
        }
    }
}

/// RAII guard releasing an operation lock from [`OperationRegistry`] when dropped.
pub struct OperationGuard {
    registry: Arc<DashMap<ResourceKey, ActiveOperation>>,
    key: ResourceKey,
    op: ActiveOperation,
}

impl OperationGuard {
    /// Returns a reference to the resource key protected by this guard.
    ///
    /// # Returns
    ///
    /// Resource key reference ([`ResourceKey`]).
    pub fn key(&self) -> &ResourceKey {
        &self.key
    }

    /// Returns a reference to the active operation protected by this guard.
    ///
    /// # Returns
    ///
    /// Active operation reference ([`ActiveOperation`]).
    pub fn operation(&self) -> &ActiveOperation {
        &self.op
    }
}

impl Drop for OperationGuard {
    fn drop(&mut self) {
        self.registry.remove(&self.key);
    }
}
