use diesel::prelude::*;

use crate::{
    connection::DbPool,
    error::{DbError, DbResult},
    models::{
        node::{LOCAL_NODE_ID, NewNode, Node, NodeChangeset, NodeStatus},
        schema::nodes,
    },
    schema::deployments,
};

pub struct NodeRepo;

impl NodeRepo {
    pub async fn get(pool: &DbPool, id: &str) -> DbResult<Node> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            nodes::table
                .filter(nodes::id.eq(&id))
                .filter(nodes::deleted_at.is_null())
                .first::<Node>(&mut conn)
                .optional()?
                .ok_or_else(|| DbError::NotFound(format!("node '{}' not found", id)))
        })
        .await?
    }

    pub async fn list(pool: &DbPool) -> DbResult<Vec<Node>> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let results = nodes::table
                .filter(nodes::deleted_at.is_null())
                .order(nodes::created_at.desc())
                .load::<Node>(&mut conn)?;
            Ok(results)
        })
        .await?
    }

    pub async fn create(pool: &DbPool, node: NewNode) -> DbResult<Node> {
        let pool = pool.clone();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let inserted = diesel::insert_into(nodes::table)
                .values(&node)
                .returning(Node::as_returning())
                .get_result(&mut conn)?;

            Ok(inserted)
        })
        .await?
    }

    pub async fn set_status(pool: &DbPool, id: &str, status: NodeStatus) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();

        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(nodes::table.filter(nodes::id.eq(&id)))
                .set(nodes::status.eq(status))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn set_status_and_ca(
        pool: &DbPool,
        id: &str,
        status: NodeStatus,
        internal_root_ca: Option<String>,
    ) -> DbResult<()> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            diesel::update(nodes::table.filter(nodes::id.eq(&id)))
                .set((
                    nodes::status.eq(status),
                    nodes::internal_root_ca.eq(internal_root_ca),
                ))
                .execute(&mut conn)?;
            Ok(())
        })
        .await?
    }

    pub async fn update(pool: &DbPool, id: &str, changeset: NodeChangeset) -> DbResult<Node> {
        let pool = pool.clone();
        let id = id.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;

            diesel::update(nodes::table.filter(nodes::id.eq(&id)))
                .set((
                    &changeset,
                    nodes::updated_at.eq(chrono::Utc::now().naive_utc()),
                ))
                .returning(Node::as_returning())
                .get_result(&mut conn)
                .map_err(Into::into)
        })
        .await?
    }

    pub async fn delete(pool: &DbPool, id_to_delete: &str) -> DbResult<()> {
        let pool = pool.clone();
        let id_to_delete = id_to_delete.to_string();
        tokio::task::spawn_blocking(move || {
            if id_to_delete == LOCAL_NODE_ID {
                return Err(DbError::Data("Cannot delete the local node".into()));
            }

            let mut conn = pool.get()?;

            let count: i64 = deployments::table
                .filter(deployments::node_id.eq(&id_to_delete))
                .count()
                .get_result(&mut conn)?;

            if count > 0 {
                diesel::update(nodes::table.filter(nodes::id.eq(&id_to_delete)))
                    .set(nodes::deleted_at.eq(chrono::Utc::now().naive_utc()))
                    .execute(&mut conn)?;
            } else {
                diesel::delete(nodes::table.filter(nodes::id.eq(&id_to_delete)))
                    .execute(&mut conn)?;
            }

            Ok(())
        })
        .await?
    }

    pub async fn has_apps(pool: &DbPool, node_id_to_check: &str) -> DbResult<bool> {
        use crate::models::schema::apps;
        let pool = pool.clone();
        let node_id_to_check = node_id_to_check.to_string();
        tokio::task::spawn_blocking(move || {
            let mut conn = pool.get()?;
            let count: i64 = apps::table
                .filter(apps::node_id.eq(node_id_to_check))
                .count()
                .get_result(&mut conn)?;

            Ok(count > 0)
        })
        .await?
    }
}
