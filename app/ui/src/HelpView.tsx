// The Help tab (decision 81): the guide to each screen, with search, and Check for Updates (app/src-tauri/src/updates.rs;
// docs/UPDATES.md). A check is made only when the person presses the button, or once a day if they turned that on;
// installing is a second step that needs their go-ahead.

import { useEffect, useMemo, useState } from 'react';
import { call, errorText, inTauri } from './bridge';
import { PAGES, search } from './help';
import { Markdown } from './help/markdown';
import { progressPercent, readPrefs, writePrefs } from './updates';
import { Icon } from './Icon';

interface UpdateState {
  version: string;
  setUp: boolean;
  host: string;
  rollback: boolean;
}

interface UpdateFound {
  available: boolean;
  version?: string;
  notes?: string;
  date?: string;
}

type Phase = 'idle' | 'checking' | 'found' | 'current' | 'installing' | 'failed' | 'backChecking' | 'backFound' | 'backNone';

function when(iso: string): string {
  const t = Date.parse(iso);
  return Number.isNaN(t) ? 'never' : new Date(t).toLocaleString();
}

function UpdatesPanel({ unsaved }: { unsaved: boolean }) {
  const desktop = inTauri;
  const [state, setState] = useState<UpdateState | null>(null);
  const [phase, setPhase] = useState<Phase>('idle');
  const [found, setFound] = useState<UpdateFound | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [percent, setPercent] = useState<number | null>(null);
  const [prefs, setPrefs] = useState(readPrefs);

  useEffect(() => {
    if (!desktop) return;
    call<UpdateState>('update.state').then(setState, (e) => setError(errorText(e)));
  }, [desktop]);

  const keep = (p: typeof prefs) => {
    setPrefs(p);
    writePrefs(p);
  };

  async function check() {
    setError(null);
    setPhase('checking');
    try {
      const r = await call<UpdateFound>('update.check');
      keep({ ...readPrefs(), lastCheck: new Date().toISOString() });
      setFound(r);
      setPhase(r.available ? 'found' : 'current');
    } catch (e) {
      setError(errorText(e));
      setPhase('failed');
    }
  }

  async function install(back: boolean) {
    if (unsaved && !window.confirm('Some work is not saved. Installing closes Prism Survey. Install anyway?')) return;
    setError(null);
    setPhase('installing');
    let stop: (() => void) | undefined;
    try {
      const { listen } = await import('@tauri-apps/api/event');
      stop = await listen<{ received: number; total: number | null }>('update-progress', (e) => setPercent(progressPercent(e.payload.received, e.payload.total)));
      const r = await call<{ installed: boolean }>(back ? 'update.rollback' : 'update.install');
      if (!r.installed) setPhase(back ? 'backNone' : 'current');
    } catch (e) {
      setError(errorText(e));
      setPhase('failed');
    } finally {
      stop?.();
    }
  }

  async function checkBack() {
    setError(null);
    setPhase('backChecking');
    try {
      const r = await call<UpdateFound>('update.rollbackCheck');
      setFound(r);
      setPhase(r.available ? 'backFound' : 'backNone');
    } catch (e) {
      setError(errorText(e));
      setPhase('failed');
    }
  }

  const busy = phase === 'checking' || phase === 'installing' || phase === 'backChecking';
  return (
    <section className="panel" aria-labelledby="upd-h">
      <h2 id="upd-h">
        <Icon name="ui-refresh" />
        Check for updates
      </h2>
      {!desktop ? (
        <p className="muted">Updates can only be checked in the installed program.</p>
      ) : !state ? (
        error ? <p className="finding">{error}</p> : <p className="muted">Reading the version…</p>
      ) : (
        <>
          <p>
            This is Prism Survey {state.version}.{' '}
            {state.setUp
              ? `A check asks ${state.host} whether a newer version has been published, sending only the program's version and the kind of computer. Nothing is downloaded or installed without your go-ahead.`
              : "This copy is not set up to check for updates. Download new versions from the project's releases page."}
          </p>
          {state.setUp && (
            <>
              <label className="check">
                <input type="checkbox" checked={prefs.daily} onChange={(e) => keep({ ...prefs, daily: e.target.checked })} />
                Check for updates once a day
              </label>
              <p className="hint">Off unless you turn it on. A newer version is only announced; nothing is downloaded. Last checked: {when(prefs.lastCheck)}.</p>
              <div className="buttons wrap">
                {phase === 'found' ? (
                  <button className="primary" onClick={() => install(false)}>
                    Download and install {found?.version}
                  </button>
                ) : phase === 'backFound' ? (
                  <button className="primary" onClick={() => install(true)}>
                    Install {found?.version}
                  </button>
                ) : (
                  <button className="primary" onClick={check} disabled={busy}>
                    <Icon name="ui-refresh" size={16} />
                    {phase === 'checking' ? 'Checking…' : 'Check now'}
                  </button>
                )}
                {state.rollback && phase !== 'backFound' && (
                  <button onClick={checkBack} disabled={busy}>
                    Return to the previous version…
                  </button>
                )}
              </div>
            </>
          )}
          <div role="status" aria-live="polite">
            {phase === 'current' && <p>You have the newest version.</p>}
            {phase === 'found' && found && (
              <>
                <p>
                  <strong>Prism Survey {found.version}</strong> is available{found.date ? ` (published ${found.date.slice(0, 10)})` : ''}. Installing it downloads the installer,
                  checks that it carries the maintainer's update signature, then closes Prism Survey and runs the installer. Save your work first.
                </p>
                {found.notes && (
                  <details open>
                    <summary>What's new</summary>
                    <pre className="updatenotes">{found.notes}</pre>
                  </details>
                )}
              </>
            )}
            {phase === 'backChecking' && <p>Looking for the previous version…</p>}
            {phase === 'backNone' && <p>No earlier version is offered.</p>}
            {phase === 'backFound' && found && (
              <p>
                <strong>Prism Survey {found.version}</strong>, the previous version, can be installed in place of this one. Its installer is checked against the maintainer's signature
                in the same way.
              </p>
            )}
            {phase === 'installing' && <p>Downloading{percent === null ? '…' : ` (${percent}%)`}. Prism Survey will close and the installer will start.</p>}
          </div>
          {error && <p className="finding">{error}</p>}
        </>
      )}
    </section>
  );
}

