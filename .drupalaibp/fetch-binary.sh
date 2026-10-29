#!/usr/bin/env bash
# Hook: `create_drupal_site_done` event (weight 10), where=web.
# Downloads the prebuilt Rust binary for this container's architecture from the
# latest GitHub release. If that fails, builds it from the source OLI copied
# into the project (slow, but the demo still works).
set -euo pipefail

cd /var/www/html
repo="nickopris/oli-demo-rust-app"
bin_dir=.ddev/rust-demo/bin
bin="$bin_dir/oli-demo-rust-app"
mkdir -p "$bin_dir"

case "$(uname -m)" in
  x86_64 | amd64) arch=amd64 ;;
  aarch64 | arm64) arch=arm64 ;;
  *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac

build_from_source() {
  echo "Building the Rust app from source (this takes a few minutes)..."
  if ! command -v cc >/dev/null 2>&1; then
    sudo apt-get update -qq && sudo apt-get install -y -qq build-essential >/dev/null
  fi
  if ! command -v cargo >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
    # shellcheck disable=SC1091
    source "$HOME/.cargo/env"
  fi
  cargo build --release --locked
  install -m 0755 target/release/oli-demo-rust-app "$bin"
}

if [[ -x "$bin" ]]; then
  echo "Rust app binary already present."
else
  url="https://github.com/${repo}/releases/latest/download/oli-demo-rust-app-linux-${arch}"
  tmp="$(mktemp)"
  echo "Downloading the Rust app (${arch})..."
  if curl -fsSL --retry 2 --max-time 120 -o "$tmp" "$url"; then
    install -m 0755 "$tmp" "$bin"
  else
    echo "Download failed: ${url}"
    build_from_source
  fi
  rm -f "$tmp"
fi

for _ in $(seq 1 30); do
  if curl -fsS http://127.0.0.1:3000/health >/dev/null 2>&1; then
    echo "Rust app is up (router port 9443)."
    exit 0
  fi
  sleep 1
done
echo "Rust app did not answer on :3000 yet; check 'ddev logs'." >&2
