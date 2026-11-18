// The shapes that the Rust commands send and receive (app/src-tauri/src/commands.rs
// and the JSON of the library: findings, report). Field names are the Rust names.

export type Severity = "error" | "warning";

export type FixKind =
  | { kind: "auto"; id: string }
  | { kind: "ask"; id: string }
  | { kind: "manual" };

export interface Location {
  /** Where the problem is, in words that a Word user knows. The page shows this. */
  place: string;
  heading: string | null;
  snippet: string | null;
  /** Counts from 0. */
  paragraph_index: number | null;
  /** The part of the .docx, for tooling. The page does not show it. */
  part: string;
}

export interface Finding {
  rule_id: string;
  severity: Severity;
  message: string;
  how_to_fix: string;
  location: Location;
  fix: FixKind;
  /** A proposal (for example `Heading 2`). The app shows it as a hint and never types it for the person. */
  suggestion: string | null;
}

export interface CheckResult {
  file: string;
  errors: number;
  warnings: number;
  exit_code: number;
  findings: Finding[];
}

export type Availability =
  | { status: "ready" }
  | { status: "at-export" }
  | { status: "unavailable"; reason: string };

/** A one-click text from the document itself, for the title. */
export interface TitleSuggestion {
  source: "file-name" | "title-paragraph" | "first-heading1";
  /** The words of the button, for example "Use the file name". */
  label: string;
  /** The text that goes into the title field when the person chooses it. */
  text: string;
}

export interface OfferView {
  finding: Finding;
  /** For the title only: up to three texts from the document. Nothing is filled in until one is chosen. */
  suggestions: TitleSuggestion[];
  position: number;
  /** For example `set-title`, `set-alt-text`, `mark-annotations-decorative`. */
  fix: string;
  ask: boolean;
  description: string;
  availability: Availability;
}

export type Input =
  | { kind: "confirm" }
  | { kind: "text"; text: string }
  | { kind: "decorative" };

export interface Choice {
  rule_id: string;
  position: number;
  input: Input;
}

export interface NotAppliedView {
  rule_id: string;
  message: string;
  reason: string;
}

/** The files of one guide. */
export interface WorkingInfo {
  /** The file that the person chose. */
  chosen: string;
  /** The Word document that Adapdf never changes. */
  original: string;
  /** The accessible copy: where the fixes go and where Word edits. */
  copy: string;
  /** The folder of the guide, `<name> - Adapdf`. */
  folder: string;
  /** The folder of the chosen file. */
  chosen_folder: string;
  is_copy: boolean;
  copy_exists: boolean;
}

export interface ApplyResult {
  /** The accessible copy that was written (full path). `null` when nothing was written. */
  copy: string | null;
  /** The file to work on from now: the copy when one was written. */
  working_file: string;
  /** The original, which is never changed. */
  original: string;
  applied: { rule_id: string; summary: string }[];
  at_export: NotAppliedView[];
  refused: NotAppliedView[];
  findings: Finding[];
  errors: number;
  warnings: number;
  exit_code: number;
}

export interface ChecklistItem {
  text: string;
  done: boolean;
}

export type Verdict = "pass" | "fail";

export interface VeraFinding {
  clause: string;
  test_number: number;
  message: string;
  how_to_fix: string;
  failed_checks: number;
  /** The first place that failed, in words (`Table 1, body row 1, cell 2`), or `null`. */
  place: string | null;
  /** veraPDF's own path, for the technical detail. */
  context: string | null;
}

export interface ProfileResult {
  profile_name: string;
  failed_rules: number;
  failed_checks: number;
  passed_rules: number;
  passed_checks: number;
  findings: VeraFinding[];
  verapdf_version: string | null;
}

export type VeraPdfSection =
  | { status: "not-run"; reason: string }
  | {
      status: "ran";
      verapdf_version: string | null;
      validation: { ua1: ProfileResult };
    };

export type MetadataSection =
  | { status: "not-run"; reason: string }
  | {
      status: "done";
      changes: { what: string; old: string | null; new: string }[];
      /** Table header cells that were given a scope, for each table. */
      table_scopes: { table: number; column: number; row: number }[];
      /** Table header cells that could not be given a scope (numbers from 1). */
      unscoped_headers: { table: number; row: number; cell: number }[];
      issues: { severity: Severity; message: string; how_to_fix: string }[];
    };

/** One fix that Adapdf made, from the fix log of the document. */
export interface FixEntry {
  time: string;
  rule: string;
  fix: string;
  place: string;
  summary: string;
  text: string | null;
}

/** The parts of the report that the app shows. The JSON has more. */
export interface Report {
  schema_version: number;
  summary: {
    verdict: Verdict;
    errors: number;
    warnings: number;
    /** Errors and warnings of the pre-check of the Word document. */
    precheck_errors: number;
    precheck_warnings: number;
    /** Rules of the PDF/UA-1 check (veraPDF) that failed. */
    pdf_failed_rules: number;
    reason: string | null;
    tool: string;
    tool_version: string;
    generated_local: string;
    document: string;
    pdf: string | null;
  };
  precheck: Finding[];
  fixes: { original: string | null; applied: FixEntry[] };
  pac: { report_file: string | null; pac_version: string | null; result: string | null };
  pdf_metadata: MetadataSection;
  verapdf: VeraPdfSection;
  checklist: { items: ChecklistItem[]; name: string | null; date: string | null };
}

export interface ReportView {
  report: Report;
  report_json: string;
  report_html: string;
  pdf: string | null;
}

export interface ExportResult {
  exit_code: number;
  view: ReportView;
  console: string;
  files: string[];
}

export type PacResult = "none" | "pass" | "fail";

export interface PacInput {
  result: PacResult;
  version: string;
  /** A file in the folder of the report, or `null`. */
  report_file: string | null;
}

export interface Signoff {
  name: string;
  /** `yyyy-mm-dd`, or empty. */
  date: string;
  items: boolean[];
  pac: PacInput;
}

/** What a failed command gives back. */
export interface CommandError {
  message: string;
}
