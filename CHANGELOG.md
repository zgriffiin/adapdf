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
  `<name> (accessible).docx` in a folder `<name> - Adapdf` next to the original, through a
  temporary file that is moved into place. They work on that copy from then on. There
  are no backup files any more: the original is the backup. The PDF and the reports go
  into the same folder, and they are named after the original (`Guide.pdf`).
- When the copy of a guide exists, the app asks **Continue with the accessible copy**
  or **Start over from the original** (this replaces the copy, with a confirm). The
  command line stops unless `--continue` or `--start-over` is given.
- The report (schema version 2) shows the Word document pre-check and the PDF check
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
