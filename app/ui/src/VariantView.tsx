import { useState } from 'react';
import { call } from './bridge';
import { Icon } from './Icon';
import type { ParentChange, Scale } from './types';

interface DraftRow {
  number: number;
  original: string;
  proposed: string;
  originalLeft?: string;
  originalRight?: string;
  left?: string;
  right?: string;
  found: boolean;
}

interface Draft {
  scale: Scale;
  rows: DraftRow[];
  titleChanged: boolean;
  instructionsChanged: boolean;
}

type RowState = { text: string; left?: string; right?: string; decision: 'open' | 'accepted' | 'flagged'; reason: string };

/// Splits `next` into pieces, marking the words that are not in `prev` in the same order (a word-level diff), so the
/// changed words show on the side-by-side screen.
export function changedWords(prev: string, next: string): { text: string; changed: boolean }[] {
  const a = prev.split(/(\s+)/).filter((x) => x !== '');
  const b = next.split(/(\s+)/).filter((x) => x !== '');
  const n = a.length;
  const m = b.length;
  const lcs: number[][] = Array.from({ length: n + 1 }, () => new Array<number>(m + 1).fill(0));
  for (let i = n - 1; i >= 0; i--) for (let j = m - 1; j >= 0; j--) lcs[i][j] = a[i] === b[j] ? lcs[i + 1][j + 1] + 1 : Math.max(lcs[i + 1][j], lcs[i][j + 1]);
  const out: { text: string; changed: boolean }[] = [];
  let i = 0;
  let j = 0;
  while (j < m) {
    if (i < n && a[i] === b[j]) {
      out.push({ text: b[j], changed: false });
      i++;
      j++;
    } else if (i < n && lcs[i + 1][j] >= lcs[i][j + 1]) {
      i++;
    } else {
      out.push({ text: b[j], changed: !/^\s+$/.test(b[j]) });
      j++;
    }
  }
  return out;
}

function Marked({ prev, next }: { prev: string; next: string }) {
  return (
    <span className="diff">
      {changedWords(prev, next).map((p, k) =>
        p.changed ? (
          <mark key={k}>{p.text}</mark>
        ) : (
          <span key={k}>{p.text}</span>
        ),
      )}
    </span>
  );
}

