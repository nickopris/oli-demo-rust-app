//! Fetches the curated tickers from Yahoo and rewrites data/snapshot.json.
//! All or nothing: if any ticker fails, the existing snapshot is left untouched.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use oli_demo_rust_app::model::Snapshot;
use oli_demo_rust_app::tickers::CURATED;
use oli_demo_rust_app::yahoo::YahooClient;

const PATH: &str = "data/snapshot.json";

#[tokio::main]
async fn main() {
    let yahoo = YahooClient::new();
    let mut companies = Vec::new();
    for (i, (ticker, _)) in CURATED.iter().enumerate() {
        if i > 0 {
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
        match yahoo.company(ticker).await {
            Ok(c) => {
                println!("{ticker}: {} bars, {} news", c.bars.len(), c.news.len());
                companies.push(c);
            }
            Err(e) => {
                eprintln!("{ticker}: {e:#}\nSnapshot not updated.");
                std::process::exit(1);
            }
        }
    }
    let fetched_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let json = serde_json::to_string_pretty(&Snapshot {
        fetched_at,
        companies,
    })
    .unwrap();
    std::fs::write(PATH, json + "\n").expect("write snapshot");
    println!("Wrote {PATH}");
}
