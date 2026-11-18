// Runs axe-core, with the colour contrast rule on, in the real WebView2 of the
// built app, on each screen that can be reached (test row A-05, step 3).
//
// jsdom has no layout and no canvas, so the Vitest tests turn the contrast rule off.
// Here the page is the real one, drawn by WebView2, so axe can measure the colours.
//
//   pnpm tauri build --no-bundle        (in app/: makes ../target/release/adapdf-app.exe)
//   node scripts/axe-webview.mjs [--exe <adapdf-app.exe>] [--vendor <vendor folder>]
//   node scripts/axe-webview.mjs --installed <installed adapdf-app.exe> [--no-java] [--watch] [--default-profile]
//
// How it works, with no package beyond axe-core (already a dev dependency) and
// Node's own WebSocket:
// * It copies the program into a scratch folder, so that `vendor/` can be next to it
//   (a release build looks only there). `--vendor` is a folder that has `jre/` and
//   `verapdf/`; it is joined into the scratch folder (a directory junction, no copy).
//   Without it, or with no Word, the one real export is skipped and said so.
// * It starts the program itself, with WebView2's debugging port on a free port on
//   127.0.0.1 and a user data folder of its own, and ends only that process tree.
// * It drives the page with the Chrome DevTools protocol: clicks, typing, and
//   `axe.run` with the WCAG 2.1 A and AA rules and `color-contrast` on.
// * The one thing it replaces is the Windows file chooser (a native window that a
//   script cannot use): the page's `fetch` call for `plugin:dialog|open` is answered
//   with the next path of a list. Every command of the app (check, fix_offers, apply_fixes, export,
//   read_report, save_signoff) is the real one.
// * The documents are copies of fixtures in a scratch folder, which is removed at
//   the end.
//
// With `--installed` the script starts the program of an installation in place (rows
// I-05, I-07, I-10, I-11): no copy, and no `vendor/` is joined, so the program must
// find the `vendor/` that the installer placed next to it. `ADAPDF_VENDOR=C:/nowhere`
// is in its environment, to show that the override is ignored. The other options are
// for this mode:
// * `--no-java` takes every Java folder out of PATH, and JAVA_HOME and its kin out of
//   the environment, of the started program only (row I-07), and shows that `where java`
//   finds nothing there.
// * `--watch` looks about once a second at the processes that the program started (its
//   own process tree) and at the network connections that they own (`netstat -ano`).
//   It lists each `java.exe` that was started (its path and command line) and fails
//   when one is not the bundled one, when none was seen, or when a process of the tree
//   has a connection to an address that is not on this machine (rows I-07 and N-03).
//   This is sampling: a connection that opens and closes between two looks is missed.
// * `--default-profile` leaves WebView2's user data folder at its default
//   (`%LOCALAPPDATA%\com.adapdf.desktop`), as when a person starts the program, so that
//   what an uninstall leaves behind can be seen (row I-04).
//
// Exit code 0: no violation on any screen. 1: a violation, or a step failed.

import { spawn, spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync } from "node:fs";
import { createServer } from "node:net";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const appDir = resolve(here, "..");
const root = resolve(appDir, "..");

function option(name, fallback) {
  const at = process.argv.indexOf(name);
  return at >= 0 ? process.argv[at + 1] : fallback;
}

const installedExe = option("--installed", undefined);
const exeSource = resolve(installedExe ?? option("--exe", join(root, "target", "release", "adapdf-app.exe")));
const vendorSource = option("--vendor", process.env.ADAPDF_AXE_VENDOR ?? join(root, "vendor"));
const noJava = process.argv.includes("--no-java");
const watch = process.argv.includes("--watch");
const defaultProfile = process.argv.includes("--default-profile");
const extraBrowserArgs = option("--browser-args", "");
const axeSource = readFileSync(join(appDir, "node_modules", "axe-core", "axe.min.js"), "utf8");
const TAGS = ["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"];

if (!existsSync(exeSource)) {
  console.error(`${exeSource} is not there. Run \`pnpm tauri build --no-bundle\` in app/ first.`);
  process.exit(1);
}

