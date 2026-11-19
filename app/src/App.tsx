import { Button } from "@carbon/react";
import { useCallback, useEffect, useRef, useState } from "react";

import {
  applyFixes,
  check,
  chooseDocument,
  choosePacReport,
  chooseReport,
  createCopy,
  exportPdf,
  fixOffers,
  messageOf,
  onFilesDropped,
  openFolder,
  openInWord,
  openReport,
  readReport,
  saveSignoff,
  workingInfo,
} from "./api";
import { FindingList } from "./components/FindingList";
import { FixFlow } from "./components/FixFlow";
import { ResultScreen } from "./components/ResultScreen";
import { ResultsHeading, StatusRegion, WorkingOn, type Status } from "./components/common";
import type { CheckResult, Choice, OfferView, ReportView, Signoff, WorkingInfo } from "./types";
import { canExport, fileNameOf, folderOf, isReady, plural, summaryText } from "./viewModel";

/** A document that is checked: the file, its files, and what was found. */
interface Doc {
  /** The file that the app works on: the accessible copy, or the original before a copy is made. */
  path: string;
  info: WorkingInfo;
  /** The person chose "Start over": the copy that exists is replaced when the next write is made. */
  replace: boolean;
  result: CheckResult;
  offers: OfferView[];
}

type Screen =
  | { name: "empty" }
  | { name: "copy-choice"; info: WorkingInfo; confirming: boolean }
  | ({ name: "findings"; confirmCopy: boolean } & Doc)
  | ({ name: "fix" } & Doc)
  | ({ name: "exporting" } & Doc)
  | { name: "result"; view: ReportView; document: string | null };

/** The document part of a screen that has one. */
function docOf(screen: Doc): Doc {
  const { path, info, replace, result, offers } = screen;
  return { path, info, replace, result, offers };
}

