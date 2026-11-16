# Test plan

Status: M2 (all pre-check rules), plus groups, canvases and annotated
screenshots (IMG-007), M3a (the Word automation layer, the first `windows_word`
tests, and table styles in the colour rules) and M3b (stage 2: the fix engine and
`adapdf fix`), including the stage 2 tests that need Word. This plan says what is
tested now and what is only planned. A row marked "planned" has no test yet.

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

Not built: Word export (only the base in `src/word` exists), PDF metadata
fixes, veraPDF, the report, the app and the installer. "Planned tests (not
built)" lists each of them, requirement by requirement.

**Milestones and phases.** The milestone numbers are not the phase numbers of
[spec 01](specs/01-product.md#phases). M1 and M2 (with M2a and M2b) together
are phase 2 (pre-check). M3 (fixes) is phase 4. M4 (Word export, PDF metadata
fixes, veraPDF and the report) is phase 3. M5 (app and installer) is phase 5.
Phase 0 (test with real guides) and phase 1 (template) have no milestone. M3a is
the first part of M3: the Word automation layer, test W-01 and the table style
colours. The milestone numbers are the plan at the time of writing.

## Test levels

| Level | What it checks | Where | Status |
|---|---|---|---|
| Unit | The .docx reader, the document model (styles, table styles, numbering, theme, formatting, text boxes, sections), finding types, fonts, text output | `src/**` (`#[cfg(test)]`) | implemented |
| Rule | Each rule against fixtures and small built documents: rule ID, severity, fix kind, location, thresholds | `tests/rules_document.rs`, `tests/rules_headings.rs`, `tests/rules_lists.rs`, `tests/rules_images.rs`, `tests/rules_textboxes.rs`, `tests/rules_tables.rs`, `tests/rules_links.rs`, `tests/rules_colour.rs`, `tests/rules_text.rs` | implemented |
| Fixture | The files in `tests/fixtures/` match the generator byte for byte. Every rule has a passing and a failing fixture. | `tests/fixtures.rs` | implemented |
| CLI | `adapdf check`: exit codes 0, 1 and 2, text output, JSON shape | `tests/cli.rs` | implemented |
| Fix (stage 2) | The fix engine on fixtures and small built documents: only the spliced bytes change, the backup, the lock file, the atomic write, each fix and its refusals, and the stage 1 re-run. `adapdf fix`: the questions, the answers on stdin, and the exit codes. | `tests/fix_safety.rs`, `tests/fix_engine.rs`, `tests/fix_fixes.rs`, `tests/cli_fix.rs` (helpers in `tests/fix_support/`), and unit tests in `src/fix/` | implemented |
| Word integration (`windows_word`) | Word opens every fixture (W-01). The session quits its Word after success, an error and a panic. The model's table style colours match what Word shows. Word's lock file name, and a file that Word has open is never written (S2-02, S2-04). Every fixed file opens in Word and Word reads the fix as the model does (S2-15). Later: Word exports a tagged PDF. | `tests/word.rs`, `tests/word_table_styles.rs` and `tests/fix_word.rs` (helpers in `tests/word_support/`), behind the cargo feature `windows_word` | implemented for W-01 to W-04 (M3a, 6 tests) and S2-02, S2-04, S2-15 and the Word part of S2-23 (4 tests); export tests planned (M4) |
| veraPDF integration | A PDF exported from a fixture has 0 veraPDF PDF/UA-1 failures | behind `windows_word`, because it needs Word to export | planned (M4) |
| Front-end | The Carbon views in the web UI: each screen, axe-core checks for WCAG 2.1 A and AA, and the focus order (TypeScript) | front-end folder, with Vitest and axe-core | planned (M5) |
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
| veraPDF | The bundled veraPDF and Java runtime are in place. For tests on Word exports, the `windows_word` entry applies too. | The exit criterion in "Stage 5" below holds for the named fixtures. |
| Front-end (M5) | The web UI builds (its front-end and the Tauri shell). No Word or Java is needed. | `pnpm test` passes with 0 failed. axe-core reports 0 violations for WCAG 2.1 A and AA on each screen. The build passes N-05. |
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
| Stage 2 (fix) tests | `cargo test --test fix_safety`, `--test fix_engine`, `--test fix_fixes` and `--test cli_fix`; unit tests: `cargo test --lib fix::` |
| Tests for one fix | `cargo test fix_` or `cargo test ask_` (the names in the stage 2 table) |
| Write the fixtures again after a change | `cargo run --example gen-fixtures` |
| Word tests (need desktop Word; they start their own hidden Word; about 3 minutes) | `cargo test --features windows_word` (also runs everything above). One file: `cargo test --features windows_word --test word`, `--test word_table_styles` or `--test fix_word` |
| Front-end tests (none yet) | `pnpm test` in the front-end folder |
| Format, lint | `cargo fmt --check` and `cargo clippy --all-targets --all-features -- -D warnings` |

veraPDF, installer and manual tests will get their steps here when they exist.

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
  all four fields. All 39 failing-fixture tests use it. The snippet
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

Measured on 2026-11-16 on commit `0864900` (M3 with the changes of review
round 1). Commands: `cargo test` and
`cargo test --features windows_word`, with the counts taken from their "test
result" lines. Two tests of M3b run on Windows only (the held file handle in
S2-03, and the folder with the name of the backup in S2-06), and so does
`the_production_clock_is_local_time` (S2-07); the counts include them.

`cargo test` runs 588 tests: 169 unit (150 before M3b; 14 new in `src/fix/`; 5 new
in `src/word/` for the process choice), 14
CLI, 13 CLI for `fix`, 14 fix engine, 38 fix (one test for each fix, and more), 12
fix safety, 8 fixture, 31 DOC rule tests, 28 HDG rule tests, 17 LST rule tests,
61 IMG rule tests, 35 TXB rule tests, 29 TBL rule tests, 23 LNK rule tests, 50
CLR rule tests and 46 TXT rule tests. `cargo test --features windows_word` runs
those 588 and 10 more: 5 in `tests/word.rs`, 1 in `tests/word_table_styles.rs`
and 4 in `tests/fix_word.rs`, so 598. The run took 2 minutes 45 seconds. The count
changes as tests are added. `cargo test -- --list` gives the current list.

