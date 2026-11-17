// The screens of the app, with the Tauri layer replaced (see test/backend.ts).
// Rows A-01 to A-04 and A-08 of the test plan, on the page.
import { act, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { App } from "./App";
import {
  CHECKLIST,
  applyResultOf,
  checkOf,
  deferred,
  doc001,
  exportResultOf,
  img001,
  img007,
  lst002,
  offerOf,
  reportOf,
  setupBackend,
  txb004,
  viewOf,
  wcagViolations,
  type Backend,
  type Handlers,
} from "./test/backend";
import type { ApplyResult, ExportResult, ReportView } from "./types";

const DOCX = "C:\\Docs\\Guide.docx";

type User = ReturnType<typeof userEvent.setup>;

/** The app with a document that has these findings and offers, already chosen. */
async function openDocument(
  findings: Parameters<typeof checkOf>[1],
  offers: ReturnType<typeof offerOf>[],
  extra: Handlers = {},
): Promise<{ user: User; backend: Backend }> {
  const backend = setupBackend({
    check: () => checkOf(DOCX, findings),
    fix_offers: () => offers,
    ...extra,
  });
  backend.chosen.push(DOCX);
  const user = userEvent.setup();
  render(<App />);
  await user.click(screen.getByRole("button", { name: "Choose a Word document" }));
  await screen.findByRole("heading", { level: 2, name: /^Guide\.docx: / });
  return { user, backend };
}

const heading = () => screen.getByRole("heading", { level: 2 });

/** The label of every control that the Tab key reaches, in order. */
async function tabOrder(user: User, count: number): Promise<string[]> {
  const labels: string[] = [];
  for (let i = 0; i < count; i++) {
    await user.tab();
    const el = document.activeElement as HTMLElement & { labels?: NodeListOf<HTMLLabelElement> | null };
    labels.push(
      el.getAttribute("aria-label") ??
        (el.labels?.[0]?.textContent ?? el.textContent ?? "").trim().replace(/\s+/g, " "),
    );
  }
  return labels;
}

// ---------------------------------------------------------------- empty

describe("the first screen", () => {
  it("asks for a document, with a button and a note about dropping", async () => {
    setupBackend({});
    render(<App />);
    expect(heading()).toHaveTextContent("Choose a Word document");
    expect(screen.getByRole("button", { name: "Choose a Word document" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Open an earlier report" })).toBeEnabled();
    expect(screen.getByText(/drop one on this window/)).toBeInTheDocument();
    // The heading has the focus, so a screen reader starts there.
    expect(heading()).toHaveFocus();
  });

  it("has no WCAG 2.1 A or AA violation", async () => {
    setupBackend({});
    const { container } = render(<App />);
    expect(await wcagViolations(container)).toEqual([]);
  });

  it("does nothing when the chooser is cancelled", async () => {
    const backend = setupBackend({});
    backend.chosen.push(null);
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "Choose a Word document" }));
    expect(backend.calls).toEqual([]);
    expect(heading()).toHaveTextContent("Choose a Word document");
  });

  it("shows why a document could not be checked", async () => {
    const backend = setupBackend({
      check: () => {
        throw { message: "Guide.docx: this is not a .docx file: no zip" };
      },
    });
    backend.chosen.push(DOCX);
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "Choose a Word document" }));
    const region = await screen.findByRole("status");
    await waitFor(() => expect(region).toHaveTextContent("Guide.docx could not be checked."));
    expect(region).toHaveTextContent("this is not a .docx file");
    expect(heading()).toHaveTextContent("Choose a Word document");
  });

  it("takes a dropped Word document, and refuses another kind of file", async () => {
    const backend = setupBackend({
      check: () => checkOf(DOCX, [txb004]),
      fix_offers: () => [],
    });
    const user = userEvent.setup();
    render(<App />);
    await waitFor(() => expect(backend.drop).not.toBe(undefined));
    act(() => backend.drop(["C:\\Docs\\Notes.txt"]));
    await waitFor(() =>
      expect(screen.getByRole("status")).toHaveTextContent("That file cannot be used."),
    );
    expect(backend.calls).toEqual([]);
    act(() => backend.drop([DOCX]));
    await screen.findByRole("heading", { level: 2, name: /^Guide\.docx: Found 0 errors and 1 warning\.$/ });
    expect(backend.calls.map((c) => c.command)).toEqual(["check", "fix_offers"]);
    expect(backend.calls[0]?.args).toEqual({ path: DOCX });
    // The document is checked again from the keyboard.
    await user.click(screen.getByRole("button", { name: "Check again" }));
    await waitFor(() => expect(backend.calls.filter((c) => c.command === "check")).toHaveLength(2));
  });
});

