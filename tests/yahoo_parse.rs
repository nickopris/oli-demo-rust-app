use oli_demo_rust_app::model::Company;
use oli_demo_rust_app::yahoo::{parse_chart, parse_news, parse_summary};
use serde_json::{json, Value};

fn fixture(name: &str) -> Value {
    let path = format!("{}/tests/fixtures/yahoo/{name}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn chart_parses_a_year_of_bars() {
    let c = parse_chart(&fixture("chart_dis.json")).unwrap();
    assert!(c.bars.len() >= 200);
    assert!(c
        .bars
        .iter()
        .all(|b| b.close.is_finite() && b.volume.is_finite()));
    assert!(c.bars.windows(2).all(|w| w[0].t < w[1].t));
    assert!(c.price.is_some());
    assert_eq!(c.exchange.as_deref(), Some("NYSE"));
}

#[test]
fn chart_skips_null_closes() {
    let full = parse_chart(&fixture("chart_dis.json")).unwrap();
    let gappy = parse_chart(&fixture("chart_nulls.json")).unwrap();
    assert_eq!(gappy.bars.len(), full.bars.len() - 3);
}

#[test]
fn chart_without_result_is_an_error() {
    assert!(
        parse_chart(&json!({"chart": {"result": null, "error": {"code": "Not Found"}}})).is_err()
    );
}

#[test]
fn summary_fills_company() {
    let mut c = Company::default();
    parse_summary(&fixture("summary_dis.json"), &mut c).unwrap();
    assert_eq!(c.name, "The Walt Disney Company");
    let roe = c.fundamentals.roe_pct.unwrap();
    assert!(roe > 0.0 && roe < 100.0);
    assert!((c.fundamentals.debt_to_equity.unwrap() - 0.412).abs() < 1e-9);
    assert!((c.dividend.yield_pct.unwrap() - 1.17).abs() < 1e-9);
    assert!(c.dividend.growth_5y_pct.is_none());
    let a = c.analysts.unwrap();
    assert_eq!(
        (a.strong_buy, a.buy, a.hold, a.sell, a.strong_sell),
        (8, 18, 12, 1, 0)
    );
    // most recent quarter first
    assert!((c.earnings[0].actual - 1.31).abs() < 1e-9);
    assert_eq!(c.market_cap, Some(190_700_000_000.0));
}

#[test]
fn summary_tolerates_missing_modules() {
    let mut c = Company::default();
    parse_summary(&json!({}), &mut c).unwrap();
    assert!(c.fundamentals.pe.is_none() && c.analysts.is_none() && c.earnings.is_empty());
    assert!(c.dividend.yield_pct.is_none());
}

#[test]
fn news_is_capped_at_three() {
    let n = parse_news(&fixture("search_dis.json"));
    assert_eq!(n.len(), 3);
    assert!(n.iter().all(|i| !i.title.is_empty() && i.published_at > 0));
    assert_eq!(n[1].publisher.as_deref(), Some("Reuters"));
}
