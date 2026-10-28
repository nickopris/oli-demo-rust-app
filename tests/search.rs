use oli_demo_rust_app::snapshot::embedded;
use oli_demo_rust_app::tickers::{search, CURATED};

fn first(q: &str) -> Option<String> {
    search(embedded(), q).into_iter().next().map(|(t, _)| t)
}

#[test]
fn finds_by_company_name() {
    assert_eq!(first("disney").as_deref(), Some("DIS"));
}

#[test]
fn finds_by_partial_name() {
    assert!(search(embedded(), "disn").iter().any(|(t, _)| t == "DIS"));
}

#[test]
fn ticker_match_is_trimmed_and_case_insensitive() {
    assert_eq!(first(" dis ").as_deref(), Some("DIS"));
    assert_eq!(first("DIS").as_deref(), Some("DIS"));
    assert_eq!(first("dis").as_deref(), Some("DIS"));
}

#[test]
fn aliases_cover_everyday_names() {
    assert_eq!(first("google").as_deref(), Some("GOOGL"));
    assert_eq!(first("facebook").as_deref(), Some("META"));
}

#[test]
fn empty_or_junk_query_returns_nothing() {
    assert!(search(embedded(), "").is_empty());
    assert!(search(embedded(), "   ").is_empty());
    assert!(search(embedded(), &"x".repeat(200)).is_empty());
}

#[test]
fn snapshot_holds_curated_tickers_in_order() {
    let tickers: Vec<&str> = embedded().companies.iter().map(|c| c.ticker.as_str()).collect();
    let curated: Vec<&str> = CURATED.iter().map(|(t, _)| *t).collect();
    assert_eq!(tickers, curated);
    assert_eq!(curated, ["DIS", "AAPL", "MSFT", "NVDA", "TSLA", "KO", "AMZN", "NFLX", "GOOGL", "META"]);
}
