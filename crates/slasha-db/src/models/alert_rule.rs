use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Queryable, Selectable, Insertable, Debug, Clone, Serialize, Deserialize, TS)]
#[diesel(table_name = crate::models::schema::alert_rules)]
#[ts(export, export_to = "./alert_rule.ts")]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub target: String,
    pub event: String,
    pub params: String,
    pub cooldown_secs: i32,
    pub action_type: String,
    pub action_config: String,
    pub last_fired_at: Option<chrono::NaiveDateTime>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export, export_to = "./alert_rule_input.ts")]
pub struct AlertRuleInput {
    pub name: String,
    pub enabled: bool,
    pub target: String,
    pub event: String,
    pub params: String,
    pub cooldown_secs: i32,
    pub action_type: String,
    pub action_config: String,
}
