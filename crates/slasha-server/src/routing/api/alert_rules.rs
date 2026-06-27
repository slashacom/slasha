use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, put},
};
use serde::Deserialize;
use serde_json::{Value, json};
use slasha_db::{
    DbPool,
    models::alert_rule::{AlertRule, AlertRuleInput},
    repos::alert_rule::AlertRuleRepo,
};

use crate::{AppState, error::HttpResult};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_rules).post(create_rule))
        .route("/{id}", put(update_rule).delete(delete_rule))
}

#[derive(Deserialize)]
struct RuleRequest {
    name: String,
    enabled: bool,
    target: String,
    event: String,
    params: Value,
    cooldown_secs: i32,
    action_type: String,
    action_config: Value,
}

impl From<RuleRequest> for AlertRuleInput {
    fn from(req: RuleRequest) -> Self {
        AlertRuleInput {
            name: req.name,
            enabled: req.enabled,
            target: req.target,
            event: req.event,
            params: req.params.to_string(),
            cooldown_secs: req.cooldown_secs,
            action_type: req.action_type,
            action_config: req.action_config.to_string(),
        }
    }
}

async fn list_rules(State(pool): State<DbPool>) -> HttpResult<Json<Vec<AlertRule>>> {
    let rules = AlertRuleRepo::list(&pool).await?;
    Ok(Json(rules))
}

async fn create_rule(
    State(pool): State<DbPool>,
    Json(payload): Json<RuleRequest>,
) -> HttpResult<Json<AlertRule>> {
    let rule = AlertRuleRepo::create(&pool, payload.into()).await?;
    Ok(Json(rule))
}

async fn update_rule(
    State(pool): State<DbPool>,
    Path(id): Path<String>,
    Json(payload): Json<RuleRequest>,
) -> HttpResult<Json<AlertRule>> {
    let rule = AlertRuleRepo::update(&pool, &id, payload.into()).await?;
    Ok(Json(rule))
}

async fn delete_rule(
    State(pool): State<DbPool>,
    Path(id): Path<String>,
) -> HttpResult<Json<Value>> {
    AlertRuleRepo::delete(&pool, &id).await?;
    Ok(Json(json!({ "status": "ok" })))
}