const scratch = mkdtempSync(join(tmpdir(), "adapdf-axe-"));
const appCopy = join(scratch, "app");
const docs = join(scratch, "docs");
mkdirSync(appCopy);
mkdirSync(docs);
const exe = installedExe ? exeSource : join(appCopy, "adapdf-app.exe");
const exeName = basename(exe);
let haveVendor;
if (installedExe) {
  const installedVendor = join(dirname(exe), "vendor");
  haveVendor = existsSync(join(installedVendor, "jre", "bin", "java.exe")) && existsSync(join(installedVendor, "verapdf", "bin"));
} else {
  copyFileSync(exeSource, exe);
  haveVendor = existsSync(join(vendorSource, "jre")) && existsSync(join(vendorSource, "verapdf"));
  if (haveVendor) {
    symlinkSync(resolve(vendorSource), join(appCopy, "vendor"), "junction");
  }
}
for (const name of ["LST-002-fail", "DOC-001-fail", "DOC-001-pass", "IMG-001-fail"]) {
  copyFileSync(join(root, "tests", "fixtures", `${name}.docx`), join(docs, `${name}.docx`));
}
const docPath = (name) => join(docs, `${name}.docx`).replaceAll("/", "\\");

const freePort = () =>
  new Promise((ok, fail) => {
    const server = createServer();
    server.listen(0, "127.0.0.1", () => {
      const { port } = server.address();
      server.close(() => ok(port));
    });
    server.on("error", fail);
  });

const sleep = (ms) => new Promise((ok) => setTimeout(ok, ms));
const pids = (image) => {
  const out = spawnSync("tasklist", ["/FI", `IMAGENAME eq ${image}`, "/FO", "CSV", "/NH"], { encoding: "utf8" }).stdout;
  return out.split(/\r?\n/).filter((l) => l.startsWith('"')).map((l) => Number(l.split('","')[1]));
};

// ----------------------------------------------------- environment and watching

/** The environment of the started program. */
function programEnvironment(port) {
  const env = { ...process.env };
  // Windows names are not case sensitive, so find the real spelling of each name.
  const key = (name) => Object.keys(env).find((k) => k.toLowerCase() === name.toLowerCase());
  if (noJava) {
    const removed = [];
    for (const name of ["JAVA_HOME", "JDK_HOME", "JRE_HOME", "CLASSPATH", "_JAVA_OPTIONS", "JAVA_TOOL_OPTIONS"]) {
      const k = key(name);
      if (k) {
        delete env[k];
        removed.push(name);
      }
    }
    const pathKey = key("PATH");
    const kept = [];
    for (const dir of env[pathKey].split(";").filter(Boolean)) {
      const hasJava = ["java.exe", "javaw.exe", "javac.exe"].some((f) => existsSync(join(dir, f)));
      if (hasJava || /java|jdk|jre|adoptium|temurin/i.test(dir)) {
        removed.push(`PATH entry ${dir}`);
      } else {
        kept.push(dir);
      }
    }
    env[pathKey] = kept.join(";");
    const where = spawnSync("where", ["java"], { env, encoding: "utf8" });
    console.log(`--no-java: removed from the environment of the program: ${removed.join("; ") || "nothing"}`);
    console.log(`           \`where java\` in that environment: ${where.status === 0 ? "FOUND " + where.stdout.trim() : "finds nothing"}`);
    if (where.status === 0) {
      throw new Error("Java is still on the PATH of the program.");
    }
  }
  env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = `--remote-debugging-port=${port} --remote-allow-origins=* ${extraBrowserArgs}`.trim();
  if (!defaultProfile) {
    env.WEBVIEW2_USER_DATA_FOLDER = join(scratch, "webview2");
  }
  if (installedExe) {
    // A release build must ignore this (row I-10); the installed program is one.
    env.ADAPDF_VENDOR = "C:/nowhere";
  }
  return env;
}

const isLoopback = (address) => /^(127\.|\[::1\]|\[::ffff:127\.)/.test(address);

/** Looks at the process tree of `rootPid` (and its network endpoints) until `stop()`. */
function startWatch(rootPid) {
  const look = join(here, "look-at-process-tree.ps1");
  const procs = new Map(); // pid -> the process, at the first look that saw it
  const connections = new Map(); // "pid proto local remote state" -> the endpoint
  let looks = 0;
  let running = true;
  const loop = (async () => {
    while (running) {
      const run = spawnSync("powershell", ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File", look, String(rootPid)], {
        encoding: "utf8",
        maxBuffer: 64 * 1024 * 1024,
      });
      try {
        const seen = JSON.parse(run.stdout);
        looks++;
        for (const p of [].concat(seen.procs ?? [])) {
          // A process that is ending can show no path in one look and have it in another.
          const known = procs.get(p.pid);
          procs.set(p.pid, known ? { ...p, exe: p.exe ?? known.exe, cmd: p.cmd ?? known.cmd } : p);
        }
        for (const c of [].concat(seen.conns ?? [])) {
          const id = `${c.pid} ${c.proto} ${c.local} ${c.remote} ${c.state}`;
          if (!connections.has(id)) connections.set(id, c);
        }
      } catch {
        // A look that failed is not counted.
      }
      await sleep(200);
    }
  })();
  return {
    async stop() {
      running = false;
      await loop;
      return { looks, procs: [...procs.values()], connections: [...connections.values()] };
    },
  };
}

