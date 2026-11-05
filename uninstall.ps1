# xfetch uninstaller script for Windows
# Usage: powershell -ExecutionPolicy Bypass -File uninstall.ps1
#        powershell -ExecutionPolicy Bypass -File uninstall.ps1 -Purge

param(
    [switch]$Purge,
    [switch]$Yes
)

Write-Host "Uninstalling xfetch..." -ForegroundColor Cyan

# Confirm
if (-not $Yes) {
    if ([Console]::IsInputRedirected) {
        Write-Host "Re-run with -Yes to uninstall without confirmation." -ForegroundColor Yellow
        exit 1
    }
    $choice = Read-Host "This will remove xfetch from your system. Continue? [y/N]"
    if ($choice -ne "y" -and $choice -ne "Y" -and $choice -ne "yes") {
        Write-Host "Aborted." -ForegroundColor Yellow
        exit 1
    }
}

$removedAny = $false

# Remove binary installed by 'cargo install'
$CargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
$Binary = Join-Path $CargoBin "xfetch.exe"
if (Test-Path $Binary) {
    Remove-Item $Binary -Force
    Write-Host "Removed binary: $Binary" -ForegroundColor Green
    $removedAny = $true
} else {
    Write-Host "Binary not found at $Binary" -ForegroundColor Yellow
}

# Remove binary installed by the prebuilt installer
$PrebuiltBin = Join-Path $env:LOCALAPPDATA "Programs\xfetch\bin"
$PrebuiltBinary = Join-Path $PrebuiltBin "xfetch.exe"
if (Test-Path $PrebuiltBinary) {
    Remove-Item $PrebuiltBinary -Force
    Write-Host "Removed binary: $PrebuiltBinary" -ForegroundColor Green
    $removedAny = $true
    $PrebuiltRoot = Join-Path $env:LOCALAPPDATA "Programs\xfetch"
    if ((Test-Path $PrebuiltRoot) -and -not (Get-ChildItem $PrebuiltRoot -Recurse -File)) {
        Remove-Item $PrebuiltRoot -Recurse -Force
    }
}

# Remove the install receipt written by the installers.
if ($env:LOCALAPPDATA) {
    $Receipt = Join-Path $env:LOCALAPPDATA "xfetch\install.json"
    if (Test-Path $Receipt) {
        Remove-Item $Receipt -Force
        Write-Host "Removed install receipt: $Receipt" -ForegroundColor Green
        $ReceiptDir = Split-Path $Receipt -Parent
        if ((Test-Path $ReceiptDir) -and -not (Get-ChildItem $ReceiptDir -Recurse -File)) {
            Remove-Item $ReceiptDir -Recurse -Force
        }
    }
}

# Remove config (only with -Purge)
$ConfigDir = Join-Path $env:APPDATA "xfetch"
if ($Purge) {
    if (Test-Path $ConfigDir) {
        Remove-Item $ConfigDir -Recurse -Force
        Write-Host "Removed config directory: $ConfigDir" -ForegroundColor Green
        $removedAny = $true
    } else {
        Write-Host "Config directory not found at $ConfigDir" -ForegroundColor Yellow
    }
} else {
    if (Test-Path $ConfigDir) {
        Write-Host "Config directory preserved: $ConfigDir" -ForegroundColor Yellow
        Write-Host "  To remove it later: Remove-Item -Recurse -Force '$ConfigDir'" -ForegroundColor Yellow
    }
}

if (-not $removedAny) {
    Write-Host "xfetch does not appear to be installed." -ForegroundColor Red
    exit 1
}

Write-Host "Uninstall complete." -ForegroundColor Green
