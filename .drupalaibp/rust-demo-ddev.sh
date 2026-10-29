#!/usr/bin/env bash
# Hook: `before_ddev_start` event (weight 0), where=host.
# Writes the DDEV config that runs the Rust app as a web_extra_daemon and
# publishes it through the router on 9443/9080. It has to exist before the
# first `ddev start`: in a Drupal build OLI copies the repo in only after the
# site is scaffolded, which would need an extra restart.
set -euo pipefail

# Hooks run from the project root; fall back to the staged script's location.
root="$PWD"
if [[ ! -d "$root/.ddev" ]]; then
  root="$(cd "$(dirname "$0")/../.." && pwd)"
fi
mkdir -p "$root/.ddev/rust-demo/bin"

cat > "$root/.ddev/config.rust-demo.yaml" <<'YAML'
# Written by .drupalaibp/rust-demo-ddev.sh: runs the Rust stock card app.
web_extra_daemons:
  - name: rust-demo
    command: /var/www/html/.ddev/rust-demo/run.sh
    directory: /var/www/html
web_extra_exposed_ports:
  - name: rust-demo
    container_port: 3000
    http_port: 9080
    https_port: 9443
YAML

cat > "$root/.ddev/rust-demo/run.sh" <<'SH'
#!/usr/bin/env bash
# Started by supervisord in the web container. Waits for the binary that
# fetch-binary.sh downloads after Drupal is scaffolded, then runs it.
bin=/var/www/html/.ddev/rust-demo/bin/oli-demo-rust-app
until [[ -x "$bin" ]]; do sleep 2; done
export PORT=3000
export DRUPAL_BASE_URL="${DRUPAL_BASE_URL:-http://localhost}"
export DRUPAL_PUBLIC_URL="${DRUPAL_PUBLIC_URL:-${DDEV_PRIMARY_URL:-}}"
exec "$bin"
SH
chmod +x "$root/.ddev/rust-demo/run.sh"

echo "Rust demo: DDEV daemon and port 9443 configured."
