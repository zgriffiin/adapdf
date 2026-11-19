import { Button } from "@carbon/react";

import type { ReportView, Signoff } from "../types";
import { folderOf, plural, precheckText, verdictWord } from "../viewModel";
import { FindingList } from "./FindingList";
import { ResultsHeading, WorkingOn } from "./common";
import { SignoffForm } from "./SignoffForm";

interface Props {
  view: ReportView;
  focusKey: number;
  busy: boolean;
  /** True when the report is about a document that the app has open. */
  canGoBack: boolean;
  onOpenReport: () => void;
  onOpenFolder: () => void;
  onChoosePacReport: () => Promise<string | null>;
  onSaveSignoff: (signoff: Signoff) => void;
  onBack: () => void;
  onStartOver: () => void;
}

function veraPdfSummary(view: ReportView): string {
  const section = view.report.verapdf;
  if (section.status === "not-run") {
    return "Not run (no PDF was made)";
  }
  const ua1 = section.validation.ua1;
  const passed = ua1.failed_rules === 0 && ua1.failed_checks === 0 && ua1.findings.length === 0;
  return passed
    ? `Pass: no PDF/UA-1 rule failed (veraPDF ${section.verapdf_version ?? "version not given"})`
    : `Fail: ${plural(Math.max(ua1.failed_rules, ua1.findings.length), "rule")} of PDF/UA-1 failed (veraPDF ${section.verapdf_version ?? "version not given"})`;
}

/** The result of an export, or of an earlier report that was opened, with the sign-off. */
export function ResultScreen({
  view,
  focusKey,
  busy,
  canGoBack,
  onOpenReport,
  onOpenFolder,
  onChoosePacReport,
  onSaveSignoff,
  onBack,
  onStartOver,
}: Props) {
  const { report } = view;
  const { summary } = report;
  const pass = summary.verdict === "pass";
  const veraFailures = report.verapdf.status === "ran" ? report.verapdf.validation.ua1.findings : [];
  const metadataIssues = report.pdf_metadata.status === "done" ? report.pdf_metadata.issues : [];

  return (
    <section aria-labelledby="result-heading">
      <ResultsHeading focusKey={focusKey} id="result-heading">
        Result for {summary.document}:{" "}
        <span className={`verdict ${pass ? "verdict-pass" : "verdict-fail"}`}>
          {verdictWord(summary.verdict)}
        </span>
      </ResultsHeading>
      {summary.reason !== null ? <p>{summary.reason}</p> : null}
      <p>
        {pass
          ? "The checks that the tool runs found no errors. A person must still check the PDF in PAC and sign off the checklist below."
          : "The PDF does not pass yet. Fix the problems below in Word, then export again."}
      </p>

      <WorkingOn
        path={`${folderOf(view.report_json)}\\${summary.document}`}
        isCopy={report.fixes.original !== null}
        busy={busy}
        onOpenFolder={onOpenFolder}
      />

      <div className="button-row">
        <Button onClick={onOpenReport} disabled={busy}>
          Open the report in your browser
        </Button>
        {canGoBack ? (
          <Button kind="secondary" onClick={onBack} disabled={busy}>
            Back to the findings
          </Button>
        ) : null}
        <Button kind="tertiary" onClick={onStartOver} disabled={busy}>
          Choose another file
        </Button>
      </div>

      <h3 className="section-heading">Summary</h3>
      <dl className="facts">
        <dt>Result</dt>
        <dd>{verdictWord(summary.verdict)}</dd>
        <dt>Word document (pre-check)</dt>
        <dd>{precheckText(summary.precheck_errors, summary.precheck_warnings)}</dd>
        <dt>PDF check (veraPDF PDF/UA-1)</dt>
        <dd>
          {report.verapdf.status === "ran"
            ? `${plural(summary.pdf_failed_rules, "rule")} failed`
            : "Not run (no PDF was made)"}
        </dd>
        <dt>PDF</dt>
        <dd>{view.pdf ?? "None was written"}</dd>
        <dt>veraPDF (PDF/UA-1 check)</dt>
        <dd>{veraPdfSummary(view)}</dd>
        <dt>Date and time</dt>
        <dd>{summary.generated_local}</dd>
        <dt>Report</dt>
        <dd>{view.report_html}</dd>
      </dl>

      <h3 className="section-heading">Fixes applied</h3>
      {report.fixes.original !== null ? (
        <p>
          The original {report.fixes.original} was not changed. Adapdf made its fixes in {summary.document} in the folder{" "}
          {report.fixes.original.replace(/\.docx$/i, "")} - Adapdf.
        </p>
      ) : null}
      {report.fixes.applied.length === 0 ? (
        <p>No fixes were made by Adapdf. The Word document is {summary.document}.</p>
      ) : (
        <ul className="finding-list">
          {report.fixes.applied.map((fix, index) => (
            <li className="finding" key={`${fix.time}-${fix.rule}-${index}`}>
              <p className="finding-title">
                {fix.time}: {fix.rule}
              </p>
              <p>{fix.summary}</p>
              <p className="hint">Where: {fix.place}</p>
              {fix.text ? <p>Text typed or chosen: {fix.text}</p> : null}
            </li>
          ))}
        </ul>
      )}

      {veraFailures.length > 0 ? (
        <section>
          <h3 className="section-heading">PDF/UA-1 rules that failed ({veraFailures.length})</h3>
          <ul className="finding-list">
            {veraFailures.map((failure) => (
              <li className="finding" key={`${failure.clause}-${failure.test_number}`}>
                <p className="finding-title">
                  {failure.clause}-{failure.test_number}: {failure.message}
                </p>
                {failure.place !== null ? <p>First place: {failure.place}.</p> : null}
                {failure.context !== null ? (
                  <details>
                    <summary>Technical detail</summary>
                    <p className="context">{failure.context}</p>
                  </details>
                ) : null}
                <p>
                  <strong>How to fix:</strong> {failure.how_to_fix}
                </p>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      {metadataIssues.length > 0 ? (
        <section>
          <h3 className="section-heading">Problems in the PDF ({metadataIssues.length})</h3>
          <ul className="finding-list">
            {metadataIssues.map((issue) => (
              <li className="finding" key={issue.message}>
                <p className="finding-title">{issue.message}</p>
                <p>
                  <strong>How to fix:</strong> {issue.how_to_fix}
                </p>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      {report.precheck.length > 0 ? (
        <section>
          <h3 className="section-heading">Findings in the Word document</h3>
          <FindingList findings={report.precheck} headingLevel={4} />
        </section>
      ) : null}

      <h3 className="section-heading">Checklist and sign-off</h3>
      <SignoffForm
        key={report.checklist.items.map((item) => (item.done ? "1" : "0")).join("") + (report.checklist.name ?? "") + (report.checklist.date ?? "") + (report.pac.result ?? "") + (report.pac.pac_version ?? "") + (report.pac.report_file ?? "")}
        items={report.checklist.items}
        name={report.checklist.name}
        date={report.checklist.date}
        pac={report.pac}
        onChoosePacReport={onChoosePacReport}
        busy={busy}
        onSave={onSaveSignoff}
      />
    </section>
  );
}
