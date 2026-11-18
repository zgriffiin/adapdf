# Third-party software

Adapdf is licensed under the Apache License 2.0 (see `LICENSE`). It includes the
software below. Each part keeps its own licence. The maintainer checked this list
for v0.1.0. It is not legal advice. A person must check it again before each
release.

## Bundled programs (folder `vendor`)

These two programs are not part of the Rust code. `scripts/fetch-vendor.ps1`
downloads them into `vendor/`, and the installer puts them in the folder `vendor`
next to the program. Adapdf starts them as separate programs. It does not link
to them.

- veraPDF (greenfield command line and validation profiles), version 1.28.2.
  Licence: GNU GPL v3 or later, or Mozilla Public License v2 or later (a dual
  licence). Adapdf distributes it under the MPL-2.0 option. The licence texts are
  in the files `vendor/verapdf/model/veraPDF-model-rel-1.28/LICENSE.GPL` and
  `LICENSE.MPL`. Source code: <https://github.com/veraPDF> and
  <https://software.verapdf.org/releases/1.28/>. Web site: <https://verapdf.org/>.
- Eclipse Temurin JRE (Java runtime), version 21.0.13+9, x64, Windows.
  Licence: GNU GPL v2 with the Classpath Exception. The folder `vendor/jre/legal/`
  has the licence text of the runtime and of every part of it, and `vendor/jre/NOTICE`
  is the notice of the runtime. Web site: <https://adoptium.net/>. Binary:
  <https://github.com/adoptium/temurin21-binaries/releases/tag/jdk-21.0.13%2B9>.
  The complete corresponding source code is offered from the same place as the
  download of the program (the GitHub release of Adapdf), as the archive that
  Adoptium publishes for this version:
  `OpenJDK21U-jdk-sources_21.0.13_9.tar.gz`
  (<https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.13%2B9/OpenJDK21U-jdk-sources_21.0.13_9.tar.gz>),
  SHA-256 `b5232c9980d8a0f0459676c084c41c6e2c771ed9e785890b03c6a438406ad1ca`
  (the value in Adoptium's file `OpenJDK21U-jdk-sources_21.0.13_9.tar.gz.sha256.txt`).

## Checked for v0.1.0

| Point | Status |
|---|---|
| `vendor/jre/legal/` and `vendor/jre/NOTICE` stay with the runtime. | Done. `scripts/build-installer.ps1` copies the whole of `vendor/`, and the installed folder has both (test plan, I-11). |
| The licence texts of veraPDF (GPLv3+ and MPLv2+) ship, and the source of veraPDF can be had. | Done. The texts are in `vendor/verapdf/model/`, and the links are above. |
| Java and veraPDF run as separate programs, and Adapdf does not link to them. | Done. Said above. The code starts them as processes (`src/verapdf`). |
| The source of the Temurin runtime is offered from the same place as the download. | At release. The maintainer attaches the archive named above to the GitHub release and checks its SHA-256 against the value above. Until it is attached, this point is open. |
| The licence texts of the Rust crates and the npm packages in the build ship. | Done. `scripts/collect-licenses.mjs` writes `THIRD-PARTY-LICENSES.txt`, which the installer puts at `licenses/THIRD-PARTY-LICENSES.txt`. Six crates publish no licence file in their package (`alloc-stdlib` 0.2.4 and 0.3.0, `selectors` 0.38.0, `webview2-com` and `webview2-com-sys` 0.39.1, `webview2-com-macros` 0.8.1); their texts were taken from the upstream repositories into `licenses/upstream/`, which the script uses for them (sources in `licenses/upstream/README.md`). |
| The licence of IBM Plex Sans (SIL OFL 1.1) ships. | Done. `licenses/IBM-Plex-OFL-1.1.txt` in the folder of the program (it is also in `THIRD-PARTY-LICENSES.txt`). |
| The licence of Adapdf ships. | Done. `LICENSE` is in the repository and in the installer at `licenses/Adapdf-LICENSE.txt`. |

This list is checked by the maintainer. It is not legal advice.

## Rust libraries

The Rust crates in `Cargo.lock` have their own licences (mostly MIT or Apache-2.0).
`lopdf` (PDF metadata, stage 4) is MIT. `THIRD-PARTY-LICENSES.txt` lists every crate
that is in the Windows build of the app (with the licence expression of each)
and the text of each licence file that the crate has. Build-only crates (such as
`tauri-build`) are not in it. Run `node scripts/collect-licenses.mjs` again after
a change of `Cargo.lock` or `app/pnpm-lock.yaml`; `node scripts/collect-licenses.mjs
--check` fails when the file is out of date.

## The app

The page of the app (`app/`) is built with these packages. They are inside the
program (the built page is carried in the program file). The fonts are files in
the page, not a download.

- IBM Plex Sans (`@ibm/plex-sans`, the `woff2` files), version 1.1.0. Licence: SIL
  Open Font License 1.1. The licence text is in the file
  `licenses/IBM-Plex-OFL-1.1.txt` in the folder of the program.
- IBM Carbon (`@carbon/react`, `@carbon/styles` and the other `@carbon/*`
  packages), version 1.120.0. Licence: Apache-2.0.
- React and React DOM, version 19.3.1. Licence: MIT.
- Tauri (the Rust crates `tauri`, `tauri-plugin-dialog`, `tauri-plugin-opener` and
  their dependencies, and `@tauri-apps/api` and `@tauri-apps/plugin-dialog`),
  version 2.12.4 and others (see `Cargo.lock` and `app/pnpm-lock.yaml`). Licence:
  MIT or Apache-2.0.

`THIRD-PARTY-LICENSES.txt` lists the npm packages of the production dependencies of
`app/package.json` and their dependencies (77 at v0.1.0, a little more than what
ends up in the built page), each with its licence and its licence text.