// ------------------------------------------------------------- findings

describe("the findings screen (A-01, A-02, A-04)", () => {
  it("groups the findings by severity, errors first, with counts", async () => {
    // The findings of LST-002-fail.docx.
    await openDocument([lst002.lst002, lst002.lst001], [offerOf(lst002.lst001)]);
    expect(heading()).toHaveTextContent("Guide.docx: Found 1 error and 1 warning.");
    const groups = screen.getAllByRole("heading", { level: 3 }).map((h) => h.textContent);
    expect(groups).toEqual(["Errors (1)", "Warnings (1)"]);
    const lists = screen.getAllByRole("list");
    const errorItem = within(lists[0]!).getByRole("listitem");
    expect(errorItem).toHaveTextContent("LST-001");
    expect(errorItem).toHaveTextContent("Where: paragraph 5");
    expect(errorItem).toHaveTextContent("How to fix: How to fix LST-001.");
    expect(within(lists[1]!).getByRole("listitem")).toHaveTextContent("LST-002");
    // The words Error and Warning are in the text, not only in a colour.
    expect(errorItem).toHaveTextContent("Error:");
    expect(within(lists[1]!).getByRole("listitem")).toHaveTextContent("Warning:");
  });

  it("turns Export off while errors remain, and says why", async () => {
    await openDocument([lst002.lst001, lst002.lst002], [offerOf(lst002.lst001)]);
    expect(screen.getByRole("button", { name: "Export PDF" })).toBeDisabled();
    expect(screen.getByText(/Export is off while errors remain/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Fix problems" })).toBeEnabled();
  });

  it("turns Export on when only warnings remain", async () => {
    await openDocument([txb004], [offerOf(txb004)]);
    expect(screen.getByRole("button", { name: "Export PDF" })).toBeEnabled();
    expect(screen.queryByText(/Export is off/)).not.toBeInTheDocument();
  });

  it("offers no fixes button when there is nothing to fix", async () => {
    await openDocument([lst002.lst002], []);
    expect(screen.queryByRole("button", { name: "Fix problems" })).not.toBeInTheDocument();
  });

  it("says when there are no problems", async () => {
    await openDocument([], []);
    expect(heading()).toHaveTextContent("Guide.docx: No problems found.");
    expect(screen.getByRole("button", { name: "Export PDF" })).toBeEnabled();
  });

  it("explains that fixes are not available when Word has the file open", async () => {
    await openDocument([lst002.lst001], [], {
      fix_offers: () => {
        throw { message: "Guide.docx is open in Word. Close the document in Word, then run the fix again." };
      },
    });
    expect(screen.getByRole("status")).toHaveTextContent("The tool cannot offer fixes now.");
    expect(screen.getByRole("status")).toHaveTextContent("open in Word");
    expect(screen.queryByRole("button", { name: "Fix problems" })).not.toBeInTheDocument();
  });

  it("shows a suggestion as text that is not applied", async () => {
    await openDocument([doc001], [offerOf(doc001)]);
    expect(screen.getByText(/Suggestion \(not applied\):/)).toBeInTheDocument();
    expect(screen.getByText(/Training guide: first steps/)).toBeInTheDocument();
  });

  it("has no WCAG 2.1 A or AA violation", async () => {
    await openDocument([lst002.lst002, lst002.lst001, doc001], [offerOf(lst002.lst001)]);
    expect(await wcagViolations(document.body)).toEqual([]);
  });

  it("puts the focus on the results heading, and the keys follow the reading order", async () => {
    const { user } = await openDocument([lst002.lst001, lst002.lst002], [offerOf(lst002.lst001)]);
    expect(heading()).toHaveFocus();
    // Fix problems, Export PDF (disabled: skipped), Check again, Choose another file.
    expect(await tabOrder(user, 3)).toEqual(["Fix problems", "Check again", "Choose another file"]);
    // The order in the page is the order of the keys: no positive tabindex.
    const tabindexes = [...document.querySelectorAll("[tabindex]")]
      .map((el) => Number(el.getAttribute("tabindex")))
      .filter((n) => n > 0);
    expect(tabindexes).toEqual([]);
  });

  it("moves the focus back to the heading after Check again", async () => {
    const { user, backend } = await openDocument([txb004], []);
    await user.click(screen.getByRole("button", { name: "Check again" }));
    await waitFor(() => expect(backend.calls.filter((c) => c.command === "check")).toHaveLength(2));
    await waitFor(() => expect(heading()).toHaveFocus());
  });
});

// ------------------------------------------------------------- fix flow

describe("the fix flow (A-02, A-03, A-04)", () => {
  it("turns Apply fixes on when a fix is ticked, and sends that choice", async () => {
    const applied = applyResultOf([lst002.lst002]);
    const { user, backend } = await openDocument(
      [lst002.lst001, lst002.lst002],
      [offerOf(lst002.lst001)],
      { apply_fixes: () => applied },
    );
    await user.click(screen.getByRole("button", { name: "Fix problems" }));
    expect(await screen.findByRole("heading", { level: 2, name: "Fix problems in Guide.docx" })).toHaveFocus();
    const apply = screen.getByRole("button", { name: "Apply fixes" });
    expect(apply).toBeDisabled();
    expect(screen.getByText("Choose at least one fix to apply.")).toBeInTheDocument();

    await user.click(screen.getByRole("checkbox", { name: /LST-001: Do apply-list-style/ }));
    expect(screen.getByRole("button", { name: "Apply 1 fix" })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "Apply 1 fix" }));

    await waitFor(() => expect(backend.calls.at(-2)?.command).toBe("apply_fixes"));
    expect(backend.calls.find((c) => c.command === "apply_fixes")?.args).toEqual({
      path: DOCX,
      choices: [{ rule_id: "LST-001", position: 0, input: { kind: "confirm" } }],
    });
    // Back on the findings, with the result in the live region and the new findings.
    const heading2 = await screen.findByRole("heading", { level: 2, name: /^Guide\.docx: Found 0 errors and 1 warning\.$/ });
    await waitFor(() => expect(heading2).toHaveFocus());
    const region = screen.getByRole("status");
    expect(region).toHaveTextContent("The fixes were applied.");
    expect(region).toHaveTextContent("1 fix applied.");
    expect(region).toHaveTextContent("Backup: Guide.backup-20260305-101530.docx.");
    expect(screen.getByRole("button", { name: "Export PDF" })).toBeEnabled();
  });

  it("selects and clears all automatic fixes", async () => {
    const offers = [offerOf(lst002.lst001), offerOf(txb004)];
    const { user } = await openDocument([lst002.lst001, txb004], offers);
    await user.click(screen.getByRole("button", { name: "Fix problems" }));
    await user.click(screen.getByRole("button", { name: "Select all automatic fixes" }));
    expect(screen.getAllByRole("checkbox").every((box) => (box as HTMLInputElement).checked)).toBe(true);
    expect(screen.getByRole("button", { name: "Apply 2 fixes" })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "Clear all automatic fixes" }));
    expect(screen.getByRole("button", { name: "Apply fixes" })).toBeDisabled();
  });

  it("never fills in the title, and shows the suggestion as a hint", async () => {
    const { user, backend } = await openDocument([doc001], [offerOf(doc001)], {
      apply_fixes: () => applyResultOf([]),
    });
    await user.click(screen.getByRole("button", { name: "Fix problems" }));
    const field = screen.getByRole("textbox", { name: "Document title" });
    expect(field).toHaveValue("");
    // The suggestion is the helper text of the field, which is not its value.
    expect(field).toHaveAccessibleDescription(/Suggestion, not filled in: Training guide: first steps/);
    expect(screen.getByRole("button", { name: "Apply fixes" })).toBeDisabled();
    // Nothing is sent until the person types.
    await user.type(field, "My own title");
    await user.click(screen.getByRole("button", { name: "Apply 1 fix" }));
    await waitFor(() => expect(backend.calls.some((c) => c.command === "apply_fixes")).toBe(true));
    expect(backend.calls.find((c) => c.command === "apply_fixes")?.args.choices).toEqual([
      { rule_id: "DOC-001", position: 0, input: { kind: "text", text: "My own title" } },
    ]);
  });

  it("has an empty alt text field, and a decorative choice that switches it off", async () => {
    const { user, backend } = await openDocument([img001], [offerOf(img001)], {
      apply_fixes: () => applyResultOf([]),
    });
    await user.click(screen.getByRole("button", { name: "Fix problems" }));
    const field = screen.getByRole("textbox", { name: /Alt text for IMG-001/ });
    expect(field).toHaveValue("");
    expect(field).toHaveAccessibleDescription(/Suggestion, not filled in: image1\.png/);
    const decorative = screen.getByRole("checkbox", { name: "Mark as decorative (it adds no information)" });
    await user.click(decorative);
    expect(field).toBeDisabled();
    expect(screen.getByRole("button", { name: "Apply 1 fix" })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: "Apply 1 fix" }));
    await waitFor(() => expect(backend.calls.some((c) => c.command === "apply_fixes")).toBe(true));
    expect(backend.calls.find((c) => c.command === "apply_fixes")?.args.choices).toEqual([
      { rule_id: "IMG-001", position: 0, input: { kind: "decorative" } },
    ]);
  });

  it("asks the question about arrows and boxes, and needs a yes", async () => {
    const { user } = await openDocument([img007], [offerOf(img007)]);
    await user.click(screen.getByRole("button", { name: "Fix problems" }));
    const yes = screen.getByRole("checkbox", { name: /the alt text of the screenshot describes/ });
    expect(yes).not.toBeChecked();
    expect(screen.getByRole("button", { name: "Apply fixes" })).toBeDisabled();
    await user.click(yes);
    expect(screen.getByRole("button", { name: "Apply 1 fix" })).toBeEnabled();
  });

  it("lists a fix that Word does at export and a fix that cannot be made", async () => {
    const offers = [
      offerOf(lst002.lst001),
      offerOf(txb004, { availability: { status: "at-export" } }),
      offerOf(img001, { availability: { status: "unavailable", reason: "The picture is in a header." } }),
    ];
    const { user } = await openDocument([lst002.lst001, txb004, img001], offers);
    await user.click(screen.getByRole("button", { name: "Fix problems" }));
    expect(screen.getByRole("heading", { level: 3, name: /Done by Word when the PDF is exported/ })).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 3, name: /The tool cannot fix these now/ })).toBeInTheDocument();
    expect(screen.getByText("The picture is in a header.")).toBeInTheDocument();
    // Neither can be chosen.
    expect(screen.getAllByRole("checkbox")).toHaveLength(1);
  });

  it("shows a fix that the tool refused", async () => {
    const refused: ApplyResult = applyResultOf([lst002.lst001], {
      backup: null,
      applied: [],
      refused: [{ rule_id: "LST-001", message: "m", reason: "The document has changed since the check." }],
    });
    const { user } = await openDocument([lst002.lst001], [offerOf(lst002.lst001)], {
      apply_fixes: () => refused,
    });
    await user.click(screen.getByRole("button", { name: "Fix problems" }));
    await user.click(screen.getByRole("checkbox", { name: /LST-001/ }));
    await user.click(screen.getByRole("button", { name: "Apply 1 fix" }));
    const region = await screen.findByText(/Nothing was changed\./);
    expect(region).toBeInTheDocument();
    expect(screen.getByRole("status")).toHaveTextContent("No backup was made, because nothing was written.");
    expect(screen.getByRole("status")).toHaveTextContent("The document has changed since the check.");
  });

  it("has no WCAG 2.1 A or AA violation, and the keys follow the reading order", async () => {
    const offers = [
      offerOf(lst002.lst001),
      offerOf(doc001),
      offerOf(img001),
      offerOf(img007),
      offerOf(txb004, { availability: { status: "at-export" } }),
    ];
    const { user } = await openDocument([lst002.lst001, doc001, img001, img007, txb004], offers);
    await user.click(screen.getByRole("button", { name: "Fix problems" }));
    expect(await wcagViolations(document.body)).toEqual([]);
    // Apply fixes is off, so the keys skip it.
    const labels = await tabOrder(user, 7);
    const expected = [
      "Select all automatic fixes",
      "LST-001: Do apply-list-style.",
      "Document title",
      "Alt text for IMG-001",
      "Mark as decorative",
      "Yes, the alt text of the screenshot describes",
      "Back to the findings",
    ];
    expected.forEach((text, i) => expect(labels[i]).toContain(text));
  });
});

