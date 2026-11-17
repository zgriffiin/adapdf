# 04 — Word template spec

Status: draft. City brand details are not known yet (open question Q3 in
[01-product](01-product.md#open-questions)).

## Purpose

Most PDF errors start in the Word file. A template that has the right styles
for training guides stops these errors before the pre-check runs. Authors
should never need text boxes, typed numbers or fake headings.

## Deliverables

| File | Content |
|---|---|
| `template/Adapdf Training Guide.dotx` | The template. |
| `template/Adapdf Author Quick Guide.docx` | A 2–3 page guide for authors, made with the template. It must pass the pipeline with 0 errors. |
| `template/sample-all-styles.docx` | A test document that uses every style and building block. It is the template's acceptance test. |

Build the template in Word by hand. A script cannot reliably create
multilevel lists, building blocks and table styles. The .dotx is a binary
file in git. Record each change in `template/CHANGELOG.md`.

## Page and text

- Page: US Letter, 1-inch margins, one column.
- Body font: Aptos (the Microsoft 365 default) or the city's brand font if
  it can be embedded and is easy to read. Body text 12 pt.
- Line spacing 1.15. Space after paragraphs comes from the style (for example,
  8 pt), never from empty paragraphs.
- All text aligned left. No justified text.
- Default language en-US, set in the template's Normal style.
- Theme colours set so that every text style passes contrast (see "Colours").

## Styles

Use Word's **built-in** styles where one exists, and change only how they
look. Word's PDF export maps built-in styles to the correct PDF tags. Custom
styles may not map correctly.

### Paragraph styles

| Style | Built-in? | Use | PDF tag |
|---|---|---|---|
| Title | yes | Guide title, once, on the first page. | `Title`. Phase 0 will check whether Word exports `Title` or `P`. If it exports `P`, use Heading 1 for the guide title. |
| Heading 1–4 | yes | Section headings. Do not use levels below 4. | `H1`–`H4` |
| Normal | yes | Body text. | `P` |
| List Bullet, List Bullet 2 | yes | Bulleted lists, 2 levels. | `L` / `LI` |
| List Number, List Number 2 | yes | Steps. Linked to one multilevel list (1., a.). | `L` / `LI` |
| List Continue | yes | A paragraph inside a step, with no number. | `P` inside `LI` |
| Caption | yes | Figure and table captions. | `Caption` or `P` |
| TOC 1–3 | yes | Table of contents. | `TOC` / `TOCI` |
| Note | custom, based on Normal | Callout: extra information. | `P` |
| Tip | custom, based on Normal | Callout: a quicker or better way. | `P` |
| Important | custom, based on Normal | Callout: something that must not be missed. | `P` |
| Warning | custom, based on Normal | Callout: risk of data loss or error. | `P` |

### Callouts

Training guides need callouts. Authors usually make them with text boxes,
which break the reading order (rule TXB-001). The template makes them as
paragraph styles:

- A left border (4.5 pt) and light shading, set in the paragraph style.
- **Each callout starts with a bold text label** ("Note:", "Tip:",
  "Important:", "Warning:"). Colour and borders alone do not tell a screen
  reader user what the callout is (WCAG 1.4.1).
- An icon is optional. If used, it must be inline and marked decorative,
  because the label already gives the meaning.
- Provide each callout as a **Quick Part** (building block) that inserts the
  styled paragraph with its label already typed.

### Character styles

| Style | Use |
|---|---|
| UI Element | Names of buttons, menus and fields: "Select **Save**". Bold. |
| Keyboard Key | Keys: "Press **Ctrl+S**". Bold. Optional light shading that passes contrast. |
| Hyperlink (built-in) | Links. Underlined, colour passes 4.5:1. |
| Strong (built-in) | Strong emphasis. Use it instead of direct bold. |

### Table style: "Adapdf Table"

- Header row: bold, shaded, and the shading passes contrast.
- Light borders on all cells. Optional banded rows, if the banding passes contrast.
- Cell margins large enough to read easily.
- A table style cannot set "Repeat as header row" because that is a property
  of each row. So provide a **Quick Table** (building block) with the header
  row already marked. The pre-check (TBL-001) catches tables made without it.

### Figures

- Screenshots inline with text, never floating.
- Caption below the figure, with the Caption style.
- Author guide: alt text says what the screenshot shows for this step (for
  example, "The Save dialog with the File name box selected"). It does not
  repeat the caption.

### Annotating screenshots in Word

Authors often draw arrows and boxes on a screenshot to show where to click.
Word makes them as separate floating shapes. A screen reader cannot read
them. Use one of these ways:

1. **Group them (preferred).** Select the screenshot and its shapes,
   right-click, and choose Group. Give the group alt text that says what the
   screenshot shows and what is highlighted (for example, "The Save dialog. A
   red box shows the File name field."). Then set the group to In Line with
   Text.
2. **Mark each shape decorative.** Right-click each arrow and box, choose Edit
   Alt Text, and select Mark as decorative. Then make sure the alt text of the
   screenshot says what the arrows and boxes point to.

Never type instructions inside a shape on a screenshot. Put them in the step
text. The screenshot stays inline (In Line with Text) in both ways. The
pre-check reports a screenshot with shapes that are neither grouped nor
decorative (IMG-007).

### Flowcharts

- Make a flowchart from shapes in a drawing canvas, then group it, or save it
  as a picture (right-click, Save as Picture) and insert the picture in line
  with text.
- Give it alt text that summarises the flowchart (WCAG 1.1.1).
- Also write its steps in the body text, for example as a numbered list. The
  alt text is a summary, not the whole flowchart.
- Until Phase 0 shows how Word tags text inside grouped shapes, the pre-check
  reports text in the shapes of a group or canvas (TXB-001). A flowchart
  saved as a picture does not have this problem.

### Links and the table of contents

Word writes a link into the PDF with a description only when the link has a
ScreenTip, and veraPDF fails a link without one (PDF/UA-1 7.18.1-2 and
7.18.5-2; see LNK-004 and LNK-005 in [03-precheck-rules](03-precheck-rules.md)).

- **Links get a ScreenTip.** Select the link text, choose Insert > Link, choose
  ScreenTip, and type a short description of where the link goes. The Author
  Quick Guide shows this. The tool can copy the link text into the ScreenTip
  (LNK-004), but a description that says more than the link text is better.
- **The table of contents is made without hyperlinks.** The template's table of
  contents (a Quick Part) is a Table of Contents content control whose field is
  `TOC \o "1-3" \z \u`, with no `\h` switch. In Word: References > Table of
  Contents > Custom Table of Contents, clear "Use hyperlinks instead of page
  numbers". Word makes the links of a table of contents again at each update,
  without ScreenTips, so a ScreenTip cannot be kept on them. Readers still
  move through the PDF with its bookmarks and headings.
- Link text says where the link goes (LNK-001), and a long web address is not
  used as link text (LNK-002).

## Headers, footers and cover

- Header: the guide title, from a `TITLE` field so it matches the
  document property.
- Footer: "Page X of Y" fields.
- Word marks headers and footers as background content in the PDF, so
  nothing in them may be the only place for important information.
- Cover page: normal paragraphs only (Title, a subtitle in Normal or a custom
  "Subtitle" style, and the date), with no text boxes. City logo inline, with
  alt text "City of <name> logo" (or marked decorative if the title already
  names the city — decide with Q3).

## Colours

- Every text style, link colour and shading combination is listed in
  `template/CHANGELOG.md` with its contrast ratio.
- Body text, links and callout text: at least 4.5:1 against their
  background.
- Headings at 18 pt or larger (or 14 pt bold): at least 3:1. Aim for 4.5:1.
- Callout borders: at least 3:1 against white (WCAG 1.4.11), even though the
  label carries the meaning.
- No meaning carried by colour alone.

## Document properties and settings in the template

- Title: empty, so the pre-check (DOC-001) asks for it. It must not hold
  the template's own name.
- Language: en-US.
- "Mark as decorative", alt-text and ScreenTip guidance appear in the Author Quick
  Guide.
- Formatting restriction (Review > Restrict Editing > "Limit formatting to a
  selection of styles"): **not set by default**. Decide with Q4.

## Installation for users

- Copy the .dotx to the personal templates folder
  (`%APPDATA%\Microsoft\Templates`). It then shows in File > New > Personal.
- To move an existing guide to the template: Developer > Document Template >
  Attach, then select "Automatically update document styles". This changes
  only how styles look. Text boxes and fake headings must still be fixed by
  hand, and the pre-check finds them. A migration feature depends on Q5.
- In Word, also turn on "Keep accessibility checker running while I work"
  (File > Options > Accessibility). It is a Word setting, not part of the
  template.

## Acceptance

The template is done when:

1. `sample-all-styles.docx` uses every style, callout, Quick Part, a Quick
   Table, a captioned screenshot, a TOC without hyperlinks, links with
   ScreenTips, a 3-level heading structure
   and both list types.
2. The Adapdf pre-check reports 0 errors and 0 warnings on it (DOC-004
   aside, until that rule can recognise the template).
3. Its PDF from the pipeline has 0 veraPDF failures and 0 PAC errors.
4. The Author Quick Guide passes the same checks.
5. All colour pairs in the changelog pass the ratios above.
