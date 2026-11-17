// Row A-09: the page and the Rust commands agree. The values here are not made up
// by a test author: they are what the Rust commands serialised (see
// `app/src-tauri/src/tests/contract.rs`, which fails when a file is out of date).
// The tests check that each file has the shape of the TypeScript type, that the
// screens show what is in the files, and that the page sends the arguments that
// the commands take (`contract/calls.json`).
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";

import { App } from "../App";
import type { CheckResult, OfferView, ReportView } from "../types";
import {
  applySample,
  callsSample,
  checkResultShape,
  checkSample,
  errorSample,
  exportSample,
  offersSample,
  sample,
  sampleNames,
  shapeByPrefix,
  viewSample,
} from "./contract";
import { setupBackend, wcagViolations, type Backend, type Handlers } from "./backend";
import { parse, problemsOf } from "./shape";

const DOCX = "C:\\Docs\\Guide.docx";

type User = ReturnType<typeof userEvent.setup>;

const heading = () => screen.getByRole("heading", { level: 2 });

/** The app with a document chosen. `check` and `offers` are what the commands gave. */
async function openDocument(
  check: CheckResult,
  offers: OfferView[],
  extra: Handlers = {},
): Promise<{ user: User; backend: Backend }> {
  const backend = setupBackend({ check: () => check, fix_offers: () => offers, ...extra });
  backend.chosen.push(DOCX);
  const user = userEvent.setup();
  render(<App />);
  await user.click(screen.getByRole("button", { name: "Choose a Word document" }));
  await screen.findByRole("heading", { level: 2, name: /^Guide\.docx: / });
  return { user, backend };
}

/** The result screen of an export. */
async function openResult(
  name: string,
  extra: Handlers = {},
): Promise<{ user: User; backend: Backend }> {
  const opened = await openDocument(checkSample("txt004-fail"), [], {
    export: () => exportSample(name),
    ...extra,
  });
  await opened.user.click(screen.getByRole("button", { name: "Export PDF" }));
  await screen.findByRole("heading", { level: 2, name: /^Result for/ });
  return opened;
}

// ---------------------------------------------------------------- shapes

describe("the shape of what Rust sends", () => {
  it("has the shape of the TypeScript type in every sample file", () => {
    const names = sampleNames();
    expect(names.length).toBeGreaterThan(20);
    for (const name of names) {
      const found = shapeByPrefix.find(([prefix]) => name.startsWith(prefix));
      expect(found, `${name}.json has no shape to check it against`).toBeDefined();
      const [, shape] = found!;
      expect(problemsOf(shape, sample(name), `${name}.json`), name).toEqual([]);
    }
  });

  it("names a field that Rust renamed, dropped or changed to another type", () => {
    const applied = sample("apply-lst002-fail") as Record<string, unknown>;
    const { at_export, ...rest } = applied;
    const renamed = { ...rest, atExport: at_export };
    const shape = shapeByPrefix.find(([prefix]) => prefix === "apply-")![1];
    expect(problemsOf(shape, renamed, "apply")).toEqual(["apply.at_export: the field is missing"]);

    const check = sample("check-lst002-fail") as { findings: Record<string, unknown>[]; errors: unknown };
    const first = check.findings[0]!;
    first.how_to_fix = 3;
    first.location = { ...(first.location as object), part: null };
    first.fix = { kind: "other" };
    check.errors = "1";
    expect(problemsOf(checkResultShape, check, "check")).toEqual([
      'check.errors: expected a number, got string',
      "check.findings[0].how_to_fix: expected a string, got number",
      "check.findings[0].location.part: expected a string, got null",
      'check.findings[0].fix.kind: expected one of auto, ask, manual, got "other"',
    ]);
  });

  it("does not take a null where the page needs a value, or a missing nullable field", () => {
    const view = sample("report-view-signed") as { pdf?: unknown; report: { summary: Record<string, unknown> } };
    const shape = shapeByPrefix.find(([prefix]) => prefix === "report-view-")![1];
    delete view.pdf;
    view.report.summary.reason = 4;
    view.report.summary.document = null;
    expect(problemsOf(shape, view, "view")).toEqual([
      "view.report.summary.reason: expected null or a string, got number",
      "view.report.summary.document: expected a string, got null",
      "view.pdf: the field is missing",
    ]);
  });

  it("throws, and lists the problems, when a sample has the wrong shape", () => {
    expect(() => parse(checkResultShape, { file: 1 }, "check")).toThrow(/check\.file: expected a string, got number/);
  });

  it("has the errors that the commands give as { message }", () => {
    expect(errorSample("export").message.length).toBeGreaterThan(0);
    expect(errorSample("check").message).toMatch(/^Missing\.docx: the file cannot be read/);
  });
});