const WORD_MESSAGE: Status = {
  kind: "info",
  title: "The document is open in Word.",
  subtitle: "Make your changes in Word, save, then choose Check again.",
};

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

  const fail = useCallback((title: string, error: unknown) => {
    setStatus({ kind: "error", title, subtitle: messageOf(error) });
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

  /**
   * Checks a document and asks which fixes there are for it, then shows it. Used
   * inside an action. `info` is what the app already knows about the files.
   */
  const load = useCallback(
    async (path: string, replace: boolean, message?: Status, known?: WorkingInfo) => {
      try {
        const info = known ?? (await workingInfo(path));
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
        show({ name: "findings", path, info, replace, result, offers, confirmCopy: false }, notice);
      } catch (error) {
        fail(`${fileNameOf(path)} could not be checked.`, error);
      }
    },
    [fail, show],
  );

  /** A file was chosen or dropped: ask about the copy when there is one, or check the file. */
  const openChosen = useCallback(
    (path: string) =>
      act(async () => {
        try {
          const info = await workingInfo(path);
          if (info.copy_exists && !info.is_copy) {
            show({ name: "copy-choice", info, confirming: false }, null);
            return;
          }
          await load(path, false, undefined, info);
        } catch (error) {
          fail(`${fileNameOf(path)} could not be checked.`, error);
        }
      }),
    [act, fail, load, show],
  );

  const loadReport = useCallback(
    (path: string) =>
      act(async () => {
        try {
          const view = await readReport(path);
          show({ name: "result", view, document: null }, null);
        } catch (error) {
          fail("The report could not be opened.", error);
        }
      }),
    [act, fail, show],
  );

  const openDropped = useCallback(
    (paths: string[]) => {
      const first = paths[0] ?? "";
      const lower = first.toLowerCase();
      if (lower.endsWith(".docx")) {
        void openChosen(first);
      } else if (/[\\/]report\.json$/.test(lower)) {
        void loadReport(first);
      } else {
        fail(
          "That file cannot be used.",
          "Drop a Word document (a file that ends with .docx), or choose one with the button.",
        );
      }
    },
    [fail, loadReport, openChosen],
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
        await openChosen(path);
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
      show({ ...docOf(screen), name: "fix" }, null);
    }
  };

  const apply = (choices: Choice[]) => {
    if (screen.name !== "fix") {
      return;
    }
    const { path, replace } = screen;
    void act(async () => {
      try {
        const outcome = await applyFixes(path, choices, replace);
        const working = outcome.working_file;
        const result: CheckResult = {
          file: working,
          errors: outcome.errors,
          warnings: outcome.warnings,
          exit_code: outcome.exit_code,
          findings: outcome.findings,
        };
        let info: WorkingInfo;
        try {
          info = await workingInfo(working);
        } catch (error) {
          fail("The fixes were applied, but the files could not be read again.", error);
          return;
        }
        let offers: OfferView[] = [];
        try {
          offers = await fixOffers(working);
        } catch {
          // The findings are shown without fixes; the person can try again.
        }
        const parts = [
          outcome.applied.length > 0
            ? `${plural(outcome.applied.length, "fix")} applied.`
            : "No fix was applied.",
          outcome.copy !== null
            ? `Saved your fixes to ${fileNameOf(outcome.copy)}. Your original ${fileNameOf(outcome.original)} was not changed.`
            : "No file was changed, because nothing was written.",
          outcome.refused.length > 0
            ? `${plural(outcome.refused.length, "fix")} not applied: ${outcome.refused.map((r) => r.reason).join(" ")}`
            : "",
        ].filter((part) => part !== "");
        show(
          { name: "findings", path: working, info, replace: false, result, offers, confirmCopy: false },
          {
            kind: outcome.refused.length > 0 ? "warning" : "success",
            title: outcome.applied.length > 0 ? "The fixes were applied." : "Nothing was changed.",
            subtitle: parts.join(" "),
          },
        );
      } catch (error) {
        fail("The fixes were not applied.", error);
      }
    });
  };

  const startExport = () => {
    if (screen.name !== "findings") {
      return;
    }
    const doc = docOf(screen);
    void act(async () => {
      show({ ...doc, name: "exporting" }, null);
      try {
        const outcome = await exportPdf(doc.path);
        show({ name: "result", view: outcome.view, document: doc.path }, null);
      } catch (error) {
        show(
          { ...doc, name: "findings", confirmCopy: false },
          { kind: "error", title: "The export did not finish.", subtitle: messageOf(error) },
        );
      }
    });
  };

  /** "Open in Word": the copy opens in the person's own Word. The original is never opened. */
  const startOpenInWord = () => {
    if (screen.name !== "findings") {
      return;
    }
    if (!screen.info.is_copy) {
      // Word must open the copy, not the original: ask to make the copy first.
      setScreen({ ...screen, confirmCopy: true });
      setFocusKey((key) => key + 1);
      return;
    }
    const { path } = screen;
    void act(async () => {
      try {
        await openInWord(path);
        setStatus(WORD_MESSAGE);
      } catch (error) {
        setStatus({ kind: "error", title: "The document did not open in Word.", subtitle: messageOf(error) });
      }
    });
  };

  const makeCopyAndOpen = () => {
    if (screen.name !== "findings") {
      return;
    }
    const { path, replace } = screen;
    void act(async () => {
      try {
        const copy = await createCopy(path, replace);
        await load(copy.chosen, false, undefined, copy);
        try {
          await openInWord(copy.chosen);
          setStatus({
            ...WORD_MESSAGE,
            subtitle: `${fileNameOf(copy.chosen)} is made. ${WORD_MESSAGE.subtitle}`,
          });
        } catch (error) {
          setStatus({ kind: "error", title: "The document did not open in Word.", subtitle: messageOf(error) });
        }
      } catch (error) {
        fail("The accessible copy was not made.", error);
      }
    });
  };

  const openFolderOf = (path: string) => {
    void act(async () => {
      try {
        await openFolder(path);
      } catch (error) {
        setStatus({ kind: "error", title: "The folder did not open.", subtitle: messageOf(error) });
      }
      setFocusKey((key) => key + 1);
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
          subtitle: "The PAC result, the name, the date and the checklist are in the report (the .html and the .json). The PDF was not changed.",
        });
      } catch (error) {
        setStatus({ kind: "error", title: "The sign-off was not saved as asked.", subtitle: messageOf(error) });
      }
      setFocusKey((key) => key + 1);
    });
  };

  const choosePac = async (folder: string): Promise<string | null> => {
    try {
      return await choosePacReport(folder);
    } catch (error) {
      setStatus({ kind: "error", title: "The file chooser did not open.", subtitle: messageOf(error) });
      return null;
    }
  };

  const continueWithCopy = (copy: string) => void act(() => load(copy, false));
  const startOver = (info: WorkingInfo) =>
    void act(() =>
      load(info.original, true, {
        kind: "info",
        title: "Starting over from the original.",
        subtitle: `${fileNameOf(info.copy)} is replaced when you apply a fix or open the document in Word. Until then it is as it was.`,
      }, info),
    );

  const backToFindings = (document: string) => void act(() => load(document, false));

  const copyName = (info: WorkingInfo) => fileNameOf(info.copy);

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
              Everything stays on this computer. Nothing is sent over the network. Your original document is
              never changed.
            </p>
          </section>
        ) : null}

        {screen.name === "copy-choice" ? (
          <section aria-labelledby="choice-heading">
            <ResultsHeading focusKey={focusKey} id="choice-heading">
              {fileNameOf(screen.info.original)} has an accessible copy
            </ResultsHeading>
            <WorkingOn
              path={screen.info.chosen}
              isCopy={false}
              busy={busy}
              onOpenFolder={() => openFolderOf(screen.info.chosen)}
            />
            <p>
              Adapdf made {copyName(screen.info)} from this document earlier, in the folder{" "}
              {fileNameOf(screen.info.folder)}. Your original is not changed.
            </p>
            {screen.confirming ? (
              <div className="copy-choice">
                <p>
                  <strong>Start over from the original?</strong> This replaces the accessible copy, including
                  any changes you made to it in Word.
                </p>
                <div className="button-row">
                  <Button kind="danger" onClick={() => startOver(screen.info)} disabled={busy}>
                    Yes, replace the accessible copy
                  </Button>
                  <Button
                    kind="secondary"
                    onClick={() => show({ ...screen, confirming: false }, null)}
                    disabled={busy}
                  >
                    No, keep the accessible copy
                  </Button>
                </div>
              </div>
            ) : (
              <div className="button-row">
                <Button onClick={() => continueWithCopy(screen.info.copy)} disabled={busy}>
                  Continue with the accessible copy
                </Button>
                <Button
                  kind="secondary"
                  onClick={() => show({ ...screen, confirming: true }, null)}
                  disabled={busy}
                >
                  Start over from the original
                </Button>
                <Button kind="ghost" onClick={() => show({ name: "empty" }, null)} disabled={busy}>
                  Choose another file
                </Button>
              </div>
            )}
          </section>
        ) : null}

        {screen.name === "findings" ? (
          <section aria-labelledby="findings-heading">
            <ResultsHeading focusKey={focusKey} id="findings-heading">
              {fileNameOf(screen.path)}: {summaryText(screen.result.findings)}
            </ResultsHeading>
            <WorkingOn
              path={screen.path}
              isCopy={screen.info.is_copy}
              busy={busy}
              onOpenFolder={() => openFolderOf(screen.path)}
            />
            {screen.info.is_copy ? null : (
              <p className="hint">
                Your original is never changed. Fixes are saved in a copy, {copyName(screen.info)}, in the
                folder {fileNameOf(screen.info.folder)}.
                {screen.replace ? " The copy that is there is replaced when the new copy is made." : ""}
              </p>
            )}
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
              <Button kind="secondary" onClick={startOpenInWord} disabled={busy}>
                Open in Word
              </Button>
              <Button kind="tertiary" onClick={() => void act(() => load(screen.path, screen.replace))} disabled={busy}>
                Check again
              </Button>
              <Button kind="ghost" onClick={() => show({ name: "empty" }, null)} disabled={busy}>
                Choose another file
              </Button>
            </div>
            {screen.confirmCopy ? (
              <div className="copy-choice">
                <p>
                  Word opens the accessible copy, not your original, so that your original is not changed. Adapdf
                  makes {copyName(screen.info)} now, in the folder {fileNameOf(screen.info.folder)}.
                  {screen.replace
                    ? " This replaces the accessible copy that is there, including any changes you made to it in Word."
                    : ""}
                </p>
                <div className="button-row">
                  <Button onClick={makeCopyAndOpen} disabled={busy}>
                    Make the copy and open it in Word
                  </Button>
                  <Button
                    kind="secondary"
                    onClick={() => show({ ...screen, confirmCopy: false }, null)}
                    disabled={busy}
                  >
                    Cancel
                  </Button>
                </div>
              </div>
            ) : null}
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
          <FixFlow
            fileName={fileNameOf(screen.path)}
            copyName={copyName(screen.info)}
            workingOn={
              <WorkingOn
                path={screen.path}
                isCopy={screen.info.is_copy}
                busy={busy}
                onOpenFolder={() => openFolderOf(screen.path)}
              />
            }
            offers={screen.offers}
            busy={busy}
            focusKey={focusKey}
            onApply={apply}
            onCancel={() => show({ ...screen, name: "findings", confirmCopy: false }, null)}
          />
        ) : null}

        {screen.name === "exporting" ? (
          <section aria-labelledby="exporting-heading" aria-busy="true">
            <ResultsHeading focusKey={focusKey} id="exporting-heading">
              Exporting {fileNameOf(screen.path)}
            </ResultsHeading>
            <WorkingOn
              path={screen.path}
              isCopy={screen.info.is_copy}
              busy={true}
              onOpenFolder={() => openFolderOf(screen.path)}
            />
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
              onOpenFolder={() => openFolderOf(screen.view.report_json)}
              onChoosePacReport={() => choosePac(folderOf(screen.view.report_json))}
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
