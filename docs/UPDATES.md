# Updates: Help, Check for updates

Prism Survey finds out about a newer version only when a person asks, or, if they turned it on, at most once a day. The
design is PrismStatistics' (its `docs/UPDATES.md`), so the Prism tools behave alike. This page says what it does, why a
download can be trusted, and what the maintainer does so a release is offered.

## What a person sees

- **Help**, **Check for updates**, then **Check now**, asks the newest full release on the public repository whether it is
  a newer version: `https://github.com/jasonswrench-beep/prism_survey/releases/latest/download/latest.json`. GitHub's
  `latest` skips pre-releases, so only full releases are offered. The request sends only what any web request sends (the
  program's version and the kind of computer), nothing about the person's scales or questionnaires.
- **Check for updates once a day**, off by default, makes the same check at most once a day, at start-up or hourly while
  the program is open. It only announces a newer version (on the status line, and a dot on the Help tab); nothing is
  downloaded.
- If a newer version exists, the page shows its number, date and notes. **Download and install** is a second step: the
  installer is downloaded, **its signature is checked against the key built into the program** (a file that doesn't
  match is refused and nothing is installed), and the installer runs for the current user. The window asks first when
  work is not saved.
- **Return to the previous version** reads `previous.json` on the same release, accepts only a version older than the
  running one, checks its signature the same way, and installs only with the person's go-ahead.
- A copy built before the key was set up says it is "not set up" and connects to nothing. In a browser (the development
  bridge) the page says updates can only be checked in the installed program.

## How it is built

- `app/src-tauri/src/updates.rs`: Tauri's updater plugin, used only from the window's own commands (`update.state`,
  `update.check`, `update.install`, `update.rollbackCheck`, `update.rollback`), handled in `main.rs` before the engine,
  which makes no network connection for them. `ENDPOINT`, `ROLLBACK_ENDPOINT` and `PUBKEY` are constants there; while
  `PUBKEY` is empty nothing is set up. Debug builds only: `PRISMSURVEY_UPDATE_ENDPOINT`, `PRISMSURVEY_ROLLBACK_ENDPOINT`
  and `PRISMSURVEY_UPDATE_PUBKEY` point the program at a test feed.
- The page is `app/ui/src/HelpView.tsx`; the daily check lives in `App.tsx`, with its setting in the window's own storage
  (`app/ui/src/updates.ts`).
- Checks time out after 20 seconds; a failure is shown in words and changes nothing.
- Two signatures, two jobs: the code signature tells Windows who made the installer, and is made during the build; the
  update signature, made **after** it over the exact bytes people download, tells Prism Survey the file is the
  maintainer's.
- Tests (`cargo test` in `app/src-tauri`): with no key or address nothing is asked; newer, same and older versions; a
  rollback offers only an older version; a missing feed fails in words; a download passes only when it matches its
  signature (a changed byte, and another key's signature, are refused). The test key pair is a test fixture, never a
  release key.

## Making the key (once, the maintainer)

Double-click `MAKE PRISM SURVEY UPDATE KEY.bat`. It asks for a password (twice), makes `.prismsurvey-update.key` and its
`.pub` in your profile folder, outside the repository, and puts the public half into `PUBKEY`. Commit that change.
**Back up both files and the password, offline.** If they are lost, installed copies can never accept updates again (a
new installer would have to be installed by hand once, carrying a new key). If someone else gets them, make a new key at
once. The key and its password never pass through anyone else.

## Making a release that the button offers

1. Raise the version (`app/src-tauri/tauri.conf.json`, `app/src-tauri/Cargo.toml`, `app/ui/package.json`): every release
   needs a higher version than the one before it, or the button won't offer it.
2. Commit everything and tag the commit `v<version>` (`CLAUDE.md`). Write what is new in the tag's message: it becomes the
   notes the button shows.
3. Double-click `BUILD SIGNED PRISM SURVEY RELEASE.bat` (it runs `app\BUILD-INSTALLER.ps1 -Sign -Updates`; the signing
   settings `OPENSTAGE_SIGN_*` must be set in Windows). It finds the key in your profile folder, stops before building if
   `PUBKEY` is not its public half, and asks for the password. It writes `latest.json` beside the installer, and
   `previous.json` (the published release's `latest.json`) when there is one.
4. Publish the release `v<version>` as a **full release** (not a pre-release), with the installer, its `.sha256.txt`,
   `latest.json` and `previous.json`. GitHub turns the space in the installer's name into a dot; `latest.json` already
   names the file that way.
5. From a computer with the previous version installed: Help, Check for updates, Check now, read the notes, install, and
   see the version change.

Versions up to and including 0.1.0 preview 2 have no button, so the first version with one is installed by hand; updates
work from then on.
