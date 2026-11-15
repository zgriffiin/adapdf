# 01 — Product spec

Status: draft, before the Phase 0 test.

## Problem

City staff write training guides in Word and publish them as PDFs. The PDFs
must be accessible under the ADA. Today the only tool that gets them to pass
is Adobe Acrobat Pro, which is a subscription. A chat-agent approach was tried
and failed: the PDFs still had errors.

## Goal

A tool installed on staff machines that turns a Word document into a PDF with
**zero errors** in the checker of record, and as few warnings as possible.
The city pays once to build it and nothing after that.

## Users

- 3 staff members on Windows 11 with Microsoft 365 Word.
- They create many documents each month, mostly training guides.
- They are not accessibility specialists. The tool must explain each problem
  in plain language and tell them how to fix it.

## Standards

| Standard | Why it applies |
|---|---|
| WCAG 2.1 Level AA | The DOJ ADA Title II rule (April 2024) makes it the requirement for state and local governments. Confirm the current deadlines and exceptions with the city's ADA coordinator. |
| PDF/UA-1 (ISO 14289-1) | The PDF standard for accessibility. The Matterhorn Protocol lists its tests. Both checkers below are based on it. |

PDF/UA-2 exists, but PDF/UA-1 has the widest checker support today. Target
UA-1. Look at UA-2 again later.

## Checkers

| Checker | Role |
|---|---|
| **veraPDF** (free, open source, command line) | Automated check. The tool runs it on every export and does not let a PDF pass if veraPDF reports a failure. |
| **PAC** (free, from axes4, Windows app) | Checker of record. A person runs it on the final PDF and saves its report as evidence. PAC also checks some WCAG items that veraPDF does not. |

"No errors" means: veraPDF PDF/UA-1 reports 0 failures **and** PAC reports
0 errors. Warnings are allowed, but the tool reports each one.

## Constraints

- **No subscriptions or per-use fees.** One-time costs are allowed, for
  example a perpetual licence if Phase 0 shows one is needed.
- **Runs locally.** Documents stay on the user's machine. Nothing is sent to a
  cloud service.
- **No admin rights to install.** The app runs from a user folder.
- **Uses the Word install the users already have.** Word does the PDF export.
- **Fix the source, not the PDF.** We have no PDF tag editor. Every error must
  be fixed in the .docx, then the PDF is exported again. The only change made
  to the PDF after export is document-level metadata (see
  [02-pipeline](02-pipeline.md)).

## Scope

In scope:

- Checking a .docx and explaining each accessibility problem.
- Safe automatic fixes in the .docx, after the user confirms.
- Tagged PDF export through Word.
- Metadata fixes in the PDF after export.
- veraPDF check and a report for each document.
- A checklist for the items a person must review.
- An accessible Word template for new guides
  ([04-word-template](04-word-template.md)).

Out of scope (v1):

- PDFs that did not come from Word, scanned documents, OCR.
- PDF forms (fillable fields).
- Editing PDF tags.
- AI-written alt text. It would need a paid API or a weaker local model.
  Reconsider after v1.
- Languages other than English. The tool sets the language, but all rules and
  messages are in English.

## Success criteria

1. Every Phase 0 sample guide, after fixes in Word, exports with 0 veraPDF
   failures and 0 PAC errors.
2. A user can go from an unchecked .docx to a passing PDF and report without
   help, by following the tool's messages.
3. Each published PDF has a saved report: pre-check results, fixes applied,
   veraPDF result, PAC report, and the human checklist signed with a name and
   date.

## Approach

```
.docx ──► 1 Pre-check ──► 2 Fix (user confirms) ──► 3 Word export ──► 4 PDF metadata fixes ──► 5 veraPDF ──► 6 Report
             ▲                                                                                     │
             └──────────── errors left? fix in Word, run again ◄───────────────────────────────────┘
                                                                         then: PAC + human checklist (manual)
```

Details are in [02-pipeline](02-pipeline.md). Pre-check rules are in
[03-precheck-rules](03-precheck-rules.md).

## Technology

| Part | Choice | Why |
|---|---|---|
| Language | Rust (stable, edition 2024) | Type-safe and memory-safe. One program with no runtime to install. No `unsafe` code, except where Windows COM needs it. |
| .docx reading | `zip` and `roxmltree` on the OOXML parts | Works without Word, so tests run fast and on any machine. `roxmltree` is read-only and gives the byte range of every XML node. |
| .docx fixes | Byte-range splices on the original XML text | A fix changes only the bytes it must change. Every other part and byte stays the same. |
| PDF export | Word through COM (the `windows` crate), `Document.ExportAsFixedFormat` | Most accurate tags for Word content. Already installed. |
| PDF metadata fixes | `lopdf`, written as an incremental update | Free. Adds the metadata change at the end of the file and leaves the content and tags as they are. |
| Automated check | veraPDF command line, with a bundled Java runtime (Eclipse Temurin) | Free. No separate Java install for users. |
| UI | Tauri 2 with IBM Carbon (`@carbon/react`), in WebView2 | The Rust pipeline runs behind a web UI. Carbon components are built for WCAG AA. Fonts (IBM Plex) and styles are bundled, so the app makes no network calls. WebView2 is part of Windows 11. |
| Packaging | Tauri NSIS installer, per user (`installMode: currentUser`) | Installs to `%LOCALAPPDATA%` with no admin rights. Adds a Start menu entry and an uninstaller. |

Alternative considered: a Word ribbon add-in (C#/VSTO). It would be nicer to
use inside Word, but it needs IT to deploy and is harder to test. Reconsider
after v1 if users want it.

## Phases

Phase 0 is postponed until real guides are in `samples/`. Until then, the
code is built to the assumptions in these specs, and Phase 0 may change them.

| Phase | Result |
|---|---|
| **0 — Test** | Export 5 real guides from Word with tags on. Run veraPDF and PAC. List each error and say whether it comes from the source document or from Word's export. Confirm the Word export settings and whether a perpetual-licence engine (Aspose.Words) is needed. Confirm the checker of record with the city. |
| **1 — Template** | Accessible Word template and author quick guide. Can run alongside phase 2. |
| **2 — Pre-check (command line)** | All error-level rules in [03](03-precheck-rules.md), with tests based on sample .docx files. |
| **3 — Export and check** | Word export, PDF metadata fixes, veraPDF run, report. |
| **4 — Fixes** | Automatic fixes with backup and confirmation. |
| **5 — App and packaging** | Tauri and Carbon UI, Tauri per-user installer, install guide for the 3 users. |

## Open questions

| # | Question | Who decides | Why it matters |
|---|---|---|---|
| Q1 | Does the city accept a PAC report as evidence, or does it use another checker (for example Acrobat's)? | City ADA coordinator | Sets what "zero errors" is measured against. |
| Q2 | Which Word export problems remain after the source is fixed? | Phase 0 result | Decides whether Word export is enough or a perpetual licence for Aspose.Words is needed. |
| Q3 | City brand: colours, fonts, logo? | City communications | Template colours must still pass contrast checks. |
| Q4 | Must the template restrict formatting to its own styles? | Doc owners | Stops fake headings, but authors may find it limiting. |
| Q5 | Do older guides need moving to the template, or only new ones? | Doc owners | Affects whether a template-migration feature is needed. |
