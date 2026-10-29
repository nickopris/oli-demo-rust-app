# OLI Rust Demo App Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A single-page Rust stock-card app (Yahoo live data with an embedded timestamped snapshot fallback, investing tips from Drupal JSON:API) plus the `.drupalaibp/` config that makes One Line Installer bring it up next to Drupal CMS.

**Architecture:** One Cargo crate (`oli-demo-rust-app`), library + two binaries: the axum server (`src/main.rs`) and `refresh-data` (`src/bin/refresh-data.rs`) which rewrites `data/snapshot.json`. The snapshot and the HTML page are compiled in with `include_str!`. OLI hooks write a DDEV daemon + exposed-port config before the first start, download a prebuilt musl binary from GitHub Releases after Drupal is scaffolded, and create the `investing_tip` content type + 5 tips after JSON:API is enabled.

**Tech Stack:** Rust stable (2021 edition), axum 0.8, tokio 1, serde/serde_json 1, reqwest 0.12 (`default-features = false`, features `json`, `rustls-tls`, `cookies`), anyhow 1; dev: tower 0.5 (`util`), http-body-util 0.1. Bash hook scripts, Drupal 11 config YAML, GitHub Actions + cargo-zigbuild.

**Spec:** `docs/superpowers/specs/2026-10-28-oli-rust-demo-design.md` (mockup: `docs/mockup.png`). Read both before starting any task.

## Global Constraints

- Curated tickers, exactly these 10, in this order: `DIS, AAPL, MSFT, NVDA, TSLA, KO, AMZN, NFLX, GOOGL, META`. Search and the band are limited to them.
- No API keys, no `.env`, no OLI `questions`. No network access in `build.rs` (there is no `build.rs`).
- Card shows only a date/time for data freshness, formatted like `28 Oct 2026, 14:05 UTC`. Never the words "live" or "cached".
- Tip fallback is labelled `offline` (subtle); the card never fails because of Drupal.
- No overall score anywhere; per-signal strength 0–100 drives only bar width/colour.
- Timeouts: Yahoo per lookup 2.5 s; Drupal tips 1 s. Live cache TTL 10 min. Backoff after a Yahoo failure 60 s.
- Server listens on `0.0.0.0:${PORT:-3000}`. DDEV router: container port 3000 → HTTPS 8443 / HTTP 8080.
- Env: `DRUPAL_BASE_URL` (default `http://localhost`), `DRUPAL_PUBLIC_URL` (default: value of `DDEV_PRIMARY_URL`, else same as base).
- Release assets: `oli-demo-rust-app-linux-amd64`, `oli-demo-rust-app-linux-arm64` (musl, static). Download URL `https://github.com/nickopris/oli-demo-rust-app/releases/latest/download/<asset>`.
- Binary path inside the project: `.ddev/rust-demo/bin/oli-demo-rust-app` (gitignored).
- Public repo `github.com/nickopris/oli-demo-rust-app`: never commit secrets, `.env*`, tokens or personal data. `.gitignore` covers `.env*`, `/target`, `.ddev/rust-demo/bin/`.
- All HTML/CSS/JS inline in one embedded file; no CDN, no external fonts.
- Third-party text (Yahoo headlines, Drupal tips) is rendered with `textContent`, never `innerHTML`.

## Review Focus

1. **Everyday names that aren't in Yahoo's company name** ("Google" → Alphabet, "Facebook" → Meta) — must find the ticker. Pinned by aliases + test in Task 2.
2. **Yahoo returning gaps** (null closes in the chart, missing quoteSummary modules, no dividends for TSLA/AMZN) — card shows `N/A`, never a parse failure for the whole ticker. Pinned in Task 5 (null-close test) and Task 4 (short/empty series test).
3. **Yahoo or Drupal hanging rather than erroring** — the request must still return within the timeout plus a margin. Pinned in Tasks 6 and 7.
4. **Markup in third-party text** (`<b>` or `<script>` in a headline or tip) — shown as literal text. Pinned by a manual check in Task 9.
5. **Messy input** (`" dis "`, `"DIS"`, `"dis"`, empty, 200-char strings, `/api/company/%3Cx%3E`) — trimmed/case-insensitive match or a friendly not-found, never a 500. Pinned in Tasks 2 and 8.

---

## File Structure