## Traceability: pre-check rules

Test names start with the rule ID in lower case, for example
`hdg_003_threshold_is_12_words`. All fixtures are in `tests/fixtures/`.

### Implemented (M1, M2a, M2b and IMG-007)

All 39 rules of spec 03 are implemented, listed in spec order. DOC, HDG and
LST came in M1; IMG, TXB and TBL in M2a (images, text boxes and tables); LNK,
CLR and TXT in M2b (links, colour and text). IMG-007 came after M2.

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
| Stage 1: errors block export | The exit code 1 on errors is tested (`exit_code_1_when_there_are_errors`). The export that is blocked does not exist yet. | partly implemented |
| Interface `adapdf check`, exit codes 0, 1, 2, `--json` | `tests/cli.rs` | implemented |
| Stage 2: fix, backup, lock-file check | "Stage 2: fixes" below (S2-01 to S2-31): `tests/fix_safety.rs`, `tests/fix_engine.rs`, `tests/fix_fixes.rs`, `fix::*` unit tests | implemented (M3b). S2-02, S2-04 and S2-15 (`tests/fix_word.rs`, windows_word) are implemented. S2-23 is implemented for the part that Word can answer; its manual part is planned. |
| Stage 2: the user picks the fixes, and ask fixes take typed text | S2-11 to S2-13; `adapdf fix` rows C-01 to C-04 | implemented (M3b) |
| Interface `adapdf fix` | C-01 to C-04: `tests/cli_fix.rs` | implemented (M3b) |
| Stage 3: Word export through COM | S3-01 to S3-19 | planned (M4) |
| Stage 4: PDF metadata fixes | S4-01 to S4-12 | planned (M4) |
| Stage 5: veraPDF run and message table | S5-01 to S5-11 | planned (M4) |
| Stage 6: HTML and JSON report, and its own accessibility | S6-01 to S6-11 | planned (M4) |
| Interface `adapdf export` | C-05 to C-08 | planned (M4) |
| Fixtures open in Word (spec 02, Testing) | W-01 (`word_opens_every_fixture`) | implemented (M3a) |
| Word automation base: own hidden Word, always quit (spec 02, "Rules for COM") | `tests/word.rs`: `the_session_quits_word_after_a_panic`, `the_session_quits_word_after_an_error`, `a_damaged_docx_gives_an_error_and_not_a_hang`, `a_missing_file_is_an_error` (windows_word). These cover the base only; the export rows S3-01 to S3-19 stay planned. | implemented (M3a) |
| App (file picker, findings list, export, report) | A-01 to A-07 | planned (M5) |
| Installer | I-01 to I-09 | planned (M5) |
| Word template and acceptance (spec 04) | T-01 to T-10 | planned (phase 1, template) |
| No network (spec 01) | N-01 to N-05 | planned |
| Phase 0 and PAC | The Phase 0 tables and P0-01 to P0-09 | planned (postponed) |

## Planned tests (not built)

Nothing in this section is built, except the rows W-01 to W-04 in "Fixture
realism", which are marked implemented, and the stage 2 rows and the `fix` rows
(C-01 to C-04), which have a status column that says what was built in M3b. Each
row says what a test must show, so that
the person who builds the stage knows when it is done. Test names are
proposals. Levels: **unit** (no Word, Java or network), **CLI** (runs the
`adapdf` binary), **windows_word** (needs Word, feature-gated), **veraPDF**
(runs the bundled veraPDF), **front-end** (TypeScript, Vitest and
axe-core) and **manual** (a person, with a record). The
columns are: requirement (with the spec it comes from), planned test name,
level, fixture or input, and the pass condition.

Test seams the unit rows need. They are design requirements of the tests. The
seam of stage 2 is built (M3b): the file system calls go through the trait
`fix::FileSystem`, which a test can record or make fail, and the clock is a
value in `fix::Environment`. The others are not in the code yet: the Word COM
calls of stage 3 go
through a small trait with a fake that records calls and can block, fail or
panic; the Word timeout and the path of veraPDF can be set by the
caller.

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
6. **An existing `Guide.pdf` is replaced.** A PDF that another program has
   locked gives exit 2 with a message that names the program's file.
7. **A report is written when stage 1 blocks the export.** It says "Fail" and
   lists the errors. No PDF is written.
8. **A PDF with no tags** (`/MarkInfo` or `/StructTreeRoot` missing) **is
   exit 1**. It is an error finding about the document, not a tool failure.
9. **`AlignLeft` (TXT-002) sets a direct setting.** The fix sets
   `w:jc w:val="left"` in the paragraph's own `w:pPr`. It does not change the
   style, so no other paragraph changes.

