// The help pages (decision 81): one Markdown file per page in ./pages, named `NN-id.md` so they list in order. The first
// `# ` line is the page's title. A link written `topic:<id>` opens that page.

import { pageFrom, type HelpPage } from './pageFrom';

export { search, type HelpPage } from './pageFrom';

const files = import.meta.glob('./pages/*.md', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

export const PAGES: HelpPage[] = Object.keys(files)
  .sort()
  .map((f) => pageFrom(f, files[f]));