```
Cargo.toml / Cargo.lock
src/lib.rs            module declarations only
src/model.rs          Snapshot, Company, Bar, Fundamentals, Analysts, Earning, Dividend, NewsItem, Quote
src/tickers.rs        CURATED list (+ aliases), search()
src/snapshot.rs       embedded() -> &'static Snapshot
src/indicators.rs     sma, ema_series, rsi, macd, bollinger, obv, linear_slope
src/signals.rs        Signal, Card, build_card(), per-signal rules, headline sentiment
src/yahoo.rs          YahooClient + pure parse_* functions
src/market.rs         LiveSource trait, MarketData (live → snapshot fallback, TTL, backoff)
src/tips.rs           Tip, TipSource (JSON:API, fallback tips)
src/web.rs            AppState, router(), handlers
src/main.rs           env config, wiring, serve
src/bin/refresh-data.rs
assets/index.html     the whole UI
data/snapshot.json    committed snapshot
tests/fixtures/...    captured Yahoo + hand-written JSON:API fixtures
.drupalaibp/config.json, *.sh, seed-tips.php, tip-config/*.yml
.github/workflows/ci.yml, release.yml
README.md, .gitignore
```

---

### Task 1: Toolchain, crate scaffold, health endpoint

**Files:**
- Create: `Cargo.toml`, `src/lib.rs`, `src/web.rs`, `src/main.rs`, `.gitignore`, `tests/web_health.rs`

**Interfaces:**
- Produces: `web::router(state: AppState<S>) -> axum::Router` (AppState grows in Task 8; here a placeholder unit-like state is fine as long as `router` keeps this name), `GET /health` → `200 "ok"`.

- [ ] **Step 1: Docker toolchain** — the user does not want Rust on the host OS. Create `scripts/cargo` (bash, executable): `docker run --rm -it?` (use `-t` only when stdin is a TTY) `-v "$PWD":/app -w /app -v oli-demo-cargo:/usr/local/cargo/registry -v oli-demo-target:/app/target -e CARGO_TERM_COLOR=never ${PORT_FLAGS} rust:1-bookworm cargo "$@"`, where `PORT_FLAGS=-p 3000:3000` when the first arg is `run`. Every `cargo …` command in this plan means `scripts/cargo …`. Shellcheck runs as `docker run --rm -v "$PWD":/mnt -w /mnt koalaman/shellcheck:stable …`. Expected: `scripts/cargo --version` prints a stable version.
- [ ] **Step 2: `git init -b main`** in the project root; create `.gitignore` with `/target`, `.env*`, `.ddev/rust-demo/bin/`, `.DS_Store`.
- [ ] **Step 3: Create the crate** (`Cargo.toml` with the Tech Stack deps, `[[bin]] name = "oli-demo-rust-app" path = "src/main.rs"`, `[profile.release] lto = true, strip = true`).
- [ ] **Step 4: Write the failing test** `tests/web_health.rs::health_returns_ok`: build the router, `oneshot(GET /health)`, assert status 200 and body `"ok"`.
- [ ] **Step 5:** `cargo test --test web_health` → FAIL (router missing).
- [ ] **Step 6: Implement** `router` with `/health`; `main.rs` binds `0.0.0.0:${PORT:-3000}` and serves.
- [ ] **Step 7:** `cargo test` → PASS; `cargo run` then `curl localhost:3000/health` → `ok`.
- [ ] **Step 8: Commit** `chore: scaffold crate with health endpoint`.

### Task 2: Model, embedded snapshot, curated tickers, search

**Files:**
- Create: `src/model.rs`, `src/tickers.rs`, `src/snapshot.rs`, `data/snapshot.json`, `tests/search.rs`

**Interfaces:**
- Produces (all `#[derive(Debug, Clone, Serialize, Deserialize)]`, `Option` fields `#[serde(default)]`):
  - `Snapshot { fetched_at: i64 /*unix secs UTC*/, companies: Vec<Company> }`
  - `Company { ticker: String, name: String, sector: Option<String>, exchange: Option<String>, price: Option<f64>, market_cap: Option<f64>, bars: Vec<Bar>, fundamentals: Fundamentals, analysts: Option<Analysts>, earnings: Vec<Earning>, dividend: Dividend, news: Vec<NewsItem> }`
  - `Bar { t: i64, close: f64, volume: f64 }` (oldest first)
  - `Fundamentals { pe, roe_pct, debt_to_equity, eps_growth_pct, revenue_growth_pct, pb: Option<f64> }` (ROE/growth in percent, D/E as a ratio)
  - `Analysts { strong_buy, buy, hold, sell, strong_sell: u32 }`
  - `Earning { actual: f64, estimate: f64 }` (most recent first)
  - `Dividend { yield_pct, rate_annual, growth_5y_pct, payout_ratio_pct: Option<f64>, ex_date, pay_date: Option<i64> }`
  - `NewsItem { title: String, publisher: Option<String>, link: Option<String>, published_at: i64 }`
  - `Quote { ticker: String, price: Option<f64>, change_pct: Option<f64> }`
  - `tickers::CURATED: [(&str /*ticker*/, &[&str] /*aliases*/); 10]` — aliases: GOOGL `["google"]`, META `["facebook"]`, others `[]`.
  - `tickers::search(snapshot: &Snapshot, q: &str) -> Vec<(String /*ticker*/, String /*name*/)>` — trim, case-insensitive; order: exact ticker, ticker prefix, name/alias substring; empty query → empty vec; max 10.
  - `snapshot::embedded() -> &'static Snapshot` (parse `include_str!("../data/snapshot.json")` once via `OnceLock`).
