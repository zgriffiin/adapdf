# Changelog

All notable changes to Adapdf are in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](https://semver.org/).

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

[0.1.0]: https://github.com/zgriffiin/adapdf/releases/tag/v0.1.0
