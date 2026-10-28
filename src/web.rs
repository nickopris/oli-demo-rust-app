use axum::{routing::get, Router};

#[derive(Clone, Default)]
pub struct AppState;

pub fn router(_state: AppState) -> Router {
    Router::new().route("/health", get(|| async { "ok" }))
}