- `data/snapshot.json` here is a hand-made placeholder: 10 companies with real names (e.g. "The Walt Disney Company", "Alphabet Inc.", "Meta Platforms, Inc."), plausible price/market cap, ≥ 15 bars each. Task 5 replaces it with real data.

- [ ] **Step 1: Write failing tests** in `tests/search.rs`:
  - `search("disney")[0].0 == "DIS"`; `search("disn")` contains DIS; `search(" dis ")[0].0 == "DIS"`; `search("DIS")[0].0 == "DIS"`
  - `search("google")[0].0 == "GOOGL"`; `search("facebook")[0].0 == "META"`
  - `search("")` and `search(&"x".repeat(200))` are empty
  - `embedded().companies.len() == 10` and tickers equal `CURATED` order
- [ ] **Step 2:** `cargo test --test search` → FAIL.
- [ ] **Step 3: Implement** model, tickers, snapshot loader, placeholder JSON.
- [ ] **Step 4:** `cargo test` → PASS.
- [ ] **Step 5: Commit** `feat: data model, embedded snapshot and ticker search`.

### Task 3: Technical indicators

**Files:**
- Create: `src/indicators.rs`, `tests/indicators.rs`

**Interfaces:**
- Produces (all on `&[f64]`, oldest first):
  `sma(c, period) -> Option<f64>`, `ema_series(c, period) -> Vec<f64>`, `rsi(c, period) -> Option<f64>` (Wilder, needs `period + 1` values), `macd(c) -> Option<Macd { line, signal, histogram: f64 }>` (12/26/9, needs ≥ 34 values), `bollinger(c, period) -> Option<Bands { upper, mid, lower: f64 }>` (2σ population std), `obv(closes, volumes) -> Vec<f64>`, `linear_slope(v) -> f64`, `clamp(v, lo, hi) -> f64`.
- Port from finnguide's `src/scorer.js` (the author's private app) lines 5–103 (read-only reference).

- [ ] **Step 1: Write failing tests:**
  - `sma(&[1.,2.,3.,4.,5.], 5) == Some(3.0)`; `sma(.., 6) == None`
  - `rsi` of 30 strictly increasing closes `== Some(100.0)`; strictly decreasing `== Some(0.0)`; 14 closes → `None`
  - `macd` of 34 constant closes → `Some` with all three ≈ 0 (1e-9); 33 closes → `None`
  - `bollinger` of 20 constant closes → upper == mid == lower
  - `obv(&[1.,2.,2.,1.], &[10.,20.,30.,40.]) == vec![0.,20.,20.,-20.]`
  - `linear_slope(&[1.,2.,3.]) == 1.0`
- [ ] **Step 2:** `cargo test --test indicators` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4:** `cargo test` → PASS.
- [ ] **Step 5: Commit** `feat: technical indicators`.

### Task 4: Signals and card builder

**Files:**
- Create: `src/signals.rs`, `tests/signals.rs`

**Interfaces:**
- Consumes: model (Task 2), indicators (Task 3).
- Produces:
  - `Signal { name: String, label: String, strength: u8, available: bool }` (N/A ⇒ `label "N/A"`, `strength 50`, `available false`)
  - `Card { ticker, name: String, sector, exchange: Option<String>, price, market_cap, change_pct: Option<f64>, spark: Vec<f64> /*last 14 closes*/, as_of: i64, momentum: Vec<Signal>, fundamentals: Vec<Signal>, dividend: Dividend, news: Vec<NewsItem> /*max 3*/ }` (Serialize)
  - `build_card(c: &Company, as_of: i64) -> Card`
  - `headline_sentiment(title: &str) -> f64` in [-1, 1] (small positive/negative word lists, e.g. beat/surge/record/upgrade/strong vs miss/plunge/lawsuit/cut/downgrade/weak)
