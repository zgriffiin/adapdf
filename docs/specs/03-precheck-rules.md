# 03 — Pre-check rules

Status: draft, before the Phase 0 test. Phase 0 results will change the
severities and add rules.

Severity:

- **error** — blocks export. It causes a veraPDF or PAC failure, or a clear
  WCAG 2.1 AA failure.
- **warning** — reported but does not block export. It is usually a quality
  problem, or a heuristic that can be wrong.

Auto-fix:

- **auto** — the tool can fix it after the user confirms.
- **ask** — the tool fixes it after the user types the needed text.
- **manual** — the user fixes it in Word, following the tool's instructions.

The WCAG column gives the main success criterion. "UA" means a PDF/UA
(Matterhorn) requirement.

## Document

| ID | Check | Severity | Fix | WCAG / UA |
|---|---|---|---|---|
| DOC-001 | Title property (File > Info > Title) is empty. | error | ask, with up to three one-click texts from the document, chosen by the user: the file name, the Title paragraph, the first Heading 1 (spec 02, stage 2) | 2.4.2, UA |
| DOC-002 | Default language is not set. | error | auto (en-US; can be changed in settings) | 3.1.1, UA |
| DOC-003 | A paragraph or run uses a language different from the default, but its language is not marked. | warning | manual | 3.1.2 |
| DOC-004 | The document is not based on the Adapdf template. | warning | manual | — |
| DOC-005 | The document has tracked changes or comments. | error | manual (accept or reject, then delete comments) | — |
| DOC-006 | A font in use cannot be embedded. | error | manual | UA |

## Headings

| ID | Check | Severity | Fix | WCAG / UA |
|---|---|---|---|---|
| HDG-001 | No headings in a document longer than one page (about 400 words). | error | manual | 1.3.1, 2.4.6 |
| HDG-002 | A heading level is skipped (for example, Heading 1 then Heading 3). | error | manual | 1.3.1, UA |
| HDG-003 | Fake heading: a short paragraph (12 words or fewer) that is all bold, or larger than the body text, and does not use a Heading style. | warning | auto (apply a Heading style at the suggested level) | 1.3.1 |
| HDG-004 | A heading paragraph is empty. | error | auto (delete it) | 1.3.1, UA |
| HDG-006 | A custom style has an outline level but is not a built-in Heading style. | warning | manual | 1.3.1 |

HDG-005 was removed: WCAG 2.1 and PDF/UA-1 do not require a single Heading 1.

## Images and objects

| ID | Check | Severity | Fix | WCAG / UA |
|---|---|---|---|---|
| IMG-001 | An image, chart, shape, group, drawing canvas or SmartArt has no alt text and is not marked decorative. | error | ask (type the alt text, or mark it decorative) | 1.1.1, UA |
| IMG-002 | The alt text looks like a file name, or is only a word such as "image", "picture" or "screenshot". | warning | ask | 1.1.1 |
| IMG-003 | The alt text is longer than 250 characters. | warning | manual (shorten it, and put the detail in the body text or a caption) | 1.1.1 |
| IMG-004 | A picture, chart, SmartArt, group, drawing canvas or shape is floating (not "In line with text") and is not marked decorative. | warning | auto (make it inline) | 1.3.2 |
| IMG-005 | The alt text is the same as the caption next to it. | warning | manual | 1.1.1 |
| IMG-006 | The document has an embedded object (OLE) or an equation. | warning | manual (Phase 0 will check how these export) | 1.1.1 |
| IMG-007 | A screenshot has shapes (arrows, boxes, lines) drawn on it in Word that are not grouped with it and not marked decorative. | error | ask (mark the shapes decorative after the user confirms the alt text describes them) | 1.1.1, UA |

Notes on the image rules:

- A group of shapes and a drawing canvas (a flowchart made in Word, for
  example) are one object. Their alt text and decorative mark are their own.
  The pictures and shapes inside need no alt text of their own. Text in the
  shapes inside still counts for TXB-001.
