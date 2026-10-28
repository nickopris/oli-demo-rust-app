#![allow(dead_code)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use oli_demo_rust_app::market::LiveSource;
use oli_demo_rust_app::model::{Company, Quote};

#[derive(Clone, Copy)]
pub enum Mode {
    Ok,
    Err,
    Hang,
}

/// A stand-in for Yahoo that counts calls and behaves as told.
#[derive(Clone)]
pub struct FakeSource {
    pub mode: Mode,
    pub calls: Arc<AtomicUsize>,
}

impl FakeSource {
    pub fn new(mode: Mode) -> Self {
        Self { mode, calls: Arc::new(AtomicUsize::new(0)) }
    }
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
    async fn act(&self) -> anyhow::Result<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        match self.mode {
            Mode::Ok => Ok(()),
            Mode::Err => anyhow::bail!("fake failure"),
            Mode::Hang => {
                tokio::time::sleep(Duration::from_secs(5)).await;
                Ok(())
            }
        }
    }
}

impl LiveSource for FakeSource {
    async fn company(&self, t: &str) -> anyhow::Result<Company> {
        self.act().await?;
        Ok(Company { ticker: t.to_string(), name: "LIVE".into(), ..Default::default() })
    }
    async fn quote(&self, t: &str) -> anyhow::Result<Quote> {
        self.act().await?;
        Ok(Quote { ticker: t.to_string(), price: Some(1.0), change_pct: Some(0.5) })
    }
}

use oli_demo_rust_app::market::{MarketConfig, MarketData};
use oli_demo_rust_app::snapshot::embedded;
use oli_demo_rust_app::tips::TipSource;
use oli_demo_rust_app::web::AppState;

/// App state backed by a fake Yahoo and an unreachable Drupal.
pub fn test_state(mode: Mode) -> AppState<FakeSource> {
    let cfg = MarketConfig { timeout: Duration::from_millis(100), ..MarketConfig::default() };
    AppState {
        market: Arc::new(MarketData::new(FakeSource::new(mode), embedded(), cfg)),
        tips: Arc::new(TipSource::new(
            "http://127.0.0.1:9".into(),
            "http://127.0.0.1:9".into(),
            Duration::from_millis(300),
        )),
    }
}
