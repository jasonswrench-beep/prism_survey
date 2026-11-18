# Prism Survey Icon List

Nov 17, 2026 · draft for Jason's review

Prism Survey needs **59 new icons** in 11 groups, plus **31 icons it reuses from PrismQDA** unchanged, so the two tools read as one family. The new icons come in two batches: 39 in Batch 1 and 20 in Batch 2. Batch 1 covers the main window: the toolbar, the scale form, the self-check and the file glyphs. Batch 2 covers the study builder, Comm Directory, change the context and licence badges.

Every button keeps its words. The icons sit beside the labels and do not replace them, because most of these commands are rare enough that a picture alone would not be clear. The exceptions are the small row buttons (move, delete, split and so on), which show the icon alone with the label as a tooltip and for screen readers. This is how they work today.

## Style brief

This is PrismQDA's standard (`design/icons/Icon_Standards_v0.1.md` in that repository), so new and reused icons match.

| Rule | Specification |
| --- | --- |
| Grid | 24 x 24 viewBox, 2 px padding (20 x 20 live area) |
| Stroke | 1.75 px, round caps, round joins, no fill |
| Color | stroke="currentColor" only; the app supplies color, including status colors |
| Corners | 2 px radius on rectangles; circles and arcs drawn as paths |
| Detail | Must read at 16 px: no more than 3 or 4 strokes of detail, no letters or numbers unless a row says so |
| Family motif | None on interface icons. The prism belongs to the app icon and the file glyphs only |
| Direction | Draw one chevron or arrow; other directions come from rotation in code |
| Originality | New drawings, not traced or copied from other software or icon sets |
| File format | Hand-written SVG code, one file per icon, paths only, no embedded images, metadata, text or transforms |
| Naming | Lowercase file names exactly as listed (group prefix, then name) |

Prompt to give with each batch:

> Draw the following icons for a desktop research application called Prism Survey as hand-written SVG code, one code block per icon, using the style rules below. They join an existing set of outline icons with the same rules, so keep the same stroke weight, corner radius and level of detail. Do not copy icons from other software. \[paste the style rules table and the rows for this batch\]

Ask for SVG code rather than images. Image generators produce pictures of icons, not usable vector files.

## Brand and files (2)

The app icon is Jason's clipboard checklist in the prism gradients (`assets/PrismSurvey-Icon.svg`, decision 11) and stays as it is. The two file glyphs are the only other full-color pieces. They follow PrismQDA's `file-project.svg`: a sheet with the app's mark small in one corner.

| Icon | File name | What it shows | Used for | Batch |
| --- | --- | --- | --- | --- |
| Scale file | file-scale.svg | Sheet with three short item lines and the clipboard mark in the corner | `.pscale` files in Windows | 1 |
| Study file | file-study.svg | Two stacked sheets with the clipboard mark in the corner | `.pstudy` files in Windows | 1 |

## The three formats (3)

Used on the three "New" buttons, in the library list beside each scale, and in the format picker. They need to tell the formats apart at 16 px.

| Icon | File name | What it shows | What the button does | Batch |
| --- | --- | --- | --- | --- |
| Likert blank-line | fmt-blank-line.svg | Three text lines, each starting with a short underline (the blank) | Starts a new scale with a blank to write a number in beside each item | 1 |
| Likert box grid | fmt-box-grid.svg | A row of four small boxes under a short line, one box with an x | Starts a new scale where respondents mark one box per row | 1 |
| Semantic differential | fmt-semantic.svg | Two short end marks with a row of four dots between them, one dot circled | Starts a new scale of opposite adjectives with points to circle between them | 1 |

## Toolbar (11)

