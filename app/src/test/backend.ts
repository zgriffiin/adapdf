// The Tauri layer, replaced for the tests: `invoke` answers from a table of
// commands, the file chooser gives a path, and the drag and drop listener is kept
// so that a test can drop a file.
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import axe from "axe-core";
import { vi } from "vitest";

import type {
  ApplyResult,
  CheckResult,
  ExportResult,
  Finding,
  OfferView,
  Report,
  ReportView,
  TitleSuggestion,
  WorkingInfo,
} from "../types";

type Handler = (args: Record<string, unknown>) => unknown;
export type Handlers = Record<string, Handler>;

export interface Backend {
  /** The calls of `invoke`, in order. */
  calls: { command: string; args: Record<string, unknown> }[];
  /** What the file chooser gives next (`null` is "cancelled"). */
  chosen: (string | null)[];
  /** Drops files on the window. */
  drop: (paths: string[]) => void;
}

/**
 * Sets up the mocks (`vi.mock` of the three Tauri modules is in `setup.ts`).
 * A handler that returns a value answers the command; one that throws makes
 * the command fail like a Rust `Err` (the page gets the thrown value).
 */
export function setupBackend(given: Handlers): Backend {
  // `working_info` answers from the path unless a test gives its own answer.
  const handlers: Handlers = { working_info: (args) => workingInfoOf(String(args.path)), ...given };
  const backend: Backend = { calls: [], chosen: [], drop: () => undefined };
  vi.mocked(invoke).mockImplementation(async (command: string, args?: unknown) => {
    const given = (args ?? {}) as Record<string, unknown>;
    backend.calls.push({ command, args: given });
    const handler = handlers[command];
    if (handler === undefined) {
      throw { message: `The test has no answer for ${command}.` };
    }
    return handler(given);
  });
  vi.mocked(open).mockImplementation(async () => backend.chosen.shift() ?? null);
  vi.mocked(getCurrentWebview).mockImplementation(
    () =>
      ({
        onDragDropEvent: async (handler: (event: { payload: unknown }) => void) => {
          backend.drop = (paths) => handler({ payload: { type: "drop", paths } });
          return () => undefined;
        },
      }) as unknown as ReturnType<typeof getCurrentWebview>,
  );
  return backend;
}

/** A promise that a test settles by hand, for a command that takes a while. */
export function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

/**
 * axe-core for WCAG 2.1 A and AA. Returns the violations, as text for a failed test.
 * The colour contrast rule is off: jsdom has no canvas and no layout, so axe cannot
 * work it out there. The colours of the page are Carbon's white theme and the few
 * colours in `styles.scss` (checked by hand: see "Known gaps in M5a").
 */
