# OLI Rust demo app

A small Rust web app that shows a stock card for ten well-known companies, built to
demonstrate that [One Line Installer](https://www.drupal.org/project/one_line_installer)
can bring any application into a DDEV project, next to Drupal CMS, from a remote config.

- **Stock data:** Yahoo Finance (free, no key). If Yahoo is slow or unreachable, the app
  serves a snapshot compiled into the binary and shows when that data was fetched.
- **Drupal:** every search also fetches a random *Investing tip* from Drupal's public
  JSON:API. Edit a tip in Drupal, search again, and the new text appears.
  Without Drupal the app shows a built-in tip marked "offline".
- **Tickers:** DIS, AAPL, MSFT, NVDA, TSLA, KO, AMZN, NFLX, GOOGL, META.

## Run it with One Line Installer

From an empty directory, run the One Line Installer one-liner with:

```
--config-url https://github.com/nickopris/oli-demo-rust-app
```

OLI builds Drupal CMS, enables JSON:API, creates the tips, downloads the prebuilt Rust
binary from the latest GitHub release and opens `https://<project>.ddev.site:9443`.
Nothing to configure: no questions, no API keys.

To offer it from an umbrella repo, add an entry to that repo's
`.drupalaibp/remote-configs.json`:

```json
[
  {
    "repo": "https://github.com/nickopris/oli-demo-rust-app",
    "config": "config",
    "label": "Rust stock card demo",
    "description": "Drupal CMS plus a Rust app that reads tips from JSON:API."
  }
]
```

### What the config does

| File | When | What |
|---|---|---|
| `.drupalaibp/config.json` | | Drupal CMS, `jsonapi`, agents and skills, opens port 9443 |
| `.drupalaibp/rust-demo-ddev.sh` | `before_ddev_start`, host | Writes `.ddev/config.rust-demo.yaml` (a `web_extra_daemon` plus router port 9443/9080) |
| `.drupalaibp/fetch-binary.sh` | `create_drupal_site_done`, web | Downloads the binary for the container's architecture; builds from source if that fails |
| `.drupalaibp/setup-tips.sh` | `provision_from_config_done`, web | Imports the `investing_tip` content type and seeds five tips |

## Develop locally

No Rust install needed: `scripts/cargo` runs cargo in Docker.

```sh
scripts/cargo test
scripts/cargo run            # http://localhost:3300 (set HOST_PORT to change)
scripts/cargo run --bin refresh-data   # refresh data/snapshot.json from Yahoo
```

With a local Rust toolchain, plain `cargo` works the same (the app listens on 3000).

| Env var | Default | Purpose |
|---|---|---|
| `PORT` | `3000` | Port the app listens on |
| `DRUPAL_BASE_URL` | `http://localhost` | Where the app calls JSON:API |
| `DRUPAL_PUBLIC_URL` | `DDEV_PRIMARY_URL`, else the base URL | Base for tip links shown to users |

## Releases

Pushing a `v*` tag runs `.github/workflows/release.yml`: it refreshes the snapshot from
Yahoo (keeping the committed one if that fails), builds static musl binaries for amd64
and arm64, and attaches them to the GitHub release as
`oli-demo-rust-app-linux-amd64` and `oli-demo-rust-app-linux-arm64`.

## Disclaimer

Data comes from Yahoo Finance's unofficial endpoints and may be delayed or wrong.
Nothing here is investment advice.
