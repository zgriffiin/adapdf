# 02 — Pipeline spec

Status: draft, before the Phase 0 test.

The pipeline turns one .docx into one PDF and one report. Each stage is a
separate module with its own tests. Stages 1, 2, 4 and 6 do not need Word.

## Inputs and outputs

For an input `Guide.docx`, the tool makes one folder next to it, `Guide - Adapdf`,
and writes everything into it. The original stays alone in its folder, and the tool
writes nothing else next to it.

| File in `Guide - Adapdf` | Content |
|---|---|
| `accessible.docx` | The accessible copy: the original with the fixes in it. The tool and Word work on this file from the first fix. It holds the fix log (stage 2). Its name is short on purpose: Word cannot open a file whose full path is longer than 259 characters (see "The path that Word opens"), and the name of the guide is already in the name of the folder. |
| `Guide.pdf` | The tagged PDF, named after the original. Written only when stage 1 finds no errors. |
| `report.html` | The report, for people to read. |
| `report.json` | The same results as data, for later tooling. |

- **The original is never changed.** There are no backup files: the original is the
  backup. The tool never writes a file that has the name of the original.
- The PDF is named after the original (`Guide.pdf`), not after the copy. The
  reports always have the names `report.html` and `report.json`. All three are in
  the folder of the copy.
- A file named `accessible.docx` inside a folder whose name ends with
  ` - Adapdf` is the working copy. Its guide is the name of the folder without
  ` - Adapdf`, and the original is `<name>.docx` next to that folder. A file with
  that name anywhere else is an original like any other.
- `adapdf export Guide.docx` on an original exports the original as it is (no
  fixes) and writes the PDF and the reports into `Guide - Adapdf`. On the copy, it
  exports the copy.

- **The path that Word opens.** Word cannot open a file whose full path is longer
  than 259 characters (measured with Word 16.0, build 16.0.20527: a file at 259
  characters opens, a file at 260 does not, read-only or for editing). Before a
  fix creates or rewrites the copy, and before the app opens the copy in Word, the
  tool counts the characters of the full path of the copy. If the path is longer
  than 259, it stops, writes nothing, and says: "The path of the Word copy would
  be N characters, and Word cannot open files with paths over 259 characters. Move
  the guide to a folder with a shorter path (for example a folder nearer the top of
  OneDrive), then try again." The app and the command line do the same.
- An existing `Guide.pdf` is replaced. If another program has it locked, the
  tool stops with exit code 2 and says which file is locked.
- The new `Guide.pdf` replaces the old one only after it has been checked. If
  the tool fails after the export, the folder is left as it was.
- When stage 1 finds errors, the tool still writes the report. The report
  says "Fail" and lists the errors. It does not write a PDF.
- A PDF that Word exported without tags (stage 4) is an error finding, so
  the exit code is 1.

## Stage 1 — Pre-check

- Opens the .docx as a zip and reads the OOXML parts:
  `word/document.xml`, `styles.xml`, `numbering.xml`, headers, footers,
  `docProps/core.xml`, and `settings.xml`.
- Runs every rule in [03-precheck-rules](03-precheck-rules.md).
- Each finding has: rule ID, severity (`error` or `warning`), a plain-language
  message, a location, a "how to fix" text, and an auto-fix ID if one exists.
- Location = nearest heading above + the first 60 characters of the
  paragraph + paragraph index. Page numbers are not known without Word, so
  they are not used. In the data, the index counts from 0. The text report
  shows it from 1.
- **Where a problem is** is shown in Word's own words (`place` of the location),
  in the apps, the text output and the report. The raw part name of the .docx
  stays in the data (`part`) for tooling only. A body paragraph: the number, the
  start of its text and the heading above it. A header or footer: "In the page
  header of section 2 (all pages)" (or "first page", "even pages"; "of section N"
  is left out when the document has one section, and a part that several sections
  show lists them; a section that names no header of its own shows the one of the
  section before it). The template setting (DOC-004): "Word's template setting
  (Developer > Document Template)". The title (DOC-001): "The document's
  properties (File > Info)". A style: "The style "<name>" (Home > Styles)". A
  table: "The table under the heading "..." (table N)". Text that a message quotes
  (TXT-003, text boxes) keeps the capitals and the spacing of the document.
- **Errors block export.** Warnings do not.

## Stage 2 — Fix

