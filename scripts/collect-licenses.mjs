// Writes THIRD-PARTY-LICENSES.txt: the licences and notice texts of the Rust crates and
// the npm packages that are in the Windows build of the desktop app. The installer
// carries the file (licenses/THIRD-PARTY-LICENSES.txt, see tauri.bundle.conf.json).
// This is a build-time step. It reads only local files and makes no network call. It
// is plain Node, with no packages to install.
//
//   node scripts/collect-licenses.mjs            write THIRD-PARTY-LICENSES.txt at the root
//   node scripts/collect-licenses.mjs --check    fail when the file is not what this would write
//
// Needs: `cargo` with the crates downloaded (`cargo fetch --locked` once), and the npm
// packages installed (`pnpm install --frozen-lockfile` in app/).
//
// Rust: `cargo metadata --filter-platform x86_64-pc-windows-msvc`, from the package
// `adapdf-app`, through the normal dependencies only (no build-only and no dev
// dependencies). A proc-macro crate is a normal dependency, so it is listed although it
// does not end up in the program. The two crates of this project are left out.
// npm: the `dependencies` of app/package.json and, from `app/node_modules`, their
// dependencies, optional dependencies and peer dependencies (pnpm keeps each package's
// dependencies next to it). This is a little more than what Vite puts in `app/dist`.
// For every package: name, version, the licence expression of the package, and the text
// of every file in its folder whose name starts with LICENSE, LICENCE, COPYING, NOTICE or
// UNLICENSE (any case). A package with no such file is listed at the top of the output
// and on stderr. The same text is printed once, with the list of packages that use it.
// The output has no date, so a run on the same tree gives the same file.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, readFileSync, readdirSync, realpathSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const outFile = join(root, "THIRD-PARTY-LICENSES.txt");
const check = process.argv.includes("--check");

const LICENCE_FILE = /^(licen[cs]e|copying|notice|unlicense)/i;

/** The licence files of a package folder: [{ name, text }], sorted by name. */
// A few crates publish no licence file in their package. For those, the text taken from
// the crate's own repository is kept in licenses/upstream/<name>-<version>/ (see the
// README there for where each came from) and used instead.
function crateLicenceFiles(p) {
  const own = licenceFiles(dirname(p.manifest_path));
  if (own.length > 0) return own;
  const upstream = join(root, "licenses", "upstream", `${p.name}-${p.version}`);
  return existsSync(upstream) ? licenceFiles(upstream) : [];
}

function licenceFiles(dir) {
  return readdirSync(dir)
    .filter((name) => LICENCE_FILE.test(name) && statSync(join(dir, name)).isFile())
    .sort()
    .map((name) => ({
      name,
      text: readFileSync(join(dir, name), "utf8")
        .replace(/^﻿/, "")
        .replace(/\r\n?/g, "\n")
        .replace(/[ \t]+$/gm, "")
        .trim(),
    }))
    .filter((file) => file.text !== "");
}

// ---- Rust ----

function rustPackages() {
  const raw = execFileSync(
    "cargo",
    ["metadata", "--format-version", "1", "--filter-platform", "x86_64-pc-windows-msvc", "--locked"],
    { cwd: root, encoding: "utf8", maxBuffer: 1 << 30, stdio: ["ignore", "pipe", "inherit"] },
  );
  const metadata = JSON.parse(raw);
  const byId = new Map(metadata.packages.map((p) => [p.id, p]));
  const nodes = new Map(metadata.resolve.nodes.map((n) => [n.id, n]));
  const start = metadata.packages.find((p) => p.name === "adapdf-app");
  if (!start) throw new Error("cargo metadata has no package adapdf-app");

  const seen = new Set([start.id]);
  const queue = [start.id];
  while (queue.length > 0) {
    const node = nodes.get(queue.pop());
    for (const dep of node.deps) {
      if (!dep.dep_kinds.some((kind) => kind.kind === null) || seen.has(dep.pkg)) continue;
      seen.add(dep.pkg);
      queue.push(dep.pkg);
    }
  }
  return [...seen]
    .map((id) => byId.get(id))
    .filter((p) => p.source !== null) // the crates of this project have no registry source
    .map((p) => ({
      kind: "crate",
      name: p.name,
      version: p.version,
      licence: p.license ?? (p.license_file ? `see ${p.license_file}` : null),
      files: crateLicenceFiles(p),
    }));
}

// ---- npm ----

