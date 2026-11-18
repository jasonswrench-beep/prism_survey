// The Prism Survey window for phase 1: the scale library, the three blank formats, import of stored scales, and the
// confirm and correct form (every parsed field in one place before anything is saved). The page image view of the full
// confirm screen comes with journal article detection in phase 3.

import { Fragment, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { call, errorText, pickFile, pickFolder, pickSave } from './bridge';
import type {
  AnchorSet,
  ChangeOutcome,
  Finding,
  NoteLine,
  ParentChange,
  Fix,
  Check,
  Format,
  ImportReport,
  Item,
  ItemSelector,
  LabelMode,
  LibraryEntry,
  LicenceStatus,
  ReverseState,
  Scale,
  ScoreMethod,
  Settings,
  Subscale,
  TagCount,
  TotalFrom,
} from './types';
import { FORMAT_LABEL } from './types';
import icon from './assets/icon.svg';
import { ArticlePane, fieldId, type ArticlePage } from './ArticlePane';
import { StudyBuilder, type Study } from './StudyBuilder';
import { DirectoryView, type Entry } from './DirectoryView';
import { VariantPanel, VariantView } from './VariantView';
import { Icon, type IconName } from './Icon';

/// The icon beside each format, on the New buttons and in the library list (icon brief, "The three formats").
const FORMAT_ICON: Record<Format, IconName> = {
  likertBlankLine: 'fmt-blank-line',
  likertBoxGrid: 'fmt-box-grid',
  semanticDifferential: 'fmt-semantic',
};

type Status = { kind: 'info' | 'error'; text: string } | null;

function describeSelector(s: ItemSelector): string {
  switch (s.kind) {
    case 'list':
      return s.numbers.join(', ');
    case 'odd':
      return 'odd';
    case 'even':
      return 'even';
    case 'block':
      return `${s.first}-${s.last}`;
  }
}

function safeName(s: string): string {
  const t = s.replace(/[<>:"/\\|?*\u0000-\u001f]/g, '_').trim().replace(/\.+$/, '');
  return t || 'Untitled scale';
}

function joinPath(folder: string, name: string): string {
  return folder.replace(/[\\/]+$/, '') + '\\' + name;
}

function fmtRange(r: [number, number] | null | undefined): string {
  if (!r) return 'none';
  const f = (x: number) => (Number.isInteger(x) ? String(x) : x.toFixed(2));
  return `${f(r[0])} to ${f(r[1])}`;
}

function renumber(items: Item[]): Item[] {
  return items.map((it, i) => ({ ...it, number: i + 1 }));
}

const REVERSE_LABEL: Record<ReverseState, string> = {
  notReversed: 'Not reversed',
  suggested: 'Suggested',
  confirmed: 'Reversed',
};

const LICENCE_LABEL: Record<LicenceStatus, string> = {
  open: 'Open',
  permissioned: 'Permissioned',
  localOnly: 'Local only',
  unknown: 'Unknown',
};

export function App() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [library, setLibrary] = useState<LibraryEntry[]>([]);
  const [scale, setScale] = useState<Scale | null>(null);
  const [path, setPath] = useState<string | null>(null);
  const [dirty, setDirty] = useState(false);
  const [notes, setNotes] = useState<string[]>([]);
  const [choices, setChoices] = useState<ImportReport[] | null>(null);
  const [check, setCheck] = useState<Check | null>(null);
  const [status, setStatus] = useState<Status>(null);
  const [libraryTags, setLibraryTags] = useState<TagCount[]>([]);
  const [tagFilter, setTagFilter] = useState('');
  const [query, setQuery] = useState('');
  const [formatFilter, setFormatFilter] = useState('');
  const [pointsFilter, setPointsFilter] = useState('');
  const [hits, setHits] = useState<SearchHit[] | null>(null);
  const [article, setArticle] = useState<OpenArticle | null>(null);
  const [showCandidates, setShowCandidates] = useState(false);
  const [selectedField, setSelectedField] = useState<string | null>(null);
  const [benchmark, setBenchmark] = useState(false);
  const [study, setStudy] = useState<Study | null>(null);
  const [studyPath, setStudyPath] = useState<string | null>(null);
  const [studyDirty, setStudyDirty] = useState(false);
  const [view, setView] = useState<'scale' | 'study' | 'directory' | 'variant'>('scale');
  const [variantOf, setVariantOf] = useState<{ path: string; title: string } | null>(null);
  const [parentChanges, setParentChanges] = useState<{ parentPath: string | null; changes: ParentChange[] } | null>(null);
  const [directoryEntry, setDirectoryEntry] = useState<string | null>(null);
  /// When a study's scale is open on the form: its place in the study, so "Back to the study" puts the edits back.
  const [editingPart, setEditingPart] = useState<number | null>(null);
  const formRef = useRef<HTMLDivElement>(null);

  const say = (text: string) => setStatus({ kind: 'info', text });
  const fail = (e: unknown) => setStatus({ kind: 'error', text: errorText(e) });

  const refreshLibrary = useCallback(async (folder?: string) => {
    try {
      setLibrary(await call<LibraryEntry[]>('library.list', folder ? { folder } : {}));
      setLibraryTags(await call<TagCount[]>('library.tags', folder ? { folder } : {}));
    } catch (e) {
      fail(e);
    }
  }, []);

  useEffect(() => {
    call<Settings>('settings.get').then(setSettings).catch(fail);
    refreshLibrary();
  }, [refreshLibrary]);

  useEffect(() => {
    const filters: Record<string, unknown> = {};
    if (tagFilter) filters.tag = tagFilter;
    if (formatFilter) filters.format = formatFilter;
    if (pointsFilter) filters.points = parseInt(pointsFilter, 10);
    const t = setTimeout(() => {
      call<{ hits: SearchHit[] }>('library.search', { query, filters })
        .then((r) => setHits(r.hits))
        .catch(() => setHits(null));
    }, 150);
    return () => clearTimeout(t);
  }, [library, query, tagFilter, formatFilter, pointsFilter]);

  async function renameTag() {
    if (!tagFilter) return;
    const to = window.prompt(`Rename the tag "${tagFilter}" on every scale that has it. A name already in use merges the two.`, tagFilter);
    if (!to || to.trim() === tagFilter) return;
    try {
      const r = await call<{ changed: string[] }>('library.renameTag', { from: tagFilter, to });
      setTagFilter(to.trim());
      await refreshLibrary();
      say(`Renamed on ${r.changed.length} scale(s).`);
    } catch (e) {
      fail(e);
    }
  }

  // The live check runs on every change, so a problem shows before the scale is saved.
  useEffect(() => {
    if (!scale) {
      setCheck(null);
      return;
    }
    const t = setTimeout(() => {
      call<Check>('scale.check', { scale }).then(setCheck).catch(() => {});
    }, 150);
    return () => clearTimeout(t);
  }, [scale]);

  // A variant opened from the library is offered its parent's changes since it was made (never applied unasked).
  const variantKey = scale?.variantOf ? `${scale.id}:${scale.variantOf.parentVersion}:${(scale.variantOf.declined ?? []).length}:${JSON.stringify(scale.variantOf.itemMap.map((e) => e.originalText))}` : '';
  useEffect(() => {
    if (!scale?.variantOf) {
      setParentChanges(null);
      return;
    }
    call<{ found: boolean; parentPath?: string; changes: ParentChange[] }>('variant.parentChanges', { scale })
      .then((r) => setParentChanges({ parentPath: r.parentPath ?? null, changes: r.changes }))
      .catch(() => setParentChanges(null));
    // Only when the variant or its link changes, not on every keystroke.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [variantKey]);

  function changeContext() {
    if (!scale || !path) return;
    if (dirty) {
      say('Save the scale first: a variant is made from a saved version.');
      return;
    }
    setVariantOf({ path, title: scale.title });
    setView('variant');
  }

  const confirmDiscard = () => !dirty || window.confirm('This scale has changes that are not saved. Leave them?');

  const load = (s: Scale, p: string | null, n: string[], isDirty: boolean) => {
    setEditingPart(null);
    setView('scale');
    setArticle(null);
    setShowCandidates(false);
    setScale(s);
    setPath(p);
    setNotes(n);
    setDirty(isDirty);
    setChoices(null);
  };

  const edit = (f: (s: Scale) => Scale) => {
    setScale((s) => (s ? f(s) : s));
    setDirty(true);
  };

  const applyOutcome = (r: { scale: Scale; outcome: ChangeOutcome }, what: string) => {
    setScale(r.scale);
    setDirty(true);
    if (r.outcome.instructionsRewritten) say(`${what}. The house instructions were rewritten to match.`);
    else if (r.outcome.findings.length) say(`${what}. The instructions were not rewritten; see the findings under them.`);
    else say(`${what}.`);
  };

  /// Links a scale to the Comm Directory entry it was brought in from (source, tags and title from the entry).
  async function linkEntry(s: Scale, entry?: Entry | null): Promise<Scale> {
    if (!entry) return s;
    try {
      return await call<Scale>('directory.link', { scale: s, id: entry.id });
    } catch (e) {
      fail(e);
      return s;
    }
  }

  async function newScale(format: Format, entry?: Entry) {
    if (!confirmDiscard()) return;
    try {
      load(await linkEntry(await call<Scale>('scale.new', { format }), entry), null, [], true);
      say(`New ${FORMAT_LABEL[format]} scale from the blank template.`);
    } catch (e) {
      fail(e);
    }
  }

  async function importFile(entry?: Entry) {
    if (!confirmDiscard()) return;
    const p = await pickFile('Import a stored scale', ['doc', 'docx', 'txt'], 'Stored scales');
    if (!p) return;
    try {
      const reports = await call<ImportReport[]>('scale.import', { path: p });
      if (entry) {
        for (const r of reports) r.scale = await linkEntry(r.scale, entry);
      }
      if (reports.length === 1) openReport(reports[0]);
      else setChoices(reports);
    } catch (e) {
      fail(e);
    }
  }

  async function findInArticle(entry?: Entry) {
    if (!confirmDiscard()) return;
    const p = await pickFile('Find a measure in a journal article', ['pdf', 'png', 'jpg', 'jpeg'], 'Articles and page pictures');
    if (!p) return;
    say('Reading the article. Scanned pages are read by text recognition, which takes a little while…');
    try {
      const r = await call<{ detection: Detection; pages: ArticlePage[] }>('article.detect', { path: p });
      setChoices(null);
      setArticle({ path: p, pages: r.pages, detection: r.detection, firstGuess: null, entry: entry ?? null });
      setView('scale');
      setShowCandidates(true);
      say(
        r.detection.candidates.length
          ? `Found ${r.detection.candidates.length} possible measure(s). Choose one to confirm.`
          : 'No measure was found. Open the article anyway and outline the items on the page to read them.',
      );
    } catch (e) {
      fail(e);
    }
  }

  async function openCandidate(i: number | null) {
    if (!article) return;
    const s = await linkEntry(i === null ? blankFromArticle(article) : article.detection.scales[i], article.entry);
    const c = i === null ? null : article.detection.candidates[i];
    setScale(s);
    setPath(null);
    setNotes([...article.detection.notes, ...(c ? [`Found on page ${c.page} because of: ${c.reasons.join('; ')}.`] : [])]);
    setDirty(true);
    setArticle({ ...article, firstGuess: s });
    setShowCandidates(false);
    say('Check every field against the page, then confirm and save.');
  }

  async function reparse(page: number, region: [number, number, number, number]) {
    if (!article) return;
    try {
      const s = await call<Scale>('article.parseRegion', { path: article.path, page, region });
      if (window.confirm(`Replace the fields with the ${s.items.length} item(s) read inside the outline?`)) {
        setScale(s);
        setDirty(true);
        say('Read again from the outline. Check every field.');
      }
    } catch (e) {
      fail(e);
    }
  }

  function selectField(field: string) {
    setSelectedField(field);
    const el = document.getElementById(fieldId(field));
    if (el) {
      el.scrollIntoView({ block: 'center' });
      el.focus();
    }
  }

  // ---------------------------------------------------------------- studies

  const changeStudy = (st: Study) => {
    setStudy(st);
    setStudyDirty(true);
  };

  async function newStudy() {
    if (study && studyDirty && !window.confirm('The open study has changes that are not saved. Leave them?')) return;
    try {
      setStudy(await call<Study>('study.new', { title: 'New study' }));
      setStudyPath(null);
      setStudyDirty(true);
      setView('study');
      say('A new study. Add scales from the library with the Add buttons beside them.');
    } catch (e) {
      fail(e);
    }
  }

  async function openStudy() {
    if (study && studyDirty && !window.confirm('The open study has changes that are not saved. Leave them?')) return;
    const p = await pickFile('Open a study', ['pstudy'], 'Prism Survey studies');
    if (!p) return;
    try {
      setStudy(await call<Study>('study.open', { path: p }));
      setStudyPath(p);
      setStudyDirty(false);
      setView('study');
      say(`Opened ${p}.`);
    } catch (e) {
      fail(e);
    }
  }

  async function saveStudy(as: boolean) {
    if (!study || !settings) return;
    let target = studyPath;
    if (!target || as) {
      target = await pickSave('Save the study', joinPath(settings.libraryFolder, safeName(study.title) + '.pstudy'), ['pstudy'], 'Prism Survey study');
      if (!target) return;
    }
    try {
      const saved = await call<Study>('study.save', { path: target, study, summary: studyPath ? 'Edited' : 'Created' });
      setStudy(saved);
      setStudyPath(target);
      setStudyDirty(false);
      say(`Saved the study to ${target}.`);
    } catch (e) {
      fail(e);
    }
  }

  async function exportStudy(copy: 'respondent' | 'researcher') {
    if (!study) return;
    const target = await pickSave('Export the study to Word', `${safeName(study.title)} - ${copy} copy.docx`, ['docx'], 'Word document');
    if (!target) return;
    try {
      const r = await call<{ notes: string[] }>('study.export', { study, path: target, copy });
      say([`Exported the study's ${copy} copy to ${target}.`, ...r.notes].join(' '));
    } catch (e) {
      fail(e);
    }
  }

  async function addLibraryScale(path: string) {
    if (!study) return;
    try {
      changeStudy(await call<Study>('study.addScale', { study, path }));
      say('Added to the study.');
    } catch (e) {
      fail(e);
    }
  }

  /// A stored scale, or a whole stored questionnaire: every scale found in the file is added, in order.
  async function importForStudy(): Promise<Scale[]> {
    const p = await pickFile('Import a stored scale or questionnaire', ['doc', 'docx', 'txt'], 'Stored scales');
    if (!p) return [];
    try {
      const reports = await call<ImportReport[]>('scale.import', { path: p });
      if (reports.length > 1) say(`The file held ${reports.length} scales; all were added. Check each one with Edit.`);
      return reports.map((r) => r.scale);
    } catch (e) {
      fail(e);
      return [];
    }
  }

  function editStudyScale(sc: Scale, index: number) {
    if (!confirmDiscard()) return;
    load(sc, null, [], false);
    setEditingPart(index);
    setView('scale');
    say("Editing the study's copy of this scale. Use Back to the study to put the changes back.");
  }

  function backToStudy() {
    if (study && scale && editingPart !== null) {
      const parts = study.parts.map((p, k) => (k === editingPart && p.kind === 'scale' ? { ...p, scale } : p));
      changeStudy({ ...study, parts });
      setDirty(false);
    }
    setEditingPart(null);
    setView('study');
  }

  async function copyForWord() {
    if (!scale) return;
    try {
      const r = await call<{ html: string; text: string }>('scale.clipboard', { scale, copy: 'respondent' });
      if ('ClipboardItem' in window) {
        await navigator.clipboard.write([
          new ClipboardItem({ 'text/html': new Blob([r.html], { type: 'text/html' }), 'text/plain': new Blob([r.text], { type: 'text/plain' }) }),
        ]);
      } else {
        await navigator.clipboard.writeText(r.text);
      }
      say('Copied. Paste it into an open Word document.');
    } catch (e) {
      fail(e);
    }
  }

  function openReport(r: ImportReport) {
    load(r.scale, null, r.notes, true);
    say(`Imported from ${r.file}. Check every field, then save it to the library.`);
  }

  async function openFile(p?: string) {
    if (!confirmDiscard()) return;
    const file = p ?? (await pickFile('Open a scale', ['pscale'], 'Prism Survey scales'));
    if (!file) return;
    try {
      load(await call<Scale>('scale.open', { path: file }), file, [], false);
      say(`Opened ${file}.`);
    } catch (e) {
      fail(e);
    }
  }

  async function saveScale(as: boolean) {
    if (!scale || !settings) return;
    let target = path;
    if (!target || as) {
      target = await pickSave('Save the scale', joinPath(settings.libraryFolder, safeName(scale.title) + '.pscale'), ['pscale'], 'Prism Survey scale');
      if (!target) return;
    }
    try {
      const summary = path ? 'Edited' : notes.length ? 'Imported and confirmed' : 'Created';
      const saved = await call<Scale>('scale.save', { path: target, scale, summary: article ? 'Found in an article and confirmed' : summary });
      if (article) {
        // The correction is kept as a labelled example; in benchmark mode it is also the article's answer key.
        const r = await call<{ answerKey: string | null }>('article.confirm', { path: article.path, firstGuess: article.firstGuess, confirmed: saved, benchmark });
        setScale(saved);
        setPath(target);
        setDirty(false);
        say(`Saved to ${target}.${r.answerKey ? ' The answer key for the benchmark was saved too.' : ''}`);
      } else {
        load(saved, target, notes, false);
        say(`Saved to ${target}.`);
      }
      refreshLibrary();
    } catch (e) {
      fail(e);
    }
  }

  async function exportWord(copy: 'respondent' | 'researcher') {
    if (!scale || !settings) return;
    const name = `${safeName(scale.title)} - ${copy} copy.docx`;
    const target = await pickSave('Export to Word', name, ['docx'], 'Word document');
    if (!target) return;
    try {
      const r = await call<{ notes: string[] }>('scale.export', { scale, path: target, copy });
      say([`Exported the ${copy} copy to ${target}.`, ...r.notes].join(' '));
    } catch (e) {
      fail(e);
    }
  }

  async function changeLibraryFolder() {
    const f = await pickFolder('Choose the library folder');
    if (!f) return;
    try {
      const s = await call<Settings>('settings.set', { settings: { libraryFolder: f } });
      setSettings(s);
      refreshLibrary(f);
    } catch (e) {
      fail(e);
    }
  }

  // Ctrl+S saves; F8 moves to the next field the importer was unsure of.
  useEffect(() => {
    const onKey = (ev: KeyboardEvent) => {
      if ((ev.ctrlKey || ev.metaKey) && ev.key.toLowerCase() === 's') {
        ev.preventDefault();
        saveScale(ev.shiftKey);
      } else if (ev.key === 'F8') {
        ev.preventDefault();
        const unresolved = Array.from(formRef.current?.querySelectorAll<HTMLElement>('[data-unsure="true"]') ?? []);
        if (!unresolved.length) return;
        const i = unresolved.findIndex((el) => el.contains(document.activeElement));
        const next = unresolved[(i + 1) % unresolved.length];
        (next.querySelector('input, textarea, select') as HTMLElement | null)?.focus();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  });

  const unsure = useMemo(() => {
    const set = new Set<string>();
    for (const p of scale?.provenance ?? []) if (p.confidence < 0.7) set.add(p.field);
    return set;
  }, [scale]);

  return (
    <div className="app">
      <div className="titlebar">
        <span className="brand">
          <img src={icon} alt="" width={24} height={24} />
          Prism Survey
        </span>
      </div>
      <header className="toolbar" role="toolbar" aria-label="Scale commands">
        <div className="group" role="group" aria-label="New scale">
          <span className="group-label">New</span>
          <button onClick={() => newScale('likertBlankLine')}>
            <Icon name="fmt-blank-line" />
            Likert blank-line
          </button>
          <button onClick={() => newScale('likertBoxGrid')}>
            <Icon name="fmt-box-grid" />
            Likert box grid
          </button>
          <button onClick={() => newScale('semanticDifferential')}>
            <Icon name="fmt-semantic" />
            Semantic differential
          </button>
        </div>
        <div className="group" role="group" aria-label="Files">
          <button onClick={() => findInArticle()}>
            <Icon name="cmd-find-article" />
            Find in an article…
          </button>
          <button onClick={() => importFile()}>
            <Icon name="cmd-import" />
            Import stored scale…
          </button>
          <button onClick={() => setView('directory')} aria-pressed={view === 'directory'}>
            <Icon name="view-directory" />
            Comm Directory
          </button>
          <button onClick={() => openFile()}>
            <Icon name="cmd-open" />
            Open…
          </button>
          <button className="primary" onClick={() => saveScale(false)} disabled={!scale}>
            <Icon name="ui-save" />
            {path ? 'Save' : 'Confirm and save…'}
          </button>
          <button onClick={() => saveScale(true)} disabled={!scale}>
            <Icon name="cmd-save-as" />
            Save as…
          </button>
          <button onClick={changeContext} disabled={!scale || !path} title="Make a linked variant of this scale for a new target, with its structure locked">
            <Icon name="cmd-variant" />
            Change the context…
          </button>
        </div>
        <div className="group" role="group" aria-label="Word export">
          <button onClick={() => exportWord('respondent')} disabled={!scale}>
            <Icon name="exp-respondent" />
            Export respondent copy…
          </button>
          <button onClick={() => exportWord('researcher')} disabled={!scale}>
            <Icon name="exp-researcher" />
            Export researcher copy…
          </button>
          <button onClick={copyForWord} disabled={!scale}>
            <Icon name="ui-copy" />
            Copy for Word
          </button>
        </div>
        <div className="group" role="group" aria-label="Study">
          <span className="group-label">Study</span>
          <button onClick={newStudy}>
            <Icon name="study-new" />
            New
          </button>
          <button onClick={openStudy}>
            <Icon name="cmd-open" />
            Open…
          </button>
          {study && view === 'scale' && (
            <button onClick={backToStudy}>
              <Icon name="view-study" />
              {editingPart !== null ? 'Back to the study' : 'Show the study'}
            </button>
          )}
          {study && view === 'study' && (
            <>
              <button className="primary" onClick={() => saveStudy(false)}>
                <Icon name="ui-save" />
                {studyPath ? 'Save' : 'Save…'}
              </button>
              <button onClick={() => exportStudy('respondent')}>
                <Icon name="exp-respondent" />
                Export respondent copy…
              </button>
              <button onClick={() => exportStudy('researcher')}>
                <Icon name="exp-researcher" />
                Export researcher copy…
              </button>
            </>
          )}
        </div>
        <label className="switch toolbar-check" title="Confirmed articles are saved as answer keys for the accuracy benchmark">
          <input type="checkbox" checked={benchmark} onChange={(e) => setBenchmark(e.target.checked)} />
          <Icon name="cmd-benchmark" />
          Benchmark mode
        </label>
      </header>

      <div className="body">
        <nav className="library" aria-label="Scale library">
          <h2>Library</h2>
          <p className="folder" title={settings?.libraryFolder}>
            {settings?.libraryFolder}
          </p>
          <button className="small" onClick={changeLibraryFolder}>
            <Icon name="src-folder" size={16} />
            Change folder…
          </button>
          <label className="field search-field">
            <span className="field-label">
              <Icon name="cmd-search" size={14} />
              Search
            </span>
            <input
              type="search"
              value={query}
              placeholder="Words, &quot;a phrase&quot;, tag:health"
              title='Words match titles, tags, subscales, items, sources and instructions. Put a phrase in quotation marks. Name a field: title:, tag:, item:, subscale:, author:, year:.'
              onChange={(e) => setQuery(e.target.value)}
            />
          </label>
          <div className="filters">
            <label className="field">
              <span className="field-label">
                <Icon name="ui-filter" size={14} />
                Tag
              </span>
              <select value={tagFilter} onChange={(e) => setTagFilter(e.target.value)}>
                <option value="">Any</option>
                {libraryTags.map((t) => (
                  <option key={t.tag} value={t.tag}>
                    {t.tag} ({t.count})
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              <span className="field-label">Format</span>
              <select value={formatFilter} onChange={(e) => setFormatFilter(e.target.value)}>
                <option value="">Any</option>
                {(Object.keys(FORMAT_LABEL) as Format[]).map((f) => (
                  <option key={f} value={f}>
                    {FORMAT_LABEL[f]}
                  </option>
                ))}
              </select>
            </label>
            <label className="field">
              <span className="field-label">Points</span>
              <select value={pointsFilter} onChange={(e) => setPointsFilter(e.target.value)}>
                <option value="">Any</option>
                {[2, 3, 4, 5, 6, 7, 8, 9, 10, 11].map((n) => (
                  <option key={n} value={n}>
                    {n}
                  </option>
                ))}
              </select>
            </label>
          </div>
          {tagFilter && (
            <button className="small" onClick={renameTag}>
              <Icon name="tag-rename" size={16} />
              Rename or merge this tag…
            </button>
          )}
          {library.length === 0 ? (
            <p className="muted">No scales saved yet.</p>
          ) : hits && hits.length === 0 ? (
            <p className="muted">No scale matches.</p>
          ) : (
            <ul>
              {(hits ? hits.map((h) => ({ h, e: library.find((e) => e.path === h.key) })).filter((x) => x.e) : library.map((e) => ({ h: null, e })))
                .map(({ h, e: entry }) => ({ h, ...(entry as LibraryEntry) }))
                .map(({ h, ...e }) => (
                <li key={e.path}>
                  <button className={'lib-entry' + (e.path === path ? ' current' : '')} onClick={() => openFile(e.path)} disabled={!!e.error}>
                    <span className="lib-title">
                      {!e.error && <Icon name={FORMAT_ICON[e.format]} size={16} />}
                      {e.title || '(untitled)'}
                    </span>
                    <span className="lib-meta">{e.error ? 'Cannot be opened' : `${FORMAT_LABEL[e.format]}, ${e.items} items`}</span>
                    {e.variantChain && e.variantChain.length > 0 && (
                      <span className="lib-chain">
                        <Icon name="lib-variant" size={14} /> Variant of {e.variantChain.join(', itself a variant of ')}
                      </span>
                    )}
                    {e.tags.length > 0 && <span className="lib-tags">{e.tags.join(' · ')}</span>}
                    {h && h.snippet && query.trim() && h.field !== 'title' && <span className="lib-snippet">{h.snippet}</span>}
                  </button>
                  {study && !e.error && (
                    <button className="small lib-add" onClick={() => addLibraryScale(e.path)} aria-label={`Add ${e.title} to the study`}>
                      <Icon name="study-add" size={16} />
                      Add to study
                    </button>
                  )}
                </li>
              ))}
            </ul>
          )}
        </nav>

        <main className={'main' + (article && scale && !showCandidates ? ' with-article' : '')} ref={formRef}>
          {view === 'variant' && variantOf ? (
            <VariantView
              parentPath={variantOf.path}
              parentTitle={variantOf.title}
              onDone={(s, n) => {
                load(s, null, n, true);
                setVariantOf(null);
                say('The variant is drafted. The variant structure check runs below; save it to the library when it is right.');
              }}
              onCancel={() => {
                setVariantOf(null);
                setView('scale');
              }}
              fail={fail}
            />
          ) : view === 'directory' ? (
            <DirectoryView
              initialEntry={directoryEntry}
              onFindInArticle={(e) => findInArticle(e)}
              onImportStored={(e) => importFile(e)}
              onStartBlank={(e) => newScale('likertBlankLine', e)}
              fail={fail}
            />
          ) : study && view === 'study' ? (
            <StudyBuilder
              study={study}
              setStudy={changeStudy}
              openScale={editStudyScale}
              currentScale={scale}
              importScale={importForStudy}
              addLibraryScale={addLibraryScale}
              showDirectoryEntry={(id) => {
                setDirectoryEntry(id);
                setView('directory');
              }}
              fail={fail}
            />
          ) : article && showCandidates ? (
            <CandidateChooser article={article} onPick={openCandidate} onCancel={() => setShowCandidates(false)} />
          ) : choices ? (
            <ScaleChooser choices={choices} onPick={openReport} onCancel={() => setChoices(null)} />
          ) : scale ? (
            <>
            {article && (
              <ArticlePane path={article.path} pages={article.pages} scale={scale} selected={selectedField} onSelect={selectField} onRegion={reparse} fail={fail} />
            )}
            <ConfirmForm
              onFocusField={setSelectedField}
              parentChanges={parentChanges}
              scale={scale}
              knownTags={libraryTags.map((t) => t.tag)}
              notes={notes}
              check={check}
              unsure={unsure}
              edit={edit}
              applyOutcome={applyOutcome}
              fail={fail}
              say={say}
            />
            </>
          ) : (
            <Welcome />
          )}
        </main>
      </div>

      <footer className={'status' + (status?.kind === 'error' ? ' error' : '')} role="status" aria-live="polite">
        {status?.kind === 'error' ? (
          <Icon name="st-error" size={16} />
        ) : (
          !status && (view === 'study' && study ? !studyDirty : !!scale && !dirty) && <Icon name="st-saved" size={16} />
        )}
        {status?.text ??
          (view === 'study' && study ? (studyDirty ? 'The study is not saved.' : 'The study is saved.') : scale ? (dirty ? 'Not saved.' : 'Saved.') : 'Ready.')}
      </footer>
    </div>
  );
}

interface SearchHit {
  key: string;
  title: string;
  collection: string;
  score: number;
  snippet: string;
  field: string;
}

interface Detection {
  file: string;
  notes: string[];
  candidates: { score: number; page: number; reasons: string[]; sampleOnly: boolean; items: unknown[] }[];
  scales: Scale[];
}

interface OpenArticle {
  path: string;
  pages: ArticlePage[];
  detection: Detection;
  /// What the tool offered first, kept for the correction example and the answer key.
  firstGuess: Scale | null;
  /// The Comm Directory entry the article was opened from.
  entry: Entry | null;
}

/// A blank scale to fill by outlining the items on the page, when nothing was found.
function blankFromArticle(article: OpenArticle): Scale {
  const base = article.detection.scales[0];
  if (base) return { ...base, items: [], provenance: [] };
  return {
    pscaleVersion: 1,
    id: crypto.randomUUID(),
    title: article.detection.file.replace(/\.[^.]+$/, ''),
    countSuffix: false,
    tags: [],
    format: 'likertBlankLine',
    response: { points: 5, anchorSet: 'agreement', labelMode: 'every', labels: ['Strongly Disagree', 'Disagree', 'Neutral', 'Agree', 'Strongly Agree'], abbreviations: ['SD', 'D', 'N', 'A', 'SA'], values: [1, 2, 3, 4, 5] },
    instructions: { text: '', origin: 'verbatim' },
    items: [],
    recode: { rule: { kind: 'reflect' } },
    subscales: [],
    scoring: { subscaleMethod: 'sum', totalMethod: 'sum', totalFrom: 'items', hasTotal: true },
    scoreRange: {},
    sourcePlacement: 'underScale',
    licence: { status: 'localOnly' },
    provenance: [],
    history: [],
  };
}

function CandidateChooser({ article, onPick, onCancel }: { article: OpenArticle; onPick: (i: number | null) => void; onCancel: () => void }) {
  const d = article.detection;
  return (
    <section className="welcome">
      <h1>Measures found in {d.file}</h1>
      {d.notes.map((n, i) => (
        <p key={i} className="muted">
          {n}
        </p>
      ))}
      {d.candidates.length === 0 && <p>No measure was found by the rules. Open the article and outline the items on the page.</p>}
      <ol className="choices">
        {d.candidates.map((c, i) => (
          <li key={i}>
            <button onClick={() => onPick(i)}>
              <Icon name={c.sampleOnly ? 'art-incomplete' : 'art-candidate'} />
              <strong>{d.scales[i].title}</strong>, page {c.page}, {d.scales[i].items.length} items{c.sampleOnly ? ' (sample items only)' : ''}
            </button>
            <span className="reason">Why: {c.reasons.join('; ')}.</span>
          </li>
        ))}
      </ol>
      <div className="buttons">
        <button onClick={() => onPick(null)}>
          <Icon name="art-outline" />
          Open the article and outline the items myself
        </button>
        <button onClick={onCancel}>Cancel</button>
      </div>
    </section>
  );
}

function Welcome() {
  return (
    <section className="welcome">
      <div className="hero">
        <img src={icon} alt="" width={64} height={64} />
        <h1>Prepare a scale</h1>
      </div>
      <p>Start a new scale from one of the three blank formats, or import a stored scale (.doc, .docx or .txt).</p>
      <p>
        Every field opens on one form to confirm and correct before anything is saved. Nothing leaves this computer: scales are kept as
        .pscale files in your library folder.
      </p>
      <p className="muted">Keys: Ctrl+S saves, Ctrl+Shift+S saves as, F8 moves to the next field the importer was unsure of.</p>
    </section>
  );
}

function ScaleChooser({ choices, onPick, onCancel }: { choices: ImportReport[]; onPick: (r: ImportReport) => void; onCancel: () => void }) {
  return (
    <section>
      <h1>This file holds {choices.length} scales</h1>
      <p>Choose the one to confirm first. Import the file again for the others.</p>
      <ul className="choices">
        {choices.map((r) => (
          <li key={r.part}>
            <button onClick={() => onPick(r)}>
              <Icon name={FORMAT_ICON[r.scale.format]} />
              Scale {r.part}: {r.scale.title || '(no title)'}, {FORMAT_LABEL[r.scale.format]}, {r.scale.items.length} items
            </button>
          </li>
        ))}
      </ul>
      <button onClick={onCancel}>Cancel</button>
    </section>
  );
}

/// The scale's own tags: free text, no fixed list. Enter or a comma adds a tag; tags already used in the library are
/// offered as the person types.
function TagEditor({ tags, known, onChange }: { tags: string[]; known: string[]; onChange: (tags: string[]) => void }) {
  const [draft, setDraft] = useState('');
  const key = (t: string) => t.trim().replace(/\s+/g, ' ').toLowerCase();
  const add = (raw: string) => {
    const parts = raw.split(',').map((p) => p.trim().replace(/\s+/g, ' ')).filter(Boolean);
    const next = [...tags];
    for (const p of parts) if (!next.some((t) => key(t) === key(p))) next.push(p);
    if (next.length !== tags.length) onChange(next);
    setDraft('');
  };
  const offered = known.filter((k) => !tags.some((t) => key(t) === key(k)));
  return (
    <div className="field">
      <span className="field-label" id="tags-label">
        Tags
      </span>
      <div className="tags" role="group" aria-labelledby="tags-label">
        {tags.map((t) => (
          <span className="tag" key={t}>
            {t}
            <button className="tag-remove" aria-label={`Remove the tag ${t}`} onClick={() => onChange(tags.filter((x) => x !== t))}>
              <Icon name="ui-close" size={14} />
            </button>
          </span>
        ))}
        <input
          aria-label="Add a tag"
          list="known-tags"
          placeholder="Add a tag, then Enter"
          value={draft}
          onChange={(e) => (e.target.value.endsWith(',') ? add(e.target.value) : setDraft(e.target.value))}
          onKeyDown={(e) => {
            if (e.key === 'Enter') {
              e.preventDefault();
              add(draft);
            } else if (e.key === 'Backspace' && !draft && tags.length) {
              onChange(tags.slice(0, -1));
            }
          }}
          onBlur={() => draft.trim() && add(draft)}
        />
        <datalist id="known-tags">
          {offered.map((k) => (
            <option key={k} value={k} />
          ))}
        </datalist>
      </div>
      <span className="hint">Your own words: a topic, a course, a study, anything that helps you find the scale again.</span>
    </div>
  );
}

/// The scoring self-check: every finding warns and never blocks. Each can be fixed in one step where a fix exists,
/// kept as it is, dismissed with a reason the scale keeps, or marked as published with a note on what the original
/// says (kept as printed, or corrected with the correction recorded). Open findings and source notes print in the
/// researcher copy.
function SelfCheck({ scale, findings, notes, edit, fail }: { scale: Scale; findings: Finding[]; notes: NoteLine[]; edit: (f: (s: Scale) => Scale) => void; fail: (e: unknown) => void }) {
  const [dismissing, setDismissing] = useState<string | null>(null);
  const [reason, setReason] = useState('');
  const [noting, setNoting] = useState<string | null>(null);
  const [original, setOriginal] = useState('');
  const [page, setPage] = useState('');
  const open = findings.filter((f) => f.status === 'open');
  const decided = findings.filter((f) => f.status !== 'open' && f.status !== 'published');
  const published = findings.filter((f) => f.status === 'published');
  const corrected = notes.filter((n) => n.handling === 'corrected');

  function startNote(key: string) {
    setNoting(noting === key ? null : key);
    setDismissing(null);
    setOriginal('');
    setPage('');
  }
  async function keepPublished(key: string) {
    try {
      const s = await call<Scale>('scale.keepAsPublished', { scale, key, original, page: page || null });
      edit(() => s);
      setNoting(null);
    } catch (e) {
      fail(e);
    }
  }
  async function correct(key: string, f: Fix) {
    try {
      const s = await call<Scale>('scale.correctFromSource', { scale, key, fix: f, original, page: page || null });
      edit(() => s);
      setNoting(null);
    } catch (e) {
      fail(e);
    }
  }
  async function removeNote(key: string) {
    try {
      const s = await call<Scale>('scale.removeSourceNote', { scale, key });
      edit(() => s);
    } catch (e) {
      fail(e);
    }
  }

  async function decide(key: string, status: 'accepted' | 'dismissed' | null, why = '') {
    try {
      const s = await call<Scale>('scale.decideFinding', { scale, key, status, reason: why });
      edit(() => s);
      setDismissing(null);
      setReason('');
    } catch (e) {
      fail(e);
    }
  }
  async function fix(f: Fix) {
    try {
      const s = await call<Scale>('scale.applyFix', { scale, fix: f });
      edit(() => s);
    } catch (e) {
      fail(e);
    }
  }

  return (
    <section className="panel" aria-labelledby="check-h">
      <h2 id="check-h">
        <Icon name="chk-selfcheck" />
        Self-check{' '}
        <span className="muted">
          ({open.length} open{decided.length ? `, ${decided.length} decided` : ''}
          {notes.length ? `, ${notes.length} source ${notes.length === 1 ? 'note' : 'notes'}` : ''})
        </span>
      </h2>
      {open.length === 0 && <p className="muted">No open findings. The self-check runs again with every change.</p>}
      <ul className="findings">
        {open.map((f) => (
          <li key={f.key} className={'finding-row ' + f.severity}>
            <span className={'badge ' + (f.severity === 'warning' ? 'warn' : 'note-badge')}>
              <Icon name={f.severity === 'warning' ? 'st-warning' : 'st-info'} size={14} />
              {f.severity === 'warning' ? 'Check' : 'Note'}
            </span>
            <strong>{f.checkLabel}.</strong> {f.message}
            <div className="buttons">
              {(f.fixes ?? []).map((x, i) => (
                <button key={i} className="small" onClick={() => fix(x)}>
                  <Icon name="chk-fix" size={16} />
                  {f.fixLabels?.[i] ?? 'Fix'}
                </button>
              ))}
              <button className="small" onClick={() => decide(f.key, 'accepted')}>
                <Icon name="ui-check" size={16} />
                Keep as it is
              </button>
              <button className="small" onClick={() => { setDismissing(dismissing === f.key ? null : f.key); setNoting(null); }} aria-expanded={dismissing === f.key}>
                <Icon name="chk-dismiss" size={16} />
                Does not apply…
              </button>
              <button className="small" onClick={() => startNote(f.key)} aria-expanded={noting === f.key}>
                <Icon name="chk-published" size={16} />
                As published…
              </button>
            </div>
            {noting === f.key && (
              <div className="note-form">
                <div className="row">
                  <label className="field grow">
                    <span className="field-label">What the original publication says</span>
                    <input autoFocus value={original} placeholder="the original lists item 4 as reverse scored" onChange={(e) => setOriginal(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && original.trim() && keepPublished(f.key)} />
                  </label>
                  <label className="field narrow">
                    <span className="field-label">Page</span>
                    <input value={page} onChange={(e) => setPage(e.target.value)} />
                  </label>
                </div>
                <div className="buttons">
                  <button className="small" disabled={!original.trim()} onClick={() => keepPublished(f.key)}>
                    <Icon name="chk-published" size={16} />
                    Keep as printed
                  </button>
                  {(f.fixes ?? []).map((x, i) => (
                    <button key={i} className="small" disabled={!original.trim()} onClick={() => correct(f.key, x)}>
                      <Icon name="chk-corrected" size={16} />
                      Correct it: {f.fixLabels?.[i] ?? 'fix'}
                    </button>
                  ))}
                </div>
                <p className="muted">The note shows beside the item here and under the scale in the researcher copy. The respondent copy is unchanged.</p>
              </div>
            )}
            {dismissing === f.key && (
              <div className="row">
                <label className="field">
                  <span className="field-label">Why it does not apply (kept with the scale)</span>
                  <input autoFocus value={reason} onChange={(e) => setReason(e.target.value)} onKeyDown={(e) => e.key === 'Enter' && reason.trim() && decide(f.key, 'dismissed', reason)} />
                </label>
                <button className="small align-end" disabled={!reason.trim()} onClick={() => decide(f.key, 'dismissed', reason)}>
                  Dismiss
                </button>
              </div>
            )}
          </li>
        ))}
      </ul>
      {(published.length > 0 || corrected.length > 0) && (
        <>
          <h3>Source notes</h3>
          <ul className="findings">
            {published.map((f) => (
              <li key={f.key} className="finding-row published">
                <span className="badge note-badge">
                  <Icon name="chk-published" size={14} />
                  As published
                </span>
                <strong>{f.checkLabel}.</strong> {f.message}
                {f.reason && <span className="reason">{f.reason}</span>}
                <button className="small" onClick={() => removeNote(f.key)}>
                  <Icon name="chk-reopen" size={16} />
                  Reopen
                </button>
              </li>
            ))}
            {corrected.map((n) => (
              <li key={n.key} className="finding-row published">
                <span className="badge note-badge">
                  <Icon name="chk-corrected" size={14} />
                  Corrected
                </span>
                <strong>{n.subject}.</strong> <span className="reason">{n.text}</span>
                <button className="small" onClick={() => removeNote(n.key)} title="Removes the note only; the correction stays">
                  Remove note
                </button>
              </li>
            ))}
          </ul>
        </>
      )}
      {decided.length > 0 && (
        <details>
          <summary>Decided findings ({decided.length})</summary>
          <ul className="findings">
            {decided.map((f) => (
              <li key={f.key} className="finding-row decided">
                <strong>{f.status === 'accepted' ? 'Kept as it is' : 'Dismissed'}:</strong> {f.checkLabel}. {f.message}
                {f.reason && <span className="reason">Reason: {f.reason}</span>}
                <button className="small" onClick={() => decide(f.key, null)}>
                  <Icon name="chk-reopen" size={16} />
                  Reopen
                </button>
              </li>
            ))}
          </ul>
        </details>
      )}
    </section>
  );
}

/// Links the scale's citations to Prism Reference Manager and formats them there in APA 7. The first time, Prism Survey
/// asks to pair and the person approves it in Prism Reference Manager's window.
function RefManLink({ scale, edit, say, fail }: { scale: Scale; edit: (f: (s: Scale) => Scale) => void; say: (t: string) => void; fail: (e: unknown) => void }) {
  const [busy, setBusy] = useState(false);
  async function link() {
    setBusy(true);
    try {
      const st = await call<{ reachable: boolean; paired: boolean }>('refman.status');
      if (!st.reachable) {
        say('Prism Reference Manager is not running, or its local connections are off (its Settings, Local connections). The citation stays as plain APA text.');
        return;
      }
      if (!st.paired) {
        const { requestId } = await call<{ requestId: string }>('refman.pair');
        say('Approve Prism Survey in Prism Reference Manager’s window…');
        let state = 'pending';
        for (let k = 0; k < 60 && state === 'pending'; k++) {
          await new Promise((r) => setTimeout(r, 2000));
          state = (await call<{ state: string }>('refman.pairStatus', { id: requestId })).state;
        }
        if (state !== 'approved') {
          say(state === 'denied' ? 'Prism Reference Manager did not allow the connection.' : 'No answer from Prism Reference Manager yet; try again.');
          return;
        }
      }
      const r = await call<{ scale: Scale; notes: string[] }>('refman.cite', { scale });
      edit(() => r.scale);
      say(r.notes.join(' '));
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  }
  if (!scale.source && !scale.adaptedFrom) return null;
  return (
    <button className="small" onClick={link} disabled={busy}>
      {busy ? 'Working…' : 'Link and format with Prism Reference Manager'}
    </button>
  );
}

interface FormProps {
  onFocusField?: (field: string) => void;
  parentChanges?: { parentPath: string | null; changes: ParentChange[] } | null;
  scale: Scale;
  knownTags: string[];
  notes: string[];
  check: Check | null;
  unsure: Set<string>;
  edit: (f: (s: Scale) => Scale) => void;
  applyOutcome: (r: { scale: Scale; outcome: ChangeOutcome }, what: string) => void;
  fail: (e: unknown) => void;
  say: (t: string) => void;
}

function Field({ label, children, unsure, hint }: { label: string; children: React.ReactNode; unsure?: boolean; hint?: string }) {
  return (
    <label className={'field' + (unsure ? ' unsure' : '')} data-unsure={unsure ? 'true' : undefined}>
      <span className="field-label">
        {label}
        {unsure && (
          <span className="badge warn">
            <Icon name="st-warning" size={14} />
            Check
          </span>
        )}
      </span>
      {children}
      {hint && <span className="hint">{hint}</span>}
    </label>
  );
}

function ConfirmForm({ onFocusField, parentChanges, scale, knownTags, notes, check, unsure, edit, applyOutcome, fail, say }: FormProps) {
  // A variant keeps its parent's structure locked (specification, "Change the context"); only wording is edited here.
  const locked = !!scale.variantOf;
  const sd = scale.format === 'semanticDifferential';
  const lowConfidence = scale.provenance.filter((p) => p.confidence < 0.7);
  const [selectorDrafts, setSelectorDrafts] = useState<Record<number, string>>({});

  async function setFormat(format: Format) {
    try {
      applyOutcome(await call('scale.setFormat', { scale, format }), `Format is now ${FORMAT_LABEL[format]}`);
    } catch (e) {
      fail(e);
    }
  }
  async function setPoints(points: number) {
    if (!Number.isFinite(points)) return;
    try {
      applyOutcome(await call('scale.setPoints', { scale, points }), `The scale now has ${points} points`);
    } catch (e) {
      fail(e);
    }
  }
  async function setAnchors(anchorSet: AnchorSet, labelMode: LabelMode) {
    try {
      applyOutcome(await call('scale.setAnchors', { scale, anchorSet, labelMode }), 'Anchors changed');
    } catch (e) {
      fail(e);
    }
  }
  async function useHouse() {
    try {
      const s = await call<Scale>('scale.useHouseInstructions', { scale });
      edit(() => s);
      say('House instructions for this format are in place.');
    } catch (e) {
      fail(e);
    }
  }
  async function setSelector(i: number, text: string) {
    try {
      const sel = await call<ItemSelector>('scale.parseSelector', { text });
      edit((s) => ({ ...s, subscales: s.subscales.map((x, k) => (k === i ? { ...x, items: sel } : x)) }));
      setSelectorDrafts((d) => {
        const { [i]: _, ...rest } = d;
        return rest;
      });
    } catch (e) {
      fail(e);
    }
  }

  const setItem = (i: number, patch: Partial<Item>) => edit((s) => ({ ...s, items: s.items.map((it, k) => (k === i ? { ...it, ...patch } : it)) }));
  const moveItem = (i: number, d: number) =>
    edit((s) => {
      const items = [...s.items];
      const j = i + d;
      if (j < 0 || j >= items.length) return s;
      [items[i], items[j]] = [items[j], items[i]];
      return { ...s, items: renumber(items) };
    });
  const deleteItem = (i: number) => edit((s) => ({ ...s, items: renumber(s.items.filter((_, k) => k !== i)) }));
  const mergeWithNext = (i: number) =>
    edit((s) => {
      if (i + 1 >= s.items.length) return s;
      const a = s.items[i];
      const b = s.items[i + 1];
      const merged: Item = { ...a, text: `${a.text} ${b.text}`.trim() };
      return { ...s, items: renumber([...s.items.slice(0, i), merged, ...s.items.slice(i + 2)]) };
    });
  const splitAt = (i: number, input: HTMLInputElement | null) =>
    edit((s) => {
      const it = s.items[i];
      const at = input?.selectionStart ?? Math.floor(it.text.length / 2);
      const first: Item = { ...it, text: it.text.slice(0, at).trim() };
      const second: Item = { number: 0, text: it.text.slice(at).trim(), reverse: { state: 'notReversed' } };
      return { ...s, items: renumber([...s.items.slice(0, i), first, second, ...s.items.slice(i + 1)]) };
    });
  const addItem = () =>
    edit((s) => ({
      ...s,
      items: renumber([...s.items, { number: 0, text: '', reverse: { state: 'notReversed' }, ...(sd ? { left: '', right: '' } : {}) }]),
    }));
  const trimEmpty = () =>
    edit((s) => {
      const items = [...s.items];
      while (items.length && !items[items.length - 1].text.trim() && !(items[items.length - 1].left ?? '').trim()) items.pop();
      return { ...s, items: renumber(items) };
    });

  const inputs = useRef<(HTMLInputElement | null)[]>([]);
  const statedRange = scale.scoreRange.stated;

  return (
    <div className="form">
      <h1 className="sr-only">Confirm and correct</h1>

      {(notes.length > 0 || lowConfidence.length > 0) && (
        <section className="panel notes" aria-labelledby="notes-h">
          <h2 id="notes-h">Check these first</h2>
          <ul>
            {lowConfidence.map((p) => (
              <li key={'p' + p.field}>
                <strong>{p.field}</strong>: found by {p.rule ?? 'a guess'} ({Math.round(p.confidence * 100)}% sure)
              </li>
            ))}
            {notes.map((n, i) => (
              <li key={i}>{n}</li>
            ))}
          </ul>
        </section>
      )}

      {scale.variantOf && (
        <VariantPanel scale={scale} changes={parentChanges?.changes ?? null} parentPath={parentChanges?.parentPath ?? null} edit={edit} fail={fail} />
      )}
      {check && <SelfCheck scale={scale} findings={check.findings} notes={check.sourceNotes ?? []} edit={edit} fail={fail} />}

      <section className="panel" aria-labelledby="basics-h">
        <h2 id="basics-h">Scale</h2>
        <div className="row">
          <Field label="Title" unsure={unsure.has('title')}>
            <input id={fieldId('title')} onFocus={() => onFocusField?.('title')} value={scale.title} onChange={(e) => edit((s) => ({ ...s, title: e.target.value }))} />
          </Field>
          <Field label="Short name">
            <input value={scale.shortName ?? ''} onChange={(e) => edit((s) => ({ ...s, shortName: e.target.value || undefined }))} />
          </Field>
        </div>
        <TagEditor tags={scale.tags ?? []} known={knownTags} onChange={(tags) => edit((s) => ({ ...s, tags }))} />
        <label className="check">
          <input type="checkbox" checked={scale.countSuffix} onChange={(e) => edit((s) => ({ ...s, countSuffix: e.target.checked }))} />
          Keep the item count in the title in step with the items (as in "Homophobia-16 Scale")
        </label>
        {check?.titleCountMismatch && (
          <p className="finding">
            The title says {check.titleCountMismatch[0]} items, but the scale has {check.titleCountMismatch[1]}.
            {scale.countSuffix ? ' Saving brings the title in step.' : ''}
          </p>
        )}
        <div className="row">
          <Field label="Format">
            <select value={scale.format} onChange={(e) => setFormat(e.target.value as Format)}>
              {(Object.keys(FORMAT_LABEL) as Format[]).map((f) => (
                <option key={f} value={f}>
                  {FORMAT_LABEL[f]}
                </option>
              ))}
            </select>
          </Field>
          <Field label="Points" unsure={unsure.has('anchors')}>
            <input type="number" min={2} max={11} disabled={locked} value={scale.response.points} onChange={(e) => setPoints(parseInt(e.target.value, 10))} />
          </Field>
          <Field label="Anchor set">
            <select value={scale.response.anchorSet} disabled={locked} onChange={(e) => setAnchors(e.target.value as AnchorSet, scale.response.labelMode)}>
              <option value="agreement">Agreement</option>
              <option value="frequency">Frequency</option>
              <option value="custom">Custom</option>
            </select>
          </Field>
          <Field label="Labels on">
            <select value={scale.response.labelMode} disabled={locked} onChange={(e) => setAnchors(scale.response.anchorSet, e.target.value as LabelMode)}>
              <option value="every">Every point</option>
              <option value="endsOnly">The ends only</option>
            </select>
          </Field>
        </div>
        {check?.forcedChoice && <p className="finding">An even point count has no middle: this scale is forced choice.</p>}
        <fieldset className="plain" disabled={locked}>
        <table className="anchors">
          <caption>Points, labels and the number each point scores</caption>
          <thead>
            <tr>
              <th scope="row">Score</th>
              {scale.response.values.map((v, i) => (
                <th key={i} scope="col">
                  <input
                    aria-label={`Score of point ${i + 1}`}
                    type="number"
                    value={v}
                    onChange={(e) =>
                      edit((s) => ({ ...s, response: { ...s.response, values: s.response.values.map((x, k) => (k === i ? parseInt(e.target.value, 10) || 0 : x)) } }))
                    }
                  />
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            <tr>
              <th scope="row">Label</th>
              {scale.response.labels.map((l, i) => (
                <td key={i}>
                  <input
                    aria-label={`Label of point ${i + 1}`}
                    value={l ?? ''}
                    onChange={(e) =>
                      edit((s) => ({
                        ...s,
                        response: { ...s.response, anchorSet: 'custom', labels: s.response.labels.map((x, k) => (k === i ? e.target.value || null : x)) },
                      }))
                    }
                  />
                </td>
              ))}
            </tr>
            {scale.format === 'likertBoxGrid' && (
              <tr>
                <th scope="row">Box header</th>
                {scale.response.abbreviations.map((a, i) => (
                  <td key={i}>
                    <input
                      aria-label={`Box header of point ${i + 1}`}
                      value={a}
                      onChange={(e) =>
                        edit((s) => ({ ...s, response: { ...s.response, abbreviations: s.response.abbreviations.map((x, k) => (k === i ? e.target.value : x)) } }))
                      }
                    />
                  </td>
                ))}
              </tr>
            )}
          </tbody>
        </table>
        </fieldset>
      </section>

      <section className="panel" aria-labelledby="instr-h">
        <h2 id="instr-h">Instructions</h2>
        <p className="muted">
          {scale.instructions.origin === 'house' && 'House instructions: rewritten whenever the format, points or anchors change.'}
          {scale.instructions.origin === 'verbatim' && 'Kept word for word from the source: never rewritten. A change of format is flagged below instead.'}
          {scale.instructions.origin === 'edited' && 'Edited by hand: no longer rewritten. A change of format is flagged below instead.'}
        </p>
        <Field label="Instructions" unsure={unsure.has('instructions')}>
          <textarea
            rows={4}
            id={fieldId('instructions')}
            onFocus={() => onFocusField?.('instructions')}
            value={scale.instructions.text}
            onChange={(e) => edit((s) => ({ ...s, instructions: { ...s.instructions, text: e.target.value, origin: 'edited' } }))}
          />
        </Field>
        {scale.instructions.origin !== 'house' && (
          <button className="small" onClick={useHouse}>
            <Icon name="instr-house" size={16} />
            Use house instructions for this format
          </button>
        )}
        {check?.instructionFindings.map((f, i) => (
          <p key={i} className="finding">
            "{f.words}": {f.message}
          </p>
        ))}
      </section>

      <section className="panel" aria-labelledby="items-h">
        <h2 id="items-h">
          Items <span className="muted">({scale.items.length})</span>
        </h2>
        <table className="items">
          <thead>
            <tr>
              <th scope="col">No.</th>
              {sd ? (
                <>
                  <th scope="col">Left adjective</th>
                  <th scope="col">Right adjective</th>
                </>
              ) : (
                <th scope="col">Item text</th>
              )}
              <th scope="col">Scoring</th>
              <th scope="col">
                <span className="sr-only">Actions</span>
              </th>
            </tr>
          </thead>
          <tbody>
            {scale.items.map((it, i) => {
              const changed = it.sourceText !== undefined && it.sourceText !== it.text;
              return (
                <Fragment key={i}>
                {it.leadIn !== undefined && (
                  <tr className="lead-in">
                    <td />
                    <td colSpan={sd ? 2 : 1}>
                      <label className="field">
                        <span className="field-label">Printed above item {it.number} (the prompt for the items that follow)</span>
                        <input value={it.leadIn} onChange={(e) => setItem(i, { leadIn: e.target.value })} />
                      </label>
                    </td>
                    <td colSpan={2}>
                      <button className="small" onClick={() => setItem(i, { leadIn: undefined })}>
                        Remove this text
                      </button>
                    </td>
                  </tr>
                )}
                <tr>
                  <td className="num">{it.number}</td>
                  {sd ? (
                    <>
                      <td>
                        <input aria-label={`Item ${it.number} left adjective`} value={it.left ?? ''} onChange={(e) => setItem(i, { left: e.target.value })} />
                        {it.text.trim() && <span className="reason">Item text kept from the earlier format: {it.text}</span>}
                      </td>
                      <td>
                        <input aria-label={`Item ${it.number} right adjective`} value={it.right ?? ''} onChange={(e) => setItem(i, { right: e.target.value })} />
                      </td>
                    </>
                  ) : (
                    <td>
                      <input
                        ref={(el) => {
                          inputs.current[i] = el;
                        }}
                        aria-label={`Item ${it.number} text`}
                        id={fieldId(`item:${it.number}`)}
                        onFocus={() => onFocusField?.(`item:${it.number}`)}
                        value={it.text}
                        onChange={(e) => setItem(i, { text: e.target.value })}
                      />
                      {changed && (
                        <span className="badge" title={`Source text: ${it.sourceText}`}>
                          <Icon name="item-changed" size={14} />
                          Changed from the source
                        </span>
                      )}
                      {(check?.sourceNotes ?? [])
                        .filter((n) => n.items.includes(it.number))
                        .map((n) => (
                          <span key={n.key} className="badge note-badge" title={n.text}>
                            <Icon name={n.handling === 'kept' ? 'chk-published' : 'chk-corrected'} size={14} />
                            {n.handling === 'kept' ? 'As published' : 'Corrected from the source'}
                            <span className="sr-only">: {n.text}</span>
                          </span>
                        ))}
                    </td>
                  )}
                  <td>
                    <select
                      aria-label={`Item ${it.number} scoring`}
                      disabled={locked}
                      value={it.reverse.state}
                      onChange={(e) => setItem(i, { reverse: { ...it.reverse, state: e.target.value as ReverseState } })}
                    >
                      {(Object.keys(REVERSE_LABEL) as ReverseState[]).map((r) => (
                        <option key={r} value={r}>
                          {REVERSE_LABEL[r]}
                        </option>
                      ))}
                    </select>
                    {it.reverse.state === 'suggested' ? (
                      <span className="chip" role="group" aria-label={`Reverse suggestion for item ${it.number}`}>
                        <Icon name="item-reverse-suggested" size={14} />
                        Suggested reversed: {it.reverse.reason ?? 'from the wording'}
                        <button className="small" disabled={locked} onClick={() => setItem(i, { reverse: { state: 'confirmed', reason: it.reverse.reason } })}>
                          <Icon name="ui-check" size={14} />
                          Accept
                        </button>
                        <button className="small" disabled={locked} onClick={() => setItem(i, { reverse: { state: 'notReversed' } })}>
                          <Icon name="ui-close" size={14} />
                          Reject
                        </button>
                      </span>
                    ) : (
                      it.reverse.reason && (
                        <span className="reason">
                          {it.reverse.state === 'confirmed' && <Icon name="item-reversed" size={14} />} {it.reverse.reason}
                        </span>
                      )
                    )}
                  </td>
                  <td className="actions">
                    <button className="icon" aria-label={`Move item ${it.number} up`} title="Move up" onClick={() => moveItem(i, -1)} disabled={locked || i === 0}>
                      <Icon name="ui-chevron-down" size={16} turn={180} />
                    </button>
                    <button className="icon" aria-label={`Move item ${it.number} down`} title="Move down" onClick={() => moveItem(i, 1)} disabled={locked || i === scale.items.length - 1}>
                      <Icon name="ui-chevron-down" size={16} />
                    </button>
                    {!sd && (
                      <button className="icon" aria-label={`Split item ${it.number} at the cursor`} onClick={() => splitAt(i, inputs.current[i] ?? null)} disabled={locked}>
                        <Icon name="item-split" size={16} />
                        Split
                      </button>
                    )}
                    {!sd && (
                      <button className="icon" aria-label={`Merge item ${it.number} with the next`} onClick={() => mergeWithNext(i)} disabled={locked || i === scale.items.length - 1}>
                        <Icon name="item-merge" size={16} />
                        Merge
                      </button>
                    )}
                    <button className="icon" aria-label={`Delete item ${it.number}`} onClick={() => deleteItem(i)} disabled={locked}>
                      <Icon name="ui-delete" size={16} />
                      Delete
                    </button>
                    {it.leadIn === undefined && (
                      <button className="icon" aria-label={`Add text above item ${it.number}`} title="Add a prompt printed above this item, such as a new perspective" onClick={() => setItem(i, { leadIn: '' })}>
                        <Icon name="item-lead-in" size={16} />
                        Text above
                      </button>
                    )}
                  </td>
                </tr>
                </Fragment>
              );
            })}
          </tbody>
        </table>
        <div className="buttons">
          <button className="small" onClick={addItem} disabled={locked}>
            <Icon name="ui-plus" size={16} />
            Add an item
          </button>
          <button className="small" onClick={trimEmpty} disabled={locked}>
            <Icon name="item-trim" size={16} />
            Remove empty lines at the end
          </button>
        </div>
        {locked && <p className="muted"><Icon name="chk-locked" size={14} /> This is a variant: the items keep their order and reverse scoring from the original, and only their wording can change.</p>}
        {scale.recode.stated && <p className="muted">The source's recode line: "{scale.recode.stated.text}"</p>}
        {scale.recode.rule.kind === 'table' && (
          <p className="muted">Recode table kept as the source writes it: {scale.recode.rule.pairs.map(([a, b]) => `${a} = ${b}`).join(', ')}</p>
        )}
      </section>

      <section className="panel" aria-labelledby="sub-h">
        <h2 id="sub-h">Subscales and scoring</h2>
        <fieldset className="plain" disabled={locked}>
        {scale.subscales.length === 0 && <p className="muted">No subscales.</p>}
        {scale.subscales.map((sub: Subscale, i) => (
          <div className="row" key={i}>
            <Field label="Subscale name" unsure={unsure.has(`subscale:${sub.name}`)} hint={sub.otherNames?.length ? `Also called ${sub.otherNames.join(', ')} in the source` : undefined}>
              <input value={sub.name} onChange={(e) => edit((s) => ({ ...s, subscales: s.subscales.map((x, k) => (k === i ? { ...x, name: e.target.value } : x)) }))} />
            </Field>
            <Field label="Items (1, 3, 5 or 1-5 or odd or even)">
              <input
                value={selectorDrafts[i] ?? describeSelector(sub.items)}
                onChange={(e) => setSelectorDrafts((d) => ({ ...d, [i]: e.target.value }))}
                onBlur={(e) => setSelector(i, e.target.value)}
              />
            </Field>
            <button className="small align-end" onClick={() => edit((s) => ({ ...s, subscales: s.subscales.filter((_, k) => k !== i) }))}>
              <Icon name="ui-delete" size={16} />
              Remove
            </button>
          </div>
        ))}
        <button className="small" onClick={() => edit((s) => ({ ...s, subscales: [...s.subscales, { name: 'New subscale', items: { kind: 'list', numbers: [] } }] }))}>
          <Icon name="sub-add" size={16} />
          Add a subscale
        </button>
        <div className="row">
          <Field label="Subscale score">
            <select value={scale.scoring.subscaleMethod} onChange={(e) => edit((s) => ({ ...s, scoring: { ...s.scoring, subscaleMethod: e.target.value as ScoreMethod } }))}>
              <option value="sum">Sum</option>
              <option value="mean">Mean</option>
            </select>
          </Field>
          <label className="check align-end">
            <input type="checkbox" checked={scale.scoring.hasTotal} onChange={(e) => edit((s) => ({ ...s, scoring: { ...s.scoring, hasTotal: e.target.checked } }))} />
            The scale has a total score
          </label>
        </div>
        {scale.scoring.hasTotal && (
          <div className="row">
            <Field label="Total name" hint={scale.scoring.otherTotalNames?.length ? `The source also calls it: ${scale.scoring.otherTotalNames.join('; ')}` : undefined}>
              <input value={scale.scoring.totalName ?? ''} onChange={(e) => edit((s) => ({ ...s, scoring: { ...s.scoring, totalName: e.target.value || undefined } }))} />
            </Field>
            <Field label="Total from">
              <select value={scale.scoring.totalFrom} onChange={(e) => edit((s) => ({ ...s, scoring: { ...s.scoring, totalFrom: e.target.value as TotalFrom } }))}>
                <option value="items">The items</option>
                <option value="subscales">The subscales</option>
              </select>
            </Field>
            <Field label="Total score">
              <select value={scale.scoring.totalMethod} onChange={(e) => edit((s) => ({ ...s, scoring: { ...s.scoring, totalMethod: e.target.value as ScoreMethod } }))}>
                <option value="sum">Sum</option>
                <option value="mean">Mean</option>
              </select>
            </Field>
          </div>
        )}
        <p>
          Score range: stated <strong>{fmtRange(statedRange)}</strong>; computed <strong>{fmtRange(check?.computedRange)}</strong>
          {statedRange && check?.computedRange && (statedRange[0] !== check.computedRange[0] || statedRange[1] !== check.computedRange[1]) && (
            <span className="finding inline"> The stated range does not match the computed one.</span>
          )}
        </p>
      </fieldset>
        <label className="field">
          <span className="field-label">How to read the scores, one line each (prints in the researcher copy)</span>
          <textarea
            rows={Math.min(10, Math.max(2, (scale.scoring.interpretation ?? []).length + 1))}
            value={(scale.scoring.interpretation ?? []).join('\n')}
            placeholder="3-5 Very calm"
            onChange={(e) =>
              edit((s) => ({ ...s, scoring: { ...s.scoring, interpretation: e.target.value ? e.target.value.split('\n') : [] } }))
            }
          />
        </label>
      </section>

      <section className="panel" aria-labelledby="src-h">
        <h2 id="src-h">Sources and licence</h2>
        <Field label="Source (APA 7)" unsure={unsure.has('source')}>
          <textarea rows={2} value={scale.source?.apa ?? ''} onChange={(e) => edit((s) => ({ ...s, source: e.target.value ? { ...(s.source ?? {}), apa: e.target.value } : undefined }))} />
        </Field>
        <RefManLink scale={scale} edit={edit} say={say} fail={fail} />
        <Field label="Adapted from (APA 7)">
          <textarea rows={2} value={scale.adaptedFrom?.apa ?? ''} onChange={(e) => edit((s) => ({ ...s, adaptedFrom: e.target.value ? { ...(s.adaptedFrom ?? {}), apa: e.target.value } : undefined }))} />
        </Field>
        <div className="row">
          <Field label="Licence status">
            <select value={scale.licence.status} onChange={(e) => edit((s) => ({ ...s, licence: { ...s.licence, status: e.target.value as LicenceStatus } }))}>
              {(Object.keys(LICENCE_LABEL) as LicenceStatus[]).map((l) => (
                <option key={l} value={l}>
                  {LICENCE_LABEL[l]}
                </option>
              ))}
            </select>
          </Field>
          <Field label="Permission note">
            <input value={scale.licence.note ?? ''} onChange={(e) => edit((s) => ({ ...s, licence: { ...s.licence, note: e.target.value || undefined } }))} />
          </Field>
          <Field label="Sources print">
            <select value={scale.sourcePlacement} onChange={(e) => edit((s) => ({ ...s, sourcePlacement: e.target.value as Scale['sourcePlacement'] }))}>
              <option value="underScale">Under the scale</option>
              <option value="references">In a References section</option>
              <option value="none">Not at all</option>
            </select>
          </Field>
        </div>
      </section>

      {scale.development && (
        <section className="panel" aria-labelledby="dev-h">
          <h2 id="dev-h">Development record</h2>
          <p className="muted">Matched to the final items by wording, never by number.</p>
          {scale.development.notes?.map((n, i) => (
            <p key={i} className="muted">
              {n}
            </p>
          ))}
          <table className="dev">
            <thead>
              <tr>
                <th scope="col">Dev. no.</th>
                <th scope="col">Item</th>
                {scale.development.factors.map((f) => (
                  <th scope="col" key={f}>
                    {f}
                  </th>
                ))}
                <th scope="col">Kept</th>
                <th scope="col">Final no.</th>
              </tr>
            </thead>
            <tbody>
              {scale.development.items.map((d) => (
                <tr key={d.number}>
                  <td className="num">{d.number}</td>
                  <td>{d.text}</td>
                  {d.loadings.map((l, k) => (
                    <td className="num" key={k}>
                      {l === null ? '' : l.toFixed(3)}
                    </td>
                  ))}
                  <td>{d.markedKept === undefined ? '' : d.markedKept ? 'Yes' : 'No'}</td>
                  <td className="num">{d.finalNumber ?? ''}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}

      {scale.unparsed && scale.unparsed.length > 0 && (
        <section className="panel" aria-labelledby="unp-h">
          <h2 id="unp-h">Lines the importer did not understand</h2>
          <p className="muted">Copy anything you need into a field above, then clear the list.</p>
          <ul className="unparsed">
            {scale.unparsed.map((u, i) => (
              <li key={i}>
                <span className="muted">Paragraph {u.line}:</span> {u.text}
              </li>
            ))}
          </ul>
          <button className="small" onClick={() => edit((s) => ({ ...s, unparsed: [] }))}>
            <Icon name="item-clear" size={16} />
            Clear the list
          </button>
        </section>
      )}

      {scale.history.length > 0 && (
        <section className="panel" aria-labelledby="hist-h">
          <h2 id="hist-h">History</h2>
          <ul>
            {scale.history.map((h) => (
              <li key={h.version}>
                Version {h.version}, {h.savedAt.replace('T', ' ').replace('Z', ' UTC')}: {h.summary}
              </li>
            ))}
          </ul>
        </section>
      )}
    </div>
  );
}
