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
