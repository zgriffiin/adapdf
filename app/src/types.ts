// The shapes that the Rust commands send and receive (app/src-tauri/src/commands.rs
// and the JSON of the library: findings, report). Field names are the Rust names.

export type Severity = "error" | "warning";

export type FixKind =
  | { kind: "auto"; id: string }
  | { kind: "ask"; id: string }
  | { kind: "manual" };

export interface Location {
  heading: string | null;
  snippet: string | null;
  /** Counts from 0. */
  paragraph_index: number | null;
  part: string;
}

export interface Finding {
  rule_id: string;
  severity: Severity;
  message: string;
  how_to_fix: string;
  location: Location;
  fix: FixKind;
  /** A proposal. The app shows it as a hint and never types it for the person. */
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

export interface OfferView {
  finding: Finding;
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

export interface ApplyResult {
  backup: string | null;
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
      issues: { severity: Severity; message: string; how_to_fix: string }[];
    };

/** The parts of the report that the app shows. The JSON has more. */
export interface Report {
  schema_version: number;
  summary: {
    verdict: Verdict;
    errors: number;
    warnings: number;
    reason: string | null;
    tool: string;
    tool_version: string;
    generated_local: string;
    document: string;
    pdf: string | null;
  };
  precheck: Finding[];
  fixes: { applied: { rule: string; summary: string }[]; backup: string | null };
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

export interface Signoff {
  name: string;
  /** `yyyy-mm-dd`, or empty. */
  date: string;
  items: boolean[];
}

/** What a failed command gives back. */
export interface CommandError {
  message: string;
}