// --------------------------------------------------------------- export

describe("the export (A-02, A-04)", () => {
  it("shows the progress, then the result in words", async () => {
    const pending = deferred<ExportResult>();
    const { user, backend } = await openDocument([txb004], [], { export: () => pending.promise });
    await user.click(screen.getByRole("button", { name: "Export PDF" }));

    // While Word works: a heading, a message in the live region, and no buttons.
    const working = await screen.findByRole("heading", { level: 2, name: "Exporting Guide.docx" });
    expect(working).toHaveFocus();
    expect(screen.getByRole("status")).toHaveTextContent("Word is making the PDF");
    expect(screen.queryByRole("button", { name: "Export PDF" })).not.toBeInTheDocument();
    expect(await wcagViolations(document.body)).toEqual([]);

    await act(async () => pending.resolve(exportResultOf(viewOf(reportOf({ precheck: [txb004] })))));
    const result = await screen.findByRole("heading", { level: 2, name: /^Result for Guide\.docx: Pass$/ });
    await waitFor(() => expect(result).toHaveFocus());
    expect(backend.calls.find((c) => c.command === "export")?.args).toEqual({ path: DOCX });
    expect(screen.getByText(/Pass: no PDF\/UA-1 rule failed \(veraPDF 1\.28\.2\)/)).toBeInTheDocument();
    expect(screen.getByText("Guide.pdf", { exact: false })).toBeInTheDocument();
    expect(screen.queryByRole("status")).toBeEmptyDOMElement();
  });

  it("says Fail in words, with the counts and the veraPDF failure", async () => {
    const view = viewOf(reportOf({ verdict: "fail", failedRules: 1 }));
    const { user } = await openDocument([txb004], [], { export: () => exportResultOf(view) });
    await user.click(screen.getByRole("button", { name: "Export PDF" }));
    expect(await screen.findByRole("heading", { level: 2, name: /^Result for Guide\.docx: Fail$/ })).toBeInTheDocument();
    expect(screen.getByText("The report has 1 failed rule of the PDF/UA-1 check (veraPDF).")).toBeInTheDocument();
    expect(screen.getByText(/Fail: 1 rule of PDF\/UA-1 failed/)).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 3, name: "PDF/UA-1 rules that failed (1)" })).toBeInTheDocument();
    expect(screen.getByText(/7\.18\.1-2: A link has no description\./)).toBeInTheDocument();
    expect(screen.getByText(/How to fix:/)).toBeInTheDocument();
  });

  it("goes back to the findings with the reason when the tool fails", async () => {
    const { user } = await openDocument([txb004], [], {
      export: () => {
        throw { message: "Microsoft Word is not installed." };
      },
    });
    await user.click(screen.getByRole("button", { name: "Export PDF" }));
    const back = await screen.findByRole("heading", { level: 2, name: /^Guide\.docx: Found/ });
    expect(back).toBeInTheDocument();
    const region = screen.getByRole("status");
    expect(region).toHaveTextContent("The export did not finish.");
    expect(region).toHaveTextContent("Microsoft Word is not installed.");
    expect(screen.getByRole("button", { name: "Export PDF" })).toBeEnabled();
  });

  it("has no WCAG 2.1 A or AA violation on the result", async () => {
    const view = viewOf(reportOf({ verdict: "fail", failedRules: 1, precheck: [txb004, lst002.lst002] }));
    const { user } = await openDocument([txb004], [], { export: () => exportResultOf(view) });
    await user.click(screen.getByRole("button", { name: "Export PDF" }));
    await screen.findByRole("heading", { level: 2, name: /^Result for/ });
    expect(await wcagViolations(document.body)).toEqual([]);
  });

  it("opens the report in the browser, and shows why when it does not open", async () => {
    let fail = false;
    const view = viewOf();
    const { user, backend } = await openDocument([txb004], [], {
      export: () => exportResultOf(view),
      open_report: () => {
        if (fail) {
          throw { message: "Guide.accessibility.html is not there. Run the export again." };
        }
        return null;
      },
    });
    await user.click(screen.getByRole("button", { name: "Export PDF" }));
    await screen.findByRole("heading", { level: 2, name: /^Result for/ });
    await user.click(screen.getByRole("button", { name: "Open the report in your browser" }));
    await waitFor(() => expect(backend.calls.some((c) => c.command === "open_report")).toBe(true));
    expect(backend.calls.find((c) => c.command === "open_report")?.args).toEqual({
      path: "C:\\Docs\\Guide.accessibility.html",
    });
    fail = true;
    await user.click(screen.getByRole("button", { name: "Open the report in your browser" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("The report did not open."));
    expect(screen.getByRole("status")).toHaveTextContent("Run the export again.");
  });

  it("goes back to the findings of the document", async () => {
    const { user, backend } = await openDocument([txb004], [], {
      export: () => exportResultOf(viewOf()),
    });
    await user.click(screen.getByRole("button", { name: "Export PDF" }));
    await screen.findByRole("heading", { level: 2, name: /^Result for/ });
    await user.click(screen.getByRole("button", { name: "Back to the findings" }));
    await screen.findByRole("heading", { level: 2, name: /^Guide\.docx: Found/ });
    expect(backend.calls.filter((c) => c.command === "check")).toHaveLength(2);
  });
});

