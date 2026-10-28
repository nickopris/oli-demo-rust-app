#!/usr/bin/env bash
# Hook: `provision_from_config_done` event (weight 10), where=web.
# Runs after JSON:API is enabled: creates the Investing tip content type,
# lets anonymous users read content, and seeds five tips once.
set -euo pipefail

cd /var/www/html
drush config:import --partial --source=/var/www/html/.drupalaibp/tip-config -y
drush role:perm:add anonymous 'access content'
drush php:script .drupalaibp/seed-tips.php
drush cache:rebuild