- IMG-004 does not report a floating object that is marked decorative (it is
  an artifact, so reading order does not apply). It does not report a text box
  with text (TXB-001 does) or a shape that IMG-007 reports.
- IMG-007 uses the paragraph the shape is anchored in, not the shape's
  position on the page. A shape belongs to a screenshot when it is anchored in
  the same paragraph as the picture, or in the paragraph just before or after
  it. A picture and shapes that are already in one group are one figure:
  IMG-001 checks the alt text of the group, and IMG-007 does not report.
- IMG-007 replaces IMG-001 and IMG-004 for the shapes it reports, so one
  screenshot gives one finding, not one for each arrow. If the picture has no
  alt text, IMG-001 still reports the picture. Shapes that contain text are
  reported by TXB-001, once for the screenshot.
- Phase 0 must confirm how Word tags a grouped picture and shapes (one Figure
  or several), and how it tags a shape marked decorative.

## Text boxes and layout

| ID | Check | Severity | Fix | WCAG / UA |
|---|---|---|---|---|
| TXB-001 | A text box or shape contains text, or a group or drawing canvas has shapes with text. | error | manual (move the text into the body and use a callout style) | 1.3.2, UA |
| TXB-002 | The document uses more than one column section. | warning | manual (check the reading order in PAC) | 1.3.2 |
| TXB-003 | Spacing made with spaces or tabs: 3 or more spaces in a row, or 2 or more tabs, inside a paragraph. | warning | manual | 1.3.1 |
| TXB-004 | Empty paragraphs used for spacing: 2 or more in a row. | warning | auto (delete the extra empty paragraphs) | 1.3.1 |

## Lists

| ID | Check | Severity | Fix | WCAG / UA |
|---|---|---|---|---|
| LST-001 | A paragraph starts with a typed number or bullet (`1.`, `1)`, `a.`, `•`, `-`, `*`) and is not a real list. | error | auto (apply the List Number or List Bullet style and remove the typed number or bullet) | 1.3.1, UA |
| LST-002 | One list mixes typed numbers and automatic numbers. | warning | manual | 1.3.1 |

## Tables

| ID | Check | Severity | Fix | WCAG / UA |
|---|---|---|---|---|
| TBL-001 | No header row is marked ("Repeat as header row at the top of each page"). | error | auto (mark the first row, after the user confirms that it holds column headings; a label and value table has none) | 1.3.1, UA |
| TBL-002 | A table is inside another table. | error | manual | 1.3.1 |
| TBL-003 | The table has merged or split cells. | error | manual (split the merged cells back (Layout > Split Cells) so every row has the same cells, or make the information text instead of a table. A table with merged cells cannot be made accessible from Word: Word does not record which cells were merged). After the export Adapdf gives the header cells a scope only in a regular table (spec 02, stage 4) | 1.3.1, UA |
| TBL-004 | Probable layout table: one row, or one column, or no text in most cells. | warning | manual | 1.3.1 |
| TBL-005 | The table has a completely empty row. | warning | auto (delete the row) | 1.3.1 |
| TBL-006 | A row can break across pages. | warning | auto (set "Do not allow row to break across pages") | — |

Notes on the table rules:

- TBL-003 was raised from warning to error on 2026-11-18: with merged cells, Word's PDF fails PDF/UA-1 7.2-42 (S5-11).

## Links

