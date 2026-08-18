# xfetch installer script for Windows
# Usage: powershell -ExecutionPolicy Bypass -File install.ps1
#        powershell -ExecutionPolicy Bypass -File install.ps1 -Yes

param(
    [switch]$Yes
)

Write-Host "Installing xfetch..." -ForegroundColor Cyan

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

# Setup Config
$ConfigDir = Join-Path $env:APPDATA "xfetch"
Write-Host "Setting up default config..." -ForegroundColor Cyan
if (-not (Test-Path $ConfigDir)) {
    New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null
}

$ConfigFile = Join-Path $ConfigDir "config.jsonc"
if (-not (Test-Path $ConfigFile)) {
    Copy-Item "configs\config.jsonc" $ConfigFile
}

# Cleanup
Set-Location $env:USERPROFILE
Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue

Write-Host "Installation complete! Run 'xfetch' to start." -ForegroundColor Green
