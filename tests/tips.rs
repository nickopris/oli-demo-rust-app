use std::time::{Duration, Instant};

use oli_demo_rust_app::tips::{parse_tips, TipSource, FALLBACK_TIPS};
use serde_json::{json, Value};

fn fixture() -> Value {
    let path = format!("{}/tests/fixtures/jsonapi_tips.json", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn parses_tips_with_public_links() {
    let tips = parse_tips(&fixture(), "https://demo.ddev.site/");
    assert_eq!(tips.len(), 2);
    assert_eq!(tips[0].title, "Spread your risk");
    assert_eq!(tips[0].url.as_deref(), Some("https://demo.ddev.site/node/7"));
    assert!(!tips[0].offline);
    assert_eq!(tips[1].text, "");
}

#[test]
fn empty_collection_parses_to_nothing() {
    assert!(parse_tips(&json!({"data": []}), "https://demo.ddev.site").is_empty());
    assert!(parse_tips(&json!({}), "https://demo.ddev.site").is_empty());
}

#[tokio::test]
async fn unreachable_drupal_falls_back_quickly() {
    let src = TipSource::new("http://127.0.0.1:9".into(), "http://127.0.0.1:9".into(), Duration::from_secs(1));
    let start = Instant::now();
    let tip = src.random_tip().await;
    assert!(start.elapsed() < Duration::from_millis(1500));
    assert!(tip.offline);
    assert!(tip.url.is_none());
    assert!(FALLBACK_TIPS.iter().any(|(t, x)| *t == tip.title && *x == tip.text));
}

#[test]
fn fallback_tips_match_the_drupal_seed() {
    let titles: Vec<&str> = FALLBACK_TIPS.iter().map(|(t, _)| *t).collect();
    assert_eq!(
        titles,
        ["Spread your risk", "Time in the market", "Mind the fees", "Emergency fund first", "Don't chase hype"]
    );
}
