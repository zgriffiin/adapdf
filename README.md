# Adapdf

Turns Word training guides into accessible PDFs (WCAG 2.1 AA, PDF/UA-1) on
the user's own machine, with no subscription.

It checks the Word document, fixes what it safely can, exports a tagged PDF
through Word, checks the PDF with veraPDF, and writes a report that serves as
evidence. A person then confirms the result in PAC and signs off the checklist.

Status: specs only. Nothing is built yet.

## Specs

1. [Product](docs/specs/01-product.md): goal, constraints, scope, technology,
   phases, open questions.
2. [Pipeline](docs/specs/02-pipeline.md): the six stages, outputs, interfaces,
   testing.
3. [Pre-check rules](docs/specs/03-precheck-rules.md): every .docx check, with
   severity and fix.
4. [Word template](docs/specs/04-word-template.md): the accessible template
   for new guides.

## Layout

```
docs/specs/       Specs
src/adapdf/       Application code (Python)
template/         Word template, author guide, template test document
tests/fixtures/   Small .docx files, one passing and one failing for each rule
samples/          Real guides for testing (git ignores this folder)
vendor/           veraPDF and Java runtime for packaging (git ignores this folder)
```

## First step

Phase 0 in the [product spec](docs/specs/01-product.md#phases): export 5 real
guides from Word and check them with veraPDF and PAC before writing code.
