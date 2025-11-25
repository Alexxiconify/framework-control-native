# Framework Control - Super Fast Build Script
# Optimized for minimum build time

param([switch]$Release, [switch]$Run)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Framework Control - Fast Build" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

$start = Get-Date
Set-Location (Split-Path -Parent $MyInvocation.MyCommand.Path)

# Build
Set-Location "service"
Write-Host "Building..." -ForegroundColor Yellow

if ($Release) {
    cargo build --release --quiet
    $exe = "target\release\framework-control.exe"
} else {
    cargo build --quiet
    $exe = "target\debug\framework-control.exe"
}

if ($LASTEXITCODE -ne 0) {
    Write-Host "Build failed!" -ForegroundColor Red
    exit 1
}

$buildTime = ((Get-Date) - $start).TotalSeconds
$size = [math]::Round((Get-Item $exe).Length / 1MB, 2)

Write-Host ""
Write-Host "✓ Built in $([math]::Round($buildTime, 1))s" -ForegroundColor Green
Write-Host "✓ Size: $size MB" -ForegroundColor Green
Write-Host "✓ Location: service\$exe" -ForegroundColor Cyan
Write-Host ""

if ($Run) {
    Write-Host "Starting..." -ForegroundColor Yellow
    & $exe
}