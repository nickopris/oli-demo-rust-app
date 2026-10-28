//! The curated tickers the demo knows about, and search over them.

use crate::model::Snapshot;

/// Ticker plus everyday names that don't appear in the company's legal name.
pub const CURATED: [(&str, &[&str]); 10] = [
    ("DIS", &[]),
    ("AAPL", &[]),
    ("MSFT", &[]),
    ("NVDA", &[]),
    ("TSLA", &[]),
    ("KO", &["coke"]),
    ("AMZN", &[]),
    ("NFLX", &[]),
    ("GOOGL", &["google"]),
    ("META", &["facebook"]),
];

const MAX_RESULTS: usize = 10;

/// Case-insensitive search over ticker, company name and aliases.
/// Ranked: exact ticker, ticker prefix, then name/alias substring.
pub fn search(snapshot: &Snapshot, q: &str) -> Vec<(String, String)> {
    let q = q.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let mut ranked: Vec<(u8, usize, String, String)> = Vec::new();
    for (i, c) in snapshot.companies.iter().enumerate() {
        let ticker = c.ticker.to_lowercase();
        let aliases = CURATED
            .iter()
            .find(|(t, _)| t.eq_ignore_ascii_case(&c.ticker))
            .map(|(_, a)| *a)
            .unwrap_or(&[]);
        let rank = if ticker == q {
            0
        } else if ticker.starts_with(&q) {
            1
        } else if c.name.to_lowercase().contains(&q) || aliases.iter().any(|a| a.contains(&q)) {
            2
        } else {
            continue;
        };
        ranked.push((rank, i, c.ticker.clone(), c.name.clone()));
    }
    ranked.sort_by_key(|(rank, i, _, _)| (*rank, *i));
    ranked
        .into_iter()
        .take(MAX_RESULTS)
        .map(|(_, _, t, n)| (t, n))
        .collect()
}
