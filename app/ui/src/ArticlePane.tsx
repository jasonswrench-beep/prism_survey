// The left pane of the confirm screen for a journal article (specification, "The confirm and correct screen"): the page
// picture with each parsed field outlined in its own colour. Clicking an outline selects the field on the form; a field
// selected on the form is drawn thicker here. Dragging a new outline on the page re-parses that region.

import { useEffect, useRef, useState } from 'react';
import { call } from './bridge';
import type { Provenance, Scale } from './types';

export interface ArticlePage {
  number: number;
  width: number;
  height: number;
  source: 'textLayer' | 'ocr' | 'none';
}

const COLOURS: Record<string, string> = {
  title: '#8e44ad',
  instructions: '#2471a3',
  anchors: '#1e8449',
  item: '#d35400',
  source: '#7f8c8d',
  other: '#b7950b',
};

function colourOf(field: string): string {
  if (field.startsWith('item:')) return COLOURS.item;
  return COLOURS[field] ?? COLOURS.other;
}

export function fieldId(field: string): string {
  return 'field-' + field.replace(':', '-');
}

export function ArticlePane({
  path,
  pages,
  scale,
  selected,
  onSelect,
  onRegion,
  fail,
}: {
  path: string;
  pages: ArticlePage[];
  scale: Scale;
  selected: string | null;
  onSelect: (field: string) => void;
  onRegion: (page: number, region: [number, number, number, number]) => void;
  fail: (e: unknown) => void;
}) {
  const firstPage = scale.provenance.find((p) => p.page)?.page ?? pages[0]?.number ?? 1;
  const [page, setPage] = useState<number>(firstPage);
  const [image, setImage] = useState<string | null>(null);
  const [drag, setDrag] = useState<{ x0: number; y0: number; x1: number; y1: number } | null>(null);
  const svgRef = useRef<SVGSVGElement>(null);
  const info = pages.find((p) => p.number === page) ?? pages[0];

  useEffect(() => {
    setPage(firstPage);
    // A new scale (another candidate, or a re-parse) starts on its own page.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [scale.id]);

  useEffect(() => {
    let live = true;
    setImage(null);
    call<{ dataUrl: string }>('article.page', { path, page, width: 1100 })
      .then((r) => live && setImage(r.dataUrl))
      .catch(fail);
    return () => {
      live = false;
    };
  }, [path, page, fail]);

  // Follow a field selected on the form to its page.
  useEffect(() => {
    if (!selected) return;
    const p = scale.provenance.find((x) => x.field === selected && x.page);
    if (p?.page && p.page !== page) setPage(p.page);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selected]);

  const outlines = scale.provenance.filter((p): p is Provenance & { region: [number, number, number, number] } => p.page === page && Array.isArray(p.region));

  function toPoints(ev: React.MouseEvent): [number, number] {
    const r = svgRef.current!.getBoundingClientRect();
    return [((ev.clientX - r.left) / r.width) * info.width, ((ev.clientY - r.top) / r.height) * info.height];
  }

  return (
    <section className="article-pane" aria-label="Article page">
      <div className="page-nav">
        <button className="small" onClick={() => setPage((p) => Math.max(pages[0].number, p - 1))} disabled={page <= pages[0].number} aria-label="Previous page">
          ←
        </button>
        <span>
          Page {page} of {pages.length}
          {info?.source === 'ocr' && <span className="badge warn">Read by text recognition</span>}
        </span>
        <button className="small" onClick={() => setPage((p) => Math.min(pages[pages.length - 1].number, p + 1))} disabled={page >= pages[pages.length - 1].number} aria-label="Next page">
          →
        </button>
      </div>
      <p className="muted small-print">Drag across the page to outline a region and read it again.</p>
      <div className="page-frame">
        {image ? <img src={image} alt={`Page ${page} of the article`} /> : <div className="page-loading">Drawing the page…</div>}
        {info && (
          <svg
            ref={svgRef}
            viewBox={`0 0 ${info.width} ${info.height}`}
            preserveAspectRatio="none"
            onMouseDown={(e) => {
              const [x, y] = toPoints(e);
              setDrag({ x0: x, y0: y, x1: x, y1: y });
            }}
            onMouseMove={(e) => {
              if (!drag) return;
              const [x, y] = toPoints(e);
              setDrag({ ...drag, x1: x, y1: y });
            }}
            onMouseUp={() => {
              if (drag && Math.abs(drag.x1 - drag.x0) > 8 && Math.abs(drag.y1 - drag.y0) > 8) {
                onRegion(page, [Math.min(drag.x0, drag.x1), Math.min(drag.y0, drag.y1), Math.max(drag.x0, drag.x1), Math.max(drag.y0, drag.y1)]);
              }
              setDrag(null);
            }}
            onMouseLeave={() => setDrag(null)}
          >
            {outlines.map((p, i) => {
              const [x0, y0, x1, y1] = p.region;
              const on = selected === p.field;
              return (
                <rect
                  key={i}
                  x={x0 - 2}
                  y={y0 - 2}
                  width={x1 - x0 + 4}
                  height={y1 - y0 + 4}
                  fill={on ? colourOf(p.field) + '33' : 'transparent'}
                  stroke={colourOf(p.field)}
                  strokeWidth={on ? 2.5 : 1}
                  onMouseDown={(e) => e.stopPropagation()}
                  onClick={() => onSelect(p.field)}
                >
                  <title>
                    {p.field} ({Math.round(p.confidence * 100)}% sure): {p.rule}
                  </title>
                </rect>
              );
            })}
            {drag && (
              <rect x={Math.min(drag.x0, drag.x1)} y={Math.min(drag.y0, drag.y1)} width={Math.abs(drag.x1 - drag.x0)} height={Math.abs(drag.y1 - drag.y0)} fill="#2471a322" stroke="#2471a3" strokeDasharray="4 3" />
            )}
          </svg>
        )}
      </div>
      <ul className="legend" aria-label="Outline colours">
        <li>
          <span style={{ background: COLOURS.title }} /> Title
        </li>
        <li>
          <span style={{ background: COLOURS.instructions }} /> Instructions
        </li>
        <li>
          <span style={{ background: COLOURS.anchors }} /> Anchors
        </li>
        <li>
          <span style={{ background: COLOURS.item }} /> Items
        </li>
      </ul>
    </section>
  );
}
