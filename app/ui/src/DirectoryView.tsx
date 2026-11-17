// The Comm Add On: the Directory of Communication Related Mental Measures, searchable, with each entry's description
// and the ways to bring the measure itself into the library (specification addendum, section 4).

import { useEffect, useState } from 'react';
import type React from 'react';
import { call } from './bridge';

export interface Entry {
  id: string;
  name: string;
  chapter: string;
  purpose: string;
  numberOfItems: string;
  format: string;
  originalSample: string;
  reliability: string;
  validity: string;
  critique: string;
  critiqueUpdated?: string;
  source: string;
  sourceApa7: string;
  doi?: string;
  relatedResearch?: string[];
  categories: string[];
  referenceCheck?: string;
}

interface Info {
  installed: boolean;
  title?: string;
  authors?: string[];
  entries?: number;
  chapters?: string[];
  hasIntroduction?: boolean;
}

interface IntroBlock {
  kind: 'heading' | 'paragraph' | 'item' | 'reference';
  text: string;
}

/// The book's preface, shown as the add-on's introduction. Runs of list lines are drawn as one list.
function Introduction({ blocks, onClose }: { blocks: IntroBlock[]; onClose: () => void }) {
  const out: React.ReactNode[] = [];
  let list: string[] = [];
  const flush = (k: number) => {
    if (list.length) {
      out.push(
        <ul key={`l${k}`} className="intro-list">
          {list.map((t, i) => (
            <li key={i}>{t}</li>
          ))}
        </ul>,
      );
      list = [];
    }
  };
  blocks.forEach((b, k) => {
    if (b.kind === 'item') {
      list.push(b.text);
      return;
    }
    flush(k);
    if (b.kind === 'heading') out.push(<h3 key={k}>{b.text}</h3>);
    else if (b.kind === 'reference') out.push(<p key={k} className="hanging">{b.text}</p>);
    else out.push(<p key={k}>{b.text}</p>);
  });
  flush(blocks.length);
  return (
    <section className="panel directory-entry introduction" aria-labelledby="intro-h">
      <h2 id="intro-h">About this directory</h2>
      <p className="muted">The preface of the book, by its authors.</p>
      {out}
      <div className="buttons">
        <button onClick={onClose}>Close</button>
      </div>
    </section>
  );
}

interface Hit {
  key: string;
  title: string;
  snippet: string;
  field: string;
}

