# Third-party software

Notes for the installer (milestone M5). They are not legal advice. A person
must check them before a release.

## Bundled with the tool

These two programs are not part of the Rust code. `scripts/fetch-vendor.ps1`
downloads them into `vendor/`, and the installer will copy them next to the
program. The tool starts them as separate programs.

| Software | Version | Licence | Source |
|---|---|---|---|
| veraPDF (greenfield command line and validation profiles) | 1.28.2 | GNU GPL v3 or later, or Mozilla Public License v2 or later (dual licence; the licence texts are in veraPDF's own files) | <https://verapdf.org/>, <https://software.verapdf.org/releases/1.28/> |
| Eclipse Temurin JRE (Java runtime) | 21.0.13+9, x64, Windows | GNU GPL v2 with the Classpath Exception (`vendor/jre/legal/` has the licence text of the runtime and of every part of it) | <https://adoptium.net/>, <https://github.com/adoptium/temurin21-binaries> |

What the installer must do:

- Keep `vendor/jre/legal/` and `vendor/jre/NOTICE` with the runtime.
- Include the licence texts of veraPDF (GPLv3+ and MPLv2+), and say where the
  source code of veraPDF can be had (the links above).
- Say that Java and veraPDF run as separate programs, and that Adapdf itself
  does not link to them.

## Rust libraries

The Rust crates in `Cargo.lock` have their own licences (mostly MIT or Apache-2.0).
`lopdf` (PDF metadata, stage 4) is MIT. Run `cargo license` or `cargo deny` on the
final lock file to produce the list for the installer.

## The app (M5a)

The page of the app (`app/`) is built with these packages. They end up in the built
page (`app/dist`), which the installer will carry inside the program. The fonts are
files in the page, not a download.

| Software | Version | Licence |
|---|---|---|
| IBM Plex Sans (`@ibm/plex-sans`, the `woff2` files in `app/dist/assets`) | 1.1.0 | SIL Open Font License 1.1. The licence text is in the package (`LICENSE.txt`); the installer must carry it with the fonts. |
| IBM Carbon (`@carbon/react`, `@carbon/styles` and the other `@carbon/*` packages) | 1.120.0 | Apache-2.0 |
| React and React DOM | 19.3.1 | MIT |
| Tauri (the Rust crates `tauri`, `tauri-plugin-dialog`, `tauri-plugin-opener` and their dependencies, and `@tauri-apps/api` and `@tauri-apps/plugin-dialog`) | 2.12.4 and others, see `Cargo.lock` and `app/pnpm-lock.yaml` | MIT or Apache-2.0 |

Run a licence tool on `app/pnpm-lock.yaml` and on `Cargo.lock` for the list that the
installer needs.
