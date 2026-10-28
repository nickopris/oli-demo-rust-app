//! HTTP routes: the page, JSON for the card, ticker band and search, and a health check.

use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;

use crate::market::{LiveSource, MarketData};
use crate::model::Quote;
use crate::signals::build_card;
use crate::snapshot::embedded;
use crate::tickers::{search, CURATED};
use crate::tips::TipSource;

const INDEX: &str = include_str!("../assets/index.html");

pub struct AppState<S> {
    pub market: Arc<MarketData<S>>,
    pub tips: Arc<TipSource>,
}

impl<S> Clone for AppState<S> {
    fn clone(&self) -> Self {
        Self {
            market: Arc::clone(&self.market),
            tips: Arc::clone(&self.tips),
        }
    }
}

pub fn router<S: LiveSource>(state: AppState<S>) -> Router {
    Router::new()
        .route("/", get(|| async { Html(INDEX) }))
        .route("/health", get(|| async { "ok" }))
        .route("/api/tickers", get(tickers))
        .route("/api/tickers/live", get(live_tickers::<S>))
        .route("/api/search", get(search_route))
        .route("/api/company/{ticker}", get(company::<S>))
        .with_state(state)
}

/// Band prices from the snapshot, so the page fills instantly.
async fn tickers() -> impl IntoResponse {
    let list: Vec<_> = embedded()
        .companies
        .iter()
        .map(|c| {
            let card = build_card(c, embedded().fetched_at);
            json!({ "ticker": c.ticker, "name": c.name, "price": card.price, "change_pct": card.change_pct })
        })
        .collect();
    Json(list)
}

async fn live_tickers<S: LiveSource>(State(st): State<AppState<S>>) -> Json<Vec<Quote>> {
    Json(st.market.quotes().await)
}

#[derive(Deserialize)]
struct SearchParams {
    #[serde(default)]
    q: String,
}

async fn search_route(Query(p): Query<SearchParams>) -> impl IntoResponse {
    let hits: Vec<_> = search(embedded(), &p.q)
        .into_iter()
        .map(|(ticker, name)| json!({ "ticker": ticker, "name": name }))
        .collect();
    Json(hits)
}

async fn company<S: LiveSource>(
    State(st): State<AppState<S>>,
    Path(ticker): Path<String>,
) -> Response {
    let (data, tip) = tokio::join!(st.market.company(&ticker), st.tips.random_tip());
    match data {
        Some((c, as_of)) => {
            Json(json!({ "card": build_card(&c, as_of), "tip": tip })).into_response()
        }
        None => {
            let tickers: Vec<&str> = CURATED.iter().map(|(t, _)| *t).collect();
            (
                StatusCode::NOT_FOUND,
                Json(json!({ "error": "not_found", "tickers": tickers })),
            )
                .into_response()
        }
    }
}
