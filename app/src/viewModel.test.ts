// Rows A-01, A-02 and A-03 of the test plan, on the view model (no page).
import { describe, expect, it } from "vitest";

import { doc001, img001, img007, lst002, offerOf, txb004 } from "./test/backend";
import {
  applyHint,
  canApply,
  canExport,
  choicesFrom,
  emptySelection,
  fileNameOf,
  folderOf,
  groupBySeverity,
  precheckText,
  offerKey,
  summaryText,
} from "./viewModel";

describe("app_groups_findings_by_severity (A-01)", () => {
  // The findings of LST-002-fail.docx, with the warning first, to show that the
  // view model puts the error first.
  const findings = [lst002.lst002, lst002.lst001];

  it("puts errors first and keeps the counts", () => {
    const { errors, warnings } = groupBySeverity(findings);
    expect(errors.map((f) => f.rule_id)).toEqual(["LST-001"]);
    expect(warnings.map((f) => f.rule_id)).toEqual(["LST-002"]);
    expect(summaryText(findings)).toBe("Found 1 error and 1 warning.");
  });

  it("says so when there is nothing", () => {
    expect(summaryText([])).toBe("No problems found.");
    expect(groupBySeverity([])).toEqual({ errors: [], warnings: [] });
  });

  it("uses the plural", () => {
    expect(summaryText([doc001, img001, txb004, lst002.lst002])).toBe("Found 2 errors and 2 warnings.");
  });
});

describe("app_buttons_follow_the_state (A-02)", () => {
  const offers = [offerOf(lst002.lst001)];

  it("turns Apply fixes off until a fix is picked", () => {
    expect(canApply(offers, emptySelection, false)).toBe(false);
    const picked = { ...emptySelection, ticked: new Set([offerKey(offers[0]!)]) };
    expect(canApply(offers, picked, false)).toBe(true);
    expect(canApply(offers, picked, true)).toBe(false);
  });

  it("turns Export off while errors remain, or while the app is busy", () => {
    expect(canExport(1, false)).toBe(false);
    expect(canExport(0, false)).toBe(true);
    expect(canExport(0, true)).toBe(false);
  });
});

describe("app_never_prefills_typed_text (A-03)", () => {
  const title = offerOf(doc001);
  const alt = offerOf(img001);

  it("makes no choice from a suggestion", () => {
    // The offer has texts from the document, and the person has typed or chosen nothing.
    expect(title.suggestions.length).toBeGreaterThan(0);
    expect(choicesFrom([title, alt], emptySelection)).toEqual([]);
    expect(canApply([title, alt], emptySelection, false)).toBe(false);
  });

  it("sends only what the person typed, without the spaces around it", () => {
    const selection = {
      ...emptySelection,
      texts: { [offerKey(title)]: "  My title  ", [offerKey(alt)]: "   " },
    };
    expect(choicesFrom([title, alt], selection)).toEqual([
      { rule_id: "DOC-001", position: 0, input: { kind: "text", text: "My title" } },
    ]);
  });

  it("marks a picture as decorative only when the person says so", () => {
    const selection = { ...emptySelection, decorative: new Set([offerKey(alt)]) };
    expect(choicesFrom([alt], selection)).toEqual([
      { rule_id: "IMG-001", position: 0, input: { kind: "decorative" } },
    ]);
  });

  it("needs a tick for the question about arrows and boxes", () => {
    const question = offerOf(img007);
    expect(choicesFrom([question], emptySelection)).toEqual([]);
    const yes = { ...emptySelection, ticked: new Set([offerKey(question)]) };
    expect(choicesFrom([question], yes)).toEqual([
      { rule_id: "IMG-007", position: 0, input: { kind: "confirm" } },
    ]);
  });

  it("leaves out a fix that is not ready", () => {
    const blocked = offerOf(lst002.lst001, {
      availability: { status: "unavailable", reason: "Not now." },
    });
    const selection = { ...emptySelection, ticked: new Set([offerKey(blocked)]) };
    expect(choicesFrom([blocked], selection)).toEqual([]);
  });
});

describe("what the screen says while Apply is off (D)", () => {
  it("says nothing when a fix is chosen", () => {
    expect(applyHint([offerOf(lst002.lst001)], 1)).toBeNull();
  });

  it("says to choose a fix when there is no title to give", () => {
    expect(applyHint([offerOf(lst002.lst001)], 0)).toBe("Choose at least one fix to apply.");
    expect(applyHint([], 0)).toBe("Choose at least one fix to apply.");
  });

  it("names the title when it is the only fix", () => {
    expect(applyHint([offerOf(doc001)], 0)).toBe("Type a title, or choose a suggestion, to apply this fix.");
  });

  it("names both when the title and other fixes are there", () => {
    expect(applyHint([offerOf(doc001), offerOf(lst002.lst001)], 0)).toBe(
      "Choose at least one fix to apply. For the title, type it, or choose a suggestion.",
    );
  });

  it("leaves out a title that cannot be fixed now", () => {
    const blocked = offerOf(doc001, { availability: { status: "unavailable", reason: "Not now." } });
    expect(applyHint([blocked], 0)).toBe("Choose at least one fix to apply.");
  });
});

describe("the names of files and folders", () => {
  it("are cut from a Windows path", () => {
    expect(fileNameOf("C:\\Docs\\Guide - Adapdf\\accessible.docx")).toBe("accessible.docx");
    expect(folderOf("C:\\Docs\\Guide - Adapdf\\accessible.docx")).toBe("C:\\Docs\\Guide - Adapdf");
    expect(fileNameOf("Guide.docx")).toBe("Guide.docx");
    expect(folderOf("Guide.docx")).toBe("");
  });

  it("say the pre-check counts with the plural", () => {
    expect(precheckText(1, 0)).toBe("1 error, 0 warnings");
    expect(precheckText(2, 1)).toBe("2 errors, 1 warning");
  });
});
