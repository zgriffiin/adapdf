# Adapdf — agent notes

Read `README.md` and the specs in `docs/specs/` before any change. The specs
are the source of truth. If code and spec disagree, raise it; do not silently
pick one.

## Rules that are easy to break

- **Fix the source, not the PDF.** The only change to a PDF after export is
  document-level metadata (spec 02, stage 4). Never edit tags or content.
- **Never write a .docx without a backup first**, and never while Word has it
  open.
- **No network calls and no paid services.** Documents stay on the machine.
- **Never invent content** such as alt text or titles. Ask the user.
- **No real city documents in git.** Real guides go in `samples/`, which git
  ignores. Test fixtures are small documents made for testing.
- Word COM: start your own hidden instance, and always close it. Never touch
  the user's open Word windows.

## Stack

Python 3.13, lxml / python-docx, pywin32, pikepdf, veraPDF (bundled with a
Java runtime), tkinter, PyInstaller. Tests that need Word are marked
`windows_word`.