- Shows the auto-fixable findings. The user picks which to apply.
- **The original is never changed.** The fixes go into the accessible copy
  `Guide - Adapdf\accessible.docx`:
  - the first time, the copy is made from the original plus the fixes: a
    temporary file in the folder, then a move into place (atomic). The folder is
    made when it is not there;
  - later, when the working file is the copy, the fixes rewrite the copy the
    same way (temporary file, then move). Before that, the tool checks that Word
    does not have the copy open (it looks for a `~$` lock file, and tries to open
    the copy for writing);
  - the original is only read. It may be open in Word.
- If the user opens an original and its copy already exists, the tool does not
  decide. The app asks: **Continue with the accessible copy** (the default), or
  **Start over from the original**, which replaces the copy, including any
  changes that were made to it in Word, and needs an explicit confirmation. The
  command line stops with a message unless `--continue` (work in the copy) or
  `--start-over` (replace the copy) is given. Start over does not delete the PDF or
  the reports; a new export replaces them.
- Applies the fixes as byte-range splices on the original XML text. Bytes
  outside the splices stay the same.
- **Fix log.** Every fix that is applied is recorded inside the copy, in the same
  write: a custom document property `Adapdf fix log` (`docProps/custom.xml`, with
  its content type and package relationship, added when missing). Its value is a
  compact JSON list, oldest first. Each entry has the local date and time, the
  rule ID, the fix ID, the place of the problem in words, what the tool did, and
  for a fix that asks, the text that the user typed or chose. Later fixes add to
  the list. Word keeps a custom property when a person edits and saves the copy,
  so the log stays.
- Runs stage 1 again and shows the new results.
- Fixes that need human input (for example, alt text or a title) ask for
  the text in the UI. The tool never invents content. For the title only, the UI
  may offer up to three one-click texts that are taken from the document itself:
  the file name (of the original, so for the copy the name of the guide and not
  `accessible`; with underscores and extra spaces cleaned, and a leading `yyyy.mm.dd -` date kept as it is), the first paragraph
  in the Title style, and the first Heading 1. Identical or empty texts are left
  out. Nothing is filled in until the user chooses one, and the user can change
  the text. Alt text has no suggestion (a caption next to the picture would make
  IMG-005 report it).

## Stage 3 — Word export

Uses Word through COM, hidden, with the document opened read-only.

Word never gets the real path of the document. The tool copies the .docx to a
short temporary folder (`%TEMP%\adapdf-<process>-<n>\guide.docx`), Word opens that
copy read-only and writes the PDF into the same folder, and the tool moves the
PDF into the folder of the guide. The temporary folder is removed after a
success, an error and a timeout. Nothing is written back to the .docx. So the
folder of the guide can have any path length that the file system allows, and
the PDF and the reports are written there.

Call `Document.ExportAsFixedFormat` with these parameters:

| Parameter | Value | Meaning |
|---|---|---|
| `OutputFileName` | the PDF path | |
| `ExportFormat` | 17 | `wdExportFormatPDF` |
| `OpenAfterExport` | false | |
| `OptimizeFor` | 0 | `wdExportOptimizeForPrint` |
| `Item` | 0 | `wdExportDocumentContent` (no comments or markup) |
| `IncludeDocProps` | true | |
| `CreateBookmarks` | 1 | `wdExportCreateHeadingBookmarks` |
| `DocStructureTags` | true | The tags. Required. |
| `BitmapMissingFonts` | true | |
| `UseISO19005_1` | false | PDF/A off. It is not needed for PDF/UA. |

Before export, the tool also:

- updates all fields (the table of contents, cross-references);
- stops if the document has tracked changes or comments (rule DOC-005).

Phase 0 must confirm these settings on the users' Word build (M365, version
16.0) and record any setting that changes PAC results.

Rules for COM:

- One Word instance per run, started by the tool and closed at the end, even
  after an error.
- Never use or close a Word window the user already has open.
- Timeout: 5 minutes per document. On timeout, close the instance the tool
  started and report the failure.

## Stage 4 — PDF metadata fixes

Document-level metadata, and the scope of table header cells. The tool never
changes the text, the tag types or the reading order. Uses `lopdf`, and writes
all changes in one incremental update (new bytes at the end of the file; the
existing bytes are not rewritten, and the metadata and the scopes are in the
same appended section). Each change is listed in the report.

