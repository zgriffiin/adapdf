// The only module that talks to Tauri. The commands are in app/src-tauri/src/shell.rs.
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";

import type {
  ApplyResult,
  CheckResult,
  Choice,
  ExportResult,
  OfferView,
  ReportView,
  Signoff,
} from "./types";

export const check = (path: string) => invoke<CheckResult>("check", { path });
export const fixOffers = (path: string) => invoke<OfferView[]>("fix_offers", { path });
export const applyFixes = (path: string, choices: Choice[]) =>
  invoke<ApplyResult>("apply_fixes", { path, choices });
export const exportPdf = (path: string) => invoke<ExportResult>("export", { path });
export const openReport = (path: string) => invoke<void>("open_report", { path });
export const readReport = (path: string) => invoke<ReportView>("read_report", { path });
export const saveSignoff = (reportJson: string, signoff: Signoff) =>
  invoke<ReportView>("save_signoff", { reportJson, signoff });

/** Asks for a .docx file. `null` when the person cancels. */
export async function chooseDocument(): Promise<string | null> {
  const chosen = await open({
    multiple: false,
    directory: false,
    title: "Choose a Word document",
    filters: [{ name: "Word documents", extensions: ["docx"] }],
  });
  return typeof chosen === "string" ? chosen : null;
}

/** Asks for the JSON of an earlier report. `null` when the person cancels. */
export async function chooseReport(): Promise<string | null> {
  const chosen = await open({
    multiple: false,
    directory: false,
    title: "Choose an accessibility report",
    filters: [{ name: "Accessibility reports", extensions: ["json"] }],
  });
  return typeof chosen === "string" ? chosen : null;
}

/**
 * Calls `onDrop` with the paths of the files that are dropped on the window.
 * Returns a function that stops listening.
 */
export async function onFilesDropped(onDrop: (paths: string[]) => void): Promise<() => void> {
  return getCurrentWebview().onDragDropEvent((event) => {
    if (event.payload.type === "drop") {
      onDrop(event.payload.paths);
    }
  });
}

/** The words of an error that a command gave back. */
export function messageOf(error: unknown): string {
  if (typeof error === "string") {
    return error;
  }
  if (typeof error === "object" && error !== null && "message" in error) {
    const { message } = error as { message: unknown };
    if (typeof message === "string") {
      return message;
    }
  }
  return "Something went wrong, and the tool gave no reason.";
}