// ------------------------------------------------------------- sign-off

describe("the sign-off (A-04, A-08)", () => {
  async function openResult(
    view: ReportView,
    extra: Handlers = {},
  ): Promise<{ user: User; backend: Backend }> {
    const opened = await openDocument([txb004], [], { export: () => exportResultOf(view), ...extra });
    await opened.user.click(screen.getByRole("button", { name: "Export PDF" }));
    await screen.findByRole("heading", { level: 2, name: /^Result for/ });
    return opened;
  }

  const form = () => screen.getByRole("form", { name: "Sign-off" });

  it("shows the seven items of the checklist, a name and a date, all empty", async () => {
    await openResult(viewOf());
    const boxes = within(form()).getAllByRole("checkbox");
    expect(boxes).toHaveLength(7);
    expect(boxes.every((box) => !(box as HTMLInputElement).checked)).toBe(true);
    expect(within(form()).getByRole("textbox", { name: "Your name" })).toHaveValue("");
    expect(within(form()).getByLabelText("Date")).toHaveValue("");
    expect(within(form()).getByText("0 of 7 checks are ticked.")).toBeInTheDocument();
  });

  it("saves 5 ticked items, the name and the date, and shows the saved report", async () => {
    const saved = viewOf(
      reportOf({
        name: "A. Reviewer",
        date: "2026-03-06",
        done: [true, true, true, true, true, false, false],
      }),
    );
    const { user, backend } = await openResult(viewOf(), { save_signoff: () => saved });
    const boxes = within(form()).getAllByRole("checkbox");
    for (const box of boxes.slice(0, 5)) {
      await user.click(box);
    }
    expect(within(form()).getByText("5 of 7 checks are ticked.")).toBeInTheDocument();
    await user.type(within(form()).getByRole("textbox", { name: "Your name" }), "A. Reviewer");
    await user.type(within(form()).getByLabelText("Date"), "2026-03-06");
    await user.click(within(form()).getByRole("button", { name: "Save sign-off" }));

    await waitFor(() => expect(backend.calls.some((c) => c.command === "save_signoff")).toBe(true));
    expect(backend.calls.find((c) => c.command === "save_signoff")?.args).toEqual({
      reportJson: "C:\\Docs\\Guide.accessibility.json",
      signoff: {
        name: "A. Reviewer",
        date: "2026-03-06",
        items: [true, true, true, true, true, false, false],
      },
    });
    const region = screen.getByRole("status");
    await waitFor(() => expect(region).toHaveTextContent("The sign-off was saved."));
    expect(region).toHaveTextContent("The PDF was not changed.");
    // The form shows what was saved, and the focus is on the heading.
    await waitFor(() =>
      expect(screen.getByRole("heading", { level: 2, name: /^Result for/ })).toHaveFocus(),
    );
    expect(within(form()).getByRole("textbox", { name: "Your name" })).toHaveValue("A. Reviewer");
  });

  it("does not save without a name, and does not make one up", async () => {
    const { user, backend } = await openResult(viewOf());
    await user.click(within(form()).getAllByRole("checkbox")[0]!);
    await user.click(within(form()).getByRole("button", { name: "Save sign-off" }));
    const name = within(form()).getByRole("textbox", { name: "Your name" });
    expect(name).toHaveValue("");
    expect(name).toHaveAttribute("aria-invalid", "true");
    expect(name).toHaveFocus();
    expect(within(form()).getByText(/Type your name to sign off/)).toBeInTheDocument();
    expect(backend.calls.some((c) => c.command === "save_signoff")).toBe(false);
    // A name of spaces is no name.
    await user.type(name, "   ");
    await user.click(within(form()).getByRole("button", { name: "Save sign-off" }));
    expect(backend.calls.some((c) => c.command === "save_signoff")).toBe(false);
  });

  it("shows the message of the tool when the sign-off cannot be saved", async () => {
    const { user } = await openResult(viewOf(), {
      save_signoff: () => {
        throw { message: "The report Guide.accessibility.json cannot be replaced (access denied). The sign-off was not saved." };
      },
    });
    await user.type(within(form()).getByRole("textbox", { name: "Your name" }), "A. Reviewer");
    await user.click(within(form()).getByRole("button", { name: "Save sign-off" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("The sign-off could not be saved in full."));
    expect(screen.getByRole("status")).toHaveTextContent("access denied");
    // What the person typed is still there.
    expect(within(form()).getByRole("textbox", { name: "Your name" })).toHaveValue("A. Reviewer");
  });

  it("shows the same name, date and ticks when the report is opened again", async () => {
    const saved = viewOf(
      reportOf({
        name: "A. Reviewer",
        date: "2026-03-06",
        done: [true, true, true, true, true, false, false],
      }),
    );
    const backend = setupBackend({ read_report: () => saved });
    backend.chosen.push("C:\\Docs\\Guide.accessibility.json");
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "Open an earlier report" }));
    await screen.findByRole("heading", { level: 2, name: /^Result for Guide\.docx:/ });
    expect(backend.calls).toEqual([
      { command: "read_report", args: { path: "C:\\Docs\\Guide.accessibility.json" } },
    ]);
    const boxes = within(form()).getAllByRole("checkbox") as HTMLInputElement[];
    expect(boxes.map((box) => box.checked)).toEqual([true, true, true, true, true, false, false]);
    expect(within(form()).getByRole("textbox", { name: "Your name" })).toHaveValue("A. Reviewer");
    expect(within(form()).getByLabelText("Date")).toHaveValue("2026-03-06");
    // There is no document to go back to, only to choose another file.
    expect(screen.queryByRole("button", { name: "Back to the findings" })).not.toBeInTheDocument();
    expect(await wcagViolations(document.body)).toEqual([]);
  });

  it("shows why an earlier report cannot be opened", async () => {
    const backend = setupBackend({
      read_report: () => {
        throw { message: "Guide.accessibility.json: this is not a report that this tool wrote." };
      },
    });
    backend.chosen.push("C:\\Docs\\Guide.accessibility.json");
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "Open an earlier report" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("The report could not be opened."));
    expect(heading()).toHaveTextContent("Choose a Word document");
  });

  it("has no WCAG 2.1 A or AA violation, with an empty form, with an error and with a filled-in form", async () => {
    await openResult(viewOf());
    expect(await wcagViolations(form())).toEqual([]);
    const user = userEvent.setup();
    await user.click(within(form()).getByRole("button", { name: "Save sign-off" }));
    expect(await wcagViolations(document.body)).toEqual([]);
    // Filled in: five items ticked, a name and a date typed, not yet saved.
    for (const box of within(form()).getAllByRole("checkbox").slice(0, 5)) {
      await user.click(box);
    }
    await user.type(within(form()).getByRole("textbox", { name: "Your name" }), "A. Reviewer");
    await user.type(within(form()).getByLabelText("Date"), "2026-03-06");
    expect(within(form()).getByText("5 of 7 checks are ticked.")).toBeInTheDocument();
    expect(await wcagViolations(document.body)).toEqual([]);
  });

  it("has keys that follow the reading order of the form", async () => {
    const { user } = await openResult(viewOf());
    (document.activeElement as HTMLElement).blur();
    // Skip the buttons above the form: the first control of the form is the first item.
    within(form()).getAllByRole("checkbox")[0]!.focus();
    const labels = await tabOrder(user, 8);
    expect(labels).toEqual([...CHECKLIST.slice(1), "Your name", "Date"]);
  });
});