/// Change the context, by hand: the researcher names the old target and the new one, the target words are replaced,
/// and each item is accepted, edited, or flagged as one that cannot be reworded without changing what it asks. The
/// structure stays locked. The finished variant opens on the confirm screen, where the variant structure check runs
/// before it is saved.
export function VariantView({
  parentPath,
  parentTitle,
  onDone,
  onCancel,
  fail,
}: {
  parentPath: string;
  parentTitle: string;
  onDone: (s: Scale, notes: string[]) => void;
  onCancel: () => void;
  fail: (e: unknown) => void;
}) {
  const [oldTarget, setOldTarget] = useState('');
  const [newTarget, setNewTarget] = useState('');
  const [context, setContext] = useState('');
  const [extra, setExtra] = useState<{ from: string; to: string }[]>([]);
  const [draft, setDraft] = useState<Draft | null>(null);
  const [rows, setRows] = useState<RowState[]>([]);
  const [busy, setBusy] = useState(false);

  async function makeDraft() {
    setBusy(true);
    try {
      const d = await call<Draft>('variant.draft', {
        path: parentPath,
        request: { oldTarget, newTarget, context: context || null, replacements: extra.filter((r) => r.from.trim()) },
      });
      setDraft(d);
      setRows(d.rows.map((r) => ({ text: r.proposed, left: r.left, right: r.right, decision: 'open', reason: '' })));
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
  }

  const setRow = (i: number, patch: Partial<RowState>) => setRows((rs) => rs.map((r, k) => (k === i ? { ...r, ...patch } : r)));
  const open = rows.filter((r) => r.decision === 'open').length;
  const badFlag = rows.some((r) => r.decision === 'flagged' && !r.reason.trim());

  async function finish() {
    if (!draft) return;
    try {
      const decisions = draft.rows.map((r, i) => {
        const st = rows[i];
        return st.decision === 'flagged'
          ? { number: r.number, text: r.original, left: r.originalLeft ?? null, right: r.originalRight ?? null, flagged: st.reason }
          : { number: r.number, text: st.text, left: st.left ?? null, right: st.right ?? null, flagged: null };
      });
      const s = await call<Scale>('variant.finish', { scale: draft.scale, rows: decisions });
      const notes = [
        `A variant of ${parentTitle}: "${oldTarget}" changed to "${newTarget}". Items, points, anchors, reverse items, subscales and scoring are kept from the original.`,
      ];
      const also = [draft.titleChanged && 'the title', draft.instructionsChanged && 'the instructions'].filter(Boolean).join(' and ');
      if (also) notes.push(`The target words were also replaced in ${also}; check the wording.`);
      const flagged = rows.filter((r) => r.decision === 'flagged').length;
      if (flagged) notes.push(`${flagged} item(s) were flagged as not rewordable and keep the original wording.`);
      onDone(s, notes);
    } catch (e) {
      fail(e);
    }
  }

  if (!draft) {
    return (
      <section className="panel variant" aria-labelledby="var-h">
        <h2 id="var-h">Change the context: {parentTitle}</h2>
        <p className="muted">
          The variant keeps this scale's items in order, its points and anchors, reverse items, subscales, scoring and score range. Only the target words and the wording around them change. It is saved as
          its own scale, linked to this one.
        </p>
        <div className="row">
          <label className="field">
            <span className="field-label">Old target, as the items name it</span>
            <input autoFocus value={oldTarget} placeholder="physician" onChange={(e) => setOldTarget(e.target.value)} />
          </label>
          <label className="field">
            <span className="field-label">New target</span>
            <input value={newTarget} placeholder="AI" onChange={(e) => setNewTarget(e.target.value)} />
          </label>
        </div>
        <label className="field">
          <span className="field-label">Context (optional)</span>
          <input value={context} placeholder="a chatbot used for health questions" onChange={(e) => setContext(e.target.value)} />
        </label>
        <h3>Other wording to replace (optional)</h3>
        {extra.map((r, i) => (
          <div className="row" key={i}>
            <label className="field">
              <span className="field-label">Replace</span>
              <input value={r.from} placeholder="my doctor" onChange={(e) => setExtra((x) => x.map((y, k) => (k === i ? { ...y, from: e.target.value } : y)))} />
            </label>
            <label className="field">
              <span className="field-label">With</span>
              <input value={r.to} placeholder="the AI I use" onChange={(e) => setExtra((x) => x.map((y, k) => (k === i ? { ...y, to: e.target.value } : y)))} />
            </label>
            <button className="small align-end" onClick={() => setExtra((x) => x.filter((_, k) => k !== i))} aria-label={`Remove replacement ${i + 1}`}>
              <Icon name="ui-delete" size={16} />
              Remove
            </button>
          </div>
        ))}
        <div className="buttons">
          <button className="small" onClick={() => setExtra((x) => [...x, { from: '', to: '' }])}>
            <Icon name="ui-plus" size={16} />
            Add wording to replace
          </button>
        </div>
        <div className="buttons">
          <button className="primary" onClick={makeDraft} disabled={busy || !oldTarget.trim() || !newTarget.trim()}>
            <Icon name="var-replace" />
            Replace the target words
          </button>
          <button onClick={onCancel}>Cancel</button>
        </div>
      </section>
    );
  }

  return (
    <section className="panel variant" aria-labelledby="var-h">
      <h2 id="var-h">
        Change the context: {parentTitle} <span className="muted">("{oldTarget}" to "{newTarget}")</span>
      </h2>
      <p className="muted">
        Each original item is beside its new wording, with the changed words marked. Accept it, edit it, or flag it when it cannot be reworded without changing what it asks; a flagged item keeps the
        original wording. {open > 0 ? `${open} item(s) still to decide.` : 'Every item is decided.'}
      </p>
      <div className="buttons">
        <button className="small" onClick={() => setRows((rs) => rs.map((r) => (r.decision === 'open' ? { ...r, decision: 'accepted' } : r)))} disabled={open === 0}>
          <Icon name="var-accept-all" size={16} />
          Accept the rest as shown
        </button>
      </div>
      <table className="variant-table">
        <thead>
          <tr>
            <th scope="col">No.</th>
            <th scope="col">Original</th>
            <th scope="col">New wording</th>
            <th scope="col">
              <span className="sr-only">Decision</span>
            </th>
          </tr>
        </thead>
        <tbody>
          {draft.rows.map((r, i) => {
            const st = rows[i];
            const sd = r.originalLeft !== undefined || r.originalRight !== undefined;
            return (
              <tr key={r.number} className={'decision-' + st.decision}>
                <td className="num">{r.number}</td>
                <td>{sd ? `${r.originalLeft ?? ''} / ${r.originalRight ?? ''}` : r.original}</td>
                <td>
                  {st.decision === 'flagged' ? (
                    <>
                      <span className="muted">Kept as the original. </span>
                      <label className="field">
                        <span className="field-label">Why it cannot be reworded (kept with the variant)</span>
                        <input autoFocus value={st.reason} onChange={(e) => setRow(i, { reason: e.target.value })} />
                      </label>
                    </>
                  ) : sd ? (
                    <div className="row">
                      <input aria-label={`Item ${r.number} new left adjective`} value={st.left ?? ''} onChange={(e) => setRow(i, { left: e.target.value, decision: 'accepted' })} />
                      <input aria-label={`Item ${r.number} new right adjective`} value={st.right ?? ''} onChange={(e) => setRow(i, { right: e.target.value, decision: 'accepted' })} />
                    </div>
                  ) : (
                    <>
                      <input aria-label={`Item ${r.number} new wording`} value={st.text} onChange={(e) => setRow(i, { text: e.target.value, decision: 'accepted' })} />
                      <Marked prev={r.original} next={st.text} />
                    </>
                  )}
                  {!r.found && st.decision !== 'flagged' && <span className="reason">No target word was found in this item; reword it by hand or accept it as it is.</span>}
                </td>
                <td className="actions">
                  <button className="small" aria-pressed={st.decision === 'accepted'} onClick={() => setRow(i, { decision: 'accepted' })}>
                    <Icon name="ui-check" size={16} />
                    Accept
                  </button>
                  <button className="small" aria-pressed={st.decision === 'flagged'} onClick={() => setRow(i, { decision: st.decision === 'flagged' ? 'open' : 'flagged' })}>
                    <Icon name="var-flag" size={16} />
                    {st.decision === 'flagged' ? 'Unflag' : 'Flag…'}
                  </button>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <div className="buttons">
        <button className="primary" onClick={finish} disabled={open > 0 || badFlag}>
          <Icon name="cmd-variant" />
          Make the variant
        </button>
        <button onClick={() => setDraft(null)}>Back</button>
        <button onClick={onCancel}>Cancel</button>
      </div>
      {badFlag && <p className="finding">Say why each flagged item cannot be reworded.</p>}
    </section>
  );
}

/// The variant's link to its parent, shown on the confirm screen: what it is adapted from, what is locked, and the
/// parent's changes since, each offered to carry or leave, never applied unasked.
export function VariantPanel({
  scale,
  changes,
  parentPath,
  edit,
  fail,
}: {
  scale: Scale;
  changes: ParentChange[] | null;
  parentPath: string | null;
  edit: (f: (s: Scale) => Scale) => void;
  fail: (e: unknown) => void;
}) {
  const v = scale.variantOf;
  if (!v) return null;
  async function carry(c: ParentChange) {
    try {
      const s = await call<Scale>('variant.carry', { scale, parentPath, change: c });
      edit(() => s);
    } catch (e) {
      fail(e);
    }
  }
  async function decline(c: ParentChange) {
    try {
      const s = await call<Scale>('variant.decline', { scale, key: c.key });
      edit(() => s);
    } catch (e) {
      fail(e);
    }
  }
  const describe = (c: ParentChange): string => {
    switch (c.kind) {
      case 'itemText':
        return `Item ${c.number} now reads "${c.now}" in the original (it read "${c.was}"). Carried, it becomes "${c.proposed}".`;
      case 'reverse':
        return `Item ${c.number} is now ${c.reversed ? 'reverse scored' : 'not reverse scored'} in the original.`;
      case 'sourceNote':
        return `A new source note in the original: ${c.text}`;
      case 'structure':
        return `The original's ${c.what.join(', ')} changed.`;
      case 'itemCount':
        return `The original now has ${c.now} items (it had ${c.was}). A variant keeps its parent's items one for one, so make a new variant instead.`;
    }
  };
  return (
    <section className="panel variant-link" aria-labelledby="vlink-h">
      <h2 id="vlink-h">
        <Icon name="lib-variant" />
        Adapted variant
      </h2>
      <p>
        Adapted from <strong>{v.parentTitle}</strong> (its saved version {v.parentVersion}): "{v.oldTarget}" changed to "{v.newTarget}"{v.context ? ` (${v.context})` : ''}.
      </p>
      <p className="muted">
        Locked from the original: the items and their order, points and anchors, reverse items, subscales, scoring and score range. For a structural change, copy the scale as a new scale instead.
        Reliability and validity reported for the original do not carry over; the researcher copy says so.
      </p>
      {changes && changes.length > 0 && (
        <>
          <h3>Changes in the original since this variant was made</h3>
          <ul className="findings">
            {changes.map((c) => (
              <li key={c.key} className="finding-row">
                {describe(c)}
                <div className="buttons">
                  {c.kind !== 'itemCount' && (
                    <button className="small" onClick={() => carry(c)} disabled={!parentPath}>
                      <Icon name="var-carry" size={16} />
                      Carry into the variant
                    </button>
                  )}
                  <button className="small" onClick={() => decline(c)}>
                    <Icon name="var-leave" size={16} />
                    Leave it
                  </button>
                </div>
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
}
