//! Turns raw company data into the labelled signals shown on the card.
//! Rules are ported from finnguide's scorer.js; each signal's 0–100 strength
//! only sizes and colours its bar. There is deliberately no combined score.

use serde::Serialize;

use crate::indicators::{bollinger, clamp, linear_slope, macd, obv, rsi, sma};
use crate::model::{Analysts, Company, Dividend, Earning, NewsItem};

#[derive(Debug, Clone, Serialize)]
pub struct Signal {
    pub name: String,
    pub label: String,
    pub strength: u8,
    pub available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Card {
    pub ticker: String,
    pub name: String,
    pub sector: Option<String>,
    pub exchange: Option<String>,
    pub price: Option<f64>,
    pub market_cap: Option<f64>,
    pub change_pct: Option<f64>,
    pub spark: Vec<f64>,
    pub as_of: i64,
    pub momentum: Vec<Signal>,
    pub fundamentals: Vec<Signal>,
    pub dividend: Dividend,
    pub news: Vec<NewsItem>,
}

const SPARK_DAYS: usize = 14;
const MAX_NEWS: usize = 3;

pub fn build_card(c: &Company, as_of: i64) -> Card {
    let closes: Vec<f64> = c.bars.iter().map(|b| b.close).collect();
    let volumes: Vec<f64> = c.bars.iter().map(|b| b.volume).collect();
    let price = c.price.or(closes.last().copied());
    let f = &c.fundamentals;
    let news: Vec<NewsItem> = c.news.iter().take(MAX_NEWS).cloned().collect();

    let momentum = vec![
        named("RSI (14)", rsi_signal(&closes)),
        named("MACD", macd_signal(&closes)),
        named("Price vs 50d SMA", sma50_signal(&closes, price)),
        named("Bollinger Band", bollinger_signal(&closes, price)),
        named("Volume", volume_signal(&closes, &volumes)),
        named("Analyst Sentiment", analyst_signal(c.analysts.as_ref())),
        named("Earnings Surprise", earnings_signal(&c.earnings)),
        named("News Sentiment", news_signal(&news)),
    ];
    let fundamentals = vec![
        named("P/E Ratio", pe_signal(f.pe)),
        named("ROE", roe_signal(f.roe_pct)),
        named("Debt/Equity", de_signal(f.debt_to_equity)),
        named("EPS Growth", growth_signal(f.eps_growth_pct, "EPS")),
        named(
            "Revenue Growth",
            growth_signal(f.revenue_growth_pct, "revenue"),
        ),
        named("P/B Ratio", pb_signal(f.pb)),
        named("Price vs 200d SMA", sma200_signal(&closes, price)),
    ];

    let change_pct = match closes.as_slice() {
        [.., prev, last] if *prev != 0.0 => Some((last - prev) / prev * 100.0),
        _ => None,
    };

    Card {
        ticker: c.ticker.clone(),
        name: c.name.clone(),
        sector: c.sector.clone(),
        exchange: c.exchange.clone(),
        price,
        market_cap: c.market_cap,
        change_pct,
        spark: closes[closes.len().saturating_sub(SPARK_DAYS)..].to_vec(),
        as_of,
        momentum,
        fundamentals,
        dividend: c.dividend.clone(),
        news,
    }
}

/// A computed (label, strength); `None` means not enough data.
type Rule = Option<(String, f64)>;

fn named(name: &str, rule: Rule) -> Signal {
    match rule {
        Some((label, strength)) => Signal {
            name: name.into(),
            label,
            strength: clamp(strength, 0.0, 100.0).round() as u8,
            available: true,
        },
        None => Signal {
            name: name.into(),
            label: "N/A".into(),
            strength: 50,
            available: false,
        },
    }
}

fn signed(v: f64) -> String {
    format!("{}{:.1}", if v >= 0.0 { "+" } else { "" }, v)
}

/// Mean absolute day-to-day move over the last 14 closes.
fn atr14(closes: &[f64]) -> f64 {
    let recent = &closes[closes.len().saturating_sub(14)..];
    if recent.len() < 2 {
        return 0.0;
    }
    recent.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f64>() / (recent.len() - 1) as f64
}

fn rsi_signal(closes: &[f64]) -> Rule {
    let r = rsi(closes, 14)?;
    let score = if r < 30.0 {
        70.0
    } else if r < 50.0 {
        50.0 + (50.0 - r)
    } else if r <= 65.0 {
        70.0 - (r - 50.0) * 2.0
    } else {
        40.0 - (r - 65.0) * 2.0
    };
    let label = if r < 30.0 {
        "Oversold (bounce opportunity)"
    } else if r < 50.0 {
        "Neutral-bearish"
    } else if r <= 65.0 {
        "Bullish"
    } else {
        "Overbought (caution)"
    };
    Some((label.into(), score))
}

fn macd_signal(closes: &[f64]) -> Rule {
    let m = macd(closes)?;
    let bullish = m.line > m.signal;
    let atr = atr14(closes);
    let normalised = if atr > 0.0 {
        m.histogram.abs() / atr
    } else {
        0.0
    };
    let strength = clamp(normalised / 2.0, 0.0, 1.0);
    Some(if bullish {
        ("Bullish crossover".into(), 60.0 + strength * 40.0)
    } else {
        ("Bearish crossover".into(), 40.0 - strength * 40.0)
    })
}

fn sma50_signal(closes: &[f64], price: Option<f64>) -> Rule {
    let s = sma(closes, 50)?;
    let p = price?;
    let pct = (p - s) / s * 100.0;
    let score = if pct < -10.0 {
        20.0
    } else if pct <= 0.0 {
        40.0 + pct * 2.0
    } else if pct <= 10.0 {
        70.0 + pct
    } else if pct <= 30.0 {
        80.0 - (pct - 10.0) * 0.75
    } else {
        65.0 - (pct - 30.0) * 0.5
    };
    Some((format!("{}% vs 50d SMA", signed(pct)), score))
}

fn bollinger_signal(closes: &[f64], price: Option<f64>) -> Rule {
    let b = bollinger(closes, 20)?;
    let p = price?;
    let range = b.upper - b.lower;
    if range == 0.0 {
        return None;
    }
    let pos = (p - b.lower) / range;
    let trend_up = match (sma(closes, 20), closes.len() >= 25) {
        (Some(now), true) => sma(&closes[..closes.len() - 5], 20).is_none_or(|ago| now >= ago),
        _ => true,
    };
    Some(if pos < 0.2 {
        if trend_up {
            ("Near lower band (bounce likely)".into(), 78.0)
        } else {
            ("Near lower band (downtrend)".into(), 45.0)
        }
    } else if pos < 0.5 {
        ("Lower half (mild bullish)".into(), 65.0)
    } else if pos < 0.8 {
        ("Upper half (neutral)".into(), 50.0)
    } else {
        ("Near upper band (caution)".into(), 30.0)
    })
}

fn volume_signal(closes: &[f64], volumes: &[f64]) -> Rule {
    let n = volumes.len();
    if n < 21 {
        return None;
    }
    let today = volumes[n - 1];
    let avg20 = volumes[n - 21..n - 1].iter().sum::<f64>() / 20.0;
    let ratio = if avg20 > 0.0 { today / avg20 } else { 1.0 };

    let prev = closes[n - 2];
    let price_pct = if prev > 0.0 {
        (closes[n - 1] - prev) / prev
    } else {
        0.0
    };
    let atr = atr14(closes);
    let last = if closes[n - 1] != 0.0 {
        closes[n - 1]
    } else {
        1.0
    };
    let norm_move = if atr > 0.0 {
        price_pct.abs() / (atr / last)
    } else {
        1.0
    };
    let move_factor = clamp(norm_move / 0.5, 0.0, 1.0);
    let up = price_pct > 0.0;

    let ratio_score = if ratio > 1.5 && up {
        50.0 + 30.0 * move_factor
    } else if ratio > 1.5 {
        50.0 - 20.0 * move_factor
    } else if ratio < 0.7 && up {
        50.0 - 10.0 * move_factor
    } else {
        55.0
    };

    let o = obv(&closes[n - 10..], &volumes[n - 10..]);
    let (obv_slope, price_slope) = (linear_slope(&o), linear_slope(&closes[n - 10..]));
    let obv_score = match (obv_slope > 0.0, price_slope > 0.0) {
        (true, false) => 72.0,
        (true, true) => 63.0,
        (false, true) => 33.0,
        (false, false) => 42.0,
    };

    let ratio_label = if ratio > 1.5 {
        "High"
    } else if ratio < 0.7 {
        "Low"
    } else {
        "Normal"
    };
    let obv_label = if obv_score >= 65.0 {
        "bullish"
    } else if obv_score >= 50.0 {
        "neutral"
    } else {
        "bearish"
    };
    Some((
        format!("RVOL {:.1}x ({ratio_label}), OBV {obv_label}", ratio),
        (ratio_score * 0.5 + obv_score * 0.5).round(),
    ))
}

fn analyst_signal(a: Option<&Analysts>) -> Rule {
    let a = a?;
    let total = a.strong_buy + a.buy + a.hold + a.sell + a.strong_sell;
    if total == 0 {
        return None;
    }
    let weighted_buy = a.strong_buy as f64 * 1.5 + a.buy as f64;
    let weighted_total = weighted_buy + (a.hold + a.sell) as f64 + a.strong_sell as f64 * 1.5;
    let pct = weighted_buy / weighted_total * 100.0;
    Some((
        format!("{:.0}% buy consensus ({total} analysts)", pct.round()),
        pct,
    ))
}

fn earnings_signal(earnings: &[Earning]) -> Rule {
    let (mut sum, mut weights) = (0.0, 0.0);
    for (i, e) in earnings.iter().take(4).enumerate() {
        if e.estimate == 0.0 {
            continue;
        }
        let surprise = clamp(
            (e.actual - e.estimate) / e.estimate.abs() * 100.0,
            -25.0,
            25.0,
        );
        let w = if i == 0 { 2.0 } else { 1.0 };
        sum += surprise * w;
        weights += w;
    }
    if weights == 0.0 {
        return None;
    }
    let avg = sum / weights;
    Some((
        format!("{}% avg EPS surprise", signed(avg)),
        50.0 + avg * 2.0,
    ))
}

fn news_signal(news: &[NewsItem]) -> Rule {
    if news.is_empty() {
        return None;
    }
    let avg = news
        .iter()
        .map(|n| headline_sentiment(&n.title))
        .sum::<f64>()
        / news.len() as f64;
    let mood = if avg > 0.15 {
        "Positive"
    } else if avg < -0.15 {
        "Negative"
    } else {
        "Neutral"
    };
    let plural = if news.len() == 1 {
        "article"
    } else {
        "articles"
    };
    Some((
        format!("{mood} ({} {plural})", news.len()),
        50.0 + avg * 50.0,
    ))
}

const POSITIVE: &[&str] = &[
    "beat",
    "beats",
    "surge",
    "surges",
    "soar",
    "soars",
    "jump",
    "jumps",
    "rally",
    "rallies",
    "record",
    "upgrade",
    "upgraded",
    "strong",
    "growth",
    "gain",
    "gains",
    "rise",
    "rises",
    "boost",
    "boosts",
    "buy",
    "buying",
    "outperform",
    "profit",
    "deal",
    "wins",
    "win",
    "bullish",
    "higher",
];
const NEGATIVE: &[&str] = &[
    "miss",
    "misses",
    "plunge",
    "plunges",
    "drop",
    "drops",
    "fall",
    "falls",
    "slump",
    "slumps",
    "lawsuit",
    "sue",
    "sues",
    "cut",
    "cuts",
    "downgrade",
    "downgraded",
    "weak",
    "loss",
    "losses",
    "layoffs",
    "probe",
    "decline",
    "declines",
    "sell",
    "selling",
    "bearish",
    "lower",
    "warning",
    "crash",
];

/// Naive keyword sentiment of a headline, in [-1, 1].
pub fn headline_sentiment(title: &str) -> f64 {
    let (mut pos, mut neg) = (0.0, 0.0);
    for word in title
        .split(|c: char| !c.is_alphanumeric())
        .map(str::to_lowercase)
    {
        if POSITIVE.contains(&word.as_str()) {
            pos += 1.0;
        } else if NEGATIVE.contains(&word.as_str()) {
            neg += 1.0;
        }
    }
    if pos + neg == 0.0 {
        0.0
    } else {
        (pos - neg) / (pos + neg)
    }
}

fn pe_signal(pe: Option<f64>) -> Rule {
    let pe = pe.filter(|v| v.is_finite())?;
    if pe <= 0.0 {
        return Some(("Unprofitable".into(), 20.0));
    }
    let score = if pe < 10.0 {
        90.0
    } else if pe <= 25.0 {
        90.0 - (pe - 10.0) * 2.0
    } else if pe <= 50.0 {
        60.0 - (pe - 25.0) * 1.2
    } else {
        30.0 - (pe - 50.0) * 0.5
    };
    let label = if pe < 15.0 {
        "Undervalued"
    } else if pe < 25.0 {
        "Fair value"
    } else if pe < 40.0 {
        "Elevated"
    } else {
        "Expensive"
    };
    Some((label.into(), score))
}

fn roe_signal(roe: Option<f64>) -> Rule {
    let roe = roe.filter(|v| v.is_finite())?;
    if roe < 0.0 {
        return Some(("Negative ROE".into(), 0.0));
    }
    let score = if roe <= 30.0 {
        roe * 2.5
    } else {
        75.0 + (roe.sqrt() - 30f64.sqrt()) * 4.0
    };
    let label = if roe >= 30.0 {
        "Exceptional"
    } else if roe >= 15.0 {
        "Good"
    } else if roe >= 8.0 {
        "Adequate"
    } else {
        "Below average"
    };
    Some((label.into(), score))
}

fn de_signal(de: Option<f64>) -> Rule {
    let de = de.filter(|v| v.is_finite())?;
    let (score, label) = if de < 0.0 {
        (95.0, "Net cash (no debt)")
    } else if de <= 0.5 {
        (100.0 - de * 40.0, "Low")
    } else if de <= 1.5 {
        (80.0 - (de - 0.5) * 30.0, "Moderate")
    } else if de <= 3.0 {
        (50.0 - (de - 1.5) * 20.0, "High")
    } else {
        (0.0, "Very high")
    };
    Some((label.into(), score))
}

fn growth_signal(growth: Option<f64>, kind: &str) -> Rule {
    let g = growth.filter(|v| v.is_finite())?;
    let score = 50.0 + g.signum() * g.abs().sqrt() * 5.0;
    let label = if g >= 20.0 {
        format!("Strong {kind} growth")
    } else if g >= 0.0 {
        format!("Positive {kind} growth")
    } else {
        format!("Declining {kind}")
    };
    Some((label, score))
}

fn pb_signal(pb: Option<f64>) -> Rule {
    let pb = pb.filter(|v| v.is_finite() && *v > 0.0)?;
    let (score, label) = if pb < 1.0 {
        (90.0, "Undervalued")
    } else if pb <= 3.0 {
        (
            90.0 - (pb - 1.0) * 20.0,
            if pb < 3.0 { "Fair" } else { "High" },
        )
    } else if pb <= 10.0 {
        (
            50.0 - (pb - 3.0) * 5.0,
            if pb < 10.0 { "High" } else { "Very high" },
        )
    } else {
        (15.0, "Very high")
    };
    Some((label.into(), score))
}

fn sma200_signal(closes: &[f64], price: Option<f64>) -> Rule {
    let s = sma(closes, 200)?;
    let pct = (price? - s) / s * 100.0;
    Some((format!("{}% vs 200d SMA", signed(pct)), 50.0 + pct * 1.5))
}
