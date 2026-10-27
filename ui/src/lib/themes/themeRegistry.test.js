import assert from 'node:assert/strict';
import test from 'node:test';
import { readdir, readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  DEFAULT_DARK_THEME,
  DEFAULT_LIGHT_THEME,
  REQUIRED_COLOR_VARS,
  THEME_CATEGORIES,
  THEMES,
  groupThemes,
  isValidPref,
  pairPrefs,
  seedPrefs,
} from './themeRegistry.js';

const here = dirname(fileURLToPath(import.meta.url));
const hex = /^#[0-9a-f]{6}$/i;

test('categories only reference existing themes with a matching category', () => {
  for (const [categoryId, category] of Object.entries(THEME_CATEGORIES)) {
    assert.ok(category.name, `category ${categoryId} needs a display name`);
    assert.ok(category.themes.length > 0, `category ${categoryId} needs at least one theme`);

    for (const id of category.themes) {
      assert.ok(THEMES[id], `category ${categoryId} references unknown theme ${id}`);
      assert.equal(THEMES[id].category, categoryId, `theme ${id} points to another category`);
    }
  }
});

test('every theme is listed exactly once across categories', () => {
  const seen = new Set();

  for (const category of Object.values(THEME_CATEGORIES)) {
    for (const id of category.themes) {
      assert.ok(!seen.has(id), `theme ${id} is listed twice`);
      seen.add(id);
    }
  }

  for (const id of Object.keys(THEMES)) {
    assert.ok(seen.has(id), `theme ${id} is missing from THEME_CATEGORIES`);
  }
});

test('every theme has consistent metadata', () => {
  for (const [id, info] of Object.entries(THEMES)) {
    assert.equal(info.id, id, 'theme id must match its key');
    assert.ok(info.name, `theme ${id} needs a name`);
    assert.ok(info.description, `theme ${id} needs a description`);
    assert.ok(info.icon, `theme ${id} needs an icon`);
    assert.ok(
      info.isDark === null || typeof info.isDark === 'boolean',
      `theme ${id} needs a boolean or null isDark`
    );

    if (info.color !== undefined) {
      assert.match(info.color, hex, `theme ${id} color must be a hex value`);
    }
  }
});

test('every concrete theme pairs with the opposite tone in its family', () => {
  for (const [id, info] of Object.entries(THEMES)) {
    if (id === 'system') {
      assert.equal(info.pair, undefined, 'system has no pair');
      continue;
    }

    assert.ok(info.pair, `theme ${id} needs a pair`);
    const pair = THEMES[info.pair];
    assert.ok(pair, `theme ${id} pair references unknown theme ${info.pair}`);
    assert.notEqual(pair.isDark, info.isDark, `theme ${id} pair must be the opposite tone`);
    assert.equal(pair.category, info.category, `theme ${id} pair must stay in its family`);
  }
});

test('pairPrefs maps a theme to its automatic light and dark counterparts', () => {
  assert.deepEqual(pairPrefs('light'), { light: 'light', dark: 'dark' });
  assert.deepEqual(pairPrefs('dark'), { light: 'light', dark: 'dark' });
  assert.deepEqual(pairPrefs('old-dark'), { light: 'old-light', dark: 'old-dark' });
  assert.deepEqual(pairPrefs('old-light'), { light: 'old-light', dark: 'old-dark' });
  assert.deepEqual(pairPrefs('latte'), { light: 'latte', dark: 'mocha' });
  assert.deepEqual(pairPrefs('mocha'), { light: 'latte', dark: 'mocha' });
  assert.deepEqual(pairPrefs('frappe'), { light: 'latte', dark: 'frappe' });
  assert.deepEqual(pairPrefs('tokyo-light'), { light: 'tokyo-light', dark: 'tokyo-night' });
  assert.deepEqual(pairPrefs('tokyo-storm'), { light: 'tokyo-light', dark: 'tokyo-storm' });
  assert.deepEqual(pairPrefs('dracula'), { light: 'alucard', dark: 'dracula' });
  assert.deepEqual(pairPrefs('gruvbox-light'), { light: 'gruvbox-light', dark: 'gruvbox-dark' });
  assert.equal(pairPrefs('system'), null);
  assert.equal(pairPrefs('nope'), null);
});