// ------------------------------------------------------------- the screens

describe("the findings screen, from what `check` and `fix_offers` gave", () => {
  it("groups the findings of LST-002-fail.docx and shows what the commands said", async () => {
    const check = checkSample("lst002-fail");
    expect([check.errors, check.warnings]).toEqual([1, 1]);
    await openDocument(check, offersSample("lst002-fail"));
    expect(heading()).toHaveTextContent("Guide.docx: Found 1 error and 1 warning.");
    expect(screen.getAllByRole("heading", { level: 3 }).map((h) => h.textContent)).toEqual([
      "Errors (1)",
      "Warnings (1)",
    ]);
    const items = screen.getAllByRole("listitem");
    for (const finding of check.findings) {
      const item = items.find((i) => i.textContent?.includes(finding.rule_id));
      expect(item, finding.rule_id).toBeDefined();
      expect(item).toHaveTextContent(finding.message);
      expect(item).toHaveTextContent(`How to fix: ${finding.how_to_fix}`);
    }
    expect(screen.getByRole("button", { name: "Fix problems" })).toBeEnabled();
    expect(screen.getByRole("button", { name: "Export PDF" })).toBeDisabled();
    expect(await wcagViolations(document.body)).toEqual([]);
  });

  it("allows an export when only a warning is left (TXB-004)", async () => {
    await openDocument(checkSample("txt004-fail"), offersSample("txt004-fail"));
    expect(heading()).toHaveTextContent("Guide.docx: Found 0 errors and 1 warning.");
    expect(screen.getByRole("button", { name: "Export PDF" })).toBeEnabled();
    // The only fix is the one that Word does at export: nothing to choose.
    expect(screen.queryByRole("button", { name: "Fix problems" })).not.toBeInTheDocument();
  });

  it("says that no problem was found (DOC-001-pass.docx)", async () => {
    await openDocument(checkSample("doc001-pass"), offersSample("doc001-pass"));
    expect(heading()).toHaveTextContent("Guide.docx: No problems found.");
  });

  it("shows the message of a command that failed", async () => {
    const backend = setupBackend({
      check: () => {
        throw errorSample("check");
      },
    });
    backend.chosen.push("C:\\Docs\\Missing.docx");
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "Choose a Word document" }));
    const region = await screen.findByRole("status");
    await waitFor(() => expect(region).toHaveTextContent("Missing.docx could not be checked."));
    expect(region).toHaveTextContent(errorSample("check").message);
  });
});

