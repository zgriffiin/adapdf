<#
.SYNOPSIS
  Builds the per-user Windows installer of the Adapdf app (a build-time step).

.DESCRIPTION
  Does these steps:

    1. Checks that vendor/ has the bundled tools (jre/bin/java.exe and
       verapdf/bin). Get them with scripts/fetch-vendor.ps1.
    2. Copies vendor/ to app/src-tauri/bundle-staging/vendor (Git ignores it).
       The bundle settings in app/src-tauri/tauri.bundle.conf.json take the tools
       from there, so that no path of this machine is in a file of the project.
       The copy leaves out the per-clause source files of the veraPDF profiles
       (the folders inside PDF_A and PDF_UA, and .github). The veraPDF command line
       does not read them, and their paths are over 200 characters long: NSIS
       cannot open a path of 260 characters or more.
    3. Runs `pnpm tauri build` in app/ with those bundle settings. Tauri builds
       the page and the program, and makes the NSIS installer.

  Tauri downloads its NSIS tools once, the first time it builds an installer
  (a build-time download). The installer and the app make no network calls.

  The installer is in target/release/bundle/nsis/. Run it with:
    powershell -ExecutionPolicy Bypass -File scripts/build-installer.ps1

  Another place for vendor/ (for example in a build from a Git worktree) is
  given with -Vendor or with the environment variable ADAPDF_VENDOR_SRC.
#>
[CmdletBinding()]
param(
    [string]$Vendor = $env:ADAPDF_VENDOR_SRC
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0

$Root = Split-Path -Parent $PSScriptRoot
if (-not $Vendor) { $Vendor = Join-Path $Root 'vendor' }
$Vendor = (Resolve-Path -LiteralPath $Vendor).Path
$App = Join-Path $Root 'app'
$Staging = Join-Path $App 'src-tauri\bundle-staging\vendor'

if (-not (Test-Path -LiteralPath (Join-Path $Vendor 'jre\bin\java.exe')) -or
    -not (Test-Path -LiteralPath (Join-Path $Vendor 'verapdf\bin'))) {
    throw "$Vendor has no jre\bin\java.exe or no verapdf\bin. Run scripts\fetch-vendor.ps1 first."
}
if (-not (Test-Path -LiteralPath (Join-Path $App 'node_modules\@ibm\plex-sans\LICENSE.txt'))) {
    throw 'The font licence is missing. Run `pnpm install --frozen-lockfile` in app/ first.'
}

# Leave out what the tools do not read (see above). The merged profile files, for
# example PDF_UA\PDFUA-1.xml and PDF_UA\WCAG-2-2-Machine.xml, stay. robocopy gets
# the folders by full path, so that no long path is made in the staging folder
# (Windows PowerShell cannot delete one).
$skip = foreach ($pattern in @('verapdf\profiles\*\PDF_A\*', 'verapdf\profiles\*\PDF_UA\*', 'verapdf\profiles\*\.github')) {
    Get-ChildItem -Path (Join-Path $Vendor $pattern) -Directory -Force | ForEach-Object { $_.FullName }
}

Write-Host "Copying $Vendor to $Staging"
# Empty the staging folder first (an earlier copy may be of another vendor/). A
# mirror of an empty folder does it, and it works for long paths too.
$empty = Join-Path ([IO.Path]::GetTempPath()) ('adapdf-empty-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $empty | Out-Null
try {
    & robocopy $empty $Staging /MIR /NFL /NDL /NJH /NJS /NP | Out-Null
    if ($LASTEXITCODE -ge 8) { throw "robocopy failed (exit code $LASTEXITCODE)." }
}
finally { Remove-Item -LiteralPath $empty -Force }
& robocopy $Vendor $Staging /MIR /XD @skip /NFL /NDL /NJH /NJS /NP | Out-Null
if ($LASTEXITCODE -ge 8) { throw "robocopy failed (exit code $LASTEXITCODE)." }

# The installer must be able to write every path (MAX_PATH is 260). The install
# folder is about 40 characters long for a user, so this leaves 100 for it.
$tooLong = Get-ChildItem -LiteralPath $Staging -Recurse -File -Force |
    Where-Object { $_.FullName.Length - $Staging.Length -gt 150 }
if ($tooLong) {
    throw "These files have a path that is too long to install: $($tooLong.FullName -join ', ')"
}

Push-Location $App
try {
    & pnpm tauri build --config src-tauri/tauri.bundle.conf.json
    if ($LASTEXITCODE -ne 0) { throw "pnpm tauri build failed (exit code $LASTEXITCODE)." }
}
finally { Pop-Location }

Get-ChildItem -LiteralPath (Join-Path $Root 'target\release\bundle\nsis') -Filter '*-setup.exe' |
    ForEach-Object { Write-Host ('Installer: {0} ({1:N0} bytes)' -f $_.FullName, $_.Length) }
