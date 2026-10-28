//! Yahoo Finance (unofficial, free, no key). Parsing is kept in pure functions so
//! it can be tested against captured payloads; the client only does HTTP.

use std::time::Duration;

use anyhow::{anyhow, bail, Context};
use serde_json::Value;
use tokio::sync::Mutex;

use crate::model::{Analysts, Bar, Company, Earning, NewsItem, Quote};

const UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0) AppleWebKit/537.36 \
                  (KHTML, like Gecko) Chrome/130.0 Safari/537.36";
const MODULES: &str = "price,summaryProfile,summaryDetail,financialData,defaultKeyStatistics,\
                       recommendationTrend,earningsHistory,calendarEvents";
const MAX_NEWS: usize = 3;

#[derive(Debug, Clone)]
pub struct ChartData {
    pub bars: Vec<Bar>,
    pub price: Option<f64>,
    pub exchange: Option<String>,
}

/// Yahoo wraps most numbers as `{"raw": 1.23, "fmt": "1.23"}`; missing values are `{}`.
fn raw(v: &Value) -> Option<f64> {
    v.get("raw").unwrap_or(v).as_f64().filter(|f| f.is_finite())
}

fn raw_i64(v: &Value) -> Option<i64> {
    raw(v).map(|f| f as i64)
}

fn text(v: &Value) -> Option<String> {
    v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(String::from)
}

pub fn parse_chart(v: &Value) -> anyhow::Result<ChartData> {
    let r = v
        .pointer("/chart/result/0")
        .filter(|r| !r.is_null())
        .ok_or_else(|| anyhow!("chart has no result: {}", v.pointer("/chart/error").unwrap_or(&Value::Null)))?;
    let ts = r["timestamp"].as_array().context("chart has no timestamps")?;
    let q = &r["indicators"]["quote"][0];
    let (closes, volumes) = (q["close"].as_array(), q["volume"].as_array());
    let (closes, volumes) = closes.zip(volumes).context("chart has no quote series")?;
    let bars = ts
        .iter()
        .zip(closes)
        .zip(volumes)
        .filter_map(|((t, c), v)| {
            Some(Bar { t: t.as_i64()?, close: c.as_f64()?, volume: v.as_f64()? })
        })
        .collect();
    let meta = &r["meta"];
    Ok(ChartData {
        bars,
        price: raw(&meta["regularMarketPrice"]),
        exchange: text(&meta["fullExchangeName"]).or_else(|| text(&meta["exchangeName"])),
    })
}

pub fn parse_summary(v: &Value, c: &mut Company) -> anyhow::Result<()> {
    let r = v.pointer("/quoteSummary/result/0").unwrap_or(&Value::Null);
    let pct = |p: &str| r.pointer(p).and_then(raw).map(|f| f * 100.0);
    let num = |p: &str| r.pointer(p).and_then(raw);

    if let Some(name) = r.pointer("/price/longName").and_then(text).or_else(|| r.pointer("/price/shortName").and_then(text)) {
        c.name = name;
    }
    c.sector = r
        .pointer("/summaryProfile/industry")
        .and_then(text)
        .or_else(|| r.pointer("/summaryProfile/sector").and_then(text))
        .or(c.sector.take());
    c.market_cap = num("/price/marketCap").or(c.market_cap);
    c.price = c.price.or_else(|| num("/price/regularMarketPrice"));

    let f = &mut c.fundamentals;
    f.pe = num("/summaryDetail/trailingPE");
    f.roe_pct = pct("/financialData/returnOnEquity");
    f.debt_to_equity = num("/financialData/debtToEquity").map(|d| d / 100.0);
    f.eps_growth_pct = pct("/financialData/earningsGrowth");
    f.revenue_growth_pct = pct("/financialData/revenueGrowth");
    f.pb = num("/defaultKeyStatistics/priceToBook");

    c.analysts = r.pointer("/recommendationTrend/trend/0").map(|t| {
        let n = |k: &str| t[k].as_u64().unwrap_or(0) as u32;
        Analysts {
            strong_buy: n("strongBuy"),
            buy: n("buy"),
            hold: n("hold"),
            sell: n("sell"),
            strong_sell: n("strongSell"),
        }
    });

    c.earnings = r
        .pointer("/earningsHistory/history")
        .and_then(Value::as_array)
        .map(|h| {
            h.iter()
                .rev()
                .filter_map(|e| Some(Earning { actual: raw(&e["epsActual"])?, estimate: raw(&e["epsEstimate"])? }))
                .collect()
        })
        .unwrap_or_default();

    let d = &mut c.dividend;
    d.yield_pct = pct("/summaryDetail/dividendYield");
    d.rate_annual = num("/summaryDetail/dividendRate");
    d.payout_ratio_pct = pct("/summaryDetail/payoutRatio");
    d.ex_date = r
        .pointer("/calendarEvents/exDividendDate")
        .and_then(raw_i64)
        .or_else(|| r.pointer("/summaryDetail/exDividendDate").and_then(raw_i64));
    d.pay_date = r.pointer("/calendarEvents/dividendDate").and_then(raw_i64);
    // Yahoo has no five-year dividend growth figure; it stays N/A.
    d.growth_5y_pct = None;
    Ok(())
}