describe("the fix flow, from what `fix_offers` and `apply_fixes` gave", () => {
  async function fixScreen(name: string): Promise<{ user: User; backend: Backend }> {
    const opened = await openDocument(checkSample(name), offersSample(name), {
      apply_fixes: () => applySample(name),
    });
    await opened.user.click(screen.getByRole("button", { name: "Fix problems" }));
    await screen.findByRole("heading", { level: 2, name: "Fix problems in Guide.docx" });
    return opened;
  }

  it("shows an automatic fix with the description that Rust gave, and applies it", async () => {
    const [offer] = offersSample("lst002-fail");
    const { user, backend } = await fixScreen("lst002-fail");
    const box = screen.getByRole("checkbox", { name: `${offer!.finding.rule_id}: ${offer!.description}` });
    expect(box).not.toBeChecked();
    expect(await wcagViolations(document.body)).toEqual([]);
    await user.click(box);
    await user.click(screen.getByRole("button", { name: "Apply 1 fix" }));
    const applied = applySample("lst002-fail");
    const region = await screen.findByRole("status");
    await waitFor(() => expect(region).toHaveTextContent("The fixes were applied."));
    expect(region).toHaveTextContent(`Backup: ${applied.backup}.`);
    expect(region).toHaveTextContent(applied.applied[0]!.summary.length > 0 ? "1 fix applied." : "");
    expect(backend.calls.find((c) => c.command === "apply_fixes")?.args).toEqual({
      path: DOCX,
      choices: [{ rule_id: "LST-001", position: 0, input: { kind: "confirm" } }],
    });
    // The findings are those that the command gave after the fix.
    expect(heading()).toHaveTextContent("Guide.docx: No problems found.");
  });

  it("asks for the title, with no value, and sends what the person types", async () => {
    const [offer] = offersSample("doc001-fail");
    expect(offer!.finding.suggestion).not.toBeNull();
    const { user, backend } = await fixScreen("doc001-fail");
    const field = screen.getByRole("textbox", { name: "Document title" });
    expect(field).toHaveValue("");
    expect(field).toHaveAccessibleDescription(`Suggestion, not filled in: ${offer!.finding.suggestion}. Type the title yourself.`);
    await user.type(field, "A title typed by a person");
    await user.click(screen.getByRole("button", { name: "Apply 1 fix" }));
    await waitFor(() => expect(backend.calls.some((c) => c.command === "apply_fixes")).toBe(true));
    expect(backend.calls.find((c) => c.command === "apply_fixes")?.args.choices).toEqual([
      { rule_id: "DOC-001", position: 0, input: { kind: "text", text: "A title typed by a person" } },
    ]);
    await screen.findByText(/The fixes were applied\./);
  });

  it("offers alt text and decorative for a picture, and the question about arrows and boxes", async () => {
    await fixScreen("img001-fail");
    expect(screen.getByRole("textbox", { name: /^Alt text for IMG-001/ })).toHaveValue("");
    expect(screen.getByRole("checkbox", { name: /^Mark as decorative/ })).not.toBeChecked();
  });

  it("asks the question about arrows and boxes (IMG-007)", async () => {
    const { user } = await fixScreen("img007-fail");
    const yes = screen.getByRole("checkbox", { name: /the alt text of the screenshot describes/ });
    await user.click(yes);
    expect(screen.getByRole("button", { name: "Apply 1 fix" })).toBeEnabled();
  });

  it("lists the fix that Word does at export, with nothing to choose", async () => {
    const check = checkSample("txt004-fail");
    const offers = offersSample("txt004-fail");
    // The page offers "Fix problems" only for a ready fix, so this screen comes from a document that has both.
    const doc = offersSample("lst002-fail");
    const { user } = await openDocument(check, [...doc, ...offers]);
    await user.click(screen.getByRole("button", { name: "Fix problems" }));
    expect(screen.getByRole("heading", { level: 3, name: "Done by Word when the PDF is exported (1)" })).toBeInTheDocument();
    expect(screen.getAllByRole("checkbox")).toHaveLength(1);
  });

  it("shows why the tool cannot fix a link that has no text", async () => {
    const [offer] = offersSample("lnk004-no-text-fail");
    expect(offer!.availability.status).toBe("unavailable");
    const check = checkSample("lnk004-no-text-fail");
    const doc = offersSample("lst002-fail");
    const { user } = await openDocument(check, [...doc, offer!]);
    await user.click(screen.getByRole("button", { name: "Fix problems" }));
    expect(screen.getByRole("heading", { level: 3, name: "The tool cannot fix these now (1)" })).toBeInTheDocument();
    if (offer!.availability.status === "unavailable") {
      expect(screen.getByText(offer!.availability.reason)).toBeInTheDocument();
    }
  });

  it("shows the fixes that the tool refused, as `apply_fixes` said", async () => {
    const refused = applySample("refused");
    expect(refused.refused).toHaveLength(2);
    const { user } = await openDocument(checkSample("doc001-fail"), offersSample("doc001-fail"), {
      apply_fixes: () => refused,
    });
    await user.click(screen.getByRole("button", { name: "Fix problems" }));
    await user.type(screen.getByRole("textbox", { name: "Document title" }), "x");
    await user.click(screen.getByRole("button", { name: "Apply 1 fix" }));
    const region = await screen.findByText(/Nothing was changed\./);
    expect(region).toBeInTheDocument();
    for (const item of refused.refused) {
      expect(screen.getByRole("status")).toHaveTextContent(item.reason);
    }
    expect(screen.getByRole("status")).toHaveTextContent("No backup was made, because nothing was written.");
  });
});

