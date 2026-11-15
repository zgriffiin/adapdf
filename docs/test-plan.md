# Test plan

Status: M1. This plan says what is tested now and what is only planned. A row
marked "planned" has no test yet.

## Scope

M1 builds stage 1 of the [pipeline](specs/02-pipeline.md#stage-1--pre-check):
open a .docx, build a typed view of it, run the pre-check rules, and report
findings with `adapdf check`. The rules in scope are DOC-001 to DOC-006,
HDG-001 to HDG-006, LST-001 and LST-002 from
[03-precheck-rules](specs/03-precheck-rules.md).

Not in M1: the other rules, fixes, Word export, PDF metadata fixes, veraPDF,
the report, the app and the installer. The tables at the end list them as
planned. The milestone numbers for them are the plan at the time of writing:
M2 the other pre-check rules, M3 fixes, M4 export, PDF metadata, veraPDF and
report, M5 app and installer.

## Test levels

| Level | What it checks | Where | Status |
|---|---|---|---|
| Unit | The .docx reader, the document model (styles, numbering, theme, formatting, text boxes, sections), finding types, fonts, text output | `src/**` (`#[cfg(test)]`) | implemented |
| Rule | Each rule against fixtures and small built documents: rule ID, severity, fix kind, location, thresholds | `tests/rules_document.rs`, `tests/rules_headings.rs`, `tests/rules_lists.rs`, `tests/rules_images.rs`, `tests/rules_textboxes.rs`, `tests/rules_tables.rs` | implemented |
| Fixture | The files in `tests/fixtures/` match the generator byte for byte. Every rule has a passing and a failing fixture. | `tests/fixtures.rs` | implemented |
| CLI | `adapdf check`: exit codes 0, 1 and 2, text output, JSON shape | `tests/cli.rs` | implemented |
| Word integration (`windows_word`) | Word opens every fixture. Later: Word exports a tagged PDF; fixes leave a file that Word opens. | `tests/`, behind the cargo feature `windows_word` | feature defined; no tests yet (planned M3, M4) |
| veraPDF integration | A PDF exported from a fixture has 0 veraPDF PDF/UA-1 failures | behind `windows_word`, because it needs Word to export | planned (M4) |
| Installer | The per-user installer installs without admin rights, runs, and removes cleanly on a clean Windows 11 machine | manual checklist | planned (M5) |
| Manual: Phase 0 and PAC | Real guides exported from Word, checked with veraPDF and PAC. A person confirms PAC reports 0 errors. | manual, with files in `samples/` | planned. Phase 0 is postponed until real guides are in `samples/`. |

## How to run

| What | Command |
|---|---|
| Everything that runs without Word | `cargo test` |
| Unit tests only | `cargo test --lib` |
| Rule tests for one group | `cargo test --test rules_document` (or `rules_headings`, `rules_lists`, `rules_images`, `rules_textboxes`, `rules_tables`) |
| Tests for one rule | `cargo test hdg_003` (test names start with the rule ID in lower case) |
| Fixture tests | `cargo test --test fixtures` |
| CLI tests | `cargo test --test cli` |
| Write the fixtures again after a change | `cargo run --example gen-fixtures` |
| Word tests (none yet) | `cargo test --features windows_word` |
| Format, lint | `cargo fmt --check` and `cargo clippy --all-targets -- -D warnings` |

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
- **Rule tests check** the rule ID, severity, fix kind, and the location
  (part, paragraph index, snippet, heading above) of the failing fixture.
- **Thresholds have edge tests** on both sides of the limit: exactly 400
  words (no finding) and 401 (finding); 12 words and 13 words for a fake
  heading; 59, 60 and 61 characters for a snippet; 3 digits and 4 digits for a
  typed number; 2 and 3 letters in another script.
- **Font lookup is injected.** DOC-006 tests use a fixed lookup. `SystemFonts`
  is tested with font files that the test writes itself. No test depends on
  the fonts on the machine.
- **No real documents** are in the repository.

## Counts at the time of writing

`cargo test` runs 276 tests on the M2a branch: 89 unit, 13 CLI, 5 fixture, 31
DOC rule tests, 31 HDG rule tests, 17 LST rule tests, 34 IMG rule tests, 27 TXB
rule tests and 29 TBL rule tests. The count changes as tests are added.
`cargo test -- --list` gives the current list.

## Traceability: pre-check rules

Test names start with the rule ID in lower case, for example
`hdg_003_threshold_is_12_words`. All fixtures are in `tests/fixtures/`.

### Implemented in M1

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
| HDG-005 | warning, manual | `HDG-005-fail/pass.docx` | `hdg_005_*` | implemented |
| HDG-006 | warning, manual | `HDG-006-fail/pass.docx` | `hdg_006_*` | implemented |
| LST-001 | error, auto | `LST-001-fail/pass.docx` | `lst_001_*` in `tests/rules_lists.rs`; `rules::lists::tests::*` for the marker patterns | implemented |
| LST-002 | warning, manual | `LST-002-fail/pass.docx` | `lst_002_*` | implemented |

"auto" and "ask" in this table are the fix kinds the finding reports. The fixes
themselves are not built (planned M3). Severities and fix kinds for all 14
rules are checked against spec 03 in
`findings::tests::severities_and_fixes_match_spec_03`.

### Implemented in M2a

| Rule | Severity, fix | Fixtures | Tests | Status |
|---|---|---|---|---|
| IMG-001 | error, ask | `IMG-001-fail/pass.docx` | `img_001_*` in `tests/rules_images.rs` | implemented |
| IMG-002 | warning, ask | `IMG-002-fail/pass.docx` | `img_002_*`; `rules::images::tests::*` for the placeholder patterns | implemented (heuristic) |
| IMG-003 | warning, manual | `IMG-003-fail/pass.docx` | `img_003_*` | implemented |
| IMG-004 | warning, auto | `IMG-004-fail/pass.docx` | `img_004_*` | implemented (pictures only, see below) |
| IMG-005 | warning, manual | `IMG-005-fail/pass.docx` | `img_005_*` | implemented (heuristic) |
| IMG-006 | warning, manual | `IMG-006-fail/pass.docx` | `img_006_*` | implemented (fixture uses an equation; OLE is tested in memory) |
| TXB-001 | error, manual | `TXB-001-fail/pass.docx` | `txb_001_*` in `tests/rules_textboxes.rs` | implemented |
| TXB-002 | warning, manual | `TXB-002-fail/pass.docx` | `txb_002_*` | implemented |
| TXB-003 | warning, manual | `TXB-003-fail/pass.docx` | `txb_003_*`; `rules::textboxes::tests::*` | implemented |
| TXB-004 | warning, auto | `TXB-004-fail/pass.docx` | `txb_004_*` | implemented |
| TBL-001 | error, auto | `TBL-001-fail/pass.docx` | `tbl_001_*` in `tests/rules_tables.rs` | implemented |
| TBL-002 | error, manual | `TBL-002-fail/pass.docx` | `tbl_002_*` | implemented |
| TBL-003 | warning, manual | `TBL-003-fail/pass.docx` | `tbl_003_*` | implemented |
| TBL-004 | warning, manual | `TBL-004-fail/pass.docx` | `tbl_004_*` | implemented (heuristic) |
| TBL-005 | warning, auto | `TBL-005-fail/pass.docx` | `tbl_005_*` | implemented |
| TBL-006 | warning, auto | `TBL-006-fail/pass.docx` | `tbl_006_*` | implemented |

The model parts are tested in `model::drawings::tests::*` and
`model::tables::tests::*`.

### Planned (M2)

None of these rules, fixtures or tests exist yet. Each will get
`<ID>-fail.docx` and `<ID>-pass.docx` and tests that name the rule.

| Rule | Status |
|---|---|
| LNK-001, LNK-002, LNK-003 | planned (M2) |
| CLR-001, CLR-002 | planned (M2) |
| TXT-001, TXT-002, TXT-003, TXT-004 | planned (M2) |

## Traceability: pipeline and interfaces

| Item (spec 02) | Tests | Status |
|---|---|---|
| Stage 1: open the .docx as a zip and read the parts | `docx::tests::*`, `model::tests::*`; CLI tests for a file that is not a .docx, a zip with no `word/document.xml`, damaged XML, and a missing file | implemented |
| Stage 1: finding fields and location | `findings::tests::*`, the location checks in the rule tests, `json_shape_of_a_finding` in `tests/cli.rs` | implemented |
| Stage 1: errors block export | The exit code 1 on errors is tested (`exit_code_1_when_there_are_errors`). The export that is blocked does not exist yet. | partly implemented |
| Interface `adapdf check`, exit codes 0, 1, 2, `--json` | `tests/cli.rs` | implemented |
| Stage 2: fix, backup, lock-file check | | planned (M3) |
| Interface `adapdf fix` | | planned (M3) |
| Stage 3: Word export through COM | | planned (M4) |
| Stage 4: PDF metadata fixes | | planned (M4) |
| Stage 5: veraPDF run and message table | | planned (M4) |
| Stage 6: HTML and JSON report, and its own accessibility | | planned (M4) |
| Interface `adapdf export` | | planned (M4) |
| App (file picker, findings list, export, report) | | planned (M5) |
| Installer | | planned (M5) |
| Word template and acceptance (spec 04) | Manual check in Word; `sample-all-styles.docx` must give 0 errors and 0 warnings (DOC-004 aside) | planned (template phase) |
| Phase 0 and PAC | Manual | planned (postponed) |

## How M1 reads the rules

Spec 03 gives each check in one line. These are the decisions M1 made where
the line leaves room. Change the code and the tests together if the spec
changes.

- **Paragraphs.** Rules look at body paragraphs in reading order, including
  paragraphs in table cells. Text in text boxes is not in that list. The
  paragraph index counts from 0 in the data and from 1 in the text report.
- **Heading.** A paragraph in a built-in Heading 1 to Heading 9 style (by the
  style's name) that has some text. Title is not a heading. An empty heading
  is not counted as a heading by HDG-001, HDG-002 and HDG-005, because
  HDG-004 reports it.
- **Formatting.** "Bold", "larger than body text" and the language are the
  effective values: document defaults, then the paragraph style and its base
  styles, then the character style, then direct formatting. Table styles are
  not applied. Word's rule that bold in a style and in a character style
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
- **HDG-005.** One finding, at the second Heading 1.
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
  other (shape, group, canvas). A drawing with no graphic data is a picture.
  VML is a picture when it has `v:imagedata`, an embedded object when it has
  `o:OLEObject` (or is a `w:object`), and a shape otherwise.
- **IMG-001.** Applies to pictures, charts, shapes and SmartArt. It does not
  apply to embedded objects (IMG-006) or to a text box that has text
  (TXB-001 reports it).
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
- **IMG-004.** Pictures only (the spec says "image"). Floating charts, shapes
  and SmartArt are not reported. VML floats when its style has
  `position:absolute`. A floating decorative picture is still reported.
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
  tables inside a text box are not read.
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

## Known gaps in M2a

- **Fixtures have no media parts.** The pictures in the IMG fixtures have no
  image data, and charts, SmartArt and embedded objects exist only in memory
  tests. Word may show the pictures as missing. The planned `windows_word`
  test that opens every fixture will need a small image part in the builder.
- **Drawings inside text boxes** and in headers, footers, footnotes and
  endnotes are not read.
- **Horizontal lines** (VML `o:hr`) are shapes without alt text, so IMG-001
  reports them. Word does not report them. Not seen in a real guide yet.
- **IMG-006 does not say how Word exports** an equation or an object. Phase 0
  decides.
- **No false-positive data** for IMG-002, IMG-005 and TBL-004.

## Known gaps in M1

- **Word is not used in any test.** No test checks that Word opens the
  generated fixtures. This is planned under `windows_word`. Until then the
  fixtures are only known to be valid for this tool's reader.
- **DOC-004 detection is not checked against a document that Word made from
  the template.** The template does not exist yet. The fixture writes the
  attached template the way Word is known to write it, but this is not
  confirmed.
- **Heuristic rules (DOC-003, HDG-003, LST-001) have no false-positive data.**
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
- **Model limits.** The model does not apply table styles, list-level
  overrides (`w:lvlOverride`) or Word's toggle rule for bold. Theme shade and
  tint are applied per colour channel, which is an approximation of Word.
  Only the standard (transitional) WordprocessingML namespace that Word writes
  is read. These matter for later rules such as CLR-001.
- **Spec 03 and spec 04 disagree on HDG-005.** Spec 04 uses Title for the
  guide title and Heading 1 for sections, so a guide made with the template has
  several Heading 1 paragraphs. HDG-005 (more than one Heading 1) would warn
  on it. M1 follows spec 03. This needs a decision.
