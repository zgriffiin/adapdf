# Third-party software

Adapdf includes the software below. Each part keeps its own licence. This list
is not legal advice. A person must check it before a release.

## Bundled programs (folder `vendor`)

These two programs are not part of the Rust code. `scripts/fetch-vendor.ps1`
downloads them into `vendor/`, and the installer puts them in the folder `vendor`
next to the program. Adapdf starts them as separate programs. It does not link
to them.

- veraPDF (greenfield command line and validation profiles), version 1.28.2.
  Licence: GNU GPL v3 or later, or Mozilla Public License v2 or later (a dual
  licence). The licence texts are in the files
  `vendor/verapdf/model/veraPDF-model-rel-1.28/LICENSE.GPL` and `LICENSE.MPL`.
  Source code: <https://github.com/veraPDF> and
  <https://software.verapdf.org/releases/1.28/>. Web site: <https://verapdf.org/>.
- Eclipse Temurin JRE (Java runtime), version 21.0.13+9, x64, Windows.
  Licence: GNU GPL v2 with the Classpath Exception. The folder `vendor/jre/legal/`
  has the licence text of the runtime and of every part of it, and `vendor/jre/NOTICE`
  is the notice of the runtime. Source code and web site: <https://adoptium.net/>,
  <https://github.com/adoptium/temurin21-binaries>.

What the installer must do:

- Keep `vendor/jre/legal/` and `vendor/jre/NOTICE` with the runtime.
- Include the licence texts of veraPDF (GPLv3+ and MPLv2+), and say where the
  source code of veraPDF can be had (the links above).
- Say that Java and veraPDF run as separate programs, and that Adapdf itself
  does not link to them.

## Rust libraries

The Rust crates in `Cargo.lock` have their own licences (mostly MIT or Apache-2.0).
`lopdf` (PDF metadata, stage 4) is MIT. Run `cargo license` or `cargo deny` on the
final lock file to produce the full list.

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

Run a licence tool on `app/pnpm-lock.yaml` and on `Cargo.lock` for the full list of
the packages that these depend on.
