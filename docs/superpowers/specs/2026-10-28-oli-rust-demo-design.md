# OLI Rust Demo App — Design

Date: 2026-10-28 · Owner: Nick Opris

## Intent

A small, polished Rust web app whose real job is to show off **One Line Installer
(OLI) 2.0** at a conference talk and in its recorded video. The talk is about OLI;
this app is the proof (mockup: `docs/mockup.png`) that an OLI config pulled in from a remote git repo can bring
in an arbitrary non-PHP application, built and served inside the same DDEV stack as
a Drupal CMS site — and that the two talk to each other.

The audience moment: one OLI command → Drupal CMS + a Rust app come up → the
browser opens on the Rust app → the presenter types "Disney" or "DIS" → a rich
stock card appears, including an investing tip fetched live from Drupal's JSON:API.

Reference implementation for look and data shape: the Node app at
`/Users/nickopris/Work/finnguide` (much larger; take inspiration only). Target look:
the mockup screenshot shared in the brainstorm (dark UI, search bar + GET DATA /
RESET, one expanded company card). OLI source reference: branch `2.0.x` of
`https://git.drupalcode.org/project/one_line_installer` (docs in
`docs/drupalaibp-json.md`, examples in `docs/examples/`).

## Goals

- Zero configuration: no API keys, no `.env` editing, no OLI questions. The OLI run
  is fully unattended.
- Rust app is reachable within seconds of DDEV starting — no on-stage compile.
- Live data when available, never dependent on it: Yahoo Finance (free, no key)
  is tried at runtime; on any failure the app serves an embedded, timestamped
  snapshot and says when it was fetched. A flaky free API failing mid-demo is the
  worst outcome — the user's explicit priority — so fallback must be fast and
  seamless.
- Looks good enough to sit on a projector while talking.
- Demonstrates Rust ↔ Drupal: Rust reads public, unauthenticated JSON:API.
- `cargo run` works standalone (outside OLI/DDEV) for development.

## Non-goals

- Any data provider other than Yahoo; any provider needing a key.
- Scores/ratings/overall verdicts, watchlists, accounts, lists, alerts, email,
  persistence of any kind — all finnguide features beyond the single card.
- Authenticated or write access to Drupal.
- Changes to OLI itself.

## Anti-goals (failure even if it "works")

- The demo stalls on `cargo build` or a network fetch.
- An error page or blank card on stage (unknown company, Drupal down, etc.).
- Setup that needs the presenter to type anything beyond the one-liner.

## The app

### Page

Single HTML page, dark theme, matching the mockup. All HTML/CSS/JS embedded in the
binary (no external assets, no CDN, no frontend build step).

- **Ticker band** across the top (new vs. mockup): the curated tickers as
  clickable chips (ticker + price + day change colour). Clicking one loads its
  card. This is what the presenter "asks about".
- Search input + **GET DATA** + **RESET** buttons.
- Search is limited to the 10 cached tickers (user decision). Type-ahead
  suggestions while typing (matches ticker or company name, case-insensitive,
  substring — "disn" finds Disney). Enter or GET DATA loads.
- Column header row: 14 DAYS · M/CAP · PRICE.
- **Header row**: ticker, company name, "sector · exchange", 14-day sparkline
  (green if up over the window, red if down), market cap, price.
- **Tip from Drupal** strip directly under the header row (new vs. mockup): tip
  title + text, linking to the Drupal node. Fresh random tip on every search.
- Expanded panel: company name / sector / exchange, then two columns:
  - **Momentum Signals**: RSI (14), MACD, Price vs 50d SMA, Bollinger Band, Volume
    (RVOL + OBV), Analyst Sentiment, Earnings Surprise, News Sentiment.
  - **Fundamentals**: P/E, ROE, Debt/Equity, EPS Growth (5Y), Revenue Growth (3Y),
    P/B, Price vs 200d SMA.
  - Each signal row: name, colored text label, thin strength bar underneath (as in
    the mockup). No overall score anywhere.
  - **Dividends & Fundamentals**: yield (TTM), per share (annual), 5Y growth rate,
    payout ratio, ex-dividend date, payment date, market cap.
  - **Recent News**: 3 items with title, snippet, relative age.
- **Data timestamp** on the card: just the date and time the shown data was
  fetched (e.g. "28 Oct 2026, 14:05 UTC") — the live fetch time, or the
  snapshot's `fetched_at` when falling back. No "live"/"cached" wording (user
  decision).
- "N/A" is a legitimate display value (as in the mockup) when a field is absent.
- Unknown query: friendly message pointing at the ticker band — never an error
  page.

### Data

**Source: Yahoo Finance** (unofficial, free, no key). Chart endpoint for daily
prices/volumes; quoteSummary (cookie + crumb handshake) for profile, fundamentals,
analyst counts, earnings, dividends; search endpoint for news only. Not every field exists for every ticker — absent ⇒ "N/A".