test('isValidPref accepts only themes of the expected tone', () => {
  assert.equal(isValidPref('latte', false), true);
  assert.equal(isValidPref('latte', true), false);
  assert.equal(isValidPref('mocha', true), true);
  assert.equal(isValidPref('mocha', false), false);
  assert.equal(isValidPref('system', false), false);
  assert.equal(isValidPref('nope', false), false);
});

test('default automatic preferences match their tones', () => {
  assert.equal(isValidPref(DEFAULT_LIGHT_THEME, false), true);
  assert.equal(isValidPref(DEFAULT_DARK_THEME, true), true);
});

test('seedPrefs keeps valid stored values and falls back to the mirror then defaults', () => {
  assert.deepEqual(seedPrefs('system', null, null), { light: 'light', dark: 'dark' });
  assert.deepEqual(seedPrefs('mocha', null, null), { light: 'latte', dark: 'mocha' });
  assert.deepEqual(seedPrefs('latte', null, null), { light: 'latte', dark: 'mocha' });
  assert.deepEqual(seedPrefs('mocha', 'alucard', 'tokyo-night'), {
    light: 'alucard',
    dark: 'tokyo-night',
  });

  // wrong tone or unknown stored ids are replaced by the mirror
  assert.deepEqual(seedPrefs('mocha', 'mocha', 'latte'), { light: 'latte', dark: 'mocha' });
  assert.deepEqual(seedPrefs('dracula', 'nope', 'nope'), { light: 'alucard', dark: 'dracula' });
  assert.deepEqual(seedPrefs('system', 'latte', 'nope'), { light: 'latte', dark: 'dark' });
});

test('groupThemes keeps category order, filters ids and drops empty groups', () => {
  const lights = Object.values(THEMES)
    .filter(info => info.isDark === false)
    .map(info => info.id);
  const lightGroups = groupThemes(lights);

  assert.deepEqual(
    lightGroups.map(group => group.id),
    ['default', 'old', 'catppuccin', 'tokyo-night', 'dracula', 'gruvbox']
  );
  assert.deepEqual(lightGroups[0].ids, ['light']);
  assert.deepEqual(lightGroups[1].ids, ['old-light']);
  assert.deepEqual(lightGroups[2].ids, ['latte']);

  for (const group of lightGroups) {
    for (const themeId of group.ids) {
      assert.equal(THEMES[themeId].isDark, false, `${themeId} leaked into the light picker`);
      assert.equal(THEMES[themeId].category, group.id, `${themeId} landed in the wrong group`);
    }
  }

  const allGroups = groupThemes(Object.keys(THEMES));
  assert.deepEqual(
    allGroups.flatMap(group => group.ids),
    Object.keys(THEMES)
  );

  assert.deepEqual(groupThemes(['nope']), []);
});

test('every theme defines all color variables in CSS', async () => {
  const files = (await readdir(here)).filter(file => file.endsWith('.css'));
  const sources = await Promise.all([
    ...files.map(file => readFile(join(here, file), 'utf8')),
    readFile(join(here, '..', '..', 'app.css'), 'utf8'),
  ]);
  const css = sources.join('\n');

  for (const id of Object.keys(THEMES)) {
    // `system` resolves to light/dark at runtime and has no block of its own
    if (id === 'system') continue;

    const block = extractBlock(css, id);
    assert.ok(block, `theme ${id} has no :root.${id} block`);

    for (const name of REQUIRED_COLOR_VARS) {
      assert.ok(block.includes(`${name}:`), `theme ${id} is missing ${name}`);
    }

    assert.match(block, /color-scheme:\s*(light|dark)/, `theme ${id} needs color-scheme`);
  }
});

function extractBlock(css, id) {
  const match = css.match(new RegExp(`:root\\.${id}[^{]*\\{([^}]*)\\}`));
  return match?.[1] ?? null;
}
