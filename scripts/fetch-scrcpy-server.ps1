#requires -Version 5
# Download the pinned scrcpy server jar into android/prebuilt/.
# Run from the repo root: pwsh -File scripts/fetch-scrcpy-server.ps1

$ErrorActionPreference = 'Stop'

$Version = '4.0'
$FileName = "scrcpy-server-v$Version"
$Url = "https://github.com/Genymobile/scrcpy/releases/download/v$Version/$FileName"
$DestDir = Join-Path $PSScriptRoot '..\android\prebuilt'
$Dest = Join-Path $DestDir $FileName

if (-not (Test-Path $DestDir)) {
    New-Item -ItemType Directory -Path $DestDir | Out-Null
}

if (Test-Path $Dest) {
    Write-Host "Already present: $Dest"
    exit 0
}

Write-Host "Downloading $Url"
Write-Host "       -> $Dest"
Invoke-WebRequest -Uri $Url -OutFile $Dest -UseBasicParsing
$size = (Get-Item $Dest).Length
Write-Host "OK ($size bytes)"
