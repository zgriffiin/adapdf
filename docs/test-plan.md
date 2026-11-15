# Test plan

Status: M2 (all pre-check rules), plus groups, canvases and annotated
screenshots (IMG-007). This plan says what is tested now and what is only planned. A row
marked "planned" has no test yet.

## Scope

M1 built stage 1 of the [pipeline](specs/02-pipeline.md#stage-1--pre-check):
open a .docx, build a typed view of it, run the pre-check rules, and report
findings with `adapdf check`. M1 covered DOC-001 to DOC-006, HDG-001 to
HDG-004, HDG-006, LST-001 and LST-002. (HDG-005, more than one Heading 1, was
removed later: WCAG 2.1 and PDF/UA-1 do not require a single Heading 1.) M2a
added the IMG, TXB and TBL rules and M2b added the LNK, CLR and TXT rules. A
later change added groups and drawing canvases as one object, widened IMG-004,
and added IMG-007 (annotated screenshots). Every rule in
[03-precheck-rules](specs/03-precheck-rules.md) is now built.

Not built: fixes, Word export, PDF metadata fixes, veraPDF, the report, the
app and the installer. The tables at the end list them as planned. The
milestone numbers for them are the plan at the time of writing: M3 fixes, M4
export, PDF metadata, veraPDF and report, M5 app and installer.

## Test levels

| Level | What it checks | Where | Status |
|---|---|---|---|
| Unit | The .docx reader, the document model (styles, numbering, theme, formatting, text boxes, sections), finding types, fonts, text output | `src/**` (`#[cfg(test)]`) | implemented |
| Rule | Each rule against fixtures and small built documents: rule ID, severity, fix kind, location, thresholds | `tests/rules_document.rs`, `tests/rules_headings.rs`, `tests/rules_lists.rs`, `tests/rules_images.rs`, `tests/rules_textboxes.rs`, `tests/rules_tables.rs`, `tests/rules_links.rs`, `tests/rules_colour.rs`, `tests/rules_text.rs` | implemented |
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
| Rule tests for one group | `cargo test --test rules_document` (or `rules_headings`, `rules_lists`, `rules_images`, `rules_textboxes`, `rules_tables`, `rules_links`, `rules_colour`, `rules_text`) |
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

`cargo test` runs 450 tests: 125 unit, 13 CLI, 5 fixture, 31 DOC rule tests,
27 HDG rule tests, 17 LST rule tests, 59 IMG rule tests, 35 TXB rule tests, 29
TBL rule tests, 23 LNK rule tests, 40 CLR rule tests and 46 TXT rule tests. The
count changes as tests are added.
`cargo test -- --list` gives the current list.

## Traceability: pre-check rules

Test names start with the rule ID in lower case, for example
`hdg_003_threshold_is_12_words`. All fixtures are in `tests/fixtures/`.

### Implemented (M1, M2a, M2b and IMG-007)

All 39 rules of spec 03 are implemented, listed in spec order. DOC, HDG and
LST came in M1; IMG, TXB and TBL in M2a (images, text boxes and tables); LNK,
CLR and TXT in M2b (links, colour and text). IMG-007 came after M2. HDG-005
was removed and is not numbered again.

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
| CLR-001 | error, manual | `CLR-001-fail/pass.docx` | `clr_001_*` in `tests/rules_colour.rs`; `model::colour::tests::*` for the WCAG values | implemented |
| CLR-002 | warning, manual | `CLR-002-fail/pass.docx` | `clr_002_*` | implemented (heuristic, see below) |
| TXT-001 | warning, manual | `TXT-001-fail/pass.docx` | `txt_001_*` in `tests/rules_text.rs` (9.5 pt and 10 pt) | implemented |
| TXT-002 | warning, auto | `TXT-002-fail/pass.docx` | `txt_002_*` | implemented |
| TXT-003 | warning, manual | `TXT-003-fail/pass.docx` | `txt_003_*` | implemented (heuristic, see below) |
| TXT-004 | warning, auto | `TXT-004-fail/pass.docx` | `txt_004_*` | implemented |

"auto" and "ask" in this table are the fix kinds the finding reports. The fixes
themselves are not built (planned M3). Severities and fix kinds are checked
against spec 03 in `findings::tests::severities_and_fixes_match_spec_03` (DOC,
HDG, IMG, TXB, LST and TBL) and `severities_and_fixes_match_spec_03` in
`tests/rules_links.rs` (LNK, CLR and TXT). The fixes `AlignLeft` (TXT-002) and
`UpdateFields` (TXT-004) are not built (planned M3).

The model parts for IMG, TXB and TBL are tested in `model::drawings::tests::*`
(including groups, canvases and VML groups) and `model::tables::tests::*`.

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
  is not counted as a heading by HDG-001 and HDG-002, because HDG-004
  reports it.
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
- **HDG-005.** Removed. Spec 04 uses several Heading 1 paragraphs, and WCAG
  2.1 and PDF/UA-1 do not require a single Heading 1. The fixtures, tests and
  spec row are deleted, and the other rules keep their numbers.
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
  exports it only then. Automatic text colour is white when the background has
  a luma (Rec. 601 on the encoded channels) below 0.5, and black otherwise.
  WCAG luminance is not used for this choice, because mid grey (#808080) has
  luminance 0.22 but Word still draws black on it. The contrast itself is the
  WCAG 2.1 formula. Large text is 18 pt or more, or 14 pt or more when bold.
  Thresholds are 4.5 and 3.0, with no rounding up (4.499 fails). Runs with no
  visible text are skipped. One finding for each pair of colours and size
  class in a paragraph. Pattern shading is not blended: a solid pattern draws
  its colour, other patterns draw the fill. Headers, footers and footnotes are
  not checked. Table styles are not applied, so a fill or colour that comes
  only from a table style is not seen. Shade and tint follow Word (a change of
  the HSL lightness; accent 1 `4472C4` with tint `99` is `8FAADC`).
- **CLR-002.** The run colour differs from the colour of the paragraph style
  (automatic and black are the same). Other emphasis stops the finding: bold,
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
  tint follow Word's change of HSL lightness (checked against two palette
  values, not against Word itself). Only the standard (transitional)
  WordprocessingML namespace that Word writes is read. The colour theme is
  read with Word's default mapping (`text1` is `dk1`); `w:clrSchemeMapping` is
  not read. These matter for CLR-001.
- **HDG-005 is removed.** Spec 04 uses several Heading 1 paragraphs and
  neither WCAG 2.1 nor PDF/UA-1 asks for one, so the rule is gone from spec 03
  and from the code.
