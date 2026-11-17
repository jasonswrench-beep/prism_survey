# Prism Survey

A Prism tool that turns a published measure into a correctly scored scale for a Word questionnaire. The design is the Prism Survey Specification (Jason's file, 2026-11-17); anything not in it is agreed with Jason first. Build decisions are in `docs/DECISIONS.md`.

## Rules

- Journal articles and Jason's stored scale files are test material only: never commit them, quote them in docs, or show them in screenshots. Tests use the synthetic scales in `tests/fixtures`.
- Commit as `jasonswrench-beep <324576087+jasonswrench-beep@users.noreply.github.com>`. It is set in this repository's git config (`user.name`, `user.email`); check it with `git config user.name` and `git config user.email` before every commit.
- No `Co-Authored-By` lines and no "Generated with Claude Code" lines, ever: not in commit messages, tag messages, pull requests or release notes.
- Commit everything and tag that commit before building any installer, so each release tag points at the exact commit the build came from. This includes the 0.1.0 signed build.
- No em dashes in prose written for this project.
- Repository text names no commercial products and makes no comparisons with products on the market (naming Word where needed is fine).
- Permissive dependency licences only (MIT, BSD, Apache 2.0, ISC, Zlib); list them in `THIRD-PARTY.md` (`python tools/third_party.py` writes and checks it). One exception, Jason's ruling of 2026-11-17: the five MPL-2.0 crates Tauri needs for the window (cssparser, cssparser-macros, dtoa-short, selectors, option-ext), unchanged. Any other new MPL or copyleft dependency is asked about first.
- Anything that needs running is a double-clickable script.
- AI stays optional: every step works without a model.
- When a decision could reasonably go either way, ask rather than pick silently.

## Working

- `cargo test --workspace` for the engine; `npm run build` in `app/ui` for the interface.
- The window and the tests share one command layer, `crates/prismsurvey-api`.
