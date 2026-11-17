# What only a person can check

## Phase 1

- [ ] Run **Prism Survey Import Check.bat** on the `Jason` folder of the stored scales and on the folder that holds the three templates. Set each researcher copy beside its original in Word and note any field that differs.
  - The build session ran it on 2026-11-17: all eight named scales, the three templates and the other files in those folders imported with no file left unread. The report lists anything the importer did not place.
- [ ] Open a respondent copy of each format in Word, add enough items to cross a page, and check that the box grid and semantic differential header rows repeat and no row splits.
- [ ] Start the window (**RUN PRISM SURVEY.bat**): make a new scale in each format, switch format and points, import a stored scale, save it to the library and export both copies.
- [ ] Keyboard only: Tab through the confirm form, F8 to the next field marked Check, Ctrl+S to save.
- [ ] A screen reader pass on the confirm form.

## Phase 2

- [ ] Read the CHECK lines in the import check report for the eight stored scales and judge each one. The build session's run on 2026-11-17 reported every worked example of the specification and nothing on Ethnocentrism item 5 or Crisis Knowledge item 6. It also reported two things the specification does not list: Risk Communicated item 4 repeats item 2 word for word, and Anal-Perfectionism item 30 is recoded though its wording runs with the kept items.
- [ ] In the window: fix one finding, keep one as it is, dismiss one with a reason, save, reopen the scale and check the decisions are still there.
- [ ] Export a researcher copy with an open finding and check the "Self-check findings still open" section.
- [ ] Source notes: on one scale, mark a finding "As published…" and "Keep as printed", and on another "Correct it", each with the original's wording and a page. Check the marker beside the item, save and reopen, then export a researcher copy and a respondent copy: the notes print under the scale in the researcher copy only.

## Answers of 2026-11-17

- [ ] Import the Intercultural Perspective Taking Scale: one scale of 20 items with four subscales named as the file's scoring lists them, and each perspective's prompt above items 6, 11 and 16. Export a respondent copy and check the prompts sit above their blocks.
- [ ] Import the Temperament Testing Scale: the bands and the reading guide are under "How to read the scores" and print in the researcher copy under "Reading the scores".
- [ ] In Comm Directory, open "About this directory" and read the preface as it ships.

## Change the context

- [ ] Open a saved scale of yours, choose "Change the context…", and turn it to a new target (physicians to AIs, or one of your own). Check the replaced wording on the side-by-side screen, edit one item, flag one, make the variant, check the self-check, and save it.
- [ ] In the library, check that the variant names its parent. Change an item in the parent and save it, then open the variant: the change is offered, carried only when you choose.
- [ ] Export the variant's researcher copy and check the Adapted variant section and the item map.

## Phase 3

- [ ] Make the answer keys: in the window, tick **Benchmark mode**, then for each article in the three article folders use **Find in an article…**, pick the right candidate (or outline the items), correct every field against the page, and save. Each save writes the article's key.
- [ ] Run **Prism Survey Benchmark.bat** and read the totals against the targets.
- [ ] Try a phone photo of a printed measure (.jpg) with **Find in an article…**.
- [ ] Check that text recognition works on this computer's Windows language (Settings, Time and language, Language: the OCR language pack).

## Phase 4

- [ ] Make a study of four of your scales in the window (New study, then Add to study beside each), try each option, and export both copies. The build session made one from Homophobia, Crisis Knowledge, Physical Aggression and Sexual Communication on 2026-11-17: Word opened both copies (6 and 8 pages) with each scale on its own page and the References section at the end.
- [ ] Paste a scale with Copy for Word into an open Word document.
- [ ] Turn on Prism Reference Manager's local connections, then use Link and format with Prism Reference Manager on a scale with a source, and approve the pairing in that window.

## Phase 5

- [ ] With a few scales saved, try the search box: a word, a misspelt word, a "quoted phrase", `tag:` and `author:`; then the Tag, Format and Points filters.
- [ ] Choose a tag in the filter and use Rename or merge this tag.

## Phase 6

- [ ] Open Comm Directory, search for a few measures you know, and read their entries against the book.
- [ ] Bring one in each way: find it in its article, import a stored copy, and type it into a blank scale; check the source and tags it brings.
- [ ] Look over the tidied categories (the chapter filter and each entry's categories) and say if any should be named differently.

## Release 0.1.0

The build session built the setup on 2026-11-17, installed it silently into a test folder, opened the window (the interface drew and Comm Directory showed its 509 entries), and uninstalled it. What only a person can check:

- [ ] Run the setup (`app\src-tauri\target\release\bundle\nsis\Prism Survey_0.1.0_x64-setup.exe`) on this computer and, if you can, on a second one: it should install without administrator rights, put Prism Survey in the Start menu, and open the window. Windows warns about an unsigned setup until a signed build is made.
- [ ] In the installed program: open Comm Directory (the add-on ships inside the setup), make a scale, save it, and export it to Word.
- [ ] Double-click a `.pscale` file: it should open in Prism Survey.
- [ ] Uninstall from Windows Settings, Apps: the program goes and your library folder stays.
