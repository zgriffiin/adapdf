import { Button, Checkbox, TextInput } from "@carbon/react";
import { useState } from "react";

import type { Choice, OfferView } from "../types";
import {
  canApply,
  choicesFrom,
  emptySelection,
  isReady,
  offerKey,
  plural,
  type Selection,
} from "../viewModel";
import { placeOf } from "./FindingList";
import { ResultsHeading } from "./common";

interface Props {
  fileName: string;
  offers: OfferView[];
  busy: boolean;
  focusKey: number;
  onApply: (choices: Choice[]) => void;
  onCancel: () => void;
}

function toggled(set: ReadonlySet<string>, key: string, on: boolean): Set<string> {
  const next = new Set(set);
  if (on) {
    next.add(key);
  } else {
    next.delete(key);
  }
  return next;
}

/** The suggestion is a hint under the field. It is never the value of the field. */
function suggestionHint(offer: OfferView, what: string): string {
  const suggestion = offer.finding.suggestion;
  return suggestion !== null
    ? `Suggestion, not filled in: ${suggestion}. Type ${what} yourself.`
    : `Type ${what} yourself. The tool does not make it up.`;
}

function OfferHeader({ offer }: { offer: OfferView }) {
  return (
    <>
      <p>
        <strong>{offer.finding.rule_id}:</strong> {offer.finding.message}
      </p>
      <p className="hint">Where: {placeOf(offer.finding)}</p>
    </>
  );
}

/**
 * The fix screen: the fixes that the tool can make. An automatic fix is a check
 * box. A fix that needs the person's words is a labelled text field, empty until
 * the person types, with "mark as decorative" for pictures.
 */
export function FixFlow({ fileName, offers, busy, focusKey, onApply, onCancel }: Props) {
  const [selection, setSelection] = useState<Selection>(emptySelection);
  const ready = offers.filter(isReady);
  const auto = ready.filter((o) => !o.ask);
  const ask = ready.filter((o) => o.ask);
  const atExport = offers.filter((o) => o.availability.status === "at-export");
  const unavailable = offers.filter((o) => o.availability.status === "unavailable");
  const chosen = choicesFrom(offers, selection).length;
  const allAutoTicked = auto.length > 0 && auto.every((o) => selection.ticked.has(offerKey(o)));

  const tick = (key: string, on: boolean) =>
    setSelection((s) => ({ ...s, ticked: toggled(s.ticked, key, on) }));
  const type = (key: string, text: string) =>
    setSelection((s) => ({ ...s, texts: { ...s.texts, [key]: text } }));
  const decorate = (key: string, on: boolean) =>
    setSelection((s) => ({ ...s, decorative: toggled(s.decorative, key, on) }));
  const tickAll = (on: boolean) =>
    setSelection((s) => ({
      ...s,
      ticked: on
        ? new Set([...s.ticked, ...auto.map(offerKey)])
        : new Set([...s.ticked].filter((key) => !auto.some((o) => offerKey(o) === key))),
    }));

  return (
    <section aria-labelledby="fix-heading">
      <ResultsHeading focusKey={focusKey} id="fix-heading">
        Fix problems in {fileName}
      </ResultsHeading>
      <p>
        Choose the fixes to make. Before it changes anything, the tool saves a backup copy of your
        document next to it.
      </p>

      {auto.length > 0 ? (
        <fieldset className="cds--fieldset">
          <legend className="cds--label">
            Fixes the tool can make for you ({auto.length})
          </legend>
          <div className="button-row">
            <Button kind="tertiary" size="sm" onClick={() => tickAll(!allAutoTicked)}>
              {allAutoTicked ? "Clear all automatic fixes" : "Select all automatic fixes"}
            </Button>
          </div>
          <ul className="offer-list">
            {auto.map((offer) => {
              const key = offerKey(offer);
              return (
                <li className="offer" key={key}>
                  <Checkbox
                    id={`tick-${key}`}
                    labelText={`${offer.finding.rule_id}: ${offer.description}`}
                    helperText={`${offer.finding.message} Where: ${placeOf(offer.finding)}`}
                    checked={selection.ticked.has(key)}
                    onChange={(_, { checked }) => tick(key, checked)}
                  />
                </li>
              );
            })}
          </ul>
        </fieldset>
      ) : null}

      {ask.length > 0 ? (
        <fieldset className="cds--fieldset">
          <legend className="cds--label">Fixes that need your words ({ask.length})</legend>
          <ul className="offer-list">
            {ask.map((offer) => {
              const key = offerKey(offer);
              if (offer.fix === "mark-annotations-decorative") {
                return (
                  <li className="offer" key={key}>
                    <OfferHeader offer={offer} />
                    <Checkbox
                      id={`tick-${key}`}
                      labelText="Yes, the alt text of the screenshot describes what the arrows and boxes show. Mark the arrows and boxes as decorative."
                      checked={selection.ticked.has(key)}
                      onChange={(_, { checked }) => tick(key, checked)}
                    />
                  </li>
                );
              }
              const isTitle = offer.fix === "set-title";
              const decorative = selection.decorative.has(key);
              const what = isTitle ? "the title" : "the alt text";
              return (
                <li className="offer" key={key}>
                  <OfferHeader offer={offer} />
                  <TextInput
                    id={`text-${key}`}
                    labelText={isTitle ? "Document title" : `Alt text for ${offer.finding.rule_id} (${placeOf(offer.finding)})`}
                    helperText={suggestionHint(offer, what)}
                    value={selection.texts[key] ?? ""}
                    disabled={decorative}
                    onChange={(event) => type(key, event.target.value)}
                  />
                  {isTitle ? null : (
                    <Checkbox
                      id={`decorative-${key}`}
                      labelText="Mark as decorative (it adds no information)"
                      checked={decorative}
                      onChange={(_, { checked }) => decorate(key, checked)}
                    />
                  )}
                </li>
              );
            })}
          </ul>
        </fieldset>
      ) : null}

      {atExport.length > 0 ? (
        <section>
          <h3 className="section-heading">Done by Word when the PDF is exported ({atExport.length})</h3>
          <ul className="offer-list">
            {atExport.map((offer) => (
              <li className="offer" key={offerKey(offer)}>
                <OfferHeader offer={offer} />
                <p className="hint">Word does this when the PDF is exported. There is nothing to choose.</p>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      {unavailable.length > 0 ? (
        <section>
          <h3 className="section-heading">The tool cannot fix these now ({unavailable.length})</h3>
          <ul className="offer-list">
            {unavailable.map((offer) => (
              <li className="offer" key={offerKey(offer)}>
                <OfferHeader offer={offer} />
                <p>
                  {offer.availability.status === "unavailable" ? offer.availability.reason : ""}
                </p>
              </li>
            ))}
          </ul>
        </section>
      ) : null}

      <div className="button-row">
        <Button onClick={() => onApply(choicesFrom(offers, selection))} disabled={!canApply(offers, selection, busy)}>
          {chosen === 0 ? "Apply fixes" : `Apply ${plural(chosen, "fix")}`}
        </Button>
        <Button kind="secondary" onClick={onCancel} disabled={busy}>
          Back to the findings
        </Button>
      </div>
      {chosen === 0 ? <p className="hint">Choose at least one fix to apply.</p> : null}
    </section>
  );
}
