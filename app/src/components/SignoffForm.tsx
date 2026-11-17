import { Button, Checkbox, TextInput } from "@carbon/react";
import { useRef, useState, type FormEvent } from "react";

import type { ChecklistItem, Signoff } from "../types";

interface Props {
  items: ChecklistItem[];
  /** What the saved report holds: empty for a report that is not signed. */
  name: string | null;
  date: string | null;
  busy: boolean;
  onSave: (signoff: Signoff) => void;
}

/**
 * The human checklist and the sign-off (test row A-08). The fields start with what
 * the report holds. The tool never fills in a name or a date.
 */
export function SignoffForm({ items, name, date, busy, onSave }: Props) {
  const [done, setDone] = useState<boolean[]>(() => items.map((item) => item.done));
  const [typedName, setTypedName] = useState(name ?? "");
  const [typedDate, setTypedDate] = useState(date ?? "");
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
    onSave({ name: typedName.trim(), date: typedDate, items: done });
  };

  const ticked = done.filter(Boolean).length;

  return (
    <form className="signoff" onSubmit={submit} noValidate aria-label="Sign-off">
      <p>
        Open the PDF in PAC and check each item below. Tick the items you have checked, then type
        your name and save. The checklist and your name are saved in the report. The PDF is not
        changed.
      </p>
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