/** Prints what the watch saw. Gives the problems found (an empty list is a pass). */
function reportWatch(seen, vendorJre, exportRan) {
  const problems = [];
  const byName = new Map();
  for (const p of seen.procs) byName.set(p.name, (byName.get(p.name) ?? 0) + 1);
  console.log(`\nWatch: ${seen.looks} looks at the process tree of the program.`);
  console.log(`  processes seen (${seen.procs.length}): ${[...byName].map(([n, c]) => `${n} x${c}`).join(", ")}`);
  const javas = seen.procs.filter((p) => /^java\.exe$/i.test(p.name));
  for (const j of javas) {
    const path = j.exe ?? /^"([^"]+)"/.exec(j.cmd ?? "")?.[1];
    console.log(`  java.exe pid ${j.pid} (parent ${j.ppid}): ${path}`);
    console.log(`    command line: ${(j.cmd ?? "").slice(0, 900)}`);
    if (!path || !path.toLowerCase().startsWith(vendorJre.toLowerCase())) {
      problems.push(`a java.exe that is not the bundled one: ${path}`);
    }
  }
  if (exportRan && javas.length === 0) {
    problems.push("no java.exe was seen during the export (a run of veraPDF is shorter than a look?)");
  }
  const remote = seen.connections.filter((c) => c.remote && !isLoopback(c.remote) && !/^(\*:\*|0\.0\.0\.0:0|\[::\]:0)$/.test(c.remote));
  const local = seen.connections.filter((c) => !remote.includes(c));
  console.log(`  endpoints seen: ${seen.connections.length} (${local.length} on this machine only: listening, or the debugging port; ${remote.length} to other addresses)`);
  for (const c of remote) {
    const owned = seen.procs.find((p) => p.pid === c.pid);
    const owner = owned?.name ?? "?";
    const kind = /--type=[^ ]+( --utility-sub-type=[^ ]+)?/.exec(owned?.cmd ?? "")?.[0] ?? "(the main process of WebView2, or another)";
    console.log(`    to another address: ${c.proto} ${c.local} -> ${c.remote} ${c.state} (pid ${c.pid}, ${owner}, ${kind})`);
    problems.push(`a connection to another address: ${c.proto} ${c.local} -> ${c.remote} (${owner})`);
  }
  return problems;
}

// ------------------------------------------------------------------ DevTools

class Page {
  constructor(socket) {
    this.socket = socket;
    this.next = 1;
    this.waiting = new Map();
    socket.addEventListener("message", (event) => {
      const message = JSON.parse(event.data);
      const handler = this.waiting.get(message.id);
      if (handler) {
        this.waiting.delete(message.id);
        message.error ? handler.fail(new Error(JSON.stringify(message.error))) : handler.ok(message.result);
      }
    });
  }

  send(method, params = {}) {
    const id = this.next++;
    return new Promise((ok, fail) => {
      this.waiting.set(id, { ok, fail });
      this.socket.send(JSON.stringify({ id, method, params }));
    });
  }

  /** Runs an expression in the page and gives back its value (a promise is awaited). */
  async run(expression) {
    const { result, exceptionDetails } = await this.send("Runtime.evaluate", {
      expression,
      awaitPromise: true,
      returnByValue: true,
    });
    if (exceptionDetails) {
      throw new Error(exceptionDetails.exception?.description ?? exceptionDetails.text);
    }
    return result.value;
  }

  /** Waits until the expression is true. */
  async until(what, expression, seconds = 30) {
    const end = Date.now() + seconds * 1000;
    while (Date.now() < end) {
      if (await this.run(expression).catch(() => false)) {
        return;
      }
      await sleep(150);
    }
    const text = await this.run("document.body.innerText.slice(-700)").catch(() => "");
    throw new Error(`Timed out after ${seconds} s: ${what}. The page shows: ${text}`);
  }
}