- Rules: port `scoreRSI, scoreMACD, scoreSMAPosition, scoreBollinger, scoreVolume, scoreAnalystSentiment, scoreEarningsSurprise, scorePE, scoreROE, scoreDebtEquity, scoreGrowth, scorePB, scoreSMA200Position` from `scorer.js` (lines 105–153, 172–279, 318–383), keeping their labels. Price for SMA/Bollinger = `c.price` or last close. News: average `headline_sentiment` over the news items, strength `50 + avg*50`, label `Positive|Neutral|Negative (N articles)` with ±0.15 thresholds. `change_pct` = last close vs previous close.
- Exact row names, in order — momentum: `RSI (14)`, `MACD`, `Price vs 50d SMA`, `Bollinger Band`, `Volume`, `Analyst Sentiment`, `Earnings Surprise`, `News Sentiment`; fundamentals: `P/E Ratio`, `ROE`, `Debt/Equity`, `EPS Growth`, `Revenue Growth`, `P/B Ratio`, `Price vs 200d SMA`. ("EPS Growth"/"Revenue Growth" drop the 5Y/3Y suffix because Yahoo only provides year-on-year figures.)

- [ ] **Step 1: Write failing tests:**
  - P/E 20 → label `Fair value`, strength 70; P/E None → `N/A`, `available false`; P/E -5 → `Unprofitable`, 20
  - ROE 20 → `Good`, 50; D/E 0.3 → `Low`, 88; P/B 2 → `Fair`, 70; EPS growth 25 → `Strong EPS growth`
  - Analysts {10, 20, 9, 0, 0} → label `80% buy consensus (39 analysts)`
  - Earnings `[{actual 1.1, estimate 1.0}]` → label `+10.0% avg EPS surprise`, strength 70
  - 50 closes at 100.0 with price 102.0 → `Price vs 50d SMA` label `+2.0% vs 50d SMA`
  - `headline_sentiment("Disney beats estimates as shares surge") > 0`; `headline_sentiment("Shares plunge after lawsuit") < 0`
  - `build_card` on a 260-bar company: `spark.len() == 14`, row names exactly as listed above
  - `build_card` on a company with `bars: vec![]`, no analysts, no earnings, no news: does not panic; every momentum signal `available == false`; `spark` empty
- [ ] **Step 2:** `cargo test --test signals` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4:** `cargo test` → PASS.
- [ ] **Step 5: Commit** `feat: signal rules and card builder`.

### Task 5: Yahoo client, refresh-data, real snapshot

**Files:**
- Create: `src/yahoo.rs`, `src/bin/refresh-data.rs`, `tests/yahoo_parse.rs`, `tests/fixtures/yahoo/{chart_dis.json, chart_nulls.json, summary_dis.json, search_dis.json}`
- Modify: `data/snapshot.json` (regenerated)

**Interfaces:**
- Produces:
  - `parse_chart(v: &serde_json::Value) -> anyhow::Result<ChartData { bars: Vec<Bar>, price: Option<f64>, exchange: Option<String> }>` — skips entries whose close or volume is null
  - `parse_summary(v: &Value, c: &mut Company) -> anyhow::Result<()>` — fills name, sector, market_cap, fundamentals, analysts, earnings, dividend; absent ⇒ `None`
  - `parse_news(v: &Value) -> Vec<NewsItem>` — max 3
  - `YahooClient::new() -> Self`; `async fn company(&self, ticker: &str) -> anyhow::Result<Company>`; `async fn quote(&self, ticker: &str) -> anyhow::Result<Quote>` (chart only, no crumb)
- Endpoints (browser-like `User-Agent` header required):
  - chart: `https://query1.finance.yahoo.com/v8/finance/chart/{T}?range=1y&interval=1d`
  - crumb: `GET https://fc.yahoo.com` (sets cookie, non-2xx is fine) then `GET https://query1.finance.yahoo.com/v1/test/getcrumb`; cache the crumb, refetch once on 401
  - summary: `https://query2.finance.yahoo.com/v10/finance/quoteSummary/{T}?modules=price,summaryProfile,summaryDetail,financialData,defaultKeyStatistics,recommendationTrend,earningsHistory,calendarEvents&crumb={crumb}`
  - news: `https://query1.finance.yahoo.com/v1/finance/search?q={T}&newsCount=3&quotesCount=0`
- Unit mapping (Yahoo `.raw` values): `financialData.returnOnEquity`, `earningsGrowth`, `revenueGrowth` ×100; `debtToEquity` ÷100; `summaryDetail.dividendYield`, `payoutRatio` ×100; `dividendRate` as is; `defaultKeyStatistics.priceToBook`; `summaryDetail.trailingPE`; `exDividendDate`, `calendarEvents.dividendDate` as unix secs; `recommendationTrend.trend[0]`; `earningsHistory.history` (reverse to most recent first, `epsActual`/`epsEstimate`). `growth_5y_pct` has no Yahoo source: leave `None` (shows N/A).
- `refresh-data`: fetch `company()` for each curated ticker in order, ~1 s apart; if every ticker succeeds, write `data/snapshot.json` (pretty) with `fetched_at = now`; if any ticker fails, print the error, exit 1, and leave the file untouched.

