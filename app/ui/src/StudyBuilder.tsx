// The Questionnaire tab (decision 79): a questionnaire is an ordered stack of parts (scales, a cover letter, transition
// sentences, demographic questions) grouped into named sections, exported as one Word document in two copies. Each
// part is a box: drag it, or use its arrow buttons (keyboard), to reorder it; select it to edit it. A section is a
// heading part: the boxes after it, up to the next section, belong to it and move with it. The cover letter is always
// first and prints on its own page. Files keep the .pstudy name, so questionnaires saved as studies still open.

import { Fragment, useEffect, useMemo, useState } from 'react';
import { call } from './bridge';
import type { Check, LibraryEntry, Scale } from './types';
import { FORMAT_LABEL } from './types';
import { Icon } from './Icon';

/// A ready-made demographic question (demographics.bank): `text` is what it adds to the block.
interface BankQuestion {
  id: string;
  group: string;
  label: string;
  text: string;
}

export type Part =
  | { kind: 'scale'; scale: Scale; libraryPath?: string }
  | { kind: 'pageBreak' }
  | { kind: 'section'; title: string; newPage: boolean }
  | { kind: 'transition'; text: string }
  | { kind: 'demographics'; title: string; text: string }
  | { kind: 'coverLetter'; title: string; text: string };

export interface Study {
  pstudyVersion: number;
  id: string;
  title: string;
  parts: Part[];
  options: {
    numbering: 'restartEach' | 'runOn';
    titles: 'show' | 'neutral' | 'hide';
    neutralWord: string;
    sources: 'underScale' | 'references' | 'none';
    font: string;
    sizePt: number;
    breakBetweenScales: boolean;
    sectionHeadings: boolean;
  };
  history: { version: number; savedAt: string; summary: string }[];
}

function partLabel(p: Part): string {
  switch (p.kind) {
    case 'scale':
      return p.scale.title || '(untitled scale)';
    case 'pageBreak':
      return 'Page break';
    case 'section':
      return p.title.trim() ? `Section ${p.title}` : 'Section';
    case 'transition':
      return 'Transition sentence';
    case 'demographics':
      return 'Demographic questions';
    case 'coverLetter':
      return 'Cover letter';
  }
}

/// The bracketed parts of a cover letter still to fill in or choose, from the engine (coverLetter.placeholders).
function useStillToFill(text: string | null): string[] {
  const [left, setLeft] = useState<string[]>([]);
  useEffect(() => {
    if (text === null) return;
    const t = setTimeout(() => {
      call<string[]>('coverLetter.placeholders', { text })
        .then(setLeft)
        .catch(() => setLeft([]));
    }, 250);
    return () => clearTimeout(t);
  }, [text]);
  return left;
}

interface Recommendation {
  key: string;
  title: string;
  collection: string;
  score: number;
  reasons: string[];
}

const isPractice = (s: Scale) => (s.tags ?? []).some((t) => t.toLowerCase() === 'practice');

/// Where each section's block of parts ends (the next section, or the end).
function blockEnd(parts: Part[], i: number): number {
  let j = i + 1;
  while (j < parts.length && parts[j].kind !== 'section') j++;
  return j;
}