| Icon | File name | What it shows | What the button does | Batch |
| --- | --- | --- | --- | --- |
| Find in an article | cmd-find-article.svg | Sheet with list lines and a magnifying glass over them | Opens a journal article (PDF or a photo of a page), finds the measures in it and parses the one chosen | 1 |
| Import stored scale | (reuse cmd-import) | | Reads a stored scale file (.doc, .docx, .txt) into the confirm form | |
| Comm Directory | view-directory.svg | Open book with two list lines on the right page | Opens the Directory of Communication Related Mental Measures | 1 |
| Open | cmd-open.svg | Open folder with its front flap tilted forward | Opens a saved scale | 1 |
| Save | (reuse ui-save) | | Saves the scale; for a new one, "Confirm and save" asks where | |
| Save as | cmd-save-as.svg | The save glyph with a small pencil at its lower right | Saves the scale under a new name or place | 1 |
| Change the context | cmd-variant.svg | Two sheets, the second linked to the first by a branching arrow | Makes a linked variant of the scale for a new target, with its structure locked | 1 |
| Export respondent copy | exp-respondent.svg | Sheet with item lines and a small person at its lower right | Writes the questionnaire as respondents see it, to Word | 1 |
| Export researcher copy | exp-researcher.svg | Sheet with item lines and a small key at its lower right | Writes the researcher copy with the scoring key, to Word | 1 |
| Copy for Word | (reuse ui-copy) | | Copies the scale for pasting into an open Word document | |
| Benchmark mode | cmd-benchmark.svg | Target of two rings with a check mark at the center | A switch: confirmed articles are saved as answer keys for the accuracy benchmark | 1 |
| New study | study-new.svg | Two stacked sheets with a plus at the lower right | Starts a new study (a questionnaire of several scales) | 1 |
| Open study | (reuse cmd-open) | | Opens a saved study | |
| Show the study | view-study.svg | Three stacked sheets, slightly offset | Goes to the study builder, or back to it from a scale | 1 |
| Save study | (reuse ui-save) | | Saves the study | |
| Export study copies | (reuse exp-respondent, exp-researcher) | | Write the whole study's two copies to Word | |
| Study length | ui-clock.svg | Clock face with two hands | Beside the study's item count and estimated minutes | 1 |

## Library pane (3)

| Icon | File name | What it shows | What it does | Batch |
| --- | --- | --- | --- | --- |
| Change folder | (reuse src-folder) | | Picks the library folder | |
| Search | (reuse cmd-search) | | Searches the library | |
| Filter | (reuse ui-filter) | | Tag, format and point-count filters | |
| Rename or merge tag | tag-rename.svg | Tag with a small pencil at its lower right | Renames a tag on every scale, or merges it into another | 1 |
| Add to study | study-add.svg | Stacked sheets with an arrow pointing in from the left | Adds this library scale to the open study | 1 |
| Variant of | lib-variant.svg | Small branching line: one stem, one branch | Marks a scale in the list that is a variant of another | 1 |
| Remove tag | (reuse ui-close) | | Removes a tag from the scale | |

## The scale form (10)

