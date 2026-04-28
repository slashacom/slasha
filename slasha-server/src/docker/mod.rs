pub mod build;
pub mod env;
pub mod logs;
pub mod network;
pub mod pipeline;
pub mod port_pool;
pub mod run;
pub mod services;

use crate::error::DeploymentError;
pub type DeploymentResult<T> = std::result::Result<T, DeploymentError>;
