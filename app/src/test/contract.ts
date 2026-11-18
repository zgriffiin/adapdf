// The samples that Rust wrote (`app/src/test/contract/*.json`, made by
// `app/src-tauri/src/tests/contract.rs`), and the description of each type that the
// page receives, in the form that `shape.ts` checks. A field of a type in
// `../types.ts` that is not listed here is a compile error.
import type {
  ApplyResult,
  Availability,
  CheckResult,
  CommandError,
  ExportResult,
  Finding,
  FixKind,
  Location,
  MetadataSection,
  OfferView,
  ProfileResult,
  TitleSuggestion,
  Report,
  ReportView,
  Severity,
  VeraFinding,
  VeraPdfSection,
  WorkingInfo,
} from "../types";
import {
  array,
  boolean,
  literal,
  nullable,
  number,
  object,
  parse,
  string,
  tagged,
  type Check,
} from "./shape";

const severity = literal<Severity>("error", "warning");

const location = object<Location>({
  place: string,
  heading: nullable(string),
  snippet: nullable(string),
  paragraph_index: nullable(number),
  part: string,
});

const fixKind = tagged<FixKind, "kind">("kind", {
  auto: { id: string },
  ask: { id: string },
  manual: {},
});

export const findingShape = object<Finding>({
  rule_id: string,
  severity,
  message: string,
  how_to_fix: string,
  location,
  fix: fixKind,
  suggestion: nullable(string),
});

export const checkResultShape = object<CheckResult>({
  file: string,
  errors: number,
  warnings: number,
  exit_code: number,
  findings: array(findingShape),
});

const availability = tagged<Availability, "status">("status", {
  ready: {},
  "at-export": {},
  unavailable: { reason: string },
});

const titleSuggestion = object<TitleSuggestion>({
  source: literal<TitleSuggestion["source"]>("file-name", "title-paragraph", "first-heading1"),
  label: string,
  text: string,
});

export const offerShape = object<OfferView>({
  finding: findingShape,
  suggestions: array(titleSuggestion),
  position: number,
  fix: string,
  ask: boolean,
  description: string,
  availability,
});

const notApplied = object<ApplyResult["refused"][number]>({
  rule_id: string,
  message: string,
  reason: string,
});

export const workingInfoShape = object<WorkingInfo>({
  chosen: string,
  original: string,
  copy: string,
  folder: string,
  chosen_folder: string,
  is_copy: boolean,
  copy_exists: boolean,
});

export const applyResultShape = object<ApplyResult>({
  copy: nullable(string),
  working_file: string,
  original: string,
  applied: array(object<ApplyResult["applied"][number]>({ rule_id: string, summary: string })),
  at_export: array(notApplied),
  refused: array(notApplied),
  findings: array(findingShape),
  errors: number,
  warnings: number,
  exit_code: number,
});

const veraFinding = object<VeraFinding>({
  clause: string,
  test_number: number,
  message: string,
  how_to_fix: string,
  failed_checks: number,
  place: nullable(string),
  context: nullable(string),
});

const profileResult = object<ProfileResult>({
  profile_name: string,
  failed_rules: number,
  failed_checks: number,
  passed_rules: number,
  passed_checks: number,
  findings: array(veraFinding),
  verapdf_version: nullable(string),
});

const veraPdfSection = tagged<VeraPdfSection, "status">("status", {
  "not-run": { reason: string },
  ran: {
    verapdf_version: nullable(string),
    validation: object<Extract<VeraPdfSection, { status: "ran" }>["validation"]>({ ua1: profileResult }),
  },
});

const metadataSection = tagged<MetadataSection, "status">("status", {
  "not-run": { reason: string },
  done: {
    changes: array(
      object<Extract<MetadataSection, { status: "done" }>["changes"][number]>({
        what: string,
        old: nullable(string),
        new: string,
      }),
    ),
    table_scopes: array(
      object<Extract<MetadataSection, { status: "done" }>["table_scopes"][number]>({
        table: number,
        column: number,
        row: number,
      }),
    ),
    unscoped_headers: array(
      object<Extract<MetadataSection, { status: "done" }>["unscoped_headers"][number]>({
        table: number,
        row: number,
        cell: number,
      }),
    ),
    issues: array(
      object<Extract<MetadataSection, { status: "done" }>["issues"][number]>({
        severity,
        message: string,
        how_to_fix: string,
      }),
    ),
  },
});

