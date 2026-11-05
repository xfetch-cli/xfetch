# xfetch installer script for Windows
# Usage: powershell -ExecutionPolicy Bypass -File install.ps1
#        powershell -ExecutionPolicy Bypass -File install.ps1 -Yes

param(
    [switch]$Yes
)

Write-Host "Installing xfetch..." -ForegroundColor Cyan

# Records that this binary came from a source checkout. The updater uses it
# to distinguish a path install from `cargo install xfetch-cli`.
function Write-InstallReceipt($Method, $Path, $Version, $Source, $SourcePath, $Repo, $Commit) {
    if (-not $env:LOCALAPPDATA) { return }
    try {
        $receiptDir = Join-Path $env:LOCALAPPDATA "xfetch"
        New-Item -ItemType Directory -Path $receiptDir -Force | Out-Null
        $receipt = [ordered]@{
            schema_version = 1
            method         = $Method
            path           = $Path
            installed_at   = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")
        }
        if ($Version)    { $receipt.version = $Version }
        if ($Source)     { $receipt.source = $Source }
        if ($SourcePath) { $receipt.source_path = $SourcePath }
        if ($Repo)       { $receipt.repo = $Repo }
        if ($Commit)     { $receipt.commit = $Commit }
        $json = $receipt | ConvertTo-Json -Compress
        $utf8 = New-Object System.Text.UTF8Encoding -ArgumentList $false
        [System.IO.File]::WriteAllText((Join-Path $receiptDir "install.json"), $json, $utf8)
    } catch {
        Write-Host "Could not write install receipt: $($_.Exception.Message)" -ForegroundColor Yellow
    }
}

# Check for Rust/Cargo
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    if ($Yes -or [Console]::IsInputRedirected) {
        Write-Host "Rust (cargo) is not installed. Installing it via rustup..." -ForegroundColor Cyan
        $installRust = $true
    } else {
        $choice = Read-Host "Rust (cargo) is not installed. Install it now? [Y/n]"
        $installRust = ($choice -ne "n")
    }

    if ($installRust) {
        Write-Host "Downloading rustup..." -ForegroundColor Cyan
        $rustup = "$env:TEMP\rustup-init.exe"
        try {
            # Pick the rustup build matching the CPU architecture
            switch ($env:PROCESSOR_ARCHITECTURE) {
                "AMD64" { $rustupArch = "x86_64-pc-windows-msvc" }
                "ARM64" { $rustupArch = "aarch64-pc-windows-msvc" }
                default {
                    Write-Host "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE" -ForegroundColor Red
                    exit 1
                }
            }
            Invoke-WebRequest -Uri "https://static.rust-lang.org/rustup/dist/$rustupArch/rustup-init.exe" -OutFile $rustup
            Start-Process $rustup -ArgumentList "-y" -NoNewWindow -Wait
            Remove-Item $rustup -Force
            # Refresh PATH
            $env:Path = [Environment]::GetEnvironmentVariable("Path", "User")
            Write-Host "Rust installed successfully." -ForegroundColor Green
        } catch {
            Write-Host "Failed to install Rust. Install it manually from https://rustup.rs/" -ForegroundColor Red
            exit 1
        }
    } else {
        exit 1
    }
}

$RepoUrl = "https://github.com/xfetch-cli/xfetch.git"
$TempDir = Join-Path $env:TEMP "xfetch_install"

# Clean previous temp dir
if (Test-Path $TempDir) {
    Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue
}
New-Item -Path $TempDir -ItemType Directory | Out-Null

# Clone
Write-Host "Cloning repository..." -ForegroundColor Cyan
try {
    git clone --depth 1 $RepoUrl $TempDir
}
catch {
    Write-Host "Failed to clone repository. Ensure git is installed." -ForegroundColor Red
    exit 1
}

# Build and install
Set-Location $TempDir
Write-Host "Building and installing xfetch..." -ForegroundColor Cyan
cargo install --locked --path .

# Make sure ~/.cargo/bin is on the user PATH (cargo install target)
$CargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
$UserPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($UserPath -notlike "*$CargoBin*") {
    [Environment]::SetEnvironmentVariable("Path", "$UserPath;$CargoBin", "User")
    Write-Host "Added $CargoBin to user PATH." -ForegroundColor Green
}

# Record the source build before the temporary clone is removed.
$InstalledVersion = ""
try {
    $InstalledVersion = ((& xfetch --version 2>$null) -replace '^xfetch(?:-cli)?\s+', '').Trim()
} catch {}
$Commit = ""
try {
    $Commit = (git -C $TempDir rev-parse HEAD 2>$null).Trim()
} catch {}
Write-InstallReceipt -Method "source" -Path (Join-Path $CargoBin "xfetch.exe") -Version $InstalledVersion -Source "install.ps1" -Repo $RepoUrl -Commit $Commit

# Setup Config
$ConfigDir = Join-Path $env:APPDATA "xfetch"
Write-Host "Setting up default config..." -ForegroundColor Cyan
if (-not (Test-Path $ConfigDir)) {
    New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
}

$ConfigFile = Join-Path $ConfigDir "config.jsonc"
if (-not (Test-Path $ConfigFile)) {
    # Generate the first config with the freshly installed binary
    # (adds the distro ASCII logo when online; falls back offline).
    & xfetch --gen-config 2>$null
    if (Test-Path $ConfigFile) {
        Write-Host "Generated config at $ConfigFile" -ForegroundColor Green
    } else {
        Write-Host "Could not generate config; skipping." -ForegroundColor Yellow
    }
}

# Cleanup
Set-Location $env:USERPROFILE
Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue

Write-Host "Installation complete! Run 'xfetch' to start." -ForegroundColor Green
