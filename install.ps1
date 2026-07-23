# xfetch installer script for Windows

Write-Host "Installing xfetch..." -ForegroundColor Cyan

# Check for Rust/Cargo
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    $choice = Read-Host "Rust (cargo) is not installed. Install it now? [Y/n]"
    if ($choice -ne "n") {
        Write-Host "Downloading rustup..." -ForegroundColor Cyan
        $rustup = "$env:TEMP\rustup-init.exe"
        try {
            Invoke-WebRequest -Uri "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe" -OutFile $rustup
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

# Install
Set-Location $TempDir
Write-Host "Building and installing xfetch..." -ForegroundColor Cyan
cargo install --path .

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

# Copy logos
$LogosDir = Join-Path $ConfigDir "logos"
if (-not (Test-Path $LogosDir)) {
    New-Item -ItemType Directory -Path $LogosDir -Force | Out-Null
}
Copy-Item "logos\*" $LogosDir -Recurse -Force

# Cleanup
Set-Location $env:USERPROFILE
Remove-Item -Path $TempDir -Recurse -Force -ErrorAction SilentlyContinue

Write-Host "Installation complete! Run 'xfetch' to start." -ForegroundColor Green
