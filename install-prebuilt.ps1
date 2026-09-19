# xfetch prebuilt installer for Windows
#
# Downloads the precompiled Windows binary from GitHub Releases, verifies its
# SHA256 checksum and installs it next to a user directory.
#
# Usage:
#   irm https://raw.githubusercontent.com/xfetch-cli/xfetch/main/install-prebuilt.ps1 | iex
#   .\install-prebuilt.ps1 -Version 1.0.0 -BinDir C:\tools\bin
#   .\install-prebuilt.ps1 -NoChecksum -NoPath

param(
    [string]$Version = "",
    [string]$BinDir = "",
    [switch]$NoChecksum,
    [switch]$NoPath
)

$ErrorActionPreference = "Stop"
$Repo = "xfetch-cli/xfetch"
$Project = "xfetch"
$Target = "x86_64-pc-windows-msvc"
$UserAgent = "xfetch-installer"

function Write-Step($Message) { Write-Host "[xfetch] $Message" -ForegroundColor Cyan }
function Write-Ok($Message)   { Write-Host "[xfetch] $Message" -ForegroundColor Green }
function Write-Warn($Message) { Write-Host "[xfetch] $Message" -ForegroundColor Yellow }
function Fail($Message) {
    Write-Host "[xfetch] $Message" -ForegroundColor Red
    exit 1
}

# Windows PowerShell 5.1 defaults to older TLS versions; GitHub needs 1.2.
if ($PSVersionTable.PSVersion.Major -lt 6) {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
}

# Resolve the release version (latest when not pinned).
if (-not $Version) {
    Write-Step "Resolving the latest release..."
    try {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{ "User-Agent" = $UserAgent } -TimeoutSec 30
        $Version = $release.tag_name -replace '^v', ''
    } catch {
        Fail "Cannot reach GitHub: $($_.Exception.Message)"
    }
}
if (-not $Version) {
    Fail "Could not determine the release version."
}

$Asset = "$Project-$Version-$Target.zip"
$BaseUrl = "https://github.com/$Repo/releases/download/v$Version"
$AssetUrl = "$BaseUrl/$Asset"
$SumsUrl = "$BaseUrl/SHA256SUMS"

# Default install directory: a per-user location, never Program Files.
if (-not $BinDir) {
    if ($env:LOCALAPPDATA) {
        $BinDir = Join-Path $env:LOCALAPPDATA "Programs\xfetch\bin"
    } elseif ($env:USERPROFILE) {
        $BinDir = Join-Path $env:USERPROFILE ".local\bin"
    } else {
        Fail "Cannot determine an install directory; pass -BinDir."
    }
}

$TempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("xfetch-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $TempDir | Out-Null

try {
    $archivePath = Join-Path $TempDir $Asset

    Write-Step "Downloading $Asset..."
    try {
        Invoke-WebRequest -Uri $AssetUrl -OutFile $archivePath -Headers @{ "User-Agent" = $UserAgent } -UseBasicParsing -TimeoutSec 300
    } catch {
        Fail "Download failed: $($_.Exception.Message)"
    }

    if (-not $NoChecksum) {
        Write-Step "Verifying SHA256..."
        $sumsPath = Join-Path $TempDir "SHA256SUMS"
        try {
            Invoke-WebRequest -Uri $SumsUrl -OutFile $sumsPath -Headers @{ "User-Agent" = $UserAgent } -UseBasicParsing -TimeoutSec 30
        } catch {
            Fail "Cannot download SHA256SUMS: $($_.Exception.Message)"
        }

        $expected = $null
        foreach ($line in Get-Content $sumsPath) {
            if ($line -match [regex]::Escape($Asset)) {
                $expected = ($line -split '\s+')[0]
                break
            }
        }
        if (-not $expected) {
            Fail "SHA256SUMS does not list $Asset; refusing to install an unverified binary."
        }

        $actual = (Get-FileHash -Algorithm SHA256 -Path $archivePath).Hash.ToLower()
        if ($actual -ne $expected.ToLower()) {
            Fail "Checksum mismatch for $Asset (expected $expected, got $actual)."
        }
        Write-Ok "Checksum verified."
    } else {
        Write-Warn "Skipping checksum verification (-NoChecksum)."
    }

    Write-Step "Extracting..."
    $stageDir = Join-Path $TempDir "stage"
    Expand-Archive -Path $archivePath -DestinationPath $stageDir -Force

    $binary = Get-ChildItem -Path $stageDir -Recurse -Filter "$Project.exe" | Select-Object -First 1
    if (-not $binary) {
        Fail "The archive does not contain $Project.exe."
    }

    New-Item -ItemType Directory -Path $BinDir -Force | Out-Null
    $installPath = Join-Path $BinDir "$Project.exe"
    # Stage next to the destination and move: a failed copy must never
    # truncate an existing installation.
    $tempPath = Join-Path $BinDir (".$Project.xfetch-tmp-" + [guid]::NewGuid().ToString("N"))
    try {
        Copy-Item -Path $binary.FullName -Destination $tempPath -Force
        Move-Item -Path $tempPath -Destination $installPath -Force
    } catch {
        Remove-Item -Path $tempPath -Force -ErrorAction SilentlyContinue
        Fail "Could not install the binary: $($_.Exception.Message)"
    }
    Write-Ok "Installed binary: $installPath"

    if (-not $NoPath -and $env:OS -eq "Windows_NT") {
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($userPath -notlike "*$BinDir*") {
            [Environment]::SetEnvironmentVariable("Path", "$userPath;$BinDir", "User")
            Write-Ok "Added $BinDir to the user PATH (restart your terminal)."
        }
    }
} finally {
    Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Ok "Installation complete. Run '$Project' to start."
