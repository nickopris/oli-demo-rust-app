#!/usr/bin/env bash
# Hook: `provision_from_config_done` event (weight 10), where=web.
# Runs after JSON:API is enabled: creates the Investing tip content type and
# seeds five tips (both idempotent), and lets anonymous users read content.
set -euo pipefail

cd /var/www/html
drush php:script .drupalaibp/seed-tips.php
drush role:perm:add anonymous 'access content'
drush cache:rebuild