export function HelpView({ page, setPage, unsaved }: { page: string; setPage: (id: string) => void; unsaved: boolean }) {
  const [query, setQuery] = useState('');
  const shown = useMemo(() => search(PAGES, query), [query]);
  const current = PAGES.find((p) => p.id === page) ?? PAGES[0];
  return (
    <div className="helpview">
      <nav className="panel helpnav" aria-label="Help pages">
        <label className="field">
          <span className="field-label">
            <Icon name="cmd-search" size={14} />
            Search Help
          </span>
          <input type="search" value={query} placeholder="A word or two" onChange={(e) => setQuery(e.target.value)} />
        </label>
        {shown.length === 0 ? (
          <p className="muted">No page has those words.</p>
        ) : (
          <ul className="qlist">
            {shown.map((p) => (
              <li key={p.id}>
                <button className={'helplink' + (p.id === current.id ? ' current' : '')} aria-current={p.id === current.id ? 'page' : undefined} onClick={() => setPage(p.id)}>
                  {p.title}
                </button>
              </li>
            ))}
          </ul>
        )}
      </nav>
      <article className="helppage" aria-labelledby="help-title">
        <section className="panel">
          <h1 id="help-title">{current.title}</h1>
          <div className="helptext">
            <Markdown text={current.text} open={setPage} />
          </div>
        </section>
        {current.id === 'updates' && <UpdatesPanel unsaved={unsaved} />}
      </article>
    </div>
  );
}
