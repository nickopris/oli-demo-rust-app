<?php

/**
 * @file
 * Seeds five investing tips. Run with `drush php:script`.
 *
 * Keep in sync with FALLBACK_TIPS in src/tips.rs.
 */

use Drupal\node\Entity\Node;

$existing = \Drupal::entityQuery('node')
  ->condition('type', 'investing_tip')
  ->accessCheck(FALSE)
  ->count()
  ->execute();

if ($existing > 0) {
  echo "Investing tips already present ($existing).\n";
  return;
}

$tips = [
  'Spread your risk' => "Diversify across companies, sectors and regions so one bad bet can't sink your portfolio.",
  'Time in the market' => 'Staying invested through ups and downs has historically beaten trying to time the perfect entry.',
  'Mind the fees' => 'A 1% yearly fee compounds too. Low-cost index funds keep more of your returns working for you.',
  'Emergency fund first' => 'Keep three to six months of expenses in cash before investing, so you never have to sell in a downturn.',
  "Don't chase hype" => 'If everyone is talking about a stock, the easy gains are usually gone. Research before you buy.',
];

foreach ($tips as $title => $text) {
  Node::create([
    'type' => 'investing_tip',
    'title' => $title,
    'field_tip_text' => $text,
    'status' => 1,
  ])->save();
}

echo 'Created ' . count($tips) . " investing tips.\n";