### Fixture realism (stage 1)

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| W-01 | Word opens every fixture (spec 02, "Testing") | `word_opens_every_fixture` (`tests/word.rs`) | windows_word | Every `tests/fixtures/*.docx` (78 files). The pictures point to a real PNG part. | Each file opens read-only in the test's own hidden Word (one session, all open at once), with no repair and no error, and Word counts at least one paragraph in it. The `WINWORD.EXE` processes after the test are the same processes as before. **Implemented (M3a).** Result on 2026-11-16, Word 16.0.20527: all 78 open; the person's own Word (PID 49988) was running and was not touched. Word also opened the fixtures before the image part was added (a picture with no image data is shown as missing, not rejected), so no fixture builder had to be fixed for Word. |
| W-02 | Word rejects a damaged file with an error and without a dialog (spec 02: a file Word cannot open is an error) | `a_damaged_docx_gives_an_error_and_not_a_hang` | windows_word | Half of a .docx; a plain text file named .docx; a zip with `word/document.xml` that is not valid XML | `WordSession::open_read_only` gives `WordError::Open` with Word's message for each, within 90 seconds. **Implemented (M3a).** A file with 0 bytes is not an error: Word opens it as an empty document (stage 1 refuses it first). |
| W-03 | The session quits its Word after a panic and after an error | `the_session_quits_word_after_a_panic`, `the_session_quits_word_after_an_error` | windows_word | A panic inside `catch_unwind` while the session is alive; `open_read_only` on a file that does not exist | The `WINWORD.EXE` processes are the same as before within 20 seconds. **Implemented (M3a).** The "error" case uses `WordError::FileNotFound`, which the session finds itself without asking Word. An error that Word reports is in W-02, which also compares the process list after it. |
| W-04 | The model's table style colours are what Word shows (CLR-001 and CLR-002 depend on them) | `table_style_colours_match_word` (`tests/word_table_styles.rs`) | windows_word | 29 test tables in 2 documents: every conditional format with its own colour and fill; `tblLook` in both forms; band sizes; header rows; merged cells; `basedOn`; a fill set in the cell; a paragraph style over the table style | For every cell, the fill and the text colour that Word reports (`Cell.Shading.BackgroundPatternColor`, `Range.Font.Color`) equal the model's. **Implemented (M3a).** Result on 2026-11-16, Word 16.0.20527: 453 of 453 cells equal. The test follows the other Word tests: it takes `exclusive()`, records the `WINWORD.EXE` ids before, runs under `within` (300 seconds), compares the ids again with `wait_for_word_pids`, and keeps its files in a `word_support::TempDir`, which is removed also when the test fails. |

### Stage 2: fixes (spec 02, stage 2)

Built in M3b (the engine is `src/fix/`; `fix::apply` works on bytes, `fix::fix_file` adds
the safety rules, `fix::offers` lists what can be fixed). Four rows (S2-02, S2-04,
S2-15 and S2-23) need Word. They are built on the Word module of M3a, in
`tests/fix_word.rs`; the manual part of S2-23 is still planned. The names in the
table are the names of the tests.