- [ ] **Step 1: Capture fixtures** with curl (chart, search) and a cookie jar plus crumb (summary) for DIS; hand-edit `chart_nulls.json` from `chart_dis.json` so three closes are `null`. Check that the fixtures hold no cookies or crumbs.
- [ ] **Step 2: Write failing tests:** `parse_chart(chart_dis)` → ≥ 200 bars, all finite; `parse_chart(chart_nulls)` → 3 fewer bars, no error; `parse_summary(summary_dis)` → name `The Walt Disney Company`, `roe_pct` in (0, 100), analysts present; `parse_summary(&json!({}))` → Ok with all fields None; `parse_news(search_dis).len() <= 3` and titles non-empty.
- [ ] **Step 3:** `cargo test --test yahoo_parse` → FAIL.
- [ ] **Step 4: Implement** the parsers, client and `refresh-data`.
- [ ] **Step 5:** `cargo test` → PASS.
- [ ] **Step 6:** `cargo run --bin refresh-data` → exit 0; `data/snapshot.json` has 10 companies, `fetched_at` ≈ now, DIS ≥ 200 bars. Re-run `cargo test` (the search tests must still pass on real names).
- [ ] **Step 7: Commit** `feat: Yahoo client and refresh-data; real snapshot`.

### Task 6: Live data with snapshot fallback

**Files:**
- Create: `src/market.rs`, `tests/market.rs`

**Interfaces:**
- Consumes: `YahooClient` (Task 5), `embedded()` (Task 2).
- Produces:
  - `trait LiveSource: Send + Sync + 'static { fn company(&self, t: &str) -> impl Future<Output = anyhow::Result<Company>> + Send; fn quote(&self, t: &str) -> impl Future<Output = anyhow::Result<Quote>> + Send; }`, implemented for `YahooClient`
  - `MarketConfig { timeout: Duration, ttl: Duration, backoff: Duration }` with `Default` = 2.5 s / 600 s / 60 s
  - `MarketData<S: LiveSource>::new(source: S, snapshot: &'static Snapshot, cfg: MarketConfig)`
  - `async fn company(&self, ticker: &str) -> Option<(Company, i64 /*as_of*/)>` — `None` only for non-curated tickers; ticker trimmed and uppercased
  - `async fn quotes(&self) -> Vec<Quote>` — all 10 concurrently; per-ticker fallback to snapshot values; honours backoff
- Behaviour: fresh cache hit → cached; else if within backoff → snapshot; else live under `timeout`; success → cache with `as_of = now`; failure or timeout → start backoff, return snapshot company with `as_of = snapshot.fetched_at`.

- [ ] **Step 1: Write failing tests** with a `FakeSource` (call counter, mode: Ok / Err / Sleep(5 s)), using a 100 ms timeout, 1 s TTL and 300 ms backoff:
  - Ok → `as_of > snapshot.fetched_at` and the company is the fake's
  - Err → `as_of == snapshot.fetched_at`
  - Sleep → returns the snapshot within 300 ms
  - two calls inside TTL → counter 1
  - Err, then an immediate second call → counter 1; after a 350 ms sleep → counter 2
  - `"ZZZZ"` → `None`, counter 0; `"dis"` → `Some`
- [ ] **Step 2:** `cargo test --test market` → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4:** `cargo test` → PASS.
- [ ] **Step 5: Commit** `feat: live market data with snapshot fallback`.

### Task 7: Drupal tips

**Files:**
- Create: `src/tips.rs`, `tests/tips.rs`, `tests/fixtures/jsonapi_tips.json` (hand-written JSON:API `data[]` with `attributes.title`, `attributes.field_tip_text`, `attributes.drupal_internal__nid`)

**Interfaces:**
- Produces:
  - `Tip { title: String, text: String, url: Option<String>, offline: bool }` (Serialize)
  - `parse_tips(v: &Value, public_url: &str) -> Vec<Tip>` — url `{public_url without trailing /}/node/{nid}`
  - `TipSource::new(base_url: String, public_url: String, timeout: Duration)`; `async fn random_tip(&self) -> Tip`
  - `FALLBACK_TIPS: [(&str, &str); 5]` — the same five tips the Drupal seed creates (Task 10), returned with `offline: true, url: None`
- Request: `GET {base}/jsonapi/node/investing_tip?filter[status]=1&fields[node--investing_tip]=title,field_tip_text,drupal_internal__nid`, `Accept: application/vnd.api+json`. Random pick: `SystemTime` nanos modulo the length (no `rand` dependency).
- Tip copy (exact; title — text):
  1. `Spread your risk` — `Diversify across companies, sectors and regions so one bad bet can't sink your portfolio.`
  2. `Time in the market` — `Staying invested through ups and downs has historically beaten trying to time the perfect entry.`
  3. `Mind the fees` — `A 1% yearly fee compounds too. Low-cost index funds keep more of your returns working for you.`
  4. `Emergency fund first` — `Keep three to six months of expenses in cash before investing, so you never have to sell in a downturn.`
  5. `Don't chase hype` — `If everyone is talking about a stock, the easy gains are usually gone. Research before you buy.`

