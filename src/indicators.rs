//! Technical indicators over daily closes (oldest first).
//! Ported from finnguide's scorer.js so values match common charting tools.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Macd {
    pub line: f64,
    pub signal: f64,
    pub histogram: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bands {
    pub upper: f64,
    pub mid: f64,
    pub lower: f64,
}

pub fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    v.max(lo).min(hi)
}

pub fn sma(c: &[f64], period: usize) -> Option<f64> {
    if period == 0 || c.len() < period {
        return None;
    }
    Some(c[c.len() - period..].iter().sum::<f64>() / period as f64)
}

/// EMA seeded with the SMA of the first `period` values; one value per bar from `period - 1`.
pub fn ema_series(c: &[f64], period: usize) -> Vec<f64> {
    if period == 0 || c.len() < period {
        return Vec::new();
    }
    let k = 2.0 / (period as f64 + 1.0);
    let mut ema = c[..period].iter().sum::<f64>() / period as f64;
    let mut out = vec![ema];
    for v in &c[period..] {
        ema = v * k + ema * (1.0 - k);
        out.push(ema);
    }
    out
}

/// Wilder-smoothed RSI.
pub fn rsi(c: &[f64], period: usize) -> Option<f64> {
    if period == 0 || c.len() < period + 1 {
        return None;
    }
    let (mut gain, mut loss) = (0.0, 0.0);
    for i in 1..=period {
        let d = c[i] - c[i - 1];
        if d > 0.0 {
            gain += d;
        } else {
            loss -= d;
        }
    }
    gain /= period as f64;
    loss /= period as f64;
    let p = period as f64;
    for i in period + 1..c.len() {
        let d = c[i] - c[i - 1];
        gain = (gain * (p - 1.0) + d.max(0.0)) / p;
        loss = (loss * (p - 1.0) + (-d).max(0.0)) / p;
    }
    if loss == 0.0 {
        return Some(100.0);
    }
    Some(100.0 - 100.0 / (1.0 + gain / loss))
}

/// MACD 12/26/9. Needs at least 34 closes (26 for the slow EMA + 9 for the signal - 1).
pub fn macd(c: &[f64]) -> Option<Macd> {
    let ema12 = ema_series(c, 12);
    let ema26 = ema_series(c, 26);
    // ema26[i] is bar 25 + i; the matching ema12 value is at index 14 + i.
    let series: Vec<f64> = ema26
        .iter()
        .enumerate()
        .map(|(i, v)| ema12[14 + i] - v)
        .collect();
    let signal = ema_series(&series, 9);
    let (line, signal) = (*series.last()?, *signal.last()?);
    Some(Macd {
        line,
        signal,
        histogram: line - signal,
    })
}

/// Bollinger bands at 2 population standard deviations.
pub fn bollinger(c: &[f64], period: usize) -> Option<Bands> {
    let mid = sma(c, period)?;
    let var = c[c.len() - period..]
        .iter()
        .map(|v| (v - mid).powi(2))
        .sum::<f64>()
        / period as f64;
    let sd = var.sqrt();
    Some(Bands {
        upper: mid + 2.0 * sd,
        mid,
        lower: mid - 2.0 * sd,
    })
}

/// On-balance volume. Empty when the inputs are too short or mismatched.
pub fn obv(closes: &[f64], volumes: &[f64]) -> Vec<f64> {
    if closes.len() < 2 || closes.len() != volumes.len() {
        return Vec::new();
    }
    let mut out = vec![0.0];
    for i in 1..closes.len() {
        let prev = out[i - 1];
        out.push(if closes[i] > closes[i - 1] {
            prev + volumes[i]
        } else if closes[i] < closes[i - 1] {
            prev - volumes[i]
        } else {
            prev
        });
    }
    out
}

/// Least-squares slope normalised by the series mean, so it is scale-independent.
pub fn linear_slope(v: &[f64]) -> f64 {
    let n = v.len();
    if n < 2 {
        return 0.0;
    }
    let mean = v.iter().sum::<f64>() / n as f64;
    if mean == 0.0 {
        return 0.0;
    }
    let mid = (n as f64 - 1.0) / 2.0;
    let (mut num, mut den) = (0.0, 0.0);
    for (i, x) in v.iter().enumerate() {
        let dx = i as f64 - mid;
        num += dx * (x - mean);
        den += dx * dx;
    }
    if den == 0.0 {
        0.0
    } else {
        num / den / mean.abs()
    }
}