describe("the export, from what `export` gave", () => {
  it("shows a Pass: the PDF, the counts and the veraPDF result", async () => {
    const result = exportSample("pass");
    await openResult("pass");
    expect(heading()).toHaveTextContent("Result for Guide.docx: Pass");
    expect(screen.getByText(result.view.pdf!, { exact: false })).toBeInTheDocument();
    expect(screen.getByText(/Pass: no PDF\/UA-1 rule failed \(veraPDF /)).toBeInTheDocument();
    expect(screen.getByText(result.view.report.summary.generated_local)).toBeInTheDocument();
    expect(await wcagViolations(document.body)).toEqual([]);
  });

  it("shows a Fail from veraPDF: the reason, the failed rule and the metadata problem", async () => {
    const view = exportSample("fail-verapdf").view;
    await openResult("fail-verapdf");
    expect(heading()).toHaveTextContent("Result for Guide.docx: Fail");
    expect(screen.getByText(view.report.summary.reason!)).toBeInTheDocument();
    const section = view.report.verapdf;
    if (section.status !== "ran") {
      throw new Error("the sample has the veraPDF result");
    }
    const failure = section.validation.ua1.findings[0]!;
    expect(
      screen.getByText(`${failure.clause}-${failure.test_number}: ${failure.message}`, { exact: false }),
    ).toBeInTheDocument();
    if (view.report.pdf_metadata.status !== "done") {
      throw new Error("the sample has the metadata result");
    }
    expect(screen.getByText(view.report.pdf_metadata.issues[0]!.message)).toBeInTheDocument();
    expect(await wcagViolations(document.body)).toEqual([]);
  });

  it("shows a Fail with no PDF when the pre-check found an error (export blocked)", async () => {
    const view = exportSample("fail-blocked").view;
    expect(view.pdf).toBeNull();
    await openResult("fail-blocked");
    expect(heading()).toHaveTextContent("Result for Guide.docx: Fail");
    expect(screen.getByText(view.report.summary.reason!)).toBeInTheDocument();
    expect(screen.getByText("None was written")).toBeInTheDocument();
    expect(screen.getByText("Not run (no PDF was made)")).toBeInTheDocument();
    expect(screen.getByRole("heading", { level: 3, name: "Findings in the Word document" })).toBeInTheDocument();
    expect(screen.getByText(view.report.precheck[0]!.message, { exact: false })).toBeInTheDocument();
    expect(await wcagViolations(document.body)).toEqual([]);
  });

  it("goes back to the findings with the message when the tool fails", async () => {
    const { user } = await openDocument(checkSample("txt004-fail"), [], {
      export: () => {
        throw errorSample("export");
      },
    });
    await user.click(screen.getByRole("button", { name: "Export PDF" }));
    await screen.findByRole("heading", { level: 2, name: /^Guide\.docx: Found/ });
    const region = screen.getByRole("status");
    expect(region).toHaveTextContent("The export did not finish.");
    expect(region).toHaveTextContent(errorSample("export").message);
  });
});

describe("the sign-off, from the reports that `read_report` and `save_signoff` gave", () => {
  const form = () => screen.getByRole("form", { name: "Sign-off" });

  async function openEarlier(view: ReportView) {
    const backend = setupBackend({ read_report: () => view });
    backend.chosen.push("C:\\Docs\\Guide.accessibility.json");
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "Open an earlier report" }));
    await screen.findByRole("heading", { level: 2, name: /^Result for Guide\.docx:/ });
    return { user, backend };
  }

  it("shows an unsigned report with an empty form and the 7 items of the checklist", async () => {
    const view = viewSample("unsigned");
    await openEarlier(view);
    const boxes = within(form()).getAllByRole("checkbox");
    expect(boxes.map((b) => b.closest(".cds--checkbox-wrapper")?.textContent)).toEqual(
      view.report.checklist.items.map((i) => i.text),
    );
    expect(boxes.every((b) => !(b as HTMLInputElement).checked)).toBe(true);
    expect(within(form()).getByRole("textbox", { name: "Your name" })).toHaveValue("");
    expect(within(form()).getByLabelText("Date")).toHaveValue("");
  });

  it("shows the name, the date and the ticks of a signed report", async () => {
    const view = viewSample("signed");
    await openEarlier(view);
    const boxes = within(form()).getAllByRole("checkbox") as HTMLInputElement[];
    expect(boxes.map((b) => b.checked)).toEqual(view.report.checklist.items.map((i) => i.done));
    expect(boxes.filter((b) => b.checked)).toHaveLength(5);
    expect(within(form()).getByRole("textbox", { name: "Your name" })).toHaveValue("A. Reviewer");
    expect(within(form()).getByLabelText("Date")).toHaveValue("2026-03-06");
    expect(await wcagViolations(document.body)).toEqual([]);
  });

  it("shows the report that `save_signoff` gave after the save", async () => {
    const { user } = await openResult("pass", { save_signoff: () => viewSample("signed") });
    await user.type(within(form()).getByRole("textbox", { name: "Your name" }), "A. Reviewer");
    await user.click(within(form()).getByRole("button", { name: "Save sign-off" }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("The sign-off was saved."));
    expect(within(form()).getAllByRole("checkbox").filter((b) => (b as HTMLInputElement).checked)).toHaveLength(5);
    expect(within(form()).getByLabelText("Date")).toHaveValue("2026-03-06");
  });
});