**Curated tickers:** exactly 10 popular tickers (user decision), listed in one
place in the repo: DIS, AAPL, MSFT, NVDA, TSLA, KO, AMZN, NFLX, GOOGL, META.
These drive the ticker band and the snapshot.

**Embedded snapshot (build time):**
- A `refresh-data` command (Rust binary or `cargo run --bin`) fetches the curated
  tickers from Yahoo and writes `data/snapshot.json` with a top-level
  `fetched_at` UTC timestamp (and per-ticker raw inputs: profile, price, market
  cap, ~1 year of daily closes + volumes, fundamentals ratios, analyst
  buy/hold/sell counts, EPS surprise %, dividend fields, 3 news items with
  publish time and a sentiment value).
- The snapshot is compiled into the binary (`include_str!`). It is **committed**
  so `cargo build` / `cargo run` work offline and deterministically.
- The release workflow runs `refresh-data` before building, so each release
  binary embeds data from the day it was built. If that refresh fails, the build
  uses the committed snapshot rather than failing.
- No network access from `build.rs`.

**Runtime:**
- Loading a ticker tries Yahoo with a short overall timeout (~2–3 s). On success,
  serve live data stamped with the fetch time. On any failure (timeout, 429,
  crumb failure, parse error), serve that ticker from the snapshot stamped with
  `fetched_at`. Partial live failures fall back for the whole ticker (simpler,
  consistent card).
- Successful live results are cached in memory for ~10 minutes to stay clear of
  Yahoo rate limits during repeated demo searches. After a live failure, stop
  trying Yahoo for ~60 s so a dead connection doesn't add a timeout to every
  click.
- Ticker band prices: from the snapshot on page load, then refreshed live in the
  background if Yahoo responds.
- Free-text search matches only the 10 curated tickers (ticker or name,
  substring). Anything else gets the friendly "try one of these" message.
- News ages are computed from real publish times; with cached data they may be
  old, which is honest given the data timestamp shown.

### Signal logic (in Rust)

Labels and bar strengths are **computed in Rust from the raw numbers**, not stored
as text — this is deliberately where the visible Rust logic lives. Port the rules
from finnguide `src/scorer.js` (RSI Wilder smoothing, MACD 12/26/9, SMA 50/200,
Bollinger 20/2σ, RVOL vs 20-day avg + OBV slope, analyst consensus %, avg EPS
surprise, news sentiment, fundamentals thresholds). Price/volume signals (RSI,
MACD, SMAs, Bollinger, RVOL/OBV) are computed from the bundled daily series;
analyst counts, EPS surprise %, per-article news sentiment and fundamentals ratios
are stored raw inputs, with only their labels/strengths derived in Rust (builder's
call). Simplify freely; the per-signal
0–100 strength drives only the bar width and color, never a combined score.

### Investing tips from Drupal

- On every successful search, the server calls
  `GET {DRUPAL_BASE_URL}/jsonapi/node/investing_tip?filter[status]=1` and picks one
  at random. No caching — editing a tip in Drupal admin shows on the next search
  (a planned live moment in the talk).
- `DRUPAL_BASE_URL` env var, default `http://localhost` (inside the DDEV web
  container nginx serves Drupal on port 80).
- Short timeout (~1s). On any failure, use a bundled tip and label it subtly
  "offline". The card must never fail because of Drupal.
- The node link shown to the user must be the public site URL
  (`https://<project>.ddev.site/node/<nid>`), not `localhost`. Use a
  `DRUPAL_PUBLIC_URL` env var (set by the OLI setup from `$DDEV_PRIMARY_URL`).

### Server

- Rust, axum + tokio + serde (+ a small HTTP client for JSON:API, e.g. reqwest
  with rustls). Keep the dependency set small so the fallback source build is fast.
