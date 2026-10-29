<?php

/**
 * @file
 * Creates the Investing tip content type and seeds five tips. Run with
 * `drush php:script`.
 *
 * Every step loads what already exists before creating it, so the script is
 * safe to re-run and tolerates displays Drupal created on its own.
 *
 * Keep in sync with FALLBACK_TIPS in src/tips.rs.
 */

use Drupal\field\Entity\FieldConfig;
use Drupal\field\Entity\FieldStorageConfig;
use Drupal\node\Entity\Node;
use Drupal\node\Entity\NodeType;

if (!NodeType::load('investing_tip')) {
  NodeType::create([
    'type' => 'investing_tip',
    'name' => 'Investing tip',
    'description' => 'Short, general investing tips shown by the Rust stock card app.',
    'new_revision' => TRUE,
    'display_submitted' => FALSE,
  ])->save();
}

if (!FieldStorageConfig::loadByName('node', 'field_tip_text')) {
  FieldStorageConfig::create([
    'field_name' => 'field_tip_text',
    'entity_type' => 'node',
    'type' => 'string_long',
  ])->save();
}

if (!FieldConfig::loadByName('node', 'investing_tip', 'field_tip_text')) {
  FieldConfig::create([
    'field_name' => 'field_tip_text',
    'entity_type' => 'node',
    'bundle' => 'investing_tip',
    'label' => 'Tip text',
    'required' => TRUE,
  ])->save();
}

$displays = \Drupal::service('entity_display.repository');
$displays->getFormDisplay('node', 'investing_tip')
  ->setComponent('field_tip_text', [
    'type' => 'string_textarea',
    'weight' => 1,
    'settings' => ['rows' => 3],
  ])
  ->save();
$displays->getViewDisplay('node', 'investing_tip')
  ->setComponent('field_tip_text', [
    'type' => 'basic_string',
    'label' => 'hidden',
    'weight' => 0,
  ])
  ->save();

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