export const reportShape = object<Report>({
  schema_version: number,
  summary: object<Report["summary"]>({
    verdict: literal<Report["summary"]["verdict"]>("pass", "fail"),
    errors: number,
    warnings: number,
    precheck_errors: number,
    precheck_warnings: number,
    pdf_failed_rules: number,
    reason: nullable(string),
    tool: string,
    tool_version: string,
    generated_local: string,
    document: string,
    pdf: nullable(string),
  }),
  precheck: array(findingShape),
  fixes: object<Report["fixes"]>({
    original: nullable(string),
    applied: array(
      object<Report["fixes"]["applied"][number]>({
        time: string,
        rule: string,
        fix: string,
        place: string,
        summary: string,
        text: nullable(string),
      }),
    ),
  }),
  pac: object<Report["pac"]>({
    report_file: nullable(string),
    pac_version: nullable(string),
    result: nullable(string),
  }),
  pdf_metadata: metadataSection,
  verapdf: veraPdfSection,
  checklist: object<Report["checklist"]>({
    items: array(object<Report["checklist"]["items"][number]>({ text: string, done: boolean })),
    name: nullable(string),
    date: nullable(string),
  }),
});

export const reportViewShape = object<ReportView>({
  report: reportShape,
  report_json: string,
  report_html: string,
  pdf: nullable(string),
});

export const exportResultShape = object<ExportResult>({
  exit_code: number,
  view: reportViewShape,
  console: string,
  files: array(string),
});

export const commandErrorShape = object<CommandError>({ message: string });

// ---------------------------------------------------------------- the files

const files = import.meta.glob<unknown>("./contract/*.json", { eager: true, import: "default" });

/** The shape that goes with the start of a file name. */
export const shapeByPrefix: [string, Check<unknown>][] = [
  ["check-", checkResultShape],
  ["working-info-", workingInfoShape],
  ["offers-", array(offerShape)],
  ["apply-", applyResultShape],
  ["export-", exportResultShape],
  ["report-view-", reportViewShape],
  ["report-file-", reportShape],
  ["command-error-", commandErrorShape],
];

/** The names of the sample files, without the folder and `.json`. */
export function sampleNames(): string[] {
  return Object.keys(files)
    .map((path) => path.replace(/^\.\/contract\//, "").replace(/\.json$/, ""))
    .filter((name) => name !== "calls")
    .sort();
}

/** What Rust wrote, as it is. */
export function sample(name: string): unknown {
  const value = files[`./contract/${name}.json`];
  if (value === undefined) {
    throw new Error(`There is no sample file ${name}.json.`);
  }
  // Each test gets its own copy, so that a test cannot change what another reads.
  return structuredClone(value);
}

/** What Rust wrote, checked and typed. */
const typed =
  <T>(prefix: string, check: Check<T>) =>
  (name: string): T =>
    parse(check, sample(`${prefix}-${name}`), `${prefix}-${name}.json`);

export const checkSample = typed("check", checkResultShape);
export const workingInfoSample = typed("working-info", workingInfoShape);
export const offersSample = typed("offers", array(offerShape));
export const applySample = typed("apply", applyResultShape);
export const exportSample = typed("export", exportResultShape);
export const viewSample = typed("report-view", reportViewShape);
export const reportFileSample = typed("report-file", reportShape);
export const errorSample = typed("command-error", commandErrorShape);

/** The arguments that the page sends with each command, by command name (written by hand). */
export const callsSample = (): Record<string, Record<string, unknown>> =>
  structuredClone(files["./contract/calls.json"]) as Record<string, Record<string, unknown>>;
