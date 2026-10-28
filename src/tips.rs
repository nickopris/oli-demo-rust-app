//! Investing tips from Drupal's JSON:API, with built-in tips when Drupal can't be reached.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
pub struct Tip {
    pub title: String,
    pub text: String,
    pub url: Option<String>,
    pub offline: bool,
}

/// Keep in sync with .drupalaibp/seed-tips.php, which creates the same tips in Drupal.
pub const FALLBACK_TIPS: [(&str, &str); 5] = [
    ("Spread your risk", "Diversify across companies, sectors and regions so one bad bet can't sink your portfolio."),
    ("Time in the market", "Staying invested through ups and downs has historically beaten trying to time the perfect entry."),
    ("Mind the fees", "A 1% yearly fee compounds too. Low-cost index funds keep more of your returns working for you."),
    ("Emergency fund first", "Keep three to six months of expenses in cash before investing, so you never have to sell in a downturn."),
    ("Don't chase hype", "If everyone is talking about a stock, the easy gains are usually gone. Research before you buy."),
];

const PATH: &str = "/jsonapi/node/investing_tip\
                    ?filter[status]=1&fields[node--investing_tip]=title,field_tip_text,drupal_internal__nid";

pub fn parse_tips(v: &Value, public_url: &str) -> Vec<Tip> {
    let base = public_url.trim_end_matches('/');
    v["data"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let a = &item["attributes"];
                    Some(Tip {
                        title: a["title"].as_str()?.to_string(),
                        text: a["field_tip_text"].as_str().unwrap_or_default().to_string(),
                        url: a["drupal_internal__nid"].as_i64().map(|nid| format!("{base}/node/{nid}")),
                        offline: false,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn pick<T>(items: &[T]) -> &T {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(0);
    &items[nanos as usize % items.len()]
}

pub struct TipSource {
    http: reqwest::Client,
    base_url: String,
    public_url: String,
}

impl TipSource {
    pub fn new(base_url: String, public_url: String, timeout: Duration) -> Self {
        let http = reqwest::Client::builder().timeout(timeout).build().expect("reqwest client builds");
        Self { http, base_url: base_url.trim_end_matches('/').to_string(), public_url }
    }

    async fn fetch(&self) -> anyhow::Result<Vec<Tip>> {
        let res = self
            .http
            .get(format!("{}{PATH}", self.base_url))
            .header("Accept", "application/vnd.api+json")
            .send()
            .await?
            .error_for_status()?;
        Ok(parse_tips(&res.json().await?, &self.public_url))
    }

    /// A random published tip from Drupal, or a built-in one marked offline.
    pub async fn random_tip(&self) -> Tip {
        match self.fetch().await {
            Ok(tips) if !tips.is_empty() => pick(&tips).clone(),
            _ => {
                let (title, text) = pick(&FALLBACK_TIPS);
                Tip { title: title.to_string(), text: text.to_string(), url: None, offline: true }
            }
        }
    }
}