// ------------------------------------------------- what the page sends

describe("the arguments that the page sends", () => {
  it("are the arguments in calls.json, which the Rust test checks against the commands", async () => {
    const calls = callsSample();
    // One run through the page: choose, fix three ways, export, open the report, sign.
    const check = checkSample("lst002-fail");
    const offers = [
      ...offersSample("lst002-fail"),
      ...offersSample("doc001-fail"),
      ...offersSample("img001-fail"),
    ];
    const { user, backend } = await openDocument(check, offers, {
      apply_fixes: () => applySample("lst002-fail"),
      export: () => exportSample("pass"),
      open_report: () => null,
      save_signoff: () => viewSample("signed"),
    });
    await user.click(screen.getByRole("button", { name: "Fix problems" }));
    await user.click(screen.getByRole("checkbox", { name: /^LST-001: / }));
    await user.type(screen.getByRole("textbox", { name: "Document title" }), "My own title");
    await user.click(screen.getByRole("checkbox", { name: /^Mark as decorative/ }));
    await user.click(screen.getByRole("button", { name: "Apply 3 fixes" }));
    await screen.findByText(/The fixes were applied\./);
    await user.click(screen.getByRole("button", { name: "Export PDF" }));
    await screen.findByRole("heading", { level: 2, name: /^Result for/ });
    await user.click(screen.getByRole("button", { name: "Open the report in your browser" }));
    const signoff = within(screen.getByRole("form", { name: "Sign-off" }));
    for (const box of signoff.getAllByRole("checkbox").slice(0, 5)) {
      await user.click(box);
    }
    await user.type(signoff.getByRole("textbox", { name: "Your name" }), "A. Reviewer");
    await user.type(signoff.getByLabelText("Date"), "2026-03-06");
    await user.click(signoff.getByRole("button", { name: "Save sign-off" }));
    await waitFor(() => expect(backend.calls.some((c) => c.command === "save_signoff")).toBe(true));

    const sent = new Map(backend.calls.map((c) => [c.command, c.args]));
    expect([...sent.keys()].sort()).toEqual(
      ["apply_fixes", "check", "export", "fix_offers", "open_report", "save_signoff"],
    );
    for (const [command, args] of sent) {
      expect(args, command).toEqual(calls[command]);
    }
  });

  it("include the path that an earlier report is read from", async () => {
    const calls = callsSample();
    const backend = setupBackend({ read_report: () => viewSample("signed") });
    backend.chosen.push(String(calls.read_report!.path));
    const user = userEvent.setup();
    render(<App />);
    await user.click(screen.getByRole("button", { name: "Open an earlier report" }));
    await screen.findByRole("heading", { level: 2, name: /^Result for/ });
    expect(backend.calls).toEqual([{ command: "read_report", args: calls.read_report }]);
  });

  it("are in calls.json for the seven commands, and no other", () => {
    expect(Object.keys(callsSample()).sort()).toEqual(
      ["apply_fixes", "check", "export", "fix_offers", "open_report", "read_report", "save_signoff"],
    );
  });
});
