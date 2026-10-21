use std::net::SocketAddr;
use std::sync::Arc;

use corner_api::config::Config;
use corner_api::geocode::NominatimGeocoder;
use corner_api::{app, db, mail, AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| "corner_api=info,tower_http=info".into()),
        )
        .init();

    let config = Config::from_env();
    let pool = db::connect(&config.database_url).await?;
    db::migrate(&pool).await?;

    let mailer = mail::from_config(&config);
    let geocoder = Arc::new(NominatimGeocoder::new(&config.nominatim_url));
    let port = config.port;
    let state = AppState::new(pool, config, mailer, geocoder);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Corner API listening on http://{addr}");
    axum::serve(
        listener,
        app(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async {
        tokio::signal::ctrl_c().await.ok();
    })
    .await?;
    Ok(())
}
