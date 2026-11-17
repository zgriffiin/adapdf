import { InlineLoading, InlineNotification } from "@carbon/react";
import { useEffect, useRef, type ReactNode } from "react";

/**
 * The heading of a screen. It gets the keyboard focus when the screen shows and
 * each time `focusKey` changes (after an action), so a screen reader reads the
 * result first. It is not a control, so it is not in the Tab order (`tabIndex -1`).
 */
export function ResultsHeading({
  focusKey,
  id,
  children,
}: {
  focusKey: number;
  id?: string;
  children: ReactNode;
}) {
  const ref = useRef<HTMLHeadingElement>(null);
  useEffect(() => {
    ref.current?.focus();
  }, [focusKey]);
  return (
    <h2 ref={ref} id={id} tabIndex={-1} className="results-heading">
      {children}
    </h2>
  );
}

export interface Status {
  kind: "info" | "success" | "warning" | "error";
  title: string;
  subtitle?: string;
}

/**
 * The live region for messages. It is always in the page, so that a screen reader
 * hears a message that is put in it. The notification inside is a new element for
 * each message and is not a live region itself (`aria-live="off"`; Carbon allows
 * only the roles alert, log and status, so it has the role log), so that the
 * message is announced once, by this region.
 */
export function StatusRegion({ status, working }: { status: Status | null; working?: string }) {
  // A new message is a new element, so it is announced, also when its words are the same.
  const count = useRef(0);
  const last = useRef<Status | null>(null);
  if (status !== last.current) {
    last.current = status;
    count.current += 1;
  }
  return (
    <div className="status-region" role="status" aria-live="polite" aria-atomic="true">
      {working ? <InlineLoading description={working} status="active" /> : null}
      {status ? (
        <InlineNotification
          key={count.current}
          role="log"
          aria-live="off"
          kind={status.kind}
          title={status.title}
          subtitle={status.subtitle}
          hideCloseButton
          lowContrast
        />
      ) : null}
    </div>
  );
}
