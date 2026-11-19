# Changelog

All notable changes to Adapdf are in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

## [0.2.0] - Pre-release

Fixes now go into an accessible copy, and your original is never changed. This
version came from trying 0.1.0 on a real guide. It is still a pre-release: PAC has
not run on real guides, and a person has not yet used the app with a screen
reader. See [the release notes](docs/release-notes/v0.2.0.md) and the
[test plan](docs/test-plan.md).

### Changed

- **The original is never changed.** `adapdf fix` and the app write the fixes into
  `accessible.docx` in a folder `<name> - Adapdf` next to the original, through a
  temporary file that is moved into place. They work on that copy from then on. There
  are no backup files any more: the original is the backup. The PDF and the reports go
  into the same folder: the PDF is named after the original (`Guide.pdf`), and the
  reports are `report.html` and `report.json`.
- **Short file names, and a check for a path that Word cannot open.** A real guide in a
  deep folder failed at the export, because Word cannot open a file whose full path is
  longer than 259 characters (measured), and the first names of the copy repeated the
  long name of the guide in the folder and in the file. The copy is now the short
  `accessible.docx` (a file with that name in a folder `<name> - Adapdf` is the working
  copy). `adapdf fix` and the app stop, and write nothing, when the path of the copy
  would be over the limit, and say to move the guide to a folder with a shorter path.
  **Open in Word** does the same. The export gives Word a copy of the guide in a short
  temporary folder, never the real path, so the PDF and the reports can be in a folder
  with a long path.
- When the copy of a guide exists, the app asks **Continue with the accessible copy**
  or **Start over from the original** (this replaces the copy, with a confirm). The
  command line stops unless `--continue` or `--start-over` is given.
- The report (schema version 3) shows the Word document pre-check and the PDF check
  (veraPDF) apart, lists every fix that was made in the document (not only the fixes of
  one run) and names the original, and puts the technical details last. The HTML report
  has no form control: the PAC result, the checklist and the sign-off are shown as text and
  recorded in the app, which now also saves the PAC result, version and report file. The
  place of a veraPDF failure is given in words ("Table 1, body row 1, cell 2"), with
  veraPDF's path kept as a technical detail.
- Findings say where the problem is in Word's words (for example "In the page header
  of section 2 (all pages)", "The document's properties (File > Info)") and not the
  name of a part of the file. The text that a message quotes (TXT-003) keeps the
  capitals and the spacing of the document. The "how to fix" texts of the manual rules
  say where to click in Word.
- The title is no longer suggested as the text of the first heading.
- **Table header cells get a scope after the export.** Word writes the first row and
  the first column of a table as header cells (`TH`), from the table's First Row and
  First Column settings, and never writes a scope for them. veraPDF fails such a table
  (PDF/UA-1 7.5-1), and no Word setting fixes this. Adapdf now adds the scope in the
  PDF metadata step (`Column` for a heading row, `Row` for a heading in the first
  column) and lists it in the report ("Changes to the PDF after export"). It changes no
  text, tag type or reading order. A heading that it cannot place (in the middle of a
  table) is left as it is and listed. A veraPDF failure 7.5-1 now means an irregular
  table: simplify it (no merged or split cells).
- **TBL-003 (merged or split cells) is now an error.** Word's PDF of such a table fails
  PDF/UA-1 7.2-42 (Word does not record which cells were merged), so the export is
  blocked until the cells are split or the table is made text.
- The fix for TBL-001 now says to mark the first row as a header row only if it holds
  column headings. A label and value table ("Training Topic: | ...") has none.

### Added

- **Open in Word**: opens the accessible copy in your own Word (never the original),
  and says to make the changes there, save, and choose Check again.
- **Title suggestions**: up to three buttons with texts from the document (the file
  name, the Title paragraph, the first Heading 1). Nothing is filled in until you
  choose one.
- **The working file on every screen**: its full name and its folder, with an
  **Open folder** button, and "Saved your fixes to ... Your original ... was not
  changed."
- **Fix log**: every fix is recorded inside the copy as the custom document property
  `Adapdf fix log` (date and time, rule, place, what was done, and the text that you
  typed or chose). It stays when you edit and save the copy in Word.

### Fixed

- The title field on the fix screen was grey on grey and could not be seen. It is now a
  white field with a border. The hint under "Apply fixes" now says what is missing ("Type a
  title, or choose a suggestion, to apply this fix.").

## [0.1.0] - Pre-release

The first release. It is a pre-release: PAC (the checker of record) has not yet
run on real guides, and the installer was tried on one machine only. See
[the release notes](docs/release-notes/v0.1.0.md) and the
[test plan](docs/test-plan.md).

### Added

- **Pre-check of a Word guide** (`adapdf check`): 41 rules of the
  [pre-check rules spec](docs/specs/03-precheck-rules.md) for the document, headings,
  lists, images, text boxes, tables, links, colour and text. Each finding has a
  severity, a place, a plain-language message and how to fix it. This includes
  LNK-004 (a link has no ScreenTip) and LNK-005 (a table of contents made of links),
  which the first real Word and veraPDF runs showed to be needed, and IMG-007 (shapes
  drawn on a screenshot that are not grouped with it).
- **Fixes** (`adapdf fix`): applies the fixes that are safe, after it asks about each
  one. It writes a backup of the `.docx` first, does not write while Word has the file
  open, and never types alt text or a title for you.
- **Word export**: exports a tagged PDF through the person's own Word, in a hidden
  Word instance that Adapdf starts and always closes.
- **PDF/UA metadata**: adds the document-level metadata that Word leaves out when it
  is missing (the PDF/UA identification, the title, the "show the title" setting and
  the language) as an incremental update. It changes no tag and no content.
- **veraPDF check**: runs the bundled veraPDF with the PDF/UA-1 profile, and the WCAG
  2.2 machine profile as warnings. A PDF does not pass when veraPDF reports a PDF/UA-1
  failure.
- **Accessibility report**: an HTML page that works with no script, and a JSON file. It
  holds the findings, the veraPDF result, the places to record the PAC result, and the
  checklist and sign-off of the person who reviews the PDF.
- **Desktop app** (Tauri 2 and IBM Carbon, in `app/`): choose or drop a Word document,
  see the findings by severity, apply fixes, export, open the report, and sign off the
  checklist. It makes no network calls and bundles its fonts.
- **Per-user installer** (NSIS, no admin rights, installs in `%LOCALAPPDATA%\Adapdf`):
  carries the app, veraPDF and a Java runtime (Eclipse Temurin 21), so staff install
  no other program. It is not signed. The licence texts of the third-party parts are
  in the installer (`licenses/THIRD-PARTY-LICENSES.txt`).
- [Install guide](docs/install-guide.md) for staff, and the
  [specs](docs/specs/) and [test plan](docs/test-plan.md).

### Known limits

- The installer is not signed, so Windows SmartScreen shows a warning.
- The pipeline and the installer were tested on one machine with an administrator
  account. A person who did not build Adapdf has not yet followed the install guide.
- The screen-reader and high-contrast walkthrough of the app is not done.
- The Word template for new guides is specified
  ([spec 04](docs/specs/04-word-template.md)) but not built.

[0.2.0]: https://github.com/zgriffiin/adapdf/releases/tag/v0.2.0
[0.1.0]: https://github.com/zgriffiin/adapdf/releases/tag/v0.1.0
