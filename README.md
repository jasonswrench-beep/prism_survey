# Prism Survey

A free Windows tool in the Prism suite that turns a published measure into a clean, correctly scored scale and places it in a Word questionnaire. Version 1 prepares instruments only: it collects no responses. Everything stays on your computer.

The design is the Prism Survey Specification (Jason Wrench, 2026-11-17). Decisions made while building are in [docs/DECISIONS.md](docs/DECISIONS.md), the scale file in [docs/PSCALE-FORMAT.md](docs/PSCALE-FORMAT.md), and the command layer in [docs/API.md](docs/API.md). What only a person can check is in [TESTING-TODO.md](TESTING-TODO.md).

## Where it stands

Phases 1 (library and formats) and 2 (the scoring self-check) are built, with tags from the [specification addendum](docs/SPEC-ADDENDUM.md):

- **The scale record** and its `.pscale` file: one plain JSON file per scale with a version number in every file, saved whole or not at all, with a history of each save.
- **The three formats** from the stored templates (Likert blank-line, Likert box grid, semantic differential), 2 to 11 points, agreement, frequency or custom anchors, labels on every point or the ends.
- **House instructions** that follow the format and point count, including the semantic differential wording for 5, 7 and 9 points. Instructions kept from a source, or edited by hand, are never rewritten; a change of format flags the words that no longer fit.
- **Import of stored scales** from `.txt`, `.docx` and Word 97-2003 `.doc`: the "_____1." item lines, anchor rows, recode lines and tables, factor lines, odd and even items, "Add questions 1-5" blocks, totals, stated score ranges, Source and Adapted from blocks, and factor tables (bold marks the kept items), matched to the final items by wording, never by number. A file with several scales is split.
- **Word export** of one scale, as a respondent copy or a researcher copy with the scoring key: native `.docx` with real styles, tables for every grid, header rows that repeat, no split rows, Times New Roman 12.
- **The window**: the library, new scales from the blank formats, import, and one form to confirm and correct every field before saving (the page image view comes with journal articles in phase 3).

- **Tags**: each scale carries the researcher's own tags, with no fixed list; the library can be shown by tag.
- **The scoring self-check**: recode list against wording, a missing recode list, late-added items, repeats in a list, duplicate items, one subscale per item, subscale and score names that disagree, item count against title, score range, instructions against format, leftover sources, and typing slips. Findings warn and never block: each can be fixed in one step where a fix exists, kept as it is, dismissed with a reason the scale keeps, or marked as published with a note on what the original says, kept as printed or corrected with the correction recorded. Open findings print in the researcher copy and the import check report; source notes print under the scale in the researcher copy, so a published measure is never corrected silently.

**Change the context** (by hand) makes a linked variant of a saved scale for a new target, such as communication apprehension with physicians turned into communication apprehension with AIs. The target words are replaced in items, title and instructions; each item is shown beside its original with the changed words marked, to accept, edit or flag; the structure stays locked and the variant structure check runs before saving. The variant is its own scale file linked to its parent, changes in the parent are offered item by item, and the researcher copy names the original and says that reliability and validity do not carry over.

Phase 3 (finding a measure in a journal article) has a first version: PDF text with word positions, OCR by the recogniser built into Windows for scanned pages, reading order across columns and tables, ranked candidate measures, parsing into a scale, the confirm screen with the page and its outlines (drag an outline to read a region again), saved corrections, benchmark answer keys and **Prism Survey Benchmark.bat**. Its accuracy is measured once answer keys exist (see [docs/DECISIONS.md](docs/DECISIONS.md)).

Phase 4 (the study builder) is built: scales from the library, a stored questionnaire or the form are stacked in order with page breaks, transition sentences and demographic questions between them; numbering restarts or runs on; titles show, hide or become neutral labels; sources print under each scale, in a References section, or not at all; and the study exports as a respondent copy and a researcher copy with the scoring key. One scale can also be copied for pasting into Word, and sources can be linked and formatted in APA 7 through Prism Reference Manager. The Prism Hub listing is drafted in [docs/PRISM-HUB-LISTING.md](docs/PRISM-HUB-LISTING.md) and waits for a signed release.

Phase 5 (tags and search) is built: the library pane searches titles, tags, subscales, item text, sources and instructions, forgiving small spelling slips, with phrases in quotation marks and named fields (`tag:`, `author:`, `item:`), and filters by tag, format and point count; a tag can be renamed or merged across the library.