- [ ] **Step 1: Write failing tests:** `parse_tips(fixture, "https://demo.ddev.site/")` → first url `https://demo.ddev.site/node/7` (matching the fixture's nid); `parse_tips(&json!({"data": []}), ..)` empty; `TipSource` at `http://127.0.0.1:9` with a 1 s timeout → `offline == true`, returns in < 1.5 s, text is one of `FALLBACK_TIPS`.
- [ ] **Step 2:** `cargo test --test tips` → FAIL.
- [ ] **Step 3: Implement** (an empty Drupal result also falls back).
- [ ] **Step 4:** `cargo test` → PASS.
- [ ] **Step 5: Commit** `feat: investing tips from Drupal JSON:API with fallback`.

### Task 8: HTTP API

**Files:**
- Modify: `src/web.rs`, `src/main.rs`
- Create: `tests/web_api.rs`

**Interfaces:**
- Consumes: `MarketData` (Task 6), `TipSource` (Task 7), `build_card` (Task 4), `search` (Task 2).
- Produces: `AppState<S> { market: Arc<MarketData<S>>, tips: Arc<TipSource> }`, `router<S: LiveSource>(state) -> Router`. Routes:
  - `GET /` → `assets/index.html` (`include_str!`, `text/html; charset=utf-8`)
  - `GET /health` → `ok`
  - `GET /api/tickers` → `[{ticker, name, price, change_pct}]` from the snapshot (instant)
  - `GET /api/tickers/live` → `MarketData::quotes()`
  - `GET /api/search?q=` → `[{ticker, name}]`
  - `GET /api/company/{ticker}` → `200 {card: Card, tip: Tip}`, or `404 {"error": "not_found", "tickers": [..10..]}`
- `main.rs` reads `PORT`, `DRUPAL_BASE_URL`, `DRUPAL_PUBLIC_URL` (fallback `DDEV_PRIMARY_URL`, then base) and wires in `YahooClient`.

- [ ] **Step 1: Write failing tests** (FakeSource in Err mode, TipSource at a closed port):
  - `/api/tickers` → 10 items in CURATED order
  - `/api/search?q=disney` → first `DIS`; `/api/search?q=` → `[]`
  - `/api/company/dis` → 200, `card.ticker == "DIS"`, `card.as_of == snapshot.fetched_at`, `tip.offline == true`
  - `/api/company/ZZZZ` and `/api/company/%3Cx%3E` → 404 with 10 tickers
  - `/` → 200, content-type starts `text/html`
- [ ] **Step 2:** `cargo test --test web_api` → FAIL.
- [ ] **Step 3: Implement** (a minimal `assets/index.html` stub is enough for this task).
- [ ] **Step 4:** `cargo test` → PASS.
- [ ] **Step 5: Commit** `feat: HTTP API`.

### Task 9: The page

**Files:**
- Modify: `assets/index.html`

**Interfaces:**
- Consumes: the Task 8 routes and JSON shapes.

Build to `docs/mockup.png`: dark background, the same type scale and spacing, search input + GET DATA + RESET, a column header row (14 DAYS · M/CAP · PRICE), a header row (ticker in accent blue, name, `sector · exchange`, SVG sparkline green if up / red if down, market cap like `$190.7B`, price `$109.60`), the expanded panel with two signal columns (label coloured by strength: ≥ 60 green, ≤ 40 pink, otherwise neutral; a thin bar whose width = strength%), Dividends & Fundamentals (percent to 2 dp, dates like `30 Jun 2026`, N/A where null), and Recent News (title, publisher, relative age such as `20h ago`, `1d ago`). Additions beyond the mockup:
- **Ticker band** above the search: 10 chips (ticker, price, change% coloured). On load fill from `/api/tickers`, then update from `/api/tickers/live`. Clicking a chip loads that card.
- **Tip strip** directly under the header row: "Tip from Drupal" — title + text, linked to `url` when present, with a small muted `offline` tag when `offline`.
- **Data timestamp** on the card: `as_of` formatted `28 Oct 2026, 14:05 UTC` (`Intl.DateTimeFormat('en-GB', {timeZone: 'UTC', …})`).
- Type-ahead dropdown from `/api/search` (debounced ~150 ms; arrow keys + Enter); Enter / GET DATA loads the top match; RESET clears the input and the card.
- Not found: a friendly line ("Try one of these:") plus the 10 tickers as chips.
- All third-party strings go in through `textContent`.

- [ ] **Step 1: Implement the page.**
- [ ] **Step 2: Verify in a browser** (use the `run` skill or Claude in Chrome against `cargo run`): screenshot the DIS card and compare it side by side with `docs/mockup.png`; click a band chip; type `disn` and use the dropdown; search `facebook` → META; search `xyz` → friendly message; press RESET.
- [ ] **Step 3: Injection check:** temporarily change one fixture-backed headline or tip to `<b>x</b><script>alert(1)</script>` (or serve one from a stub) and confirm it renders as literal text. Revert.
- [ ] **Step 4: Offline check:** with networking blocked (or `DRUPAL_BASE_URL=http://127.0.0.1:9`), the card still renders from the snapshot with the snapshot timestamp and an `offline` tip.
- [ ] **Step 5: Commit** `feat: stock card UI`.

### Task 10: OLI config, hooks and Drupal content

**Files:**
- Create: `.drupalaibp/config.json`, `.drupalaibp/rust-demo-ddev.sh`, `.drupalaibp/fetch-binary.sh`, `.drupalaibp/setup-tips.sh`, `.drupalaibp/seed-tips.php`, `.drupalaibp/tip-config/{node.type.investing_tip.yml, field.storage.node.field_tip_text.yml, field.field.node.investing_tip.field_tip_text.yml, core.entity_form_display.node.investing_tip.default.yml, core.entity_view_display.node.investing_tip.default.yml}`

Do **not** name the YAML folder `config/`: OLI's own `import_config` step would also pick it up.

`config.json`:
```json
{
  "label": "Rust stock card demo",
  "description": "Drupal CMS plus a Rust web app in the same DDEV project. The Rust app shows stock cards (Yahoo Finance, with an embedded snapshot fallback) and pulls a random investing tip from Drupal's JSON:API.",
  "ddev_project_type": "drupal11",
  "is_drupal": true,
  "agents": ["claude"],
  "deny_commands": ["git push", "ssh"],
  "install_modules": ["jsonapi"],
  "launch_path": ":8443",
  "append_welcome_text": [
    "{BOLD}{GREEN}Rust stock card app:{RESET} {SITE_URL}:8443",
    "Drupal: {DRUPAL_LOGIN_URL} ({DRUPAL_USERNAME} / {DRUPAL_PASSWORD})",
    "Try: Disney, AAPL, Nvidia. Edit an Investing tip in Drupal, then search again to see it change."
  ],
  "hooks": {
    "before_ddev_start": [ { "script": "./rust-demo-ddev.sh", "weight": 0, "where": "host" } ],
    "create_drupal_site_done": [ { "script": "./fetch-binary.sh", "weight": 10, "where": "web" } ],
    "provision_from_config_done": [ { "script": "./setup-tips.sh", "weight": 10, "where": "web" } ]
  }
}
```
Before writing it, check `skills` against the OLI 2.0.x docs: leave the key out so the defaults apply, unless the docs say an omitted key means none.

Scripts (`#!/usr/bin/env bash`, `set -euo pipefail`, idempotent, header comment saying event/where like the OLI examples):
- **`rust-demo-ddev.sh`** (host, project root): writes `.ddev/config.rust-demo.yaml` with a `web_extra_daemons` entry `{name: rust-demo, command: /var/www/html/.ddev/rust-demo/run.sh, directory: /var/www/html}` and a `web_extra_exposed_ports` entry `{name: rust-demo, container_port: 3000, http_port: 8080, https_port: 8443}`. It also writes `.ddev/rust-demo/run.sh` (chmod +x), which loops `sleep 2` until `.ddev/rust-demo/bin/oli-demo-rust-app` is executable, then exports `DRUPAL_BASE_URL=${DRUPAL_BASE_URL:-http://localhost}` and `DRUPAL_PUBLIC_URL=${DRUPAL_PUBLIC_URL:-$DDEV_PRIMARY_URL}` and `exec`s the binary.
- **`fetch-binary.sh`** (web): `cd /var/www/html`; if the binary already exists, skip the download. Map `uname -m` (`x86_64` → amd64, `aarch64` → arm64), then `curl -fsSL --retry 2 --max-time 120` the release asset to a temporary file, chmod it and move it into place. On failure: install `build-essential` with `sudo apt-get` if `cc` is missing, install rustup minimal if `cargo` is missing, run `cargo build --release --locked` and copy `target/release/oli-demo-rust-app`. Finally poll `http://127.0.0.1:3000/health` for up to 30 s and print the result.
- **`setup-tips.sh`** (web): `cd /var/www/html`; `drush config:import --partial --source=/var/www/html/.drupalaibp/tip-config -y`; `drush role:perm:add anonymous 'access content'`; `drush php:script .drupalaibp/seed-tips.php`; `drush cr`.
- **`seed-tips.php`**: if no `investing_tip` nodes exist, create the five Task 7 tips (published, `field_tip_text` set). Otherwise print "tips already present".
- YAML: content type `investing_tip` (label `Investing tip`); field storage `field_tip_text` of type `string_long` (plain text, so JSON:API returns a string); a form display with the `string_textarea` widget and a view display with the `basic_string` formatter, so the field can be edited in admin. Omit `uuid` keys.

- [ ] **Step 1: Write the files.**
- [ ] **Step 2:** `shellcheck .drupalaibp/*.sh` → clean (install with `brew install shellcheck` if missing, after asking).
- [ ] **Step 3: Dry-run the host hook** in a scratch directory: `mkdir -p .ddev && bash rust-demo-ddev.sh`. Check the YAML parses (`ddev debug configyaml` comes later in Task 13; for now `python3 -c 'import yaml…'` or a visual check) and run it twice to confirm it's idempotent.
- [ ] **Step 4: Commit** `feat: OLI config, DDEV daemon and Drupal tip content`.

### Task 11: CI, release workflow, README

**Files:**
- Create: `.github/workflows/ci.yml`, `.github/workflows/release.yml`, `README.md`

- **ci.yml** (push/PR): `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `shellcheck .drupalaibp/*.sh`.
- **release.yml** (on `push: tags: ['v*']`, `permissions: contents: write`): checkout; stable toolchain plus targets `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`; install zig + `cargo-zigbuild`; `cargo run --bin refresh-data || git checkout -- data/snapshot.json`; `cargo zigbuild --release --target …` for both; rename the binaries to the two asset names; `gh release create "$GITHUB_REF_NAME" <assets> --generate-notes` using `GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}` only.
- **README.md:**
  - what this is (an OLI demo)
  - run locally (`cargo run`, then open http://localhost:3000)
  - `refresh-data`
  - env vars
  - running via OLI (`--config-url https://github.com/nickopris/oli-demo-rust-app`)
  - a sample umbrella `remote-configs.json` entry (`{"repo": "https://github.com/nickopris/oli-demo-rust-app", "config": "config", "label": "Rust stock card demo"}`)
  - how releases work
  - data disclaimer (Yahoo data is unofficial and not investment advice)

- [ ] **Step 1: Write the files.**
- [ ] **Step 2:** `cargo fmt --check && cargo clippy -- -D warnings && cargo test` → all clean locally.
- [ ] **Step 3: Commit** `ci: tests and musl release builds; README`.

### Task 12: Publish to GitHub

- [ ] **Step 1: Secret scan:** `git ls-files` (no `.env*`, no `target/`, no binaries); `git grep -nIiE '(api[_-]?key|secret|token|password|crumb|cookie|BEGIN [A-Z ]*PRIVATE)'`, and review each hit (the expected ones are docs and the `GITHUB_TOKEN` reference); grep the fixtures for `A3=` / `crumb`. Run `gitleaks detect` if it's available.
- [ ] **Step 2: Create and push:** `gh repo create nickopris/oli-demo-rust-app --public --source . --push` (the user approved a public repo).
- [ ] **Step 3:** Wait for CI to go green (`gh run watch`).
- [ ] **Step 4: Release:** `git tag v0.1.0 && git push origin v0.1.0`; wait for the release workflow; confirm `curl -fsSIL https://github.com/nickopris/oli-demo-rust-app/releases/latest/download/oli-demo-rust-app-linux-arm64` returns 200 and the same for amd64.
- [ ] **Step 5:** Download the amd64 asset, run `file` on it, and expect "statically linked".

### Task 13: End-to-end through One Line Installer

- [ ] **Step 1:** Ask the user before running, because it creates a DDEV project and takes several minutes. Run it in a fresh directory, e.g. a new `oli-rust-e2e` folder, using the 2.0.x branch one-liner from the OLI README with `--config-url https://github.com/nickopris/oli-demo-rust-app --yolo`.
- [ ] **Step 2: Verify:**
  - OLI output shows the fetch-binary step succeeding.
  - The browser opens `https://<project>.ddev.site:8443`.
  - The welcome text lists the app URL.
  - `curl -s https://<project>.ddev.site/jsonapi/node/investing_tip` returns 5 tips anonymously.
  - Searching `Disney` shows a card with a non-offline tip whose link opens the Drupal node.
  - Editing that tip in Drupal admin and searching again shows the new text.
  - `ddev restart` brings the app back without re-running hooks.
- [ ] **Step 3:** Record any OLI behaviour that differed from the spec's "Verified OLI behaviour" list. Fix it in the scripts, tag `v0.1.1` and re-run.