const q = JSON.stringify;
const clickButton = (page, name) =>
  page.run(`(() => {
    const b = [...document.querySelectorAll("button")].find((b) => b.textContent.trim() === ${q(name)} && !b.disabled);
    if (!b) throw new Error("no enabled button " + ${q(name)});
    b.click();
  })()`);
/** Goes back to the first screen from any screen that has a way out. */
async function toFirstScreen(page) {
  const has = (name) =>
    page.run(`[...document.querySelectorAll("button")].some((b) => b.textContent.trim() === ${q(name)} && !b.disabled)`);
  if (await has("Back to the findings")) {
    await clickButton(page, "Back to the findings");
    await page.until("the findings", `[...document.querySelectorAll("h2")].some((h) => h.textContent.includes(": "))`);
  }
  await clickButton(page, "Choose another file");
  await page.until("the first screen", hasHeading("Choose a Word document"));
}
const hasHeading = (text) =>
  `[...document.querySelectorAll("h2")].some((h) => h.textContent.includes(${q(text)}))`;
const setValue = (page, selector, value) =>
  page.run(`(() => {
    const el = document.querySelector(${q(selector)});
    if (!el) throw new Error("no field " + ${q(selector)});
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(el, ${q(value)});
    el.dispatchEvent(new Event("input", { bubbles: true }));
  })()`);
const choose = (page, path) => page.run(`(window.__chosen = window.__chosen || []).push(${q(path)})`);

// -------------------------------------------------------------------- axe

const results = [];

async function axeScreen(page, name) {
  const outcome = await page.run(`(async () => {
    const r = await axe.run(document, {
      runOnly: { type: "tag", values: ${q(TAGS)} },
      resultTypes: ["violations", "incomplete", "passes"],
    });
    const contrast = (list) => list.filter((x) => x.id === "color-contrast");
    const nodes = (list) => contrast(list).reduce((n, x) => n + x.nodes.length, 0);
    return {
      violations: r.violations.map((v) => ({
        id: v.id,
        nodes: v.nodes.map((n) => ({
          target: n.target.join(" "),
          summary: n.any.concat(n.all, n.none).map((c) => c.message).join(" | "),
        })),
      })),
      contrastChecked: nodes(r.passes),
      contrastIncomplete: contrast(r.incomplete).flatMap((x) =>
        x.nodes.map((n) => n.target.join(" ") + ": " + n.any.map((c) => c.message).join(" | "))),
      rulesRun: r.passes.length + r.violations.length + r.incomplete.length,
    };
  })()`);
  const contrastViolations = outcome.violations.filter((v) => v.id === "color-contrast");
  results.push({ name, ...outcome, contrastViolations: contrastViolations.length });
  const flag = outcome.violations.length === 0 ? "ok  " : "FAIL";
  console.log(
    `${flag} ${name}: ${outcome.violations.length} violations (color-contrast: ${contrastViolations.length}), ` +
      `contrast measured on ${outcome.contrastChecked} elements, ${outcome.contrastIncomplete.length} contrast checks incomplete`,
  );
  for (const v of outcome.violations) {
    for (const n of v.nodes) {
      console.log(`       ${v.id}: ${n.target} ${n.summary}`);
    }
  }
  for (const text of outcome.contrastIncomplete) {
    console.log(`       incomplete: ${text}`);
  }
}

// -------------------------------------------------------------------- run

let child;
let watcher;
let exportRan = false;
let failed = false;
const before = { app: pids(exeName), word: pids("WINWORD.EXE") };
console.log(`WINWORD.EXE before: ${before.word.length ? before.word.join(",") : "none"}`);

