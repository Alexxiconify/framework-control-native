# Framework Control - Quick Build Script (No MSI)
# Faster build for testing - skips MSI creation

param(
    [string]$Port = "8090",
    [string]$Token = "framework-control-secure-token-2024"
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Framework Control - Quick Build" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

# Initialize Visual Studio environment
Write-Host "[1/3] Initializing Visual Studio environment..." -ForegroundColor Yellow
$vsDevCmdPath = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"

if (-not (Test-Path $vsDevCmdPath)) {
    Write-Host "ERROR: Visual Studio Build Tools not found" -ForegroundColor Red
    exit 1
}

$tempFile = [System.IO.Path]::GetTempFileName()
cmd /c "`"$vsDevCmdPath`" -arch=amd64 -host_arch=amd64 && set > `"$tempFile`""
Get-Content $tempFile | ForEach-Object {
    if ($_ -match '^([^=]+)=(.*)$') {
        Set-Item -Path "env:$($matches[1])" -Value $matches[2]
    }
}
Remove-Item $tempFile
Write-Host "  Done" -ForegroundColor Green
Write-Host ""

# Build web UI
Write-Host "[2/3] Building web UI..." -ForegroundColor Yellow
Set-Location "web"
if (-not (Test-Path "node_modules")) {
    npm install --no-audit --no-fund --prefer-offline
}
npm run build
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Web build failed" -ForegroundColor Red
    Set-Location ".."
    exit 1
}
Write-Host "  Done" -ForegroundColor Green
Set-Location ".."
Write-Host ""

# Build Rust service
Write-Host "[3/3] Building Rust service..." -ForegroundColor Yellow
Set-Location "service"
$buildStart = Get-Date
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Cargo build failed" -ForegroundColor Red
    Set-Location ".."
    exit 1
}
$buildTime = ((Get-Date) - $buildStart).TotalSeconds
Write-Host "  Done in $([math]::Round($buildTime, 1))s" -ForegroundColor Green
Set-Location ".."
Write-Host ""

$exePath = "service\target\release\framework-control-service.exe"
Write-Host "========================================" -ForegroundColor Green
Write-Host "Build Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Executable: $exePath" -ForegroundColor Cyan
Write-Host ""
Write-Host "To run manually:" -ForegroundColor Yellow
Write-Host "  `$env:FRAMEWORK_CONTROL_PORT = `"$Port`"" -ForegroundColor White
Write-Host "  `$env:FRAMEWORK_CONTROL_TOKEN = `"$Token`"" -ForegroundColor White
Write-Host "  .\$exePath" -ForegroundColor White
Write-Host ""