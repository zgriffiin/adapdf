<#
.SYNOPSIS
  Downloads veraPDF and a Java runtime into vendor/ (a build-time step).

.DESCRIPTION
  Adapdf runs the veraPDF command line with a bundled Java runtime, so that
  users need no Java install. This script gets both once, into the folder
  vendor/ (git ignores it):

    vendor/jre/       Eclipse Temurin JRE 21 (x64, Windows)
    vendor/verapdf/   veraPDF greenfield command line, installed headless

  Each download is checked against a SHA-256 value that is written in this
  script. A different value stops the script and nothing is installed.
  Running the script again does nothing when the right versions are there.

  This is the only place where the project downloads anything. The finished
  program makes no network calls.

  Run it with:
    powershell -ExecutionPolicy Bypass -File scripts/fetch-vendor.ps1

  Add -Force to download and install both again.

  Licences are in THIRD-PARTY.md.
#>
[CmdletBinding()]
param(
    [switch]$Force
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2.0
$ProgressPreference = 'SilentlyContinue'   # much faster downloads in Windows PowerShell 5.1
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
Add-Type -AssemblyName System.IO.Compression.FileSystem

# ---- Pinned versions ---------------------------------------------------------

# Eclipse Temurin JRE 21, from Adoptium's GitHub releases. The SHA-256 value is
# the one in the publisher's file OpenJDK21U-jre_x64_windows_hotspot_21.0.13_9.zip.sha256.txt.
$JreVersion = '21.0.13+9'
$JreUrl     = 'https://github.com/adoptium/temurin21-binaries/releases/download/jdk-21.0.13%2B9/OpenJDK21U-jre_x64_windows_hotspot_21.0.13_9.zip'
$JreSha256  = '2332c0b4cff6bed722da3567c91d5e47568bcba863fa4945d0a8d6fc76158628'

# veraPDF greenfield command line (an IzPack installer). veraPDF publishes only
# a PGP signature (.asc) for it, not a checksum file. The SHA-256 value below is
# of the file as downloaded over HTTPS from software.verapdf.org.
$VeraVersion = '1.28.2'
$VeraUrl     = 'https://software.verapdf.org/releases/1.28/verapdf-greenfield-1.28.2-installer.zip'
$VeraSha256  = 'd1693a5f0bf0997180f6d97e8a5568b0cf39e27eee338d439c8adb58435c1e89'

# ---- Paths -------------------------------------------------------------------

$Root     = Split-Path -Parent $PSScriptRoot
$Vendor   = Join-Path $Root 'vendor'
$JreDir   = Join-Path $Vendor 'jre'
$VeraDir  = Join-Path $Vendor 'verapdf'
$Work     = Join-Path $Vendor ('.work-' + [Guid]::NewGuid().ToString('N'))
$JreMark  = Join-Path $JreDir '.adapdf-version'
$VeraMark = Join-Path $VeraDir '.adapdf-version'

function Get-Marker($path) {
    if (Test-Path -LiteralPath $path) { return (Get-Content -LiteralPath $path -Raw).Trim() }
    return ''
}

function Save-Verified($url, $sha256, $target) {
    Write-Host "Downloading $url"
    Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $target
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $target).Hash.ToLowerInvariant()
    if ($actual -ne $sha256) {
        throw "SHA-256 mismatch for $url`n  expected $sha256`n  got      $actual`nNothing was installed."
    }
    Write-Host "  SHA-256 ok ($actual)"
}

function Expand-Zip($zip, $destination) {
    [System.IO.Compression.ZipFile]::ExtractToDirectory($zip, $destination)
}

function Replace-Directory($source, $destination) {
    if (Test-Path -LiteralPath $destination) { Remove-Item -LiteralPath $destination -Recurse -Force }
    Move-Item -LiteralPath $source -Destination $destination
}

# ---- Work --------------------------------------------------------------------

$jreMarker  = "temurin-jre $JreVersion sha256=$JreSha256"
$veraMarker = "verapdf-greenfield $VeraVersion sha256=$VeraSha256"
$jreJava    = Join-Path $JreDir 'bin\java.exe'

$needJre  = $Force -or -not (Test-Path -LiteralPath $jreJava) -or ((Get-Marker $JreMark) -ne $jreMarker)
$needVera = $Force -or -not (Test-Path -LiteralPath (Join-Path $VeraDir 'bin')) -or ((Get-Marker $VeraMark) -ne $veraMarker)

