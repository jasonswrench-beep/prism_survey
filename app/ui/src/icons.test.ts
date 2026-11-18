import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { test } from 'node:test';
import { ICON_ROOT, buildModule, isUpToDate, parseIcon } from '../scripts/build-icons';
import { NEW_ICONS, REUSED_ICONS } from './iconPlan';

const OUTLINE = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.75" stroke-linecap="round" stroke-linejoin="round">';
const BRIEF = readFileSync(resolve(ICON_ROOT, '../../design/icons/Prism Survey Icon List.md'), 'utf8');
const files = readdirSync(ICON_ROOT).filter((n) => n.endsWith('.svg')).map((n) => n.slice(0, -4));
const planned = new Set<string>([...REUSED_ICONS, ...NEW_ICONS]);

test('the generated icon module matches the SVG files in assets/icons', () => {
  assert.ok(isUpToDate(), 'run `npm run icons` after changing an icon file');
});

test('building the module twice gives the same text', () => {
  assert.equal(buildModule(), buildModule());
});

test('the 31 icons reused from PrismQDA are all in assets/icons', () => {
  assert.equal(REUSED_ICONS.length, 31);
  assert.deepEqual(REUSED_ICONS.filter((n) => !files.includes(n)), []);
});

test('every file in assets/icons has a name the brief plans, so a misnamed file is caught', () => {
  assert.deepEqual(files.filter((n) => !planned.has(n)), []);
});

test('the plan in iconPlan.ts is the brief: 57 new interface icons, 2 file glyphs, 31 reused', () => {
  // New icons are the file names in the brief's tables.
  const tableNames = new Set<string>();
  for (const line of BRIEF.split('\n').filter((l) => l.startsWith('|'))) {
    for (const m of line.matchAll(/\|\s*([a-z][a-z0-9-]*)\.svg\s*(?=\|)/g)) tableNames.add(m[1]);
  }
  const fileGlyphs = [...tableNames].filter((n) => n.startsWith('file-')).sort();
  assert.deepEqual(fileGlyphs, ['file-scale', 'file-study']);
  assert.deepEqual([...tableNames].filter((n) => !n.startsWith('file-')).sort(), [...NEW_ICONS].sort());
  assert.equal(NEW_ICONS.length, 57);
  assert.equal(new Set(NEW_ICONS).size, NEW_ICONS.length, 'no name is listed twice');

  // Reused icons are the list under "Reused from PrismQDA", and every "(reuse x)" in the tables names a planned icon.
  const reusedLine = BRIEF.split('\n').find((l) => l.startsWith('31 files, copied unchanged'));
  assert.ok(reusedLine, 'the brief lists its reused icons');
  const listed = [...reusedLine.split(':').slice(1).join(':').matchAll(/\b([a-z]+-[a-z0-9-]+)\b/g)].map((m) => m[1]);
  assert.deepEqual([...new Set(listed)].sort(), [...REUSED_ICONS].sort());
  for (const m of BRIEF.matchAll(/\(reuse ([^)]*)\)/g)) {
    for (const n of m[1].split(',').map((s) => s.trim().split(' ')[0]).filter((s) => /^[a-z]+-[a-z0-9-]+$/.test(s))) {
      assert.ok(planned.has(n), `the brief reuses ${n}, which is not in the plan`);
    }
  }
});

test('icon geometry stays inside the 24 pixel grid, roughly', () => {
  for (const name of files) {
    const icon = parseIcon(readFileSync(resolve(ICON_ROOT, `${name}.svg`), 'utf8'), name);
    assert.equal(icon.viewBox, '0 0 24 24', name);
    for (const p of icon.paths) {
      for (const m of p.d.matchAll(/[MLHV]\s*(-?\d+(?:\.\d+)?)(?:[\s,]+(-?\d+(?:\.\d+)?))?/g)) {
        for (const n of [m[1], m[2]].filter((x): x is string => x !== undefined)) {
          const v = Number(n);
          assert.ok(v >= -0.5 && v <= 24.5, `${name}: coordinate ${v} is outside the grid`);
        }
      }
    }
  }
});

test('the parser rejects anything the icon standard does not allow', () => {
  const ok = `${OUTLINE}<path d="M3 3L9 9"/></svg>`;
  assert.equal(parseIcon(ok, 'ok.svg').paths.length, 1);
  assert.equal(parseIcon(`${OUTLINE}<path d="M3 3L9 9" stroke-dasharray="2 3"/></svg>`, 'dash.svg').paths[0].dash, '2 3', 'a dashed line is allowed');
  const cases: [string, string][] = [
    ['a script', `${OUTLINE}<script>alert(1)</script><path d="M3 3L9 9"/></svg>`],
    ['an embedded image', `${OUTLINE}<image href="data:image/png;base64,AAAA"/><path d="M3 3L9 9"/></svg>`],
    ['a transform', `${OUTLINE}<path d="M3 3L9 9" transform="rotate(5)"/></svg>`],
    ['an event handler', `${OUTLINE}<path d="M3 3L9 9" onload="x()"/></svg>`],
    ['an external reference', `${OUTLINE}<path d="M3 3L9 9" style="fill:url(http://example.test/a)"/></svg>`],
    ['text in the path data', `${OUTLINE}<path d="M3 3L9 9 javascript:x"/></svg>`],
    ['a hard-coded color', `${OUTLINE}<path d="M3 3L9 9" stroke="#ff0000"/></svg>`],
    ['a different stroke width', ok.replace('1.75', '2')],
    ['a filled icon', ok.replace('fill="none"', 'fill="currentColor"')],
    ['a dash pattern that is not numbers', `${OUTLINE}<path d="M3 3L9 9" stroke-dasharray="url(#x)"/></svg>`],
    ['a different viewBox', ok.replace('0 0 24 24', '0 0 32 32')],
    ['a text element', `${OUTLINE}<text>A</text><path d="M3 3L9 9"/></svg>`],
    ['no paths', `${OUTLINE}</svg>`],
    ['an unclosed svg', `${OUTLINE}<path d="M3 3L9 9"/>`],
  ];
  for (const [what, svg] of cases) {
    assert.throws(() => parseIcon(svg, 'bad.svg'), `a file with ${what} should be rejected`);
  }
});