| Check | Fix if missing |
|---|---|
| XMP `pdfuaid:part` = 1 | Add it. |
| XMP `dc:title` and the Info `Title` | Copy from the .docx title. |
| `/ViewerPreferences /DisplayDocTitle true` | Set it. |
| Catalog `/Lang` | Copy from the .docx default language. |
| `/MarkInfo /Marked true` and a `/StructTreeRoot` | **Do not fix.** If missing, the export failed. Report an error. |
| A table header cell (`TH`) with no `/Scope` | Add `/Scope`, by the place of the cell in the table. See "Table scopes" below. |

Phase 0 will show which of these Word already writes. Keep only the fixes that
are needed. Phase 0 (the Word 16.0.20527 test, S4-11 and S5-14) showed that Word
writes the title, the language, `DisplayDocTitle` and the tags, and does not write
`pdfuaid:part` or any `/Scope`.

### Table scopes

Word writes a `TH` for the first row and the first column of a table, from
`w:tblLook` (First Row and First Column; no `w:tblLook` means both on), and writes
no `/Scope` for it. veraPDF PDF/UA-1 rule 7.5-1 fails for such a table, and no Word
setting changes this (test plan S5-14). So stage 4 adds the scope. It reads the
structure tree (`/RoleMap` is followed: a custom tag that maps to `Table`, `THead`,
`TBody`, `TFoot`, `TR`, `TH` or `TD` is that element). The rows of a table are
counted in this order: the rows of `THead`, then the rows of `TBody` and the rows
that are directly in the `Table`, then the rows of `TFoot`. The cells of a row are
its `TH` and `TD`, from 1. For each `TH` that has no `/Scope` in its attributes:

1. in a `THead`, or in the first row of the table when that row has no `TD`: the
   scope is `Column`;
2. otherwise, if it is the first cell of its row: the scope is `Row`. (This includes
   the first cell of a first row that also has a `TD`: it is a label, as in a table
   that has only a First Column. A column heading would be wrong there.)
3. otherwise the tool does not guess. It leaves the cell as it is and lists it in
   the report: "header cell not given a scope (table N, row R, cell C)". veraPDF may
   then still fail rule 7.5-1 for the table (see the table of veraPDF rules).

