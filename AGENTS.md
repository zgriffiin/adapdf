# Adapdf — agent notes

Read `README.md` and the specs in `docs/specs/` before any change. The specs
are the source of truth. If code and spec disagree, raise it; do not silently
pick one.

## Rules that are easy to break

- **Fix the source, not the PDF.** After export, Adapdf changes only
  document-level metadata and adds a `/Scope` to table header cells that Word left
  without one (spec 02, stage 4). Never edit text, tag types or the reading order.
- **Never change the user's original .docx.** Fixes go into
  `accessible.docx`, in the folder `<name> - Adapdf` next to the original.
  The copy is written atomically, and never while Word has it open. Everything
  else that Adapdf makes (the PDF and the reports) goes into the same folder.
- **No network calls and no paid services.** Documents stay on the machine.
- **Never invent content** such as alt text or titles. Ask the user. The app may
  offer a one-click suggestion that is taken from the document itself (the file
  name, the Title paragraph, the first Heading 1 for the title). It fills nothing in
  until the user chooses it, and the user can change it. Alt text has no
  suggestion.
- **No real city documents in git.** Real guides go in `samples/`, which git
  ignores. Test fixtures are small documents made for testing.
- Word COM: start your own hidden instance, and always close it. Never touch
  the user's open Word windows.
- **Word cannot open a path over 259 characters** (`fix::WORD_PATH_LIMIT`, measured).
  The copy is always the short `accessible.docx`; a fix or "Open in Word" stops
  with a plain message when its full path is too long; the export gives Word a
  copy in a short temporary folder, never the real path.

## Stack

Rust (stable, edition 2024). `zip` and `roxmltree` read the .docx; fixes are
byte-range splices on the original XML. `lopdf` writes the PDF metadata and the table scopes as an
incremental update. Word runs through COM with the `windows` crate. veraPDF
ships with a bundled Temurin Java runtime. The UI is Tauri 2 with IBM Carbon (`@carbon/react`; front-end packages with
pnpm). Tauri builds a per-user NSIS installer (no admin rights). Tests that
need Word sit behind the cargo feature `windows_word`.

## Checks before you finish

For every change (fast, a few minutes):

```
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo test -p adapdf-app          # if you changed the app (run pnpm build first)
pnpm test                         # in app/, if you changed app/src
```

Also run only the Word or veraPDF test file of the area you changed, for
example `cargo test --features windows_word --test fix_word`. Do not run the
full `--features windows_word,verapdf` suite (25 to 30 minutes) for a change.
It runs once per release, before the installer is built.

After you change a fixture or the fixture builder, run
`cargo run --example gen-fixtures`. A test fails when `tests/fixtures/` differs
from the generator.
