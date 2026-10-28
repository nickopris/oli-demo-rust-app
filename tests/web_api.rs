mod common;

use axum::body::Body;
use axum::http::{header, Request, StatusCode};
use common::{test_state, Mode};
use http_body_util::BodyExt;
use oli_demo_rust_app::snapshot::embedded;
use oli_demo_rust_app::web::router;
use serde_json::Value;
use tower::ServiceExt;

async fn get(uri: &str) -> (StatusCode, Option<String>, Vec<u8>) {
    let res = router(test_state(Mode::Err))
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = res.status();
    let ctype = res
        .headers()
        .get(header::CONTENT_TYPE)
        .map(|v| v.to_str().unwrap().to_string());
    (
        status,
        ctype,
        res.into_body().collect().await.unwrap().to_bytes().to_vec(),
    )
}

async fn get_json(uri: &str) -> (StatusCode, Value) {
    let (s, _, b) = get(uri).await;
    (s, serde_json::from_slice(&b).unwrap())
}

#[tokio::test]
async fn tickers_in_curated_order() {
    let (s, v) = get_json("/api/tickers").await;
    assert_eq!(s, StatusCode::OK);
    let t: Vec<&str> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|q| q["ticker"].as_str().unwrap())
        .collect();
    assert_eq!(
        t,
        ["DIS", "AAPL", "MSFT", "NVDA", "TSLA", "KO", "AMZN", "NFLX", "GOOGL", "META"]
    );
    assert!(v[0]["name"].as_str().unwrap().contains("Disney"));
    assert!(v[0]["price"].is_number());
}

#[tokio::test]
async fn live_tickers_fall_back() {
    let (s, v) = get_json("/api/tickers/live").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v.as_array().unwrap().len(), 10);
}

#[tokio::test]
async fn search_endpoint() {
    let (_, v) = get_json("/api/search?q=disney").await;
    assert_eq!(v[0]["ticker"], "DIS");
    let (_, v) = get_json("/api/search?q=").await;
    assert_eq!(v, serde_json::json!([]));
    let (s, v) = get_json("/api/search").await;
    assert_eq!((s, v), (StatusCode::OK, serde_json::json!([])));
}

#[tokio::test]
async fn company_card_from_snapshot_with_offline_tip() {
    let (s, v) = get_json("/api/company/dis").await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(v["card"]["ticker"], "DIS");
    assert_eq!(v["card"]["as_of"], embedded().fetched_at);
    assert_eq!(v["card"]["momentum"].as_array().unwrap().len(), 8);
    assert_eq!(v["tip"]["offline"], true);
}

#[tokio::test]
async fn unknown_company_is_friendly_404() {
    for uri in ["/api/company/ZZZZ", "/api/company/%3Cx%3E"] {
        let (s, v) = get_json(uri).await;
        assert_eq!(s, StatusCode::NOT_FOUND, "{uri}");
        assert_eq!(v["error"], "not_found");
        assert_eq!(v["tickers"].as_array().unwrap().len(), 10);
    }
}

#[tokio::test]
async fn index_is_html() {
    let (s, ctype, _) = get("/").await;
    assert_eq!(s, StatusCode::OK);
    assert!(ctype.unwrap().starts_with("text/html"));
}
