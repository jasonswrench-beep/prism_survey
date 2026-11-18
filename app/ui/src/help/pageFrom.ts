// Reading and searching help pages, apart from the loader (index.ts) so the tests can use them without the build.

export interface HelpPage {
  id: string;
  title: string;
  text: string;
}

/** A page from its file name (`NN-id.md`) and text, whose first `# ` line is the title. */
export function pageFrom(file: string, text: string): HelpPage {
  const name = file.split('/').pop()!.replace(/\.md$/, '');
  const id = name.replace(/^\d+-/, '');
  const title = /^# (.+)$/m.exec(text)?.[1].trim() ?? id;
  return { id, title, text };
}

/** The pages whose title or text holds every word of the search, title matches first. */
export function search(pages: HelpPage[], query: string): HelpPage[] {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return pages;
  const has = (p: HelpPage, w: string) => p.title.toLowerCase().includes(w) || p.text.toLowerCase().includes(w);
  const hits = pages.filter((p) => words.every((w) => has(p, w)));
  const inTitle = (p: HelpPage) => words.every((w) => p.title.toLowerCase().includes(w));
  return [...hits.filter(inTitle), ...hits.filter((p) => !inTitle(p))];
}
