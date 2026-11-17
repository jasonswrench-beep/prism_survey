# Prism Survey 0.1.0

The first release. Prism Survey turns a published measure into a correctly scored scale for a Word questionnaire: it reads the measure from a journal article or a stored scale file, lets you confirm every field, checks the scoring, and writes the scale or a whole study to Word with its scoring key.

This is early software with very little use so far. Everything below has been run by automatic checks and by the build session on the author's computer; what still needs a person is listed in [TESTING-TODO.md](../TESTING-TODO.md). Keep copies of your scale files.

## What it does

- **A library of scales** in plain files (`.pscale`, one per scale) in a folder you choose, with free-form tags and a search that forgives small spelling slips, takes "quoted phrases" and named fields (`tag:`, `author:`, `item:`), and filters by tag, format and point count.
- **Three formats:** Likert items with a blank line, a Likert box grid, and semantic differential, with house instructions you can edit.
- **Import of stored scales** from Word (`.doc`, including Word 6 and 95 files, and `.docx`) and plain text, including files that hold several scales.
- **Finding a measure in a journal article** (PDF, or a photo of a printed page): the program lists the candidate measures, parses the one you choose, and shows the page beside the confirm form. Scanned pages are read by the text recogniser built into Windows. Every field is confirmed by you before anything enters the library, and your corrections are kept on your computer to improve later reading.
- **The scoring self-check:** reverse-scored items that look wrongly marked, recode lists that do not match the items, repeated items, score ranges that do not add up. Each finding is fixed, kept or dismissed with a reason, and open findings print in the researcher copy. A finding that is in the original publication can be marked as published, with what the original says and its page: the scale is kept as printed, or corrected with the correction recorded, and the note prints in the scoring key.
- **Change the context:** a linked variant of a saved scale reworded for a new target (for example, from physicians to AIs). The target words are replaced for you; you accept, edit or flag each item beside its original, the scale's structure stays locked, and a check confirms it held. Changes in the original are offered to the variant one at a time, never applied on their own. Rewording by a model comes later.
- **Studies:** stack scales with page breaks, transition sentences and demographic questions; numbering restarts or runs on; scale titles show, hide or become neutral labels; sources print under each scale, in a References section, or not at all. A study exports as a respondent copy and a researcher copy with the scoring key. One scale can also be copied for pasting into an open Word document.
- **Suggestions for a study:** measures from your library and the Comm Directory in the same area as the study's scales, leaving out near-copies of what is already there, weighed against the time you set and the study's format, and favouring measures you have used together before. Each suggestion shows its reasons.
- **Stored files with more in them:** the same items asked again under new prompts (a perspective-taking exercise) come in as one scale with a subscale per prompt, each prompt printed above its block; a source's guide to reading the scores (bands such as "3-5 Very calm") is kept and printed in the researcher copy.
- **The Comm Add On:** the *Directory of Communication Related Mental Measures* (Wrench, Jowi and Goodboy) as 509 searchable entries with the book's preface as its introduction, with references in APA 7 (DOIs for 466 and issue numbers added) and each measure brought into the library from its article, a stored copy, or typed in.
- **Prism Reference Manager:** when it is running with its local connections turned on, sources can be linked and formatted in APA 7 through it.

## What it does not do yet

- **The optional model** is not in this release. Every step works without one; model help for detection, the self-check and suggestions waits on the suite's shared local AI runtime.
- **Detection accuracy has not been measured.** The benchmark is built and runs, but its answer keys are still to be made.
- **Refreshed critiques and newer measures** for the Comm Directory are still to come; each entry shows the book's critique.
- Version 1 prepares instruments. It does not host surveys, collect or score responses, or share a library online.

## Installing

Windows 11, 64-bit. Run `Prism Survey_0.1.0_x64-setup.exe`; it installs for your Windows account only and needs no administrator rights. A `.sha256.txt` file beside the setup holds its SHA-256, so you can check the download. The program connects to nothing on its own; the one exception is Prism Reference Manager on your own computer, when you use it.
