mod config;
mod db;
mod routes;
mod state;

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = config::Config::from_env()?;
    let pool = db::connect(&config.database_url, config.database_max_connections).await?;

    db::run_migrations(&pool).await?;

    let state = AppState { pool };

    let app = routes::router(state);

    let listener = tokio::net::TcpListener::bind(config.server_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
