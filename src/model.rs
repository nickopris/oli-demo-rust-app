//! Raw data shapes shared by the snapshot, the Yahoo client and the card builder.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    /// Unix seconds (UTC) when the data was fetched.
    pub fetched_at: i64,
    pub companies: Vec<Company>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Company {
    pub ticker: String,
    pub name: String,
    #[serde(default)]
    pub sector: Option<String>,
    #[serde(default)]
    pub exchange: Option<String>,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub market_cap: Option<f64>,
    /// Daily bars, oldest first.
    #[serde(default)]
    pub bars: Vec<Bar>,
    #[serde(default)]
    pub fundamentals: Fundamentals,
    #[serde(default)]
    pub analysts: Option<Analysts>,
    /// Most recent quarter first.
    #[serde(default)]
    pub earnings: Vec<Earning>,
    #[serde(default)]
    pub dividend: Dividend,
    #[serde(default)]
    pub news: Vec<NewsItem>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Bar {
    pub t: i64,
    pub close: f64,
    pub volume: f64,
}

/// ROE and growth figures are percentages; debt/equity is a ratio.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Fundamentals {
    #[serde(default)]
    pub pe: Option<f64>,
    #[serde(default)]
    pub roe_pct: Option<f64>,
    #[serde(default)]
    pub debt_to_equity: Option<f64>,
    #[serde(default)]
    pub eps_growth_pct: Option<f64>,
    #[serde(default)]
    pub revenue_growth_pct: Option<f64>,
    #[serde(default)]
    pub pb: Option<f64>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Analysts {
    pub strong_buy: u32,
    pub buy: u32,
    pub hold: u32,
    pub sell: u32,
    pub strong_sell: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Earning {
    pub actual: f64,
    pub estimate: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Dividend {
    #[serde(default)]
    pub yield_pct: Option<f64>,
    #[serde(default)]
    pub rate_annual: Option<f64>,
    #[serde(default)]
    pub growth_5y_pct: Option<f64>,
    #[serde(default)]
    pub payout_ratio_pct: Option<f64>,
    #[serde(default)]
    pub ex_date: Option<i64>,
    #[serde(default)]
    pub pay_date: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsItem {
    pub title: String,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
    pub published_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quote {
    pub ticker: String,
    pub price: Option<f64>,
    pub change_pct: Option<f64>,
}
