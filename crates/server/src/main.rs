use diesel::r2d2::{ConnectionManager, Pool};
use dotenv::dotenv;
use slasha_server::AppState;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let state = AppState {
        db_pool: Pool::builder()
            .build(ConnectionManager::<diesel::sqlite::SqliteConnection>::new(
                std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            ))
            .expect("Failed to create DB pool"),
        jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET must be set"),
    };

    slasha_server::run(Some("0.0.0.0:3000".parse().unwrap()), state).await?;

    Ok(())
}