| Icon | File name | What it shows | What it does | Batch |
| --- | --- | --- | --- | --- |
| Check this field | (reuse st-warning) | | Marks a field the importer was unsure of; F8 jumps to the next | |
| Use house instructions | instr-house.svg | Sheet with two text lines and a short blank slot in the middle line | Rewrites the instructions from the template for this format and point count | 1 |
| Move item up / down | (reuse ui-chevron-down, rotated for up) | | Moves an item in the list | |
| Split item | item-split.svg | One line broken into two shorter lines, one above the other, with a small gap | Splits an item in two at the cursor | 1 |
| Merge item | item-merge.svg | Two short lines joining into one longer line | Joins an item with the next one | 1 |
| Delete item | (reuse ui-delete) | | Deletes an item | |
| Text above | item-lead-in.svg | A long line with a short line above it and a small down arrow between them | Adds a prompt printed above the item, such as a new perspective for the block it starts | 1 |
| Add an item | (reuse ui-plus) | | Adds an item at the end | |
| Remove empty lines | item-trim.svg | Three lines, the last two dashed, with a small eraser | Removes the empty template lines at the end | 1 |
| Reverse scored | item-reversed.svg | Two arrows in a loop, one pointing each way | Shows an item marked reverse scored, beside its scoring choice | 1 |
| Reverse suggestion | item-reverse-suggested.svg | The reversed loop drawn with one dashed arrow | A reverse-scoring suggestion waiting to be accepted or rejected | 1 |
| Accept / Reject a suggestion | (reuse ui-check, ui-close) | | Accepts or rejects a reverse suggestion | |
| Changed from the source | item-changed.svg | Text line with a small dot above its end | Marks an item edited away from the source's wording | 1 |
| Add a subscale | sub-add.svg | A bracket grouping three short lines, with a plus beside it | Adds a subscale | 1 |
| Remove a subscale | (reuse ui-delete) | | Removes a subscale | |
| Link with Prism Reference Manager | (Prism Reference Manager's own app mark, small) | | Links the sources to Prism Reference Manager and formats them in APA 7 | |
| Clear the list | item-clear.svg | Two lines with a diagonal stroke through them | Clears the lines the importer did not understand | 1 |

## The self-check (7)

| Icon | File name | What it shows | What it does | Batch |
| --- | --- | --- | --- | --- |
| Self-check | chk-selfcheck.svg | Clipboard with one check mark and one line | Heads the self-check panel and its count of open findings | 1 |
| Check (a finding) | (reuse st-warning) | | A finding that is probably a scoring error | |
| Note (a finding) | (reuse st-info) | | A finding worth knowing that is not an error in itself | |
| Fix | chk-fix.svg | Check mark with a small spark at its upper right | Makes the change the finding suggests, in one step | 1 |
| Keep as it is | (reuse ui-check) | | Accepts the finding and keeps the scale as it is | |
| Does not apply | chk-dismiss.svg | Circle with a diagonal stroke | Dismisses the finding, with a reason the scale keeps | 1 |
| As published | chk-published.svg | Open book with a small bookmark ribbon | Marks a finding that is in the original publication, with a note on what it says; also the marker beside the item | 1 |
| Keep as printed | (reuse chk-published) | | Keeps the published wording and records the note | |
| Correct it | chk-corrected.svg | Open book with a small pencil across the right page | Makes the fix and records it as a change from the published measure; also the "Corrected from the source" marker | 1 |
| Reopen | chk-reopen.svg | Circular arrow around a short line | Reopens a decided finding | 1 |
| Variant structure held | chk-locked.svg | Padlock with its shackle closed | Shows that a variant's structure is locked, beside the locked fields | 1 |

## Finding a measure in an article (3)

| Icon | File name | What it shows | What it does | Batch |
| --- | --- | --- | --- | --- |
| Candidate measure | art-candidate.svg | Sheet with a bracket around three list lines | Each measure found in the article, offered to choose | 1 |
| Outline it myself | art-outline.svg | Sheet with a dashed rectangle drawn over its lines | Opens the article to draw an outline around the items by hand | 1 |
| Previous / next page | (reuse ui-chevron-right, rotated) | | Moves through the article's pages | |
| Sample items only | art-incomplete.svg | Three list lines, the last one dashed | Marks a candidate whose article prints only sample items | 1 |

## Study builder (8)

| Icon | File name | What it shows | What it does | Batch |
| --- | --- | --- | --- | --- |
| Edit | (reuse ui-edit) | | Opens a scale of the study on the form | |
| Move up / down, Remove | (reuse ui-chevron-down, ui-delete) | | Reorders or removes a part of the study | |
| Add the scale on the form | (reuse study-add) | | Adds the scale being edited to the study | |
| Import a questionnaire | (reuse cmd-import) | | Reads a stored questionnaire and adds every scale in it | |
| Page break | study-page-break.svg | Bottom edge of one sheet over the top edge of the next, with a dashed line between | Adds a page break between scales | 2 |
| Transition sentence | study-transition.svg | Two blocks with one short line between them | Adds a sentence between scales | 2 |
| Demographic questions | study-demographics.svg | Person with two short lines beside it | Adds a block of demographic questions the researcher types | 2 |
| Bring up to date | study-refresh.svg | Two stacked sheets with a circular arrow | Updates the study's copies of scales from the library (never silently) | 2 |
| Suggest measures | study-suggest.svg | Compass with its needle | Suggests measures that fit the study, each with its reasons | 2 |
| Neutral label | study-label.svg | Tag shape with a short line inside | Shows a scale's neutral label ("Section 3") in place of its title | 2 |
| Numbering runs on | study-numbering.svg | Two short columns of dots joined by a curved arrow | Numbering that runs on through the study rather than restarting | 2 |
| Sources at the end | study-references.svg | Sheet with three lines, each with a hanging first line | Gathers sources into a References section | 2 |

## Comm Directory (3)

| Icon | File name | What it shows | What it does | Batch |
| --- | --- | --- | --- | --- |
| About this directory | (reuse set-about) | | Shows the book's preface | |
| Find it in the article | (reuse cmd-find-article) | | Brings the measure in from its journal article | |
| Import a stored copy | (reuse cmd-import) | | Brings the measure in from a stored file | |
| Type it into a blank scale | (reuse fmt-blank-line) | | Starts a blank scale linked to the entry | |
| Copy the reference | (reuse ui-copy) | | Copies the APA 7 reference | |
| Show in the Comm Directory | (reuse view-directory) | | Opens a suggested measure's entry | |
| Reference checked | dir-ref-checked.svg | Short reference lines with a check mark | The reference matched Crossref and has its DOI | 2 |
| Reference to check | dir-ref-check.svg | Short reference lines with a question mark (allowed) | The reference needs a person to look at it | 2 |
| Chapter | dir-chapter.svg | Closed book seen from the side | The chapter filter | 2 |

## Change the context (5)

| Icon | File name | What it shows | What it does | Batch |
| --- | --- | --- | --- | --- |
| Replace the target words | var-replace.svg | Two short words with a curved arrow from the first to the second | Replaces the old target with the new one in items, title and instructions | 2 |
| Add wording to replace | (reuse ui-plus) | | Adds another pair of words to replace | |
| Accept | (reuse ui-check) | | Accepts an item's new wording | |
| Accept the rest | var-accept-all.svg | Two check marks, one behind the other | Accepts every item not yet decided, as shown | 2 |
| Flag | var-flag.svg | Flag on a short pole | Flags an item that cannot be reworded without changing what it asks | 2 |
| Make the variant | (reuse cmd-variant) | | Builds the variant for the confirm screen | |
| Carry into the variant | var-carry.svg | Arrow from a sheet on the left to a sheet on the right | Carries one change from the original into the variant | 2 |
| Leave it | var-leave.svg | Arrow from a sheet that stops at a short bar | Leaves a change in the original out of the variant, for good | 2 |
| Adapted variant | (reuse lib-variant) | | Heads the variant panel on the confirm screen | |

## Licence badges (4)

Shown beside each scale's licence, and in the library list when a scale is not open.

| Icon | File name | What it shows | What it means | Batch |
| --- | --- | --- | --- | --- |
| Open | lic-open.svg | Padlock with its shackle open | Open licence that allows redistribution | 2 |
| Permissioned | lic-permissioned.svg | Sheet with a round seal at its lower right | Written permission from the rights holder is on file | 2 |
| Local only | lic-local.svg | Monitor with a small house on its screen | Stays on this computer; never shared or shipped | 2 |
| Unknown | lic-unknown.svg | Circle with a question mark (allowed) | Licence not known; shows on the scale and in the researcher copy | 2 |

## Status (reused)

The status line and saving states reuse PrismQDA's `st-saved`, `st-saving`, `st-save-failed`, `st-error`, `st-success` and `st-warning` unchanged.

## Reused from PrismQDA

31 files, copied unchanged from `prismqda/assets/icons/regular`: cmd-import, cmd-search, set-about, src-folder, st-error, st-info, st-save-failed, st-saved, st-saving, st-success, st-warning, ui-check, ui-chevron-down, ui-chevron-right, ui-close, ui-copy, ui-delete, ui-edit, ui-filter, ui-plus, ui-save, and (for the menus and panes as they come) ui-more, ui-help, ui-refresh, ui-external, ui-eye, ui-sort, ui-drag, ui-pin, ui-pane-collapse and ui-command.

## How they reach the window

As in PrismQDA (its decisions 40 and 41): the SVG files live in `assets/icons`, a generator turns them into a TypeScript module with a strict check of the style rules, and the window draws them as ordinary SVG elements that take the text color, so dark mode and Windows high contrast work without extra rules.
