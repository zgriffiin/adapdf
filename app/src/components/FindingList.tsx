import { Tag } from "@carbon/react";

import type { Finding } from "../types";
import { groupBySeverity } from "../viewModel";

/** Where a finding is, in Word's words (the same words as `adapdf check` and the report). */
export function placeOf(finding: Finding): string {
  return finding.location.place;
}

function FindingItem({ finding }: { finding: Finding }) {
  return (
    <li className="finding">
      <p className="finding-title">
        <Tag type={finding.severity === "error" ? "red" : "gray"} size="sm">
          {finding.rule_id}
        </Tag>
        <span>
          <span className="cds--visually-hidden">
            {finding.severity === "error" ? "Error: " : "Warning: "}
          </span>
          {finding.message}
        </span>
      </p>
      <p>
        <strong>Where:</strong> {placeOf(finding)}
      </p>
      <p>
        <strong>How to fix:</strong> {finding.how_to_fix}
      </p>
      {finding.suggestion !== null ? (
        <p className="hint">
          <strong>Suggestion (not applied):</strong> {finding.suggestion}
        </p>
      ) : null}
    </li>
  );
}

function Group({
  title,
  note,
  findings,
  Heading,
}: {
  title: string;
  note: string;
  findings: Finding[];
  Heading: "h3" | "h4";
}) {
  if (findings.length === 0) {
    return null;
  }
  return (
    <section>
      <Heading className="section-heading">
        {title} ({findings.length})
      </Heading>
      <p className="hint">{note}</p>
      <ul className="finding-list">
        {findings.map((finding, index) => (
          <FindingItem key={`${finding.rule_id}-${index}`} finding={finding} />
        ))}
      </ul>
    </section>
  );
}

/** The findings, errors first, then warnings, each with its count. */
export function FindingList({
  findings,
  headingLevel = 3,
}: {
  findings: Finding[];
  /** The level of the headings of the two groups. */
  headingLevel?: 3 | 4;
}) {
  const { errors, warnings } = groupBySeverity(findings);
  const Heading = headingLevel === 3 ? "h3" : "h4";
  return (
    <>
      <Group
        title="Errors"
        note="These block the export. Fix them first."
        findings={errors}
        Heading={Heading}
      />
      <Group
        title="Warnings"
        note="These do not block the export."
        findings={warnings}
        Heading={Heading}
      />
    </>
  );
}
