# Test plan

Status: M2 (all pre-check rules), plus groups, canvases and annotated
screenshots (IMG-007), M3a (the Word automation layer, the first `windows_word`
tests, and table styles in the colour rules) and M3b (stage 2: the fix engine and
`adapdf fix`), including the stage 2 tests that need Word, M4a (stage 3: the
Word export as a library), M4b (stage 4, the PDF metadata fixes, and stage 5, the
veraPDF run, as library code) and M4c (stage 6, the report, and the command
`adapdf export`, which joins stages 1, 3, 4, 5 and 6), and the rules LNK-004 and
LNK-005 with their fixes, which the first S5-10 run asked for (decided
2026-11-16), and M5a (the desktop app: Tauri and IBM Carbon, its commands, the
front-end tests and the check of the built page), and M5b (the per-user installer,
tried by a real install and uninstall on one machine). This plan says what is tested now and what is only planned. A row
marked "planned" has no test yet.

## Scope

M1 built stage 1 of the [pipeline](specs/02-pipeline.md#stage-1--pre-check):
open a .docx, build a typed view of it, run the pre-check rules, and report
findings with `adapdf check`. M1 covered DOC-001 to DOC-006, HDG-001 to
HDG-004, HDG-006, LST-001 and LST-002 (HDG-005 was removed; see "How M1 reads
the rules"). M2a added the IMG, TXB and TBL rules and M2b added the LNK, CLR and TXT rules. A
later change added groups and drawing canvases as one object, widened IMG-004,
and added IMG-007 (annotated screenshots). Every rule in
[03-precheck-rules](specs/03-precheck-rules.md) is now built.

M3a added the base that the Word export will build on: `src/word` starts the
tool's own hidden Word through COM and always quits it (Windows only). It also
added the first `windows_word` tests (W-01: Word opens every fixture), and it
made the model apply table styles, so that CLR-001 and CLR-002 see a text colour
or a fill that comes only from a table style.

M3b built stage 2 ([02 pipeline](specs/02-pipeline.md#stage-2--fix)) and the
command `adapdf fix`: the fixes for every auto and ask fix ID, the checks and the
backup before a write, and stage 1 again after the fixes. The rows that need
Word (S2-02, S2-04, S2-15 and the part of S2-23 that Word can answer) were built
on the Word module of M3a, in `tests/fix_word.rs`. They also gave the real name
of Word's lock file (the rule in `fix::lock_file_names` was corrected), and the
name of a backup now uses local time.

M4b built stage 4 (`src/pdfmeta`) and stage 5 (`src/verapdf`) as library code,
the script that downloads the bundled tools (`scripts/fetch-vendor.ps1`) and the
first test for N-01. "How M4b reads stages 4 and 5" and "Known gaps in M4b"
below say what was decided and what is left.

M4a built stage 3 ([02 pipeline](specs/02-pipeline.md#stage-3--word-export)) as
the library `adapdf::export` (`export_pdf`): stage 1 first, then the tool's own
hidden Word opens the document read-only, updates the fields and calls
`ExportAsFixedFormat`; a timeout ends only the tool's own Word. The S3 rows are
built: see "Stage 3: Word export" and "How M4a reads stage 3". M4a also exports
the 33 pass fixtures of S5-10 (35 since LNK-004 and LNK-005) and checks that each PDF is tagged, so the veraPDF
exit criterion has inputs.

M4c built stage 6 (`src/report`: the report as a value, as HTML and as JSON) and
the command `adapdf export` (`src/pipeline.rs`, with `src/main.rs`). It also ran
the real Word and the real veraPDF on the fixtures for the first time, so it
had the first answer to the exit criterion S5-10: **29 of the 33 pass fixtures
gave 0 veraPDF PDF/UA-1 failures, and 4 did not** (the 3 link fixtures and the
table of contents fixture). The reason was one problem: Word writes no
description for a link, unless the link has a ScreenTip, and no pre-check rule
looked for that. The user decided on 2026-11-16 to add two rules, **LNK-004** (a
link has no ScreenTip) and **LNK-005** (the table of contents is made of links),
both errors with an auto fix. They are built, the pass fixtures were updated,
and S5-10 was run again on 35 pass fixtures: see "S5-10 result" for both runs.
"How M4c reads stage 6 and export", "How M2b reads the rules" and "Known gaps in
M4c" say what was found, what was decided and what is left.

M5a built the desktop app in `app/` (a Vite, React and TypeScript page with
`@carbon/react`, and a Tauri 2 shell in `app/src-tauri/`), the Tauri commands that
call the library, the saving of the sign-off into the report (row A-08), and the
checks of the built page (N-05, the window's content security policy, the
permissions). "How M5a builds the app" and "Known gaps in M5a" below say what was
decided and what is left.

M5b built the per-user NSIS installer (`scripts/build-installer.ps1`), the install
guide (`docs/install-guide.md`) and the config tests of the installer
(`app/src-tauri/tests/installer_config.rs`), and ran a real install, a check and an
export from the installed copy, and an uninstall. "How M5b builds the installer" and
"Known gaps in M5b" below say what was done and what is left. "Planned tests (not
built)" lists each of the rows that still have no test, requirement by requirement.

**Milestones and phases.** The milestone numbers are not the phase numbers of
[spec 01](specs/01-product.md#phases). M1 and M2 (with M2a and M2b) together
are phase 2 (pre-check). M3 (fixes) is phase 4. M4 (Word export, PDF metadata
fixes, veraPDF and the report) is phase 3. M5 (app and installer) is phase 5.
Phase 0 (test with real guides) and phase 1 (template) have no milestone. M3a is
the first part of M3: the Word automation layer, test W-01 and the table style
colours. M4a is stage 3 (the Word export, as a library); stages 4 and 5 are M4b,
and stage 6 with `adapdf export` is M4c. M4a, M4b and M4c are built. M5a is the app; M5b is the
installer and the bundled tools (built). The milestone numbers are the plan at the time of
writing.

## Test levels

| Level | What it checks | Where | Status |
|---|---|---|---|
| Unit | The .docx reader, the document model (styles, table styles, numbering, theme, formatting, text boxes, sections), finding types, fonts, text output | `src/**` (`#[cfg(test)]`) | implemented |
| Rule | Each rule against fixtures and small built documents: rule ID, severity, fix kind, location, thresholds | `tests/rules_document.rs`, `tests/rules_headings.rs`, `tests/rules_lists.rs`, `tests/rules_images.rs`, `tests/rules_textboxes.rs`, `tests/rules_tables.rs`, `tests/rules_links.rs`, `tests/rules_colour.rs`, `tests/rules_text.rs` | implemented |
| Fixture | The files in `tests/fixtures/` match the generator byte for byte. Every rule has a passing and a failing fixture. | `tests/fixtures.rs` | implemented |
| CLI | `adapdf check`: exit codes 0, 1 and 2, text output, JSON shape | `tests/cli.rs` | implemented |
| Fix (stage 2) | The fix engine on fixtures and small built documents: only the spliced bytes change, the backup, the lock file, the atomic write, each fix and its refusals, and the stage 1 re-run. `adapdf fix`: the questions, the answers on stdin, and the exit codes. | `tests/fix_safety.rs`, `tests/fix_engine.rs`, `tests/fix_fixes.rs`, `tests/fix_links.rs`, `tests/cli_fix.rs` (helpers in `tests/fix_support/`), and unit tests in `src/fix/` | implemented |
| Word integration (`windows_word`) | Word opens every fixture (W-01). The session quits its Word after success, an error and a panic. The model's table style colours match what Word shows. Word's lock file name, and a file that Word has open is never written (S2-02, S2-04). Every fixed file opens in Word and Word reads the fix as the model does (S2-15). Word exports a tagged PDF, in the tool's own hidden Word, with the fields updated, without touching a person's Word or the .docx; a timeout ends only the tool's own Word (S3). The exit codes of `adapdf export` with the real Word: a file that Word rejects, a timeout, warnings only, a locked PDF (S3-04, S3-06, S3-14, S3-19). | `tests/word.rs`, `tests/word_table_styles.rs`, `tests/fix_word.rs`, `tests/export_word.rs`, `tests/export_fixtures.rs` and `tests/export_pipeline_word.rs` (helpers in `tests/word_support/` and `tests/export_support/`), behind the cargo feature `windows_word` | implemented for W-01 to W-04 (M3a, 6 tests), S2-02, S2-04, S2-15 and the Word part of S2-23 (4 tests), the Word rows of S3 (M4a, 15 tests) and their exit codes (M4c, 4 tests) |
| PDF metadata (stage 4) | The metadata checks and fixes on small PDFs made by test code (classic and cross-reference-stream layouts, with and without an earlier update): the original bytes are a prefix of the output, each fix, no change when nothing is missing, no change to content or tags, encrypted and damaged PDFs are refused. On a real Word export: what Word writes and what stage 4 adds (S4-11). | `tests/pdfmeta_fixes.rs` (PDF builder in `tests/pdf_support/`), unit tests in `src/pdfmeta/`, and `pdfmeta_on_a_real_word_export` in `tests/export_verapdf.rs` | implemented (M4b); S4-11 implemented (M4c, needs both features) |
| veraPDF (stage 5) without Java | The run command, the JSON of veraPDF read into findings, the table of rule IDs, the pass decision, missing tools, crashes, the timeout, the WCAG profile (with a fake runner and JSON recorded from the real veraPDF) | `tests/verapdf_parse.rs`, `tests/data/verapdf/*.json` | implemented (M4b) |
| veraPDF integration (feature `verapdf`) | The real veraPDF with the bundled Java on PDFs made by test code (S5-09), and on the stage 4 output (S4-10) | `tests/verapdf_real.rs`, behind the cargo feature `verapdf` (needs `vendor/`) | implemented (M4b, 8 tests) |
| veraPDF on Word exports (features `windows_word` and `verapdf`) | A PDF exported from a fixture has 0 veraPDF PDF/UA-1 failures (S5-10, S5-11). The real `adapdf export` on a pass fixture (C-05). | `tests/export_verapdf.rs`, behind both cargo features, because it needs Word to export and veraPDF to check | implemented (M4c; S5-12 added with LNK-004 and LNK-005). All 6 tests of the file pass; `verapdf_zero_failures_on_pass_fixtures` passed for 35 of 35 fixtures (it failed for 4 of 33 before the two rules; see "S5-10 result") |
| Report (stage 6) | The report as a value: the verdict, the seven sections, the escaping of text, the JSON against the HTML, and the accessibility of the HTML (headings, tables, labels, contrast, no outside resources) with no browser (S6-01 to S6-08, S6-10) | unit tests in `src/report/tests.rs` | implemented (M4c) |
| Export command without Word or Java | `pipeline::export_docx` with fake stages: every exit code and the report files (C-05 to C-07, S3-12, S3-13, S3-16, S5-06 to S5-08, S6-09); the real `adapdf export` in the cases that need neither Word nor veraPDF (stage 1 errors, tool failures that come first) | `tests/export_pipeline.rs`, `tests/cli_export.rs`, `tests/cli.rs` (C-08) | implemented (M4c) |
| No network | No HTTP client in the dependency graph (N-01), and a built web UI that loads nothing from the network, with a content security policy that blocks it (N-05) | `tests/no_network.rs`, `app/src-tauri/tests/frontend_bundle.rs` | N-01 implemented (M4b; M5a made its scope explicit for the workspace, see "How M5a builds the app"); N-05 implemented (M5a); N-02 to N-04 are planned |
| Front-end | The Carbon views in the web UI: each screen, axe-core checks for WCAG 2.1 A and AA, the focus order, the view model (grouping, buttons, never pre-filled text) and the sign-off form, with the Tauri layer replaced (TypeScript) | `app/src/viewModel.test.ts`, `app/src/App.test.tsx`, `app/src/test/axe.test.tsx`, `app/src/test/contract.test.tsx` (helpers in `app/src/test/`), Vitest, Testing Library, jsdom and axe-core. The colour contrast is measured in the real WebView2 by `app/scripts/axe-webview.mjs` (A-10) | implemented (M5a; 74 tests after review round 1) |
| App commands (Rust) | The functions behind the Tauri commands give the same findings, fix results and export result as the library calls of the command line (A-07), and the sign-off is saved into the report (A-08). The built page, the window's content security policy, the permissions and the installer settings (N-05, I-02, I-08) | unit tests in `app/src-tauri/src/tests.rs`, `app/src-tauri/src/tests/contract.rs` (the data contract with the page, A-09) and `signoff.rs` (`cargo test -p adapdf-app --lib`), and `app/src-tauri/tests/frontend_bundle.rs` (after `pnpm build`) | implemented (M5a; after review round 1: 22 + 7 tests, and 1 more behind `windows_word` and `verapdf`) |
| Installer | The per-user installer installs without admin rights, runs, and removes cleanly on a clean Windows 11 machine | manual checklist | planned (M5) |
| Manual: Phase 0 and PAC | Real guides exported from Word, checked with veraPDF and PAC. A person confirms PAC reports 0 errors. | manual, with files in `samples/` | planned. Phase 0 is postponed until real guides are in `samples/`. |

### Entry and exit criteria

| Level | Entry (what must be true before the level runs) | Exit (what must be true to call it done) |
|---|---|---|
| Unit and rule | The crate builds. No Word, Java or network is needed. | `cargo test` passes with 0 failed, 0 ignored. Every rule has a passing and a failing fixture. Thresholds have tests on both sides. |
| Fixture | The generator code changed, or a fixture was added. | `cargo run --example gen-fixtures` was run. The drift test is green. All `*-pass` fixtures have 0 findings. Every `wp:docPr` id is unique in each fixture. |
| CLI | The `adapdf` binary builds. | Exit codes 0, 1 and 2 are tested for each command. On exit 2 nothing is written to stdout. Output for `--json` parses. |
| Fix (stage 2, without Word) | The crate builds. No Word, Java or network is needed. Tests write only in their own folders under `target/tmp/`, and remove them. | `cargo test` passes with 0 failed, 0 ignored. Every S2 row that does not need Word and every `fix` row has a test with the name in the row. After each fix, only the target bytes changed (S2-08), the file is valid (S2-09) and stage 1 gives the expected findings (S2-10). |
| `windows_word` | A Windows machine with desktop Word 16.0 (Microsoft 365). The tests start their own Word instances; no Word window of a person needs to be closed. The Word build number is noted. | `cargo test --features windows_word` passes. After the run the number of `WINWORD.EXE` processes is the same as before. No file the tests made is left in the temp folder. |
| veraPDF | The bundled veraPDF and Java runtime are in `vendor/` (run `scripts/fetch-vendor.ps1` once). For tests on Word exports, the `windows_word` entry applies too. | `cargo test --features verapdf` passes with 0 failed. The exit criterion in "Stage 5" below (S5-10) holds for the named fixtures: `cargo test --features windows_word,verapdf --test export_verapdf` passes. Met on the evening of 2026-11-16 (local time) for 35 of 35 fixtures and the 6 picture fixtures; the first run, before LNK-004 and LNK-005, failed for 4 of 33 (see "S5-10 result"). |
| Front-end (M5a) | `pnpm install` was done in `app/` (from the lock file). No Word, Java or network is needed. | `pnpm test` passes with 0 failed. `pnpm exec tsc --noEmit` and `pnpm build` pass. axe-core reports 0 violations for WCAG 2.1 A and AA on each screen. After `pnpm build`, `cargo test -p adapdf-app --test frontend_bundle` passes (N-05). `cargo test -p adapdf-app --lib` passes (it includes the check that the files of the data contract are what Rust serialises now, A-09). For a release, `node app/scripts/axe-webview.mjs` (A-10; needs the built program) reports 0 violations with the colour contrast rule on. |
| Installer (manual) | A clean Windows 11 machine or snapshot with a standard (non-admin) user, no Java and no earlier install. | Every row of the installer table is signed with a date, the installer version and the machine used. |
| Phase 0 and PAC (manual) | At least 5 real guides in `samples/` (git ignores them), PAC installed (version noted), the Word build noted. | Every row of the Phase 0 tables has a result. Changes to specs 02 and 03 that the results ask for are made. |

## How to run

| What | Command |
|---|---|
| Everything that runs without Word | `cargo test` |
| Unit tests only | `cargo test --lib` |
| Rule tests for one group | `cargo test --test rules_document` (or `rules_headings`, `rules_lists`, `rules_images`, `rules_textboxes`, `rules_tables`, `rules_links`, `rules_colour`, `rules_text`) |
| Tests for one rule | `cargo test hdg_003` (test names start with the rule ID in lower case) |
| Fixture tests | `cargo test --test fixtures` |
| CLI tests | `cargo test --test cli` (and `cargo test --test cli_fix` for `adapdf fix`) |
| Stage 2 (fix) tests | `cargo test --test fix_safety`, `--test fix_engine`, `--test fix_fixes`, `--test fix_links` and `--test cli_fix`; unit tests: `cargo test --lib fix::` |
| Tests for one fix | `cargo test fix_` or `cargo test ask_` (the names in the stage 2 table) |
| Write the fixtures again after a change | `cargo run --example gen-fixtures` |
| Word tests (need desktop Word; they start their own hidden Word; about 11 minutes) | `cargo test --features windows_word` (also runs everything above). One file: `cargo test --features windows_word --test word`, `--test word_table_styles`, `--test fix_word`, `--test export_word` (S3, about 3 minutes), `--test export_fixtures` (35 exports, about 5 minutes: 286 seconds on 2026-11-17) or `--test export_pipeline_word` (the exit codes of `export`, about 30 seconds) |
| The guard that ends only `WINWORD.EXE` (S3-20; no Word needed) | `cargo test --test word_end_process` |
| Stage 6 (report) tests | `cargo test --lib report::` (no Word or Java) |
| `adapdf export` tests that need neither Word nor veraPDF | `cargo test --test export_pipeline` (fake stages) and `cargo test --test cli_export` (the real program, in the cases that stop before Word) |
| `adapdf export` with the real Word (about 1 minute) | `cargo test --features windows_word --test export_pipeline_word`. Also run by `cargo test --features windows_word`. |
| `adapdf export` with the real Word and the real veraPDF (about 14 minutes: 35 + 6 + 24 exports of about 8 to 10 seconds each, and S4-11, C-05 and S5-12) | `cargo test --features windows_word,verapdf --test export_verapdf`. It needs both features, so no other command builds it. One test: add its name, for example `-- verapdf_zero_failures_on_pass_fixtures --nocapture` (the test prints each fixture and its failed rules). |
| Stage 4 (PDF metadata) tests | `cargo test --test pdfmeta_fixes`; unit tests: `cargo test --lib pdfmeta::` |
| Stage 5 (veraPDF) tests that need no Java | `cargo test --test verapdf_parse` |
| Get the bundled veraPDF and Java (once; needs the network) | `powershell -ExecutionPolicy Bypass -File scripts/fetch-vendor.ps1` (checks pinned SHA-256 values; does nothing if `vendor/` is up to date; `-Force` to fetch again) |
| Tests with the real veraPDF (need `vendor/`; about 40 seconds) | `cargo test --features verapdf` (also runs everything above). One file: `cargo test --features verapdf --test verapdf_real`. With the feature on and `vendor/` missing, these tests fail and say what to run. A plain `cargo test` does not build them. |
| No network (N-01) | `cargo test --test no_network` |
| Front-end tests (in `app/`; after `pnpm install`) | `pnpm test` (74 tests), `pnpm exec tsc --noEmit`, `pnpm build` |
| Tests of the app commands and the sign-off (Rust; need no Node and no window) | `cargo test -p adapdf-app --lib` |
| Write the sample files of the page contract again, after a change to a type that the page receives (A-09), and look at the diff | `ADAPDF_REGENERATE_CONTRACT=1 cargo test -p adapdf-app --lib contract` (the files are in `app/src/test/contract/`; `calls.json` is written by hand). Then `pnpm test` in `app/`. |
| Colour contrast and the other axe-core rules in the real WebView2 (A-10; about 1 minute; starts the app and Word) | `pnpm tauri build --no-bundle` in `app/`, then `node scripts/axe-webview.mjs --vendor <vendor folder>` in `app/`. Without `--vendor` the one real export is skipped. |
| The installer settings (I-02, I-08, I-11 and the neutral name; 4 tests; the tests read two JSON files, but compiling the app needs `pnpm build`, as for the next row) | `cargo test -p adapdf-app --test installer_config` |
| N-05, the settings of the window, the capabilities and the language of the page (after `pnpm build`; 7 tests) | `cargo test -p adapdf-app --test frontend_bundle`. A plain `cargo test -p adapdf-app` also builds the program of the app, which reads `app/dist` when it is compiled, so it needs `pnpm build` first. |
| The command `export` of the app with the real Word and veraPDF (about 10 seconds; set `ADAPDF_VENDOR` when `vendor/` is not in the source tree) | `cargo test -p adapdf-app --lib --features windows_word,verapdf app_export_with_word` |
| Build or start the app | `pnpm tauri dev` and `pnpm tauri build --no-bundle` in `app/` (the program is `target/release/adapdf-app.exe`) |
| Build the installer (about 3 minutes with a cached build; needs `vendor/`, see `scripts/fetch-vendor.ps1`; M5b) | `powershell -ExecutionPolicy Bypass -File scripts/build-installer.ps1` (add `-Vendor <folder>` when `vendor/` is not at the root of the tree, for example in a Git worktree). The installer is `target/release/bundle/nsis/Adapdf_0.1.0_x64-setup.exe`. |
| Install it, run a check and an export from the installed copy, and uninstall (I-01 to I-05, I-10, I-11, N-03; needs Word; changes this machine) | By hand, see "How M5b builds the installer": `Adapdf_0.1.0_x64-setup.exe /S`, then in `app/`: `node scripts/axe-webview.mjs --installed %LOCALAPPDATA%\Adapdf\adapdf-app.exe --no-java --watch`, then `%LOCALAPPDATA%\Adapdf\uninstall.exe /S`. |
| Format, lint | `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` (the root package only). The app: `cargo fmt --all --check` and, after `pnpm build`, `cargo clippy -p adapdf-app --all-targets -- -D warnings` |

The manual tests of the app (A-05, A-06) and the installer rows that a person must do (I-06,
I-07, I-09) have no script; "How M5b builds the installer" says how the others were run.

## Test conventions

- **Fixtures come from code.** `DocxBuilder` in `src/fixtures.rs` writes a
  small valid .docx. The zip is stored (not compressed) with a fixed time
  stamp, so the same code gives the same bytes. This replaces the hand-built
  Word fixtures that the first version of spec 02 asked for.
- **One failing and one passing fixture for every rule**, named
  `tests/fixtures/<RULE-ID>-fail.docx` and `<RULE-ID>-pass.docx`. The failing
  fixture must trigger its rule. The passing fixture must have no findings
  from any rule.
- **No drift.** `tests/fixtures.rs` fails when a file on disk differs from the
  generator, or a file is missing or extra.
- **Fixtures are written the way Word writes them, where this is known.**
  Every `wp:docPr` id is unique in a document (`DocxBuilder::build` numbers
  them; `drawing_ids_are_unique_in_every_fixture`). A DrawingML shape, group or
  canvas is inside `mc:AlternateContent`, with `mc:Choice` (`Requires` `wps`,
  `wpg` or `wpc`) and a VML `mc:Fallback`, like `text_box()`. A picture points
  to a real 8 x 8 PNG part (`src/fixtures/media.rs`; the builder adds the part,
  its relationship and its content type). The model reads
  only the Choice copy, so nothing is counted twice
  (`img_007_a_shape_in_alternate_content_is_counted_once`,
  `a_shape_group_or_canvas_in_alternate_content_is_one_drawing`). See "Known
  gaps in M2a" for what is still different from Word.
- **CLI tests write each fixture to its own file.** The `fixture()` helper in
  `tests/cli.rs` adds a counter to the file name, so tests that run in
  parallel never write the same path.
- **Fix tests** (M3b) run the fix on the bytes of a fixture, or of a small
  document made by the builder, and compare. Two helpers in
  `tests/fix_support/mod.rs` check S2-08. `assert_exactly_these_edits` is the
  check that can find a part that was rewritten: the test has a table, written
  by hand from the fixture, of the exact original text and the exact new text of
  each change; the entries must have the same names and order, every part but
  the target must have the same bytes, and the target must equal the original
  with those edits and no other change. It does not use the splice list of the
  engine. `assert_only_the_splices_changed` only checks that the engine is
  consistent with itself: it applies the engine's own splice list by hand, and so
  it cannot see a splice that is too wide. Tests that write files use a folder of their
  own under `target/tmp/fix/` (`TempDir`), which is removed at the end. The
  clock and the file calls are injected, so no test depends on the time of day.
- **Rule tests check** the rule ID, severity, fix kind, and the whole
  location (part, paragraph index, snippet, heading above) of the failing
  fixture. The shared helper `assert_location` in `tests/common/mod.rs` checks
  all four fields. All 41 failing-fixture tests use it. The snippet
  is the first 60 characters of the paragraph, so a paragraph that holds only a
  drawing has the snippet `""` (IMG-001 to IMG-005, IMG-007). TXB-001 uses the
  text in the box instead. Problems with the document as a whole (DOC-001,
  DOC-002, DOC-004, DOC-005, HDG-001) have no paragraph, snippet or heading,
  and the tests assert that. TXT-003 has no paragraph; its snippet is the extra
  words, and its part is the header or footer.
- **Every finding is complete.** `every_finding_of_every_fixture_is_complete`
  in `tests/fixtures.rs` runs every fixture and checks, for every finding, that
  the message and the "how to fix" text are not empty, that the snippet and the
  heading are at most 60 characters (characters, not bytes), and that the rule
  ID is in spec 03. `the_rules_in_the_code_are_the_rules_in_spec_03` checks
  that the rule IDs of the code and of the spec 03 tables are the same set.
- **Thresholds have edge tests** on both sides of the limit: exactly 400
  words (no finding) and 401 (finding); 12 words and 13 words for a fake
  heading; 59, 60 and 61 characters for a snippet; 3 digits and 4 digits for a
  typed number; 2 and 3 letters in another script.
- **Font lookup is injected.** DOC-006 tests use a fixed lookup. `SystemFonts`
  is tested with font files that the test writes itself. No test depends on
  the fonts on the machine.
- **Word tests are safe for a person's Word.** Each test starts its own hidden
  Word (`WordSession`) and takes one shared lock, so two tests never start Word
  at the same time. The shared helpers are in `tests/word_support/`. A test that
  needs Word to write its `~$` file opens the document for editing
  (`open_for_editing`) and closes it with "do not save"; no test saves a
  document. The `WINWORD.EXE` processes are listed with `tasklist` (not
  with the code under test) before and after, and the two lists must be equal,
  so a Word that the person has open is never closed. A test that could hang
  (a damaged file) runs on its own thread with a time limit; when the limit
  passes, the test ends the Word process that it started and fails. The files
  that a test writes go in a folder under the temp folder, which is removed at
  the end.
- **No real documents** are in the repository.

## Counts at the time of writing

**After M5b**, `cargo test` at the root runs **775** tests (unchanged), `cargo test -p
adapdf-app` runs **33** (29 before: the 4 tests of `installer_config.rs`; the test of the
installer settings in `frontend_bundle.rs` was moved there and its drag and drop part
stays), and `pnpm test` runs 74 (unchanged). The table is in "How M5b builds the
installer".

**After M5a and its review round 1**, `cargo test` at the root runs **775** tests (the
769 below plus 4 in `src/report/tests.rs`, 1 in `tests/no_network.rs` and 1 in
`src/verapdf/mod.rs`), and the app has its own: 29 with `cargo test -p adapdf-app` and
74 with `pnpm test`. The table is in "How M5a builds the app". The measurements below
are those of M4 and are not repeated for M5a (the `windows_word` and `verapdf` rows
were not run again, except `app_export_with_word_matches_the_cli_export`).

Measured on 2026-11-17 (local time) on the tree after M4 review round 1 (the tree
with LNK-004 and LNK-005, plus the changes of round 1: the PDF is checked before it
replaces `Guide.pdf`, the guard test of `end_word_process`, the checks of the export
arguments and of the read-only open, the text of the table of contents, the
`ADAPDF_VENDOR` change, and the check of `src/` for sockets). The counts are summed
from the "test result" lines of each command; every command was run on the tree
of round 1, one after the other. A second change of round 1 (both reports are written to
temporary files before anything is moved: C-11) touched only `src/pipeline.rs` and
`tests/export_pipeline.rs`; after it `cargo test` and `cargo test --features verapdf`
were run again (the first two rows below), and `cargo test --features windows_word
--test export_pipeline_word` (4 passed, 46 seconds). The two Word rows were
measured before that change. The independent review of M4 (round 2, commit
`0ac5bc8`) then ran `cargo test --features windows_word,verapdf --no-fail-fast` on
the final tree: 812 passed, 0 failed, 27 minutes 14 seconds including a fresh build,
with no `WINWORD.EXE` before or after. The `windows_word` row (798) is derived from
that run (812 less the 6 of `export_verapdf` and the 8 of `verapdf_real`), not
measured on its own. After that review the temporary PDF got a short name
(`~adapdf-<process>-<n>.pdf`); `cargo test` (769) and the real-Word files
`export_word` (14) and `export_pipeline_word` (4) were run again and passed. Windows only: two tests of M3b (the held file handle in
S2-03 and the folder with the name of the backup in S2-06), `the_production_clock_is_local_time`
(S2-07), the 21 unit tests of `src/export/`, the 2 unit tests of `src/word/export.rs`,
the 3 tests of `tests/verapdf_parse.rs` in its `process` module, `tests/word_end_process.rs`,
and all tests of `tests/export_pipeline.rs` and `tests/cli_export.rs` (they use
`adapdf::pipeline`, which needs Word's types); the counts include them.

| Command | Tests run | Failed | Time |
|---|---|---|---|
| `cargo test` | 769 | 0 | 41 seconds (the build was cached) |
| `cargo test --features verapdf` | 777 | 0 | 70 seconds (cached) |
| `cargo test --features windows_word` | 798 (derived from the next row; 796 measured before the second change) | 0 | 11 minutes 19 seconds (the 796 run) |
| `cargo test --features windows_word,verapdf --no-fail-fast` | 812 (measured by the review, commit `0ac5bc8`) | 0 | 27 minutes 14 seconds, fresh build |

(Without `--no-fail-fast`, cargo stops at the first test file that has a failure,
so the files after it do not run; use the flag to get all counts. The counts before
round 1 were 759, 767, 788 and 802; the difference is 8 in each, and 10 in the first two rows after the second change.)

**`cargo test`: 769.** 228 unit (225 before: the 2 tests of the export arguments in
`src/word/export.rs` and `a_release_build_ignores_adapdf_vendor`), 14 CLI, 3 CLI for
`export` (`tests/cli_export.rs`), 13 CLI for `fix`, **27 for the export pipeline with
fakes** (`tests/export_pipeline.rs`; 22 before: `a_locked_pdf_is_found_before_word_starts`,
`a_pdf_that_is_locked_at_the_end_changes_nothing`,
`a_verapdf_failure_replaces_the_old_pdf_and_the_old_reports`,
`a_report_that_cannot_be_replaced_leaves_the_new_pdf_and_says_so` and
`a_blocked_export_writes_its_reports_without_leaving_temporary_files`; the test of the
report that cannot be written was changed), 14 fix engine, 38 fix, 25
for the link fixes (`tests/fix_links.rs`), 12 fix safety, 8 fixture, 24 PDF metadata, 22 veraPDF
without Java, **4 no network** (3 before: `the_source_makes_no_socket`), **1 for the guard
of `end_word_process`** (`tests/word_end_process.rs`, new), 31 DOC rule tests, 28 HDG, 17 LST,
61 IMG, 35 TXB, 29 TBL, 39 LNK, 50 CLR and 46 TXT rule tests. The total before round 1 of 759
plus 3 unit, 5 pipeline, 1 no network and 1 guard test is 769.

**`--features verapdf`: 777.** The 769 and the 8 of `tests/verapdf_real.rs`.

**`--features windows_word`: 796 when measured (798 now, derived).** The 767 of that time and 29 more: 5 in `tests/word.rs`, 1 in
`tests/word_table_styles.rs`, 4 in `tests/fix_word.rs`, 14 in `tests/export_word.rs`,
1 in `tests/export_fixtures.rs` (35 exports) and 4 in
`tests/export_pipeline_word.rs` (the exit codes with the real Word); the guard test of
S3-20 is already in the 767. The run took 11 minutes 19 seconds (`export_fixtures` 286
seconds, `export_word` 193 seconds, `fix_word` 53 seconds, `export_pipeline_word` 34
seconds). Before and after the run the list of `WINWORD.EXE` processes was the same
(empty).

**`--features windows_word,verapdf`: 810 when measured, 812 measured by the review.** The 796, the 8 of
`tests/verapdf_real.rs` and 6 in `tests/export_verapdf.rs`
(`cli_export_exit_0_on_a_pass`, `verapdf_zero_failures_on_pass_fixtures`,
`verapdf_zero_failures_on_the_picture_pass_fixtures`,
`verapdf_on_warning_fixtures_is_recorded`, `pdfmeta_on_a_real_word_export` and
`verapdf_zero_failures_after_the_link_fixes`); that file took 823 seconds (about 14
minutes). **All 810 passed**, including S5-10 for 35 of 35 fixtures, with the new order
of the command (the PDF is checked under a temporary name, and then moved). The list of
`WINWORD.EXE` processes was empty before and after the run.

The count changes as tests are added. `cargo test -- --list` gives the current
list. Earlier measurements: M4a on its own commit, 609 (`cargo test`) and 634
(`windows_word`); M4b on its own commit, 647 and 655 (`verapdf`); M4c, 709, 717,
738 and 751 (with 1 failed: S5-10); the tree with LNK-004 and LNK-005, 759, 767, 788
and 802 (all passed).

## Traceability: pre-check rules

Test names start with the rule ID in lower case, for example
`hdg_003_threshold_is_12_words`. All fixtures are in `tests/fixtures/`.

### Implemented (M1, M2a, M2b and IMG-007)

All 41 rules of spec 03 are implemented, listed in spec order. DOC, HDG and
LST came in M1; IMG, TXB and TBL in M2a (images, text boxes and tables); LNK,
CLR and TXT in M2b (links, colour and text). IMG-007 came after M2; LNK-004 and
LNK-005 came with the S5-10 result (2026-11-16).

| Rule | Severity, fix | Fixtures | Tests | Status |
|---|---|---|---|---|
| DOC-001 | error, ask | `DOC-001-fail/pass.docx` | `doc_001_*` in `tests/rules_document.rs` | implemented |
| DOC-002 | error, auto | `DOC-002-fail/pass.docx` | `doc_002_*` | implemented |
| DOC-003 | warning, manual | `DOC-003-fail/pass.docx` | `doc_003_*` | implemented (heuristic, see below) |
| DOC-004 | warning, manual | `DOC-004-fail/pass.docx` | `doc_004_*` | implemented (detection not checked against a Word-made file, see below) |
| DOC-005 | error, manual | `DOC-005-fail/pass.docx` | `doc_005_*`; header and footer counting in `model::tests::tracked_changes_in_headers_are_counted` | implemented |
| DOC-006 | error, manual | `DOC-006-fail/pass.docx` | `doc_006_*`; `fonts::tests::*` for the font files | implemented |
| HDG-001 | error, manual | `HDG-001-fail/pass.docx` | `hdg_001_*` in `tests/rules_headings.rs` | implemented |
| HDG-002 | error, manual | `HDG-002-fail/pass.docx` | `hdg_002_*` | implemented |
| HDG-003 | warning, auto | `HDG-003-fail/pass.docx` | `hdg_003_*` | implemented (heuristic) |
| HDG-004 | error, auto | `HDG-004-fail/pass.docx` | `hdg_004_*` | implemented |
| HDG-006 | warning, manual | `HDG-006-fail/pass.docx` | `hdg_006_*` | implemented |
| IMG-001 | error, ask | `IMG-001-fail/pass.docx` | `img_001_*` in `tests/rules_images.rs` (groups and canvases: `img_001_a_group_*`) | implemented (a group or canvas is one object; shapes that IMG-007 reports are skipped) |
| IMG-002 | warning, ask | `IMG-002-fail/pass.docx` | `img_002_*`; `rules::images::tests::*` for the placeholder patterns | implemented (heuristic) |
| IMG-003 | warning, manual | `IMG-003-fail/pass.docx` | `img_003_*` | implemented |
| IMG-004 | warning, auto | `IMG-004-fail/pass.docx` | `img_004_*` | implemented (every floating object that is not decorative, see below) |
| IMG-005 | warning, manual | `IMG-005-fail/pass.docx` | `img_005_*` | implemented (heuristic) |
| IMG-006 | warning, manual | `IMG-006-fail/pass.docx` | `img_006_*` | implemented (fixture uses an equation; OLE is tested in memory) |
| IMG-007 | error, ask | `IMG-007-fail/pass.docx` | `img_007_*` | implemented (heuristic: anchor paragraph, see below) |
| TXB-001 | error, manual | `TXB-001-fail/pass.docx` | `txb_001_*` in `tests/rules_textboxes.rs` | implemented (also text in groups and canvases, and one finding for the text shapes on a screenshot) |
| TXB-002 | warning, manual | `TXB-002-fail/pass.docx` | `txb_002_*` | implemented |
| TXB-003 | warning, manual | `TXB-003-fail/pass.docx` | `txb_003_*`; `rules::textboxes::tests::*` | implemented |
| TXB-004 | warning, auto | `TXB-004-fail/pass.docx` | `txb_004_*` | implemented |
| LST-001 | error, auto | `LST-001-fail/pass.docx` | `lst_001_*` in `tests/rules_lists.rs`; `rules::lists::tests::*` for the marker patterns | implemented |
| LST-002 | warning, manual | `LST-002-fail/pass.docx` | `lst_002_*` | implemented |
| TBL-001 | error, auto | `TBL-001-fail/pass.docx` | `tbl_001_*` in `tests/rules_tables.rs` | implemented |
| TBL-002 | error, manual | `TBL-002-fail/pass.docx` | `tbl_002_*` | implemented |
| TBL-003 | warning, manual | `TBL-003-fail/pass.docx` | `tbl_003_*` | implemented |
| TBL-004 | warning, manual | `TBL-004-fail/pass.docx` | `tbl_004_*` | implemented (heuristic) |
| TBL-005 | warning, auto | `TBL-005-fail/pass.docx` | `tbl_005_*` | implemented |
| TBL-006 | warning, auto | `TBL-006-fail/pass.docx` | `tbl_006_*` | implemented |
| LNK-001 | warning, manual | `LNK-001-fail/pass.docx` | `lnk_001_*` in `tests/rules_links.rs` | implemented |
| LNK-002 | warning, manual | `LNK-002-fail/pass.docx` | `lnk_002_*` (40 and 41 characters) | implemented |
| LNK-003 | error, manual | `LNK-003-fail/pass.docx` | `lnk_003_*`; `model::fields::tests::*` for reading links, fields and bookmarks | implemented |
| LNK-004 | error, auto | `LNK-004-fail/pass.docx` | `lnk_004_*` in `tests/rules_links.rs` (element, simple and complex field, empty and blank ScreenTips, table of contents entries left out, links in headers); `model::fields::tests::*` for the ScreenTip, the source ranges and the place in a table of contents; the fix: S2-32 | implemented (decided 2026-11-16) |
| LNK-005 | error, auto | `LNK-005-fail/pass.docx` | `lnk_005_*` (`\h` among other switches, not `\h` in a quoted word or `\hz`, content control, two tables); the fix: S2-33 | implemented (decided 2026-11-16) |
| CLR-001 | error, manual | `CLR-001-fail/pass.docx` | `clr_001_*` in `tests/rules_colour.rs` (table styles: `clr_001_white_header_text_on_a_dark_fill_from_the_table_style_passes`, `..._fails_for_header_text_that_is_only_too_light_because_of_the_table_style`, `..._banded_rows_that_fail_come_from_the_band_fill`, `..._tbl_look_can_switch_the_band_off`, `..._a_fill_set_in_the_cell_beats_the_table_style`, `..._follows_the_based_on_chain_of_a_table_style` and two more); `model::colour::tests::*` for the WCAG values; `model::table_styles::tests::*`; `table_style_colours_match_word` (windows_word) | implemented (table styles applied, see "Known gaps in M2b") |
| CLR-002 | warning, manual | `CLR-002-fail/pass.docx` | `clr_002_*` (table styles: `clr_002_the_table_style_colour_is_the_colour_of_the_text_around`, `clr_002_white_header_text_from_the_table_style_is_not_reported`) | implemented (heuristic, see below) |
| TXT-001 | warning, manual | `TXT-001-fail/pass.docx` | `txt_001_*` in `tests/rules_text.rs` (9.5 pt and 10 pt) | implemented |
| TXT-002 | warning, auto | `TXT-002-fail/pass.docx` | `txt_002_*` | implemented |
| TXT-003 | warning, manual | `TXT-003-fail/pass.docx` | `txt_003_*` | implemented (heuristic, see below) |
| TXT-004 | warning, auto | `TXT-004-fail/pass.docx` | `txt_004_*` | implemented |

"auto" and "ask" in this table are the fix kinds the finding reports. The fixes
themselves are built (M3b): see "Stage 2: fixes" below, which has a row for each
fix ID. Severities and fix kinds are checked
against spec 03 in `findings::tests::severities_and_fixes_match_spec_03` (DOC,
HDG, IMG, TXB, LST and TBL) and `severities_and_fixes_match_spec_03` in
`tests/rules_links.rs` (LNK, CLR and TXT). The fix `UpdateFields` (TXT-004)
writes nothing: Word does it at export (M4; decided point 1).

The model parts for IMG, TXB and TBL are tested in `model::drawings::tests::*`
(including groups, canvases and VML groups) and `model::tables::tests::*`.

## Traceability: pipeline and interfaces

| Item (spec 02) | Tests | Status |
|---|---|---|
| Stage 1: open the .docx as a zip and read the parts | `docx::tests::*`, `model::tests::*`; CLI tests for a file that is not a .docx, a zip with no `word/document.xml`, damaged XML, and a missing file | implemented |
| Stage 1: finding fields and location | `findings::tests::*`, the location checks in the rule tests, `json_shape_of_a_finding` in `tests/cli.rs` | implemented |
| Stage 1: errors block export | The exit code 1 on errors is tested (`exit_code_1_when_there_are_errors`). The library export refuses on any error finding, Word is not started and no PDF is written (S3-12, S3-13: `export_is_refused_when_doc_005_fires`, `export_writes_no_pdf_when_stage_1_has_errors`). The command `adapdf export` gives exit 1 with a Fail report and no PDF (decided point 7): `cli_export_exit_1_on_errors`, `cli_export_blocks_every_error_rule_and_doc_005` and, with fakes, `pipeline_writes_no_pdf_when_stage_1_has_errors` | implemented (library: M4a; command and report: M4c) |
| Interface `adapdf check`, exit codes 0, 1, 2, `--json` | `tests/cli.rs` | implemented |
| Stage 2: fix, backup, lock-file check | "Stage 2: fixes" below (S2-01 to S2-31): `tests/fix_safety.rs`, `tests/fix_engine.rs`, `tests/fix_fixes.rs`, `fix::*` unit tests | implemented (M3b). S2-02, S2-04 and S2-15 (`tests/fix_word.rs`, windows_word) are implemented. S2-23 is implemented for the part that Word can answer; its manual part is planned. |
| Stage 2: the user picks the fixes, and ask fixes take typed text | S2-11 to S2-13; `adapdf fix` rows C-01 to C-04 | implemented (M3b) |
| Interface `adapdf fix` | C-01 to C-04: `tests/cli_fix.rs`; C-09 and C-10 (the link fixes): `tests/fix_links.rs` | implemented (M3b; C-09, C-10 with LNK-004 and LNK-005) |
| Stage 3: Word export through COM | S3-01 to S3-20: `src/export/tests.rs` (unit, fake Word), `tests/export_word.rs` and `tests/export_fixtures.rs` (windows_word); the exit codes of the command: `tests/export_pipeline.rs` (fake Word) and `tests/export_pipeline_word.rs` (windows_word) | implemented (M4a) as the library `adapdf::export`, and (M4c) the exit codes and the report parts of S3-04, S3-06, S3-12, S3-13, S3-14, S3-16 and S3-19; S3-17 is still planned. M4 review round 1 changed the order of the command: the PDF is made under a temporary name and replaces `Guide.pdf` only after stages 4 and 5 (C-11) |
| Stage 4: PDF metadata fixes | S4-01 to S4-12 (`src/pdfmeta`) | implemented (M4b: `tests/pdfmeta_fixes.rs`, S4-10 in `tests/verapdf_real.rs`; M4c: S4-11 in `tests/export_verapdf.rs`, and the exit codes in `tests/export_pipeline.rs`) |
| Stage 5: veraPDF run and message table | S5-01 to S5-12 (`src/verapdf`) | implemented (M4b: S5-01 to S5-09 in `tests/verapdf_parse.rs` and `tests/verapdf_real.rs`; M4c: the exit codes of S5-06 to S5-08 in `tests/export_pipeline.rs` and `tests/cli_export.rs`, S5-10 and S5-11 in `tests/export_verapdf.rs`). S5-10 passes for 35 of 35 fixtures and the 6 picture fixtures after LNK-004 and LNK-005 (the first run failed for 4 of 33; see "S5-10 result"); S5-12 (the link fixes, then the export) is in the same file. |
| The bundled tools (spec 01, technology) | `scripts/fetch-vendor.ps1`, run by hand; `Tool::locate` in `tests/verapdf_parse.rs` and `tests/verapdf_real.rs`; `ADAPDF_VENDOR` in `tests/cli_export.rs` (debug builds only: I-10) | the script was written and was not run as a script by the worker (see "Known gaps in M4b"); the same steps were done by hand and the real veraPDF tests pass on the result. The orchestrator found that the first run stopped at `Get-FileHash` (a module that did not load in the shell that was used); M4c replaced it, and the other module cmdlets that the script used for files, with .NET calls. The changed script was checked only for its SHA-256 function, against `sha256sum`. |
| Stage 6: HTML and JSON report, and its own accessibility | S6-01 to S6-11: `src/report/tests.rs`, `tests/export_pipeline.rs`, `tests/cli_export.rs` | implemented (M4c) except S6-11 (manual, not done) |
| Interface `adapdf export` | C-05 to C-08 and C-11: `tests/export_pipeline.rs`, `tests/cli_export.rs`, `tests/cli.rs`, `tests/export_verapdf.rs` | implemented (M4c) |
| Fixtures open in Word (spec 02, Testing) | W-01 (`word_opens_every_fixture`) | implemented (M3a) |
| Word automation base: own hidden Word, always quit (spec 02, "Rules for COM") | `tests/word.rs`: `the_session_quits_word_after_a_panic`, `the_session_quits_word_after_an_error`, `a_damaged_docx_gives_an_error_and_not_a_hang`, `a_missing_file_is_an_error` (windows_word). These cover the base only; the export rows S3-01 to S3-20 were built later (M4a, M4c, M4 review round 1: see "Stage 3: Word export"), and S3-17 is still planned. | implemented (M3a) |
| App (file picker, findings list, export, report) | A-01 to A-10 | implemented (M5a): A-01 to A-04, A-07 to A-10, and the part of A-06 that a test can do. A-05 (manual walkthrough) and the manual part of A-06 are planned; the axe run (A-10) was repeated on the installed copy in M5b |
| Installer | I-01 to I-11 | implemented (M5b): I-02, I-08 and the unit part of I-10 and I-11 are tests; I-01, I-03, I-04, I-05, I-10 and I-11 were run by hand on one machine (2026-11-17); I-06, I-07 and I-09 are planned (a person, or another machine) |
| Word template and acceptance (spec 04) | T-01 to T-10 | planned (phase 1, template) |
| No network (spec 01) | N-01 to N-05 | N-01 implemented (M4b: `tests/no_network.rs`; round 1 added the check of `src/`; M5a: the app's Windows tree, and the check of `app/src-tauri/src/`); N-05 implemented (M5a: `app/src-tauri/tests/frontend_bundle.rs`); N-03 partly (M5b: a sampled look at the connections, not a firewall log); N-04 checked by hand (M5b), no test; N-02 planned |
| Phase 0 and PAC | The Phase 0 tables and P0-01 to P0-09 | planned (postponed) |

## Planned tests (not built)

Nothing in this section is built, except the rows W-01 to W-04 in "Fixture
realism", which are marked implemented, the stage 2 rows and the `fix` rows
(C-01 to C-04), which have a status column that says what was built in M3b, the
stage 3 to 6 rows (S3, S4, S5 and S6), the rows C-05 to C-08 (built in M4a, M4b
and M4c; their text says what was built) and the row N-01. Each
row says what a test must show, so that
the person who builds the stage knows when it is done. Test names are
proposals (for the built rows they are the real names). Levels: **unit** (no Word, Java or network), **CLI** (runs the
`adapdf` binary), **windows_word** (needs Word, feature-gated), **veraPDF**
(runs the bundled veraPDF), **front-end** (TypeScript, Vitest and
axe-core) and **manual** (a person, with a record). The
columns are: requirement (with the spec it comes from), planned test name,
level, fixture or input, and the pass condition.

Test seams the unit rows need. They are design requirements of the tests. The
seam of stage 2 is built (M3b): the file system calls go through the trait
`fix::FileSystem`, which a test can record or make fail, and the clock is a
value in `fix::Environment`. The seam of stage 3 is built (M4a): the Word calls
go through two small traits (`WordApp` and `WordDoc` in `src/export/engine.rs`),
with a fake in `src/export/tests.rs` that records calls and can block, fail or
panic; the ending of a process is a third trait (`EndProcess`); the timeout is
`ExportOptions::timeout`, and `ExportOptions::after_open` is a hook that runs on
the Word thread with the process id and Word's `Document.ReadOnly` of the open document
(`export::OpenDocument`), so that a test can hold Word at a known place and read what Word has. The seam of stage
5 is built (M4b): the process calls go through the trait `verapdf::Runner`, which
a test can replace with a fake that records the command and gives any output,
error or timeout, and the Java and veraPDF paths (`verapdf::Tool`) and the
timeout are arguments.

### Decided points

The plan found these points while it was written. They are now decided. The
rows say "(decided point N)" where they depend on one. Points 6 to 8 are also in
spec 02.

1. **`UpdateFields` (TXT-004) is done at export.** Only Word can rebuild a
   table of contents, and stage 2 has no Word. The fix writes nothing to the
   .docx. The fix list shows it as "done at export", and stage 3 updates the
   fields. It is tested in stage 3 only.
2. **Alt text in `mc:AlternateContent` goes into both copies.** The fix writes
   `descr` in the `mc:Choice` DrawingML and `alt` in the `mc:Fallback` VML,
   so the two copies agree. The model still reads only the Choice copy.
3. **Empty paragraphs (TXB-004): one stays.** The fix deletes all but one
   paragraph of each row of empty paragraphs.
4. **A heading style the document does not define (HDG-003) is added.** The
   fix adds Word's built-in definition of that Heading style to `styles.xml`,
   then applies it. It never refuses for this reason.
5. **Word's lock-file name is read from a real Word** (row S2-02), and the
   observed names are copied into this plan. Done: see row S2-02.
6. **An existing `Guide.pdf` is replaced, but only after the new PDF has been
   checked** (M4 review round 1). A PDF that another program has locked gives
   exit 2 with a message that names the program's file. If a tool fails after the
   export, the folder is left as it was (C-11).
7. **A report is written when stage 1 blocks the export.** It says "Fail" and
   lists the errors. No PDF is written.
8. **A PDF with no tags** (`/MarkInfo` or `/StructTreeRoot` missing) **is
   exit 1**. It is an error finding about the document, not a tool failure.
9. **`AlignLeft` (TXT-002) sets a direct setting.** The fix sets
   `w:jc w:val="left"` in the paragraph's own `w:pPr`. It does not change the
   style, so no other paragraph changes.
10. **LNK-004: a link has no ScreenTip (decided by the user, 2026-11-16).**
    Error; the fix is auto, after the user confirms. It copies the link's own
    visible text into its ScreenTip (`w:tooltip`, or `\o "text"` in a `HYPERLINK`
    field). That is the author's text, not invented content. A link with no text,
    or only spaces, is refused with a message. Entries of a table of contents are
    not reported (point 11). Reason: Word writes a PDF link without `/Contents`
    unless the link has a ScreenTip, and veraPDF fails PDF/UA-1 7.18.1-2 and
    7.18.5-2 (S5-10). Whether PAC also fails it is for Phase 0 (P0-10).
11. **LNK-005: the table of contents is made of links (decided by the user,
    2026-11-16).** Error; the fix is auto, after the user confirms. It removes
    the `\h` switch from the `TOC` field instruction and nothing else. Stage 3
    updates the table at export and Word then makes it without links. Readers
    still move through the PDF with its bookmarks (`CreateBookmarks`) and the
    headings. Stage 4 does not write `/Contents`: it still changes only
    document-level metadata.

### Fixture realism (stage 1)

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| W-01 | Word opens every fixture (spec 02, "Testing") | `word_opens_every_fixture` (`tests/word.rs`) | windows_word | Every `tests/fixtures/*.docx` (82 files). The pictures point to a real PNG part. | Each file opens read-only in the test's own hidden Word (one session, all open at once), with no repair and no error, and Word counts at least one paragraph in it. The `WINWORD.EXE` processes after the test are the same processes as before. **Implemented (M3a).** Result on 2026-11-16, Word 16.0.20527: all 78 open (82 after LNK-004 and LNK-005: the 82 pass in the full `windows_word` run of 2026-11-16, evening); the person's own Word (PID 49988) was running and was not touched. Word also opened the fixtures before the image part was added (a picture with no image data is shown as missing, not rejected), so no fixture builder had to be fixed for Word. |
| W-02 | Word rejects a damaged file with an error and without a dialog (spec 02: a file Word cannot open is an error) | `a_damaged_docx_gives_an_error_and_not_a_hang` | windows_word | Half of a .docx; a plain text file named .docx; a zip with `word/document.xml` that is not valid XML | `WordSession::open_read_only` gives `WordError::Open` with Word's message for each, within 90 seconds. **Implemented (M3a).** A file with 0 bytes is not an error: Word opens it as an empty document (stage 1 refuses it first). |
| W-03 | The session quits its Word after a panic and after an error | `the_session_quits_word_after_a_panic`, `the_session_quits_word_after_an_error` | windows_word | A panic inside `catch_unwind` while the session is alive; `open_read_only` on a file that does not exist | The `WINWORD.EXE` processes are the same as before within 20 seconds. **Implemented (M3a).** The "error" case uses `WordError::FileNotFound`, which the session finds itself without asking Word. An error that Word reports is in W-02, which also compares the process list after it. |
| W-04 | The model's table style colours are what Word shows (CLR-001 and CLR-002 depend on them) | `table_style_colours_match_word` (`tests/word_table_styles.rs`) | windows_word | 29 test tables in 2 documents: every conditional format with its own colour and fill; `tblLook` in both forms; band sizes; header rows; merged cells; `basedOn`; a fill set in the cell; a paragraph style over the table style | For every cell, the fill and the text colour that Word reports (`Cell.Shading.BackgroundPatternColor`, `Range.Font.Color`) equal the model's. **Implemented (M3a).** Result on 2026-11-16, Word 16.0.20527: 453 of 453 cells equal. The test follows the other Word tests: it takes `exclusive()`, records the `WINWORD.EXE` ids before, runs under `within` (300 seconds), compares the ids again with `wait_for_word_pids`, and keeps its files in a `word_support::TempDir`, which is removed also when the test fails. |

### Stage 2: fixes (spec 02, stage 2)

Built in M3b (the engine is `src/fix/`; `fix::apply` works on bytes, `fix::fix_file` adds
the safety rules, `fix::offers` lists what can be fixed). Four rows (S2-02, S2-04,
S2-15 and S2-23) need Word. They are built on the Word module of M3a, in
`tests/fix_word.rs`; the manual part of S2-23 is still planned. The names in the
table are the names of the tests.

Fix IDs in `src/findings.rs`: 13 auto (`SetDefaultLanguage`,
`ApplyHeadingStyle`, `DeleteEmptyHeading`, `ApplyListStyle`,
`MakeImageInline`, `DeleteEmptyParagraphs`, `MarkHeaderRow`, `DeleteEmptyRow`,
`DisallowRowBreak`, `AlignLeft`, `UpdateFields`, `AddScreenTip`, `RemoveTocLinks`) and 3 ask (`SetTitle`,
`SetAltText`, `MarkAnnotationsDecorative`). Each has its own row below.

| # | Requirement | Test name (planned name if not built) | Level | Fixture or input | Pass condition | Status |
|---|---|---|---|---|---|---|
| S2-01 | Check for the `~$` lock file before any write (spec 02, stage 2) | `fix_refuses_when_a_lock_file_exists` | unit | A copy of `DOC-002-fail.docx` in a temp folder, with a lock file that the test makes next to it, for file names of 5, 8, 9 and 20 characters and one with spaces | Exit 2. The message names the file and says to close it in Word. The file bytes are unchanged. No backup is written. | implemented: `fix_refuses_when_a_lock_file_exists` in `tests/fix_safety.rs`. The lock file is the one name that `lock_file_names` gives for each file name (the rule of S2-02; see "How M3b reads stage 2"). The message is at library level; exit 2 is in C-03. |
| S2-02 | The lock-file name is the one Word writes (decided point 5) | `fix_detects_the_lock_file_word_writes` (`tests/fix_word.rs`) | windows_word | The test starts its own hidden Word and opens copies of `DOC-002-fail.docx` for editing (`WordSession::open_for_editing`, never saved), one at a time, in a folder of its own under the temp folder. File names: base names of 1 to 12, 20 and 35 characters; `My Guide.docx`; `Guía de uso.docx`; `Guide.DOCX` | For each file, the `~$` files in the folder are exactly the ones `fix::lock_file_names` gives; `fix_file` stops with `FixError::LockFile` and the file bytes are the same; after Word closes the document no `~$` file is left. The observed names are copied below. | **Implemented.** Result on 2026-11-16, Word 16.0.20527: 17 of 17 names equal the rule. Word replaces the start of a long name with `~$`: **base name (before the extension) of 6 characters or fewer: `~$` + the whole name** (`a.docx` gives `~$a.docx`, `abcdef.docx` gives `~$abcdef.docx`); **7 characters: `~$` + the name without its first character** (`abcdefg.docx` gives `~$bcdefg.docx`); **8 characters or more: `~$` + the name without its first 2 characters** (`abcdefgh.docx` gives `~$cdefgh.docx`; 12 characters give `~$cdefghijkl.docx`; 35 characters give `~$cdefghijklmnopqrstuvwxyz012345678.docx`). `My Guide.docx` gives `~$ Guide.docx`, `Guía de uso.docx` gives `~$ía de uso.docx`, `Guide.DOCX` gives `~$Guide.DOCX`. `lock_file_names` was a guess that checked two forms; it now gives only this name. Not tried: names with a character outside the Basic Multilingual Plane, a name with more than one dot, and an extension other than `.docx`. |
| S2-03 | Try to open the file for writing before any write | `fix_refuses_when_the_file_cannot_be_opened_for_writing` | unit | A copy of `DOC-002-fail.docx` held open by the test with no sharing; a second copy with the read-only attribute | Exit 2 for both. Bytes unchanged. No backup. | implemented: `fix_refuses_when_the_file_cannot_be_opened_for_writing` in `tests/fix_safety.rs`. The held handle part runs on Windows only (it uses `share_mode(0)`); the read-only part runs everywhere. |
| S2-04 | Never write while Word has the file open (AGENTS.md; spec 02) | `fix_never_writes_while_word_has_the_file_open` (`tests/fix_word.rs`) | windows_word | A copy of `DOC-002-fail.docx` as `Guide.docx`, open in the test's own Word: first for editing, then read-only, then for editing again with a file system whose `exists` says no to every `~$` name; then `Other.docx` open for editing while `Guide.docx` is fixed | Exit 2 (the library gives an error and `adapdf fix` exits 2 with nothing on stdout and the file name on stderr). The bytes of the file are the same after each refused attempt (`bytes_unchanged`: the file is read again and compared byte for byte with the original, which is stronger than SHA-256). No backup and no temporary file in the folder. With the owner file hidden from the check, the exclusive open alone stops the fix with `FixError::CannotWrite`. With Word open on a different file, the fix runs and writes a backup. | **Implemented.** Result on 2026-11-16, Word 16.0.20527: open for editing, Word writes `~$Guide.docx` and the fix stops with `FixError::LockFile`. Open read-only, Word writes **no** `~$` file, but it holds the file, so the exclusive open fails and the fix stops with `FixError::CannotWrite`. Both exit 2. With Word open for editing and the `~$` file hidden from the check (`BlindToLockFiles`, so that the owner file check cannot fire), the exclusive open of the file fails and the fix stops with `FixError::CannotWrite`: the safeguard works on its own. In all three refused cases the bytes are unchanged and no backup or temporary file is made. With Word open on `Other.docx` the fix runs. |
| S2-05 | The backup is written before any write to the target (AGENTS.md; spec 02) | `fix_writes_the_backup_before_the_first_write` | unit | Any fix on a fixture copy, with the writer recording every call | The first recorded write is the backup. It was closed and read back equal before the first write to the target. | implemented: `fix_writes_the_backup_before_the_first_write` in `tests/fix_safety.rs`. The recorded calls are: open the file, write the backup, read the backup back, write the temporary file, move it over the file. The read-back is tested in `fix_stops_when_the_backup_does_not_read_back_the_same`: a file system whose `read` gives other bytes, and one whose `read` fails, each give `FixError::Backup`, the target bytes are unchanged, the bad copy is deleted (a read error used to leave it; that was fixed) and no temporary file is written. |
| S2-06 | No backup, no write | `fix_stops_when_the_backup_cannot_be_written` | unit | A directory that has the backup's name, so the backup cannot be made | Exit 2. Target bytes unchanged. | implemented: `fix_stops_when_the_backup_cannot_be_written` in `tests/fix_safety.rs` (the writer fails, also after half of the bytes), and, on Windows, `a_folder_with_the_name_of_the_backup_stops_the_fix` (a folder with the name of the backup: Windows says "access denied", and the tool stops). On Windows only a file with the name, not a folder, gets the next name (S2-07). |
| S2-07 | Backup name `Guide.backup-<yyyymmdd-hhmmss>.docx`, bytes equal to the original (spec 02, "Inputs and outputs") | `fix_backup_name_and_bytes` | unit | `DOC-002-fail.docx` as `Guide.docx`, clock fixed at 2026-03-05 10:15:30 | `Guide.backup-20260305-101530.docx` is next to the source. Its bytes equal the original. A second run in the same second does not overwrite it. | implemented: `fix_backup_name_and_bytes` in `tests/fix_safety.rs`. The second backup in the same second is `Guide.backup-20260305-101530-1.docx`. The time on the real clock is local time: `the_production_clock_is_local_time` (Windows) compares `Timestamp::now()` with the local time that PowerShell (`Get-Date`) gives, within 2 seconds. The test machine is 7 hours behind UTC, so UTC would fail it. The clock is `Environment.now`, so every other test still sets a fixed time. |
| S2-08 | Only the target XML bytes change; all other parts and all bytes outside the changes are identical (spec 02 stage 2; spec 01) | `fix_changes_only_the_spliced_bytes` | unit | The failing fixture of every fix ID in the rows below; the busy Word-like case of `new_properties_go_in_schema_order_in_a_busy_paragraph_and_row` | Every part but the target has equal bytes after unzip, and the entries have the same names and order. The target part equals the original with a table of exact edits and has no other change. For each edit the table has the exact original text that goes and the exact new text; it is written by hand for each fix from its fixture, and it is not the engine's splice list, so a fix that writes a whole paragraph again (or widens its splice) fails. The splice list of the engine must also agree with the result and must not overlap. | implemented: `fix_changes_only_the_spliced_bytes` and `fix_changes_only_the_target_part` in `tests/fix_engine.rs`. The first uses `assert_exactly_these_edits` with the table `expected_edits()` (16 fixes, 26 edits, for example HDG-004 removes one whole `<w:p>...</w:p>` and TXT-002 changes `both` to `left` and nothing else), and also `assert_only_the_splices_changed`, which only checks the engine against its own splice list. The busy case in `tests/fix_fixes.rs` goes through `assert_exactly_these_edits` too (4 edits; its styles define Heading 1, so the style part does not change; an undefined Heading style that is added is S2-18). **Mutation check, 2026-11-16:** a temporary change made the `AlignLeft` fix write its paragraph again with `xml:space="preserve"` removed. The new check failed for TXT-002 ("word/document.xml is the original with these edits and no other change"), and `fix_reruns_stage_1_with_every_auto_problem_in_one_document`, which uses only the splice-list check, still passed. A second temporary change wrote the `w:pStyle` element again with an added space, and the busy case failed. Both changes were removed. It runs for every fix that writes (16 rules). `untouched_entries_are_copied_raw` checks that untouched entries keep their compressed bytes, method and CRC. `a_byte_order_mark_is_kept` checks a part with a byte order mark. |
| S2-09 | The written file is a valid .docx | `fix_result_is_a_valid_docx` | unit | Same | The zip opens, the target part parses as XML, and `Document::from_bytes` reads it. | implemented: `fix_result_is_a_valid_docx` in `tests/fix_engine.rs`. |
| S2-10 | Stage 1 runs again after the fixes and shows the new results (spec 02) | `fix_reruns_stage_1_and_the_fixed_finding_is_gone` | unit | Each failing fixture in the rows below, then one document with every auto problem | The finding of the rule is gone. No new finding appears, except those the rows list. After all auto fixes the document has no auto finding left (decided point 1 aside); the manual findings are the same rules and counts as before. | implemented: `fix_reruns_stage_1_and_the_fixed_finding_is_gone` and `fix_reruns_stage_1_with_every_auto_problem_in_one_document` in `tests/fix_engine.rs`. The extra findings that the plan allows (TBL-004 after S2-28) are tested in the test of that fix. |
| S2-11 | The user picks which fixes to apply (spec 02) | `fix_applies_only_the_fixes_the_user_picked` | unit | A document with 3 auto problems; the user picks 1 | Only that rule's finding is gone. The other two remain. Bytes outside that splice are equal. | implemented: `fix_applies_only_the_fixes_the_user_picked` in `tests/fix_engine.rs`. `the_same_text_cannot_be_changed_by_two_different_fixes`, `two_fixes_on_the_same_drawing_do_not_overlap` and the tests for one `w:pPr` and one `w:trPr` check fixes that meet in one place. |
| S2-12 | Ask fixes never write content the user did not type (AGENTS.md; spec 02) | `ask_fix_without_text_writes_nothing` | unit | Each ask fix ID with empty input and with spaces only; `DOC-001-fail.docx` shows the suggestion "Test guide" but the user types nothing | No write, no backup. The finding remains. The suggestion is never used as input. | implemented: `ask_fix_without_text_writes_nothing` in `tests/fix_engine.rs`. For `MarkAnnotationsDecorative` the "input" is the confirmation: text or "decorative" is refused. `cli_fix_shows_the_suggestion_but_never_fills_it_in` in `tests/cli_fix.rs` checks the CLI. |
| S2-13 | The typed text is what is written | `ask_fix_writes_exactly_the_typed_text` | unit | Text `A & B <C> "D" é 日本` for the title and for alt text | The XML is escaped. Reading the file back gives the same text. Nothing else is added. | implemented: `ask_fix_writes_exactly_the_typed_text` in `tests/fix_engine.rs`. Line breaks and tabs in alt text are written as character references and read back. |
| S2-14 | A failed write does not damage the source | `fix_write_is_atomic` | unit | The writer fails after half of the bytes | The original file is intact. No partial file stays. | implemented: `fix_write_is_atomic` in `tests/fix_safety.rs`: the writer fails after half of the bytes, and the move fails. The backup stays in both cases (it is a whole copy). |
| S2-15 | Word still opens the fixed file (spec 02, "Testing") | `fix_result_opens_in_word` (`tests/fix_word.rs`) | windows_word | Every failing fixture that has an auto or ask fix (16), fixed with its fix by `fix_file` in a folder under the temp folder; also HDG-003 with the Heading 2 style not defined (decided point 4), alt text typed on a picture, shape, group, canvas, VML picture, VML shape and VML group, "Mark as decorative", the ScreenTip of a `w:hyperlink`, a complex field, a simple field and a field in pieces (link text with quote marks, a backslash, `&`, `<`, `>`, `é` and Japanese letters), link texts of 299 characters, and `\h` removed from a table of contents in a content control. 32 cases | In one hidden Word, the original and the fixed file both open read-only (repair off, alerts off) with no error. For documents with no table, the change of Word's paragraph count equals the change of the model's. Word reads the fix as the model does: title (`BuiltInDocumentProperties`), the style of the heading (`Heading 2`) and of the list paragraphs (`List Number`), the alt text (`InlineShapes(1).AlternativeText`), a floating picture that is inline after the fix, the header row (`Rows(1).HeadingFormat`), the number of rows, `AllowBreakAcrossPages`, left alignment (`Paragraphs(n).Alignment` is 3 before, 0 after), the ScreenTip (`Hyperlinks(1).ScreenTip`) and "Use hyperlinks instead of page numbers" (`TablesOfContents(1).UseHyperlinks`, true before and false after). | **Implemented.** Result on the evening of 2026-11-16 (local time), Word 16.0.20527: 32 of 32 cases open and agree (23 of 23 in the first run, before the link fixes). Word reads the ScreenTip as the text of the link in all four forms, with the awkward text, so the escapes (`\"`, `\\`, XML) are right for Word 16.0.20527. **Finding:** for a link text of 299 characters Word reads a `w:tooltip` as its first 260 characters and the `\o` text of a field whole; the test expects exactly that (see "Known gaps in M2b"). The paragraph count is compared as a change, not as a total: Word also counts the end of each table row, so the totals of Word and the model differ. TXT-004 is left out because its fix writes nothing. DOC-002, IMG-007 and the decorative mark only have to open (the checks that Word has for them, language and decorative, are not read). The in-memory chart and SmartArt images are left out: Word cannot open even their original files (see "Known gaps in M3b"). Word gives an error when it is asked for the alt text of a document that is not the active one, so `WordDocument::inline_shape_alt_text` first activates the (hidden) document. |
| S2-16 | `SetTitle` (DOC-001, ask) | `ask_set_title_writes_dc_title` | unit | `DOC-001-fail.docx` (empty `dc:title`); a copy with no `dc:title` element | The typed title is in `docProps/core.xml` only. DOC-001 is gone. Spaces only is refused. | implemented: `ask_set_title_writes_dc_title` in `tests/fix_fixes.rs`, and `set_title_refuses_a_document_with_no_properties_part`. |
| S2-17 | `SetDefaultLanguage` (DOC-002, auto) | `fix_set_default_language` | unit | `DOC-002-fail.docx`; a copy with an empty `w:lang`; a copy with other `rPr` children in the defaults | `w:lang` has the language of the setting (default en-US) in the document defaults of `word/styles.xml`. DOC-002 is gone. A setting of en-GB writes en-GB. | implemented: `fix_set_default_language`, `set_default_language_writes_the_language_of_the_setting` and `set_default_language_in_a_styles_part_with_fewer_parts` in `tests/fix_fixes.rs`. |
| S2-18 | `ApplyHeadingStyle` (HDG-003, auto; the user confirms each paragraph) | `fix_apply_heading_style_at_the_suggested_level` | unit | `HDG-003-fail.docx`; a copy where the style is not defined in `styles.xml` (decided point 4); one with no heading above | The paragraph has the suggested style (Heading 2 in the fixture). The text is the same. HDG-003 is gone. No HDG-002 appears. An undefined Heading style is added from Word's built-in definition, then applied. | implemented: `fix_apply_heading_style_at_the_suggested_level`, `fix_apply_heading_style_when_the_style_is_not_defined` and `fix_apply_heading_style_with_no_heading_above_and_a_style_already_set` in `tests/fix_fixes.rs`. Only Heading 1 to Heading 4 are defined (the tool never suggests a deeper level). |
| S2-19 | `DeleteEmptyHeading` (HDG-004, auto) | `fix_delete_empty_heading` | unit | `HDG-004-fail.docx`; a copy where the empty heading holds a section break; one where it holds a bookmark | The empty paragraph is removed and HDG-004 is gone. The neighbours are byte-equal. A paragraph with a section break or a bookmark is not deleted, and the user is told why. | implemented: `fix_delete_empty_heading` and `fix_delete_empty_heading_refuses_what_it_must_keep` in `tests/fix_fixes.rs`. A page break, a field or a picture in the heading also stops the deletion, and so does the last paragraph of a table cell. |
| S2-20 | `ApplyListStyle` (LST-001, auto) | `fix_apply_list_style` | unit | `LST-001-fail.docx`; a typed-bullet copy; a marker split across two runs; a tab after the marker | The marker and its separator are gone and the text after them is the same. The List Number or List Bullet style is set. LST-001 is gone. LST-002 does not appear. | implemented: `fix_apply_list_style` and four more tests (`fix_apply_list_style_for_every_form_of_typed_marker`, `..._keeps_formatting_of_the_text_that_stays`, `..._refuses_when_the_document_has_no_list_style`, `..._closes_a_mixed_list`) in `tests/fix_fixes.rs`. |
| S2-21 | `SetAltText` (IMG-001 and IMG-002, ask), with the "Mark as decorative" choice | `ask_set_alt_text_writes_descr_or_alt` | unit | `IMG-001-fail.docx`, `IMG-002-fail.docx`; copies with a chart, a shape, a group, a VML picture, and a shape in `mc:AlternateContent` (decided point 2) | The text is in `descr` (DrawingML) and, when there is a VML copy, in its `alt` too, so both copies agree. "Decorative" writes the decorative extension instead. IMG-001 is gone. IMG-002 is gone unless the typed text is itself a placeholder. | implemented: `ask_set_alt_text_writes_descr_or_alt`, `ask_set_alt_text_for_charts_shapes_groups_and_vml`, `ask_set_alt_text_can_mark_as_decorative` and `ask_set_alt_text_for_two_drawings_in_one_paragraph` in `tests/fix_fixes.rs`. "Decorative" also clears a placeholder `descr`. VML cannot be marked decorative: refused. |
| S2-22 | `MakeImageInline` (IMG-004, auto) | `fix_make_inline_for_pictures_charts_and_smart_art` | unit | `IMG-004-fail.docx`; in-memory floating chart and SmartArt | `wp:anchor` becomes `wp:inline` and keeps the extent, `wp:docPr` and graphic. IMG-004 is gone. | implemented: `fix_make_inline_for_pictures_charts_and_smart_art` in `tests/fix_fixes.rs`. A floating picture, chart or SmartArt in `mc:AlternateContent` and a VML picture are refused. |
| S2-23 | IMG-004 caveat: groups, canvases and shapes (see "How M2a reads the rules") | `fix_make_inline_for_groups_canvases_and_shapes` | windows_word, then manual | In-memory `Image::group()`, `canvas()` and `shape()` with `.floating()`, made inline by the fix and opened in Word | A person looks at the result in Word for each object and records whether the layout inside is kept. Until then, the fix is not offered for these three objects and the finding says to do it in Word. After that, the branch that was chosen is tested. | **Partly implemented.** The fix is not offered for these objects (`fix_does_not_offer_inline_for_groups_canvases_and_shapes` in `tests/fix_fixes.rs`), so there is no fixed file to open in Word. What Word can answer is tested in `fix_make_inline_for_groups_canvases_and_shapes` (`tests/fix_word.rs`, windows_word): Word opens the three floating documents and shows each as 1 floating shape and 0 inline objects, so the finding is right that they float. Result on 2026-11-16, Word 16.0.20527: group, canvas and shape each 1 floating and 0 inline. Still planned (manual): a person makes each object inline in Word and records whether the layout inside is kept, and the branch that is chosen is then built and tested. |
| S2-24 | `MarkAnnotationsDecorative` (IMG-007, ask) | `ask_mark_annotations_decorative` | unit | `IMG-007-fail.docx` (three shapes in `mc:AlternateContent`) | After the user confirms that the picture's alt text describes the arrows, each unresolved shape's `wp:docPr` gets the decorative extension (the empty element is expanded and the namespace is declared). IMG-007 is gone. The picture's alt text is unchanged. With no confirmation, nothing is written. If the picture has no alt text, the fix waits for IMG-001. | implemented: `ask_mark_annotations_decorative` and `ask_mark_annotations_waits_for_the_alt_text_of_the_picture` in `tests/fix_fixes.rs`. |
| S2-25 | IMG-007 caveat: VML shapes cannot be marked decorative | `ask_mark_annotations_for_vml_shapes` | unit | A picture with two `Image::vml_shape().floating()` | The tool does not claim to mark them. It asks for alt text for each, or tells the user to convert the file. IMG-007 is gone only after one of these. | implemented: `ask_mark_annotations_for_vml_shapes` in `tests/fix_fixes.rs`. The tool takes the "tells the user to convert the file" branch (and to give each shape alt text in Word). It does not ask for the alt text of the shapes itself. If any unresolved shape is VML, the whole fix is refused. |
| S2-26 | `DeleteEmptyParagraphs` (TXB-004, auto; decided point 3) | `fix_delete_empty_paragraphs` | unit | `TXB-004-fail.docx` (2 in a row); a row of 4; two rows in one document | All but one of each row is deleted. TXB-004 is gone. A paragraph that holds a drawing, break, field or section break is not deleted. | implemented: `fix_delete_empty_paragraphs` and `fix_delete_empty_paragraphs_keeps_what_is_not_empty` in `tests/fix_fixes.rs`. An "empty" paragraph that holds a bookmark (the rule counts it as empty) is not deleted. |
| S2-27 | `MarkHeaderRow` (TBL-001, auto; the user confirms the first row is the header) | `fix_mark_header_row` | unit | `TBL-001-fail.docx`; a copy whose first row has `w:trPr` without `w:tblHeader`; a copy with no `w:trPr`; a nested table | `<w:tblHeader/>` is in the first row's `w:trPr` of the right table only. `w:cantSplit` stays. TBL-001 is gone. Without confirmation nothing is written. | implemented: `fix_mark_header_row` and `fix_mark_header_row_changes_the_right_table_only` in `tests/fix_fixes.rs`. |
| S2-28 | `DeleteEmptyRow` (TBL-005, auto) | `fix_delete_empty_row` | unit | `TBL-005-fail.docx` (row 3 empty); two empty rows; the empty row last | The empty rows are gone and TBL-005 is gone. Other rows are byte-equal. If one row is left, the re-run may show TBL-004 and the test expects it. | implemented: `fix_delete_empty_row` and `fix_delete_empty_row_refuses_what_it_must_keep` in `tests/fix_fixes.rs`. A row that holds a merged cell is kept, and a table in which every row is empty is refused. |
| S2-29 | `DisallowRowBreak` (TBL-006, auto) | `fix_disallow_row_break` | unit | `TBL-006-fail.docx`; a copy with `w:trPr` but no `w:cantSplit`; a copy with `w:cantSplit w:val="0"` | Every row has `<w:cantSplit/>`. The header flag stays. TBL-006 is gone. | implemented: `fix_disallow_row_break` in `tests/fix_fixes.rs`, and `header_row_and_row_break_fixes_on_the_same_row_make_one_trpr`. |
| S2-30 | `AlignLeft` (TXT-002, auto) | `fix_align_left` | unit | `TXT-002-fail.docx` (direct `jc both`); a copy where the style is justified | The paragraph has `w:jc w:val="left"` in its own `w:pPr`. Its effective alignment is left, the style is unchanged, and TXT-002 is gone (decided point 9). | implemented: `fix_align_left` in `tests/fix_fixes.rs`, and `heading_style_and_align_left_on_the_same_paragraph_make_one_ppr`. |
| S2-31 | `UpdateFields` (TXT-004, auto; decided point 1) | `fix_update_fields_is_done_at_export` | unit | `TXT-004-fail.docx` | The fix writes nothing to the .docx and no backup is made. The finding is listed as "done at export". The proof is `export_updates_fields_before_exporting` in stage 3. | implemented: `fix_update_fields_is_done_at_export` in `tests/fix_engine.rs`. The proof that the export does it stays in stage 3. |
| S2-32 | `AddScreenTip` (LNK-004, auto; the user confirms; decided 2026-11-16) | `fix_add_screen_tip_to_a_hyperlink_element`, `fix_add_screen_tip_to_an_internal_link_and_replaces_an_empty_one`, `fix_add_screen_tip_to_a_simple_field_edits_only_the_instruction`, `..._in_single_quotes`, `fix_add_screen_tip_to_a_complex_field_edits_only_the_instruction`, `..._with_its_instruction_in_pieces`, `..._gives_the_instruction_xml_space_preserve`, `..._keeps_the_other_switches_of_a_field`, `fix_add_screen_tip_escapes_quotes_backslashes_and_xml_characters`, `fix_add_screen_tip_writes_the_field_escapes_and_the_xml_escapes`, `fix_add_screen_tip_folds_spaces_tabs_and_line_breaks_in_the_text`, `fix_add_screen_tip_refuses_a_link_with_no_text`, `fix_add_screen_tip_refuses_a_field_that_it_cannot_extend`, `fix_add_screen_tip_changes_only_the_link_that_was_chosen`, `fix_add_screen_tip_leaves_the_entries_of_a_table_of_contents_alone` (`tests/fix_links.rs`); the exact edit in `fix_changes_only_the_spliced_bytes` (`tests/fix_engine.rs`) | unit | `LNK-004-fail.docx` (a `w:hyperlink`); built documents with a simple field, a complex field (also in pieces), an internal link, a link text with `"`, `\`, `&`, `<`, `>`, `é` and Japanese letters, spaces and tabs in the text, links with no text, fields with an empty `\o` or an open quote mark | The ScreenTip is the visible text of the link, with runs of white space made into one space, and nothing else is invented. A `w:hyperlink` gets `w:tooltip`; a field gets `\o "text"` after its last word (the hand-written edit shows one insertion and no other byte changed; the quote marks of the rest of the field keep their form). The text is escaped for the field (`\"`, `\\`) and for XML; the model and Word read the same text back. LNK-004 is gone and no new finding appears. A link with no text, an empty `\o` and an open quote mark are refused with a message that says to type the ScreenTip in Word, and `adapdf fix` shows them as "the tool cannot fix this now". Only the chosen link changes. The entries of a table of contents are not touched. | implemented |
| S2-33 | `RemoveTocLinks` (LNK-005, auto; the user confirms; decided 2026-11-16) | `fix_remove_toc_links_removes_the_h_switch_and_nothing_else`, `fix_remove_toc_links_for_every_place_of_the_switch`, `fix_remove_toc_links_in_a_content_control`, `fix_remove_toc_links_changes_only_the_table_that_was_chosen`, `fix_remove_toc_links_when_the_instruction_is_in_pieces`, `fix_remove_toc_links_keeps_the_old_entries_and_does_not_make_a_new_finding` (`tests/fix_links.rs`); the exact edit in `fix_changes_only_the_spliced_bytes` | unit | `LNK-005-fail.docx`; tables with `\h` first, last, alone, in upper case, among other switches (`\o "1-3" \h \z \u`), after a quoted `\t "Callout \h,2"`, in a content control, two in one document, and an instruction in two pieces | One deletion of ` \h` (the switch and the spaces before it); the rest of the instruction, including `&quot;`, is as it was. A `\h` in a quoted word is not removed. A switch split between two `w:instrText` is refused with the Word way (clear "Use hyperlinks instead of page numbers"). The old entries (`w:hyperlink`) stay until Word updates the table at export, and they are not LNK-004 findings. LNK-005 is gone and no new finding appears. | implemented. `fix_both_link_fixes_in_one_run_clear_both_findings` and `fix_the_link_fixes_write_a_backup_and_the_new_file` run both fixes together. |

### Stage 3: Word export (spec 02, stage 3)

Built in M4a as the library `adapdf::export` (`src/export/`), with no CLI
command and no report (those are M4c). The names below are the real test names.
"unit" tests are in `src/export/tests.rs` and use a fake Word (the traits in
`src/export/engine.rs`); "windows_word" tests are in `tests/export_word.rs` and
`tests/export_fixtures.rs`. See "How M4a reads stage 3" and "Known gaps in M4a".

| # | Requirement | Test name | Level | Fixture or input | Pass condition | Status |
|---|---|---|---|---|---|---|
| S3-01 | One hidden Word started by the tool | `export_starts_its_own_hidden_word` | windows_word | `clean()` document | While the export runs (a hook on the Word thread), a `WINWORD.EXE` process exists that was not in the list taken before. A `WordSession` (the type the export uses) reports `Visible` false and `DisplayAlerts` 0. The hook is also told Word's `Document.ReadOnly` of the open document, and it is true (spec 02: read-only; S3-09). | implemented. The `Visible` and alerts check runs on a session that the test starts itself, because the hook cannot ask Word; the export starts its Word with the same `WordSession::start`. The read-only check (round 1) is read from the document that the export itself opened. |
| S3-02 | The user's Word windows are not used or closed (AGENTS.md; spec 02) | `export_leaves_the_users_word_alone` | windows_word | The test starts a separate "user" Word (a `WordSession`) with fixture B open for editing and an unsaved edit. It notes the number of documents, the text, `Saved` (false) and the file bytes. Then it exports fixture A. A second step exports B itself. | After each export the user's process is alive, has the same number of documents and text, `Saved` is still false, the bytes of B are the same, and the `~$` lock file of B is still there. The `WINWORD.EXE` list equals the list before (it includes the real user's Word, PID 49988 on the test machine). | implemented. The test's "user" Word is hidden (the tool has no API to show Word), so "its window still exists" is not checked. The variant (the same file) is in the same test. |
| S3-03 | The instance is closed after success | `export_closes_word_after_success` | windows_word | `clean()` document | The list of `WINWORD.EXE` process IDs equals the list before (checked for up to 20 seconds; the export itself returns only after Word has quit). No `~$` or temporary file remains. The .docx bytes and modified time are unchanged (it was opened read-only). The 30 second bound of the session is explained in "Known gaps in M3a". | implemented |
| S3-04 | The instance is closed after an error | `export_closes_word_after_an_error` | windows_word | (1) An output folder that does not exist (the error comes before Word starts). (2) A .docx that stage 1 accepts and Word rejects (an empty `[Content_Types].xml`: "The file appears to be corrupted"). | The error is `OutputFolder` and `Word(Open)`. The process list equals the list before. No PDF and no temporary file remain. | implemented, with these differences from the first plan: stage 1 cannot be skipped by a hook (the export has no such switch), so the Word-side error is a file that only Word rejects; a read-only output folder is not tested (on Windows the read-only attribute of a folder does not stop new files; an ACL would be needed). **Exit code 2 (M4c):** `pipeline_word_exit_2_when_word_cannot_open_the_file` (`tests/export_pipeline_word.rs`, windows_word, with the real Word and a stand-in veraPDF) gives exit 2, a message that names the file and says that it "appears to be corrupted", only the .docx left in the folder, and the same `WINWORD.EXE` list; `pipeline_exit_2_when_word_rejects_the_file` is the same with a fake Word. |
| S3-05 | Closed also when the code panics | `export_closes_the_instance_when_the_code_panics` (unit); `export_closes_word_when_the_word_thread_panics` (windows_word) | unit and windows_word | A fake Word that panics inside the export call; the real Word with a hook that panics | Unit: the fake records `Quit` and the result is `WorkerStopped`; no temporary PDF. Real: the process list equals the list before. | implemented |
| S3-06 | Timeout: end the instance the tool started and report the failure (spec 02) | `export_times_out_and_closes_only_its_own_word` (windows_word); `export_times_out_and_ends_only_its_own_word` (unit); `export_times_out_before_word_has_started` and `a_result_is_not_lost_to_the_deadline` (unit) | unit and windows_word | Unit: a fake whose export blocks for 1 second with the timeout at 100 ms; a second fake "user" Word. Real: a hook that blocks until the tool's Word is gone, timeout 15 seconds, and a separate "user" Word (a `WordSession`) with an unsaved edit. | The result is `Timeout` with the id of the tool's Word and `ended` true. The ending is called with the tool's id only (unit: `[111]`; the user's fake has id 222 and is not called). Real: the tool's process is gone; the user's process, document count, text and `Saved` are as before; no PDF and no temporary file. A deadline before Word has started waits for the id and ends it. A result that arrives at the deadline is a result. | implemented. **Exit code 2 (M4c):** `pipeline_word_exit_2_on_a_timeout` (windows_word; timeout 1 ms through `ExportOptions`) gives exit 2, "did not finish the export within" and "ended the copy of Word that it had started" on stderr, an empty stdout, no PDF or report, and an unchanged process list; `pipeline_exit_2_on_a_timeout` is the fake. The command has no switch for the timeout: it is always 5 minutes. |
| S3-07 | The timeout is 5 minutes (spec 02) | `export_timeout_is_five_minutes_by_default` | unit | The default value | 300 seconds. | implemented |
| S3-08 | Timeout with the real Word | `export_times_out_with_real_word` | windows_word | `clean()` document with the timeout set to 1 ms | `Timeout` with `ended` true. The process list equals the list before. No PDF, no temporary file. The .docx is unchanged. | implemented |
| S3-09 | `ExportAsFixedFormat` parameters (spec 02, table) and a read-only open | `export_passes_the_spec_parameters`, `the_parameters_are_the_spec_table` | unit | A fake that records the call; the table in `docs/specs/02-pipeline.md` | The parameters are 17, false, 0, 0, true, 1, true, true, false for `ExportFormat`, `OpenAfterExport`, `OptimizeFor`, `Item`, `IncludeDocProps`, `CreateBookmarks`, `DocStructureTags`, `BitmapMissingFonts`, `UseISO19005_1`, and they equal the values in the spec table. The output file is a temporary name next to the PDF. | implemented. The open is read-only because the traits offer only a read-only open. **Round 1:** the real `Document.ReadOnly` is read through the hook of S3-01 (true). The argument list that `export_as_fixed_format` builds is now a function of its own (`export_arguments`), and two unit tests in `src/word/export.rs` read it: `the_export_arguments_are_the_spec_table` (the 10 names, their order and their types, number or Boolean, and the values, against the table in spec 02) and `each_parameter_goes_to_its_own_argument` (changing one field changes only the argument of that name, so `OptimizeFor`, `Item` and `BitmapMissingFonts`, which show nothing in the PDF, cannot be swapped with another). Checked by mutation: `Item` given the value of `OptimizeFor` fails the second test. What is not observed: that Word uses `OptimizeFor`, `Item` and `BitmapMissingFonts` as the table says (see "Known gaps in M4a"). The `WordSession::open` arguments (hidden, not in the recent files, no repair, no dialog) are read in the code and run in every Word test, not recorded. |
| S3-10 | The parameters have the effect the spec says | `export_makes_a_tagged_pdf_with_bookmarks_and_properties` | windows_word | `clean()` with a Heading 2 added | The PDF has `/MarkInfo` with `/Marked true`, a `/StructTreeRoot`, `/Outlines` with a bookmark for each heading, the Info `/Title` and the XMP title, and no `pdfaid`. | implemented |
| S3-11 | Fields are updated before the export (spec 02) | `export_updates_fields_before_exporting` | unit and windows_word | Unit: a fake that records the order. Word: a document with a table of contents of 1 entry and 4 headings, made without the `\h` switch (a table with `\h` is refused by stage 1: LNK-005); the same document with an up-to-date table of 4 entries; the `TXT-004-fail.docx` fixture (a heading renamed after the table was made). | Unit: `update_fields` comes after the open and before `export`. Word: the PDF has 4 `Link` tags (read with `lopdf`: the tags are in compressed object streams), one for each entry that Word wrote, and not the 1 of the stale table; the numbers of `TOC`, `TOCI` and `Link` tags are those of the up-to-date document; the `TXT-004-fail.docx` PDF has 2 `Link` tags. There is no link annotation (`/Subtype/Link`) in any of them. The .docx on disk is unchanged. This also proves S2-31. | implemented, but not as the first plan said: the PDF text is not read (see "Known gaps in M4a"), so a renamed heading with the same number of entries is not proved by the PDF. The first version counted `/Subtype/Link` annotations of a table with `\h`; LNK-005 made that table an error, so the test counts tags. |
| S3-12 | Export is refused when DOC-005 fires (spec 02) | `export_is_refused_when_doc_005_fires` (unit); `export_is_refused_when_doc_005_fires_and_starts_no_word` (windows_word) | unit and windows_word | `DOC-005-fail.docx` (tracked change); a document with only a comment | `ExportError::Blocked` with the DOC-005 finding (error). The fake Word is never started (the start panics if called); with the real Word the process list is unchanged. No PDF. The message names the rule and "tracked changes" or "comments". The .docx is unchanged. | implemented. **Exit code 1 (M4c):** `pipeline_refuses_doc_005_with_a_fail_report` (fake stages that are never called) and `cli_export_blocks_every_error_rule_and_doc_005` (the real program, no Word needed): exit 1, the report lists DOC-005 and says "tracked changes", no PDF. |
| S3-13 | The PDF is written only when stage 1 has no errors (spec 02, "Inputs and outputs") | `export_writes_no_pdf_when_stage_1_has_errors` | unit now; CLI planned | Each of the 17 `*-fail.docx` fixtures whose rule is an error (DOC-001, 002, 005, 006; HDG-001, 002, 004; IMG-001, 007; TXB-001; LST-001; TBL-001, 002; LNK-003, 004, 005; CLR-001) | Unit: `Blocked` with the rule of the fixture for each of the 17; Word is not started; no `.pdf`. CLI (M4c): exit 1, `Guide.accessibility.html` exists, says Fail and lists the errors (decided point 7). | implemented. Library part (M4a); the command and report part (M4c): `pipeline_writes_no_pdf_when_stage_1_has_errors` (all 17 error fixtures: exit 1, `Guide.accessibility.html` and `.json` exist, the verdict is fail, the rule is listed, `pdf` is null, stages 4 and 5 are "not-run", no Word or veraPDF call) and `cli_export_blocks_every_error_rule_and_doc_005` / `cli_export_exit_1_on_errors` (the real program) |
| S3-14 | Warnings do not block the export | `export_runs_when_only_warnings_exist` | unit and windows_word | `HDG-006-fail.docx` and `TXT-002-fail.docx` | A tagged PDF is written, and the result lists the warning. | implemented. **Report (M4c):** `pipeline_word_runs_when_only_warnings_exist` (windows_word, HDG-006-fail and TXT-002-fail with the real Word): exit 0, a tagged PDF, and the report has 0 errors, 1 warning and names the rule; `pipeline_runs_when_only_warnings_exist` is the fake. |
| S3-15 | The source .docx is not changed by the export | `export_leaves_the_docx_unchanged`; `export_makes_tagged_pdfs_of_the_pass_fixtures` | windows_word | Two documents; the 35 pass fixtures of S5-10 | Bytes and modified time are the same before and after. | implemented |
| S3-16 | Word is not installed | `export_reports_missing_word` | unit | A fake start that fails with `WordError::NotInstalled` | `ExportError::Word(NotInstalled)`; the message says Microsoft Word is not installed. No panic, no PDF. **Exit code 2 (M4c):** `pipeline_exit_2_when_word_is_missing` (fake Word): exit 2, stderr says Microsoft Word is not installed and names the file, stdout empty, no report. | implemented |
| S3-17 | A document Word cannot open (for example a password) | `export_reports_a_password_protected_document` | windows_word | A copy that the test saves with a password through Word | An error with a clear message. The process list equals the list before. | planned. A password-protected .docx is not a zip file, so stage 1 refuses it before Word starts (`a_file_that_is_not_a_docx_is_a_check_error` covers that refusal with a plain text file). Making a real one needs Word to save with a password, which the Word module cannot do yet. The Word-side open error is covered by S3-04. |
| S3-18 | Awkward paths | `export_handles_awkward_paths` | windows_word | A folder and file name with spaces, non-ASCII letters, a dash and brackets; a path of about 220 characters | The PDF is written next to the file and is tagged. | implemented. A path over 260 characters is not tested (Word does not open it). |
| S3-19 | Existing or locked `Guide.pdf` (decided point 6) | `export_replaces_an_existing_pdf`, `export_reports_a_locked_pdf_and_keeps_it`, `a_pdf_that_cannot_be_replaced_at_the_end_keeps_the_old_one` (unit); `export_replaces_or_reports_a_locked_pdf` (windows_word) | unit and windows_word | A `Guide.pdf` from an earlier run; another with an exclusive handle held; one that gets locked while Word works | The first is replaced. The second is `PdfLocked`, names `Guide.pdf`, and is not changed; Word is not started. The third (locked after the early check) is `PdfLocked` too, and the old PDF stays. No temporary file remains. **Exit code 2 (M4c):** `pipeline_word_exit_2_for_a_locked_pdf_and_exit_0_for_one_to_replace` (windows_word): with an exclusive handle on `Guide.pdf` the export gives exit 2 and names `Guide.pdf`, the old bytes stay and no other file appears; after the handle is closed the next run replaces the PDF (exit 0). `pipeline_exit_2_for_a_locked_pdf_names_the_pdf` is the fake. **Round 1:** the command now checks `Guide.pdf` itself, before Word starts (`a_locked_pdf_is_found_before_word_starts`: a real exclusive handle, Word not called, an earlier PDF and earlier reports with the same bytes, no new file), and the move at the end is the command's (`a_pdf_that_is_locked_at_the_end_changes_nothing`: the lock comes while veraPDF works; exit 2 names `Guide.pdf`, the old PDF and the old reports are unchanged and the temporary PDF is gone). | implemented |
| S3-20 | The timeout ends the Word that the tool started and nothing else: the image name is checked (spec 02, "Rules for COM") | `a_process_that_is_not_word_is_not_ended` (`tests/word_end_process.rs`) | unit (Windows; no Word) | A child process that the test starts: `cmd /c pause`, with its input piped | `end_word_process(pid)` is an error that names `cmd.exe` and `WINWORD.EXE` and says it was not ended; `try_wait` still gives `None` (the process runs). The test then ends its own child through its own handle, lets the handle go, and the same call on the id gives `Ok(false)`. | implemented (M4 review round 1). **Mutation check:** with the name test in `end_if_named` (`src/word/com.rs`) switched off (`if false && ...`), the test fails at the first assertion, because the call ended `cmd.exe`; the code was restored. **A defect found by this test and fixed:** the first version of the test failed at the last step. A process that has ended can still be opened while another program holds a handle to it (here, the `Child` value of the test), and then its image name cannot be read, so `end_process_named` gave an error and not `Ok(false)` as its comment says. `end_if_named` now reads the exit code first (`GetExitCodeProcess`) and gives `Ok(false)` for a process that is not running. The id of a process that was reused is still caught by the name check. |

Also built, without a row of the first plan: `a_missing_output_folder_is_an_error_that_names_it`,
`output_that_is_not_a_pdf_is_an_error_and_keeps_the_old_pdf` (a Word that wrote
nothing, or not a PDF), `a_file_that_is_not_a_docx_is_a_check_error`,
`the_docx_is_not_changed_by_the_fake_export` and `error_messages_are_plain`.

### Stage 4: PDF metadata fixes (spec 02, stage 4)

Fixtures: small PDFs made by test code with `lopdf`, in memory, by the `Pdf`
builder in `tests/pdf_support/mod.rs` (not kept as files, so there is no drift
test). `Pdf::complete()` is `tagged-complete`; `Pdf::all_missing()` is
`all-missing`; the other names of the first plan (`missing-pdfua`,
`missing-title`, `missing-displaydoctitle`, `missing-lang`, `no-markinfo`,
`no-structtreeroot`) are the builder's fields changed one at a time. The `modern`
field gives object streams and a cross-reference stream; `with_foreign_update`
adds an update made by another writer.

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| S4-01 | Incremental update: new bytes at the end, existing bytes not rewritten | `pdfmeta_output_starts_with_the_original_bytes` | unit | `all-missing` (classic and stream variants) | The original bytes are a byte prefix of the output. The output is longer. The new tail has its own cross-reference section whose `/Prev` points to the old one. **Implemented (M4b).** The tests run on four layouts: classic, classic after an update by another program (`with_foreign_update`, made with `lopdf`'s own incremental writer), cross-reference stream with object streams, and that after an update. Each is read again with `lopdf` (one more cross-reference section, no repair), and the `/Prev` in the new tail is the old `startxref`. The prefix holds by construction: `pdfmeta::Outcome::appended` has only the new bytes, and the output is the original and then those bytes. |
| S4-02 | XMP `pdfuaid:part` = 1 | `pdfmeta_adds_pdfuaid_part_1` | unit | `missing-pdfua`; a copy that already has 1; a copy that has 2 | Missing: added, other XMP content kept. Already 1: no change. A value other than 1 is reported as an error and not changed. **Implemented (M4b).** The case with no `/Metadata` at all is also tested (`a_missing_xmp_stream_is_made`). |
| S4-03 | XMP `dc:title` and Info `Title` from the .docx title | `pdfmeta_copies_the_title` | unit | `missing-title` with the .docx title "Adapdf test guide"; a copy missing only one of the two | Both are set to the .docx title; only the missing one is added when one is present. An empty .docx title: nothing is invented and an error is reported. **Implemented (M4b).** More: an empty title in the PDF counts as missing, a title with other letters is written as UTF-16 in the Info dictionary and escaped in the XMP, and the title and language come from the .docx model (`the_source_metadata_comes_from_the_docx`). |
| S4-04 | `/ViewerPreferences /DisplayDocTitle true` | `pdfmeta_sets_display_doc_title` | unit | `missing-displaydoctitle`; a copy with a `ViewerPreferences` that has other keys; a copy with it true | Set to true. Other keys are kept. Already true: no change. **Implemented (M4b).** Direct and indirect `ViewerPreferences`. |
| S4-05 | Catalog `/Lang` from the .docx default language | `pdfmeta_copies_the_language` | unit | `missing-lang` with default language en-US; a copy that has `/Lang` | `/Lang (en-US)` is added. An existing value is not changed. **Implemented (M4b).** No language in the .docx and none in the PDF is an error, and nothing is invented. |
| S4-06 | `/MarkInfo /Marked true` and `/StructTreeRoot` are not fixed; the error is reported (spec 02) | `pdfmeta_reports_missing_markinfo_and_structtreeroot` | unit | `no-markinfo`, `no-structtreeroot` | No `/MarkInfo` or `/StructTreeRoot` object is in the appended bytes. An error finding says the export failed (decided point 8). **Implemented (M4b), and stricter than the row:** when either is missing nothing at all is appended, not even the other metadata fixes, because the export failed and the file is thrown away. `MarkInfo` with `Marked false` counts as missing. |
| S4-07 | The content and the tags are never changed (AGENTS.md; spec 01) | `pdfmeta_does_not_touch_content_or_tags` | unit | `all-missing` | The objects in the appended tail are only the catalog, the Info dictionary and the metadata stream (and `ViewerPreferences` if it is an indirect object). No page, content stream, font or structure element is redefined. The catalog's `/StructTreeRoot` and `/Pages` references are the same as before. **Implemented (M4b).** Run on all four layouts. The appended text is also searched for `/Kids`, `/MediaBox`, `/StructElem`, `/Font`, `BT` and `/Contents`. |
| S4-08 | Nothing missing means no change | `pdfmeta_no_change_when_nothing_is_missing` | unit | `tagged-complete` | The output bytes equal the input. The report says there were no changes. **Implemented (M4b)** for both layouts, and `fix_file` leaves the file and its modified time alone. |
| S4-09 | Each change is listed in the report | `pdfmeta_changes_are_listed` | unit | Each fixture above | The list of changes (what, before, after) matches the differences found by comparing the two files. **Implemented (M4b).** Each change has the field, a plain description, the old value and the new value; the JSON names are tested. |
| S4-10 | The result is a readable PDF and the metadata failures are gone | `pdfmeta_output_passes_verapdf_metadata_checks` | veraPDF | Outputs of S4-02 to S4-05 | veraPDF reads the file. The metadata-related failures that the input had are gone. **Implemented (M4b)** in `tests/verapdf_real.rs` (feature `verapdf`), on the four layouts. Before the fix veraPDF fails exactly 5-1, 7.1-9 and 7.1-10; after it, 0 failures. veraPDF has its own PDF reader, so this also shows that the new cross-reference section is valid for a reader that is not `lopdf`. `the_language_failure_goes_away_when_the_pdf_has_a_language` does the same for 7.2-34. |
| S4-11 | What Word already writes (spec 02: "keep only the fixes that are needed") | `pdfmeta_on_a_real_word_export` (`tests/export_verapdf.rs`) | windows_word with veraPDF | A pass fixture exported by Word (`DOC-001-pass.docx`), and a document with an accented title and the language es-US | Values that Word wrote are left alone. The values that were added are recorded. The output starts with Word's bytes. veraPDF accepts the result. | **Implemented (M4c).** Result on 2026-11-16, Word 16.0.20527: **Word already writes the Info `Title`, the XMP `dc:title`, `/ViewerPreferences /DisplayDocTitle true`, the catalog `/Lang` (`en` for `en-US`; also written for `es-US`), `/MarkInfo /Marked true` and the `/StructTreeRoot`. Word does not write `pdfuaid:part`: stage 4 adds exactly that one value (field `xmp-pdfuaid-part`) and nothing else, for both documents.** No change replaces a value (every `old` is empty), the issues are empty, the output (Word's bytes and the appended update) passes veraPDF with 0 failures (so veraPDF's own reader accepts the update of a real Word file, which has a classic cross-reference table), and stage 4 on the output changes nothing. For the Phase 0 table: the title and language fixes of stage 4 were not needed on these two exports; keep them as a safeguard (a missing `dc:title` or `/Lang` with another Word build or other export options). |
| S4-12 | A PDF the tool cannot read | `pdfmeta_reports_a_pdf_it_cannot_read` | unit | A truncated PDF; an encrypted PDF | Exit 2. The input file is unchanged. **Implemented (M4b)** as typed errors (`PdfMetaError::Unreadable`, `Encrypted`, `Io`); the exit code 2 belongs to the `export` command. The input file is unchanged. A PDF whose cross-reference table `lopdf` had to repair is refused too, because an update that points to a wrong table would be wrong. Encrypted: with an owner password only and with a user password. |

### Stage 5: veraPDF (spec 02, stage 5)

Fixtures: JSON output of veraPDF 1.28.2 for small PDFs made by test code (the same
`Pdf` builder), recorded and saved in `tests/data/verapdf/` (7 files; only the file
name in them was shortened). The real-tool rows run the bundled veraPDF.

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| S5-01 | The command is `verapdf --flavour ua1 --format json <pdf>` | `verapdf_runs_the_ua1_profile_command` | unit | A fake runner that records the command | Arguments are exactly `--flavour ua1 --format json` and the PDF path. The bundled veraPDF is used, not one on the PATH. **Implemented (M4b).** The command is the Java of the bundle with the veraPDF jar files and the main class `org.verapdf.apps.GreenfieldCliWrapper`, which is what `verapdf.bat` runs. The arguments after the main class are exactly the ones in the row. |
| S5-02 | Each failed rule becomes a finding with a plain message | `verapdf_json_failed_rules_become_findings` | unit | A saved JSON with 3 failed rules, 2 of them mapped | 3 findings. Each has the clause, the test number, a message, how to fix, and the context path. **Implemented (M4b)** on JSON recorded from the real veraPDF 1.28.2: 3 failed rules, 7 failed rules, and 1 (the figure). All of them are mapped, so the unmapped case is in S5-04. veraPDF writes the failed rules in a different order from run to run; the tool sorts them by clause (as numbers) and test number. |
| S5-03 | A table links common veraPDF rule IDs to the Word cause and the fix (for example "Figure without alt text" to IMG-001) | `verapdf_every_mapped_rule_has_a_message` | unit | The table, and one saved JSON for each mapped ID | Every mapped ID gives its message and its linked pre-check rule, which is in `RuleId::ALL`. The table has no ID without a test. **Implemented (M4b).** The table has 29 entries (28 in M4b; M4c added 7.18.1-2). The test makes a JSON for every entry from a recorded report and checks the message, the fix and the linked rule; `the_mapped_rule_ids_are_these` lists the IDs, so a new entry cannot be added without a place in the test. 11 of the IDs are also in the reports that were recorded from the real veraPDF. |
| S5-04 | An ID that is not in the table is still shown | `verapdf_unmapped_rule_keeps_the_original_text` | unit | A saved JSON with an unmapped rule | A finding with the original clause text and ID. It is not dropped. **Implemented (M4b).** The finding has veraPDF's own description, the clause and the test number. |
| S5-05 | The WCAG machine-check profile, if installed, is shown as warnings (spec 02) | `verapdf_wcag_profile_results_are_warnings` | unit | A fake runner for a veraPDF that has the profile, and one that does not | Profile results have severity warning and do not change pass or fail. Without the profile there is no second run, and the report says it was not available. **Implemented (M4b)**, with the real profile file `WCAG-2-2-Machine.xml` of veraPDF 1.28.2 ("WCAG 2.2 machine validation profile"; `--profile <file>`). The second run happens only when that file is in the bundle. A WCAG finding that repeats a PDF/UA-1 failure (same specification, clause and test) is not shown twice. A WCAG run that fails is recorded (`WcagOutcome::Failed`) and does not change the result. The real run is in `the_wcag_machine_profile_of_the_bundle_runs`. |
| S5-06 | veraPDF or Java is missing: exit 2 with a clear message | `verapdf_missing_gives_exit_2` | CLI | The path of veraPDF set to a folder that does not exist; a veraPDF folder without the Java runtime | Exit 2 in both. stderr names what is missing. stdout is empty. **Implemented (M4b) as typed errors** (`JavaMissing`, `VeraPdfMissing`), checked before anything is started, with the path in the message. **The exit code 2 and the empty stdout (M4c):** `verapdf_missing_gives_exit_2` in `tests/export_pipeline.rs` (fake tool check: Java and veraPDF each; exit 2, stderr names the missing path, Word is not started, stdout empty) and, with the real program, `cli_export_exit_2_on_tool_failure` in `tests/cli_export.rs` (the environment variable `ADAPDF_VENDOR` points at a folder that does not exist, one with no Java and one with no veraPDF: exit 2, stdout empty, the message names the file and the missing tool, no PDF and no report). The check comes before Word starts, but after stage 1: a document with stage 1 errors still gets its Fail report when veraPDF is missing (`a_blocked_export_does_not_need_the_tools`). |
| S5-07 | A crash is not a pass | `verapdf_crash_or_bad_output_is_not_a_pass` | unit | A runner that exits with an error, gives empty output, or gives invalid JSON | Exit 2, not pass. **Implemented (M4b)** as typed errors for 11 cases, including a PDF that veraPDF cannot read (exit code 7 and a `taskException` in the JSON), a timeout and a program that cannot start. **The exit code 2 (M4c):** `verapdf_crash_or_bad_output_is_exit_2` (fake runner errors: a crash, a timeout and bad output give exit 2, "The PDF could not be checked", no report). The timeout is also tested with a real process (`verapdf_timeout_stops_the_process`) and with the real veraPDF. |
| S5-08 | Any failure means the PDF does not pass | `verapdf_any_failure_means_not_pass` | unit | JSON with 1 failed check; JSON with 0 failed checks | 1 failed check: not pass, exit 1. 0 failed: pass. **Implemented (M4b).** Not compliant, a failed check, or a failed rule: not a pass, also when the rule list is empty. A job that did not end normally is an error. **The exit code 1 (M4c):** `pipeline_exit_1_on_a_verapdf_failure` (the recorded JSON of a figure without alt text: exit 1, the rule 7.3-1 is in the report with the pre-check rule IMG-001, the PDF is kept next to the Fail report) and `pipeline_wcag_results_are_warnings_and_do_not_fail_the_run`. |
| S5-09 | The real veraPDF runs on a PDF made by test code | `verapdf_runs_on_a_generated_pdf` | veraPDF | `tagged-complete` | The JSON parses and the failure count is the one the test knows. **Implemented (M4b)** (feature `verapdf`): `Pdf::complete()` (the `tagged-complete` fixture) has 0 failures, in the classic layout and with object streams; an untagged PDF with an unembedded font has exactly 7 failed rules; a figure without alt text fails only 7.3-1 and is linked to IMG-001. |
| S5-10 | **Exit criterion.** Word-made PDFs of passing fixtures have 0 veraPDF failures (spec 01, success criterion 1, applied to the fixtures) | `verapdf_zero_failures_on_pass_fixtures` | veraPDF with windows_word | The 35 pass fixtures that have content and need no picture data: DOC-001 to DOC-006, HDG-001 to HDG-004, HDG-006, LST-001, LST-002, TXB-001 to TXB-004, TBL-001 to TBL-006, LNK-001 to LNK-005, CLR-001, CLR-002, TXT-001 to TXT-004 and IMG-006. | Each is exported by Word and checked. 0 failures for each. The 6 pass fixtures with a picture (IMG-001 to IMG-005 and IMG-007) are in `verapdf_zero_failures_on_the_picture_pass_fixtures`. Failing fixtures are not in the criterion, because stage 1 blocks those with errors. A failure here is a finding for Phase 0: a missing source rule or a Word export problem. **Built (M4c). First run: it failed for 4 of the 33 fixtures (LNK-001, LNK-002, LNK-003 and TXT-004); the test was strict and was not changed to hide them.** That led to the rules LNK-004 and LNK-005 (decided points 10 and 11). **Second run, with the 2 rules and the 2 new pass fixtures: 35 of 35 give exit 0 and 0 failed rules, and so do the 6 picture fixtures.** See "S5-10 result" for each fixture, the veraPDF rules, the classification and the evidence. |
| S5-11 | Information only: veraPDF on failing fixtures that have only warnings | `verapdf_on_warning_fixtures_is_recorded` | veraPDF with windows_word | The `*-fail` fixtures of warning rules that can be exported | The veraPDF failures are recorded, to see which warnings need to become errors. No pass condition. **Implemented (M4c)** as a test that records and has no pass condition except that the tool does not fail. Result on the evening of 2026-11-16 (local time), with LNK-004 and LNK-005: of the 24 failing fixtures of warning rules, 1 is blocked by an error of another rule (`LST-002-fail.docx` also has LST-001: the typed number is the problem of both rules) and 23 give 0 veraPDF failures after stage 4 (DOC-003, DOC-004, HDG-003, HDG-006, IMG-002 to IMG-006, TXB-002 to TXB-004, TBL-003 to TBL-006, LNK-001, LNK-002, CLR-002, TXT-001 to TXT-004). In the first run (2026-11-16, before the two rules) LNK-001, LNK-002 and TXT-004 gave the two link failures (7.18.1-2 and 7.18.5-2); their documents now have a ScreenTip or a table of contents without `\h`. **No warning rule needs to become an error because of a veraPDF failure.** |
| S5-12 | A guide with links that have no ScreenTip and a table of contents made of links: stage 1 refuses it, `adapdf fix` does both auto fixes, and `adapdf export` gives a PDF with 0 veraPDF failures (decided points 10 and 11) | `verapdf_zero_failures_after_the_link_fixes` | veraPDF with windows_word | A built guide: a table of contents with `\h`; a `w:hyperlink`, a complex `HYPERLINK` field and a simple one, with no ScreenTip | Before: the export gives exit 1 with LNK-004 three times and LNK-005, and writes no PDF. `adapdf fix` with four yes answers: exit 0, "Fixes applied (4)". After: exit 0, 0 failed rules; the PDF has 3 `/Subtype/Link` annotations (the table of contents has none), each with `/Contents` equal to its link text. | implemented. Result on the evening of 2026-11-16 (local time), Word 16.0.20527, veraPDF 1.28.2: passes. |

### Stage 6: report (spec 02, stage 6)

The sections of the report are the seven in spec 02. "Automated HTML test"
means a test that parses the report and checks it. If the report is written as
well-formed XHTML, `roxmltree` can parse it with no new dependency; otherwise
the test needs a parser, which needs a decision.

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| S6-01 | The 7 sections are present, in order: Summary; Pre-check findings; Fixes applied; PDF metadata changes; veraPDF result; PAC; Human checklist | `report_has_the_seven_sections_in_order` | unit | A fixed set of results | The `h2` headings are those seven, in that order. A section with no content says so ("None"). **Implemented (M4c)** in `src/report/tests.rs`: the `h2` headings are the seven, in order, for a full failing report and for a passing one; in the passing one, sections 2, 3 and 4 say "None." (and 3 says that no backup was found). |
| S6-02 | Summary: pass or fail, number of errors and warnings, tool version, date | `report_summary_fields` | unit | Results with 2 errors and 3 warnings; a fixed clock | The summary shows fail, 2, 3, the crate version and the date. Pass or fail is in words, not only colour. **Implemented (M4c)**, with a report of 2 pre-check errors, 3 pre-check warnings, 1 failed veraPDF rule and 1 WCAG warning (3 errors and 4 warnings in the summary): the verdict, the counts, the tool version (`CARGO_PKG_VERSION`) and the date are in the summary table, next to their names, and the result is the words "Result: Fail" and "Result: Pass". |
| S6-03 | Fixes applied, including the backup file name | `report_lists_fixes_and_the_backup_name` | unit | Results with 2 fixes and the backup `Guide.backup-20260305-101530.docx` | Both fixes and the exact backup name are in section 3. **Implemented (M4c)** in `src/report/tests.rs` and `tests/export_pipeline.rs`. The `export` command applies no fix, so its report says "None. No fixes were applied in this run." and, if a `Guide.backup-*.docx` is next to the file, names the newest one (decided here: by time, and then by the number in the name, `-1`, `-2`; `report_names_the_newest_backup_next_to_the_document`, `the_newest_backup_is_found_by_its_time_and_then_its_number`). The section and the JSON can list applied fixes (a report that was built with 2 fixes lists both and the backup name), for the later app, which applies fixes before it exports. |
| S6-04 | PAC: a place to attach or link the PAC report file, with the result | `report_has_a_pac_field` | unit | Any | Section 6 has labelled fields for the PAC report file and the result. **Implemented (M4c)**: labelled fields for the PAC report file (a file name or a link), the PAC version and the PAC result (a list: not checked yet, pass, fail). The JSON has `pac` with the three values, all null when the report is made. |
| S6-05 | Human checklist with a name and date for sign-off, the seven items of spec 02 | `report_checklist_items_and_signoff` | unit | Any | The seven items are there word for word. Each has a labelled checkbox. Name and date have labelled fields. **Implemented (M4c)**: the seven items are the constant `CHECKLIST`, and the test reads `docs/specs/02-pipeline.md` and checks that each item is there word for word (the spec's own line breaks aside). Each has a checkbox with a `label for`; none is ticked. Name (text) and Date (`type="date"`) are labelled. The JSON has the items with `done: false` and `name` and `date` as null. |
| S6-06 | The HTML report is itself accessible: headings, real tables with `th` and `scope`, `lang`, title, contrast | `report_html_is_accessible` | unit | A report with findings | `html` has `lang`. `title` is not empty. One `h1`, and no skipped level. Every data table has `th` with `scope`, and a caption. No table is used for layout. Every form control has a label. Every colour pair in the style sheet has a contrast of at least 4.5:1, checked with `contrast_ratio`. Pass or fail is never shown by colour alone. No link text is one of the vague phrases of LNK-001. **Implemented (M4c)** in `report_html_is_accessible` and `report_html_colours_have_enough_contrast`. The HTML is well-formed, so `roxmltree` reads it with no new crate. Checked: `lang="en"`, a title that holds the document name and the result, one `h1`, no skipped heading level, every table has a caption and `th` with `scope` (col or row), no nested or layout table, every `input` and `select` has a `label for`, no script and no `on*` attribute, no vague link text. Contrast: the test reads the style sheet and checks every rule that sets `color` against its `background` (or the white page) with `model::colour::contrast_ratio`: 4.5:1 or more for every pair, and 3:1 or more for the state borders. The result is also shown as a word. |
| S6-07 | Document text cannot break the report | `report_escapes_text_from_the_document` | unit | A finding whose snippet is `<script>alert(1)</script> & "x"` | The HTML shows the text; there is no `script` element. The JSON has the text unchanged. **Implemented (M4c)**: the snippet `<script>alert(1)</script> & "x"` and the document name `A <b> & "Guide".docx` appear as text, there is no `script` or `b` element, and the JSON has the text unchanged. Control characters are replaced (`report_escape_handles_control_characters`). |
| S6-08 | The JSON has the same results as the HTML | `report_json_matches_html` | unit | The same fixed results | The same findings (rule ID, message, location), counts, fixes, metadata changes, veraPDF failures and backup name appear in both. **Implemented (M4c)**: for a report with every part filled, the JSON values (counts, rule IDs, messages, how-to-fix text, snippets and headings, fixes, backup name, metadata changes, veraPDF clause and message, WCAG warnings, Word build, fields) are all in the text of the HTML. |
| S6-09 | The files are `Guide.accessibility.html` and `Guide.accessibility.json`, next to the PDF | `report_files_are_written_next_to_the_pdf` | CLI | `export` on a pass fixture; `export` on an error fixture (decided point 7) | Both files exist with these names. **Implemented (M4c)**: `report_files_are_written_next_to_the_pdf` (`tests/export_pipeline.rs`: a pass with fake stages and a name with dots and spaces, `My Guide v2.1.docx` gives `My Guide v2.1.pdf`, `.accessibility.html` and `.accessibility.json`), `cli_export_exit_1_on_errors` (the real program, an error fixture: the report files and no PDF) and `cli_export_exit_0_on_a_pass` (windows_word with verapdf: all three). |
| S6-10 | No outside resources | `report_loads_nothing_from_the_network` | unit | A report | No `src`, `href` to a network address, or `@import`; the style sheet is inside the file. **Implemented (M4c)**: the HTML has no `http:` or `https:` at all, no `@import`, no `url(`, no `link`, `script`, `img`, `iframe`, `object`, `embed`, `video`, `audio` or `source` element, no `src`, and every `href` starts with `#` (the section list). One `style` element holds the style sheet. |
| S6-11 | The report works with a screen reader | `report_read_with_a_screen_reader` | manual | A report from a failing fixture with several findings, opened in a browser | With NVDA and with Narrator: the seven sections can be reached from the headings list; each table is read with its column headers; the checklist and the sign-off fields can be used with the keyboard only; the pass or fail result is read. Done once for each release, with the screen reader and browser versions noted. **Not done.** A person has not read a report with NVDA and Narrator (no screen reader was available to the worker). See "Known gaps in M4c". |

### CLI: `fix` and `export` exit codes (spec 02, "Interfaces")

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| C-01 | `fix`: 0 when no error remains after the fixes (or nothing to fix) | `cli_fix_exit_0_when_no_error_remains` | CLI | `DOC-002-fail.docx`, fix confirmed | Exit 0. A backup exists. Re-running `check` gives exit 0. Status: implemented in `tests/cli_fix.rs`: `cli_fix_exit_0_when_no_error_remains`, and `cli_fix_exit_0_when_there_is_nothing_to_fix` (no backup). |
| C-02 | `fix`: 1 when errors remain after the fixes | `cli_fix_exit_1_when_errors_remain` | CLI | `HDG-002-fail.docx` (manual error); a document with one auto and one manual error | Exit 1. The auto fix was applied. The manual error is listed. Status: implemented: `cli_fix_exit_1_when_errors_remain` (both inputs). The answers come from stdin. |
| C-03 | `fix`: 2 for a tool failure | `cli_fix_exit_2_on_tool_failure` | CLI | A locked file; a file that is not a .docx; a missing file; a backup that cannot be written; wrong arguments | Exit 2. stderr says why. stdout is empty. The file is unchanged. Status: implemented: `cli_fix_exit_2_on_tool_failure` (locked file, not a .docx, missing file, wrong arguments), and, on Windows, `cli_fix_exit_2_when_the_backup_cannot_be_written` (folders with the names of the backups of the next 30 seconds, so that the real clock hits one). The questions of `fix` go to stderr, and the result goes to stdout only after the file is written, so stdout is empty on every exit 2. |
| C-04 | `fix` asks before each fix | `cli_fix_confirms_each_fix` | CLI | A document with 3 auto problems, with answers piped in: yes, no, yes | The first and the third are applied. The second is not. Status: implemented: `cli_fix_confirms_each_fix` (answers y, n, y). More: `cli_fix_applies_nothing_when_every_answer_is_no_or_the_input_ends`, `cli_fix_asks_again_for_an_answer_it_does_not_understand`, `cli_fix_shows_the_suggestion_but_never_fills_it_in`, `cli_fix_asks_for_alt_text_or_decorative`, `cli_fix_asks_before_marking_the_arrows_decorative`, `cli_fix_says_what_it_cannot_do_and_what_word_does` and `cli_fix_writes_the_text_for_the_picture_it_was_asked_about`. `adapdf fix` has no flags. |
| C-05 | `export`: 0 on a pass | `cli_export_exit_0_on_a_pass` | windows_word | `DOC-001-pass.docx` | Exit 0. `Guide.pdf` and the two report files exist. **Implemented (M4c)** as `cli_export_exit_0_on_a_pass` in `tests/export_verapdf.rs` (windows_word and verapdf): the real program, the real Word and the real veraPDF; stdout starts with "Guide.docx: Pass (0 errors, 0 warnings).", the four files are in the folder, the .docx bytes are the same, the PDF has `pdfuaid:part` (stage 4), the JSON says pass, veraPDF 1.28.2 and 0 failed rules, and the `WINWORD.EXE` processes are the same as before. `pipeline_exit_0_on_a_pass_and_writes_three_files` is the same with fakes. |
| C-06 | `export`: 1 for stage 1 errors, veraPDF failures or a PDF with no tags | `cli_export_exit_1_on_errors` | CLI | `HDG-002-fail.docx` (no PDF); a stage 5 fake that reports 1 failure; the stage 4 input `no-structtreeroot`, an untagged PDF (decided point 8) | Exit 1 in all three. **Implemented (M4c).** (1) `cli_export_exit_1_on_errors` (the real program): exit 1, a Fail report, no PDF. (2) `pipeline_exit_1_on_a_verapdf_failure`. (3) `pipeline_exit_1_for_an_untagged_pdf` (both `no-markinfo` and `no-structtreeroot`): exit 1, the PDF is left as Word wrote it (nothing appended) next to a Fail report that lists the problem (decided point 8). An untagged PDF has a verdict (Fail), so it is kept and replaces an older `Guide.pdf` (round 1: the test starts with an earlier PDF and reports in the folder). (2) and (3) call `pipeline::export_docx`, which the command calls with the real stages, here with fake stages: the real program cannot be made to give a veraPDF failure or an untagged PDF on demand. The real program on a link fixture gives exit 1 (S5-10). |
| C-07 | `export`: 2 for a tool failure (Word missing, timeout, veraPDF or Java missing) | `cli_export_exit_2_on_tool_failure` | CLI | The fakes of S3-16, S3-06 and S5-06 | Exit 2. stdout is empty. **Implemented (M4c).** The real program: `cli_export_exit_2_on_tool_failure` (a file that is not there, a file that is not a .docx, no file given, and the three veraPDF and Java cases of S5-06). With fakes: Word not installed (`pipeline_exit_2_when_word_is_missing`), timeout (`pipeline_exit_2_on_a_timeout`), Word rejects the file, locked PDF, veraPDF crash, timeout and bad output (`verapdf_crash_or_bad_output_is_exit_2`), a PDF that cannot be read (`a_pdf_that_cannot_be_read_is_exit_2`), an unreadable .docx and a report that cannot be written (`a_report_that_cannot_be_written_is_exit_2`). Each checks: exit 2, nothing on stdout, a message on stderr that names the file, and no report from this run. **Round 1:** the tests that fail after the export or before the PDF is in place (Word missing, timeout, Word rejects the file, locked PDF, veraPDF or Java missing, veraPDF crash, timeout and bad output, a PDF that cannot be read) start with an earlier `Guide.pdf`, `Guide.accessibility.html` and `Guide.accessibility.json` in the folder, and check on disk afterwards: the same four files, byte for byte, and no temporary file (`assert_earlier_outputs_untouched`). This includes the report that cannot be written (`a_report_that_cannot_be_written_is_exit_2`, see C-11); the one exception is a report that cannot be replaced after the PDF was moved (see "Known gaps in M4c"). With the real Word: S3-04, S3-06 and S3-19; the timeout and the Word that rejects the file (`tests/export_pipeline_word.rs`) also start with earlier outputs and check them. |
| C-08 | The current test that `export` is not in this version | `exit_code_2_for_wrong_usage` in `tests/cli.rs` | CLI | `adapdf export a.docx` | This test must be changed when `export` is built. **Done (M4c):** `exit_code_2_for_wrong_usage` now checks `adapdf export` with no file (exit 2) and with a file that is not there (exit 2, stdout empty, stderr names the file). |
| C-09 | `fix` offers the two link fixes and applies them (decided 2026-11-16) | `cli_fix_offers_both_link_fixes_and_applies_them` (`tests/fix_links.rs`) | CLI | A guide with three links that have no ScreenTip (a `w:hyperlink`, a complex field, a simple field) and a table of contents with `\h`; answers piped in: four times yes | stderr says "4 problem(s) the tool can fix" and shows the text of both fixes. stdout lists "Fixes applied (4)" with `[LNK-004] Added the ScreenTip "parking policy"` and `[LNK-005] Removed the \h switch`, and "No problems found." Exit 0. Every link has a ScreenTip, except the entries of the table of contents. | implemented |
| C-10 | `fix` says when a link fix cannot be done | `cli_fix_says_when_a_link_fix_cannot_be_done` (`tests/fix_links.rs`) | CLI | A guide with a link whose text is only spaces | stderr says "The tool cannot fix this now: The link has no text ..." and asks nothing. Nothing is written and there is no backup. Exit 1 (the error remains). | implemented |
| C-11 | `export` replaces `Guide.pdf` only after the new PDF has been checked (spec 02, "Inputs and outputs"; decided point 6; M4 review round 1) | `a_verapdf_failure_replaces_the_old_pdf_and_the_old_reports`, and the tool-failure tests of C-07 | CLI library (fake stages), and windows_word for the real Word | A folder with an earlier `Guide.pdf` and earlier reports | The order is: stage 1, the tools, the check that `Guide.pdf` is not locked, Word exports to a temporary file in the folder, stage 4 on it, stage 5 on it, both reports written to temporary files, then the move over `Guide.pdf`, then the move of the two reports. A tool failure (exit 2) at any step before the moves, including a report that cannot be written, removes the temporary files and leaves the folder as it was. When veraPDF fails (verdict Fail, exit 1) the new PDF replaces the old one and the Fail report replaces the old reports (`a_verapdf_failure_replaces_the_old_pdf_and_the_old_reports`; an untagged PDF also: C-06). Stage 1 blocked: a Fail report written the same way, no PDF; an older PDF stays (`a_blocked_export_writes_its_reports_without_leaving_temporary_files`). A report that cannot be written: `a_report_that_cannot_be_written_is_exit_2` uses a document name of 225 characters, so that the temporary name of the HTML report is over the limit of 255 characters for a file name and the final names are not; the run is exit 2 naming the HTML report, and the old PDF and the old reports are byte-identical and no file is new. A move that fails after the PDF move: `a_report_that_cannot_be_replaced_leaves_the_new_pdf_and_says_so` (the HTML name is a folder): exit 2 names the report and says the new PDF is in place; see "Known gaps in M4c". | implemented |

### No network (spec 01, constraints; AGENTS.md)

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| N-01 | The shipped program has no HTTP client | `no_http_client_in_the_dependency_graph` | unit | `Cargo.lock` (read as text) and the `windows` crate features in `Cargo.toml` | None of `reqwest`, `hyper`, `ureq`, `curl`, `attohttpc`, `isahc`, `surf`, `h2`, `native-tls`, `rustls`, `openssl` is a package. No `windows` feature for networking (`Win32_Networking`, `Win32_NetworkManagement`). Equivalent check with `cargo tree -e normal`. **Implemented (M4b)** in `tests/no_network.rs`: the lock file (all packages, also those only used for tests), `Cargo.toml`, and `cargo tree -e normal --locked --offline`. The list of crates has 17 names; a third test shows that the check finds a client when there is one. **Round 1:** `the_source_makes_no_socket` reads every `.rs` file of `src/` and fails on `std::net`, `core::net`, `TcpStream`, `TcpListener`, `UdpSocket` or `ToSocketAddrs` (a connection made with the standard library is in no crate of the graph). Checked by mutation: a line `use std::net::TcpStream;` in `src/lib.rs` fails it; the file was restored. It reads the source text, so it does not see a socket made through a Windows API call (the `windows` features for the network are checked in the first test). |
| N-02 | A full run works with the network off | `export_runs_with_the_network_disabled` | manual | A pass fixture, on a machine with the network adapters disabled | The same PDF and the same veraPDF result as with the network on. |
| N-03 | The app, the bundled Java and WebView2 make no connection | `no_connection_attempts_in_a_full_run` | manual | A Windows Firewall rule that blocks outbound traffic for the installed app exe (built as `adapdf-app.exe`; the installed file name comes from `productName` (`Adapdf`) in `tauri.conf.json` unless `mainBinaryName` is set, so M5b writes the real name here), the bundled `java.exe` and `msedgewebview2.exe` (the one that the app starts), with logging on, and a full export in the app | The run succeeds and the firewall log has no entry for those three programs. **Partly done (M5b), 2026-11-17.** No firewall rule was made (that needs admin rights, and a change to the machine). Instead `node app/scripts/axe-webview.mjs --installed <exe> --watch` ran the installed `adapdf-app.exe` and, about every 1.3 seconds (18 and 20 looks in the two runs), listed the process tree of the app (`app/scripts/look-at-process-tree.ps1`: `adapdf-app.exe`, 6 `msedgewebview2.exe`, 2 `java.exe`, 2 `conhost.exe`) with the TCP and UDP endpoints that those processes own (`netstat -ano`), during the check of `LST-002-fail.docx` and the real export of `DOC-001-pass.docx`. Result: 3 endpoints, all on this machine only (listening, or the debugging port that the script itself opened), 0 to another address; the script fails on one. This is sampling: a connection that opens and closes between two looks is missed, and a firewall log is not made. The firewall-log test stays planned. The installed file name is `adapdf-app.exe` (the product name is `Adapdf`; the program keeps the package name). |
| N-04 | Only made-up documents are in git (AGENTS.md) | `repo_has_only_made_up_documents` | unit | `git ls-files` | The tracked `.docx` and `.pdf` files are the catalogue, the PDF fixtures and the template files, and nothing else. **Checked by hand (M5b), 2026-11-17; the test is still planned.** `git ls-files`: 82 `.docx` files, all in `tests/fixtures`; no `.pdf`, `.exe`, `.zip`, `.jar`, `.dll` or `.msi`; nothing from `vendor/`, `target/` or `app/src-tauri/bundle-staging/` (Git ignores all three). The installer and the staging copy of `vendor/` are build output, not source. |
| N-05 | The web UI loads nothing from the network (spec 01, constraints) | `frontend_bundle_has_no_remote_urls` (`app/src-tauri/tests/frontend_bundle.rs`), with `the_remote_url_scan_finds_an_address`, `the_fonts_are_bundled` and `the_csp_allows_only_the_app_and_its_ipc` | Rust, after the front-end build (`pnpm build`) | The built front-end assets in `app/dist`, and `tauri.conf.json` | No `http://` or `https://` address in the built files (HTML, JavaScript, CSS) except the entries of an explicit list, each with its reason: XML namespace names (`http://www.w3.org/`), React's text for a minified error message (`https://react.dev/errors/`), a warning text of `prop-types` (`http://fb.me/use-check-prop-types`) and Tauri's own IPC origin (`http://ipc.localhost`). No protocol-relative `url(//`, `src="//"`, `href="//"` and no `@import`. `index.html` loads only `/assets/`. The style sheet names only IBM Plex Sans files that are in `dist/assets`. In the content security policy, `default-src`, `font-src`, `img-src`, `script-src` and `style-src` are `'self'` only, `connect-src` is `'self' ipc: http://ipc.localhost`, there is no `unsafe-inline`, `unsafe-eval`, `data:` or `blob:`, and `object-src` is `'none'`. The URL scan finds addresses written into the files; the policy also blocks a request that code makes at run time (`fetch()`), so both are needed. **Implemented (M5a).** Checked by mutation: a `<link href="https://fonts.googleapis.com/...">` added to `dist/index.html` fails the test (the file was restored). Also checked on the running app (see "How M5a builds the app"). |

Carbon's Sass can load IBM Plex from IBM's servers (`$use-akamai-cdn`) or from a
package path that only webpack understands (`~@ibm/plex`). M5a turns Carbon's
`@font-face` rules off and bundles the files itself: see "How M5a builds the app".

### App (spec 02, "Interfaces"; spec 01, technology)

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| A-01 | Findings are grouped by severity | `app_groups_findings_by_severity` | front-end (TypeScript) | The findings of `LST-002-fail.docx` (the view model, and the findings screen) | Errors first, then warnings, with counts. **Implemented (M5a)** as the `describe` block of that name in `app/src/viewModel.test.ts` (3 tests) and `the findings screen` in `app/src/App.test.tsx`: the headings "Errors (1)" and "Warnings (1)" in that order, with the rule, message, place and how to fix of each finding, and the words Error and Warning in the text, not only a colour. |
| A-02 | Fix and export buttons follow the state | `app_buttons_follow_the_state` | front-end (TypeScript) | The view model with no fix picked; with errors; with none | "Apply fixes" is off when nothing is picked. "Export" is off while errors remain. **Implemented (M5a)**: `app/src/viewModel.test.ts` and, on the page, `turns Apply fixes on when a fix is ticked` and `turns Export off while errors remain, and says why` (a disabled button has no key stop, so the page says why in text next to it) in `app/src/App.test.tsx`. Export is also off while the app is busy. |
| A-03 | The tool never fills in alt text or a title (AGENTS.md) | `app_never_prefills_typed_text` | front-end (TypeScript) | `DOC-001-fail.docx` and `IMG-001-fail.docx` | The input field is empty. The suggestion is shown as a separate hint. **Implemented (M5a)**: the view model makes no choice from a suggestion (`app/src/viewModel.test.ts`), the page has an empty field whose helper text (its accessible description) holds "Suggestion, not filled in: ...", and nothing is sent until the person types (`never fills in the title...`, `has an empty alt text field...` in `app/src/App.test.tsx`). The Rust side takes a fix as the rule and the position of a finding and the person's input, and never a text of its own (`offers_carry_the_suggestion_and_no_text`). |
| A-04 | The app is usable with a screen reader (spec 01: Carbon in WebView2) | `app_widgets_have_names_and_roles` | front-end (TypeScript) | Each screen, tested with Vitest and axe-core (`pnpm test`) | axe-core reports 0 violations for WCAG 2.1 A and AA on each screen. The keyboard focus order follows the reading order. **Implemented (M5a)** as one test for each screen state in `app/src/App.test.tsx` (`has no WCAG 2.1 A or AA violation` for: the first screen, the findings, the fix flow with every kind of fix, the export in progress, the export result, the sign-off form empty, with an error and filled in, and an earlier report), and tests of the Tab order (findings, fix flow, sign-off form). Another test (`app/src/test/axe.test.tsx`) shows that axe finds a button with no name and a picture with no alt text, so a pass means something. **The colour contrast rule is off** in jsdom (no layout, no canvas). It is measured in the real WebView2 by `app/scripts/axe-webview.mjs` (A-10: 0 violations on 14 screens). The focus goes to the heading of the screen after each action, and one live region (`role="status"`) carries the messages. |
| A-05 | Keyboard, screen reader and display settings | `app_manual_walkthrough` | manual | The app on Windows 11 | The whole flow (choose a file, see findings, apply a fix, export, open the report) works with the keyboard alone; Narrator and NVDA read each step; the app is usable at 200% scaling and in a high-contrast theme; dragging a file onto the window works. Before the walkthrough, run `pnpm tauri build --no-bundle` and `node scripts/axe-webview.mjs --vendor <vendor folder>` in `app/` (A-10): axe-core, with the colour contrast rule on, finds 0 violations on each screen in the real WebView2 (first screen, findings, fix flow, a blocked export, a Pass, the sign-off form empty, with an error, filled in and saved). A person then checks what a tool cannot: the high-contrast theme and 200% scaling. **Planned.** Not done by the worker: it needs a person, Narrator or NVDA, and the file dialog of Windows. M5a started the real program and called its commands through the real IPC (see "How M5a builds the app"); that is not this row. |
| A-06 | The report opens from the app | `app_opens_the_report` | manual | After an export | The report opens from a local file path in the default browser. **Partly built.** `only_the_html_of_a_report_can_be_opened` (Rust) tests the check that the command makes before it opens anything: only a file that ends with `.accessibility.html`, that exists, with an absolute path that is not a network path (`\server\share`). A network drive that is mapped to a letter (`Z:`) is not told from a local drive (that needs `GetDriveTypeW` and a new dependency); see "Known gaps in M5a". The page test `opens the report in the browser, and shows why when it does not open` checks the call and the message. That the default browser really opens the file is **planned** (manual). M5b did not try this on the installed app: `open_report` starts the default browser of the person's machine, and the script does not call it. It stays planned. |
| A-07 | The Tauri commands call the same pipeline as the CLI | `app_commands_match_the_cli` | unit (Rust); the `export` case with Word is windows_word and verapdf | `check`, `fix` and `export` commands on `LST-002-fail.docx`, `TXB-004-fail.docx` and a pass fixture | Each command returns the same findings, fix results and the same pass, errors or tool-failure result (exit 0, 1 or 2 in the CLI) as the matching CLI command on the same input. **Implemented (M5a)** in `app/src-tauri/src/tests.rs`. `app_commands_match_the_cli_check`: the findings, the counts and the exit result equal `check_file` and the JSON of `adapdf check --json`, on `LST-002-fail` (1 error, 1 warning, exit 1), `TXB-004-fail` (warning only, exit 0) and `DOC-001-pass` (exit 0); a missing file and a file that is not a .docx are an error (exit 2 in the CLI). `app_commands_match_the_cli`: for the fixes of 8 cases (`TXB-004`, `LST-002`, `TBL-001`, `DOC-001` with typed text, `DOC-002`, `IMG-001` marked decorative, `IMG-001` with typed alt text, and the yes to the question of `IMG-007`), the offers equal `fix::offers`, and the fix chosen in the app gives the same .docx bytes, the same backup name, the same findings and the same exit result as `fix::interactive::run` (the code of `adapdf fix`) fed with the matching answers. Also: nothing to fix, refusals, a file that Word has open. The export: `app_export_matches_the_cli_when_stage_1_finds_errors` and `app_export_gives_an_error_for_what_the_cli_exits_2` (the same report, the same console text and the same files as `export_docx`, with fake stages that fail the test if Word is started), and `app_export_with_word_matches_the_cli_export` (features `windows_word` and `verapdf`; it was `app_export_with_word_matches_the_cli`, which did not run the command line at all, and was renamed in review round 1). It exports the pass fixture `DOC-001-pass` twice with the real Word and veraPDF, once through the app command and once through `pipeline::export_docx` with `RealStages`, which is the code that `adapdf export` runs (`src/main.rs` calls it the same way), each in its own folder, and compares the exit code (0), the four files in each folder, the files that the run reports, the report (equal, except the time of the run and the seconds that Word took), and the console text (equal, except the folder and the time). It does not start the `adapdf` program, so the argument parsing of the command line is not covered (that is `tests/cli_export.rs`). Passed on 2026-11-17 with `ADAPDF_VENDOR` set (28 seconds; no `WINWORD.EXE` before or after). |
| A-08 | The sign-off is saved as evidence. A static HTML file cannot save what is typed or ticked without a script, so the app saves the human checklist and the sign-off (name, date, the state of each of the seven items) into the report evidence, the JSON and the HTML (spec 02, stage 6, point 7; "Known gaps in M4c") | `app_saves_the_signoff_in_the_report` | front-end (TypeScript) and unit (Rust, for the file write) | A report from `LST-002-fail.docx`; in the app: tick 5 of the 7 items, type a name, pick a date, save | After the sign-off in the app, the saved JSON has `checklist.name`, `checklist.date` and `done` true for the 5 items and false for the 2, and the saved HTML shows the same name, date and states (checked boxes, the typed values). Opening the report in the app again shows the same name, date and ticks. The two files are replaced one after the other, **the JSON first** (review round 1; before, the page was first). The JSON is the record that `read_report` and the app read, so a failure in between leaves the record signed and the page behind (it understates the sign-off); the old order left a page that claimed a sign-off that the record did not have. The page of the app says "The sign-off was not saved as asked." above the message. The report written by `export` is still unsigned, and the sign-off does not change the verdict, the findings or any other part of the file. **Implemented (M5a), with one change to the text of this row, decided by the orchestrator's brief: an empty name is refused** (it was "saved as it is"). A missing date is still saved as it is (`null`) and never filled in. A date that is given must be a real date (`yyyy-mm-dd`). Rust (`app/src-tauri/src/tests.rs`): `app_saves_the_signoff_in_the_report` (the JSON, the HTML equal to `html::render` of the signed report, 5 boxes `checked`, the typed values, the rest of the JSON unchanged, `read_report` gives the same, signing again replaces it), `the_signoff_never_touches_the_pdf_and_leaves_no_temporary_file`, `a_date_that_was_not_given_is_not_filled_in`, `an_empty_name_or_a_bad_request_is_refused_and_nothing_changes`, `a_failed_move_leaves_the_old_report_and_no_temporary_file` (the first move fails: the JSON is read-only), `a_half_saved_signoff_says_what_was_saved` (the second move fails: a folder has the name of the page; the JSON has the sign-off, the page is as it was, the message says "The sign-off is saved in ...json, but the page ...html cannot be replaced" and what to do, and saving again makes both files agree), `a_file_that_is_not_a_report_of_this_tool_is_refused`, `a_signoff_does_not_change_the_verdict`. The report library (root, `src/report/tests.rs`): `the_json_of_a_report_reads_back_to_the_same_report`, `the_html_shows_a_sign_off_that_the_report_holds`, and the accessibility and no-network checks of the HTML run on a signed report too. Page (`app/src/App.test.tsx`, `the sign-off`): the call with the name, date and 7 states; no call and a focused, labelled error without a name; the same name, date and ticks when an earlier report is opened. |
| A-09 | The page and the Rust commands agree on the data (spec 01: Tauri over the library; review round 1 of M5a). `app/src/types.ts` is written by hand, so a Rust type that changes must not go unseen | `the_samples_for_the_page_are_what_rust_serialises`, `the_samples_have_every_kind_of_value`, `the_arguments_of_the_page_are_the_parameters_of_the_commands` (Rust, `app/src-tauri/src/tests/contract.rs`); `app/src/test/contract.test.tsx` (Vitest) | unit (Rust) and front-end (TypeScript) | Real values of every type that the page receives, made by the same functions that the Tauri commands call: `CheckResult` and `Vec<OfferView>` for 7 documents (`LST-002-fail`, `DOC-001-fail`, `DOC-001-pass`, `IMG-001-fail`, `IMG-007-fail`, `TXT-004-fail`, and a link with no text for `LNK-004`), so offers are ready (auto and ask), at export, and unavailable with a reason; `ApplyResult` for each of them (applied with a backup, refused, at export); `ExportResult` for a Fail that was blocked by an error (`commands::export`, no Word), a Fail from veraPDF with a problem in the PDF, and a Pass (the report library with the veraPDF JSON of `tests/data/verapdf`; the whole pipeline is A-07); the error `{ message }` of a tool failure; `ReportView` unsigned and signed (from `saved_report()`), and the report file as `export` writes it. | Rust writes them to `app/src/test/contract/*.json` (the path of the test folder is written as `C:\Docs`). **The Rust test fails when a file differs from what Rust serialises now** (and when a file is left over or missing); `ADAPDF_REGENERATE_CONTRACT=1 cargo test -p adapdf-app --lib contract` writes them again, and the diff is reviewed. In Vitest: a small checker (`app/src/test/shape.ts`, no package) lists every field of each TypeScript type in `app/src/test/contract.ts` (the compiler says when a field is missing from the list) and checks the primitive type of each, with `null` only where the type says so; every file passes it, and a renamed, missing or retyped field is named in the problem. The findings screen, the fix flow (each kind of offer, the apply result, the refused fixes), the result screens (Pass, Fail from veraPDF, Fail with no PDF), the error message and the sign-off screens (unsigned, signed, after a save) are rendered from the files, and the text of the files is looked for on the screen. The other direction: `contract/calls.json` has the arguments that the page sends with each of the 7 commands; Vitest drives the page and checks that it sends exactly these, and the Rust test checks that the keys are the camelCase of the parameters of the `#[tauri::command]` functions in `shell.rs` (read as text; the `AppHandle` that Tauri fills in is left out) and that the command names are those in `generate_handler!`, and that Rust reads `choices` (confirm, text and decorative) and `signoff` from the file. **Implemented (M5a, review round 1). Checked by mutation:** `#[serde(rename = "atExport")]` on `ApplyResult.at_export` fails the Rust test (the file differs); after the files were written again, 5 Vitest tests fail with `at_export: the field is missing`, while the old page tests (hand-made data) still passed, which is why this row exists; the field was restored and the files written again (identical to before). `report_json` renamed `json_path` in `shell.rs` fails the Rust test (`["reportJson", "signoff"]` against `["jsonPath", "signoff"]`); `reportJson` sent as `reportJSON` in `api.ts` fails 2 page tests. All three were restored. Not covered: the shape of a value that the page does not read (the checker lists the fields of the types in `types.ts`, and the JSON has more), and the name of a command in `api.ts` that the page sends (the Vitest test sees it, because `calls.json` is keyed by command). |
| A-10 | The colour contrast of the app is measured in the real WebView2 (the Vitest tests cannot: jsdom has no layout) | `node app/scripts/axe-webview.mjs` (a script; it exits 1 on a violation) | script, on the built program | `pnpm tauri build --no-bundle` first. The script copies `adapdf-app.exe` into a scratch folder, joins `vendor/` next to it (a directory junction), starts it with WebView2's debugging port on a free local port and its own user data folder, drives the page with the DevTools protocol (Node's own `WebSocket`; no package), and runs axe-core (from `app/node_modules`) with the WCAG 2.1 A and AA rules and `color-contrast` on at each screen, then ends only the process tree that it started and removes the scratch folder. It first plants a light grey text on white and requires axe to find it. The commands are the real ones; only the Windows file chooser is replaced (the page's `fetch` for `plugin:dialog\|open` is answered with the next path). Screens: the first screen; the first screen with an error message; the findings, the fix flow and the findings after a fix (success message) of `LST-002-fail.docx`; the fix flow with a text field and a suggestion for `DOC-001-fail.docx` and `IMG-001-fail.docx`; the result of an export that was blocked by errors (a report that the real `export` command wrote for `DOC-001-fail.docx`, opened as an earlier report) with the empty sign-off form; the sign-off form with the error for a missing name; the findings with no problems, the export in progress, a real Pass (Word and veraPDF) with the empty sign-off form, the form filled in (5 ticks, a name, a date), and the result after the sign-off was saved. 0 violations on each, and 0 of the colour contrast rule. **Implemented (M5a, review round 1). Result of 2026-11-17: see "How M5a builds the app".** Not covered: a high-contrast theme and 200% scaling (A-05), hover and focus states (axe reads the page as it stands), a window that is not 1100 by 800. |

### Installer (spec 01, constraints and technology)

Most rows are manual, on a clean Windows 11 machine with a standard user. I-02
and I-08 are unit tests of `tauri.conf.json`, and I-11 has one for the bundle settings
(`app/src-tauri/tests/installer_config.rs`). M5b ran the manual rows that it could on
the build machine, a standard Windows 11 account (not a clean machine), with the silent
switch `/S`; see "How M5b builds the installer". The installer ships the desktop app only
(decision of the orchestrator, review round 1 of M5a): staff use the app, and the
`adapdf` command line stays a developer tool. So the rows below say "the app", and a
failure is a message in the app, not an exit code.

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| I-01 | No admin rights | `installer_asks_for_no_admin_rights` | manual | Run the installer as a standard user | No UAC prompt. Files are in `%LOCALAPPDATA%`. Nothing is written under `C:\Program Files` or to `HKLM`. **Done on the build machine (M5b), 2026-11-17.** The shell was not elevated (`whoami /groups`: Medium Mandatory Level); `Adapdf_0.1.0_x64-setup.exe /S` finished with exit code 0 in 9 seconds, so no prompt waited for a click. Files: `%LOCALAPPDATA%\Adapdf`. No `Adapdf` key under `HKLM\Software\Microsoft\Windows\CurrentVersion\Uninstall`, `HKLM\Software\Adapdf` or `HKLM\Software\WOW6432Node\Adapdf`; nothing in `C:\Program Files`, `C:\Program Files (x86)` or the all-users Start menu. Not seen by a person: the screens of an interactive install (I-09). |
| I-02 | The installer is set to install for the current user only | `installer_config_is_per_user` (`app/src-tauri/tests/installer_config.rs`) | unit | `tauri.conf.json` | `bundle.targets` is `["nsis"]` and `bundle.windows.nsis.installMode` is `currentUser`, so the install folder is under `%LOCALAPPDATA%` and no admin prompt is shown. **Implemented (M5b).** Checked by mutation (`both` fails it). |
| I-03 | Start menu entry | `installer_adds_a_start_menu_entry` | manual | After install | An entry for the current user exists and starts the app. **Done (M5b).** After the install, `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Adapdf.lnk` exists, and its target is `%LOCALAPPDATA%\Adapdf\adapdf-app.exe` (read with `WScript.Shell`; 2 installs). The shortcut was not clicked: the script started the same exe by its path. |
| I-04 | Uninstall removes the files | `installer_uninstall_removes_files` | manual | Uninstall from Settings | The install folder and the Start menu entry are gone. Reports and backups next to the user's documents are not touched. **Done (M5b), 2026-11-17, 2 uninstalls** with `%LOCALAPPDATA%\Adapdf\uninstall.exe /S`: the folder `%LOCALAPPDATA%\Adapdf`, `Adapdf.lnk` and the `HKCU\...\Uninstall\Adapdf` entry are gone. Left behind, outside the repository (not deleted): `HKCU\Software\Adapdf\Adapdf` (one value, the install folder; it was there **before** the first install of the test, so an older install left it, and the uninstaller does not remove it), and `%LOCALAPPDATA%\com.adapdf.desktop` (WebView2's data, 48,572,325 bytes before the test and after it; it is from earlier runs of the app, and the test runs used a scratch data folder, so they did not change it). See "Known gaps in M5b". |
| I-05 | The bundled veraPDF and Java run on a machine with no Java, in the installed app | `installer_runs_verapdf_without_java` | manual | The installed app on a machine (or Windows account, or folder tree) that has **no source tree of this project** and no Java: `where java` finds nothing and `JAVA_HOME` is unset; then choose `DOC-001-pass.docx` in the app and press Export PDF | The export runs veraPDF from the bundle (the result says Pass and names the veraPDF version). A second run with another Java first on the PATH still uses the bundled one. A third run with `ADAPDF_VENDOR` set to another folder still uses the bundled one (I-10). **Done on the build machine (M5b), 2026-11-17** (the proof of the bundled Java that the plan of M5b asks for; this machine has Java and a source tree, so the run of the clean machine is still planned): with `--no-java` the script took `JAVA_HOME` and the Eclipse Adoptium folder out of the environment of the started app (`where java` there: nothing) and the real export of `DOC-001-pass.docx` gave Pass with veraPDF 1.28.2. The `java.exe` that the app started (seen twice in each of 2 runs, parent: the app) was `%LOCALAPPDATA%\Adapdf\vendor\jre\bin\java.exe` with the class path `...\Adapdf\vendor\verapdf\bin\*`; the second one ran `--profile ...\vendor\verapdf\profiles\veraPDF-validation-profiles-rel-1.28\PDF_UA\WCAG-2-2-Machine.xml`, so the WCAG profile is in the installed copy and runs. The program names `java.exe` by its full path and never uses the PATH. Not done: a run with another Java first on the PATH. The run must be without the source tree: a debug build also looks for `vendor/` at the path of the source tree that it was built from (`CARGO_MANIFEST_DIR`), and would find the tools there and hide a `vendor/` that the installer did not place (I-11). A release build does not look there (review round 1, `a_release_build_looks_only_next_to_the_program`), but the machine without the source tree is still the proof. |
| I-06 | Awkward user names | `installer_with_awkward_user_name` | manual | A user name with a space and a non-ASCII letter | Install, run and uninstall work. |
| I-07 | Word is missing | `app_without_word` | manual | The installed app on a machine with no Word | Choosing a document works and shows its findings. Pressing Export PDF on a document with no errors gives an error message in the app ("The export did not finish.", with the reason that names Word), the page goes back to the findings, and no PDF and no report is written. (The same failure is exit 2 on the command line, a developer tool that the installer does not ship: A-07.) **Not done (M5b):** the build machine has Word. (The M5b task called the proof of the bundled Java "I-07"; in this plan that is I-05.) |
| I-08 | The installer does not download WebView2 (Windows 11 has it; spec 01, technology) | `installer_skips_webview2_download` (`app/src-tauri/tests/installer_config.rs`) | unit | `tauri.conf.json` | `bundle.windows.webviewInstallMode` is `{ "type": "skip" }`. Any other mode can download WebView2, which is a network call. **Implemented (M5b).** Checked by mutation (`downloadBootstrapper` fails it). |
| I-09 | The install guide works for a new user (spec 01, phase 5) | `installer_guide_followed_without_help` | manual | The install guide, on a clean Windows 11 user account with no admin rights. A person who did not build the app follows it. | The person installs, runs a check and uninstalls without help. Any help given is a fail. **Planned (M5b built the guide, `docs/install-guide.md`; a person who did not build the app must still follow it on a clean account).** |
| I-10 | The installed app uses its own bundled `vendor/` and ignores `ADAPDF_VENDOR`, which is a developer override (decided in M4 review round 1) | `a_release_build_ignores_adapdf_vendor` and `a_release_build_looks_only_next_to_the_program` (`src/verapdf/mod.rs`); manual run of the release program | unit and manual | `Tool::locate_with` with a value and the kind of build, and `vendor_folders`; then the release app, `target/release/adapdf-app.exe` with `vendor/` next to it, started with `ADAPDF_VENDOR=C:/nowhere`, and a Pass export of `DOC-001-pass.docx` | Unit: with a release build (`debug_build` false) the value is ignored and the result equals the result with no value; a debug build uses exactly that folder; an empty value is not a folder; a release build tries only the folder next to the program (the source tree is a place for debug builds only). Manual, 2026-11-17 with Word 16.0.20527: (1) before review round 1, the release `adapdf.exe export` ignored the variable, exit 0 (it found the tools through the source tree path, which is now removed); (2) after it, `app/scripts/axe-webview.mjs` started a copy of the release app in a scratch folder with `vendor/` joined next to it and `ADAPDF_VENDOR=C:/nowhere` in the environment, and the real export of `DOC-001-pass.docx` gave Pass (see "How M5a builds the app"). The installed app (M5b) repeats it with the installer's own layout: **done, 2026-11-17**, the installed `adapdf-app.exe` started with `ADAPDF_VENDOR=C:/nowhere` and with no `vendor/` joined by the script, a real export of `DOC-001-pass.docx` gave Pass (Word 16.0, veraPDF 1.28.2), 2 runs. | the unit parts, the manual run on a release build and the run of the installed app are done |
| I-11 | `vendor/` is placed next to the app exe by the installer, and the program finds it there (decided in review round 1: the installer ships the desktop app only, not the `adapdf` command line, which stays a developer tool) | `installer_places_vendor_next_to_the_exe` | unit (`tauri.conf.json`) and manual | M5b: `bundle.resources` in `tauri.conf.json` (or the mechanism that M5b chooses) maps the `vendor` folder to `vendor` next to the program. Manual: install, then look in the install folder | Unit: `bundle.resources` has the `vendor` mapping. Manual: after the install, `jre\bin\java.exe` and `verapdf\bin` are in a `vendor` folder next to the app exe (`Tool::locate` looks only there in a release build), and the export of I-05 works. **Implemented (M5b).** Unit: `installer_places_vendor_next_to_the_exe` reads `app/src-tauri/tauri.bundle.conf.json` (`bundle-staging/vendor/` goes to `vendor/`; the licence files go to `THIRD-PARTY.md` and `licenses/`), requires that `tauri.conf.json` has no `resources` (so that `cargo test -p adapdf-app` needs no `vendor/`: the bundle settings are added only by `scripts/build-installer.ps1`, with `--config`), and that the installer has no licence page; `installer_config_has_a_neutral_name_and_no_machine_paths` requires the name and publisher `Adapdf` and no drive letter, rooted or backslash path in either file (checked by mutation: `resources/`, and a path in the publisher). Manual: the installed folder has `adapdf-app.exe`, `uninstall.exe`, `THIRD-PARTY.md`, `licenses\IBM-Plex-OFL-1.1.txt` and `vendor\` with `jre\bin\java.exe` and `verapdf\bin`; Tauri puts a resource at its target path next to the exe, so `Tool::locate` needed no change. |

### Template acceptance (spec 04, "Acceptance")

The template and its files do not exist yet. Tests marked unit read the files
in `template/`.

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| T-01 | Acceptance 1: `sample-all-styles.docx` uses every style, callout, Quick Part, a Quick Table, a captioned screenshot, a TOC, links, a 3-level heading structure and both list types | `template_sample_uses_everything` | unit | `template/sample-all-styles.docx` and the .dotx | Each style in the spec 04 tables (Title, Heading 1 to 4, Normal, List Bullet, List Bullet 2, List Number, List Number 2, List Continue, Caption, TOC 1 to 3, Note, Tip, Important, Warning; UI Element, Keyboard Key, Hyperlink, Strong; "Adapdf Table") is used. The glossary part of the .dotx holds the four callouts and the Quick Table. Each callout starts with a bold label. There is a `TOC` field, a link, Heading 1 to 3, a caption under a picture that has alt text, and a table with `w:tblHeader`. |
| T-02 | Acceptance 1, by hand | `template_quick_parts_in_word` | manual | Insert each Quick Part and the Quick Table in a new document made from the template | Each gives the styled paragraph with its label, or the table with the header row marked. |
| T-03 | Acceptance 2: the pre-check reports 0 errors and 0 warnings (DOC-004 aside) | `template_sample_has_no_findings` | unit | `sample-all-styles.docx` | No finding except DOC-004. |
| T-04 | DOC-004 recognises a document made from the template | `template_made_document_is_recognised_by_doc_004` | unit | A document made in Word with File > New > Personal, saved as a fixture made for testing (not a city document) | DOC-004 does not fire. This closes the known gap in M1 and the "aside" in acceptance 2. |
| T-05 | Page and text settings (spec 04: Letter, 1-inch margins, one column, 12 pt body, 1.15 spacing, left aligned, en-US) | `template_page_and_text_settings` | unit | The .dotx | The values are as listed. A new document has DOC-001 (empty title) and no DOC-002. |
| T-06 | Header has a `TITLE` field and footer has page fields; no text boxes on the cover | `template_header_footer_and_cover` | unit | The sample | TXT-003 and TXB-001 give nothing. |
| T-07 | Acceptance 3: its PDF has 0 veraPDF failures and 0 PAC errors | `template_sample_pdf_passes` | veraPDF with windows_word, then manual | `sample-all-styles.docx` exported by the pipeline | 0 veraPDF failures. A person runs PAC and keeps the report: 0 errors. |
| T-08 | Acceptance 4: the Author Quick Guide passes the same checks, and is 2 to 3 pages | `template_quick_guide_passes` | unit, veraPDF, manual | `Adapdf Author Quick Guide.docx` | T-03 and T-07 hold for it. The PDF has 2 or 3 pages. |
| T-09 | Acceptance 5: all colour pairs in the changelog pass the ratios | `template_changelog_contrast_pairs` | unit | `template/CHANGELOG.md` (the table format is set when the template is made) | The ratio of each pair, recomputed with `contrast_ratio`, equals the listed one to 2 decimals. Body, link and callout text have at least 4.5:1. Large headings have at least 3:1. Callout borders have at least 3:1 on white. Every colour used in the .dotx is in the changelog. |
| T-10 | No restriction on formatting by default (spec 04) | `template_has_no_formatting_restriction` | unit | The .dotx | `word/settings.xml` has no `w:documentProtection`. |

### Phase 0 (spec 01, phases; spec 03, notes)

Phase 0 needs at least 5 real guides in `samples/`. Real guides are not put in
git. The result of each row is written in this plan, with the date.

**How a heuristic rule is measured.** A person lists every finding of the rule
on the sample guides and marks each as true or false (a false positive, FP).
The person also reads the guides for the cases the rule should have found and
did not (false negatives, FN). The FP rate is FP divided by the findings of
the rule. With fewer than 10 findings for a rule, every finding is read and
the reason is recorded, and the rate is not used. The numbers below are
proposals; the product owner confirms them before Phase 0.

| Rule | Measure on the sample guides | What keeps or changes the severity |
|---|---|---|
| DOC-003 (warning) | FP: reported text that is not a foreign-language passage, for example Greek letters in units or formulas. FN: Spanish or other Latin-script passages (not detectable by the rule). | Stays warning. If FN are common, the gap goes into the spec: languages other than English are out of scope for v1. If FP is above 30%, change the rule (for example, ignore runs with fewer than 3 letters of a symbol font). |
| HDG-003 (warning, auto fix) | FP: short bold or large lines that are not headings (callout labels, captions, bold list lead-ins, signature lines). FN: fake headings of more than 12 words, or made with colour, underline or capitals only. | Stays warning. The fix changes the document, so if FP is above 20% add exemptions (for example, a line that ends with a colon) before the fix is switched on. |
| IMG-002 (warning) | FP: alt text that the rule calls a placeholder but a person finds meaningful. FN: poor alt text that the rule misses ("final", "image of image", a file name with spaces). | Stays warning. If FP is above 20%, shorten the placeholder lists. |
| IMG-005 (warning) | FP: alt text equal to the caption that is acceptable. FN: captions typed in Normal style (the rule needs the Caption style). Count the guides that use the Caption style at all. | Stays warning. If most guides use no Caption style, the rule misses most cases, and widening the detection is a decision. |
| IMG-007 (error, ask fix) | FP: shapes reported that are not annotations of that picture (a divider line or a box next to a picture in an adjacent paragraph). FN: annotation shapes anchored more than 1 paragraph from the picture. Record the paragraph distance of each true annotation. | Demote to warning if FP is more than 20% of the findings (one click clears a false alarm, so the limit is higher than for LST-001). If any true annotation is more than 1 paragraph away, change `ANNOTATION_MAX_PARAGRAPH_DISTANCE`. Otherwise stays error. |
| LST-001 (error, auto fix) | FP: paragraphs that start like a list item but are not ("A. Smith wrote ..."). FN: other markers (roman numerals, `(1)`, `Step 1:`, dashes). | Demote to warning if FP is more than 10% of the findings, or if 2 or more different FP patterns can only be cleared by rewording correct text. Otherwise stays error. |
| TBL-004 (warning) | FP: real data tables with one row or with many empty cells. FN: layout tables with 2 columns of text. | Stays warning. If FP is above 30%, change the "empty cells" threshold (`LAYOUT_EMPTY_CELLS_PERCENT`). |
| CLR-001 (error), the automatic-colour rule | First, the grey-ramp test (next table). Then: FP: findings that a person measuring the PDF does not agree with (text over pictures, shapes, gradients). FN: low-contrast text that the tool missed (table styles, headers, footers). | Demote to warning if FP is more than 10% of the findings. If the grey-ramp test fails, change the rule first. |
| CLR-002 (warning) | FP: coloured text that has other cues the rule does not see (an icon, a label in another run). FN: colour as the only cue in shading or highlight. | Stays warning. |
| TXT-003 (warning) | FP: header and footer words that are fine (department name, version, "Confidential"). List the words and how often each appears. | Stays warning. If the same words appear in most guides, add them to the allowed list (a decision). |

| # | Phase 0 item | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| P0-01 | The automatic-colour rule matches Word (CLR-001; `src/model/colour.rs` says it is an approximation) | `clr_001_auto_colour_matches_word` | windows_word | A fixture with automatic-colour text on shading from #000000 to #FFFFFF in 17 steps, exported by Word; the text colour is read from the PDF content stream | For each step, the colour the model predicts (black or white) equals the colour in the PDF, except for the one step each side of the switch. If not, `AUTO_DARK_LUMA` or the rule changes. |
| P0-02 | Word export settings of spec 02 on the users' Word build (16.0, Microsoft 365) | `phase0_export_settings` | manual | One guide, exported with each parameter changed in turn: `DocStructureTags`, `CreateBookmarks`, `IncludeDocProps`, `OptimizeFor`, `UseISO19005_1`, `BitmapMissingFonts` | A table of the PAC and veraPDF results for each setting. Spec 02 is confirmed or changed. The Word build number is recorded. |
| P0-03 | How Word tags a picture grouped with shapes, a shape marked decorative, and an unmarked shape (spec 03 notes) | `phase0_grouped_picture_tagging` | manual | Three small documents made for the test: (a) a group of picture and arrows with alt text, in line; (b) a picture with arrows marked decorative; (c) a picture with unmarked arrows | The tags are read in PAC. The result says for each case: one Figure or several, and which shapes become artifacts. Specs 03 and 04 are updated. |
| P0-04 | Text in the shapes of a group or canvas (spec 04, "Flowcharts") | `phase0_group_text_tagging` | manual | A grouped flowchart with text, exported | Whether the text is in the tags and in what order. TXB-001's message for groups is confirmed or changed. |
| P0-05 | The Title style exports as `Title` or `P` (spec 04) | `phase0_title_tag` | manual | A document with a Title paragraph | If `P`, spec 04 says to use Heading 1 for the guide title. |
| P0-06 | Equations and embedded objects (IMG-006, spec 03) | `phase0_equation_and_ole_export` | manual | A document with one equation and one embedded object | Whether each is tagged with alt text. The severity of IMG-006 is confirmed. |
| P0-07 | The first heading when it is not Heading 1 (HDG-002; "How M1 reads the rules") | `phase0_first_heading_level` | manual | A document whose first heading is Heading 2 | Whether PAC or veraPDF reports it. HDG-002 keeps or drops that case. |
| P0-08 | PAC is the checker of record (spec 01, Q1) | `phase0_pac_checker_of_record` | manual | Each sample PDF and each acceptance PDF, checked in PAC (version noted) | The PAC report is saved with the PDF. 0 errors is the pass result. A table shows the findings where veraPDF and PAC differ. The city's ADA coordinator confirms in writing that the PAC report is acceptable as evidence. |
| P0-09 | Each error comes from the source or from Word's export (spec 01, Phase 0) | `phase0_error_origin_table` | manual | Every error on the sample guides after fixes in Word | A table: error, source or Word export, the pre-check rule that should have found it (or "missing rule"). |
| P0-10 | Does PAC also fail a link with no `/Contents`, and a table of contents with link tags (LNK-004, LNK-005; decided points 10 and 11) | `phase0_link_description_in_pac` | manual | Four PDFs from Word: (a) a link with no ScreenTip, (b) the same link with a ScreenTip, (c) a table of contents with `\h`, (d) the same table without `\h` (Word 16.0.20527 gives (a) and (c) 2 veraPDF failures and (b) and (d) none; (d) still has one `Link` tag for each entry, with no link annotation) | The PAC result for each of the four is written in this plan. If PAC does not fail (a) and (c), LNK-004 and LNK-005 may become warnings (the product owner decides). If PAC fails (d) because of the `Link` tags without an annotation, the table of contents needs another way (for example `\n`, no page numbers, or a table made by hand). |

## How M1 reads the rules

Spec 03 gives each check in one line. These are the decisions M1 made where
the line leaves room. Change the code and the tests together if the spec
changes.

- **Paragraphs.** Rules look at body paragraphs in reading order, including
  paragraphs in table cells. Text in text boxes is not in that list. The
  paragraph index counts from 0 in the data and from 1 in the text report.
- **Heading.** A paragraph in a built-in Heading 1 to Heading 9 style (by the
  style's name) that has some text. Title is not a heading. An empty heading
  is not counted as a heading by HDG-001 and HDG-002, because HDG-004
  reports it.
- **Formatting.** "Bold", "larger than body text" and the language are the
  effective values: document defaults, then the paragraph style and its base
  styles, then the character style, then direct formatting. For text in a table
  the table style comes after the document defaults and before the paragraph
  style. Word's rule that bold in a style and in a character style
  switch each other off is not applied: the last layer wins.
- **DOC-001.** The title is empty when it is missing or only spaces. The
  suggestion is the text of the first paragraph, in document order, that is
  in the Title style or is a Heading 1. It is a suggestion only; nothing
  applies it.
- **DOC-002.** The default language is the language of the default paragraph
  style (usually Normal) after document defaults. If neither sets one, the
  theme font language in `settings.xml` is used. The rule fires when all three
  are missing.
- **DOC-003.** M1 has no language detector. A run is reported when it has at
  least 3 letters in a non-Latin script (Greek, Cyrillic, Armenian,
  Devanagari, Thai, Chinese, Japanese, Korean, Hebrew, Arabic) and the
  matching language setting is missing or is English. Chinese, Japanese and
  Korean text uses the East Asian language setting; Hebrew and Arabic use the
  right-to-left setting; the other scripts use the main one. The rule runs
  only when the document default is English. Text in another Latin-script
  language, such as Spanish, is not found. One finding per paragraph.
- **DOC-004.** The document passes when the template named in
  `w:attachedTemplate` in `settings.xml` has the file name
  `Adapdf Training Guide.dotx` (case ignored). Otherwise it fires, and the
  message names the other template if there is one.
- **DOC-005.** Tracked changes are revision elements (insert, delete, move,
  and formatting changes) in the body, headers, footers, footnotes and
  endnotes. Comments are the comments in `word/comments.xml`. One finding
  covers both.
- **DOC-006.** The fonts "in use" are the fonts of runs that have text: the
  `ascii` font for ASCII characters and the `hAnsi` font for other characters.
  East Asian and right-to-left fonts are not checked. `fontTable.xml` is not
  used, because it also lists fonts that no text uses. A font can be embedded
  unless its OS/2 `fsType` says "restricted licence" or "bitmap embedding
  only". Fonts are looked up by family or full name, ignoring case, in the
  Windows fonts folder, the per-user fonts folder and the Microsoft 365 cloud
  fonts folder (`Microsoft\FontCache\4\CloudFonts`, where Aptos is). If one
  face of a family is restricted, the family is restricted. A font that is not
  installed is an error with its own message. One finding per font.
- **HDG-001.** More than 400 words in all body paragraphs (including tables)
  and no heading. Exactly 400 words passes.
- **HDG-002.** A heading more than one level below the heading before it. The
  first heading counts as a skip if it is not Heading 1. (The spec example
  is Heading 1 then Heading 3. The first-heading case follows PDF/UA, which
  wants the first heading to be H1; confirm this in Phase 0.)
- **HDG-003.** At most 12 words, not in a Heading style, and all text is bold
  or all text is larger than the body text (the size of the default
  paragraph style). To limit false alarms it skips table cells, list items,
  and the Title, Subtitle, Caption and TOC styles. The suggested level is one
  below the heading above it (at most Heading 4), or Heading 1 if there is
  none.
- **HDG-005.** Removed, and the number is not used again. It was "more than
  one Heading 1". Spec 04 uses several Heading 1 paragraphs, and WCAG 2.1 and
  PDF/UA-1 do not require a single Heading 1. The fixtures, tests and spec row
  are deleted, and the other rules keep their numbers.
- **HDG-006.** The style (not a built-in Heading style) has an outline level
  from 1 to 9, directly or through a base style; the level "body text" does
  not count. One finding per style, at the first paragraph that uses it. A style
  that no paragraph uses is not reported.
- **LST-001.** A paragraph that is not in a real list (no list numbering from
  the paragraph or its style) starts with a typed marker followed by a space
  or tab and then text. A marker is 1 to 3 digits, or one letter, then `.` or
  `)`; or one of these bullets: `•` `·` `▪` `●` `○` `◦` `■` `□` `‣` `⁃` `-`
  `*` and the Symbol-font bullet. Headings are skipped, because typed section
  numbers such as "1. Overview" are not lists. A symbol character in the
  Symbol font counts as a typed bullet.
- **LST-002.** A list is a run of paragraphs, one after the other, that are
  each a typed number or an automatically numbered item. Any other paragraph
  ends it. It fires when the list has both kinds. Bullets (typed or
  automatic) are not numbers. One finding per list, at its first typed number.

## How M2a reads the rules

Decisions for the IMG, TXB and TBL rules where spec 03 leaves room.

- **Alt text.** The description is `descr` on `wp:docPr` (DrawingML) or `alt`
  on the VML shape. The title (`title`, or `o:title`) is used only when the
  description is empty. Text of only spaces is no alt text. A drawing marked
  decorative (`adec:decorative` with `val="1"`) has no alt text to check: it
  passes IMG-001 with no alt text, and IMG-002, IMG-003 and IMG-005 skip it.
  VML has no decorative mark.
- **What is a drawing.** One `w:drawing`, `w:pict` or `w:object` in a run, in
  the part Word reads (`mc:Choice`, not `mc:Fallback`). The kind comes from
  the graphic data URI: picture, chart (also chartex), SmartArt (diagram),
  group (`wordprocessingGroup`), canvas (`wordprocessingCanvas`), other
  (shape). A drawing with no graphic data is a picture. VML is a group when
  its first shape is a `v:group` (its inner pictures are not read), a picture
  when it has `v:imagedata`, an embedded object when it has `o:OLEObject` (or
  is a `w:object`), and a shape otherwise. A group or canvas is one drawing:
  the alt text, the decorative mark and the floating state come from its own
  `wp:docPr` (or the attributes of the VML group), and the pictures and shapes
  inside are not drawings of their own.
- **IMG-001.** Applies to pictures, charts, shapes, groups, canvases and
  SmartArt. It does not apply to embedded objects (IMG-006), to a text box that
  has text (TXB-001 reports it), or to a shape that IMG-007 reports. A group or
  canvas whose shapes have text still needs its own alt text.
- **IMG-002.** Placeholder alt text is: only one of `image`, `picture`,
  `screenshot`, `graphic`, `photo`, `figure` (case, spaces and punctuation
  around it ignored); or text that ends in an image extension; or one of
  those words or a camera prefix (`img`, `dsc`, `dscn`, `pic`, `screen shot`)
  followed only by digits and the separators `space _ - . : ( ) #`, with at
  least one digit (`IMG_1234`, `Screenshot 2024-03-05 101530`, `Picture 1`).
  The lists are named constants in `rules/images.rs`. Text such as `Setup
  screen.png` is reported even though a person might mean it.
- **IMG-003.** Counts characters (not bytes) of the trimmed alt text. 250
  passes, 251 fails.
- **IMG-004.** Any floating (`wp:anchor`) object that needs a place in the
  reading order: picture, chart, SmartArt, group, canvas and shape. VML floats
  when its style has `position:absolute`. Not reported: a decorative object (an
  artifact has no reading order), a text box with text (TXB-001 reports it; a
  group with text is not a text box and is reported), and a shape that IMG-007
  reports. The fix is still `MakeImageInline`. Inline conversion is simple for
  a picture, chart or SmartArt. It is not checked for a group or canvas (Word
  may keep the layout of the shapes inside, or not), and for a single floating
  shape it moves the shape off its place. M3 must check this before it applies
  the fix to groups, canvases and shapes.
- **IMG-005.** The caption is the paragraph just before or just after the
  image, in a style named Caption. Alt text and caption are the same when they
  match after case, repeated spaces and punctuation at both ends are ignored.
  A label such as `Figure 1:` is not removed, so `Figure 1: Printer` and alt
  text `Printer` are not the same.
- **IMG-006.** One finding for the document, at the first object or equation,
  with the counts (spec 03 says "the document has"). Equations are `m:oMath`
  elements that are not inside another equation.
- **Text boxes (TXB-001).** M1 left paragraphs in `w:txbxContent` out of the
  body paragraph list, and out of the runs of the paragraph that holds the
  box, so no other rule sees them. M2a keeps that. The text is stored on the
  drawing (`Drawing::text`) and is what TXB-001 reads. A box with no text, or
  only spaces, is not reported. One finding for each box; the location is the
  paragraph that holds the box and the snippet is the box text. Pictures and
  tables inside a text box are not read. The text in the shapes of a group or
  canvas is joined and reported the same way, once for the group or canvas,
  with its own message. Shapes drawn on a screenshot are reported once for the
  screenshot (see "Annotated screenshots" below).
- **TXB-002.** A section with `w:cols w:num` above 1. One finding for the
  document, at the first paragraph of the first such section.
- **TXB-003.** More than 2 spaces in a row, or more than 1 tab in a row, in the
  paragraph text. The text joins the runs, so a row that crosses runs counts.
  Spaces and tabs are counted separately ("in a row" applies to both). A
  paragraph with only spaces is empty (TXB-004), not reported here. One
  finding for each paragraph.
- **TXB-004.** A paragraph is empty when it has no text but spaces, line
  breaks or tabs, and no drawing, equation, page or column break, section
  break, "page break before" or field. Paragraphs in a table are skipped, and
  they end a row of empty paragraphs. One finding for each row of 2 or more,
  at its first paragraph, with the count.
- **Tables.** Every `w:tbl` in the body (not in a text box) is read, nested
  ones too. The location is the first paragraph in the table, the heading
  above it, and the text of the first cell. One finding for each table for
  every table rule, so a table with 30 rows that can break gives one TBL-006
  finding with a count.
- **TBL-001.** The first row has no `w:tblHeader` (or it is switched off). It
  applies to every table, including one-row and layout tables.
- **TBL-003.** Any cell with `w:gridSpan` above 1 or any `w:vMerge`.
- **TBL-004.** One row, or every row one grid column wide, or more than 50%
  of the cells with no text (`LAYOUT_EMPTY_CELLS_PERCENT`). Cells that
  continue a vertical merge are not counted. A cell with only a picture has
  no text.
- **TBL-005.** A row where every cell is empty (no text, picture, equation,
  break or field). A row with a cell that continues a vertical merge is not
  reported. The message lists the row numbers.
- **TBL-006.** A row has no `w:cantSplit` (or it is switched off).

## Known gaps in M3a

- **One Word build.** Everything in `windows_word` was run only on Word
  16.0.20527 (Microsoft 365), on one machine, with the person's own Word open.
  Another build may differ, most likely in table style details (W-04).
- **No timeout in `WordSession` itself.** Dropping the session quits Word and
  waits at most 30 seconds for the process to end. It does not end a Word that
  is stuck. How long Word needs to end was measured with a temporary line of
  code (not kept) that printed the wait, with the bound set to 120 seconds:
  3.1 to 4.9 seconds in 14 quits (3.7 to 4.1 seconds for sessions with no
  document open, and 3.7 seconds for the session of W-01 with 78 documents open), and 8.6 seconds once (the first
  run of W-04 after a build). The bound was 10 seconds, which a slow quit can
  pass; it is now 30 seconds. The time that Word needs does not depend clearly
  on the number of documents. The 5 minute timeout and the end of the tool's own process are
  stage 3 (S3-06 to S3-08, built in M4a: see "How M4a reads stage 3"). The session gives the process id for that
  (`WordSession::pid`); the tests use it only to end a hung test's own Word.
- **The process id comes from a probe document.** Word has no process id
  property, so the session adds one hidden empty document to its new Word, finds
  the process that owns its window, and closes it (see the doc comment of
  `WordSession`). Before it adds the probe, it checks that the object is a
  fresh automation Word (`Visible` is false and `Documents.Count` is 0), so a
  Word of the person that COM gave while an unrelated Word started at the same
  moment is not touched. The decisions are pure functions with unit tests
  (`word::tests::*`, 5 tests): a new process and a fresh Word is accepted; no
  new process, a visible Word, or a Word with a document is
  `WordError::SharedInstance`; several new processes are all kept; the window
  owner must be one of them (`SharedInstance` if not, `UnknownProcess` if the
  window has no process). The real case where COM gives the person's own Word
  did not occur and is not tested with Word; the checks that Word itself
  answers (`Visible`, `Documents.Count`, the window owner) run in every Word
  test, with the person's own Word open, and always pass for the new Word.
- **Password-protected files** are not tested (S3-17). `open_read_only` passes a
  fixed wrong password so that Word reports an error and does not ask in a
  dialog. Whether that works for every kind of protection is not checked.
- **A 0-byte file opens in Word** as an empty document. It is not an error at
  this level; stage 1 refuses it first.

## Known gaps in M2a

- **Charts, SmartArt and embedded objects have no parts.** The pictures in the
  IMG fixtures now point to a real PNG part, and Word opens them (W-01). Charts,
  SmartArt and embedded objects exist only in memory tests, with no part behind
  them, so Word has never been asked to open a document with one. Word does not
  report a picture that fails to load, so W-01 does not show that the PNG is
  drawn.
- **Fixtures are still not the same as Word files.** Shapes, groups and
  canvases are now written with `mc:AlternateContent`, and every `wp:docPr` id
  is unique. Still different: the VML copy is simplified and every VML shape id
  is the same; the VML copy of a canvas is a VML group, which is a guess; the
  root element has no `mc:Ignorable`.
- **Drawings inside text boxes** and in headers, footers, footnotes and
  endnotes are not read.
- **Horizontal lines** (VML `o:hr`) are shapes without alt text, so IMG-001
  reports them. Word does not report them. Not seen in a real guide yet.
- **IMG-006 does not say how Word exports** an equation or an object. Phase 0
  decides.
- **No false-positive data** for IMG-002, IMG-005, IMG-007 and TBL-004.
- **A callout text box next to a screenshot** is read as a shape drawn on the
  screenshot (it is floating and in a neighbouring paragraph), so it gets the
  screenshot message of TXB-001. The fix advice (put the text in the step
  text) still applies.
- **A group of only arrows** that is anchored next to a picture is not an
  annotation (groups are not shapes for IMG-007). It is one group: IMG-001
  needs its alt text, and IMG-004 reports it when it floats.
- **VML shapes cannot be marked decorative** in the file (there is no VML
  mark). A VML shape on a screenshot with no alt text is reported by IMG-007,
  and the M3 fix must handle it (for example by writing the alt text, or by
  asking the user to convert the file).
- **Word's tagging of groups and decorative shapes** is not checked (Phase 0).

## Annotated screenshots, groups and canvases

Decisions for IMG-007 and the changes to IMG-001, IMG-004 and TXB-001. Users
are city staff who draw arrows, boxes and lines over screenshots in Word, and
who make flowcharts from shapes. Change the code and the tests together if
spec 03 changes.

- **Screenshot.** A picture (inline or floating, DrawingML or VML, not inside
  a group) with one or more annotation shapes. An annotation shape is a
  floating (`wp:anchor`, or VML `position:absolute`) shape that is not a
  picture, chart, SmartArt, group or canvas. A text box is a shape.
- **Detection uses the anchor paragraph, not the position.** A shape belongs to
  a picture when it is anchored in the same paragraph, or in the paragraph
  just before or just after (`ANNOTATION_MAX_PARAGRAPH_DISTANCE` = 1). Page
  layout is not known without Word. A shape two paragraphs away is a shape on
  its own: IMG-001 and IMG-004 report it as before.
- **One owner.** A shape belongs to one picture: the one in its paragraph, else
  the one before, else the one after. A paragraph with two pictures is one
  screenshot. Each screenshot gives at most one IMG-007 and one TXB-001.
- **Already grouped.** A group is one drawing, so a picture and shapes in one
  group are not a screenshot. IMG-001 checks the alt text of the group, and
  IMG-007 does not fire. The tool does not look at what is inside the group.
- **Unresolved shape.** A shape with no text, no decorative mark and no alt
  text. If a screenshot has any, IMG-007 fires once (error, ask) at the
  paragraph of the picture, with the number of unresolved shapes. A shape with
  alt text is fine for IMG-007 (IMG-004 still reports it if it floats).
- **Suppression.** `rules::images::unresolved_annotations` lists the
  unresolved shapes. IMG-001 and IMG-004 skip them, so IMG-007 replaces both.
  Nothing else is hidden: the picture's own IMG-001 (no alt text), IMG-002,
  IMG-003 and IMG-005 are separate problems and still fire.
- **Shapes with text on a screenshot.** TXB-001 fires once for the screenshot
  (message: "Text typed in a shape on a screenshot is read out of order or not
  at all"), at the paragraph of the picture, with the joined text as the
  snippet. These shapes are not counted by IMG-007. A shape that is marked
  decorative is not reported, because the fix text offers that when the text
  repeats the step text. (Outside a screenshot, TXB-001 still reports a
  decorative text box.)
- **Groups and canvases.** One drawing each. Text in the shapes inside counts
  for TXB-001 (one finding for the group or canvas, with a message about
  saving the flowchart as a picture and writing its steps in the body).
- **IMG-004 and the fix.** The fix ID `MakeImageInline` is unchanged. See the
  IMG-004 note above.
- **Spec 03 says** (new note): Phase 0 must confirm how Word tags a grouped
  picture and shapes (one Figure or several), and how it tags a shape marked
  decorative.

## How M2b reads the rules

Decisions made where spec 03 leaves room. Change the code and the tests
together if the spec changes.

- **Links.** A link is a `w:hyperlink` element or a `HYPERLINK` field (simple
  or complex, in the body only). A hyperlink with `r:id` is external. One with
  `w:anchor`, or a field with `\l` and no file, is internal. A field that has
  a file and `\l` is external. The link text is the visible text. Links in
  headers, footers and footnotes are not read.
- **LNK-001.** Whole-text match against five phrases (`click here`, `here`,
  `link`, `more`, `read more`), ignoring case, white space and punctuation at
  both ends.
- **LNK-002.** The trimmed text starts with `http://`, `https://` or `www.`
  (any case), has no spaces, and has more than 40 characters.
- **LNK-003.** The bookmark is looked up among all `w:bookmarkStart` names in
  `word/document.xml`, ignoring case, plus Word's `_top`. Hidden `_Toc`
  bookmarks count like any other, so a table of contents whose bookmarks are
  gone is reported.
- **LNK-004 (decided 2026-11-16, after the S5-10 result).** A link has no
  ScreenTip when its `w:tooltip` (on a `w:hyperlink`) or the argument of its
  `\o` switch (on a simple or complex `HYPERLINK` field) is missing, empty or
  only spaces. A link that is an entry of a table of contents (inside the result
  of a `TOC` field, or inside a "Table of Contents" content control) is not
  reported: LNK-005 covers it. Links with no text (a picture link, say) are
  reported; the fix refuses them. Severity error, so the rule blocks the export
  of a guide with such a link until the fix is applied (a person confirms it) or
  the author types the ScreenTips in Word.
- **LNK-005 (decided 2026-11-16).** A `TOC` field (also in a "Table of
  Contents" content control) whose instruction has the `\h` switch. The switch
  is found as a word, so `\h` inside a quoted word (`\t "Style \h,1"`) and `\hz`
  are not it. A table of contents that is only a `TOC` simple field
  (`w:fldSimple`) is not read; Word writes a complex field for it.
- **The fixes (`src/fix/links.rs`).** `AddScreenTip` copies the visible text of
  the link, with runs of spaces, tabs and line breaks made into one space, into
  `w:tooltip`, or inserts `\o "text"` after the last word of the field
  instruction (before its trailing spaces; in the last `w:instrText` that has
  text when the instruction is in pieces). The text is escaped for the field
  syntax (`\` as `\\`, `"` as `\"`) and then for XML. Both are splices of the
  raw bytes; the rest of the instruction keeps its form. An `xml:space="preserve"`
  is added to a `w:instrText` that has none. A link with no text, a field with an
  empty `\o` switch, and a field with an open quote mark are refused with a
  message. `RemoveTocLinks` deletes the `\h` switch and the spaces before it
  from the `w:instrText` that holds it (a switch split between two
  `w:instrText` elements is refused). The field tokenizer of the model now reads
  `\"` and `\\` inside a quoted word, so that a ScreenTip with a quote mark does
  not change what the model reads as the target of the link.
- **veraPDF mapping.** 7.18.1-2 and 7.18.5-2 both name LNK-004 as the pre-check
  rule (a table entry names one rule); their how-to-fix text names LNK-005 for
  the table of contents.
- **CLR-001.** Foreground is the effective run colour (styles, theme colour
  with tint and shade, then direct formatting). Background is, from the top:
  run highlight, run shading, paragraph shading, table cell or table shading,
  the page colour, white. The page colour counts only when
  `w:displayBackgroundShape` is on in `settings.xml`, because Word shows and
  exports it only then. Automatic text colour is an approximation of Word's own
  choice, which Microsoft does not document: white when the background has a
  luma (Rec. 601 on the encoded channels) below 0.5, and black otherwise. WCAG
  luminance is not used for this choice, because mid grey (#808080) has
  luminance 0.22, and Word is believed to draw black on it. That is the one
  case known; the rule is not confirmed against Word (see "Known gaps in
  M2b"). The contrast itself is the WCAG 2.1 formula. Large text is 18 pt or more, or 14 pt or more when bold.
  Thresholds are 4.5 and 3.0, with no rounding up (4.499 fails). Runs with no
  visible text are skipped. One finding for each pair of colours and size
  class in a paragraph. Pattern shading is not blended: a solid pattern draws
  its colour, other patterns draw the fill. Headers, footers and footnotes are
  not checked. Table styles are applied (see "Known gaps in M2b"): the fill
  and the text colour that a table style gives a cell count when the cell, the
  paragraph and the run set none. Shade and tint follow Word (a change of
  the HSL lightness; accent 1 `4472C4` with tint `99` is `8FAADC`).
- **CLR-002.** The run colour differs from the colour of the paragraph style,
  or of the table style for text in a table (automatic and black are the same). Other emphasis stops the finding: bold,
  italic or underline that differs from the paragraph style, a highlight or
  run shading, a character style, or a run inside a hyperlink. A coloured run
  in a bold heading is therefore still reported, because the heading is bold
  anyway. One finding for each colour in a paragraph. Text that is white on a
  dark fill is reported too, because the rule cannot tell a legibility choice
  from a meaning.
- **Body paragraph (TXT-001, TXT-002).** A paragraph in the document body
  (table cells included) that is not a heading, has no outline level, and is
  not in a Title, Subtitle, Caption or TOC style. Headers, footers, footnotes
  and endnotes are not body text.
- **TXT-001.** One finding for each paragraph, for its smallest run that has
  visible text. Smaller than 10 pt (20 half points) is reported; 10 pt passes.
- **TXT-002.** The effective alignment (after styles) is `both`,
  `distribute`, `thaiDistribute` or a kashida value. A paragraph with no text
  is skipped.
- **TXT-003.** Allowed: results of the fields PAGE, NUMPAGES, SECTIONPAGES,
  DATE, TIME, CREATEDATE, SAVEDATE, PRINTDATE and TITLE; the typed words
  "page" and "of"; typed numbers; and the document title, typed in any case.
  The title is the Title property, or the text of the first Title or Heading 1
  paragraph (the one DOC-001 suggests). Anything else is reported, including
  the result of other fields (AUTHOR, FILENAME) and typed dates. One finding
  for each header or footer part, with the extra words (lower case) as the
  snippet.
- **TXT-004.** A table of contents is a `TOC` field, or a content control
  whose building block gallery is "Table of Contents" (reported once when it
  holds a field). It is out of date when its field is marked `w:dirty`, or its
  entries (the TOC 1 to TOC 9 paragraphs) are not the headings. The headings
  are the built-in Heading styles in the levels of the `\o` switch (1 to 9
  when there is none), or the paragraphs with an outline level in those levels
  when the field has `\u`. Entry text is compared without the page number (the
  part after the last tab, unless `\n`), without a heading number, and with
  white space folded; order matters. With `\t` or `\b` the table may list other
  paragraphs or only part of the document, so only a missing heading is
  reported.

## Known gaps in M2b

- **The automatic text colour in CLR-001 is an approximation of Word.**
  Microsoft does not document the rule. The code (`auto_text_colour` in
  `src/model/colour.rs`) uses white text when the background luma is below 0.5
  and black text otherwise. It matches the one fact known at the time:
  mid grey (#808080) gets black text. It has not been compared with Word. It
  will be confirmed by `clr_001_auto_colour_matches_word` (row P0-01): export a
  fixture with automatic-colour text on shading from #000000 to #FFFFFF and
  compare each step with the colour in the PDF. Phase 0 repeats the check on
  the sample guides. Until then a finding on automatic text near the switch
  point (a mid-dark shading) can be wrong in either direction.
- **No false-positive data for CLR-002 and TXT-003.** Both are heuristics.
  Phase 0 measures them (see the Phase 0 table).
- **The table of contents fixtures are generated, not made by Word.** The TXT-004
  checks (the `TOC` field, the building-block content control, `w:dirty`, the
  entries, and the page numbers after the last tab) are written to match how
  Word is known to write a table of contents. They have not been checked
  against a document that Word made.
- **Links in headers, footers and footnotes are not read** (LNK-001 to
  LNK-004). A link there that has no ScreenTip gives the same veraPDF failure and
  no pre-check finding; the template has no links in the header or footer.
- **LNK-004 and LNK-005 follow veraPDF, not PAC.** Whether PAC also fails a link
  with no `/Contents` is not known; Phase 0 confirms it (P0 table). The rules
  also make Word's own table of contents unclickable in the PDF; readers use the
  PDF bookmarks and the headings.
- **The ScreenTip is the link text, with no limit on its length, and Word cuts
  a `w:tooltip` at 260 characters.** The tool copies the whole text. Word 16.0.20527
  (S2-15, a link text of 299 characters) reads a `w:tooltip` as its first 260
  characters, the length of its own ScreenTip box, and writes only those into the
  PDF as `/Contents` (checked by exporting a fixed guide: the annotation had 260
  characters). It reads the `\o` text of a `HYPERLINK` field whole (299
  characters, in the PDF too). veraPDF passes both. So a long link text gives a
  ScreenTip that is the start of the text, which is still the author's own words.
  The text of a link that is a bare address is copied as it is (LNK-002 still
  reports it).
- **Table styles are applied, with limits.** The model gives a cell the text
  colour, the other run properties, the paragraph properties and the fill of its
  table style (`src/model/table_styles.rs` says in what order). It was checked
  against Word 16.0.20527 cell by cell (W-04: 453 cells, all equal), and that
  check found three behaviours that are easy to get wrong: the first row
  and the last row win over the first column and the last column, the column
  bands win over the row bands, and a table with no band size has no bands. Not
  covered: the order among the corner cells other than the top left (a guess; they
  meet only in a table of one cell); table styles in text boxes, headers and
  footers (their text is not checked anyway); the table style's borders, margins
  and widths (they do not matter for colour); the effect of a document's
  compatibility mode (the test files have no compatibility settings); theme
  colours in table styles (read like other theme colours, not compared with
  Word); and a table style's paragraph shading (`w:pPr/w:shd`).
- **`w:clrSchemeMapping` is not read.** The theme colours use Word's default
  mapping. A document with another mapping can give a wrong colour in CLR-001.

## How M3b reads stage 2

Decisions made where spec 02, the decided points and the rows leave room. Change
the code and the tests together if the spec changes.

- **Splices on the original text.** All fixes of a run are worked out against
  the original document, and all splices go into the original text of each
  part. Paragraph numbers in the findings stay right when a fix deletes a
  paragraph. A splice inside a deletion is dropped (the deletion wins). The
  same splice twice counts once. Other overlaps stop the run (`FixError::Conflict`)
  with no change to the file. Several fixes for one `w:pPr` or one `w:trPr` are
  written as one edit, with the children in schema order
  (`new_properties_go_in_schema_order_in_a_busy_paragraph_and_row`). After the splices, the
  tool reads the new parts and the whole document again; if that fails, nothing
  is written.
- **Which finding.** A request names its finding. The engine runs the rule
  again and checks that the finding is still there (a finding from an older
  version of the file is refused). Findings can be the same in every field, for
  example two pictures in one paragraph that both have no alt text, so a
  request can carry the position of the finding among the findings of its rule,
  and `fix::offers` gives it. `adapdf fix` always uses it.
- **Offers.** `fix::offers` works out whether each fix can be applied, by
  running all the fixes in memory with placeholder text. A fix that the tool
  will not do is "not available", with the reason. `adapdf fix` shows the
  reason and does not ask.
- **Order of the checks before a write.** The `~$` file, then an exclusive open
  of the file (this also reads it), then the fixes in memory, then the backup
  (written, closed, read back and compared), then the new file in a temporary
  file in the same folder (`Guide.docx.adapdf-0.tmp`), then the move over the
  file. `adapdf fix` makes the first two checks before it asks anything, and
  again before it writes. The handle is let go just before the move, because
  Windows cannot replace an open file.
- **The lock file.** `fix::lock_file_names` gives the one name that Word writes,
  by the rule that S2-02 read from a real Word (see row S2-02): for `Guide.docx`
  it is `~$Guide.docx`, and for a base name of 7 or more characters the `~$`
  replaces the first 1 or 2 characters. A document that Word has open read-only
  has no `~$` file; the exclusive open of the file catches that (S2-04).
- **The backup.** Its name uses local time (`Timestamp::now`, from
  `GetLocalTime` on Windows; on other systems UTC, because the standard library
  has no local time). An existing file with the name makes the tool use
  `...-1.docx`, `...-2.docx` and so on. On Windows a folder with the name is an
  "access denied" error, not "already exists", and the tool stops (S2-06). The
  backup stays when a later step fails; it is a whole copy. A backup that cannot be read back, or reads back different, is deleted and the tool stops.
- **Nothing to write.** When no fix changes the file (all declined, all refused,
  or only `UpdateFields`), there is no backup and the file is not touched.
- **`SetTitle`.** The text goes in `dc:title`. A `dc:title` that is empty,
  self-closing or only spaces is filled; a missing one is added. A document with
  no `docProps/core.xml` is refused (the tool does not add parts).
- **`SetDefaultLanguage`.** `w:lang w:val` in `docDefaults/rPrDefault/rPr` of
  `word/styles.xml`, in schema order. Missing parent elements are added. The
  language is `FixOptions::default_language` (en-US); a value that is not a
  language tag is refused.
- **`ApplyHeadingStyle`.** The level comes from the suggestion ("Heading 2").
  The style that the document has with that name is used. If it has none, Word's
  definition of Heading 1 to 4 (Microsoft 365 template, without the link to a
  character style) is added at the end of `styles.xml` once, based on the default
  paragraph style. An existing `w:pStyle` is replaced.
- **`DeleteEmptyHeading`, `DeleteEmptyParagraphs`.** A paragraph is deleted only
  when it holds nothing but spaces, tabs and plain line breaks. A section break,
  a bookmark, a comment mark, a page or column break, a field, a drawing or any
  other element stops the deletion, with the reason. A paragraph that a table
  cell or the end of the document needs is kept. For a row of empty paragraphs
  the first stays and the others go (decided point 3); if some cannot go, they
  stay and the message says so.
- **`ApplyListStyle`.** The leading spaces, the marker and the spaces and tabs
  after it are removed from the runs (a run that held only that is removed, a
  run that holds more keeps its other content), and the style List Number or
  List Bullet is set. The style must exist in `styles.xml`. If it does not,
  the fix is refused: its numbering lives in `numbering.xml`, and adding it is
  not planned. An existing `w:pStyle` is replaced.
- **`SetAltText`.** The text goes in `descr` of `wp:docPr` (DrawingML) or `alt`
  of the first VML shape, and also in `alt` of the VML copy in `mc:Fallback`
  (decided point 2). A new attribute is written after the last one. "Mark as
  decorative" writes the extension that Word writes, expands `<wp:docPr/>` if
  needed, sets `val="0"` to `1`, and clears a `descr` that was there. It is
  refused for VML.
- **`MakeImageInline`.** Pictures, charts and SmartArt written as DrawingML
  only. `wp:anchor` becomes `wp:inline` (distances 0); only the extent, the
  effect extent, `wp:docPr`, `wp:cNvGraphicFramePr` and the graphic stay.
  Groups, canvases and single shapes, VML objects, and objects in
  `mc:AlternateContent` are not changed. The message says to do it in Word
  (S2-23).
- **`MarkAnnotationsDecorative`.** The user's confirmation is the input. The
  picture must have alt text (or be decorative); if not, the fix waits for
  IMG-001. DrawingML shapes only: if any unresolved shape is VML, the whole fix
  is refused, with the two ways out (alt text in Word, or convert the file).
  The tool does not ask for alt text for VML shapes (S2-25).
- **`MarkHeaderRow`, `DisallowRowBreak`, `DeleteEmptyRow`.** The table is the
  one of the finding (the n-th table with the problem, in table order, so a
  nested table is told apart from its outer table). Empty rows that start a
  merge are kept; a table with only empty rows is refused. If the header row
  itself is deleted, the next row does not become the header row.
- **`AlignLeft`.** `w:jc w:val="left"` in the paragraph's own `w:pPr` (decided
  point 9).
- **`UpdateFields`.** Listed as "done at export". Nothing is written.
- **`adapdf fix`.** Questions go to stderr and answers are read from stdin; the
  result (backup, fixes applied, fixes not applied, the new check) goes to
  stdout after the write. A yes is `y` or `yes`, a no is `n`, `no` or an empty
  line, and the end of the input is "no". Another answer is asked again. For an
  ask fix the text (or the choice) is typed; the suggestion is shown only as
  "Suggestion (not applied)". For alt text the first question is "Mark it as
  decorative?", and the second asks for the text. An empty text skips the fix.
  Text typed in the CLI is trimmed. There are no flags.

## Known gaps in M3b

- **Stage 2 and Word: what is checked and what is not.** S2-15 shows that Word
  opens the original and the fixed file for 23 cases, and that Word reads the
  title, heading and list styles, alt text, header row, row count, row split,
  alignment and the inline picture as the model does. It does not read the
  default language (DOC-002), the decorative mark and the alt text of shapes and
  groups, so those are checked only by "Word opens it".
- **The in-memory chart and SmartArt images are stand-ins.** Word cannot open
  even the original file of `Image::chart()` and `Image::smart_art()`
  ("Word experienced an error trying to open the file"), so the fixes for them
  (alt text, `MakeImageInline`) are not checked in Word. The fixtures in
  `tests/fixtures/` have no chart or SmartArt, so W-01 is not affected.
- **The lock-file rule was read on one Word build** (16.0.20527) and for `.docx`
  names. See the "Not tried" list in row S2-02.
- **The Word checks are on one machine and one build.** As for W-01 and W-04.
- **S2-23 is partly manual.** A person has not yet looked at a group, canvas or
  shape made inline in Word.
- **Only the body is fixed.** The model does not read headers, footers,
  footnotes and text boxes, so no fix changes them.
- **Not fixed by the tool**, and said so in the offer: groups, canvases, single
  shapes, VML objects and objects in `mc:AlternateContent` for IMG-004; VML
  shapes for IMG-007 and for "Mark as decorative"; list styles that the document
  does not define; a document with no `docProps/core.xml` for DOC-001.
- **The tool is not checked on a real guide.** The fixes run on made-up
  documents only (see "Known gaps in M1").
- **The file can be changed between the release of the exclusive handle and the
  move.** The gap is very short; the tool does not guard it.

## How M4b reads stages 4 and 5

Decisions that the specs leave open. A reader of the code needs them.

**Stage 4 (`src/pdfmeta`)**

- **API.** `pdfmeta::check_bytes(pdf, &SourceMetadata) -> Result<Outcome>` works
  out the fix and writes nothing. `Outcome::appended` holds only the bytes to put
  after the original; `Outcome::apply(original)` is the new file. `fix_file(path,
  &source)` does it for a file: it writes the new file to `<name>.adapdf-metadata.tmp`
  next to it and renames it over the original, and it does not touch the file
  when nothing is missing. `SourceMetadata::from_document` takes the .docx title
  and default language from the model. The `Report` has `changes` (field, plain
  description, old value, new value) and `issues` (error findings with a message
  and how to fix).
- **How the prefix property is guaranteed.** The tail is built by `pdfmeta::update`
  with its own writer (not `lopdf`'s `IncrementalDocument`), and only the tail is
  ever produced. It writes the changed objects, then a cross-reference section
  of the same kind as the file's last one (a table, or a stream when the last
  `startxref` points to a stream), then a trailer with `/Prev` set to the old
  `startxref`. `lopdf` is used to read, and to write each object's bytes. The
  `lopdf` writer was not used for the update because for a cross-reference
  stream it copies keys of the old stream dictionary (`/DecodeParms`, `/XRefStm`)
  that must not be repeated. Before it returns, `check_bytes` reads the new file
  again: it must open, have no cross-reference repair and have exactly one more
  cross-reference section, or the result is an error and nothing is written.
- **Untagged PDF: nothing is written.** If `/MarkInfo /Marked true` or
  `/StructTreeRoot` is missing, the result has the error findings and no change at
  all, even for metadata that is also missing. The export failed, and the file
  is not used. This is stricter than row S4-06.
- **Empty means missing.** A blank title or language in the PDF is replaced; a
  value that is there is never changed, except that a `pdfuaid:part` other than 1
  is an error and is left as it is. An empty title or language in the .docx gives
  an error finding and nothing is invented.
- **XMP.** A new `rdf:Description` is added before the end of `rdf:RDF` (with the
  `rdf:about` that the packet already uses); an empty `dc:title` element is
  replaced where it is; the rest of the packet is kept as text. A missing
  metadata stream is made. XMP that is not UTF-8 or not XML is refused (exit 2
  later), not guessed at.
- **Refused:** an encrypted PDF (typed error `Encrypted`), a PDF whose
  cross-reference table `lopdf` had to repair, and one whose last `startxref`
  does not agree with what `lopdf` read.
- **What Word writes** is not known yet (S4-11, Phase 0). The checks are written to leave
  anything that is there alone.

**Stage 5 (`src/verapdf`)**

- **API.** `verapdf::validate(&Tool, pdf) -> Result<Validation, VeraPdfError>`
  (`validate_with` takes a `Runner` and a timeout; the default is 180 seconds).
  `Tool::locate()` looks for `vendor/` next to the program and then at the root of
  the source tree; `Tool::new(java, verapdf_home)` and `Tool::in_vendor(dir)` give
  other paths. `Validation::passed()` is true only when the UA-1 run has no failed
  rule or check. `Validation::findings()` lists UA-1 errors and then WCAG
  warnings. Findings and results serialise to JSON.
- **The command.** The bundled `java.exe` is run directly with `-classpath
  <verapdf>/bin/*`, UTF-8 output flags and the main class
  `org.verapdf.apps.GreenfieldCliWrapper` (what `verapdf.bat` does, without
  `cmd.exe` and without `JAVA_HOME`), then `--flavour ua1 --format json <pdf>`.
  The window is hidden on Windows. veraPDF's exit codes are 0 (compliant), 1 (not
  compliant) and 7 (cannot read the PDF); the tool reads the JSON, not the code.
- **Offline.** veraPDF has no offline switch to set. The command line has no
  update check (the updater class `SoftwareUpdater` is in the GUI jar, which the
  command line does not call), and the PDF/UA-1 profile is inside the jar. This
  was read from the jar contents; no packet capture was done (N-02 and N-03 stay
  manual).
- **WCAG profile.** veraPDF 1.28.2 has no `--flavour` for WCAG, but its
  `profiles/*/PDF_UA/WCAG-2-2-Machine.xml` ("WCAG 2.2 machine validation profile")
  runs with `--profile <file> --format json`. The tool runs it when the file is
  there and shows its findings as warnings, without the ones that repeat a UA-1
  failure. veraPDF 1.30.4 does not install these profile files, which is why 1.28.2
  is pinned. WTPDF profiles (`wt1a`, `wt1r`) are not WCAG and are not run.
- **Findings.** `VeraFinding` has the specification, clause, test number, veraPDF's
  description, the number of failed checks, the context and message of the first
  failure, a plain message, how to fix, and the linked pre-check rule. The table
  (`verapdf::rules::TABLE`) has 29 rules of ISO 14289-1. A rule that is not in it
  keeps veraPDF's description. Findings are sorted by clause number and test
  number, because veraPDF's order changes from run to run.
- **Errors, not passes:** missing Java or veraPDF (named, checked before anything
  starts), a launch failure, a timeout (the process is killed), no output, output
  that is not veraPDF JSON, a job that did not end normally, and a PDF that
  veraPDF cannot read.

**The bundled tools.** `scripts/fetch-vendor.ps1` pins veraPDF 1.28.2 (SHA-256
`d1693a5f0bf0997180f6d97e8a5568b0cf39e27eee338d439c8adb58435c1e89`, from
`software.verapdf.org/releases/1.28/`) and Temurin JRE 21.0.13+9 x64 Windows
(SHA-256 `2332c0b4cff6bed722da3567c91d5e47568bcba863fa4945d0a8d6fc76158628`,
equal to the value in the publisher's `.sha256.txt`). veraPDF publishes only a PGP
signature, not a checksum file, so its value is of the file as downloaded over
HTTPS; the signature could not be checked because its key was not found on a key
server. The installer is IzPack; the script answers it with an XML file and runs
it with the bundled JRE. Licences: `THIRD-PARTY.md`.

## Known gaps in M4b

- **The fetch script was not run as a script.** The tool that runs commands for
  the worker did not allow `powershell`. The same steps (download, SHA-256 check,
  unzip, IzPack install with the same answer file, remove the uninstaller, move
  into `vendor/`) were done by hand, and the veraPDF tests pass on that result.
  The orchestrator then ran the script on a clean copy: it stopped after the
  first download at `Get-FileHash` ("not recognized": a module that did not load
  in that shell, probably a `PSModulePath` that Windows PowerShell 5.1 got from
  PowerShell 7). M4c replaced `Get-FileHash` with `System.Security.Cryptography.SHA256`
  and `Get-Content` and `Set-Content` with `System.IO.File` calls (the script
  already used `System.IO.Compression` and not `Expand-Archive`). The new SHA-256
  function was run on a test file and gives the value of `sha256sum`. The whole
  script has still not been run after the change; the orchestrator runs it again.
- **The pack names in the answer file are those of veraPDF 1.28.2.** Version 1.30.x
  has other pack names (`veraPDF GUI`, `veraPDF CLI`). Changing the pinned version
  means changing them.
- **Stage 4 is tested mostly on PDFs made by `lopdf`.** S4-11 (M4c) adds two real
  Word exports (a classic cross-reference table, and veraPDF reads the update).
  If Word writes a layout that `lopdf` reads differently (a hybrid file, a
  linearized file), the result is a typed error, not a bad file.
- **S5-10 and S5-11** (veraPDF on Word exports) were built in M4c. S5-10 failed (first run)
  for 4 fixtures and passes after LNK-004 and LNK-005: see "S5-10 result".
- **Exit codes** for stage 4 and 5 errors (2) and failures (1) were built in M4c
  (C-06, C-07).
- **The rule table is a start.** 29 of the 100 or so rules of the profile, chosen
  from the Word problems that are likely. The links to pre-check rules for 7.1-3
  (to TXB-001) and the table rules are best guesses until
  Phase 0 shows real failures.
- **No real WCAG-only warning has been seen.** On every test PDF the WCAG
  profile only repeated UA-1 failures, so the warnings are tested with the fake
  runner and the recorded WCAG JSON only.
- **No check that the network is not used.** See "Offline" above.
- **`Tool::locate` finds `vendor/` next to the program or in the source tree.**
  The installer layout (M5) must put it next to `adapdf.exe`.
- **N-04** (only made-up documents are tracked) is still planned.

## How M4a reads stage 3

`adapdf::export::export_pdf(docx, pdf, fonts, options)` is the whole of stage 3.
It has no CLI command and writes no report; the caller (`pipeline::export_docx`,
M4c) does that.

- **Order.** (1) Stage 1 on the .docx (`fonts` is for DOC-006). Any finding with
  severity error, DOC-005 included, gives `ExportError::Blocked { findings }`
  with all the findings of stage 1 (so that the report can say "Fail" and list
  them). Word is not started, nothing is written. A file that stage 1 cannot read
  is `ExportError::Check`. (2) The target is checked: an existing PDF must not be
  held by another program (an exclusive open, as stage 2 does for the .docx), and
  the tool makes an empty temporary file `~adapdf-<process>-<n>.pdf` (a short name, so long document names stay inside the path limit) in the
  target folder, which also shows that the folder takes new files
  (`OutputFolder` otherwise). (3) The Word part (below). (4) The temporary file
  must start with `%PDF-` (`NoPdf` otherwise), and it is moved over `Guide.pdf`
  (`PdfLocked` when that fails and the PDF exists). A failed export removes the
  temporary file and never touches the old PDF. (The command gives `export_pdf` a
  temporary path of its own, so that `Guide.pdf` is replaced only after stages 4
  and 5: see "How M4c reads stage 6 and export" and C-11.)
- **The Word part.** The tool's own hidden Word (`WordSession`) opens the .docx
  read-only, updates the fields (`Fields.Update` on the main text, then
  `TablesOfContents(i).Update` for each table of contents; it never saves),
  calls `ExportAsFixedFormat` by parameter name with the values of spec 02 (the
  constant `ExportParameters::SPEC`, which a unit test compares with the table in
  the spec), closes the document without saving, and the session quits Word.
  The parameters that the spec does not name keep Word's defaults (the whole
  document).
- **What is reported.** `ExportResult`: the PDF path, the stage 1 findings (only
  warnings, since an error refuses), `FieldUpdate` (how many fields, how many
  tables of contents, the number of the first field that Word could not update),
  the `ExportParameters` that were passed, Word's build and the duration (the
  start of Word to the PDF in place).
- **Threads.** A `WordSession` is not `Send` (COM, single-threaded apartment), so
  the whole Word part runs on one worker thread that makes its own session. The
  caller waits on a channel with a deadline. The worker sends the id of its Word
  process as soon as it has it, and the result at the end, after the session is
  dropped (so Word has quit when the caller gets the result). A panic on the
  worker unwinds through the session (which quits Word) and the caller gets
  `WorkerStopped`.
- **The timeout (5 minutes, `ExportOptions::timeout`).** At the deadline the
  caller ends the Word process that the worker reported, and only that one:
  its id is `WordSession::pid`, which the session proved belongs to the Word that
  it started (it is a new process that owns the session's own window; see the
  doc comment of `WordSession`), and `word::end_word_process` opens the process
  once, reads its image name from that open handle and ends it only if it is
  `WINWORD.EXE`, so an id that another program has reused is not ended, and the
  user's Word is never in the list. If the deadline passes before Word has
  started, the caller waits up to 60 seconds for the id, then ends that Word. If
  the id never comes nothing is ended and the error says so. A result that
  arrives at the deadline is used as a result. After the end, the worker's COM
  calls fail by themselves and it finishes; the caller waits up to 15 seconds for
  it, then removes the temporary PDF. The error `Timeout` says what was ended.
- **Test seams.** The Word calls are two traits (`WordApp`, `WordDoc`) with a fake
  in the unit tests, the ending of a process is a trait (`EndProcess`), and
  `ExportOptions::after_open` is a hook that runs on the Word thread, after the
  document is open, with the process id and Word's `Document.ReadOnly` of that
  document (`export::OpenDocument`). The Word tests use it to hold Word at a
  known place (a hang that is deterministic), to read the process id and the
  read-only state, and to panic.
- **What Word 16.0.20527 wrote** in the exports of M4a (for M4b and the Phase 0
  table, S4-11): `/MarkInfo<</Marked true>>`, a `/StructTreeRoot`, `/Outlines`
  with `/Title(...)` for each heading, the Info `/Title`, the XMP `dc:title`,
  `/ViewerPreferences` with `/DisplayDocTitle true`, `/Lang(en)` for a document
  whose language is `en-US`, and no `pdfuaid` in the XMP. A table of contents
  entry is one `/Subtype/Link` annotation. Each export takes 5 to 7 seconds
  (Word starts and quits for each one).

## Known gaps in M4a

- **The text of the table of contents is read from the PDF** (closed in M4 review
  round 1). `lopdf` is in the project (M4b), and its `Document::extract_text`
  decodes the font maps that Word writes. `export_updates_fields_before_exporting`
  counts the links (a table of 1 entry in a document with 4 headings gives 4) and
  now also reads the text: each of "Setup steps" and "Cleaning up" is in the PDF
  twice (the entry and the heading), and for the `TXT-004-fail.docx` case (a
  heading renamed after the table was made) "Installation" is there twice and the
  old name "Setup" not at all. It reads the text of all pages as one string, so it
  does not say that the text is inside the table, only that the table was
  rebuilt.
- **`OptimizeFor`, `Item` and `BitmapMissingFonts` are checked as arguments, not
  as effects.** The unit tests of S3-09 read the argument list (names, order,
  types, values) and show that each parameter reaches its own argument. Word's
  use of them leaves nothing that a test on these small fixtures can see in the
  PDF (no comments or markup to leave out, no missing fonts), so nobody checked
  that Word honours the values. The path in `OutputFileName` is a text that the
  test does not read back (the `windows` feature for that is not turned on); the
  real exports show that it is used.
- **Only the main text and the tables of contents are updated.** Fields in
  headers, footers, text boxes, footnotes and tables of figures are not updated
  by the tool. Word updates the page number fields while it lays out the pages.
  `FieldUpdate::first_failed` gives the first field that Word could not update;
  nothing makes it an error.
- **The "user" Word in the tests is hidden.** S3-02 and S3-06 start a separate
  Word with a document and an unsaved edit, through `WordSession` (always
  hidden). A visible Word with a window is not tested, and "its window still
  exists" is not checked. The real person's Word (PID 49988 on the test machine)
  was running during every Word run, and the lists of `WINWORD.EXE` processes
  (from `tasklist`) were the same before and after each test.
- **A hang before the tool knows its Word.** If Word does not start within 60
  seconds after the deadline, the id is never known, nothing is ended, and the
  error says so (`pid` is none). This is not tested with a real Word (a fake that
  starts slowly covers the wait). Word was not seen to hang while it started.
- **A Word that was ended may leave a recovery file.** The tool ends a hung Word
  with `TerminateProcess`. The tool's Word has no edited document (the tool
  updates fields in memory only, and AutoRecover saves on a timer), but whether a
  later start of the person's Word offers "document recovery" for it was not
  checked.
- **Password-protected files (S3-17) are planned.** Stage 1 refuses them (they
  are not a zip file); a real one needs Word to save with a password.
- **A read-only output folder is not tested**, and neither is a path over 260
  characters. The first needs an ACL; Word does not open the second.
- **The early lock check is stricter than the move.** A PDF that a viewer holds
  open and shares for delete could be replaced by the move, but the exclusive
  open of the early check refuses it, as stage 2 does for the .docx. The message
  names the file. It is the safe choice; relax it only if users report it.
- **One Word build.** All the `windows_word` results are from Word 16.0.20527
  (Microsoft 365) on one machine.
- **The command and the report** were built in M4c. The exit codes in the S3 rows
  (1 for a blocked export, 2 for a timeout, a locked PDF or a missing Word) are
  the job of `pipeline::export_docx`; the library gives typed errors for them.

## How M4c reads stage 6 and export

**The command (`src/pipeline.rs`, called by `src/main.rs`).**
`pipeline::export_docx(docx, &dyn Stages, fonts, now) -> Run` is the whole
command. `Stages` has the three calls that need something outside the tool
(`check_tools`, `export` and `validate`); `RealStages` is the real Word and the
real veraPDF, and the tests give fakes. `Run` has the exit (`Exit::Pass`,
`ErrorsFound`, `ToolFailure`; code 0, 1 or 2), the report, the files written, and
the text for stdout and stderr. The order is:

1. Stage 1 on the .docx. A file that cannot be read is exit 2. Any error: write
   the report (verdict Fail, no PDF, stages 4 and 5 "not run") and exit 1.
2. `check_tools`: Java and veraPDF must be there, so a missing tool is found
   before Word starts. It comes after stage 1, so a document with errors still
   gets its Fail report on a machine that has no veraPDF.
3. The check that an existing `Guide.pdf` is not locked by another program
   (`ensure_not_locked`; exit 2 naming `Guide.pdf`), and an empty temporary file
   `~adapdf-<process>-<n>.pdf` (a short name, so long document names stay inside the path limit) in the folder (`TempPdf`, the type that
   `export_pdf` uses; it also shows that the folder takes new files).
4. Stage 3, `export_pdf` to the temporary path (it runs stage 1 again; an
   `ExportError::Blocked` from it gives the same Fail report). Any other
   `ExportError` is exit 2. `export_pdf` writes its own temporary file and renames
   it over the path it is given, so the machinery of M4a is reused and `Guide.pdf`
   is not touched here.
5. Stage 4, `pdfmeta::fix_file(temporary, SourceMetadata::from_document(..))`, with
   the title and the language of the .docx. A `PdfMetaError` is exit 2.
6. Stage 5, `verapdf::validate` on the temporary file. A `VeraPdfError` is exit 2.
7. Both stages have finished, so there is a verdict (pass or fail). Stage 6:
   `report::build`, then the HTML and the JSON are rendered and written to temporary
   files in the folder (`Guide.accessibility.html.adapdf-<process>-<n>.tmp`, and the
   same for the JSON; `TempFile`). If either cannot be written, exit 2 names that
   report and nothing has been moved.
8. Only when both temporary reports exist: the temporary PDF is moved over
   `Guide.pdf` (`TempPdf::move_over`; if that fails, exit 2 names `Guide.pdf` and the
   old PDF is unchanged), and then the HTML and the JSON are moved over their
   names (`Guide.accessibility.html`, `Guide.accessibility.json`). Exit 0 when the
   verdict is Pass, 1 when it is Fail. A run that stage 1 blocks writes its Fail
   report the same way (steps 7 and 8, with no PDF).

The temporary files are removed when the command returns by any other way (the
types remove them when they are dropped), so every exit 2 before step 8 leaves the
folder as it was: the old PDF and the old reports are not changed and no new file
stays. Before M4 review round 1 the new `Guide.pdf` was put in place at step 4, so
a failure of stage 4 or 5 left it there, next to a report of an earlier run.

| Exit | When |
|---|---|
| 0 | The verdict is Pass: no stage 1 error, no stage 4 error, no failed PDF/UA-1 rule. Warnings do not matter. |
| 1 | The verdict is Fail: a stage 1 error (no PDF; a Fail report); a PDF without `/MarkInfo /Marked true` or `/StructTreeRoot` (decided point 8) or with a `pdfuaid:part` other than 1; any failed PDF/UA-1 rule or check of veraPDF. |
| 2 | The .docx cannot be read; veraPDF or Java is missing; Word is not installed, cannot start or cannot open the file; the export times out (5 minutes); `Guide.pdf` is locked by another program (decided point 6); the PDF cannot be read or changed (damaged, encrypted); veraPDF crashes, times out or gives output that is not its JSON; the report cannot be written. stderr has `adapdf: <file>: <plain message>`; stdout is empty. Before the moves of step 8 the folder is left as it was (C-11), also when a report cannot be written; only a move that fails after the PDF was moved can leave the new PDF in place (see "Known gaps in M4c"). |

Decisions that the specs leave open (all documented here, none in the spec):

- **A tool failure writes no report.** The report is the evidence of a check that
  ran. A report from an earlier run is not removed or changed, and (since M4 review
  round 1) neither is an earlier `Guide.pdf`: the new PDF is not put in place until
  it has a verdict. (The alternative,
  a Fail report with "veraPDF did not run" after a tool failure, was not chosen:
  it would put a "Fail" on file for a problem that is not in the document. The
  report type can say "not run" for a stage, so adding it is a small change.)
- **The PDF is kept when the report says Fail** for an untagged PDF or a veraPDF
  failure: it is evidence for the person who fixes the problem, and the report
  beside it says Fail. Both have a verdict, so the new PDF replaces the old one
  (C-11). Stage 1 errors are different: no PDF is made (spec 02). An
  older `Guide.pdf` is not deleted when stage 1 blocks; the folder can then hold a
  PDF from an earlier run next to a new Fail report. The report names its source
  .docx (`summary.document`, and the title and heading of the HTML), so a reader
  can match the files, but it does not say that the PDF beside it is older. veraPDF is also run on an
  untagged PDF (stage 4 appends nothing to it), so the report shows both.
- **The verdict (`report::build`).** Fail when the number of errors is above 0:
  the stage 1 errors, the stage 4 issues that are errors, and the failed rules of
  the PDF/UA-1 profile (one for each rule; at least 1 when veraPDF says "not
  compliant" and lists none). Warnings are the stage 1 warnings and the WCAG
  profile's findings; they never change the verdict. A report in which a stage
  did not run and no error is known is Fail too (not reachable by the command).
- **Section 3, fixes.** `export` applies no fix, so it says "None. No fixes were
  applied in this run." The backup line names the newest
  `<name>.backup-<yyyymmdd-hhmmss>[-n].docx` next to the file, or says that none
  was found. A report that has applied fixes (for the later app) lists them in a
  table. The decision to name an old backup is a choice: it shows the reader that
  the document was changed by `adapdf fix` at some earlier time; it does not say
  that the PDF was made from a fixed file (that cannot be known).
- **Time.** The report uses the local time (`fix::Timestamp::now`, the same clock
  as the name of a backup): `yyyy-mm-dd hh:mm:ss`. The JSON field is
  `generated_local`; there is no time zone in it.
- **veraPDF version.** It is read from the `core` release in veraPDF's own JSON
  (`ProfileResult::verapdf_version`), not from the file names.
- **`ADAPDF_VENDOR`.** In a debug build (and so in the tests), `Tool::locate` uses
  the folder in this environment variable when it is set, and only that one, so that
  a wrong path gives an error that names it. The tests of S5-06 use it. A release
  build ignores the variable (`Tool::locate_with`, I-10), so the installed program
  uses the `vendor/` that it was shipped with and a variable in a person's
  environment cannot point it at another veraPDF or Java. (A release build still
  also looks in the source tree that it was built from, `CARGO_MANIFEST_DIR/vendor`,
  when `vendor/` next to the program is missing; the installer layout of M5 must
  have `vendor/` next to `adapdf.exe`.)
- **Rule table.** The entries for 7.18.1-2 and 7.18.5-2 now say what really
  happens (Word's ScreenTip; a table of contents). In M4c they had no linked pre-check
  rule (the old link to LNK-001 was wrong: a link with good text still fails). Both now
  name LNK-004 (2026-11-16; a table entry names one rule, and the how-to-fix text names
  LNK-005 for the table of contents). The table has 29 entries.
- **Existing reports are replaced** without a backup, as the PDF is.

**The report (`src/report`).** `report::build(Input) -> Report`; `Report::to_json()`
gives the JSON, `html::render(&Report)` the HTML, `Report::console_summary` the
text of stdout. The JSON has these top-level fields, in the order of the
sections: `schema_version` (1), `summary` (`verdict` pass or fail, `errors`,
`warnings`, `reason`, `tool`, `tool_version`, `generated_local`, `document`,
`pdf`), `export` (Word build, the 9 parameters, `fields_updated`,
`tables_of_contents_updated`, `first_failed_field`, `seconds`; null when no PDF
was made), `precheck` (the stage 1 findings, as `adapdf check --json` gives
them), `fixes` (`applied`, `backup`), `pdf_metadata` (`status` `done` with
`changes` and `issues`, or `not-run` with `reason`), `verapdf` (`status` `ran`
with `verapdf_version` and `validation` (the UA-1 and WCAG results), or
`not-run`), `pac` (`report_file`, `pac_version`, `result`, all null) and
`checklist` (`items` with `done`, `name`, `date`). The schema version changes
when a field is removed or changes meaning.

**The HTML.** One file, no script, no outside resource. `<main>` holds the `h1`,
a list of links to the seven sections, and the seven sections as `h2`. Tables
have a caption and `th scope`; the first column of a table of facts is a row
header. The checklist, the PAC fields and the sign-off are labelled controls in
`fieldset`s, not inside a `form`: they work in a browser opened on the local
file, and are not saved when the page is closed (see "Known gaps in M4c"). The
markup is also well-formed XML, so the tests read it with `roxmltree` (it needs
`allow_dtd` for `<!DOCTYPE html>`). The colours are in the style sheet only, and
the test reads them from the style sheet that is written.

## S5-10 result

S5-10 was run twice. The first run (M4c) failed for 4 fixtures and led to the rules
LNK-004 and LNK-005. The second run is the result that counts now.

### Second run: with LNK-004 and LNK-005

Measured on the evening of 2026-11-16 (local time; the decision that led to it is
dated 2026-11-16) with Word 16.0.20527 and veraPDF 1.28.2 (WCAG profile
`WCAG-2-2-Machine.xml` present), by `cargo test --features windows_word,verapdf
--test export_verapdf -- --nocapture --test-threads=1` (6 tests, all passed, 996
seconds). Each fixture is exported by Word, stage 4 is run, and veraPDF checks the
final PDF. **Every PDF: exit 0, 0 failed rules.** Per fixture:

| Test | Fixtures | Result |
|---|---|---|
| `verapdf_zero_failures_on_pass_fixtures` (S5-10, 35 pass fixtures) | DOC-001 to DOC-006 (6), HDG-001 to HDG-004 and HDG-006 (5), LST-001 and LST-002 (2), TXB-001 to TXB-004 (4), TBL-001 to TBL-006 (6), **LNK-001 to LNK-005 (5)**, CLR-001 and CLR-002 (2), TXT-001 to TXT-004 (4), IMG-006 (1) | each one: exit 0, 0 failed rules (35 of 35) |
| `verapdf_zero_failures_on_the_picture_pass_fixtures` | IMG-001 to IMG-005, IMG-007 | each one: exit 0, 0 failed rules (6 of 6) |
| `verapdf_zero_failures_after_the_link_fixes` (S5-12) | A guide with 3 links that have no ScreenTip (a `w:hyperlink`, a complex field, a simple field) and a table of contents with `\h`. Before: stage 1 refuses it (LNK-004 three times, LNK-005) and no PDF is written. `adapdf fix` (four yes answers) applies both auto fixes. Then `adapdf export`. | exit 0, 0 failed rules. The PDF has 3 `/Subtype/Link` annotations (the table of contents has none) and each has `/Contents` with the link text (`/Contents(parking policy)`, `/Contents(the same policy)`, `/Contents(and again)`), so Word writes the ScreenTip of a field link (`\o`) as it does for `w:tooltip`. |
| `verapdf_on_warning_fixtures_is_recorded` (S5-11) | the 24 failing fixtures of warning rules | 23 give exit 0 and 0 failed rules; `LST-002-fail.docx` is blocked by an error of another rule (it also has LST-001). The 3 that failed in the first run (LNK-001-fail, LNK-002-fail, TXT-004-fail) now give 0, because their documents have a ScreenTip or no `\h` (see below). |

The criterion was not weakened and the test is the one of the first run, except
that its list has the 2 new pass fixtures (35). What changed for the 4 fixtures
that failed:

- LNK-001-pass, LNK-002-pass, LNK-003-pass (and the matching fail fixtures) now
  have a ScreenTip on their link (`w:tooltip`). This is not hiding a problem: the
  new fixture `LNK-004-fail.docx` is the same kind of link without a ScreenTip, and
  stage 1 reports it as an error. The first run's PDFs for the same documents
  without ScreenTips failed 7.18.1-2 and 7.18.5-2.
- TXT-004-pass and TXT-004-fail now have a table of contents with the field
  `TOC \o "1-3" \z \u` (no `\h`), and its entries are plain runs, as Word writes
  them. `LNK-005-fail.docx` is the same table with `\h`, and stage 1 reports it.
- A table of contents made without `\h` still has one `Link` tag in the PDF for
  each entry (read with `lopdf` in S3-11), but no link annotation. veraPDF passes
  it. Whether PAC does is Phase 0 (P0-10).

### First run: before LNK-004 and LNK-005

Measured on 2026-11-16 with Word 16.0.20527 and veraPDF 1.28.2, by
`verapdf_zero_failures_on_pass_fixtures` in `tests/export_verapdf.rs` (33 pass
fixtures then):

| Fixtures | Result |
|---|---|
| DOC-001 to DOC-006, HDG-001 to HDG-004, HDG-006, LST-001, LST-002, TXB-001 to TXB-004, TBL-001 to TBL-006, CLR-001, CLR-002, TXT-001 to TXT-003, IMG-006 (29) | 0 failed rules |
| IMG-001 to IMG-005 and IMG-007 (6, in `verapdf_zero_failures_on_the_picture_pass_fixtures`) | 0 failed rules |
| LNK-001-pass, LNK-002-pass, LNK-003-pass | 2 failed rules each: **7.18.1-2** and **7.18.5-2** |
| TXT-004-pass | the same 2 failed rules |

**The two rules.** 7.18.1-2: an annotation has neither a `Contents` key nor an
`Alt` entry in its structure element. 7.18.5-2: a link annotation has no alternate
description in `Contents`. veraPDF's context is the same in all four:
`root/document[0]/pages[0](.. PDPage)/annots[0](.. PDLinkAnnot)`.

**What Word does (evidence).** For `LNK-001-pass.docx` Word writes the link as
`<</Subtype/Link/Rect[..]/BS<</W 0>>/F 4/A<</Type/Action/S/URI/URI(https://example.org/parking-policy) >>/StructParent 1>>`:
no `/Contents`. The same file with `w:tooltip="Parking policy page"` on the
`w:hyperlink` (that is Word's "ScreenTip") gives the same annotation with
`/Contents(Parking policy page)`, and the export passes veraPDF (0 failed rules,
exit 0). For `TXT-004-pass.docx` the links are the entries of the table of contents
that Word makes again when the fields are updated. The same file with the `\h`
switch removed from the field (`TOC \o "1-3" \z \u`; in Word, "Use hyperlinks
instead of page numbers" turned off) gives 0 link annotations and passes veraPDF
(0 failed rules). Both checks were made on copies in a scratch folder, with the
same command (`adapdf export`); the fixtures were not changed.

**Classification.**

| Fixtures | Class | Why |
|---|---|---|
| LNK-001-pass, LNK-002-pass, LNK-003-pass | **(b) a missing pre-check rule** | The documents are valid and have good link text, and the PDF still fails: the cause is a link with no ScreenTip, and nothing in spec 03 looked for that. They are not a fixture builder problem (a). Word's export is doing what it always does (c, but the author can fix it in the source), and stage 4 cannot fix it without editing an annotation (d, and that would break "only document-level metadata"). |
| TXT-004-pass | **(c) a Word export limitation**, with a workaround that is a decision | Word's own table of contents (with hyperlinks) cannot give a link a lasting ScreenTip: Word makes the entries again at each update. Without `\h` there are no links and the PDF passes. |

**What was done in M4c, and what was decided after it.**

- M4c fixed the veraPDF table entries for 7.18.1-2 and 7.18.5-2 (a better message
  and the fix text), so that the report tells the author what to do. The exit
  criterion was not weakened, and the fixtures were not given ScreenTips to hide
  the missing rule. Making stage 4 write `/Contents` into annotations is outside
  spec 02 ("only document-level metadata", AGENTS.md: never edit tags or content)
  and also needs text that the tool does not have.
- **The user decided on 2026-11-16** (decided points 10 and 11): a new rule LNK-004
  (error, auto fix: copy the link's own text into the ScreenTip), and a new rule
  LNK-005 (error, auto fix: remove `\h` from the table of contents field). Both are
  built (spec 03; S2-32, S2-33; the fixtures `LNK-004` and `LNK-005`), and the
  second run above is the result. Stage 4 is unchanged.
- **Still open (Phase 0, P0-10):** whether PAC (the checker of record, spec 01)
  also fails a link with no `/Contents`, and whether it fails the `Link` tags of a
  table of contents that has no link annotation. The rules follow veraPDF until
  then. An error rule means that a guide with a link and no ScreenTip is not
  exported until the author or `adapdf fix` adds the ScreenTip.

## Known gaps in M4c

- **S5-10 passes** (35 of 35, and the 6 picture fixtures) since LNK-004 and
  LNK-005 were built (2026-11-16; "S5-10 result"). It failed for LNK-001,
  LNK-002, LNK-003 and TXT-004 in the first run. What is still open is the part
  that veraPDF cannot say: PAC (P0-10) may fail a link with no `/Contents`, or
  the `Link` tags that a table of contents without `\h` still has.
- **LNK-004 and LNK-005 are errors, so they block the export** of every guide that
  has a link with no ScreenTip or a table of contents with `\h`. That is most
  Word-made guides until `adapdf fix` (or the author) has done the fixes. The
  fixes need only a yes. The ScreenTip that the fix writes is the link text, so
  it is no better than the link text (a link "here" gets the ScreenTip "here";
  LNK-001 still warns about it).
- **A table of contents made without `\h` cannot be clicked in the PDF.** Readers
  use the PDF bookmarks (`CreateBookmarks`) and the headings. A table of contents
  that is only a `TOC` simple field (`w:fldSimple`) is not read by LNK-005; links
  in headers, footers and footnotes are not read by LNK-004.
- **Escaping in a field instruction is checked against Word 16.0.20527 only.**
  Word read back a ScreenTip with quote marks, a backslash, `&`, `<`, `>`, `é` and
  Japanese letters as the same text, for a `w:hyperlink`, a complex field, a
  simple field and a field in pieces (S2-15); other Word builds were not tried.
- **S6-11 (a screen reader reads the report) was not done.** No browser, NVDA,
  Narrator or axe-core was run on the report. The accessibility checks that were
  made are structural: the HTML is parsed, and the headings, tables, labels,
  link texts and contrast (from the style sheet) are tested. They cannot say how
  a screen reader reads the page, or how the page looks. Someone must open a
  report from a failing fixture with several findings and do S6-11.
- **The checklist and the sign-off cannot be saved.** A browser does not keep
  what is typed or ticked in a page that has no script and no form action, so
  closing the page loses it. The person can print the page to PDF after the
  sign-off, or the later app can store it. As evidence, the report on disk is
  always the unsigned one. This is a gap between spec 02 ("a name and a date for
  sign-off") and a page that needs no script; decided for the app (M5) in row A-08:
  the app saves the checklist and the sign-off into the JSON and the HTML.
- **A tool failure leaves no report**, and an old report or PDF is not removed or
  changed (see "How M4c reads stage 6 and export" and C-11). Since M4 review round
  1 this also holds for the PDF of a run that fails in stage 4 or 5, and (round 1,
  second change) for a report that cannot be written: both reports are written to
  temporary files first, and nothing is moved until both exist
  (`a_report_that_cannot_be_written_is_exit_2`). Two cases are left: (1) **a move
  that fails after the PDF was moved** (rare: a folder, or a file that another
  program holds, has the name of a report;
  `a_report_that_cannot_be_replaced_leaves_the_new_pdf_and_says_so`). Exit 2 names
  the report and says that the new PDF is in place. What can be left: the new
  `Guide.pdf`, and the report of an earlier run for each report that was not
  replaced (the HTML is moved first, so a failure there leaves both old reports; a
  failure of the JSON leaves the new HTML and the old JSON). No temporary file
  stays. A full undo would need a copy of the old PDF; a check of the report names
  before the move (not a folder, not held) was not built. (2) When **stage 1
  blocks** a new run, the old PDF stays (the Fail report replaces the old reports,
  written the same way, and the PDF from an earlier run stays next to it). The
  report names its source .docx, so a reader can match the files.
- **The WCAG profile has shown no warning of its own.** On the one report that was
  read for this (LNK-001-pass) the profile ran and had 0 warnings of its own, and
  the other exports were not read for it, so the warning table of the report is
  tested with recorded JSON only.
- **The timeout of 5 minutes cannot be changed on the command line**; the tests
  change it through the library.
- **Stage 1 runs twice** in a successful run (once in the command, once in
  `export_pdf`). It takes milliseconds; the second run is the guard that a
  library caller gets.
- **The time has no time zone** in the report.
- **The pass fixtures of S5-10 are small.** They test the rules one at a time. A
  real guide has links, a table of contents, many tables and pictures at once;
  Phase 0 is still where the real result comes from.
- **One Word build** (16.0.20527) and one veraPDF (1.28.2). A run takes about 10
  seconds (Word 5 to 7 seconds, and the rest is veraPDF, which is started twice,
  for UA-1 and for WCAG, and each start is a Java process).

## How M5a builds the app

**Layout.** `app/` is a Vite, React and TypeScript page with `@carbon/react`
(Carbon v11), and `app/src-tauri/` is the Tauri 2 package `adapdf-app`, which
depends on the `adapdf` library by path. The repository root is a Cargo workspace
with the members `.` and `app/src-tauri`. The root package is the only default
member, so `cargo test`, `cargo clippy` and `cargo build` at the root work as
before and need no Node: they do not build the app. The app has its own commands
(see "How to run"). The lock file is shared.

**Tauri commands** (`app/src-tauri/src/shell.rs`, thin) call functions in
`commands.rs` and `signoff.rs` (the logic, tested without a window). They run on a
worker thread (`spawn_blocking`): a command that is not `async` runs on the main
thread, and the window would freeze while Word exports. A tool failure (exit 2 in
the CLI) is an `Err` with a plain message, and the page shows it in the live
region. The commands: `check`, `fix_offers`, `apply_fixes`, `export`,
`open_report`, `read_report` (to open an earlier report) and `save_signoff`.
`apply_fixes` names a finding by its rule and its position and takes the
person's input, so the page cannot send a finding of its own; the backup and the
lock checks are those of `fix::fix_file`, which the app does not repeat.

**Pages and keys.** The first screen has the button to choose a document (the
Tauri dialog) and a button to open an earlier report; a file dropped on the window
does the same (a Tauri drag and drop event, `dragDropEnabled`). The findings
screen groups findings by severity. The fix screen has a check box for each auto
fix, a labelled text field for each ask fix (empty; the suggestion is its helper
text) and "Mark as decorative" for alt text. The export has a progress state; the
result says Pass or Fail in words, with the counts and the veraPDF result, and has
the sign-off form. After each action the heading of the screen gets the focus, and
one live region (`role="status"`), in the page from the start, carries the messages
(Carbon's `InlineNotification` is inside it with `aria-live="off"`, so that a
message is announced once).

**Fonts.** Carbon's `@font-face` rules are turned off (`$css--font-face: false` in
`app/src/styles.scss`). `app/src/fonts.css` has the four faces that Carbon uses
(IBM Plex Sans 300, 400, 400 italic and 600), pointing at the `woff2` files of the
package `@ibm/plex-sans`; Vite copies them to `dist/assets` and rewrites the
`url()` (`assetsInlineLimit: 0`, so no `data:` URL is needed). IBM Plex Mono is not
bundled: the page has no code text, and Carbon's mono stack falls back to a system
font.

**Content security policy** (`tauri.conf.json`): `default-src 'self'; connect-src
'self' ipc: http://ipc.localhost; font-src 'self'; img-src 'self'; script-src
'self'; style-src 'self'; object-src 'none'; base-uri 'self'`. No `unsafe-inline`:
Carbon and React set styles through the style object of the DOM, which the policy
allows. The running app showed no policy violation (below).

**Capabilities** (`capabilities/default.json`): `core:event:allow-listen`,
`core:event:allow-unlisten` (to hear a dropped file) and `dialog:allow-open`. No
shell, no HTTP, no file system plugin and no opener permission for the page: the
report is opened by the Rust command `open_report`, which checks the path first.
`the_capabilities_are_minimal` also requires that `capabilities/` holds only
`default.json` and that `tauri.conf.json` has no inline `app.security.capabilities`
(a second file or an inline capability would add permissions that the test does not
read; checked by mutation: an extra `capabilities/extra.json`, and an inline
capability, each fail the test). The page cannot call a plugin that it has no
permission for (checked on the running app: `plugin:fs|read_text_file` gave "not allowed by ACL"). The opener
plugin is used from Rust only, so its JavaScript package is not installed.

**Installer settings** (`bundle.windows.webviewInstallMode` is `skip`, and NSIS
`installMode` is `currentUser`; rows I-02 and I-08 were tested in
`frontend_bundle.rs` and are now in `installer_config.rs`). M5b built the installer.

**N-01 and Tauri.** Tauri's manifest lists `reqwest` (with `hyper` and
`hyper-util`) for Android and iOS only. They are in `Cargo.lock`, because the lock
file lists every target, and they are not in the Windows tree of the app or of the
library (`cargo tree -p adapdf-app -e normal,build --target x86_64-pc-windows-msvc`
has none of the 17 crates of N-01). N-01 was changed to say so: the lock file test
allows exactly those three names (`MOBILE_ONLY`), and a new test,
`no_http_client_in_the_app_windows_tree`, checks the Windows tree of the app. This
test runs `cargo tree -p adapdf-app`, so it needs the app's crates in the cargo
cache (or the network, the first time). The other crates that the app adds are Tauri
and its plugins, `webview2-com`, `wry` and `tao`.

**Checked on the running program** (2026-11-17, by a script in the scratch folder,
through the debugging port of WebView2; it started the program itself and ended it).
With `target/release/adapdf-app.exe` (from `pnpm tauri build --no-bundle`) and
with `pnpm tauri dev`: the window started and the page loaded; the heading had the
focus; the fonts were IBM Plex Sans from the bundle; the page made only requests to
`tauri.localhost` (and, in `tauri dev`, `localhost:5173`) and to `ipc.localhost`;
no console message and no policy violation in the release build; the command
`check` through the real IPC gave 1 error and 1 warning for `LST-002-fail.docx`;
a missing file, and `open_report` with a web address, gave the plain error messages;
a plugin that the page has no permission for was refused. The one thing in
`tauri dev` was a 404 for `favicon.ico` from the Vite server (the page has no icon).
No `WINWORD.EXE` was running before or after, and no program was left running.

**Review round 1** (an independent reviewer scored M5a 8.6 of 10; each point was
checked against the code first):

- **The page and the Rust commands agree on the data (A-09).** Real values of every
  type that the page receives are written by Rust to `app/src/test/contract/*.json`
  and checked in both languages. The mechanism for the names of the arguments is the
  simplest that is honest: `contract/calls.json` is the arguments that the page sends,
  Vitest checks that the page sends exactly these, and a Rust test reads `shell.rs` as
  text and checks that they are the camelCase of the parameters of the
  `#[tauri::command]` functions. So `reportJson` and `report_json` cannot drift apart
  without a failing test on one side (mutation checks in A-09). The Rust side reads the
  source text and does not run Tauri's own macro; a command that is not a plain
  `async fn name(...) ->` in `shell.rs` would need the parser in `contract.rs` to change.
- **Capabilities folder.** See "Capabilities" above.
- **Colour contrast in the real WebView2 (A-10).** Measured on 2026-11-17 with
  `app/scripts/axe-webview.mjs` on the release program (`pnpm tauri build --no-bundle`,
  Chromium 157 in WebView2, a window of 1100 by 800), with `ADAPDF_VENDOR=C:/nowhere` set
  and `vendor/` joined next to a copy of the program. axe-core 4.14.0, rules WCAG 2.1 A and
  AA, `color-contrast` on. A planted grey (`#c6c6c6` on white) was found, so the check
  works. Results (violations, of which `color-contrast`; elements that the contrast rule
  measured; checks that axe could not decide):

  | Screen | Violations | Contrast: elements measured / not decided |
  |---|---|---|
  | The first screen | 0 (0) | 7 / 0 |
  | The first screen with an error message (a file that is not there) | 0 (0) | 9 / 0 |
  | Findings of `LST-002-fail.docx` | 0 (0) | 23 / 0 |
  | Fix flow of `LST-002-fail.docx` | 0 (0) | 10 / 0 |
  | Findings after a fix (success message) | 0 (0) | 8 / 0 |
  | Fix flow of `DOC-001-fail.docx` (text field, suggestion) | 0 (0) | 13 / 0 |
  | Fix flow of `IMG-001-fail.docx` (text field, decorative) | 0 (0) | 14 / 0 |
  | Result of a blocked export (Fail; a report that the real `export` wrote), with the empty sign-off form | 0 (0) | 51 / 0 |
  | The sign-off form with the error for a missing name | 0 (0) | 52 / 0 |
  | Findings of `DOC-001-pass.docx` (no problems) | 0 (0) | 6 / 0 |
  | Export in progress (real Word) | 0 (0) | 5 / 0 |
  | Result of the real export: Pass, with the empty sign-off form | 0 (0) | 40 / 0 |
  | The sign-off form filled in (5 ticks, a name, a date) | 0 (0) | 40 / 0 |
  | The result after the sign-off was saved (success message) | 0 (0) | 42 / 0 |

  14 screens, 0 violations. The Fail verdict (`#a2191f`), the Pass verdict (`#0e6027`),
  the grey hint text and the notification colours are all in these screens. The real
  Word and veraPDF ran for the Pass (about 10 seconds); no `WINWORD.EXE` before or
  after, no `adapdf-app.exe` left, and the scratch folder was removed. Not measured: a
  high-contrast theme, 200% scaling and other window sizes (A-05, a person), and hover
  and focus states (axe reads the page as it stands).
- **Young crates.** See "Known gaps in M5a" (Dependencies).
- **Installer rows for the app.** I-05, I-07, I-10 and N-03 now say "the app"; I-11 is new
  (`vendor/` next to the exe through `bundle.resources`, done in M5b). A release
  build no longer looks for `vendor/` at the path of the source tree
  (`CARGO_MANIFEST_DIR`; `vendor_folders` in `src/verapdf/mod.rs`, with
  `a_release_build_looks_only_next_to_the_program`). A debug build and the tests keep
  that place. One effect: `target/release/adapdf.exe` (the command line, a developer
  tool) finds the tools only when `vendor/` is next to it, or in a debug build.
  The release app run of A-10 had `vendor/` next to it through a directory junction
  and found it there.
- **A half-saved sign-off.** The JSON is replaced first and the page second, so that
  a failure in between leaves the record (the JSON) signed and the page behind, which
  understates the sign-off, instead of a page that claims a sign-off that the record does
  not have. The message says what was saved. The page of the app says "The sign-off could
  not be saved in full." (it said "was not saved", which is wrong after a half save).
  See A-08.
- **Wording.** A-04 now has an axe check of a filled-in sign-off form (5 ticks, a name, a
  date). `app_export_with_word_matches_the_cli` became
  `app_export_with_word_matches_the_cli_export` and really runs the code of
  `adapdf export` on the same input (A-07).
- **Small points.** `the_page_has_a_language_and_a_title` (`<html lang="en">` and a
  title, in `app/index.html` and in `dist/index.html`; checked by mutation). A-07 has
  `Decorative` (IMG-001) and the IMG-007 yes. A mapped network drive is in the comment
  and the plan (not rejected: that needs `GetDriveTypeW` and a new dependency).

**Counts** (measured 2026-11-17 on the final tree of review round 1):

| Command | Tests run | Failed |
|---|---|---|
| `cargo test` (root) | 775 (774 before review round 1: +1 in `src/verapdf/mod.rs`) | 0 |
| `cargo test -p adapdf-app` (after `pnpm build`) | 29: 22 in the library (17 in `src/tests.rs`, 4 in `src/tests/contract.rs`, 1 in `src/signoff.rs`) and 7 in `tests/frontend_bundle.rs` | 0 |
| `cargo test -p adapdf-app --lib --features windows_word,verapdf` (`ADAPDF_VENDOR` set) | 23 (the 22 and `app_export_with_word_matches_the_cli_export`) | 0 |
| `pnpm test` (in `app/`) | 74: 37 in `App.test.tsx`, 10 in `viewModel.test.ts`, 1 in `test/axe.test.tsx`, 26 in `test/contract.test.tsx` | 0 |
| `pnpm exec tsc --noEmit` (in `app/`) | no error | |
| `node scripts/axe-webview.mjs` (A-10) | 14 screens | 0 violations |

(Before review round 1: root 774, `cargo test -p adapdf-app` 23, `pnpm test` 48.)

Versions and publish dates are in the report of the milestone (the commit message
has the Tauri and Carbon versions); `app/pnpm-lock.yaml` and `Cargo.lock` have the
rest.

## Known gaps in M5a

- **The colour contrast is measured on the screens of A-10 only.** jsdom cannot measure
  it, so the Vitest tests leave the rule off; `app/scripts/axe-webview.mjs` measures it in
  the real WebView2 (0 violations on 14 screens, review round 1). The script is run by
  hand (it needs the built program, and Word for the Pass), and it is not part of
  `cargo test` or `pnpm test`. Not measured: a high-contrast theme, 200% scaling, other
  window sizes, hover and focus states, and screens that need a state that the script
  does not reach (for example a veraPDF failure on a real PDF).
- **A-05 (keyboard, Narrator, NVDA, 200% scaling, high contrast, a real drop of a
  file) and the browser part of A-06 are not done.** The file dialog and the drop
  are Windows features that the tests replace. A person must walk the flow.
- **The Tauri shell (`shell.rs`) has no test that runs it.** It only starts the
  commands on a worker thread. A test reads its text (the names of the commands and of
  their parameters, A-09), and the commands were called through the real IPC once by hand
  and by `axe-webview.mjs` on each run (check, fix_offers, apply_fixes, export,
  read_report and save_signoff; `open_report` is not called by the script).
- **A sign-off with unticked items can be saved.** The report keeps each item's
  state, and says "n of 7 checks are ticked", but nothing stops a person from
  signing with items open (a person may stop half way). The name is required; the
  date is not.
- **A report that `export` writes again replaces a signed report.** `export` always
  writes an unsigned report (decided in M4c), so the sign-off is lost when a document
  is exported again. This is by design (the new PDF needs a new check), but the app
  does not warn about it.
- **A half-saved sign-off is possible.** The two files are replaced one after the other.
  If the second move fails (for example, a program holds the page), the JSON has the
  sign-off and the page does not. The JSON is first so that the record is the one that is
  right, the message says what was saved, and saving again makes both agree
  (`a_half_saved_signoff_says_what_was_saved`). The two moves are not one step, and a
  crash between them leaves the same state without a message.
- **A mapped network drive is not told from a local drive.** `check_report_html` refuses
  a network path (`\\server\share`) and a path that is not absolute. A drive letter that
  maps to a share (`Z:`) passes: telling it apart needs `GetDriveTypeW`, which needs the
  `windows` crate in the app (a change to `Cargo.lock`). The file is one that the app wrote
  next to the person's document, or that the person chose, so this is a limit and not a way
  for the page to open another file.
- **The sign-off changes the JSON and the HTML, not the PAC fields.** The PAC report
  file, version and result are still fields of the HTML page that a browser cannot
  save (a static page); the app does not save them.
- **The first version of the page shows the PDF path, not a link to it.** The report
  is opened in the browser; the PDF is not opened by the app.
- **Dependencies.** `pnpm add` of a second group of packages fails with the pnpm
  error `ERR_PNPM_EXOTIC_SUBDEP` once `pnpm-lock.yaml` has a Tauri package in it (pnpm
  10.26 with `block-exotic-subdeps`). The packages were added by writing
  `package.json` with exact versions and adding one package into an empty folder
  (the lock file was made by `pnpm add`). `@tauri-apps/api` and `@tauri-apps/cli`
  are 2.12.4 and not 2.12.5, which was less than 48 hours old.
- **Twelve crates in `Cargo.lock` were less than 48 hours old when they were
  locked** (rule: install only versions at least 48 hours old). They are needed by
  Tauri 2.12.4, and `cargo update -p ... --precise` is blocked in the build
  environment, so they were not lowered. Published between 2026-11-15T17:40Z and
  2026-11-16T19:58Z (the report of M5a has the time of each; the lock file does not
  store it, and it was not looked up again here):

  | Crate | Version |
  |---|---|
  | `brotli` | 9.1.3 |
  | `brotli-decompressor` | 6.3.2 (the lock file also has 5.0.3, which is older) |
  | `jiff` | 0.2.46 |
  | `jiff-core` | 0.1.8 |
  | `jiff-static` | 0.2.46 |
  | `plist` | 1.10.4 |
  | `tauri-codegen` | 2.7.5 |
  | `tauri-macros` | 2.7.5 |
  | `tauri-plugin` | 2.7.3 |
  | `tauri-runtime` | 2.12.3 |
  | `tauri-runtime-wry` | 2.12.4 |
  | `tauri-utils` | 2.10.3 |

  **This branch is not merged to `main` before all twelve are at least 48 hours old
  (about 2026-11-18 20:00 UTC, which is 48 hours after the newest, published at
  2026-11-16T19:58Z), and the dates are checked again then** (for example with
  `cargo info` or the crates.io page of each crate). If one of them is still younger, the
  merge waits.
- **The page needs `app/dist` to compile the program** (`generate_context!`). The
  library and its tests are separate for this reason (`cargo test -p adapdf-app
  --lib` needs no build of the page).

## How M5b builds the installer

**One command.** `powershell -ExecutionPolicy Bypass -File scripts/build-installer.ps1`
(add `-Vendor <folder>` when `vendor/` is not at the root of the tree). It checks that
`vendor/` has `jre/bin/java.exe` and `verapdf/bin`, copies `vendor/` to
`app/src-tauri/bundle-staging/vendor` (Git ignores it), and runs
`pnpm tauri build --config src-tauri/tauri.bundle.conf.json` in `app/`. Tauri builds the
page (`pnpm build`), the release program and the NSIS installer. It takes about 3 minutes
with a cached build. Tauri downloads its NSIS tools the first time it makes an installer (a
build-time download, like `fetch-vendor.ps1`; the installer and the app make no network
calls). They were already in `%LOCALAPPDATA%\tauri\NSIS` (dated 2026-10-19), and the log of
the build has no download.

**Two config files, on purpose.** `app/src-tauri/tauri.conf.json` has what every build
needs: `bundle.targets` `["nsis"]`, `bundle.windows.nsis.installMode` `currentUser`,
`bundle.windows.webviewInstallMode` `{ "type": "skip" }`, the product name and publisher
`Adapdf` (neutral; no organisation's name), and the short and long description.
`app/src-tauri/tauri.bundle.conf.json` has `bundle.resources` and is added by the script only.
If `resources` were in the first file, every `cargo build` and `cargo test -p adapdf-app`
would need the staging folder. The sources are paths relative to the config file, so no path
of this machine is in a file of the project (`installer_config.rs` checks it). The resources:
`bundle-staging/vendor/` to `vendor/`, `THIRD-PARTY.md` to `THIRD-PARTY.md`, and the OFL
licence of IBM Plex Sans (from `app/node_modules/@ibm/plex-sans/LICENSE.txt`) to
`licenses/IBM-Plex-OFL-1.1.txt`.

**Why a staging copy.** NSIS cannot open a path of 260 characters or more. veraPDF's
`profiles/*/PDF_A/<flavour>/<clause>/` and `PDF_UA/<folder>/` folders (the per-clause source
files of the merged profile files) have paths over 200 characters. The veraPDF command line
reads the merged files (`PDF_UA/PDFUA-1.xml`, `PDF_UA/WCAG-2-2-Machine.xml` and so on), not
those folders, and the script leaves them out, with the `.github` folder of the profiles.
The merged files stay: the staged `profiles/veraPDF-validation-profiles-rel-1.28/PDF_UA/`
has `PDFUA-1.xml` and `WCAG-2-2-Machine.xml`, and both runs work from the installed copy (see
step 4 below). The script stops when a file of the staging copy has a path longer than
150 characters inside `vendor/`. The staging copy was 162 MB (`vendor/` is 165 MB); it is
deleted after the build.

**What `Tool::locate` needed.** Nothing. Tauri puts a resource at its target path under the
install folder, so `vendor/` is next to `adapdf-app.exe`, which is the only place that a
release build looks in (`vendor_folders`; no source-tree fallback in a release build). The
installed program found the bundled tools with `ADAPDF_VENDOR=C:/nowhere` in its
environment (the variable is ignored in a release build: I-10).

**No licence page.** The WIP set `bundle.licenseFile` to `THIRD-PARTY.md`. That makes the
installer show a page that a person must accept, with Markdown text. It was taken out:
`THIRD-PARTY.md` and the font licence are in the install folder (the licence texts of
veraPDF and of the Java runtime are in `vendor/`), and `installer_config.rs` checks that
the setting is not there.

**The installer.** `target/release/bundle/nsis/Adapdf_0.1.0_x64-setup.exe`, 41,792,563
bytes (39.86 MiB), SHA-256 `3714d9316c3617c5c0dac661cbba18fba755ad9daf0ca2935533636e6a5edd0b`
(of the file that was tested; a new build gives another value). `target/release/bundle` is 40
MB. The build did not change `Cargo.lock` or `app/pnpm-lock.yaml` (`git status`).

**The installed folder** (`%LOCALAPPDATA%\Adapdf`, 175 MB, top 2 levels):

```
adapdf-app.exe
uninstall.exe
THIRD-PARTY.md
licenses/IBM-Plex-OFL-1.1.txt
vendor/jre/        NOTICE, bin/ (java.exe and the runtime), conf/, legal/, lib/, release
vendor/verapdf/    bin/, model/, plugins/, profiles/, verapdf.bat, verapdf-gui.bat
```

**The run of 2026-11-17 on the build machine** (Windows 11, a standard user, the shell not
elevated; Word 16.0 installed; Eclipse Adoptium JDK 21 on the PATH and `JAVA_HOME` set; the
source tree of the worktree present). Steps, in order:

1. Before: `WINWORD.EXE` not running, no `adapdf-app.exe`, no `%LOCALAPPDATA%\Adapdf`;
   `%LOCALAPPDATA%\com.adapdf.desktop` 48,572,325 bytes; `HKCU\Software\Adapdf\Adapdf` there
   already (an older install; value `%LOCALAPPDATA%\Adapdf`).
2. `Adapdf_0.1.0_x64-setup.exe /S`: exit code 0 after 9 seconds, no prompt (I-01). The folder,
   the `HKCU\...\Uninstall\Adapdf` entry (DisplayName, Publisher `Adapdf`, InstallLocation,
   UninstallString, all under `%LOCALAPPDATA%\Adapdf`) and the Start menu shortcut `Adapdf.lnk`
   were made. Nothing under `HKLM`, `C:\Program Files` or the all-users Start menu.
3. `node scripts/axe-webview.mjs --installed %LOCALAPPDATA%\Adapdf\adapdf-app.exe --no-java
   --watch` (twice; the second run printed more of the command line). The script starts the
   installed program in place, with no copy and no `vendor/` joined, `ADAPDF_VENDOR=C:/nowhere`,
   `JAVA_HOME` and the Java folders out of the PATH of the started program only, and WebView2's
   debugging port with a scratch user data folder. It checks `LST-002-fail.docx` (the findings
   through the real IPC, then the fix flow), exports `DOC-001-pass.docx` with the real Word and
   veraPDF (Pass, "veraPDF 1.28.2"; the script fails on anything but Pass in this mode), and runs
   axe-core on 14 screens: **0 violations, 0 of them colour contrast, 0 contrast checks
   undecided** in both runs (the planted grey text was found first). It then ended only the
   process tree that it started and removed its scratch folder.
4. The watch of the two runs (18 and 20 looks): 11 processes in the tree (`adapdf-app.exe` 1,
   `msedgewebview2.exe` 6, `java.exe` 2, `conhost.exe` 2). Both `java.exe` were
   `%LOCALAPPDATA%\Adapdf\vendor\jre\bin\java.exe` with `-classpath
   %LOCALAPPDATA%\Adapdf\vendor\verapdf\bin\*`, started by the app: the first `--flavour ua1`
   on the temporary PDF, the second `--profile
   %LOCALAPPDATA%\Adapdf\vendor\verapdf\profiles\veraPDF-validation-profiles-rel-1.28\PDF_UA\WCAG-2-2-Machine.xml`.
   So the WCAG machine profile survived the staging and runs from the installed copy (I-05, I-11).
   Endpoints: 3, all on this machine, none to another address (N-03).
5. `%LOCALAPPDATA%\Adapdf\uninstall.exe /S`: exit code 0. The folder, `Adapdf.lnk` and the
   uninstall entry were gone. A second install, a read of the shortcut (target
   `%LOCALAPPDATA%\Adapdf\adapdf-app.exe`) and a second uninstall gave the same.
6. After: `WINWORD.EXE` not running (before and after each run of the script, and at the
   end), no `adapdf-app.exe`, `java.exe` or `uninstall.exe` left. Left outside the repository,
   not deleted: `HKCU\Software\Adapdf\Adapdf` (existed before), `%LOCALAPPDATA%\com.adapdf.desktop`
   (48,572,325 bytes, unchanged), `%LOCALAPPDATA%\tauri` (125 MB: `NSIS` 7.9 MB, from 2026-10-19,
   and `WixTools314` 117 MB, from 2026-04; both older than M5b). No scratch folder is left in
   `C:\tmp` for the script.

**Checks** (2026-11-17, on the final tree; the page was built by `pnpm build`):

| Command | Result |
|---|---|
| `cargo fmt --all --check` | no difference |
| `cargo clippy --all-targets -- -D warnings`, and with `--all-features` (root) | no warning |
| `cargo clippy -p adapdf-app --all-targets -- -D warnings`, and with `--all-features` | no warning |
| `cargo test` (root) | 775 passed, 0 failed (33 test result lines) |
| `cargo test -p adapdf-app` | 33 passed, 0 failed: 22 in the library, 7 in `frontend_bundle.rs`, **4 in `installer_config.rs`** (new; 29 before) |
| `pnpm exec tsc --noEmit` (in `app/`) | no error |
| `pnpm test` (in `app/`) | 74 passed, 0 failed |
| `node scripts/axe-webview.mjs --installed ... --no-java --watch` | 14 screens, 0 violations, 0 undecided, the export gave Pass |

`installer_config.rs` was checked by four mutations (the install mode `both`, the WebView2
mode `downloadBootstrapper`, the `vendor/` target changed, a path in the publisher): each failed
the test that it should, and the files were restored.

## Known gaps in M5b

- **The installer is not signed.** Windows SmartScreen may show "Windows protected your PC" on
  a file from the network. `docs/install-guide.md` tells the person to click "More info" and
  "Run anyway". A code-signing certificate is a decision for the owner.
- **Only one machine, and not a clean one.** The run was a silent install (`/S`) on the build
  machine, which has Java, Word and the source tree. Nobody saw the screens of an interactive
  install, the shortcut was not clicked, and the installer was not run over an existing
  install (an upgrade) or while the app was running. I-06 (awkward user name), I-07 (no Word)
  and I-09 (a person who did not build it follows the guide) are planned. The guide says what
  the installer's screens do from what Tauri's NSIS template does, not from a look at them.
- **I-05 on a clean machine, and a Java first on the PATH, are not done.** The bundled Java is
  proved by the command line of the process that the app started and by a run with Java taken
  out of the environment. The program names `java.exe` by its full path, so another Java on
  the PATH cannot be used, but this was not run.
- **N-03 is sampling, not a firewall log.** The look is about once in 1.3 seconds, and a
  connection that opens and closes in between is missed. A firewall rule needs admin rights.
- **The uninstaller leaves two things.** `HKCU\Software\Adapdf\Adapdf` (the install folder; it
  was there before the test) and `%LOCALAPPDATA%\com.adapdf.desktop` (WebView2's data; a silent
  uninstall does not delete app data). Both are small. A person who starts the app from the
  Start menu makes the second one (the test runs used a scratch folder for it). Whether the
  uninstaller of an interactive uninstall offers to delete the data was not tried.
- **veraPDF's per-clause profile folders are not installed** (see "How M5b builds the
  installer"). The PDF/UA-1 and WCAG machine profiles run from the installed copy. Other
  veraPDF flavours that the app does not use were not tried.
- **The text of the licence files is a person's job.** `THIRD-PARTY.md` says what is bundled
  and where the licences and the source are; it is not legal advice. The licence texts of
  the Rust crates and the npm packages are not in the installer (`cargo license` on the lock
  file is still to do; see "Rust libraries" in `THIRD-PARTY.md`).
- **Still true from M5a:** the branch is not merged to `main` before the twelve young crates
  of "Known gaps in M5a" are 48 hours old (about 2026-11-18 20:00 UTC). A-05 and the browser
  part of A-06 are not done.

## Known gaps in M1

- **Word is used only in the `windows_word` tests.** Since M3a, W-01
  (`word_opens_every_fixture`) shows that Word opens every fixture, and since
  M3b, S2-15 shows that Word opens every fixed file. `cargo test`
  without the feature still does not use Word. The fixtures are still made by
  code, not by Word.
- **DOC-004 detection is not checked against a document that Word made from
  the template.** The template does not exist yet. The fixture writes the
  attached template the way Word is known to write it, but this is not
  confirmed.
- **Heuristic rules (DOC-003, HDG-003, LST-001) have no false-positive data**
  (nor do IMG-002, IMG-005, IMG-007, TBL-004, CLR-002 and TXT-003: see the
  gaps in M2a and M2b and the Phase 0 table).
  Spec 03 says to keep the heuristics at warning until the sample guides show
  how often they are wrong. That needs Phase 0 files. LST-001 is an error in
  the spec; a paragraph that starts like a list but is not one (for example
  "A. Smith wrote ...") would be reported.
- **Fonts on the machine.** The font reader is tested with font files made by
  the tests. It was also run by hand on this developer machine, where it found
  installed fonts (for example Arial) and a Microsoft 365 cloud font (Aptos).
  That check is not automated. The first font lookup reads all installed font
  files (about 420 MB on this machine). It took about 0.6 seconds in a release
  build and up to 1.3 seconds in the first debug run. There is no test for
  speed.
- **Real documents.** During development `adapdf check` was run by hand on a
  few documents that Word made, to see that it does not fail on real files.
  The results are not kept and are not a test.
- **Model limits.** The model does not apply list-level
  overrides (`w:lvlOverride`) or Word's toggle rule for bold. Theme shade and
  tint follow Word's change of HSL lightness (checked against two palette
  values, not against Word itself). Only the standard (transitional)
  WordprocessingML namespace that Word writes is read. The colour theme is
  read with Word's default mapping (`text1` is `dk1`); `w:clrSchemeMapping` is
  not read. These matter for CLR-001.
