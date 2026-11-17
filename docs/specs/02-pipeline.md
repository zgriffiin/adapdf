# 02 — Pipeline spec

Status: draft, before the Phase 0 test.

The pipeline turns one .docx into one PDF and one report. Each stage is a
separate module with its own tests. Stages 1, 2, 4 and 6 do not need Word.

## Inputs and outputs

For an input `Guide.docx`, the tool writes these files next to it:

| File | Content |
|---|---|
| `Guide.pdf` | The tagged PDF. Written only when stage 1 finds no errors. |
| `Guide.accessibility.html` | The report, for people to read. |
| `Guide.accessibility.json` | The same results as data, for later tooling. |
| `Guide.backup-<yyyymmdd-hhmmss>.docx` | A copy of the source, written before any fix. |

The output folder can be changed in settings. The tool never overwrites the
source without first writing a backup.

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
- **Errors block export.** Warnings do not.

## Stage 2 — Fix

- Shows the auto-fixable findings. The user picks which to apply.
- Before writing anything:
  - checks that Word does not have the file open (looks for a `~$` lock
    file, and tries to open the file for writing);
  - writes the backup copy.
- Applies the fixes as byte-range splices on the original XML text, then
  saves the .docx. Bytes outside the splices stay the same.
- Runs stage 1 again and shows the new results.
- Fixes that need human input (for example, alt text or a title) ask for
  the text in the UI. The tool never invents content.

## Stage 3 — Word export

Uses Word through COM, hidden, with the document opened read-only:

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

Only document-level metadata. The tool never changes the content or the tags.
Uses `lopdf`, and writes the change as an incremental update (new bytes at the
end of the file; the existing bytes are not rewritten). Each change is listed
in the report.

| Check | Fix if missing |
|---|---|
| XMP `pdfuaid:part` = 1 | Add it. |
| XMP `dc:title` and the Info `Title` | Copy from the .docx title. |
| `/ViewerPreferences /DisplayDocTitle true` | Set it. |
| Catalog `/Lang` | Copy from the .docx default language. |
| `/MarkInfo /Marked true` and a `/StructTreeRoot` | **Do not fix.** If missing, the export failed. Report an error. |

Phase 0 will show which of these Word already writes. Keep only the fixes that
are needed.

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
  ScreenTip).
- Any failure means the PDF does not pass.

## Stage 6 — Report

The HTML report must itself be accessible (headings, real tables, good
contrast). It has these sections:

1. Summary: pass or fail, number of errors and warnings, tool version, date.
2. Pre-check findings.
3. Fixes applied, including the backup file name.
4. PDF metadata changes.
5. veraPDF result.
6. PAC: a place to attach or link the PAC report file, with the result.
7. Human checklist, with a name and date for sign-off:
   - Each piece of alt text describes what the image shows, for the guide's purpose.
   - Images marked decorative really are decorative.
   - The reading order makes sense when read aloud. Check with the PAC
     screen-reader preview.
   - Colour is not the only way information is shown.
   - Link text makes sense out of context.
   - Headings describe their section.
   - Tables are data tables, not layout.

The report is the evidence that the document passed. It is saved next to the
PDF.

## Interfaces

Command line (phases 2–4):

```
adapdf check  Guide.docx            # stage 1 only
adapdf fix    Guide.docx            # stages 1–2, confirms each fix
adapdf export Guide.docx            # stages 1, 3, 4, 5, 6
```

Exit codes: 0 = pass, 1 = errors found, 2 = tool failure.

App (phase 5): choose a file (or drag it onto the window), see the findings
grouped by severity, apply fixes, export, and open the report. Optional, if
Phase 0 shows COM can do it reliably: a "Show in Word" button that opens the
document and selects the problem.

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
