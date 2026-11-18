# The command layer

The window sends every request through one Tauri command, `call`, with a command name and JSON arguments; [crates/prismsurvey-api](../crates/prismsurvey-api/src/lib.rs) answers. The tests call the same function. The window holds the scale being edited and sends it with each command. Nothing is written to disk except by `scale.save`, `scale.export` and `settings.set`.

| Command | Arguments | Answer |
|---|---|---|
| `scale.new` | `format` | A blank scale from the template (40 lines, or 30 for a semantic differential), with house instructions. |
| `scale.import` | `path` (.doc, .docx, .txt) | One report per scale in the file: `scale`, `file`, `part`, `isTemplate`, `notes`. |
| `scale.open` | `path` (.pscale) | The scale. |
| `scale.save` | `path`, `scale`, `summary` | The scale as saved, with its new history entry and computed range. |
| `scale.setFormat` | `scale`, `format` | `{ scale, outcome }`; `outcome.instructionsRewritten`, or `outcome.findings` for verbatim or edited instructions. |
| `scale.setPoints` | `scale`, `points` (2 to 11) | `{ scale, outcome }` |
| `scale.setAnchors` | `scale`, `anchorSet`, `labelMode` | `{ scale, outcome }` |
| `scale.setLabels` | `scale`, `labels` (lowest first) | `{ scale, outcome }`: custom labels on every point when they fit the point count, else the first and last as the ends |
| `anchors.common` | | the five-point label sets the form offers, lowest first |
| `scale.editInstructions` | `scale`, `text` | The scale, its instructions now marked edited. |
| `scale.useHouseInstructions` | `scale`, `stem` (optional) | The scale with house instructions for its format. |
| `scale.parseSelector` | `text` ("1, 3, 5", "1-5", "odd", "even") | An item selector. |
| `scale.check` | `scale` | The live check: computed range, instruction findings, forced choice, title count mismatch, and `findings`, the full self-check with the scale's decisions applied. |
| `scale.decideFinding` | `scale`, `key`, `status` (`accepted`, `dismissed`, or null to reopen), `reason` | The scale with the decision recorded. A dismissal needs a reason. |
| `scale.applyFix` | `scale`, `fix` (one of a finding's `fixes`) | The scale with the fix made. |
| `scale.keepAsPublished` | `scale`, `key`, `original` (what the original publication says), `page` | The scale unchanged but for a source note kept as published; the finding stays visible with status `published`. |
| `scale.correctFromSource` | `scale`, `key`, `fix` (one of the finding's `fixes`), `original`, `page` | The scale with the fix made and the correction recorded as a change from the source. |
| `directory.introduction` | none | `{title, blocks}`: the Directory's preface and its references as `heading`, `paragraph`, `item` and `reference` blocks. |
| `variant.draft` | `path` (a saved scale), `request` (`oldTarget`, `newTarget`, optional `context`, `replacements`: `[{from, to}]`) | `{scale, rows, titleChanged, instructionsChanged}`: the variant drafted by replacing the target words, unsaved, and each item's `original` beside its `proposed` wording, with `found` saying whether a target word was replaced. |
| `variant.finish` | `scale` (the draft), `rows` (`[{number, text, left, right, flagged}]`, one per item; a flagged item sends its original wording and the reason) | The variant with each item's decision, ready for the confirm screen and the variant structure check. |
| `variant.parentChanges` | `scale` (a variant), optional `folder` | `{found, parentPath, parentTitle, parentVersion, changes}`: the parent's changes since the variant was made (`itemText`, `reverse`, `sourceNote`, `structure`, `itemCount`), leaving out declined ones. Nothing is applied. |
| `variant.carry` | `scale`, `parentPath`, `change` | The variant with that one change carried from the parent. |
| `variant.decline` | `scale`, `key` | The variant with the change marked as not to be offered again. |
| `scale.removeSourceNote` | `scale`, `key` | The scale without that source note. Removing a note kept as published reopens the finding; a correction is not undone. |
| `scale.export` | `scale`, `path`, `copy` (`respondent`, `researcher`) | `{ notes }`, such as the permission note for a published measure. |
| `library.list` | `folder` (optional) | The `.pscale` files in the library folder, with their tags. |
| `library.search` | `query`, `filters` (`format`, `points`, `minItems`, `maxItems`, `licence`, `tag`, `collection`), `folder` (optional) | `{ hits, total }`: ranked matches with a snippet of the text that matched. |
| `library.tags` | `folder` (optional) | Every tag in the library with how many scales carry it, most used first. |
| `library.renameTag` | `from`, `to`, `folder` (optional) | Renames or merges a tag on every scale that carries it; `{ changed }` lists the files saved. |
| `article.detect` | `path` (.pdf, .png, .jpg) | `{ detection, pages }`: candidates best first, each parsed into a scale, and each page's size and how it was read. |
| `article.page` | `path`, `page`, `width` | `{ dataUrl }`: a picture of the page. |
| `article.parseRegion` | `path`, `page`, `region` (points) | The scale read from inside the outline. |
| `article.confirm` | `path`, `firstGuess`, `confirmed`, `benchmark` | Keeps the correction example; in benchmark mode saves the answer key. |
| `study.new` / `study.open` / `study.save` | `title` / `path` / `path`, `study`, `summary` | A study. |
| `study.addScale` | `study`, `path` | The study with a copy of the library scale added. |
| `study.refresh` | `study` | `{ study, changed }`: scales brought up to date from their library files. |
| `study.length` | `study` | `{ items, minutes }`. |
| `study.recommend` | `study`, `settings` (`{ maxMinutes, limit }`, optional) | `{ recommendations }`: each with `key` (a library path, or `dir:` and a Comm Directory id), `title`, `collection`, `score` and `reasons`. |
| `study.export` | `study`, `path`, `copy` | `{ notes }`. |
| `scale.clipboard` | `scale`, `copy` | `{ html, text }` for pasting into Word. |
| `refman.status` / `refman.pair` / `refman.pairStatus` / `refman.forget` | / / `id` / | Prism Reference Manager: whether it answers and is paired; pairing. |
| `refman.cite` | `scale` | `{ scale, notes }`: source and adapted-from work linked and formatted in APA 7. |
| `directory.info` | | `{ installed, title, authors, entries, chapters, categories }` for the Comm Add On. |
| `directory.search` | `query`, `chapter`, `filters` | `{ hits, total }` over the entries. |
| `directory.entry` | `id` | One entry. |
| `directory.link` | `scale`, `id` | The scale linked to the entry: source, tags and (when untitled) title from it. |
| `settings.get` / `settings.set` | `settings` | `{ libraryFolder }`, kept in `%APPDATA%\Prism Survey\settings.json`. |