export function StudyBuilder({
  study,
  setStudy,
  openScale,
  currentScale,
  importScale,
  addLibraryScale,
  showDirectoryEntry,
  library,
  onNew,
  onOpen,
  onOpenSample,
  fail,
}: {
  study: Study | null;
  setStudy: (s: Study) => void;
  openScale: (s: Scale, index: number) => void;
  currentScale: Scale | null;
  importScale: () => Promise<Scale[]>;
  addLibraryScale: (path: string) => void;
  showDirectoryEntry: (id: string) => void;
  library: LibraryEntry[];
  onNew: () => void;
  onOpen: () => void;
  onOpenSample: (path: string) => void;
  fail: (e: unknown) => void;
}) {
  const [length, setLength] = useState<{ items: number; minutes: number } | null>(null);
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  const [selected, setSelected] = useState<number | null>(null);
  const [maxMinutes, setMaxMinutes] = useState(20);
  const [recs, setRecs] = useState<Recommendation[] | null>(null);
  const [bank, setBank] = useState<BankQuestion[]>([]);
  const [practice, setPractice] = useState<LibraryEntry[]>([]);
  const [sampleFolder, setSampleFolder] = useState<string | null>(null);
  const [find, setFind] = useState('');
  const [checks, setChecks] = useState<Record<number, number>>({});

  useEffect(() => {
    call<BankQuestion[]>('demographics.bank')
      .then(setBank)
      .catch(() => setBank([]));
    call<string | null>('samples.folder')
      .then(async (folder) => {
        setSampleFolder(folder);
        if (folder) setPractice((await call<LibraryEntry[]>('library.list', { folder })).filter((e) => !e.error));
      })
      .catch(() => setPractice([]));
  }, []);

  useEffect(() => {
    if (!study) return;
    call<{ items: number; minutes: number }>('study.length', { study }).then(setLength).catch(() => {});
    // The self-check of each scale, for the badge on its box (open findings only).
    const t = setTimeout(async () => {
      const next: Record<number, number> = {};
      for (const [i, p] of study.parts.entries()) {
        if (p.kind !== 'scale') continue;
        try {
          const c = await call<Check>('scale.check', { scale: p.scale });
          next[i] = c.findings.filter((f) => f.status === 'open').length;
        } catch {
          /* the badge is left off */
        }
      }
      setChecks(next);
    }, 300);
    return () => clearTimeout(t);
  }, [study]);

  const coverText = study && study.parts[0]?.kind === 'coverLetter' ? study.parts[0].text : null;
  const left = useStillToFill(coverText);

  const libraryShown = useMemo(() => {
    const q = find.trim().toLowerCase();
    return library.filter((e) => !e.error && (!q || e.title.toLowerCase().includes(q) || e.tags.some((t) => t.toLowerCase().includes(q))));
  }, [library, find]);

  if (!study) {
    return (
      <div className="qstart">
        <section className="panel" aria-labelledby="qstart-h">
          <h1 id="qstart-h">Start a questionnaire</h1>
          <p>A questionnaire puts scales, a cover letter and demographic questions in order, in sections, and exports the whole thing to Word as a respondent copy and a researcher copy.</p>
          <div className="buttons wrap">
            <button className="primary" onClick={onNew}>
              <Icon name="study-new" />
              New questionnaire
            </button>
            <button onClick={onOpen}>
              <Icon name="cmd-open" />
              Open…
            </button>
            {sampleFolder && (
              <button onClick={() => onOpenSample(`${sampleFolder}/Sample Questionnaire.pstudy`)}>Open the sample questionnaire</button>
            )}
          </div>
          {sampleFolder && <p className="muted">The sample is built from made-up practice scales that come with Prism Survey, so you can see how the pieces fit before building your own.</p>}
        </section>
      </div>
    );
  }

  const parts = study.parts;
  const hasCover = parts[0]?.kind === 'coverLetter';
  const first = hasCover ? 1 : 0;
  const set = (patch: Partial<Study>) => setStudy({ ...study, ...patch });
  const setOpt = (patch: Partial<Study['options']>) => setStudy({ ...study, options: { ...study.options, ...patch } });
  const setPart = (i: number, p: Part) => set({ parts: parts.map((x, k) => (k === i ? p : x)) });
  const setParts = (next: Part[]) => {
    setSelected(null);
    set({ parts: next });
  };

  /// Moves one box to a new place; nothing goes above the cover letter.
  function moveItem(from: number, to: number) {
    to = Math.max(first, Math.min(parts.length - 1, to));
    if (from === to || from < first) return;
    const next = [...parts];
    const [p] = next.splice(from, 1);
    next.splice(to, 0, p);
    setParts(next);
  }
  /// Moves a section with all its boxes up past the block before it, or down past the block after it.
  function moveSection(i: number, dir: -1 | 1) {
    const end = blockEnd(parts, i);
    const block = parts.slice(i, end);
    if (dir < 0) {
      let prev = i - 1;
      while (prev >= first && parts[prev].kind !== 'section') prev--;
      if (prev < first) prev = i > first ? first : -1;
      if (prev < 0) return;
      setParts([...parts.slice(0, prev), ...block, ...parts.slice(prev, i), ...parts.slice(end)]);
    } else {
      if (end >= parts.length) return;
      const nextEnd = blockEnd(parts, end);
      setParts([...parts.slice(0, i), ...parts.slice(end, nextEnd), ...block, ...parts.slice(nextEnd)]);
    }
  }
  /// A section dropped on another section goes before it (moving up) or after its block (moving down).
  function dropSectionOn(from: number, target: number) {
    if (from === target) return;
    const end = blockEnd(parts, from);
    if (target > from && target < end) return;
    const block = parts.slice(from, end);
    const rest = [...parts.slice(0, from), ...parts.slice(end)];
    let at = target < from ? target : target - block.length;
    if (target > from) at = blockEnd(rest, at);
    rest.splice(Math.max(first, at), 0, ...block);
    setParts(rest);
  }
  function drop(target: number) {
    if (dragFrom === null) return;
    if (parts[dragFrom]?.kind === 'section' && parts[target]?.kind === 'section') dropSectionOn(dragFrom, target);
    else if (parts[target]?.kind === 'section') moveItem(dragFrom, dragFrom < target ? target : target + 1);
    else moveItem(dragFrom, target);
    setDragFrom(null);
  }
  const remove = (i: number) => setParts(parts.filter((_, k) => k !== i));
  const add = (p: Part) => {
    set({ parts: [...parts, p] });
    setSelected(parts.length);
  };

  async function suggest() {
    try {
      const r = await call<{ recommendations: Recommendation[] }>('study.recommend', { study, settings: { maxMinutes, limit: 10 } });
      setRecs(r.recommendations);
    } catch (e) {
      fail(e);
    }
  }
  /// The cover letter goes first; the questionnaire's title and estimated minutes go in where the template asks.
  async function addCoverLetter(which: 'short' | 'long') {
    if (!study || hasCover) return;
    try {
      const t = await call<{ title: string; text: string }>('coverLetter.template', { length: which });
      let text = t.text;
      const title = study.title.trim();
      if (title && title !== 'New study' && title !== 'New questionnaire') text = text.split('[STUDY TITLE]').join(title);
      if (length && length.minutes > 0) text = text.split('[MINUTES]').join(String(length.minutes));
      set({ parts: [{ kind: 'coverLetter', title: t.title, text }, ...parts] });
      setSelected(0);
    } catch (e) {
      fail(e);
    }
  }

  // Item numbers as the respondent copy prints them, for each scale box.
  const numbers: Record<number, [number, number]> = {};
  let runOn = 0;
  let scaleNumber = 0;
  const scaleNo: Record<number, number> = {};
  for (const [i, p] of parts.entries()) {
    if (p.kind !== 'scale') continue;
    const n = p.scale.items.length;
    numbers[i] = study.options.numbering === 'runOn' ? [runOn + 1, runOn + n] : [1, n];
    runOn += n;
    scaleNo[i] = ++scaleNumber;
  }
  // Boxes before the first section, then each section with its boxes.
  const groups: { section: number | null; items: number[] }[] = [{ section: null, items: [] }];
  for (let i = first; i < parts.length; i++) {
    if (parts[i].kind === 'section') groups.push({ section: i, items: [] });
    else groups[groups.length - 1].items.push(i);
  }
  const sectionCount = groups.filter((g) => g.section !== null).length;
  const sel = selected !== null && selected < parts.length ? parts[selected] : null;

  const moveButtons = (i: number, label: string) => (
    <span className="qmoves">
      <button className="icon" aria-label={`Move ${label} up`} title="Move up" onClick={() => moveItem(i, i - 1)} disabled={i <= first}>
        <Icon name="ui-chevron-down" size={16} turn={180} />
      </button>
      <button className="icon" aria-label={`Move ${label} down`} title="Move down" onClick={() => moveItem(i, i + 1)} disabled={i === parts.length - 1}>
        <Icon name="ui-chevron-down" size={16} />
      </button>
      <button className="icon" aria-label={`Remove ${label}`} title="Remove" onClick={() => remove(i)}>
        <Icon name="ui-delete" size={16} />
      </button>
    </span>
  );
  const dragProps = (i: number) => ({
    draggable: true,
    onDragStart: () => setDragFrom(i),
    onDragOver: (e: React.DragEvent) => e.preventDefault(),
    onDrop: (e: React.DragEvent) => {
      e.stopPropagation();
      drop(i);
    },
    onDragEnd: () => setDragFrom(null),
  });

  function box(i: number) {
    const p = parts[i];
    const isSel = selected === i;
    const cls = 'qbox' + (isSel ? ' selected' : '') + (dragFrom === i ? ' dragging' : '');
    const pick = (
      <button className="qbox-pick" aria-expanded={isSel} onClick={() => setSelected(isSel ? null : i)}>
        {p.kind === 'scale' ? `${scaleNo[i]}. ${partLabel(p)}` : partLabel(p)}
      </button>
    );
    if (p.kind === 'pageBreak') {
      return (
        <li key={i} className={cls + ' qbreak'} {...dragProps(i)}>
          <span className="drag-handle" aria-hidden="true">
            <Icon name="ui-drag" size={16} />
          </span>
          <span className="qbreak-line" />
          <span className="qkind">Page break</span>
          <span className="qbreak-line" />
          {moveButtons(i, 'page break')}
        </li>
      );
    }
    return (
      <li key={i} className={cls} {...dragProps(i)}>
        <div className="qbox-head">
          <span className="drag-handle" aria-hidden="true">
            <Icon name="ui-drag" size={16} />
          </span>
          <div className="qbox-main">
            {p.kind !== 'scale' && <span className="qkind">{p.kind === 'transition' ? 'Transition' : p.kind === 'demographics' ? 'Demographics' : ''}</span>}
            {p.kind === 'scale' ? (
              <>
                <span className="qbox-title">
                  {pick}
                  {isPractice(p.scale) && <span className="badge note-badge">Practice scale</span>}
                </span>
                <span className="qmeta">
                  {FORMAT_LABEL[p.scale.format]}, {p.scale.items.length} items, {p.scale.response.points} points.{' '}
                  {numbers[i] && `Items ${numbers[i][0]} to ${numbers[i][1]}.`}
                </span>
              </>
            ) : p.kind === 'transition' ? (
              <span className="qbox-title">
                {pick}
                <span className="qpreview">{p.text.trim() || 'Empty: select it to write the sentence.'}</span>
              </span>
            ) : (
              <span className="qbox-title">
                {pick}
                {p.kind === 'demographics' && (
                  <span className="qmeta">{p.text.split('\n').filter((l) => l.trim() && !l.trim().startsWith('☐')).length} questions</span>
                )}
              </span>
            )}
          </div>
          {p.kind === 'scale' && checks[i] !== undefined && (
            <span className={'badge ' + (checks[i] ? 'warn' : 'ok-badge')}>
              <Icon name={checks[i] ? 'st-warning' : 'st-success'} size={14} />
              {checks[i] ? `${checks[i]} to look at` : 'Self-check clear'}
            </span>
          )}
          {p.kind === 'scale' && (
            <button className="small" onClick={() => openScale(p.scale, i)}>
              <Icon name="ui-edit" size={16} />
              Open
            </button>
          )}
          {moveButtons(i, partLabel(p))}
        </div>
        {isSel && p.kind === 'transition' && (
          <label className="field qedit">
            <span className="field-label">Transition sentence</span>
            <input autoFocus value={p.text} placeholder="The next questions ask about…" onChange={(e) => setPart(i, { ...p, text: e.target.value })} />
          </label>
        )}
        {isSel && p.kind === 'demographics' && (
          <div className="qedit">
            <label className="field">
              <span className="field-label">Heading</span>
              <input value={p.title} onChange={(e) => setPart(i, { ...p, title: e.target.value })} />
            </label>
            <label className="field">
              <span className="field-label">Questions, one per line</span>
              <textarea rows={Math.min(16, Math.max(5, p.text.split('\n').length + 1))} value={p.text} onChange={(e) => setPart(i, { ...p, text: e.target.value })} />
              <span className="hint">A line starting with ☐ prints as an answer choice under the question above it.</span>
            </label>
            {bank.length > 0 && (
              <label className="field">
                <span className="field-label">Add a ready-made question</span>
                <select
                  value=""
                  onChange={(e) => {
                    const q = bank.find((x) => x.id === e.target.value);
                    if (q) setPart(i, { ...p, text: p.text.trim() ? `${p.text.trimEnd()}\n${q.text}` : q.text });
                  }}
                >
                  <option value="">Choose a question…</option>
                  {[...new Set(bank.map((q) => q.group))].map((g) => (
                    <optgroup key={g} label={g}>
                      {bank
                        .filter((q) => q.group === g)
                        .map((q) => (
                          <option key={q.id} value={q.id}>
                            {q.label}
                          </option>
                        ))}
                    </optgroup>
                  ))}
                </select>
              </label>
            )}
          </div>
        )}
        {p.kind === 'scale' && p.libraryPath && isSel && <span className="reason">From the library: {p.libraryPath}</span>}
      </li>
    );
  }

  return (
    <div className="qtab">
      <aside className="qadd panel" aria-labelledby="qadd-h">
        <h2 id="qadd-h">Add to the questionnaire</h2>
        <label className="field">
          <span className="field-label">
            <Icon name="cmd-search" size={14} />
            Search your library
          </span>
          <input type="search" value={find} placeholder="Title or tag" onChange={(e) => setFind(e.target.value)} />
        </label>
        <h3 className="qgroup">Your library</h3>
        {libraryShown.length === 0 ? (
          <p className="muted">{library.length ? 'No scale matches.' : 'No scales saved yet.'}</p>
        ) : (
          <ul className="qlist">
            {libraryShown.map((e) => (
              <li key={e.path}>
                <span className="qlist-text">
                  <span className="lib-title">{e.title || '(untitled)'}</span>
                  <span className="lib-meta">
                    {FORMAT_LABEL[e.format]}, {e.items} items
                  </span>
                </span>
                <button className="small" onClick={() => addLibraryScale(e.path)} aria-label={`Add ${e.title} to the questionnaire`}>
                  Add
                </button>
              </li>
            ))}
          </ul>
        )}
        {practice.length > 0 && (
          <>
            <h3 className="qgroup">Practice scales</h3>
            <p className="hint">Made-up scales that come with Prism Survey, to try the program. They are not tested measures.</p>
            <ul className="qlist">
              {practice.map((e) => (
                <li key={e.path}>
                  <span className="qlist-text">
                    <span className="lib-title">{e.title}</span>
                    <span className="lib-meta">
                      {FORMAT_LABEL[e.format]}, {e.items} items
                    </span>
                  </span>
                  <button className="small" onClick={() => addLibraryScale(e.path)} aria-label={`Add ${e.title} to the questionnaire`}>
                    Add
                  </button>
                </li>
              ))}
            </ul>
            {sampleFolder && (
              <button className="small" onClick={() => onOpenSample(`${sampleFolder}/Sample Questionnaire.pstudy`)}>
                Open the sample questionnaire
              </button>
            )}
          </>
        )}
        <h3 className="qgroup">Other ways in</h3>
        <div className="buttons wrap">
          {currentScale && (
            <button className="small" onClick={() => set({ parts: [...parts, { kind: 'scale', scale: currentScale }] })}>
              <Icon name="study-add" size={16} />
              Add the scale on the form
            </button>
          )}
          <button
            className="small"
            onClick={async () => {
              const found = await importScale();
              if (found.length) set({ parts: [...parts, ...found.map((scale): Part => ({ kind: 'scale', scale }))] });
            }}
          >
            <Icon name="cmd-import" size={16} />
            Import a stored scale or questionnaire…
          </button>
        </div>
        <h3 className="qgroup">Blocks</h3>
        <div className="qblocks">
          <button onClick={() => addCoverLetter('short')} disabled={hasCover} title={hasCover ? 'The questionnaire already has a cover letter' : undefined}>
            <Icon name="ui-plus" size={16} />
            Short cover letter
          </button>
          <button onClick={() => addCoverLetter('long')} disabled={hasCover} title={hasCover ? 'The questionnaire already has a cover letter' : undefined}>
            <Icon name="ui-plus" size={16} />
            Long cover letter
          </button>
          <button onClick={() => add({ kind: 'section', title: `Section ${sectionCount + 1}`, newPage: sectionCount > 0 })}>
            <Icon name="ui-plus" size={16} />
            Section
          </button>
          <button onClick={() => add({ kind: 'transition', text: '' })}>
            <Icon name="study-transition" size={16} />
            Transition
          </button>
          <button onClick={() => add({ kind: 'demographics', title: 'About you', text: 'What is your age? ______\nWhat is your gender? ______' })}>
            <Icon name="study-demographics" size={16} />
            Demographics
          </button>
        </div>
        <button
          className="small"
          onClick={async () => {
            try {
              const r = await call<{ study: Study; changed: string[] }>('study.refresh', { study });
              setStudy(r.study);
              if (!r.changed.length) alert('Every scale already matches its library file.');
            } catch (e) {
              fail(e);
            }
          }}
        >
          <Icon name="study-refresh" size={16} />
          Bring scales up to date from the library
        </button>
      </aside>

      <div className="qmain">
        <section className="panel qtitle" aria-labelledby="qtitle-h">
          <h2 id="qtitle-h" className="sr-only">
            Questionnaire
          </h2>
          <label className="field">
            <span className="field-label">Questionnaire title</span>
            <input value={study.title} onChange={(e) => set({ title: e.target.value })} />
          </label>
          {length && (
            <span className="qchips">
              <span className="qchip">{Object.keys(scaleNo).length} scales</span>
              <span className="qchip">{length.items} items</span>
              <span className="qchip">
                <Icon name="ui-clock" size={14} />
                about {length.minutes} minutes
              </span>
            </span>
          )}
        </section>

        {hasCover && parts[0].kind === 'coverLetter' && (
          <section className={'panel qcover' + (selected === 0 ? ' selected' : '')} aria-label="Cover letter">
            <div className="qbox-head">
              <div className="qbox-main">
                <span className="qkind">Cover letter, always first, on its own page</span>
                <span className="qbox-title">
                  <button className="qbox-pick" aria-expanded={selected === 0} onClick={() => setSelected(selected === 0 ? null : 0)}>
                    {parts[0].title || 'Cover letter'}
                  </button>
                </span>
              </div>
              <span className={'badge ' + (left.length ? 'warn' : 'ok-badge')}>
                <Icon name={left.length ? 'st-warning' : 'st-success'} size={14} />
                {left.length ? `${left.length} parts to fill in` : 'Filled in'}
              </span>
              <button className="icon" aria-label="Remove the cover letter" title="Remove" onClick={() => remove(0)}>
                <Icon name="ui-delete" size={16} />
              </button>
            </div>
            {selected === 0 && (
              <div className="qedit">
                <label className="field">
                  <span className="field-label">Heading</span>
                  <input value={parts[0].title} onChange={(e) => setPart(0, { ...(parts[0] as Extract<Part, { kind: 'coverLetter' }>), title: e.target.value })} />
                </label>
                <label className="field">
                  <span className="field-label">The letter, one paragraph per line</span>
                  <textarea
                    rows={Math.min(24, Math.max(8, parts[0].text.split('\n').length + 2))}
                    value={parts[0].text}
                    onChange={(e) => setPart(0, { ...(parts[0] as Extract<Part, { kind: 'coverLetter' }>), text: e.target.value })}
                  />
                  <span className="hint">
                    A line starting with ## prints as a heading. The template covers the consent elements of 45 CFR 46.116 that apply to a survey; your IRB's own template and
                    wording come first.
                  </span>
                </label>
                {left.length > 0 && (
                  <div className="finding">
                    <strong>Still to fill in or choose ({left.length}):</strong> {left.map((x) => (x.length > 60 ? `${x.slice(0, 57)}…]` : x)).join('; ')}
                  </div>
                )}
              </div>
            )}
          </section>
        )}

        {parts.length === first && <p className="qdrop">Add scales and blocks from the panel on the left. They appear here as boxes you can drag into order.</p>}

        {groups.map((g, gi) => {
          if (g.section === null) {
            if (!g.items.length) return null;
            return (
              <section key="loose" className="panel qsection" aria-label="Not in a section">
                <ol className="qboxes">{g.items.map(box)}</ol>
              </section>
            );
          }
          const si = g.section;
          const s = parts[si] as Extract<Part, { kind: 'section' }>;
          const isSel = selected === si;
          const n = groups.slice(0, gi + 1).filter((x) => x.section !== null).length;
          const scalesHere = g.items.filter((i) => parts[i].kind === 'scale');
          return (
            <Fragment key={si}>
              {s.newPage && (
                <div className="qpage" aria-hidden="true">
                  <span className="qbreak-line" />
                  New page
                  <span className="qbreak-line" />
                </div>
              )}
              <section className={'panel qsection' + (isSel ? ' selected' : '') + (dragFrom === si ? ' dragging' : '')} aria-label={`Section ${n}, ${s.title}`} onDragOver={(e) => e.preventDefault()} onDrop={() => drop(si)}>
                <header className="qsection-head" {...dragProps(si)}>
                  <span className="drag-handle" aria-hidden="true">
                    <Icon name="ui-drag" size={18} />
                  </span>
                  <span className="qkind">Section {n}</span>
                  <h2>
                    <button className="qbox-pick" aria-expanded={isSel} onClick={() => setSelected(isSel ? null : si)}>
                      {s.title || '(no heading)'}
                    </button>
                  </h2>
                  <span className="qmeta">
                    {scalesHere.length} {scalesHere.length === 1 ? 'scale' : 'scales'}, {scalesHere.reduce((a, i) => a + (parts[i] as Extract<Part, { kind: 'scale' }>).scale.items.length, 0)} items
                  </span>
                  <span className="qmoves">
                    <button className="icon" aria-label={`Move section ${n} up`} title="Move the section up" onClick={() => moveSection(si, -1)} disabled={si <= first}>
                      <Icon name="ui-chevron-down" size={16} turn={180} />
                    </button>
                    <button className="icon" aria-label={`Move section ${n} down`} title="Move the section down" onClick={() => moveSection(si, 1)} disabled={blockEnd(parts, si) >= parts.length}>
                      <Icon name="ui-chevron-down" size={16} />
                    </button>
                    <button className="icon" aria-label={`Remove the heading of section ${n}`} title="Remove the section heading; its boxes stay" onClick={() => remove(si)}>
                      <Icon name="ui-delete" size={16} />
                    </button>
                  </span>
                </header>
                {isSel && (
                  <div className="qedit row">
                    <label className="field">
                      <span className="field-label">Section heading</span>
                      <input autoFocus value={s.title} onChange={(e) => setPart(si, { ...s, title: e.target.value })} />
                    </label>
                    <label className="check align-end">
                      <input type="checkbox" checked={s.newPage} onChange={(e) => setPart(si, { ...s, newPage: e.target.checked })} />
                      Start this section on a new page
                    </label>
                  </div>
                )}
                {g.items.length ? <ol className="qboxes">{g.items.map(box)}</ol> : <p className="qdrop">Drag boxes here, or move them with their arrows.</p>}
              </section>
            </Fragment>
          );
        })}
      </div>

      <aside className="qside">
        {sel && sel.kind === 'scale' && selected !== null && (
          <section className="panel" aria-labelledby="qsel-h">
            <span className="qkind">Selected</span>
            <h2 id="qsel-h">{sel.scale.title || '(untitled scale)'}</h2>
            <dl className="qfacts">
              <dt>Format</dt>
              <dd>{FORMAT_LABEL[sel.scale.format]}</dd>
              <dt>Items</dt>
              <dd>
                {sel.scale.items.length}
                {numbers[selected] && `, numbered ${numbers[selected][0]} to ${numbers[selected][1]}`}
              </dd>
              <dt>Points</dt>
              <dd>
                {sel.scale.response.points}, labels {sel.scale.response.labelMode === 'every' ? 'on every point' : 'on the ends'}
              </dd>
              <dt>Licence</dt>
              <dd>{sel.scale.licence.status === 'open' ? 'Open' : sel.scale.licence.status === 'permissioned' ? 'Permission on file' : sel.scale.licence.status === 'localOnly' ? 'This computer only' : 'Not known'}</dd>
            </dl>
            {sel.scale.licence.note && <p className="hint">{sel.scale.licence.note}</p>}
            <div className="buttons wrap">
              <button className="primary" onClick={() => openScale(sel.scale, selected)}>
                <Icon name="ui-edit" size={16} />
                Open on the scale form
              </button>
              <button onClick={() => remove(selected)}>
                <Icon name="ui-delete" size={16} />
                Remove
              </button>
            </div>
          </section>
        )}

        <section className="panel" aria-labelledby="opts-h">
          <h2 id="opts-h">How it prints</h2>
          <label className="field">
            <span className="field-label">Item numbers</span>
            <select value={study.options.numbering} onChange={(e) => setOpt({ numbering: e.target.value as Study['options']['numbering'] })}>
              <option value="restartEach">Start again at 1 for each scale</option>
              <option value="runOn">Run on through the questionnaire</option>
            </select>
          </label>
          <label className="field">
            <span className="field-label">Scale titles on the respondent copy</span>
            <select value={study.options.titles} onChange={(e) => setOpt({ titles: e.target.value as Study['options']['titles'] })}>
              <option value="show">Shown</option>
              <option value="neutral">Neutral labels ("Section 3")</option>
              <option value="hide">Hidden</option>
            </select>
          </label>
          {study.options.titles === 'neutral' && (
            <label className="field">
              <span className="field-label">Label word</span>
              <input value={study.options.neutralWord} onChange={(e) => setOpt({ neutralWord: e.target.value })} />
            </label>
          )}
          <label className="field">
            <span className="field-label">Sources</span>
            <select value={study.options.sources} onChange={(e) => setOpt({ sources: e.target.value as Study['options']['sources'] })}>
              <option value="underScale">Under each scale</option>
              <option value="references">Together in a References section at the end</option>
              <option value="none">Not printed</option>
            </select>
          </label>
          <div className="row">
            <label className="field">
              <span className="field-label">Font</span>
              <input value={study.options.font} onChange={(e) => setOpt({ font: e.target.value })} />
            </label>
            <label className="field qnarrow">
              <span className="field-label">Size (points)</span>
              <input type="number" min={8} max={16} value={study.options.sizePt} onChange={(e) => setOpt({ sizePt: parseInt(e.target.value, 10) || 12 })} />
            </label>
          </div>
          <label className="check">
            <input type="checkbox" checked={study.options.sectionHeadings ?? true} onChange={(e) => setOpt({ sectionHeadings: e.target.checked })} />
            Section headings on the respondent copy
          </label>
          <label className="check">
            <input type="checkbox" checked={study.options.breakBetweenScales} onChange={(e) => setOpt({ breakBetweenScales: e.target.checked })} />
            Start each scale on a new page
          </label>
        </section>

        <section className="panel" aria-labelledby="sugg-h">
          <h2 id="sugg-h">Suggestions</h2>
          <p className="muted">
            Measures that could sit beside the scales already here: a related area, a length that fits, the same format, or a pairing from your earlier questionnaires. Each
            comes with its reasons; whether the combination suits your design is your call.
          </p>
          <label className="field">
            <span className="field-label">Longest the questionnaire should take (minutes)</span>
            <input type="number" min={5} max={120} value={maxMinutes} onChange={(e) => setMaxMinutes(parseInt(e.target.value, 10) || 20)} />
          </label>
          <button className="small" onClick={suggest} disabled={!parts.some((p) => p.kind === 'scale')}>
            <Icon name="study-suggest" size={16} />
            Suggest measures
          </button>
          {recs && recs.length === 0 && <p className="muted">Nothing to suggest yet. Tags on your scales give the suggestions more to go on.</p>}
          {recs && recs.length > 0 && (
            <ul className="suggestions">
              {recs.map((r) => (
                <li key={r.key}>
                  <span className="lib-title">{r.title}</span> <span className="muted">({r.collection})</span>
                  <ul className="reasons">
                    {r.reasons.map((x, i) => (
                      <li key={i}>{x}</li>
                    ))}
                  </ul>
                  {r.key.startsWith('dir:') ? (
                    <button className="small" onClick={() => showDirectoryEntry(r.key.slice(4))}>
                      <Icon name="view-directory" size={16} />
                      Show in the Comm Directory
                    </button>
                  ) : (
                    <button
                      className="small"
                      onClick={() => {
                        addLibraryScale(r.key);
                        setRecs(recs.filter((x) => x.key !== r.key));
                      }}
                    >
                      <Icon name="study-add" size={16} />
                      Add to the questionnaire
                    </button>
                  )}
                </li>
              ))}
            </ul>
          )}
        </section>
      </aside>
    </div>
  );
}