export async function wcagViolations(root: Element): Promise<string[]> {
  const results = await axe.run(root, {
    runOnly: { type: "tag", values: ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"] },
    rules: { "color-contrast": { enabled: false } },
  });
  return results.violations.map(
    (v) => `${v.id}: ${v.help} (${v.nodes.map((n) => n.target.join(" ")).join("; ")})`,
  );
}

// --------------------------------------------------------------- sample data

function finding(
  rule_id: string,
  severity: Finding["severity"],
  message: string,
  fix: Finding["fix"],
  extra: Partial<Finding> = {},
): Finding {
  return {
    rule_id,
    severity,
    message,
    how_to_fix: `How to fix ${rule_id}.`,
    location: {
      place: 'paragraph 5 "3. Close the file.", after the heading "Test guide"',
      heading: "Test guide",
      snippet: "3. Close the file.",
      paragraph_index: 4,
      part: "word/document.xml",
    },
    fix,
    suggestion: null,
    ...extra,
  };
}

/** The findings of `LST-002-fail.docx`: an error with a fix, and a warning without one. */
export const lst002 = {
  lst001: finding(
    "LST-001",
    "error",
    'This paragraph starts with a typed number ("3.") instead of a real numbered list.',
    { kind: "auto", id: "apply-list-style" },
  ),
  lst002: finding(
    "LST-002",
    "warning",
    "This list mixes typed numbers with automatic numbering.",
    { kind: "manual" },
  ),
};

/** The one warning of `TXB-004-fail.docx`. */
export const txb004 = finding(
  "TXB-004",
  "warning",
  "2 empty paragraphs in a row are used to make space.",
  { kind: "auto", id: "delete-empty-paragraphs" },
);

/** `DOC-001-fail.docx`: no title. The one-click texts are in the offer, not in the finding. */
export const doc001 = finding(
  "DOC-001",
  "error",
  "The document has no title in its properties.",
  { kind: "ask", id: "set-title" },
  {
    location: {
      place: "The document's properties (File > Info)",
      heading: null,
      snippet: null,
      paragraph_index: null,
      part: "docProps/core.xml",
    },
  },
);

/** The three one-click texts of a title (some documents have fewer). */
export const titleSuggestions: TitleSuggestion[] = [
  { source: "file-name", label: "Use the file name", text: "Guide" },
  { source: "title-paragraph", label: "Use the Title paragraph", text: "Parks Training Guide" },
  { source: "first-heading1", label: "Use the first Heading 1", text: "Pre-Requisites:" },
];

/** `IMG-001-fail.docx`: a picture with no alt text. */
export const img001 = finding(
  "IMG-001",
  "error",
  "This picture has no alt text.",
  { kind: "ask", id: "set-alt-text" },
  { suggestion: "image1.png" },
);

export const img007 = finding(
  "IMG-007",
  "error",
  "Arrows and boxes are drawn on a screenshot.",
  { kind: "ask", id: "mark-annotations-decorative" },
);

export function checkOf(path: string, findings: Finding[]): CheckResult {
  const errors = findings.filter((f) => f.severity === "error").length;
  return {
    file: path,
    errors,
    warnings: findings.length - errors,
    exit_code: errors > 0 ? 1 : 0,
    findings,
  };
}

export function offerOf(
  f: Finding,
  options: {
    position?: number;
    availability?: OfferView["availability"];
    suggestions?: TitleSuggestion[];
  } = {},
): OfferView {
  const fix = f.fix.kind === "manual" ? "none" : f.fix.id;
  return {
    finding: f,
    suggestions: options.suggestions ?? (fix === "set-title" ? titleSuggestions : []),
    position: options.position ?? 0,
    fix,
    ask: f.fix.kind === "ask",
    description: `Do ${fix}.`,
    availability: options.availability ?? { status: "ready" },
  };
}

export const ORIGINAL = "C:\\Docs\\Guide.docx";
export const COPY = "C:\\Docs\\Guide - Adapdf\\Guide (accessible).docx";
export const GUIDE_FOLDER = "C:\\Docs\\Guide - Adapdf";

/** The files of the guide `Guide.docx`, for the path that was chosen. */
export function workingInfoOf(path: string, copyExists = false): WorkingInfo {
  const isCopy = path === COPY;
  return {
    chosen: path,
    original: ORIGINAL,
    copy: COPY,
    folder: GUIDE_FOLDER,
    chosen_folder: isCopy ? GUIDE_FOLDER : "C:\\Docs",
    is_copy: isCopy,
    copy_exists: isCopy || copyExists,
  };
}

export function applyResultOf(findings: Finding[], extra: Partial<ApplyResult> = {}): ApplyResult {
  const errors = findings.filter((f) => f.severity === "error").length;
  return {
    copy: COPY,
    working_file: COPY,
    original: ORIGINAL,
    applied: [{ rule_id: "LST-001", summary: "Applied the list style to paragraph 5." }],
    at_export: [],
    refused: [],
    findings,
    errors,
    warnings: findings.length - errors,
    exit_code: errors > 0 ? 1 : 0,
    ...extra,
  };
}

export const CHECKLIST = [
  "Each piece of alt text describes what the image shows, for the guide's purpose.",
  "Images marked decorative really are decorative.",
  "The reading order makes sense when read aloud. Check with the PAC screen-reader preview.",
  "Colour is not the only way information is shown.",
  "Link text makes sense out of context.",
  "Headings describe their section.",
  "Tables are data tables, not layout.",
];

export function reportOf(
  options: {
    verdict?: "pass" | "fail";
    precheck?: Finding[];
    name?: string | null;
    date?: string | null;
    done?: boolean[];
    failedRules?: number;
  } = {},
): Report {
  const pass = (options.verdict ?? "pass") === "pass";
  const precheck = options.precheck ?? [];
  const failedRules = options.failedRules ?? (pass ? 0 : 1);
  return {
    schema_version: 3,
    summary: {
      verdict: pass ? "pass" : "fail",
      errors: pass ? 0 : Math.max(failedRules, precheck.filter((f) => f.severity === "error").length),
      warnings: precheck.filter((f) => f.severity === "warning").length,
      precheck_errors: precheck.filter((f) => f.severity === "error").length,
      precheck_warnings: precheck.filter((f) => f.severity === "warning").length,
      pdf_failed_rules: failedRules,
      reason: pass ? null : "The report has 1 failed rule of the PDF/UA-1 check (veraPDF).",
      tool: "adapdf",
      tool_version: "0.2.0",
      generated_local: "2026-03-05 10:15:30",
      document: "Guide.docx",
      pdf: "Guide.pdf",
    },
    precheck,
    fixes: { original: null, applied: [] },
    pac: { report_file: null, pac_version: null, result: null },
    pdf_metadata: { status: "done", changes: [], table_scopes: [], unscoped_headers: [], issues: [] },
    verapdf: {
      status: "ran",
      verapdf_version: "1.28.2",
      validation: {
        ua1: {
          profile_name: "PDF/UA-1 validation profile",
          failed_rules: failedRules,
          failed_checks: failedRules,
          passed_rules: 100,
          passed_checks: 1000,
          findings:
            failedRules === 0
              ? []
              : [
                  {
                    clause: "7.18.1",
                    test_number: 2,
                    message: "A link has no description.",
                    how_to_fix: "Give the link a ScreenTip.",
                    failed_checks: 1,
                    place: "Link 2",
                    context: "root/K[0](5 0 obj SEDocument Document)/K[1](9 0 obj SELink Link)",
                  },
                ],
          verapdf_version: "1.28.2",
        },
      },
    },
    checklist: {
      items: CHECKLIST.map((text, i) => ({ text, done: options.done?.[i] ?? false })),
      name: options.name ?? null,
      date: options.date ?? null,
    },
  };
}

export function viewOf(report: Report = reportOf()): ReportView {
  return {
    report,
    report_json: `${GUIDE_FOLDER}\\Guide.accessibility.json`,
    report_html: `${GUIDE_FOLDER}\\Guide.accessibility.html`,
    pdf: report.summary.pdf === null ? null : `${GUIDE_FOLDER}\\Guide.pdf`,
  };
}

export function exportResultOf(view: ReportView): ExportResult {
  return {
    exit_code: view.report.summary.verdict === "pass" ? 0 : 1,
    view,
    console: "",
    files: [view.report_json, view.report_html],
  };
}