Fix IDs in `src/findings.rs`: 11 auto (`SetDefaultLanguage`,
`ApplyHeadingStyle`, `DeleteEmptyHeading`, `ApplyListStyle`,
`MakeImageInline`, `DeleteEmptyParagraphs`, `MarkHeaderRow`, `DeleteEmptyRow`,
`DisallowRowBreak`, `AlignLeft`, `UpdateFields`) and 3 ask (`SetTitle`,
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
| S2-08 | Only the target XML bytes change; all other parts and all bytes outside the changes are identical (spec 02 stage 2; spec 01) | `fix_changes_only_the_spliced_bytes` | unit | The failing fixture of every fix ID in the rows below; the busy Word-like case of `new_properties_go_in_schema_order_in_a_busy_paragraph_and_row` | Every part but the target has equal bytes after unzip, and the entries have the same names and order. The target part equals the original with a table of exact edits and has no other change. For each edit the table has the exact original text that goes and the exact new text; it is written by hand for each fix from its fixture, and it is not the engine's splice list, so a fix that writes a whole paragraph again (or widens its splice) fails. The splice list of the engine must also agree with the result and must not overlap. | implemented: `fix_changes_only_the_spliced_bytes` and `fix_changes_only_the_target_part` in `tests/fix_engine.rs`. The first uses `assert_exactly_these_edits` with the table `expected_edits()` (14 fixes, 24 edits, for example HDG-004 removes one whole `<w:p>...</w:p>` and TXT-002 changes `both` to `left` and nothing else), and also `assert_only_the_splices_changed`, which only checks the engine against its own splice list. The busy case in `tests/fix_fixes.rs` goes through `assert_exactly_these_edits` too (4 edits; its styles define Heading 1, so the style part does not change; an undefined Heading style that is added is S2-18). **Mutation check, 2026-11-16:** a temporary change made the `AlignLeft` fix write its paragraph again with `xml:space="preserve"` removed. The new check failed for TXT-002 ("word/document.xml is the original with these edits and no other change"), and `fix_reruns_stage_1_with_every_auto_problem_in_one_document`, which uses only the splice-list check, still passed. A second temporary change wrote the `w:pStyle` element again with an added space, and the busy case failed. Both changes were removed. It runs for every fix that writes (14 rules). `untouched_entries_are_copied_raw` checks that untouched entries keep their compressed bytes, method and CRC. `a_byte_order_mark_is_kept` checks a part with a byte order mark. |
| S2-09 | The written file is a valid .docx | `fix_result_is_a_valid_docx` | unit | Same | The zip opens, the target part parses as XML, and `Document::from_bytes` reads it. | implemented: `fix_result_is_a_valid_docx` in `tests/fix_engine.rs`. |
| S2-10 | Stage 1 runs again after the fixes and shows the new results (spec 02) | `fix_reruns_stage_1_and_the_fixed_finding_is_gone` | unit | Each failing fixture in the rows below, then one document with every auto problem | The finding of the rule is gone. No new finding appears, except those the rows list. After all auto fixes the document has no auto finding left (decided point 1 aside); the manual findings are the same rules and counts as before. | implemented: `fix_reruns_stage_1_and_the_fixed_finding_is_gone` and `fix_reruns_stage_1_with_every_auto_problem_in_one_document` in `tests/fix_engine.rs`. The extra findings that the plan allows (TBL-004 after S2-28) are tested in the test of that fix. |
| S2-11 | The user picks which fixes to apply (spec 02) | `fix_applies_only_the_fixes_the_user_picked` | unit | A document with 3 auto problems; the user picks 1 | Only that rule's finding is gone. The other two remain. Bytes outside that splice are equal. | implemented: `fix_applies_only_the_fixes_the_user_picked` in `tests/fix_engine.rs`. `the_same_text_cannot_be_changed_by_two_different_fixes`, `two_fixes_on_the_same_drawing_do_not_overlap` and the tests for one `w:pPr` and one `w:trPr` check fixes that meet in one place. |
| S2-12 | Ask fixes never write content the user did not type (AGENTS.md; spec 02) | `ask_fix_without_text_writes_nothing` | unit | Each ask fix ID with empty input and with spaces only; `DOC-001-fail.docx` shows the suggestion "Test guide" but the user types nothing | No write, no backup. The finding remains. The suggestion is never used as input. | implemented: `ask_fix_without_text_writes_nothing` in `tests/fix_engine.rs`. For `MarkAnnotationsDecorative` the "input" is the confirmation: text or "decorative" is refused. `cli_fix_shows_the_suggestion_but_never_fills_it_in` in `tests/cli_fix.rs` checks the CLI. |
| S2-13 | The typed text is what is written | `ask_fix_writes_exactly_the_typed_text` | unit | Text `A & B <C> "D" é 日本` for the title and for alt text | The XML is escaped. Reading the file back gives the same text. Nothing else is added. | implemented: `ask_fix_writes_exactly_the_typed_text` in `tests/fix_engine.rs`. Line breaks and tabs in alt text are written as character references and read back. |
| S2-14 | A failed write does not damage the source | `fix_write_is_atomic` | unit | The writer fails after half of the bytes | The original file is intact. No partial file stays. | implemented: `fix_write_is_atomic` in `tests/fix_safety.rs`: the writer fails after half of the bytes, and the move fails. The backup stays in both cases (it is a whole copy). |
| S2-15 | Word still opens the fixed file (spec 02, "Testing") | `fix_result_opens_in_word` (`tests/fix_word.rs`) | windows_word | Every failing fixture that has an auto or ask fix (14), fixed with its fix by `fix_file` in a folder under the temp folder; also HDG-003 with the Heading 2 style not defined (decided point 4), alt text typed on a picture, shape, group, canvas, VML picture, VML shape and VML group, and "Mark as decorative". 23 cases | In one hidden Word, the original and the fixed file both open read-only (repair off, alerts off) with no error. For documents with no table, the change of Word's paragraph count equals the change of the model's. Word reads the fix as the model does: title (`BuiltInDocumentProperties`), the style of the heading (`Heading 2`) and of the list paragraphs (`List Number`), the alt text (`InlineShapes(1).AlternativeText`), a floating picture that is inline after the fix, the header row (`Rows(1).HeadingFormat`), the number of rows, `AllowBreakAcrossPages`, and left alignment (`Paragraphs(n).Alignment` is 3 before, 0 after). | **Implemented.** Result on 2026-11-16, Word 16.0.20527: 23 of 23 cases open and agree. The paragraph count is compared as a change, not as a total: Word also counts the end of each table row, so the totals of Word and the model differ. TXT-004 is left out because its fix writes nothing. DOC-002, IMG-007 and the decorative mark only have to open (the checks that Word has for them, language and decorative, are not read). The in-memory chart and SmartArt images are left out: Word cannot open even their original files (see "Known gaps in M3b"). Word gives an error when it is asked for the alt text of a document that is not the active one, so `WordDocument::inline_shape_alt_text` first activates the (hidden) document. |
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

### Stage 3: Word export (spec 02, stage 3)

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| S3-01 | One hidden Word started by the tool | `export_starts_its_own_hidden_word` | windows_word | `DOC-001-pass.docx` | While the export runs, a `WINWORD.EXE` process exists that was not in the list taken before. Its `Visible` is false and its alerts are off. |
| S3-02 | The user's Word windows are not used or closed (AGENTS.md; spec 02) | `export_leaves_the_users_word_alone` | windows_word | The test first starts a visible "user" Word with fixture B open and an unsaved edit. It notes the process ID, the number of documents, the text, `Saved` (false) and the file bytes. Then it exports fixture A. | After the export, the user's process is alive, has the same number of documents and text, `Saved` is still false, the file bytes are the same, and its window still exists. A variant exports the same file the user has open. |
| S3-03 | The instance is closed after success | `export_closes_word_after_success` | windows_word | `DOC-001-pass.docx` | The list of `WINWORD.EXE` process IDs equals the list before, within 30 seconds. No `~$` file remains. The .docx bytes are unchanged (it was opened read-only). The bound was 5 seconds in the first plan; it is too tight. Measured on 2026-11-16 (Word 16.0.20527): Word needed 3.1 to 4.9 seconds to end after `Quit` in 14 quits (3.7 seconds with 78 documents open, in W-01), and 8.6 seconds in one more (W-04, a session that had opened and closed 2 documents). A reviewer saw a process that stayed longer than 10 seconds. So the session waits up to 30 seconds, and this row uses the same bound. |
| S3-04 | The instance is closed after an error | `export_closes_word_after_an_error` | windows_word | An output folder that does not exist; a read-only output folder; a file that is a valid zip with a damaged `word/document.xml` (stage 1 skipped by a test hook) | Exit 2 each time. The process list equals the list before. No partial PDF remains. |
| S3-05 | Closed also when the code panics | `export_closes_the_instance_when_the_code_panics` | unit | A fake Word that panics inside the export call | The fake records `Quit`. |
| S3-06 | Timeout: close the instance the tool started and report the failure (spec 02) | `export_times_out_and_closes_only_its_own_word` | unit | A fake whose export blocks for 2 seconds, with the timeout set to 100 ms; a second fake "user" instance | The result is a timeout error and exit 2. `Quit` or terminate is called on the tool's instance only. No PDF file. |
| S3-07 | The timeout is 5 minutes (spec 02) | `export_timeout_is_five_minutes_by_default` | unit | The default value | 300 seconds. |
| S3-08 | Timeout with the real Word | `export_times_out_with_real_word` | windows_word | `DOC-001-pass.docx` with the timeout set to 1 ms | Exit 2. No `WINWORD.EXE` of the tool remains. No partial PDF. |
| S3-09 | `ExportAsFixedFormat` parameters (spec 02, table) and a read-only open | `export_passes_the_spec_parameters` | unit | A fake that records the call | `OutputFileName` is the PDF path next to the .docx; `ExportFormat` 17; `OpenAfterExport` false; `OptimizeFor` 0; `Item` 0; `IncludeDocProps` true; `CreateBookmarks` 1; `DocStructureTags` true; `BitmapMissingFonts` true; `UseISO19005_1` false. The document is opened read-only, hidden, and not added to recent files. |
| S3-10 | The parameters have the effect the spec says | `export_makes_a_tagged_pdf_with_bookmarks_and_properties` | windows_word | A pass fixture with a Heading 1 and a Heading 2 | The PDF has `/MarkInfo` with `/Marked true`, a `/StructTreeRoot`, `/Outlines` for the headings, an Info `/Title`, and no `pdfaid` in the XMP (PDF/A off). |
| S3-11 | Fields are updated before the export (spec 02) | `export_updates_fields_before_exporting` | unit, then windows_word | Unit: a fake that records the order. windows_word: `TXT-004-fail.docx` (a heading renamed after the table of contents was made) | Unit: the field update call comes before `ExportAsFixedFormat`. Word: the PDF text has the new heading in the table of contents and not the old one. The .docx on disk is unchanged. This also proves S2-31. |
| S3-12 | Export is refused when DOC-005 fires (spec 02) | `export_is_refused_when_doc_005_fires` | unit | `DOC-005-fail.docx` (tracked change); a copy with only a comment | Exit 1. The fake Word is never called. No PDF. The message names tracked changes or comments. |
| S3-13 | The PDF is written only when stage 1 has no errors (spec 02, "Inputs and outputs") | `export_writes_no_pdf_when_stage_1_has_errors` | CLI | Each of the 15 `*-fail.docx` fixtures whose rule is an error (DOC-001, 002, 005, 006; HDG-001, 002, 004; IMG-001, 007; TXB-001; LST-001; TBL-001, 002; LNK-003; CLR-001) | Exit 1. No `.pdf` next to the file. Word is not started. `Guide.accessibility.html` exists, its summary says Fail, and it lists the errors (decided point 7). |
| S3-14 | Warnings do not block the export | `export_runs_when_only_warnings_exist` | windows_word | `HDG-006-fail.docx` and `TXT-002-fail.docx` | A PDF is written. The report lists the warnings. |
| S3-15 | The source .docx is not changed by the export | `export_leaves_the_docx_unchanged` | windows_word | Every exportable pass fixture (see stage 5) | Bytes and modified time are the same before and after. |
| S3-16 | Word is not installed | `export_reports_missing_word` | unit | A fake factory that fails with "class not registered" | Exit 2. The message says Microsoft Word is not installed. No panic. |
| S3-17 | A document Word cannot open (for example a password) | `export_reports_a_password_protected_document` | windows_word | A copy that the test saves with a password through Word | Exit 2 with a clear message. The process list equals the list before. |
| S3-18 | Awkward paths | `export_handles_awkward_paths` | windows_word | A folder and file name with spaces and a non-ASCII letter; a long path | The PDF is written next to the file. |
| S3-19 | Existing or locked `Guide.pdf` (decided point 6) | `export_replaces_or_reports_a_locked_pdf` | windows_word | A `Guide.pdf` from an earlier run; another with an exclusive handle held | The first is replaced. The second gives exit 2 with a clear message. |

### Stage 4: PDF metadata fixes (spec 02, stage 4)

Fixtures: small PDFs made by test code (the planned `lopdf` dependency can
write them), kept in `tests/pdf-fixtures/` with a drift test like the .docx
fixtures. Names: `tagged-complete`, `missing-pdfua`, `missing-title`,
`missing-displaydoctitle`, `missing-lang`, `no-markinfo`, `no-structtreeroot`
and `all-missing`, each with a classic cross-reference table and, where it
matters, a cross-reference stream.

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| S4-01 | Incremental update: new bytes at the end, existing bytes not rewritten | `pdfmeta_output_starts_with_the_original_bytes` | unit | `all-missing` (classic and stream variants) | The original bytes are a byte prefix of the output. The output is longer. The new tail has its own cross-reference section whose `/Prev` points to the old one. |
| S4-02 | XMP `pdfuaid:part` = 1 | `pdfmeta_adds_pdfuaid_part_1` | unit | `missing-pdfua`; a copy that already has 1; a copy that has 2 | Missing: added, other XMP content kept. Already 1: no change. A value other than 1 is reported as an error and not changed. |
| S4-03 | XMP `dc:title` and Info `Title` from the .docx title | `pdfmeta_copies_the_title` | unit | `missing-title` with the .docx title "Adapdf test guide"; a copy missing only one of the two | Both are set to the .docx title; only the missing one is added when one is present. An empty .docx title: nothing is invented and an error is reported. |
| S4-04 | `/ViewerPreferences /DisplayDocTitle true` | `pdfmeta_sets_display_doc_title` | unit | `missing-displaydoctitle`; a copy with a `ViewerPreferences` that has other keys; a copy with it true | Set to true. Other keys are kept. Already true: no change. |
| S4-05 | Catalog `/Lang` from the .docx default language | `pdfmeta_copies_the_language` | unit | `missing-lang` with default language en-US; a copy that has `/Lang` | `/Lang (en-US)` is added. An existing value is not changed. |
| S4-06 | `/MarkInfo /Marked true` and `/StructTreeRoot` are not fixed; the error is reported (spec 02) | `pdfmeta_reports_missing_markinfo_and_structtreeroot` | unit | `no-markinfo`, `no-structtreeroot` | No `/MarkInfo` or `/StructTreeRoot` object is in the appended bytes. An error finding says the export failed (decided point 8). |
| S4-07 | The content and the tags are never changed (AGENTS.md; spec 01) | `pdfmeta_does_not_touch_content_or_tags` | unit | `all-missing` | The objects in the appended tail are only the catalog, the Info dictionary and the metadata stream (and `ViewerPreferences` if it is an indirect object). No page, content stream, font or structure element is redefined. The catalog's `/StructTreeRoot` and `/Pages` references are the same as before. |
| S4-08 | Nothing missing means no change | `pdfmeta_no_change_when_nothing_is_missing` | unit | `tagged-complete` | The output bytes equal the input. The report says there were no changes. |
| S4-09 | Each change is listed in the report | `pdfmeta_changes_are_listed` | unit | Each fixture above | The list of changes (what, before, after) matches the differences found by comparing the two files. |
| S4-10 | The result is a readable PDF and the metadata failures are gone | `pdfmeta_output_passes_verapdf_metadata_checks` | veraPDF | Outputs of S4-02 to S4-05 | veraPDF reads the file. The metadata-related failures that the input had are gone. |
| S4-11 | What Word already writes (spec 02: "keep only the fixes that are needed") | `pdfmeta_on_a_real_word_export` | windows_word | A pass fixture exported by Word | Values that Word wrote are left alone. The values that were added are recorded in the Phase 0 table. |
| S4-12 | A PDF the tool cannot read | `pdfmeta_reports_a_pdf_it_cannot_read` | unit | A truncated PDF; an encrypted PDF | Exit 2. The input file is unchanged. |

### Stage 5: veraPDF (spec 02, stage 5)

Fixtures: JSON output of veraPDF for small PDFs made by test code, saved in
`tests/verapdf/`.

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| S5-01 | The command is `verapdf --flavour ua1 --format json <pdf>` | `verapdf_runs_the_ua1_profile_command` | unit | A fake runner that records the command | Arguments are exactly `--flavour ua1 --format json` and the PDF path. The bundled veraPDF is used, not one on the PATH. |
| S5-02 | Each failed rule becomes a finding with a plain message | `verapdf_json_failed_rules_become_findings` | unit | A saved JSON with 3 failed rules, 2 of them mapped | 3 findings. Each has the clause, the test number, a message, how to fix, and the context path. |
| S5-03 | A table links common veraPDF rule IDs to the Word cause and the fix (for example "Figure without alt text" to IMG-001) | `verapdf_every_mapped_rule_has_a_message` | unit | The table, and one saved JSON for each mapped ID | Every mapped ID gives its message and its linked pre-check rule, which is in `RuleId::ALL`. The table has no ID without a test. |
| S5-04 | An ID that is not in the table is still shown | `verapdf_unmapped_rule_keeps_the_original_text` | unit | A saved JSON with an unmapped rule | A finding with the original clause text and ID. It is not dropped. |
| S5-05 | The WCAG machine-check profile, if installed, is shown as warnings (spec 02) | `verapdf_wcag_profile_results_are_warnings` | unit | A fake runner for a veraPDF that has the profile, and one that does not | Profile results have severity warning and do not change pass or fail. Without the profile there is no second run, and the report says it was not available. |
| S5-06 | veraPDF or Java is missing: exit 2 with a clear message | `verapdf_missing_gives_exit_2` | CLI | The path of veraPDF set to a folder that does not exist; a veraPDF folder without the Java runtime | Exit 2 in both. stderr names what is missing. stdout is empty. |
| S5-07 | A crash is not a pass | `verapdf_crash_or_bad_output_is_not_a_pass` | unit | A runner that exits with an error, gives empty output, or gives invalid JSON | Exit 2, not pass. |
| S5-08 | Any failure means the PDF does not pass | `verapdf_any_failure_means_not_pass` | unit | JSON with 1 failed check; JSON with 0 failed checks | 1 failed check: not pass, exit 1. 0 failed: pass. |
| S5-09 | The real veraPDF runs on a PDF made by test code | `verapdf_runs_on_a_generated_pdf` | veraPDF | `tagged-complete` | The JSON parses and the failure count is the one the test knows. |
| S5-10 | **Exit criterion.** Word-made PDFs of passing fixtures have 0 veraPDF failures (spec 01, success criterion 1, applied to the fixtures) | `verapdf_zero_failures_on_pass_fixtures` | veraPDF with windows_word | The 33 pass fixtures that have content and need no picture data: DOC-001 to DOC-006, HDG-001 to HDG-004, HDG-006, LST-001, LST-002, TXB-001 to TXB-004, TBL-001 to TBL-006, LNK-001 to LNK-003, CLR-001, CLR-002, TXT-001 to TXT-004 and IMG-006. | Each is exported by Word and checked. 0 failures for each. The 6 pass fixtures with a picture (IMG-001 to IMG-005 and IMG-007) are added when the builder writes a small image part (see "Known gaps in M2a"). Failing fixtures are not in the criterion, because stage 1 blocks those with errors. A failure here is a finding for Phase 0: a missing source rule or a Word export problem. |
| S5-11 | Information only: veraPDF on failing fixtures that have only warnings | `verapdf_on_warning_fixtures_is_recorded` | veraPDF with windows_word | The `*-fail` fixtures of warning rules that can be exported | The veraPDF failures are recorded, to see which warnings need to become errors. No pass condition. |

### Stage 6: report (spec 02, stage 6)

The sections of the report are the seven in spec 02. "Automated HTML test"
means a test that parses the report and checks it. If the report is written as
well-formed XHTML, `roxmltree` can parse it with no new dependency; otherwise
the test needs a parser, which needs a decision.

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| S6-01 | The 7 sections are present, in order: Summary; Pre-check findings; Fixes applied; PDF metadata changes; veraPDF result; PAC; Human checklist | `report_has_the_seven_sections_in_order` | unit | A fixed set of results | The `h2` headings are those seven, in that order. A section with no content says so ("None"). |
| S6-02 | Summary: pass or fail, number of errors and warnings, tool version, date | `report_summary_fields` | unit | Results with 2 errors and 3 warnings; a fixed clock | The summary shows fail, 2, 3, the crate version and the date. Pass or fail is in words, not only colour. |
| S6-03 | Fixes applied, including the backup file name | `report_lists_fixes_and_the_backup_name` | unit | Results with 2 fixes and the backup `Guide.backup-20260305-101530.docx` | Both fixes and the exact backup name are in section 3. |
| S6-04 | PAC: a place to attach or link the PAC report file, with the result | `report_has_a_pac_field` | unit | Any | Section 6 has labelled fields for the PAC report file and the result. |
| S6-05 | Human checklist with a name and date for sign-off, the seven items of spec 02 | `report_checklist_items_and_signoff` | unit | Any | The seven items are there word for word. Each has a labelled checkbox. Name and date have labelled fields. |
| S6-06 | The HTML report is itself accessible: headings, real tables with `th` and `scope`, `lang`, title, contrast | `report_html_is_accessible` | unit | A report with findings | `html` has `lang`. `title` is not empty. One `h1`, and no skipped level. Every data table has `th` with `scope`, and a caption. No table is used for layout. Every form control has a label. Every colour pair in the style sheet has a contrast of at least 4.5:1, checked with `contrast_ratio`. Pass or fail is never shown by colour alone. No link text is one of the vague phrases of LNK-001. |
| S6-07 | Document text cannot break the report | `report_escapes_text_from_the_document` | unit | A finding whose snippet is `<script>alert(1)</script> & "x"` | The HTML shows the text; there is no `script` element. The JSON has the text unchanged. |
| S6-08 | The JSON has the same results as the HTML | `report_json_matches_html` | unit | The same fixed results | The same findings (rule ID, message, location), counts, fixes, metadata changes, veraPDF failures and backup name appear in both. |
| S6-09 | The files are `Guide.accessibility.html` and `Guide.accessibility.json`, next to the PDF | `report_files_are_written_next_to_the_pdf` | CLI | `export` on a pass fixture; `export` on an error fixture (decided point 7) | Both files exist with these names. |
| S6-10 | No outside resources | `report_loads_nothing_from_the_network` | unit | A report | No `src`, `href` to a network address, or `@import`; the style sheet is inside the file. |
| S6-11 | The report works with a screen reader | `report_read_with_a_screen_reader` | manual | A report from a failing fixture with several findings, opened in a browser | With NVDA and with Narrator: the seven sections can be reached from the headings list; each table is read with its column headers; the checklist and the sign-off fields can be used with the keyboard only; the pass or fail result is read. Done once for each release, with the screen reader and browser versions noted. |

### CLI: `fix` and `export` exit codes (spec 02, "Interfaces")

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| C-01 | `fix`: 0 when no error remains after the fixes (or nothing to fix) | `cli_fix_exit_0_when_no_error_remains` | CLI | `DOC-002-fail.docx`, fix confirmed | Exit 0. A backup exists. Re-running `check` gives exit 0. Status: implemented in `tests/cli_fix.rs`: `cli_fix_exit_0_when_no_error_remains`, and `cli_fix_exit_0_when_there_is_nothing_to_fix` (no backup). |
| C-02 | `fix`: 1 when errors remain after the fixes | `cli_fix_exit_1_when_errors_remain` | CLI | `HDG-002-fail.docx` (manual error); a document with one auto and one manual error | Exit 1. The auto fix was applied. The manual error is listed. Status: implemented: `cli_fix_exit_1_when_errors_remain` (both inputs). The answers come from stdin. |
| C-03 | `fix`: 2 for a tool failure | `cli_fix_exit_2_on_tool_failure` | CLI | A locked file; a file that is not a .docx; a missing file; a backup that cannot be written; wrong arguments | Exit 2. stderr says why. stdout is empty. The file is unchanged. Status: implemented: `cli_fix_exit_2_on_tool_failure` (locked file, not a .docx, missing file, wrong arguments), and, on Windows, `cli_fix_exit_2_when_the_backup_cannot_be_written` (folders with the names of the backups of the next 30 seconds, so that the real clock hits one). The questions of `fix` go to stderr, and the result goes to stdout only after the file is written, so stdout is empty on every exit 2. |
| C-04 | `fix` asks before each fix | `cli_fix_confirms_each_fix` | CLI | A document with 3 auto problems, with answers piped in: yes, no, yes | The first and the third are applied. The second is not. Status: implemented: `cli_fix_confirms_each_fix` (answers y, n, y). More: `cli_fix_applies_nothing_when_every_answer_is_no_or_the_input_ends`, `cli_fix_asks_again_for_an_answer_it_does_not_understand`, `cli_fix_shows_the_suggestion_but_never_fills_it_in`, `cli_fix_asks_for_alt_text_or_decorative`, `cli_fix_asks_before_marking_the_arrows_decorative`, `cli_fix_says_what_it_cannot_do_and_what_word_does` and `cli_fix_writes_the_text_for_the_picture_it_was_asked_about`. `adapdf fix` has no flags. |
| C-05 | `export`: 0 on a pass | `cli_export_exit_0_on_a_pass` | windows_word | `DOC-001-pass.docx` | Exit 0. `Guide.pdf` and the two report files exist. |
| C-06 | `export`: 1 for stage 1 errors, veraPDF failures or a PDF with no tags | `cli_export_exit_1_on_errors` | CLI | `HDG-002-fail.docx` (no PDF); a stage 5 fake that reports 1 failure; the stage 4 input `no-structtreeroot`, an untagged PDF (decided point 8) | Exit 1 in all three. |
| C-07 | `export`: 2 for a tool failure (Word missing, timeout, veraPDF or Java missing) | `cli_export_exit_2_on_tool_failure` | CLI | The fakes of S3-16, S3-06 and S5-06 | Exit 2. stdout is empty. |
| C-08 | The current test that `export` is not in this version | `exit_code_2_for_wrong_usage` in `tests/cli.rs` | CLI | `adapdf export a.docx` | This test must be changed when `export` is built. |

### No network (spec 01, constraints; AGENTS.md)

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| N-01 | The shipped program has no HTTP client | `no_http_client_in_the_dependency_graph` | unit | `Cargo.lock` (read as text) and the `windows` crate features in `Cargo.toml` | None of `reqwest`, `hyper`, `ureq`, `curl`, `attohttpc`, `isahc`, `surf`, `h2`, `native-tls`, `rustls`, `openssl` is a package. No `windows` feature for networking (`Win32_Networking`, `Win32_NetworkManagement`). Equivalent check with `cargo tree -e normal`. |
| N-02 | A full run works with the network off | `export_runs_with_the_network_disabled` | manual | A pass fixture, on a machine with the network adapters disabled | The same PDF and the same veraPDF result as with the network on. |
| N-03 | The tool, the bundled Java and WebView2 make no connection | `no_connection_attempts_in_a_full_run` | manual | A Windows Firewall rule that blocks outbound traffic for `adapdf.exe`, the bundled `java.exe` and `msedgewebview2.exe`, with logging on, and a full `export` | The run succeeds and the firewall log has no entry for those three programs. |
| N-04 | Only made-up documents are in git (AGENTS.md) | `repo_has_only_made_up_documents` | unit | `git ls-files` | The tracked `.docx` and `.pdf` files are the catalogue, the PDF fixtures and the template files, and nothing else. |
| N-05 | The web UI loads nothing from the network (spec 01, constraints) | `frontend_bundle_has_no_remote_urls` | CLI, after the front-end build | The built front-end assets, and `tauri.conf.json` | 0 matches for `http://` and `https://` in `src`, `href`, `url()` and `@import`, except Tauri's own IPC origin (`http://ipc.localhost`, a local address inside WebView2) and XML namespace strings, which the test lists by name. In the CSP, `default-src`, `connect-src`, `font-src` and `img-src` allow only `'self'` and Tauri's IPC scheme. The URL scan finds addresses written into the files; the CSP check also blocks script requests such as `fetch()`, so both are needed. |

Carbon's Sass loads IBM Plex from a CDN by default. The build must point the
font path at the bundled files, or N-05 fails.

### App (spec 02, "Interfaces"; spec 01, technology)

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| A-01 | Findings are grouped by severity | `app_groups_findings_by_severity` | front-end (TypeScript) | The view model for `LST-002-fail.docx` | Errors first, then warnings, with counts. |
| A-02 | Fix and export buttons follow the state | `app_buttons_follow_the_state` | front-end (TypeScript) | The view model with no fix picked; with errors; with none | "Apply fixes" is off when nothing is picked. "Export" is off while errors remain. |
| A-03 | The tool never fills in alt text or a title (AGENTS.md) | `app_never_prefills_typed_text` | front-end (TypeScript) | `DOC-001-fail.docx` and `IMG-001-fail.docx` | The input field is empty. The suggestion is shown as a separate hint. |
| A-04 | The app is usable with a screen reader (spec 01: Carbon in WebView2) | `app_widgets_have_names_and_roles` | front-end (TypeScript) | Each screen, tested with Vitest and axe-core (`pnpm test`) | axe-core reports 0 violations for WCAG 2.1 A and AA on each screen. The keyboard focus order follows the reading order. |
| A-05 | Keyboard, screen reader and display settings | `app_manual_walkthrough` | manual | The app on Windows 11 | The whole flow (choose a file, see findings, apply a fix, export, open the report) works with the keyboard alone; Narrator and NVDA read each step; the app is usable at 200% scaling and in a high-contrast theme; dragging a file onto the window works. |
| A-06 | The report opens from the app | `app_opens_the_report` | manual | After an export | The report opens from a local file path in the default browser. |
| A-07 | The Tauri commands call the same pipeline as the CLI | `app_commands_match_the_cli` | unit (Rust); the `export` case is windows_word | `check`, `fix` and `export` commands on `LST-002-fail.docx`, `TXB-004-fail.docx` and a pass fixture | Each command returns the same findings, fix results and the same pass, errors or tool-failure result (exit 0, 1 or 2 in the CLI) as the matching CLI command on the same input. |

### Installer (spec 01, constraints and technology)

Most rows are manual, on a clean Windows 11 machine with a standard user. I-02
and I-08 are unit tests of `tauri.conf.json`.

| # | Requirement | Planned test name | Level | Fixture or input | Pass condition |
|---|---|---|---|---|---|
| I-01 | No admin rights | `installer_asks_for_no_admin_rights` | manual | Run the installer as a standard user | No UAC prompt. Files are in `%LOCALAPPDATA%`. Nothing is written under `C:\Program Files` or to `HKLM`. |
| I-02 | The installer is set to install for the current user only | `installer_config_is_per_user` | unit | `tauri.conf.json` | `bundle.windows.nsis.installMode` is `currentUser`, so the install folder is under `%LOCALAPPDATA%` and no admin prompt is shown. |
| I-03 | Start menu entry | `installer_adds_a_start_menu_entry` | manual | After install | An entry for the current user exists and starts the app. |
| I-04 | Uninstall removes the files | `installer_uninstall_removes_files` | manual | Uninstall from Settings | The install folder and the Start menu entry are gone. Reports and backups next to the user's documents are not touched. |
| I-05 | The bundled veraPDF and Java run on a machine with no Java | `installer_runs_verapdf_without_java` | manual | `where java` finds nothing and `JAVA_HOME` is unset; then an `export` of a pass fixture | The export runs veraPDF from the bundle. A second run with another Java first on the PATH still uses the bundled one. |
| I-06 | Awkward user names | `installer_with_awkward_user_name` | manual | A user name with a space and a non-ASCII letter | Install, run and uninstall work. |
| I-07 | Word is missing | `app_without_word` | manual | A machine with no Word | `check` works. `export` gives exit 2 with a clear message. |
| I-08 | The installer does not download WebView2 (Windows 11 has it; spec 01, technology) | `installer_skips_webview2_download` | unit | `tauri.conf.json` | `bundle.windows.webviewInstallMode` is `{ "type": "skip" }`. Any other mode can download WebView2, which is a network call. |
| I-09 | The install guide works for a new user (spec 01, phase 5) | `installer_guide_followed_without_help` | manual | The install guide, on a clean Windows 11 user account with no admin rights. A person who did not build the app follows it. | The person installs, runs a check and uninstalls without help. Any help given is a fail. |

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
  stage 3 (S3-06 to S3-08). The session gives the process id for that
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
  LNK-003).
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