pub fn parse_news(v: &Value) -> Vec<NewsItem> {
    v["news"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|n| {
                    Some(NewsItem {
                        title: text(&n["title"])?,
                        publisher: text(&n["publisher"]),
                        link: text(&n["link"]),
                        published_at: n["providerPublishTime"].as_i64()?,
                    })
                })
                .take(MAX_NEWS)
                .collect()
        })
        .unwrap_or_default()
}

pub struct YahooClient {
    http: reqwest::Client,
    crumb: Mutex<Option<String>>,
}

impl Default for YahooClient {
    fn default() -> Self {
        Self::new()
    }
}

impl YahooClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            .user_agent(UA)
            .cookie_store(true)
            .timeout(Duration::from_secs(10))
            .build()
            .expect("reqwest client builds");
        Self { http, crumb: Mutex::new(None) }
    }

    async fn get_json(&self, url: &str) -> anyhow::Result<Value> {
        let res = self.http.get(url).send().await?;
        let status = res.status();
        if !status.is_success() {
            bail!("Yahoo returned {status} for {url}");
        }
        Ok(res.json().await?)
    }

    async fn chart(&self, ticker: &str, range: &str) -> anyhow::Result<ChartData> {
        let url = format!("https://query1.finance.yahoo.com/v8/finance/chart/{ticker}?range={range}&interval=1d");
        parse_chart(&self.get_json(&url).await?)
    }

    async fn crumb(&self, refresh: bool) -> anyhow::Result<String> {
        let mut crumb = self.crumb.lock().await;
        if refresh || crumb.is_none() {
            // Sets the session cookie; the response itself is a 404 and is ignored.
            let _ = self.http.get("https://fc.yahoo.com").send().await;
            let res = self.http.get("https://query1.finance.yahoo.com/v1/test/getcrumb").send().await?;
            if !res.status().is_success() {
                bail!("Yahoo crumb request returned {}", res.status());
            }
            let value = res.text().await?.trim().to_string();
            if value.is_empty() || value.contains(' ') {
                bail!("Yahoo returned an unusable crumb");
            }
            *crumb = Some(value);
        }
        Ok(crumb.clone().unwrap_or_default())
    }

    async fn summary(&self, ticker: &str) -> anyhow::Result<Value> {
        for refresh in [false, true] {
            let crumb = self.crumb(refresh).await?;
            let url = format!(
                "https://query2.finance.yahoo.com/v10/finance/quoteSummary/{ticker}?modules={MODULES}&crumb={crumb}"
            );
            let res = self.http.get(&url).send().await?;
            match res.status().as_u16() {
                401 | 403 => continue,
                s if (200..300).contains(&s) => return Ok(res.json().await?),
                s => bail!("Yahoo quoteSummary returned {s} for {ticker}"),
            }
        }
        bail!("Yahoo rejected the crumb for {ticker}")
    }

    async fn news(&self, ticker: &str) -> Vec<NewsItem> {
        let url = format!(
            "https://query1.finance.yahoo.com/v1/finance/search?q={ticker}&newsCount={MAX_NEWS}&quotesCount=0"
        );
        self.get_json(&url).await.map(|v| parse_news(&v)).unwrap_or_default()
    }

    /// Full company data. Chart and summary are required; news is best effort.
    pub async fn company(&self, ticker: &str) -> anyhow::Result<Company> {
        let (chart, summary, news) =
            tokio::join!(self.chart(ticker, "1y"), self.summary(ticker), self.news(ticker));
        let chart = chart?;
        let mut c = Company {
            ticker: ticker.to_string(),
            name: ticker.to_string(),
            exchange: chart.exchange,
            price: chart.price,
            bars: chart.bars,
            news,
            ..Default::default()
        };
        parse_summary(&summary?, &mut c)?;
        Ok(c)
    }

    /// Latest price and day change, from the chart endpoint only (no crumb needed).
    pub async fn quote(&self, ticker: &str) -> anyhow::Result<Quote> {
        let chart = self.chart(ticker, "5d").await?;
        let prev = chart.bars.len().checked_sub(2).map(|i| chart.bars[i].close);
        let price = chart.price.or(chart.bars.last().map(|b| b.close));
        let change_pct = match (price, prev) {
            (Some(p), Some(prev)) if prev != 0.0 => Some((p - prev) / prev * 100.0),
            _ => None,
        };
        Ok(Quote { ticker: ticker.to_string(), price, change_pct })
    }
}