| ID | Check | Severity | Fix | WCAG / UA |
|---|---|---|---|---|
| LNK-001 | The link text is vague: "click here", "here", "link", "more", "read more". | warning | manual | 2.4.4 |
| LNK-002 | The link text is a bare URL longer than 40 characters. | warning | manual | 2.4.4 |
| LNK-003 | An internal link points to a bookmark that does not exist. | error | manual | 2.4.4 |
| LNK-004 | A link has no ScreenTip. | error | auto (copy the link's own text into the ScreenTip, after the user confirms) | 2.4.4, 1.1.1, UA |
| LNK-005 | The table of contents is made of links: its `TOC` field has the `\h` switch. | error | auto (remove the `\h` switch from the field, after the user confirms) | 2.4.4, 1.1.1, UA |

Notes on the link rules (decided 2026-11-16, from the S5-10 result in the
[test plan](../test-plan.md#s5-10-result)):

- **Why LNK-004 and LNK-005 exist.** Word writes a PDF link annotation without
  `/Contents` (the description of the link) unless the link has a ScreenTip.
  veraPDF then fails PDF/UA-1 7.18.1-2 and 7.18.5-2. Seen with Word 16.0.20527
  and veraPDF 1.28.2 on every pass fixture that had a link, and on a
  table of contents with `\h`. A ScreenTip makes Word write `/Contents`; the
  table of contents is the exception, because Word makes its links again at each
  update, without ScreenTips. Whether PAC also fails these links is to be
  confirmed in Phase 0. The rules follow veraPDF until then.
- **LNK-004** reports a `w:hyperlink` element or a `HYPERLINK` field (simple or
  complex) with no ScreenTip, or one that is empty or only spaces. The ScreenTip
  is `w:tooltip` on a `w:hyperlink`, or the `\o "text"` switch of a field. It
  does not report a link that is an entry of a table of contents (in a `TOC`
  field result, or in a "Table of Contents" content control): LNK-005 covers
  those.
- **The LNK-004 fix** copies the link's own visible text into its ScreenTip, with
  runs of spaces, tabs and line breaks made into one space. This is the
  author's own text, not invented content. A link with no text, or only spaces,
  is refused with a message, and the user types a ScreenTip in Word. LNK-001 and
  LNK-002 still report a vague or a bare-address link text; copying it does not
  make it better. The fix does not change a field that already has an empty `\o`
  switch. Word cuts a `w:tooltip` at 260 characters (the length of its own
  ScreenTip box), so a very long link text gives a ScreenTip that is the start of
  the text.
- **LNK-005** reports a `TOC` field (also inside a "Table of Contents" content
  control) whose instruction has the `\h` switch, for example
  `TOC \o "1-3" \h \z \u`. **The fix** removes the `\h` switch from the field
  instruction and nothing else. Stage 3 updates the table of contents before
  the export, and Word then makes it without links. Readers still move through
  the PDF with its bookmarks (`CreateBookmarks`) and its headings. In Word the
  same change is References > Table of Contents > Custom Table of Contents, then
  clear "Use hyperlinks instead of page numbers".
- Neither fix changes the PDF. Stage 4 still writes only document-level
  metadata.

## Colour and text

| ID | Check | Severity | Fix | WCAG / UA |
|---|---|---|---|---|
| CLR-001 | Text contrast is below 4.5:1, or below 3:1 for large text (18 pt, or 14 pt bold), against the paragraph shading, cell shading or page colour. | error | manual | 1.4.3 |
| CLR-002 | Text uses a colour other than the style's colour and has no other emphasis such as bold. The colour may be the only signal. | warning | manual | 1.4.1 |
| TXT-001 | Body text is smaller than 10 pt. | warning | manual | — |
| TXT-002 | A paragraph is justified. | warning | auto (align it left) | — |
| TXT-003 | Headers or footers contain text other than the title, page numbers and dates. Screen readers skip headers and footers in the PDF. | warning | manual | 1.3.1 |
| TXT-004 | The table of contents is out of date. | warning | auto (update the fields at export) | 2.4.5 |

## Notes for implementation

- Contrast: use the WCAG relative-luminance formula. Use the effective colour
  after style inheritance and theme colours. "Automatic" text colour is black
  on a light background.
- Fake-heading and layout-table rules are heuristics. Keep them at warning
  until the sample documents show few false positives.
- Each rule needs a passing and a failing fixture. See
  [02-pipeline](02-pipeline.md#testing).
