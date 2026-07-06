use chrono::NaiveDateTime;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = crate::models::schema::github_app_config)]
pub struct GithubAppConfig {
    pub id: String,
    pub app_id: String,
    pub client_id: String,
    pub client_secret: String,
    pub private_key: String,
    pub webhook_secret: String,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

pub struct NewGithubAppConfig {
    pub app_id: String,
    pub client_id: String,
    pub client_secret: String,
    pub private_key: String,
    pub webhook_secret: String,
}

#[derive(AsChangeset)]
#[diesel(table_name = crate::models::schema::github_app_config)]
pub struct GithubAppConfigChangeset {
    pub app_id: String,
    pub client_id: String,
    pub client_secret: String,
    pub private_key: String,
    pub webhook_secret: String,
    pub updated_at: NaiveDateTime,
}
