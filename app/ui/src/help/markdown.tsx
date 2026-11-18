// A small Markdown renderer for the help pages, adapted from PrismStatistics' (same author and licence): headings,
// paragraphs, bullet and numbered lists, tables, **bold**, *italic*, `code` and links. A link to `topic:<id>` opens that
// help page; the window opens no web pages, so any other link shows as its words and address.

import { Fragment, type ReactNode } from 'react';

function inline(text: string, open: (id: string) => void, key = ''): ReactNode[] {
  const out: ReactNode[] = [];
  const re = /(\*\*([^*]+)\*\*|\*([^*]+)\*|`([^`]+)`|\[([^\]]+)\]\(([^)]+)\))/g;
  let last = 0;
  let m: RegExpExecArray | null;
  let n = 0;
  while ((m = re.exec(text))) {
    if (m.index > last) out.push(text.slice(last, m.index));
    const k = `${key}-${n++}`;
    if (m[2] !== undefined) out.push(<strong key={k}>{inline(m[2], open, k)}</strong>);
    else if (m[3] !== undefined) out.push(<em key={k}>{m[3]}</em>);
    else if (m[4] !== undefined) out.push(<code key={k}>{m[4]}</code>);
    else {
      const [label, href] = [m[5], m[6]];
      out.push(
        href.startsWith('topic:') ? (
          <a
            key={k}
            href={`#${href.slice(6)}`}
            onClick={(e) => {
              e.preventDefault();
              open(href.slice(6));
            }}
          >
            {label}
          </a>
        ) : (
          <Fragment key={k}>
            {label} ({href})
          </Fragment>
        ),
      );
    }
    last = m.index + m[0].length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

/** Renders a help page. The first `# ` heading is left out: the viewer shows the title itself. */
export function Markdown({ text, open }: { text: string; open: (id: string) => void }) {
  const lines = text.replace(/\r/g, '').split('\n');
  const blocks: ReactNode[] = [];
  let i = 0;
  let skippedTitle = false;
  while (i < lines.length) {
    const line = lines[i];
    const k = `b${i}`;
    if (!line.trim()) {
      i++;
    } else if (/^#{1,4} /.test(line)) {
      const level = line.indexOf(' ');
      const t = line.slice(level + 1);
      i++;
      if (level === 1 && !skippedTitle) {
        skippedTitle = true;
        continue;
      }
      blocks.push(level <= 2 ? <h3 key={k}>{inline(t, open, k)}</h3> : <h4 key={k}>{inline(t, open, k)}</h4>);
    } else if (/^\s*([-*]|\d+\.) /.test(line)) {
      const ordered = /^\s*\d+\. /.test(line);
      const items: string[] = [];
      while (i < lines.length && /^\s*([-*]|\d+\.) /.test(lines[i])) {
        let item = lines[i].replace(/^\s*([-*]|\d+\.) /, '');
        i++;
        while (i < lines.length && /^\s{2,}\S/.test(lines[i]) && !/^\s*([-*]|\d+\.) /.test(lines[i])) item += ' ' + lines[i++].trim();
        items.push(item);
      }
      const lis = items.map((t, j) => <li key={j}>{inline(t, open, `${k}-${j}`)}</li>);
      blocks.push(ordered ? <ol key={k}>{lis}</ol> : <ul key={k}>{lis}</ul>);
    } else if (line.startsWith('|')) {
      const rows: string[][] = [];
      while (i < lines.length && lines[i].startsWith('|')) {
        if (!/^\|[\s:|-]+\|$/.test(lines[i].trim()))
          rows.push(
            lines[i]
              .trim()
              .replace(/^\||\|$/g, '')
              .split('|')
              .map((c) => c.trim()),
          );
        i++;
      }
      const [head, ...body] = rows;
      blocks.push(
        <table key={k} className="helptable">
          <thead>
            <tr>
              {head.map((c, j) => (
                <th key={j} scope="col">
                  {inline(c, open, `${k}h${j}`)}
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {body.map((r, j) => (
              <tr key={j}>
                {r.map((c, l) => (
                  <td key={l}>{inline(c, open, `${k}r${j}c${l}`)}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>,
      );
    } else {
      const para: string[] = [];
      while (i < lines.length && lines[i].trim() && !/^(#{1,4} |\||\s*([-*]|\d+\.) )/.test(lines[i])) para.push(lines[i++].trim());
      blocks.push(<p key={k}>{inline(para.join(' '), open, k)}</p>);
    }
  }
  return <Fragment>{blocks}</Fragment>;
}