if (-not $needJre -and -not $needVera) {
    Write-Host "vendor/ is up to date: Temurin JRE $JreVersion and veraPDF $VeraVersion."
    exit 0
}

New-Item -ItemType Directory -Path $Work -Force | Out-Null
try {
    if ($needJre) {
        $zip = Join-Path $Work 'jre.zip'
        Save-Verified $JreUrl $JreSha256 $zip
        $unpacked = Join-Path $Work 'jre'
        Expand-Zip $zip $unpacked
        $inner = Get-ChildItem -LiteralPath $unpacked -Directory | Select-Object -First 1
        if ($null -eq $inner -or -not (Test-Path -LiteralPath (Join-Path $inner.FullName 'bin\java.exe'))) {
            throw 'The JRE archive does not have bin\java.exe where it was expected.'
        }
        Set-Content -LiteralPath (Join-Path $inner.FullName '.adapdf-version') -Value $jreMarker -Encoding ASCII
        Replace-Directory $inner.FullName $JreDir
        Write-Host "Installed the Temurin JRE $JreVersion in $JreDir"
    }

    if ($needVera) {
        $zip = Join-Path $Work 'verapdf.zip'
        Save-Verified $VeraUrl $VeraSha256 $zip
        $unpacked = Join-Path $Work 'verapdf-installer'
        Expand-Zip $zip $unpacked
        $jar = Get-ChildItem -LiteralPath $unpacked -Recurse -Filter 'verapdf-izpack-installer-*.jar' | Select-Object -First 1
        if ($null -eq $jar) { throw 'The veraPDF archive has no installer jar.' }

        # A headless install: the IzPack installer reads its answers from an XML file.
        # veraPDF's installer requires its GUI pack, so that one is selected too.
        $target = Join-Path $Work 'verapdf-installed'
        $answers = @"
<?xml version="1.0" encoding="UTF-8" standalone="no"?>
<AutomatedInstallation langpack="eng">
    <com.izforge.izpack.panels.htmlhello.HTMLHelloPanel id="welcome"/>
    <com.izforge.izpack.panels.target.TargetPanel id="install_dir">
        <installpath>$([Security.SecurityElement]::Escape($target))</installpath>
    </com.izforge.izpack.panels.target.TargetPanel>
    <com.izforge.izpack.panels.packs.PacksPanel id="sdk_pack_select">
        <pack index="0" name="veraPDF Batch files" selected="true"/>
        <pack index="1" name="veraPDF Mac and *nix Scripts" selected="false"/>
        <pack index="2" name="veraPDF Validation model" selected="true"/>
        <pack index="3" name="veraPDF Documentation" selected="false"/>
        <pack index="4" name="veraPDF Sample Plugins" selected="false"/>
        <pack index="5" name="veraPDF GUI" selected="true"/>
    </com.izforge.izpack.panels.packs.PacksPanel>
    <com.izforge.izpack.panels.install.InstallPanel id="install"/>
    <com.izforge.izpack.panels.finish.FinishPanel id="finish"/>
</AutomatedInstallation>
"@
        $answersFile = Join-Path $Work 'auto-install.xml'
        [IO.File]::WriteAllText($answersFile, $answers, (New-Object Text.UTF8Encoding($false)))

        # The bundled JRE runs the installer, so no Java install is needed here either.
        # (If only veraPDF is fetched again, vendor/jre is already in place.)
        & $jreJava -jar $jar.FullName $answersFile
        if ($LASTEXITCODE -ne 0) { throw "The veraPDF installer failed (exit code $LASTEXITCODE)." }
        if (-not (Get-ChildItem -LiteralPath (Join-Path $target 'bin') -Filter 'greenfield-apps-*.jar' -ErrorAction SilentlyContinue)) {
            throw 'The veraPDF install has no greenfield-apps jar in bin\.'
        }
        # The uninstaller is not needed in a folder that is copied around.
        $uninstaller = Join-Path $target 'Uninstaller'
        if (Test-Path -LiteralPath $uninstaller) { Remove-Item -LiteralPath $uninstaller -Recurse -Force }
        Set-Content -LiteralPath (Join-Path $target '.adapdf-version') -Value $veraMarker -Encoding ASCII
        Replace-Directory $target $VeraDir
        Write-Host "Installed veraPDF $VeraVersion in $VeraDir"
    }
}
finally {
    if (Test-Path -LiteralPath $Work) { Remove-Item -LiteralPath $Work -Recurse -Force -ErrorAction SilentlyContinue }
}

Write-Host 'Done. vendor/ has the bundled tools.'