Phase 6 (the Comm Add On) has its first part: the Directory of Communication Related Mental Measures as 509 searchable entries with tidied categories, browsable in the window (Comm Directory), with each measure brought into the library from its article, a stored copy, or typed in, and linked to its entry. References are in APA 7: DOIs for 466 of the 509 and issue numbers added from a one-time Crossref lookup, with the close calls and unmatched references listed for review in [addons/comm-directory/REFERENCE-REVIEW.md](addons/comm-directory/REFERENCE-REVIEW.md). The book's preface is the add-on's introduction ("About this directory"). Still to do: refreshed critiques and newer measures (see [docs/DECISIONS.md](docs/DECISIONS.md)).

Version 0.1.0 is ready to release once it has been tried by hand: what it does and does not do is in [docs/RELEASE-NOTES-0.1.0.md](docs/RELEASE-NOTES-0.1.0.md).

Phase 7 (recommendations) is built for its three rule-based bases: in the study builder, "Suggest measures" offers library scales and Comm Directory entries in the same area as the study's scales, leaves out near-copies of what is already there, weighs their length against the time the researcher sets and their format against the rest of the study, and favours measures used together in the researcher's earlier studies. Every suggestion shows its reasons.

Next: answer keys and tuning for phase 3, the rest of phase 6, and phase 8 (the optional model), which waits on the shared core's local AI runtime (see [CORE-REQUESTS.md](CORE-REQUESTS.md)). The revised phase plan, with search, recommendations and the Comm Add On, is in [docs/SPEC-ADDENDUM.md](docs/SPEC-ADDENDUM.md).

## Trying it

Double-click:

- **Prism Survey Import Check.bat**: choose a folder of stored scales; for each one it writes the `.pscale` file, a respondent copy and a researcher copy in Word, and one report, under `Documents\Prism Survey Import Check`. Set the researcher copies beside your originals.
- **RUN PRISM SURVEY.bat**: starts the window from the source.
- **TEST PRISM SURVEY.bat**: runs every automatic check.
- **Prism Survey Benchmark.bat**: runs detection over the benchmark folders and compares it with the answer keys made in benchmark mode.
- **BUILD PRISM SURVEY INSTALLER.bat**: builds the Windows installer (unsigned) into `app\src-tauri\target\release\bundle\nsis`, with its SHA-256 and the tag it was built from beside it. It builds only from a tagged commit with nothing uncommitted. A signed build is described at the top of `app\BUILD-INSTALLER.ps1`.

You need Rust (1.92 or newer) and Node.js.

## Building and testing

```
cargo test --workspace
```

The command-line tool:

```
cargo run -p prismsurvey-cli -- import "My Scale.doc" --out library
cargo run -p prismsurvey-cli -- export "library\My Scale.pscale" --researcher
cargo run -p prismsurvey-cli -- check-folder "D:\Some folder"
```

The interface (from `app/ui`): `npm ci`, then `npm run build`, or `npm run tauri dev` for the window. To try the screens in a browser against the real engine, run `cargo run -p prismsurvey-cli --features dev-server -- serve` beside `npm run dev`.

## Layout

| Folder | What it holds |
|---|---|
| `crates/prismsurvey-core` | The scale record, `.pscale` files, formats and house instructions. |
| `crates/prismsurvey-import` | Readers for `.txt`, `.docx` and `.doc`, and the stored-scale parser. |
| `crates/prismsurvey-docx` | Word export. |
| `crates/prismsurvey-detect` | Finding measures in journal articles: PDF words, OCR, reading order, candidates, parsing, the benchmark. |
| `crates/prismsurvey-api` | The command layer the window and tests share. |
| `crates/prismsurvey-cli` | The `prismsurvey` command-line tool. |
| `app/src-tauri`, `app/ui` | The desktop window. |
| `tests/fixtures/self-check` | Synthetic scales for the self-check. |
| `addons/comm-directory` | The Comm Add On's data, built from the Directory with `prismsurvey addon-build`. |

Journal articles and stored scale files are test material on the author's computer only. They are never committed, shown in screenshots or shipped.

## License

Apache License 2.0. See [LICENSE](LICENSE) and [NOTICE](NOTICE). Third-party crates and their licenses are listed in [THIRD-PARTY.md](THIRD-PARTY.md).