- Endpoints (shape is the builder's call): page at `/`, search/suggest, company
  detail (+ tip), `/health`.
- Listens on `0.0.0.0:${PORT:-3000}`.

## OLI integration

Repo: **public** GitHub repo `https://github.com/nickopris/oli-demo-rust-app`
(user decision). It contains the Rust source and `.drupalaibp/config.json`.
**No secrets in the repo, ever:** no tokens, keys, `.env` files, or personal data;
`.gitignore` covers `.env*`, `target/`, `.ddev/rust-demo/bin/`; the release
workflow uses only the built-in `GITHUB_TOKEN`; scan the tree before the first
push. A `v*` release must exist before the talk, otherwise every run falls back
to the slow source build.
OLI loads it via `--config-url https://github.com/nickopris/oli-demo-rust-app`, or
from the user's umbrella repo whose `.drupalaibp/remote-configs.json` lists it
(include a sample entry in the README).

Verified OLI behaviour this design relies on (2.0.x source):
- `--config-url` copies the whole clone into the project. For a fresh-scaffold
  Drupal build that copy happens at `create_drupal_site_done`, i.e. **after** the
  first `ddev start`.
- `ddev restart` only happens when the config lists `ddev_addons`.
- Hooks on early events (`before_ddev_start`, …) are staged early under
  `.ddev/drupalaibp-hooks/`; `where: host` is required before containers run.
- `launch_path` (e.g. `":8443"`) makes OLI's final step open that port.
- `append_welcome_text` supports `{SITE_URL}` and color tokens.
- Events `before_<step>` / `<step>_done` exist for every pipeline step.

### config.json (shape)

- `is_drupal: true`, Drupal CMS profile (OLI default).
- `install_modules: ["jsonapi"]`.
- `agents`, `skills`, `deny_commands`: OLI defaults (claude; superpowers etc.;
  `git push`, `ssh`).
- `questions`: none.
- `launch_path: ":8443"` (or whatever port is chosen).
- `append_welcome_text`: Rust app URL, Drupal URL + admin/admin, "Try: Disney,
  AAPL, Nvidia", and a hint to edit a tip in Drupal and search again.
- `label` / `description` set, so it reads well in an umbrella picker.

### Hooks

1. **`before_ddev_start`, `where: host`** — writes into `.ddev/`:
   - `config.rust-demo.yaml` with `web_extra_daemons` (runs the wrapper) and
     `web_extra_exposed_ports` (router serves container port 3000 on HTTPS 8443 /
     HTTP 8080), plus `web_environment` for `DRUPAL_BASE_URL` /
     `DRUPAL_PUBLIC_URL` if needed.
   - the daemon wrapper script: waits until the binary exists, then `exec`s it.
   This makes the very first `ddev start` include the app and its port, with no
   extra restart. (A committed `.ddev/` file would arrive too late — see above.)
2. **`create_drupal_site_done`, `where: web`** (weight > -100, so after OLI's
   repo copy) — `fetch-binary.sh`: map `uname -m` (x86_64 → amd64, aarch64 →
   arm64), download the matching binary from the GitHub **latest release** into
   `.ddev/rust-demo/bin/` (gitignored), make it executable, wait until `/health`
   responds, print success. If download fails, fall back to installing a Rust
   toolchain and `cargo build --release` from the now-copied source, then copy the
   result to the same path (slow but the demo still works). Must be idempotent.
   *Why not `ddev_started`:* it fires before `create_drupal_site`, when the project
   root must stay empty for `composer create-project` and the Rust source isn't
   copied in yet (builder check, verified against `pipeline.sh`). The run is
   bounded by the Drupal CMS install anyway, and the browser only opens at the end.
3. **`provision_from_config_done`, `where: web`** — after `jsonapi` is enabled:
   - import the `investing_tip` content type + its field config from YAML in the
     repo (do this in the hook with `drush config:import --partial --source=…`,
     not via OLI's `.drupalaibp/config/` step — that step resolves its source
     from `LAUNCH_DIR`, which may not be the clone in a `--config-url` build).
   - create 5 published tips if none exist (idempotent).
   - ensure anonymous has `access content` (Drupal CMS grants it by default;
     assert rather than assume).

### Drupal content

- Content type `investing_tip`: title + a text field for the tip body. Drupal CMS
  may not ship a reusable `body` field storage, so the YAML defines its own field
  storage (e.g. `field_tip_text`) rather than relying on `node.body`.
- 5 tips, short (1–2 sentences), generic and educational (diversification,
  time in market, fees, emergency fund, don't chase hype). Not advice about
  specific stocks.

### Release pipeline

GitHub Actions on `v*` tags: build release binaries for `x86_64-unknown-linux-gnu`
and `aarch64-unknown-linux-gnu` (static/musl preferred so they run regardless of
the DDEV web image's glibc), attach to the GitHub Release as
`oli-demo-rust-app-linux-amd64` and `oli-demo-rust-app-linux-arm64`. The hook uses
`releases/latest/download/<asset>` so the config never changes per release.

## Decided by the user

- Purpose: OLI demo; Rust app is a vehicle. Simplify finnguide to the single card;
  no scores/ratings.
- Failure mid-demo is worse than stale numbers. Yahoo Finance at runtime, with an
  embedded, timestamped snapshot of the curated tickers (built at release time) as
  fallback; the UI shows only the date/time the data was fetched (no
  live/cached wording).
- Search limited to the cached tickers.
- Ticker band at the top with 10 popular tickers, all cached in the snapshot.
- Must be fast: prebuilt binaries, no on-stage compile.
- Repo hosted on GitHub; consumed via remote config.
- Drupal included: JSON:API on, ≤5 investing tips in a content type, Rust fetches a
  random one per search, public unauthenticated endpoints only.
- Drupal CMS profile.
- Public repo at github.com/nickopris/oli-demo-rust-app; no secrets in it.

## Builder's calls (made during brainstorm, flagged for review)

- Signal labels computed in Rust rather than stored as text.
- Type-ahead suggestions; unknown query shows clickable available tickers.
- No caching of tips (live edit demo moment); offline bundled fallback.
- Tip strip placement under the header row.
- Ports 3000 (container) / 8443 / 8080; musl static binaries.
- Exact endpoints, crate choices beyond axum/serde, data generation method,
  file layout, tests.
