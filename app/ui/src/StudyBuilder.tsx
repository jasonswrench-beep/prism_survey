// The study builder (specification, "Building a study"): an ordered stack of scales, with page breaks, transition
// sentences and a demographic block between them, exported as one Word document in two copies. Parts are reordered by
// dragging or with the arrow buttons (keyboard).

import { useEffect, useState } from 'react';
import { call } from './bridge';
import type { Scale } from './types';
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
  | { kind: 'transition'; text: string }
  | { kind: 'demographics'; title: string; text: string };

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
  };
  history: { version: number; savedAt: string; summary: string }[];
}

function partLabel(p: Part): string {
  switch (p.kind) {
    case 'scale':
      return p.scale.title || '(untitled scale)';
    case 'pageBreak':
      return 'Page break';
    case 'transition':
      return 'Transition sentence';
    case 'demographics':
      return 'Demographic questions';
  }
}

interface Recommendation {
  key: string;
  title: string;
  collection: string;
  score: number;
  reasons: string[];
}

export function StudyBuilder({
  study,
  setStudy,
  openScale,
  currentScale,
  importScale,
  addLibraryScale,
  showDirectoryEntry,
  fail,
}: {
  study: Study;
  setStudy: (s: Study) => void;
  openScale: (s: Scale, index: number) => void;
  currentScale: Scale | null;
  importScale: () => Promise<Scale[]>;
  addLibraryScale: (path: string) => void;
  showDirectoryEntry: (id: string) => void;
  fail: (e: unknown) => void;
}) {
  const [length, setLength] = useState<{ items: number; minutes: number } | null>(null);
  const [dragFrom, setDragFrom] = useState<number | null>(null);
  const [maxMinutes, setMaxMinutes] = useState(20);
  const [recs, setRecs] = useState<Recommendation[] | null>(null);
  const [bank, setBank] = useState<BankQuestion[]>([]);

  useEffect(() => {
    call<BankQuestion[]>('demographics.bank')
      .then(setBank)
      .catch(() => setBank([]));
  }, []);

  async function suggest() {
    try {
      const r = await call<{ recommendations: Recommendation[] }>('study.recommend', { study, settings: { maxMinutes, limit: 10 } });
      setRecs(r.recommendations);
    } catch (e) {
      fail(e);
    }
  }

  useEffect(() => {
    call<{ items: number; minutes: number }>('study.length', { study }).then(setLength).catch(() => {});
  }, [study]);

  const set = (patch: Partial<Study>) => setStudy({ ...study, ...patch });
  const setOpt = (patch: Partial<Study['options']>) => setStudy({ ...study, options: { ...study.options, ...patch } });
  const setPart = (i: number, p: Part) => set({ parts: study.parts.map((x, k) => (k === i ? p : x)) });
  const remove = (i: number) => set({ parts: study.parts.filter((_, k) => k !== i) });
  const move = (from: number, to: number) => {
    if (to < 0 || to >= study.parts.length || from === to) return;
    const parts = [...study.parts];
    const [p] = parts.splice(from, 1);
    parts.splice(to, 0, p);
    set({ parts });
  };
  const add = (p: Part) => set({ parts: [...study.parts, p] });
  let scaleNumber = 0;

  return (
    <div className="form">
      <section className="panel" aria-labelledby="study-h">
        <h2 id="study-h">
          <Icon name="view-study" />
          Study
        </h2>
        <div className="row">
          <label className="field">
            <span className="field-label">Study title</span>
            <input value={study.title} onChange={(e) => set({ title: e.target.value })} />
          </label>
        </div>
        {length && (
          <p className="muted">
            <Icon name="ui-clock" size={14} /> {study.parts.filter((p) => p.kind === 'scale').length} scales, {length.items} items, about {length.minutes} minutes to complete.
          </p>
        )}
      </section>

      <section className="panel" aria-labelledby="parts-h">
        <h2 id="parts-h">In order</h2>
        {study.parts.length === 0 && <p className="muted">Add scales from the library (the Add buttons beside each scale), or with the buttons below.</p>}
        <ol className="parts">
          {study.parts.map((p, i) => {
            const n = p.kind === 'scale' ? ++scaleNumber : 0;
            return (
              <li
                key={i}
                className={'part' + (dragFrom === i ? ' dragging' : '')}
                draggable
                onDragStart={() => setDragFrom(i)}
                onDragOver={(e) => e.preventDefault()}
                onDrop={() => {
                  if (dragFrom !== null) move(dragFrom, i);
                  setDragFrom(null);
                }}
                onDragEnd={() => setDragFrom(null)}
              >
                <div className="part-head">
                  <span className="drag-handle" aria-hidden="true">
                    <Icon name="ui-drag" size={16} />
                  </span>
                  <strong>{p.kind === 'scale' ? `${n}. ${partLabel(p)}` : partLabel(p)}</strong>
                  {p.kind === 'scale' && (
                    <span className="muted">
                      {' '}
                      {FORMAT_LABEL[p.scale.format]}, {p.scale.items.length} items
                    </span>
                  )}
                  <span className="part-actions">
                    {p.kind === 'scale' && (
                      <button className="small" onClick={() => openScale(p.scale, i)}>
                        <Icon name="ui-edit" size={16} />
                        Edit
                      </button>
                    )}
                    <button className="icon" aria-label={`Move ${partLabel(p)} up`} title="Move up" onClick={() => move(i, i - 1)} disabled={i === 0}>
                      <Icon name="ui-chevron-down" size={16} turn={180} />
                    </button>
                    <button className="icon" aria-label={`Move ${partLabel(p)} down`} title="Move down" onClick={() => move(i, i + 1)} disabled={i === study.parts.length - 1}>
                      <Icon name="ui-chevron-down" size={16} />
                    </button>
                    <button className="icon" aria-label={`Remove ${partLabel(p)}`} onClick={() => remove(i)}>
                      <Icon name="ui-delete" size={16} />
                      Remove
                    </button>
                  </span>
                </div>
                {p.kind === 'transition' && (
                  <label className="field">
                    <span className="sr-only">Transition sentence</span>
                    <input value={p.text} placeholder="The next questions ask about…" onChange={(e) => setPart(i, { ...p, text: e.target.value })} />
                  </label>
                )}
                {p.kind === 'demographics' && (
                  <>
                    <label className="field">
                      <span className="field-label">Heading</span>
                      <input value={p.title} onChange={(e) => setPart(i, { ...p, title: e.target.value })} />
                    </label>
                    <label className="field">
                      <span className="field-label">Questions, one per line</span>
                      <textarea
                        rows={Math.min(16, Math.max(5, p.text.split('\n').length + 1))}
                        value={p.text}
                        onChange={(e) => setPart(i, { ...p, text: e.target.value })}
                      />
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
                  </>
                )}
                {p.kind === 'scale' && p.libraryPath && <span className="reason">From the library: {p.libraryPath}</span>}
              </li>
            );
          })}
        </ol>
        <div className="buttons wrap">
          {currentScale && (
            <button className="small" onClick={() => add({ kind: 'scale', scale: currentScale })}>
              <Icon name="study-add" size={16} />
              Add the scale on the form
            </button>
          )}
          <button
            className="small"
            onClick={async () => {
              const found = await importScale();
              if (found.length) set({ parts: [...study.parts, ...found.map((scale): Part => ({ kind: 'scale', scale }))] });
            }}
          >
            <Icon name="cmd-import" size={16} />
            Import a stored scale or questionnaire…
          </button>
          <button className="small" onClick={() => add({ kind: 'pageBreak' })}>
            <Icon name="study-page-break" size={16} />
            Add a page break
          </button>
          <button className="small" onClick={() => add({ kind: 'transition', text: '' })}>
            <Icon name="study-transition" size={16} />
            Add a transition sentence
          </button>
          <button className="small" onClick={() => add({ kind: 'demographics', title: 'About you', text: 'What is your age? ______\nWhat is your gender? ______' })}>
            <Icon name="study-demographics" size={16} />
            Add demographic questions
          </button>
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
        </div>
      </section>

      <section className="panel" aria-labelledby="sugg-h">
        <h2 id="sugg-h">Suggestions</h2>
        <p className="muted">
          Measures that could sit beside the scales already here: a related area, a length that fits, the same format, or
          a pairing from your earlier studies. Each comes with its reasons; whether the combination suits your design is
          your call.
        </p>
        <div className="row">
          <label className="field">
            <span className="field-label">Longest the questionnaire should take (minutes)</span>
            <input type="number" min={5} max={120} value={maxMinutes} onChange={(e) => setMaxMinutes(parseInt(e.target.value, 10) || 20)} />
          </label>
        </div>
        <div className="buttons">
          <button className="small" onClick={suggest} disabled={!study.parts.some((p) => p.kind === 'scale')}>
            <Icon name="study-suggest" size={16} />
            Suggest measures
          </button>
        </div>
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
                    Add to the study
                  </button>
                )}
              </li>
            ))}
          </ul>
        )}
      </section>

      <section className="panel" aria-labelledby="opts-h">
        <h2 id="opts-h">How it prints</h2>
        <div className="row">
          <label className="field">
            <span className="field-label">Item numbers</span>
            <select value={study.options.numbering} onChange={(e) => setOpt({ numbering: e.target.value as Study['options']['numbering'] })}>
              <option value="restartEach">Start again at 1 for each scale</option>
              <option value="runOn">Run on through the study</option>
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
        </div>
        <div className="row">
          <label className="field">
            <span className="field-label">Sources</span>
            <select value={study.options.sources} onChange={(e) => setOpt({ sources: e.target.value as Study['options']['sources'] })}>
              <option value="underScale">Under each scale</option>
              <option value="references">Together in a References section at the end</option>
              <option value="none">Not printed</option>
            </select>
          </label>
          <label className="field">
            <span className="field-label">Font</span>
            <input value={study.options.font} onChange={(e) => setOpt({ font: e.target.value })} />
          </label>
          <label className="field">
            <span className="field-label">Size (points)</span>
            <input type="number" min={8} max={16} value={study.options.sizePt} onChange={(e) => setOpt({ sizePt: parseInt(e.target.value, 10) || 12 })} />
          </label>
        </div>
        <label className="check">
          <input type="checkbox" checked={study.options.breakBetweenScales} onChange={(e) => setOpt({ breakBetweenScales: e.target.checked })} />
          Start each scale on a new page
        </label>
      </section>
    </div>
  );
}
