use diesel::r2d2::{ConnectionManager, Pool};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use dotenv::dotenv;
use slasha_server::{AppState, utils::ensure_dir};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

fn run_migrations(state: &AppState) -> anyhow::Result<()> {
    let mut conn = state
        .db_pool
        .get()
        .expect("Failed to get DB connection from pool");
    conn.run_pending_migrations(MIGRATIONS)
        .expect("Failed to run migrations");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let data_dir = dirs::home_dir()
        .expect("Failed to get home directory")
        .join(".slasha");

    let db_path = ensure_dir(&data_dir).join("slasha.db");
    let repos_dir = ensure_dir(&data_dir.join("repos"));

    let state = AppState {
        db_pool: Pool::builder()
            .build(ConnectionManager::<diesel::sqlite::SqliteConnection>::new(
                db_path.to_str().unwrap(),
            ))
            .expect("Failed to create DB pool"),
        jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
        repos_dir,
    };

    run_migrations(&state)?;

    slasha_server::run(Some("0.0.0.0:3000".parse().unwrap()), state).await?;

    Ok(())
}
