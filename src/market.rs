//! Live data from Yahoo when it answers quickly, the embedded snapshot when it doesn't.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::task::JoinSet;

use crate::model::{Company, Quote, Snapshot};
use crate::tickers::CURATED;
use crate::yahoo::YahooClient;

pub trait LiveSource: Send + Sync + 'static {
    fn company(&self, t: &str) -> impl Future<Output = anyhow::Result<Company>> + Send;
    fn quote(&self, t: &str) -> impl Future<Output = anyhow::Result<Quote>> + Send;
}

impl LiveSource for YahooClient {
    fn company(&self, t: &str) -> impl Future<Output = anyhow::Result<Company>> + Send {
        YahooClient::company(self, t)
    }
    fn quote(&self, t: &str) -> impl Future<Output = anyhow::Result<Quote>> + Send {
        YahooClient::quote(self, t)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MarketConfig {
    /// Longest we wait for Yahoo on one lookup.
    pub timeout: Duration,
    /// How long a live result is reused.
    pub ttl: Duration,
    /// How long we stop asking Yahoo after it fails.
    pub backoff: Duration,
}

impl Default for MarketConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_millis(2500),
            ttl: Duration::from_secs(600),
            backoff: Duration::from_secs(60),
        }
    }
}

struct Cached<T> {
    at: Instant,
    as_of: i64,
    value: T,
}

pub struct MarketData<S> {
    source: Arc<S>,
    snapshot: &'static Snapshot,
    cfg: MarketConfig,
    companies: Mutex<HashMap<String, Cached<Company>>>,
    quotes: Mutex<HashMap<String, Cached<Quote>>>,
    down_until: Mutex<Option<Instant>>,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn curated(ticker: &str) -> Option<&'static str> {
    let t = ticker.trim();
    CURATED
        .iter()
        .map(|(c, _)| *c)
        .find(|c| c.eq_ignore_ascii_case(t))
}

impl<S: LiveSource> MarketData<S> {
    pub fn new(source: S, snapshot: &'static Snapshot, cfg: MarketConfig) -> Self {
        Self {
            source: Arc::new(source),
            snapshot,
            cfg,
            companies: Mutex::default(),
            quotes: Mutex::default(),
            down_until: Mutex::default(),
        }
    }

    fn snapshot_company(&self, ticker: &str) -> Option<&'static Company> {
        self.snapshot.companies.iter().find(|c| c.ticker == ticker)
    }

    fn yahoo_down(&self) -> bool {
        self.down_until
            .lock()
            .unwrap()
            .is_some_and(|until| Instant::now() < until)
    }

    fn mark_down(&self) {
        *self.down_until.lock().unwrap() = Some(Instant::now() + self.cfg.backoff);
    }

    fn fresh<T: Clone>(
        &self,
        map: &Mutex<HashMap<String, Cached<T>>>,
        t: &str,
    ) -> Option<(T, i64)> {
        let map = map.lock().unwrap();
        map.get(t)
            .filter(|c| c.at.elapsed() < self.cfg.ttl)
            .map(|c| (c.value.clone(), c.as_of))
    }

    /// The company plus the unix time its data was fetched. `None` for tickers outside the curated list.
    pub async fn company(&self, ticker: &str) -> Option<(Company, i64)> {
        let t = curated(ticker)?;
        if let Some(hit) = self.fresh(&self.companies, t) {
            return Some(hit);
        }
        if !self.yahoo_down() {
            match tokio::time::timeout(self.cfg.timeout, self.source.company(t)).await {
                Ok(Ok(c)) => {
                    let as_of = now_secs();
                    self.companies.lock().unwrap().insert(
                        t.to_string(),
                        Cached {
                            at: Instant::now(),
                            as_of,
                            value: c.clone(),
                        },
                    );
                    return Some((c, as_of));
                }
                _ => self.mark_down(),
            }
        }
        self.snapshot_company(t)
            .map(|c| (c.clone(), self.snapshot.fetched_at))
    }

    fn snapshot_quote(&self, t: &str) -> Quote {
        let c = self.snapshot_company(t);
        let closes: Vec<f64> = c
            .map(|c| c.bars.iter().map(|b| b.close).collect())
            .unwrap_or_default();
        let price = c.and_then(|c| c.price).or(closes.last().copied());
        let prev = closes.len().checked_sub(2).map(|i| closes[i]);
        let change_pct = match (price, prev) {
            (Some(p), Some(prev)) if prev != 0.0 => Some((p - prev) / prev * 100.0),
            _ => None,
        };
        Quote {
            ticker: t.to_string(),
            price,
            change_pct,
        }
    }

    /// Prices for every curated ticker, live where possible, in curated order.
    pub async fn quotes(&self) -> Vec<Quote> {
        let tickers: Vec<&'static str> = CURATED.iter().map(|(t, _)| *t).collect();
        let mut live: HashMap<String, Quote> = HashMap::new();
        let mut wanted = Vec::new();
        for t in &tickers {
            match self.fresh(&self.quotes, t) {
                Some((q, _)) => {
                    live.insert(t.to_string(), q);
                }
                None => wanted.push(*t),
            }
        }
        if !wanted.is_empty() && !self.yahoo_down() {
            let mut set = JoinSet::new();
            for t in wanted {
                let source = Arc::clone(&self.source);
                let timeout = self.cfg.timeout;
                set.spawn(async move { tokio::time::timeout(timeout, source.quote(t)).await });
            }
            let mut failed = false;
            while let Some(res) = set.join_next().await {
                match res {
                    Ok(Ok(Ok(q))) => {
                        self.quotes.lock().unwrap().insert(
                            q.ticker.clone(),
                            Cached {
                                at: Instant::now(),
                                as_of: now_secs(),
                                value: q.clone(),
                            },
                        );
                        live.insert(q.ticker.clone(), q);
                    }
                    _ => failed = true,
                }
            }
            if failed {
                self.mark_down();
            }
        }
        tickers
            .iter()
            .map(|t| live.remove(*t).unwrap_or_else(|| self.snapshot_quote(t)))
            .collect()
    }
}
