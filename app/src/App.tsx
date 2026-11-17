import { Button } from "@carbon/react";
import { useCallback, useEffect, useRef, useState } from "react";

import {
  applyFixes,
  check,
  chooseDocument,
  chooseReport,
  exportPdf,
  fixOffers,
  messageOf,
  onFilesDropped,
  openReport,
  readReport,
  saveSignoff,
} from "./api";
import { FindingList } from "./components/FindingList";
import { FixFlow } from "./components/FixFlow";
import { ResultScreen } from "./components/ResultScreen";
import { ResultsHeading, StatusRegion, type Status } from "./components/common";
import type { CheckResult, Choice, OfferView, ReportView, Signoff } from "./types";
import { canExport, isReady, plural, summaryText } from "./viewModel";

type Screen =
  | { name: "empty" }
  | { name: "findings"; path: string; result: CheckResult; offers: OfferView[] }
  | { name: "fix"; path: string; result: CheckResult; offers: OfferView[] }
  | { name: "exporting"; path: string; result: CheckResult; offers: OfferView[] }
  | { name: "result"; view: ReportView; document: string | null };

const fileNameOf = (path: string) => path.split(/[\\/]/).pop() ?? path;

export function App() {
  const [screen, setScreen] = useState<Screen>({ name: "empty" });
  const [status, setStatus] = useState<Status | null>(null);
  const [busy, setBusy] = useState(false);
  // Counts up after each action, so that the heading of the screen gets the focus.
  const [focusKey, setFocusKey] = useState(0);
  const busyRef = useRef(false);

  const show = useCallback((next: Screen, message: Status | null) => {
    setScreen(next);
    setStatus(message);
    setFocusKey((key) => key + 1);
  }, []);

  /** Runs one action. Only one runs at a time. */
  const act = useCallback(async (work: () => Promise<void>) => {
    if (busyRef.current) {
      return;
    }
    busyRef.current = true;
    setBusy(true);
    try {
      await work();
    } finally {
      busyRef.current = false;
      setBusy(false);
    }
  }, []);

  /** Checks a document and asks which fixes there are for it. */
  const loadDocument = useCallback(
    (path: string, message?: Status) =>
      act(async () => {
        try {
          const result = await check(path);
          let offers: OfferView[] = [];
          let notice: Status | null = message ?? null;
          try {
            offers = await fixOffers(path);
          } catch (error) {
            notice = {
              kind: "warning",
              title: "The tool cannot offer fixes now.",
              subtitle: messageOf(error),
            };
          }
          show({ name: "findings", path, result, offers }, notice);
        } catch (error) {
          setStatus({
            kind: "error",
            title: `${fileNameOf(path)} could not be checked.`,
            subtitle: messageOf(error),
          });
          setFocusKey((key) => key + 1);
        }
      }),
    [act, show],
  );

  const loadReport = useCallback(
    (path: string) =>
      act(async () => {
        try {
          const view = await readReport(path);
          show({ name: "result", view, document: null }, null);
        } catch (error) {
          setStatus({
            kind: "error",
            title: "The report could not be opened.",
            subtitle: messageOf(error),
          });
          setFocusKey((key) => key + 1);
        }
      }),
    [act, show],
  );

  const openDropped = useCallback(
    (paths: string[]) => {
      const first = paths[0] ?? "";
      const lower = first.toLowerCase();
      if (lower.endsWith(".docx")) {
        void loadDocument(first);
      } else if (lower.endsWith(".accessibility.json")) {
        void loadReport(first);
      } else {
        setStatus({
          kind: "error",
          title: "That file cannot be used.",
          subtitle: "Drop a Word document (a file that ends with .docx), or choose one with the button.",
        });
        setFocusKey((key) => key + 1);
      }
    },
    [loadDocument, loadReport],
  );

  // Dropping a file on the window is the same as choosing it with the button.
  useEffect(() => {
    let stop: (() => void) | undefined;
    let cancelled = false;
    onFilesDropped(openDropped).then(
      (unlisten) => {
        if (cancelled) {
          unlisten();
        } else {
          stop = unlisten;
        }
      },
      () => undefined,
    );
    return () => {
      cancelled = true;
      stop?.();
    };
  }, [openDropped]);

  const choose = async () => {
    try {
      const path = await chooseDocument();
      if (path !== null) {
        await loadDocument(path);
      }
    } catch (error) {
      setStatus({ kind: "error", title: "The file chooser did not open.", subtitle: messageOf(error) });
    }
  };

  const chooseEarlierReport = async () => {
    try {
      const path = await chooseReport();
      if (path !== null) {
        await loadReport(path);
      }
    } catch (error) {
      setStatus({ kind: "error", title: "The file chooser did not open.", subtitle: messageOf(error) });
    }
  };

  const startFixing = () => {
    if (screen.name === "findings") {
      show({ ...screen, name: "fix" }, null);
    }
  };

  const apply = (choices: Choice[]) => {
    if (screen.name !== "fix") {
      return;
    }
    const { path } = screen;
    void act(async () => {
      try {
        const outcome = await applyFixes(path, choices);
        const result: CheckResult = {
          file: path,
          errors: outcome.errors,
          warnings: outcome.warnings,
          exit_code: outcome.exit_code,
          findings: outcome.findings,
        };
        let offers: OfferView[] = [];
        try {
          offers = await fixOffers(path);
        } catch {
          // The findings are shown without fixes; the person can try again.
        }
        const parts = [
          outcome.applied.length > 0
            ? `${plural(outcome.applied.length, "fix")} applied.`
            : "No fix was applied.",
          outcome.backup !== null ? `Backup: ${outcome.backup}.` : "No backup was made, because nothing was written.",
          outcome.refused.length > 0
            ? `${plural(outcome.refused.length, "fix")} not applied: ${outcome.refused.map((r) => r.reason).join(" ")}`
            : "",
        ].filter((part) => part !== "");
        show(
          { name: "findings", path, result, offers },
          {
            kind: outcome.refused.length > 0 ? "warning" : "success",
            title: outcome.applied.length > 0 ? "The fixes were applied." : "Nothing was changed.",
            subtitle: parts.join(" "),
          },
        );
      } catch (error) {
        setStatus({ kind: "error", title: "The fixes were not applied.", subtitle: messageOf(error) });
        setFocusKey((key) => key + 1);
      }
    });
  };

  const startExport = () => {
    if (screen.name !== "findings") {
      return;
    }
    const { path, result, offers } = screen;
    void act(async () => {
      show({ name: "exporting", path, result, offers }, null);
      try {
        const outcome = await exportPdf(path);
        show({ name: "result", view: outcome.view, document: path }, null);
      } catch (error) {
        show(
          { name: "findings", path, result, offers },
          { kind: "error", title: "The export did not finish.", subtitle: messageOf(error) },
        );
      }
    });
  };

  const showReport = (view: ReportView) => {
    void act(async () => {
      try {
        await openReport(view.report_html);
      } catch (error) {
        setStatus({ kind: "error", title: "The report did not open.", subtitle: messageOf(error) });
      }
      setFocusKey((key) => key + 1);
    });
  };

  const sign = (view: ReportView, signoff: Signoff) => {
    void act(async () => {
      try {
        const saved = await saveSignoff(view.report_json, signoff);
        setScreen((current) => (current.name === "result" ? { ...current, view: saved } : current));
        setStatus({
          kind: "success",
          title: "The sign-off was saved.",
          subtitle: "The name, the date and the checklist are in the report (the .html and the .json). The PDF was not changed.",
        });
      } catch (error) {
        setStatus({ kind: "error", title: "The sign-off was not saved.", subtitle: messageOf(error) });
      }
      setFocusKey((key) => key + 1);
    });
  };

  const backToFindings = (document: string) => void loadDocument(document);

  return (
    <div className="app-shell">
      <header>
        <h1>Adapdf</h1>
        <p className="hint">Check a Word document, fix what can be fixed, and make an accessible PDF.</p>
      </header>
      <main>
        {/* One live region for the whole app, in the page from the start, so that a screen reader hears each message that is put in it. */}
        <StatusRegion
          status={status}
          working={
            screen.name === "exporting"
              ? "Word is making the PDF, and then the PDF is checked. This can take a few minutes."
              : undefined
          }
        />
        {screen.name === "empty" ? (
          <section aria-labelledby="start-heading">
            <ResultsHeading focusKey={focusKey} id="start-heading">
              Choose a Word document
            </ResultsHeading>
            <div className="drop-area">
              <p>Choose a Word document (.docx) to check. You can also drop one on this window.</p>
              <div className="button-row centered">
                <Button onClick={() => void choose()} disabled={busy}>
                  Choose a Word document
                </Button>
                <Button kind="tertiary" onClick={() => void chooseEarlierReport()} disabled={busy}>
                  Open an earlier report
                </Button>
              </div>
            </div>
            <p className="hint">
              Everything stays on this computer. Nothing is sent over the network.
            </p>
          </section>
        ) : null}

        {screen.name === "findings" ? (
          <section aria-labelledby="findings-heading">
            <ResultsHeading focusKey={focusKey} id="findings-heading">
              {fileNameOf(screen.path)}: {summaryText(screen.result.findings)}
            </ResultsHeading>
            <div className="button-row">
              {screen.offers.some(isReady) ? (
                <Button onClick={startFixing} disabled={busy}>
                  Fix problems
                </Button>
              ) : null}
              <Button
                kind={screen.offers.some(isReady) ? "secondary" : "primary"}
                onClick={startExport}
                disabled={!canExport(screen.result.errors, busy)}
              >
                Export PDF
              </Button>
              <Button kind="tertiary" onClick={() => void loadDocument(screen.path)} disabled={busy}>
                Check again
              </Button>
              <Button kind="ghost" onClick={() => show({ name: "empty" }, null)} disabled={busy}>
                Choose another file
              </Button>
            </div>
            {screen.result.errors > 0 ? (
              <p className="hint">
                Export is off while errors remain. Fix the errors, in this app or in Word, then check
                again.
              </p>
            ) : null}
            <FindingList findings={screen.result.findings} />
          </section>
        ) : null}

        {screen.name === "fix" ? (
          <>
            <FixFlow
              fileName={fileNameOf(screen.path)}
              offers={screen.offers}
              busy={busy}
              focusKey={focusKey}
              onApply={apply}
              onCancel={() => show({ ...screen, name: "findings" }, null)}
            />
          </>
        ) : null}

        {screen.name === "exporting" ? (
          <section aria-labelledby="exporting-heading" aria-busy="true">
            <ResultsHeading focusKey={focusKey} id="exporting-heading">
              Exporting {fileNameOf(screen.path)}
            </ResultsHeading>
            <p className="hint">Please wait. Do not close this window.</p>
          </section>
        ) : null}

        {screen.name === "result" ? (
          <>
            <ResultScreen
              view={screen.view}
              focusKey={focusKey}
              busy={busy}
              canGoBack={screen.document !== null}
              onOpenReport={() => showReport(screen.view)}
              onSaveSignoff={(signoff) => sign(screen.view, signoff)}
              onBack={() => screen.document !== null && backToFindings(screen.document)}
              onStartOver={() => show({ name: "empty" }, null)}
            />
          </>
        ) : null}
      </main>
    </div>
  );
}
