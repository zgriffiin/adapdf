// What the screens decide from the data, without any markup, so that it can be
// tested on its own (rows A-01 and A-02).
import type { Choice, Finding, OfferView, Verdict } from "./types";

export interface Grouped {
  errors: Finding[];
  warnings: Finding[];
}

/** Errors first, then warnings. The order inside a group is the order of the check. */
export function groupBySeverity(findings: readonly Finding[]): Grouped {
  return {
    errors: findings.filter((f) => f.severity === "error"),
    warnings: findings.filter((f) => f.severity === "warning"),
  };
}

export function plural(count: number, word: string): string {
  const many = /(x|s|ch|sh)$/.test(word) ? `${word}es` : `${word}s`;
  return `${count} ${count === 1 ? word : many}`;
}

/** "Found 2 errors and 1 warning." or "No problems found." */
export function summaryText(findings: readonly Finding[]): string {
  if (findings.length === 0) {
    return "No problems found.";
  }
  const { errors, warnings } = groupBySeverity(findings);
  return `Found ${plural(errors.length, "error")} and ${plural(warnings.length, "warning")}.`;
}

/** The export is not offered while an error remains: errors block it (spec 02, stage 1). */
export function canExport(errorCount: number, busy: boolean): boolean {
  return errorCount === 0 && !busy;
}

/** What a person has done in the fix screen. Keys come from {@link offerKey}. */
export interface Selection {
  /** The auto fixes (and the annotation question) that are ticked. */
  ticked: ReadonlySet<string>;
  /** The text typed for each ask fix. Nothing is in it until the person types. */
  texts: Readonly<Record<string, string>>;
  /** The alt text fixes that are marked decorative. */
  decorative: ReadonlySet<string>;
}

export const emptySelection: Selection = {
  ticked: new Set(),
  texts: {},
  decorative: new Set(),
};

export function offerKey(offer: OfferView): string {
  return `${offer.finding.rule_id}#${offer.position}`;
}

export function isReady(offer: OfferView): boolean {
  return offer.availability.status === "ready";
}

/** The fixes that the person has chosen, as the command takes them. */
export function choicesFrom(offers: readonly OfferView[], selection: Selection): Choice[] {
  const choices: Choice[] = [];
  for (const offer of offers) {
    if (!isReady(offer)) {
      continue;
    }
    const key = offerKey(offer);
    const base = { rule_id: offer.finding.rule_id, position: offer.position };
    if (offer.fix === "set-title") {
      const text = (selection.texts[key] ?? "").trim();
      if (text !== "") {
        choices.push({ ...base, input: { kind: "text", text } });
      }
    } else if (offer.fix === "set-alt-text") {
      const text = (selection.texts[key] ?? "").trim();
      if (selection.decorative.has(key)) {
        choices.push({ ...base, input: { kind: "decorative" } });
      } else if (text !== "") {
        choices.push({ ...base, input: { kind: "text", text } });
      }
    } else if (selection.ticked.has(key)) {
      choices.push({ ...base, input: { kind: "confirm" } });
    }
  }
  return choices;
}

/** "Apply fixes" is off until at least one fix is chosen. */
export function canApply(offers: readonly OfferView[], selection: Selection, busy: boolean): boolean {
  return !busy && choicesFrom(offers, selection).length > 0;
}

export function verdictWord(verdict: Verdict): "Pass" | "Fail" {
  return verdict === "pass" ? "Pass" : "Fail";
}
