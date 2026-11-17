# The .pscale file

Every scale is one plain JSON file with the extension `.pscale`, kept in a library folder the person chooses (by default `Documents\Prism Survey Library`). A scale can be backed up, copied or handed to a colleague as that one file. The authority is the Rust model in [crates/prismsurvey-core/src/model.rs](../crates/prismsurvey-core/src/model.rs); this page explains it.

## Version

Every file starts with `"pscaleVersion": 1`. A file from a newer version of Prism Survey is refused with a message saying so, never half read. When a field changes meaning, the version goes up and `pscale::migrate` brings older files forward when they open. Files are written beside their final name and then moved into place, so a failed save never leaves half a file.

## Fields

| Field | What it holds |
|---|---|
| `pscaleVersion` | The format version (1). |
| `id` | A random identifier that stays with the scale across saves and copies. |
| `title`, `shortName` | Display title and an optional short name. |
| `countSuffix` | When true, a number in the title such as the 16 in "Homophobia-16 Scale" is kept equal to the item count on save. |
| `tags` | The researcher's own tags, in the order added. No fixed list; tags that differ only in capitals or spacing count as one. |
| `format` | `likertBlankLine`, `likertBoxGrid` or `semanticDifferential`. |
| `response` | `points` (2 to 11), `anchorSet` (`agreement`, `frequency`, `custom`), `labelMode` (`every`, `endsOnly`), `labels` (one per point, `null` for an unlabelled point), `abbreviations` (box grid header), `values` (what each point scores). |
| `instructions` | `text`, `origin` (`house`: rewritten when the format or points change; `verbatim`: kept word for word from a source; `edited`: changed by hand) and the optional `houseStem` (the opening sentence of house instructions). |
| `items` | `number`, verbatim `text`, `left` and `right` for a semantic differential, `reverse` (`state`: `notReversed`, `suggested`, `confirmed`; `reason`), `sourceText`, the wording as the source had it, so an edited item shows as changed from the source, and `leadIn`, text printed just above the item as the prompt for the block it starts. |
| `recode` | `rule`: `{ "kind": "reflect" }` (lowest plus highest, minus the answer) or `{ "kind": "table", "pairs": [[1, 5], ...] }`; `stated`: the source's recode line with its numbers as written, repeats kept. |
| `subscales` | `name`, `items` (`{ "kind": "list", "numbers": [...] }`, `odd`, `even`, or `{ "kind": "block", "first": 1, "last": 5 }`), and `otherNames` the source also uses. |
| `scoring` | `subscaleMethod` and `totalMethod` (`sum`, `mean`), `totalFrom` (`items`, `subscales`), `hasTotal`, `totalName`, `otherTotalNames`, `stated` (the source's scoring sentences) and `interpretation` (how the source says to read the scores, line by line; printed in the researcher copy). |
| `scoreRange` | `stated` (as the source gives it), `statedText`, and `computed` (refreshed on every save). |
| `source`, `adaptedFrom` | `apa` (APA 7 text, kept so the scale opens without Prism Reference Manager), `doi`, `referenceManagerId` (once linked). |
| `sourcePlacement` | `underScale`, `references` or `none`. |
| `licence` | `status` (`open`, `permissioned`, `localOnly`, `unknown`) and `note`. Imported and typed scales start as `localOnly`. |
| `development` | The development item pool: `factors` (column names), `factorLabels`, `items` (`number`, `text`, `loadings`, `markedKept`, `finalNumber`, `matchScore`) and `notes`. Development items are matched to final items by wording, never by number. |
| `provenance` | For each parsed field: `field`, `file`, `page`, `line` (the paragraph), `region` (on a page image, phase 3), `confidence` (0 to 1) and `rule`. |
| `history` | Each save: `version`, `savedAt` (UTC) and `summary`. |
| `unparsed` | Lines from an imported file that no rule understood, kept for the confirm form until cleared. |
| `checkDecisions` | Decisions on self-check findings: `key` (the finding's key), `status` (`accepted`: kept as it is; `dismissed`: does not apply), `reason`, `decidedAt`. |
| `sourceNotes` | Findings that sit in the original publication: `key` (the finding's key), `subject` (the check or field), `items`, `original` (what the original says), `page`, `handling` (`kept`: kept as published; `corrected`: this copy corrects it), `change` (for a correction, what this copy does instead) and `notedAt`. A note prints under the scale in the researcher copy; the respondent copy is unchanged. |
| `variantOf` | For a variant reworded for a new target: `parentId`, `parentTitle`, `parentVersion` (the parent's saved version it branched from or last took a change from), `oldTarget`, `newTarget`, `context`, `replacements` (further wording replaced: `from`, `to`), `itemMap` (each item's `number` beside its `originalNumber`, `originalText` and, when it could not be reworded, `flagged` with the reason), `locked` (the parent's structure when the variant was made: `itemCount`, `response`, `reversed`, `subscales`, `recode`, `scoring`), and `declined` (parent changes the researcher chose not to carry). |

## Room for later phases

The record already carries what the codebook export (`.pstat`) and data collection will need: item numbers and text, the points with their labels and values (value labels), the recode rule, subscale definitions and the score range. Missing-value codes and new item types will be added fields with a version step.