try {
  const port = await freePort();
  child = spawn(exe, [], {
    cwd: installedExe ? dirname(exe) : appCopy,
    stdio: "ignore",
    env: programEnvironment(port),
  });
  console.log(`Started ${exeName} (process ${child.pid}), debugging port ${port}, folder ${scratch}`);
  if (watch) {
    watcher = startWatch(child.pid);
  }

  let target;
  for (let i = 0; i < 100 && !target; i++) {
    await sleep(300);
    const list = await fetch(`http://127.0.0.1:${port}/json/list`).then((r) => r.json()).catch(() => []);
    target = list.find((t) => t.type === "page" && /tauri\.localhost/.test(t.url));
  }
  if (!target) {
    throw new Error("The page of the app did not appear on the debugging port.");
  }
  const socket = new WebSocket(target.webSocketDebuggerUrl);
  await new Promise((ok, fail) => {
    socket.addEventListener("open", ok);
    socket.addEventListener("error", fail);
  });
  const page = new Page(socket);
  await page.until("the first screen", hasHeading("Choose a Word document"));
  await page.run(axeSource);

  // The Windows file chooser is replaced; every command of the app is real.
  // Tauri freezes `__TAURI_INTERNALS__.invoke`, but it sends each call with `fetch`
  // to its own address (`http://ipc.localhost/<command>`), so the answer to the
  // call `plugin:dialog|open` is made here, in the way Tauri's own answer is made.
  await page.run(`(() => {
    window.__chosen = [];
    const realFetch = window.fetch.bind(window);
    window.fetch = (input, init) => {
      const url = typeof input === "string" ? input : input.url;
      if (new RegExp("ipc.localhost/plugin(%3A|:)dialog(%7C|[|])open", "i").test(url)) {
        const path = window.__chosen.shift() ?? null;
        return Promise.resolve(
          new Response(JSON.stringify(path), {
            status: 200,
            headers: { "Content-Type": "application/json", "Tauri-Response": "ok" },
          }),
        );
      }
      return realFetch(input, init);
    };
  })()`);
  const probe = await page.run(`(async () => {
    window.__chosen.push("probe");
    return await window.__TAURI_INTERNALS__.invoke("plugin:dialog|open", {});
  })()`);
  if (probe !== "probe") {
    throw new Error("The file chooser could not be replaced.");
  }
  console.log(
    `The program: ${await page.run("navigator.userAgent")}, ${await page.run("window.innerWidth + 'x' + window.innerHeight")}`,
  );

  // A check of the check: light grey text on white must be found, or "0 violations"
  // would mean nothing.
  const planted = await page.run(`(async () => {
    const p = document.createElement("p");
    p.id = "planted-contrast-fault";
    p.style.cssText = "color:#c6c6c6;background:#ffffff";
    p.textContent = "Light grey text on white";
    document.body.append(p);
    const r = await axe.run(document, { runOnly: { type: "rule", values: ["color-contrast"] } });
    p.remove();
    return r.violations.flatMap((v) => v.nodes.map((n) => n.target.join(" ")));
  })()`);
  if (!planted.includes("#planted-contrast-fault")) {
    throw new Error(`axe did not find a planted contrast fault (found: ${JSON.stringify(planted)}).`);
  }
  console.log("ok   axe finds a planted contrast fault (grey #c6c6c6 on white), so a pass means something");

  // 1. The first screen, and the error message when a file cannot be checked.
  await axeScreen(page, "first screen");
  await choose(page, docPath("Missing"));
  await clickButton(page, "Choose a Word document");
  await page.until("the error message", `document.querySelector('[role="status"]')?.textContent.includes("could not be checked")`);
  await axeScreen(page, "first screen with an error message");

  // 2. LST-002-fail.docx: the findings, the fix flow, after a fix.
  await choose(page, docPath("LST-002-fail"));
  await clickButton(page, "Choose a Word document");
  await page.until("the findings", hasHeading("LST-002-fail.docx: Found"));
  await axeScreen(page, "findings of LST-002-fail.docx");
  await clickButton(page, "Fix problems");
  await page.until("the fix flow", hasHeading("Fix problems in LST-002-fail.docx"));
  await axeScreen(page, "fix flow of LST-002-fail.docx");
  await page.run(`document.querySelector('input[type="checkbox"][id^="tick-"]').click()`);
  await clickButton(page, "Apply 1 fix");
  await page.until("the result of the fix", `document.querySelector('[role="status"]')?.textContent.includes("The fixes were applied")`);
  await axeScreen(page, "findings after a fix (success message)");

  // 3. The fix flow with text fields (the suggestion is the helper text).
  for (const name of ["DOC-001-fail", "IMG-001-fail"]) {
    await toFirstScreen(page);
    await choose(page, docPath(name));
    await clickButton(page, "Choose a Word document");
    await page.until("the findings", hasHeading(`${name}.docx: Found`));
    await clickButton(page, "Fix problems");
    await page.until("the fix flow", hasHeading(`Fix problems in ${name}.docx`));
    await axeScreen(page, `fix flow of ${name}.docx (text field)`);
  }

  // 4. A report of an export that was blocked by errors (stage 1; Word is not started).
  const blocked = await page.run(
    `window.__TAURI_INTERNALS__.invoke("export", { path: ${q(docPath("DOC-001-fail"))} }).then((r) => r.view.report_json)`,
  );
  await toFirstScreen(page);
  await choose(page, blocked);
  await clickButton(page, "Open an earlier report");
  await page.until("the report", hasHeading("Result for DOC-001-fail.docx"));
  await axeScreen(page, "result: Fail, export blocked by errors (and the empty sign-off form)");
  await clickButton(page, "Save sign-off");
  await page.until("the name error", `!!document.querySelector("#signoff-name-error")`, 15);
  await axeScreen(page, "sign-off form with the error for a missing name");

  // 5. A real export with Word and veraPDF: the progress screen, a Pass, the sign-off.
  if (haveVendor) {
    await toFirstScreen(page);
    await choose(page, docPath("DOC-001-pass"));
    await clickButton(page, "Choose a Word document");
    await page.until("the findings", hasHeading("DOC-001-pass.docx: "));
    await axeScreen(page, "findings of DOC-001-pass.docx (no problems)");
    await clickButton(page, "Export PDF");
    await page.until("the progress screen", hasHeading("Exporting DOC-001-pass.docx"));
    await axeScreen(page, "export in progress");
    await page.until("the result", hasHeading("Result for DOC-001-pass.docx"), 240);
    const verdict = await page.run(`document.querySelector(".verdict")?.textContent`);
    console.log(`     the real export of DOC-001-pass.docx gave: ${verdict}`);
    exportRan = true;
    const veraLines = await page.run(
      `document.body.innerText.split("\\n").filter((l) => /veraPDF|WCAG/.test(l)).slice(0, 6).join(" | ")`,
    );
    console.log(`     the result names veraPDF and WCAG: ${veraLines}`);
    if (installedExe && !/pass/i.test(verdict ?? "")) {
      throw new Error(`The export from the installed program did not give a Pass: ${verdict}`);
    }
    await axeScreen(page, `result: ${verdict}, with the empty sign-off form`);
    for (let i = 1; i <= 5; i++) {
      await page.run(`document.querySelector("#check-${i}").click()`);
    }
    await setValue(page, "#signoff-name", "A. Reviewer");
    await setValue(page, "#signoff-date", "2026-03-06");
    await axeScreen(page, "sign-off form filled in (5 of 7 ticked, name and date)");
    await clickButton(page, "Save sign-off");
    await page.until("the saved message", `document.querySelector('[role="status"]')?.textContent.includes("The sign-off was saved")`);
    await axeScreen(page, "result after the sign-off was saved (success message)");
  } else {
    console.log("SKIPPED: the export with Word and veraPDF (no --vendor folder with jre/ and verapdf/).");
  }
  socket.close();
} catch (error) {
  failed = true;
  console.error(`FAILED: ${error.message}`);
} finally {
  if (watcher) {
    const problems = reportWatch(await watcher.stop(), join(dirname(exe), "vendor", "jre", "bin") + "\\", exportRan);
    if (problems.length > 0) {
      failed = true;
      for (const problem of problems) console.error(`FAILED: ${problem}`);
    }
  }
  // Only the process that this script started, and what it started.
  if (child?.pid) {
    spawnSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], { stdio: "ignore" });
  }
  await sleep(1500);
  const after = { app: pids(exeName), word: pids("WINWORD.EXE") };
  console.log(`${exeName} after: ${after.app.length ? after.app.join(",") : "none"}; WINWORD.EXE after: ${after.word.length ? after.word.join(",") : "none"}`);
  try {
    rmSync(scratch, { recursive: true, force: true });
  } catch (error) {
    console.error(`The scratch folder ${scratch} was not removed: ${error.message}`);
  }
}

const violations = results.reduce((n, r) => n + r.violations.length, 0);
const contrast = results.reduce((n, r) => n + r.contrastViolations, 0);
// A contrast check that axe could not decide is not a pass.
const undecided = results.reduce((n, r) => n + r.contrastIncomplete.length, 0);
console.log(
  `\n${results.length} screens, ${violations} violations in all, ${contrast} of them color-contrast, ${undecided} contrast checks undecided.`,
);
process.exit(failed || violations > 0 || undecided > 0 ? 1 : 0);
