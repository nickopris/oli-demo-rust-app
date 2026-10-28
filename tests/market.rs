mod common;

use std::time::{Duration, Instant};

use common::{FakeSource, Mode};
use oli_demo_rust_app::market::{MarketConfig, MarketData};
use oli_demo_rust_app::snapshot::embedded;

fn cfg() -> MarketConfig {
    MarketConfig {
        timeout: Duration::from_millis(100),
        ttl: Duration::from_secs(1),
        backoff: Duration::from_millis(300),
    }
}

fn market(mode: Mode) -> (MarketData<FakeSource>, FakeSource) {
    let src = FakeSource::new(mode);
    (MarketData::new(src.clone(), embedded(), cfg()), src)
}

#[tokio::test]
async fn live_success_is_stamped_now() {
    let (m, _) = market(Mode::Ok);
    let (c, as_of) = m.company("DIS").await.unwrap();
    assert_eq!(c.name, "LIVE");
    assert!(as_of > embedded().fetched_at);
}

#[tokio::test]
async fn live_failure_falls_back_to_snapshot() {
    let (m, _) = market(Mode::Err);
    let (c, as_of) = m.company("DIS").await.unwrap();
    assert_eq!(c.name, "The Walt Disney Company");
    assert_eq!(as_of, embedded().fetched_at);
}

#[tokio::test]
async fn hanging_source_is_cut_off_by_timeout() {
    let (m, _) = market(Mode::Hang);
    let start = Instant::now();
    let (_, as_of) = m.company("DIS").await.unwrap();
    assert!(start.elapsed() < Duration::from_millis(300));
    assert_eq!(as_of, embedded().fetched_at);
}

#[tokio::test]
async fn live_results_are_cached_within_ttl() {
    let (m, src) = market(Mode::Ok);
    m.company("DIS").await.unwrap();
    m.company("dis").await.unwrap();
    assert_eq!(src.calls(), 1);
}

#[tokio::test]
async fn failure_starts_backoff() {
    let (m, src) = market(Mode::Err);
    m.company("DIS").await.unwrap();
    m.company("AAPL").await.unwrap();
    assert_eq!(src.calls(), 1);
    tokio::time::sleep(Duration::from_millis(350)).await;
    m.company("AAPL").await.unwrap();
    assert_eq!(src.calls(), 2);
}

#[tokio::test]
async fn unknown_ticker_is_none_and_never_hits_yahoo() {
    let (m, src) = market(Mode::Ok);
    assert!(m.company("ZZZZ").await.is_none());
    assert!(m.company("  dis ").await.is_some());
    assert_eq!(src.calls(), 1);
}

#[tokio::test]
async fn quotes_cover_all_tickers_with_fallback() {
    let (m, _) = market(Mode::Err);
    let q = m.quotes().await;
    assert_eq!(q.len(), 10);
    assert_eq!(q[0].ticker, "DIS");
    assert!(q.iter().all(|q| q.price.is_some()));

    let (m, _) = market(Mode::Ok);
    assert!(m.quotes().await.iter().all(|q| q.price == Some(1.0)));
}