A `TH` that already has a `/Scope` is not changed. The scope is written as the
table attributes `/A << /O /Table /Scope /Column >>` (or `/Row`). When the `TH`
already has `/A`, the old attributes are kept and the new dictionary is added
after them (`/A` becomes an array). Only the `TH` objects change, and nothing in
them but `/A`. A `TH` that is not an object of its own (a dictionary inside its
parent's array) is not changed and is listed as in rule 3. A `TH` in a compressed
object stream is written as a new object with the same number in the appended
section.

The report lists the cells that got a scope for each table, not one line for each
cell: "Table 1: 2 header cells given Column, 3 given Row". Running stage 4 on its
own output changes nothing.

## Stage 5 — veraPDF

- Runs the bundled veraPDF command line with the PDF/UA-1 profile:
  `verapdf --flavour ua1 --format json <pdf>`.
- If the installed veraPDF version has a WCAG machine-check profile, run it as
  well and show its results as warnings until we know how well it matches
  PAC.
- Changes each failed rule into a finding with a plain-language message. Keep
  a table that links common veraPDF rule IDs to the likely cause in Word and
  the fix (for example, "Figure without alt text → IMG-001", and "link without
  a description → LNK-004": Word writes a link's description only from its
  ScreenTip; "table cell with no header → TBL-003": veraPDF 7.5-1 means a table
  that stage 4 could not give scopes to, because it is not regular).
- Any failure means the PDF does not pass.

## Stage 6 — Report

The HTML report must itself be accessible (headings, real tables, good
contrast). It is a static page: it has no form control, because a page with no
script cannot save what is typed or ticked. The PAC result, the checklist and the
sign-off are recorded in the app, and the page shows them as text ("PAC: not
checked yet", "Not checked yet" or "Checked" for each item, "Not signed yet" or
"Signed by <name> on <date>"). It has these sections, in this order:

1. Summary: pass or fail, the pre-check of the Word document (errors and warnings),
   the PDF check (the number of rules of veraPDF PDF/UA-1 that failed), the file
   names, tool version, date. The two checks are shown apart, never added into one
   number.
2. Pre-check findings. Each has its place in Word's words (see "Where a problem
   is" in stage 1).
3. Fixes applied: every fix in the fix log of the document, not only the fixes of
   one run, with the date, the rule, the place, what was done and the text that
   was typed or chosen. The section says: "The original <name>.docx was not
   changed. Adapdf made its fixes in accessible.docx in the folder <name> - Adapdf."
   For a log with no
   entry it says: "No fixes were made by Adapdf. The Word document is <name>."
4. veraPDF result. The first place of each failure is given in words (for
   example "Table 1, body row 1, cell 2") worked out from the structure tree of the
   PDF; veraPDF's own path is kept in a collapsed "Technical detail".
5. PAC: the result, the PAC version and the PAC report file (a file in the folder
   of the guide), recorded in the app.
6. Human checklist and sign-off:
   - Each piece of alt text describes what the image shows, for the guide's purpose.
   - Images marked decorative really are decorative.
   - The reading order makes sense when read aloud. Check with the PAC
     screen-reader preview.
   - Colour is not the only way information is shown.
   - Link text makes sense out of context.
   - Headings describe their section.
   - Tables are data tables, not layout.
7. Technical details: how Word made the PDF (the Word build, the fields that
   were updated, the export time and the parameters of `ExportAsFixedFormat`), and
   the changes made to the PDF after the export (stage 4: the metadata and the table
   scopes), as a subsection "Changes to the PDF after export".

The report is the evidence that the document passed. It is saved in the folder
of the guide, next to the PDF.

## Interfaces

Command line (phases 2–4):

```
adapdf check  Guide.docx            # stage 1 only
adapdf fix    Guide.docx            # stages 1–2, confirms each fix
adapdf export Guide.docx            # stages 1, 3, 4, 5, 6
```

`adapdf fix` never changes `Guide.docx`. The fixes go into
`Guide - Adapdf\accessible.docx`. When that copy exists, `adapdf fix Guide.docx` stops with
exit code 2 and says why, unless a flag is given: `--continue` works in the
copy (its fixes stay and the new fixes go in), and `--start-over` replaces the
copy with a new one made from the original. The two flags exclude each other.
`adapdf fix "Guide - Adapdf\accessible.docx"` works in that copy
directly; `--start-over` is refused for a copy (give the original).

Exit codes: 0 = pass, 1 = errors found, 2 = tool failure.

App (phase 5): choose a file (or drag it onto the window), see the findings
grouped by severity, apply fixes, export, open the report, and sign off the
human checklist.

- Every screen that has a document shows the **working file**: its full name and
  its folder (for example "Working on: accessible.docx in C:\...\Guide -
  Adapdf"), and an **Open folder** button that opens that folder.
- After fixes, the app says: "Saved your fixes to accessible.docx. Your
  original Guide.docx was not changed." The app then works on the copy.
- When the user opens an original whose copy exists, the app asks **Continue with
  the accessible copy** or **Start over from the original** (with the warning that
  the copy is replaced, including changes made in Word, and an explicit confirm).
- **Open in Word** opens the accessible copy in the user's own Word with the
  program that Windows has for .docx files. If only the original is open in the app,
  the app first asks to make the copy, so that the user edits the copy. The app does
  not control Word afterwards: it says "Make your changes in Word, save, then
  choose Check again." While Word has the copy open, a fix says "Close the
  document in Word first, then try again."
- The title field offers the one-click texts of stage 2. "Apply" is off until a
  text is typed or chosen, and the hint says what is missing.
- A static HTML page cannot save a name or a tick, so the app saves the PAC result
  (result, PAC version, PAC report file), the checklist, the name and the date into
  the report (the JSON and the HTML). It never changes the PDF, and the report that
  `export` writes is always unsigned.

## Testing

- Pre-check and fix rules: one small sample .docx for each rule, in
  `tests/fixtures/`, with a passing example and a failing example. Code
  generates the fixtures (`cargo run --example gen-fixtures`). They are not
  built by hand in Word. A test checks that the files on disk match the
  generator byte for byte. A `windows_word` test checks that Word opens each
  one. Do not use real city documents.
- Export and veraPDF: integration tests behind the cargo feature
  `windows_word`. They run only on a machine with Word.
- Fixes: after each fix, run the pre-check again, check that only the target
  XML changed, and check that Word still opens the file.
- Real guides go in `samples/`, which git ignores. Use them for the Phase 0
  test and for manual checks before a release.
- [test-plan](../test-plan.md) lists every test level and links each rule to
  its tests.
