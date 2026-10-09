<#
.SYNOPSIS
    Crea dist\hasher-<versione>-windows-x86_64.zip (forma portable, nessun installer).

.DESCRIPTION
    Lo zip contiene hasher.exe, README.md e la cartella LICENSES-fonts (licenze OFL dei font
    incorporati). Va lanciato dopo `cargo build --release`.

.EXAMPLE
    cargo build --release
    pwsh scripts/package-windows.ps1 0.1.0
#>
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string]$Version,

    [Parameter(Position = 1)]
    [string]$Binary
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Root = Split-Path -Parent $PSScriptRoot
$Version = $Version.TrimStart('v')
if (-not $Binary) {
    $Binary = Join-Path $Root 'target/release/hasher.exe'
}

$Name = "hasher-$Version-windows-x86_64"
$Dist = Join-Path $Root 'dist'
$Zip = Join-Path $Dist "$Name.zip"
$Readme = Join-Path $Root 'README.md'
$FontLicenses = Get-ChildItem -Path (Join-Path $Root 'assets/fonts') -Filter 'LICENSE-*.txt'

foreach ($f in @($Binary, $Readme)) {
    if (-not (Test-Path -LiteralPath $f -PathType Leaf)) {
        throw "File mancante: $f"
    }
}

$Stage = Join-Path ([System.IO.Path]::GetTempPath()) ("hasher-pkg-" + [guid]::NewGuid().ToString('N'))
try {
    New-Item -ItemType Directory -Path $Stage | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $Stage 'LICENSES-fonts') | Out-Null

    Copy-Item -LiteralPath $Binary -Destination (Join-Path $Stage 'hasher.exe')
    Copy-Item -LiteralPath $Readme -Destination (Join-Path $Stage 'README.md')
    foreach ($lic in $FontLicenses) {
        Copy-Item -LiteralPath $lic.FullName -Destination (Join-Path $Stage 'LICENSES-fonts')
    }

    New-Item -ItemType Directory -Path $Dist -Force | Out-Null
    if (Test-Path -LiteralPath $Zip) {
        Remove-Item -LiteralPath $Zip -Force
    }
    Compress-Archive -Path (Join-Path $Stage '*') -DestinationPath $Zip -CompressionLevel Optimal
}
finally {
    if (Test-Path -LiteralPath $Stage) {
        Remove-Item -LiteralPath $Stage -Recurse -Force
    }
}

Write-Host "Creato: $Zip"