export function DirectoryView({
  initialEntry,
  onFindInArticle,
  onImportStored,
  onStartBlank,
  fail,
}: {
  initialEntry?: string | null;
  onFindInArticle: (e: Entry) => void;
  onImportStored: (e: Entry) => void;
  onStartBlank: (e: Entry) => void;
  fail: (e: unknown) => void;
}) {
  const [info, setInfo] = useState<Info | null>(null);
  const [query, setQuery] = useState('');
  const [chapter, setChapter] = useState('');
  const [hits, setHits] = useState<Hit[]>([]);
  const [entry, setEntry] = useState<Entry | null>(null);
  const [intro, setIntro] = useState<IntroBlock[] | null>(null);

  async function showIntro() {
    try {
      const r = await call<{ blocks: IntroBlock[] }>('directory.introduction');
      setEntry(null);
      setIntro(r.blocks);
    } catch (e) {
      fail(e);
    }
  }

  useEffect(() => {
    call<Info>('directory.info').then(setInfo).catch(fail);
  }, [fail]);

  useEffect(() => {
    if (initialEntry) call<Entry>('directory.entry', { id: initialEntry }).then(setEntry).catch(fail);
  }, [initialEntry, fail]);

  useEffect(() => {
    if (!info?.installed) return;
    const t = setTimeout(() => {
      call<{ hits: Hit[] }>('directory.search', { query, chapter: chapter || null })
        .then((r) => setHits(r.hits))
        .catch(fail);
    }, 150);
    return () => clearTimeout(t);
  }, [info, query, chapter, fail]);

  async function open(id: string) {
    try {
      setIntro(null);
      setEntry(await call<Entry>('directory.entry', { id }));
    } catch (e) {
      fail(e);
    }
  }

  if (!info) return <p className="muted">Opening the Comm Directory…</p>;
  if (!info.installed) {
    return (
      <section className="welcome">
        <h1>Comm Directory</h1>
        <p>The Comm Add On is not installed.</p>
      </section>
    );
  }

  return (
    <div className="directory">
      <section className="panel directory-list" aria-labelledby="dir-h">
        <h2 id="dir-h">{info.title}</h2>
        <p className="muted">
          {info.entries} measures. {info.authors?.join(', ')}.
        </p>
        {info.hasIntroduction && (
          <div className="buttons">
            <button className="small" onClick={showIntro} aria-pressed={!!intro}>
              About this directory
            </button>
          </div>
        )}
        <div className="row">
          <label className="field">
            <span className="field-label">Search</span>
            <input type="search" value={query} placeholder='Words, "a phrase", purpose:, author:' onChange={(e) => setQuery(e.target.value)} />
          </label>
          <label className="field">
            <span className="field-label">Chapter</span>
            <select value={chapter} onChange={(e) => setChapter(e.target.value)}>
              <option value="">All chapters</option>
              {info.chapters?.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
          </label>
        </div>
        <p className="muted">{hits.length} shown</p>
        <ul className="dir-hits">
          {hits.map((h) => (
            <li key={h.key}>
              <button className={'lib-entry' + (entry?.id === h.key ? ' current' : '')} onClick={() => open(h.key)}>
                <span className="lib-title">{h.title}</span>
                {query.trim() && h.field !== 'title' && h.snippet && <span className="lib-snippet">{h.snippet}</span>}
              </button>
            </li>
          ))}
        </ul>
      </section>

      {intro && !entry && <Introduction blocks={intro} onClose={() => setIntro(null)} />}
      {entry && (
        <section className="panel directory-entry" aria-labelledby="entry-h">
          <h2 id="entry-h">{entry.name}</h2>
          <p className="muted">
            {entry.chapter}
            {entry.categories.filter((c) => c !== entry.chapter).length
              ? ` · also ${entry.categories.filter((c) => c !== entry.chapter).join(', ')}`
              : ''}
          </p>
          <dl className="entry-fields">
            <dt>Purpose</dt>
            <dd>{entry.purpose}</dd>
            <dt>Number of items</dt>
            <dd>{entry.numberOfItems || 'Not given'}</dd>
            <dt>Format</dt>
            <dd>{entry.format}</dd>
            <dt>Original sample</dt>
            <dd>{entry.originalSample}</dd>
            <dt>Reliability</dt>
            <dd>{entry.reliability}</dd>
            <dt>Validity</dt>
            <dd>{entry.validity}</dd>
            <dt>Critique</dt>
            <dd>{entry.critiqueUpdated ?? entry.critique}</dd>
            <dt>Source (APA 7)</dt>
            <dd className="hanging">{entry.sourceApa7}</dd>
            {entry.referenceCheck && (
              <dd className={entry.referenceCheck.startsWith('Check') ? 'finding' : 'muted'}>{entry.referenceCheck}</dd>
            )}
            {entry.relatedResearch && entry.relatedResearch.length > 0 && (
              <>
                <dt>Related research</dt>
                <dd>
                  {entry.relatedResearch.map((r, i) => (
                    <p key={i} className="hanging">
                      {r}
                    </p>
                  ))}
                </dd>
              </>
            )}
          </dl>
          <p className="muted">The directory describes the measure; its items are in the source article. Bring the measure in one of these ways:</p>
          <div className="buttons wrap">
            <button onClick={() => onFindInArticle(entry)}>Find it in the article (PDF)…</button>
            <button onClick={() => onImportStored(entry)}>Import a stored copy…</button>
            <button onClick={() => onStartBlank(entry)}>Type it into a blank scale</button>
            <button
              className="small"
              onClick={() => {
                navigator.clipboard.writeText(entry.sourceApa7).catch(() => {});
              }}
            >
              Copy the reference
            </button>
          </div>
        </section>
      )}
    </div>
  );
}
