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
use Drupal\workflows\Entity\Workflow;

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

// Drupal CMS opts every new content type into its editorial workflow, which
// would leave the tips as unpublished drafts. Tips are plain demo content.
$unmoderated = FALSE;
if (\Drupal::moduleHandler()->moduleExists('content_moderation')) {
  foreach (Workflow::loadMultipleByType('content_moderation') as $workflow) {
    $type = $workflow->getTypePlugin();
    if ($type->appliesToEntityTypeAndBundle('node', 'investing_tip')) {
      $type->removeEntityTypeAndBundle('node', 'investing_tip');
      $workflow->save();
      $unmoderated = TRUE;
    }
  }
}

$storage = \Drupal::entityTypeManager()->getStorage('node');
$existing = $storage->loadByProperties(['type' => 'investing_tip']);

if ($existing) {
  // Tips seeded while the type was moderated are drafts: publish them.
  if ($unmoderated) {
    foreach ($existing as $node) {
      $node->setPublished()->save();
    }
    echo 'Published ' . count($existing) . " investing tips.\n";
  }
  echo 'Investing tips already present (' . count($existing) . ").\n";
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
