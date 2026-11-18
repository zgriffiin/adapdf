import { Button, Checkbox, Select, SelectItem, TextInput } from "@carbon/react";
import { useRef, useState, type FormEvent } from "react";

import type { ChecklistItem, PacResult, Report, Signoff } from "../types";
import { fileNameOf } from "../viewModel";

interface Props {
  items: ChecklistItem[];
  /** What the saved report holds: empty for a report that is not signed. */
  name: string | null;
  date: string | null;
  pac: Report["pac"];
  busy: boolean;
  /** Asks for the PAC report file (a file chooser). `null` when the person cancels. */
  onChoosePacReport: () => Promise<string | null>;
  onSave: (signoff: Signoff) => void;
}

const pacResultOf = (value: string | null): PacResult => (value === "pass" || value === "fail" ? value : "none");

/**
 * The PAC result, the human checklist and the sign-off (test row A-08). The fields
 * start with what the report holds. The tool never fills in a name or a date. The
 * report page has no form: it shows what is saved here as text.
 */
export function SignoffForm({ items, name, date, pac, busy, onChoosePacReport, onSave }: Props) {
  const [done, setDone] = useState<boolean[]>(() => items.map((item) => item.done));
  const [typedName, setTypedName] = useState(name ?? "");
  const [typedDate, setTypedDate] = useState(date ?? "");
  const [pacResult, setPacResult] = useState<PacResult>(pacResultOf(pac.result));
  const [pacVersion, setPacVersion] = useState(pac.pac_version ?? "");
  const [pacFile, setPacFile] = useState<string | null>(pac.report_file);
  const [nameMissing, setNameMissing] = useState(false);
  const nameField = useRef<HTMLInputElement>(null);

  const submit = (event: FormEvent) => {
    event.preventDefault();
    if (typedName.trim() === "") {
      setNameMissing(true);
      nameField.current?.focus();
      return;
    }
    setNameMissing(false);
    onSave({
      name: typedName.trim(),
      date: typedDate,
      items: done,
      pac: {
        result: pacResult,
        version: pacResult === "none" ? "" : pacVersion,
        report_file: pacResult === "none" ? null : pacFile,
      },
    });
  };

  const choosePacReport = async () => {
    const chosen = await onChoosePacReport();
    if (chosen !== null) {
      setPacFile(chosen);
    }
  };

  const ticked = done.filter(Boolean).length;

  return (
    <form className="signoff" onSubmit={submit} noValidate aria-label="Sign-off">
      <p>
        Open the PDF in PAC and check each item below. Record the PAC result, tick the items you
        have checked, then type your name and save. The PAC result, the checklist and your name are
        saved in the report. The PDF is not changed.
      </p>
      <fieldset className="cds--fieldset">
        <legend className="cds--label">PAC result</legend>
        <Select
          id="pac-result"
          labelText="PAC result"
          value={pacResult}
          onChange={(event) => setPacResult(pacResultOf(event.target.value))}
        >
          <SelectItem value="none" text="Not checked yet" />
          <SelectItem value="pass" text="Pass: PAC found 0 errors" />
          <SelectItem value="fail" text="Fail: PAC found errors" />
        </Select>
        {pacResult !== "none" ? (
          <>
            <TextInput
              id="pac-version"
              labelText="PAC version"
              value={pacVersion}
              onChange={(event) => setPacVersion(event.target.value)}
            />
            <div className="pac-file">
              <p>
                <strong>PAC report file:</strong> {pacFile === null ? "None chosen" : fileNameOf(pacFile)}
              </p>
              <div className="button-row">
                <Button kind="tertiary" size="sm" onClick={() => void choosePacReport()} disabled={busy}>
                  Choose the PAC report file
                </Button>
                {pacFile !== null ? (
                  <Button kind="ghost" size="sm" onClick={() => setPacFile(null)} disabled={busy}>
                    Remove the PAC report file
                  </Button>
                ) : null}
              </div>
              <p className="hint">The file must be in the folder of this guide, so that it stays with the report.</p>
            </div>
          </>
        ) : null}
      </fieldset>
      <fieldset className="cds--fieldset">
        <legend className="cds--label">Checks that a person makes</legend>
        {items.map((item, index) => (
          <Checkbox
            key={item.text}
            id={`check-${index + 1}`}
            labelText={item.text}
            checked={done[index] ?? false}
            onChange={(_, { checked }) =>
              setDone((current) => current.map((value, i) => (i === index ? checked : value)))
            }
          />
        ))}
        <p className="hint" aria-live="polite">
          {ticked} of {items.length} checks are ticked.
        </p>
      </fieldset>
      <TextInput
        id="signoff-name"
        ref={nameField}
        labelText="Your name"
        value={typedName}
        aria-invalid={nameMissing ? "true" : undefined}
        aria-describedby={nameMissing ? "signoff-name-error" : undefined}
        onChange={(event) => {
          setTypedName(event.target.value);
          setNameMissing(false);
        }}
      />
      {/* Carbon's own error state (`invalid`) points `aria-errormessage` at text that is not a live region, which axe-core reports. This message is an alert, and the field points at it. */}
      {nameMissing ? (
        <p id="signoff-name-error" role="alert" className="field-error">
          Type your name to sign off. The tool does not fill it in for you.
        </p>
      ) : null}
      <TextInput
        id="signoff-date"
        type="date"
        labelText="Date"
        helperText="Pick the date you sign. The tool does not fill it in for you."
        value={typedDate}
        onChange={(event) => setTypedDate(event.target.value)}
      />
      <Button type="submit" disabled={busy}>
        Save sign-off
      </Button>
    </form>
  );
}
