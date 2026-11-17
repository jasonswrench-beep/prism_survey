# Specification addendum: tags, search, recommendations and the Comm Add On

Agreed with Jason on 2026-11-17, after the Prism Survey Specification of the same date. All four join version 1. This page is the record until the specification itself is revised.

## 1. Tags

- Every scale carries its own tags, typed by the researcher. There is no fixed list, so each researcher organises measures in the terms of their own discipline.
- Tags are kept as typed; two tags that differ only in capital letters or spacing count as one, and the first spelling is kept.
- Typing a tag suggests the tags already used in the library. Tags can be renamed or merged across the whole library in one step.
- Tags are stored in the `.pscale` file (`tags`), so they travel with a scale handed to a colleague.
- Add-on entries carry their tidied categories as starting tags; a researcher can add their own on top.

## 2. Search

- One search box over the whole library and every installed add-on: titles, short names, item text, subscale names, tags, sources (authors, year, journal), purpose and notes.
- Forgiving matching: word stems, small spelling slips, and phrases in quotation marks.
- Filters: format, point count, item count, reliability where it is known, licence status, tags, add-on or own library.
- Runs on the researcher's machine from a local index; nothing is sent anywhere.

## 3. Recommendations for a longer questionnaire

From a scale or a study in progress, Prism Survey suggests other measures that would work well with it. Every suggestion shows its reasons and makes no claim about the validity of the combination.

- **Construct fit:** measures that complement the ones chosen without measuring the same construct twice, judged from tags, categories, subscale names and wording.
- **Respondent burden:** total items and estimated minutes against a length the researcher sets, and a preference for scales that share a format and anchors, so respondents switch less often.
- **The researcher's own studies:** measures stacked together before in their own questionnaires.
- **Optional model:** suggestions from the local model or a key the researcher supplies, off by default, like the rest of the suite.

## 4. Comm Add On: the Directory of Communication Related Mental Measures

Source: Wrench, Jowi and Goodboy, *Directory of Communication Related Mental Measures*. Jason confirmed on 2026-11-17 that the rights to the whole text are his.

- **What it is:** 509 directory entries in 27 chapters, each with test name, purpose, number of items, format, original sample, reliability, validity, critique, source, categories, and related research for 101 of them. The book describes measures; it does not contain their items.
- **How it works:** an installable add-on with a searchable directory. Each entry has an "import this measure" action that builds the real scale from its journal article (detection, phase 3) or a stored file, and the scale stays linked to its entry.
- **Updating, all four agreed:**
  1. Every source in APA 7, with DOIs and issue numbers found once when the add-on is prepared, then checked.
  2. Categories tidied (variants merged, split names joined) and used as starting tags.
  3. Critiques refreshed to current standards for reliability and validity reporting.
  4. Measures published after 2007 added.
- **Agreed with Jason on 2026-11-17:** refreshed critiques are drafted by a build session for Jason's review, after 0.1.0; newer measures come from a literature search a build session makes, from which Jason chooses; the preface on the history of mental measures ships as the add-on's introduction.
- Finding DOIs means asking a public citation database about each reference. That happens once, when the add-on is prepared, with Jason's go-ahead; the program itself makes no connection.

## Revised build phases

| Phase | Delivers | Status |
|---|---|---|
| 1. Library and formats | As specified, plus the `tags` field | Built 2026-11-17 |
| 2. Self-check | As specified, with source notes (revised specification) | Built 2026-11-17 |
| 3. Detection | As specified | First version 2026-11-17; waiting for answer keys |
| 4. Study builder | As specified | Built 2026-11-17; Prism Hub listing waits for a signed release |
| 5. Tags and search | Tag editing and management, the search index and filters | Built 2026-11-17 |
| 6. Comm Add On | The directory as data, its updates, the add-on install, import from an entry | Data, browser, import, DOIs and issue numbers built 2026-11-17; critiques and newer measures open |
| 7. Recommendations | All four bases, with reasons | Construct fit, burden and past studies built 2026-11-17; the model basis comes with phase 8 |
| Change the context, by hand | The specification's linked variants without a model (open decision 15, moved up by Jason) | Built 2026-11-17 |
| 8. Optional model | As the specification's phase 5, now also serving recommendations and the model mode of change the context | Waits on the shared core's local AI runtime (C8); request in `CORE-REQUESTS.md` |

Version 1 (0.1.0) ships after phase 7; the optional model can follow as a point release, as before.
