use std::sync::Arc;
use std::time::Duration;

use oli_demo_rust_app::market::{MarketConfig, MarketData};
use oli_demo_rust_app::snapshot::embedded;
use oli_demo_rust_app::tips::TipSource;
use oli_demo_rust_app::web::{router, AppState};
use oli_demo_rust_app::yahoo::YahooClient;

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port = env_or("PORT", "3000");
    let drupal = env_or("DRUPAL_BASE_URL", "http://localhost");
    let public = env_or("DRUPAL_PUBLIC_URL", &env_or("DDEV_PRIMARY_URL", &drupal));

    let state = AppState {
        market: Arc::new(MarketData::new(
            YahooClient::new(),
            embedded(),
            MarketConfig::default(),
        )),
        tips: Arc::new(TipSource::new(
            drupal.clone(),
            public,
            Duration::from_secs(1),
        )),
    };

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("oli-demo-rust-app listening on {addr} (Drupal: {drupal})");
    axum::serve(listener, router(state)).await?;
    Ok(())
}