function npmPackages() {
  const appDir = join(root, "app");
  const appManifest = JSON.parse(readFileSync(join(appDir, "package.json"), "utf8"));
  const found = new Map(); // real path -> package
  const queue = Object.keys(appManifest.dependencies ?? {}).map((name) => ({
    name,
    nodeModules: join(appDir, "node_modules"),
    required: true,
  }));
  while (queue.length > 0) {
    const { name, nodeModules, required } = queue.pop();
    const link = join(nodeModules, ...name.split("/"));
    if (!existsSync(link)) {
      // An optional dependency of another platform is not installed: that is fine.
      if (required) throw new Error(`${name} is not in ${nodeModules}. Run pnpm install --frozen-lockfile in app/.`);
      continue;
    }
    const dir = realpathSync(link);
    if (found.has(dir)) continue;
    const manifest = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
    found.set(dir, {
      kind: "npm",
      name: manifest.name,
      version: manifest.version,
      licence:
        typeof manifest.license === "string"
          ? manifest.license
          : (manifest.license?.type ??
            (Array.isArray(manifest.licenses) ? manifest.licenses.map((l) => l.type).join(" OR ") : null)),
      files: licenceFiles(dir),
    });
    // pnpm keeps the dependencies of a package in the same node_modules folder as it.
    const own = dirname(dir);
    const siblings = name.startsWith("@") ? dirname(own) : own;
    const optional = new Set(Object.keys(manifest.optionalDependencies ?? {}));
    for (const dep of new Set([
      ...Object.keys(manifest.dependencies ?? {}),
      ...optional,
      ...Object.keys(manifest.peerDependencies ?? {}),
    ])) {
      const isOptional = optional.has(dep) || manifest.peerDependenciesMeta?.[dep]?.optional === true;
      queue.push({ name: dep, nodeModules: siblings, required: !isOptional });
    }
  }
  return [...found.values()];
}

// ---- Output ----

const byName = (a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : a.version < b.version ? -1 : a.version > b.version ? 1 : 0);
const crates = rustPackages().sort(byName);
const npm = npmPackages().sort(byName);

// One entry for each distinct text, numbered in the order of first use.
const texts = new Map(); // hash -> { number, text, files: Set of file names, users: [] }
for (const pkg of [...crates, ...npm]) {
  pkg.numbers = [];
  for (const file of pkg.files) {
    const hash = createHash("sha256").update(file.text).digest("hex");
    if (!texts.has(hash)) texts.set(hash, { number: texts.size + 1, text: file.text, names: new Set(), users: [] });
    const entry = texts.get(hash);
    entry.names.add(file.name);
    entry.users.push(`${pkg.kind} ${pkg.name} ${pkg.version}`);
    if (!pkg.numbers.includes(entry.number)) pkg.numbers.push(entry.number);
  }
}

const missing = [...crates, ...npm].filter((pkg) => pkg.files.length === 0);
const line = (pkg) =>
  `${pkg.name} ${pkg.version}: ${pkg.licence ?? "(no licence field)"}; ${
    pkg.numbers.length > 0 ? `text ${pkg.numbers.join(", ")}` : "NO LICENCE FILE FOUND"
  }`;

const out = [];
out.push(
  "THIRD-PARTY LICENCES",
  "",
  "This file lists the Rust crates and the npm packages that are in the Windows build of the",
  "Adapdf desktop app, each with its licence and the text of its licence and notice files.",
  "Adapdf starts veraPDF and a Java runtime (Eclipse Temurin) as separate programs; their",
  "licence texts are in the folder vendor next to the program (see THIRD-PARTY.md).",
  "The list is made by scripts/collect-licenses.mjs. It is checked by the maintainer. It is",
  "not legal advice.",
  "",
  `Rust crates: ${crates.length}. npm packages: ${npm.length}. Distinct texts: ${texts.size}.`,
  "",
);
if (missing.length > 0) {
  out.push(
    `${missing.length} package(s) with no licence file in their folder:`,
    ...missing.map((pkg) => `  ${pkg.kind} ${pkg.name} ${pkg.version}: ${pkg.licence ?? "(no licence field)"}`),
    "",
  );
}
out.push("== Rust crates ==", "", ...crates.map(line), "", "== npm packages ==", "", ...npm.map(line), "");
out.push("== Licence and notice texts ==", "");
for (const entry of texts.values()) {
  out.push(
    "-".repeat(78),
    `Text ${entry.number} (file name: ${[...entry.names].sort().join(", ")})`,
    `Used by: ${entry.users.join("; ")}`,
    "-".repeat(78),
    "",
    entry.text,
    "",
  );
}
const content = out.join("\n");

console.log(`Rust crates: ${crates.length}, npm packages: ${npm.length}, distinct texts: ${texts.size}.`);
for (const pkg of missing) console.error(`No licence file: ${pkg.kind} ${pkg.name} ${pkg.version} (${pkg.licence ?? "no licence field"})`);

if (check) {
  const current = existsSync(outFile) ? readFileSync(outFile, "utf8").replaceAll("\r\n", "\n") : null;
  if (current !== content) {
    console.error("THIRD-PARTY-LICENSES.txt is missing or out of date. Run: node scripts/collect-licenses.mjs");
    process.exit(1);
  }
  console.log("THIRD-PARTY-LICENSES.txt is up to date.");
} else {
  writeFileSync(outFile, content);
  console.log(`Wrote ${outFile}`);
}
