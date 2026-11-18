import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { pageFrom, search } from './help/pageFrom';
import { dailyCheckDue, progressPercent } from './updates';

const DIR = join(dirname(fileURLToPath(import.meta.url)), 'help', 'pages');
const pages = readdirSync(DIR)
  .filter((f) => f.endsWith('.md'))
  .sort()
  .map((f) => pageFrom(f, readFileSync(join(DIR, f), 'utf8')));

test('every help page is named NN-id.md and opens with its title', () => {
  assert.ok(pages.length >= 10);
  for (const f of readdirSync(DIR).filter((n) => n.endsWith('.md'))) assert.match(f, /^\d\d-[a-z0-9-]+\.md$/);
  for (const p of pages) assert.match(p.text, /^# \S/, p.id);
  assert.equal(new Set(pages.map((p) => p.id)).size, pages.length, 'no two pages share an id');
  assert.equal(pages[0].id, 'getting-started');
  assert.ok(pages.some((p) => p.id === 'updates'), 'the updates page is where Check for updates lives');
});

test('every topic link opens a page that exists', () => {
  const ids = new Set(pages.map((p) => p.id));
  for (const p of pages) {
    for (const m of p.text.matchAll(/\]\(topic:([^)]+)\)/g)) assert.ok(ids.has(m[1]), `${p.id} links to ${m[1]}`);
  }
});

test('the pages follow the house rules: no em dashes, no other web addresses', () => {
  for (const p of pages) {
    assert.ok(!p.text.includes('—'), `${p.id} has an em dash`);
    assert.ok(!/\]\((?!topic:)/.test(p.text), `${p.id} links outside Help`);
  }
});

test('search finds pages by words, title matches first', () => {
  const r = search(pages, 'cover letter');
  assert.equal(r[0].id, 'cover-letters');
  assert.deepEqual(search(pages, ''), pages);
  assert.deepEqual(search(pages, 'zzzz nothing'), []);
});

test('the daily update check is due only when on and a day has passed', () => {
  const now = new Date('2026-11-18T12:00:00Z');
  assert.equal(dailyCheckDue(false, '', now), false);
  assert.equal(dailyCheckDue(true, '', now), true);
  assert.equal(dailyCheckDue(true, '2026-11-18T01:00:00Z', now), false);
  assert.equal(dailyCheckDue(true, '2026-11-17T11:00:00Z', now), true);
  assert.equal(progressPercent(50, 200), 25);
  assert.equal(progressPercent(5, null), null);
});
