# Framework Control - Native GUI Build Script
# Single-file build - No web dependencies needed!

param(
    [switch]$Release,
    [switch]$Run
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Framework Control - Native GUI Build" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

# Initialize Visual Studio environment
Write-Host "[1/2] Initializing build environment..." -ForegroundColor Yellow
$vsDevCmdPath = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"

if (Test-Path $vsDevCmdPath)
    $tempFile = [System.IO.Path]::GetTempFileName()
    cmd /c "`"$vsDevCmdPath`" -arch=amd64 -host_arch=amd64 && set > `"$tempFile`""
    Get-Content $tempFile | ForEach-Object {
        if ($_ -match '^([^=]+)=(.*)$') {
            Set-Item -Path "env:$($matches[1])" -Value $matches[2]
        }
    }
    Remove-Item $tempFile
if ($Release) {
    Write-Host "  Building in RELEASE mode..." -ForegroundColor Cyan
    cargo build --release
    $exePath = "target\release\framework-control.exe"
}
else {
    Write-Host "  Building in DEBUG mode (faster)..." -ForegroundColor Cyan
    cargo build
    $exePath = "target\debug\framework-control.exe"
}

if ($LASTEXITCODE -ne 0) {
    Write-Host ""
    Write-Host "ERROR: Build failed!" -ForegroundColor Red
    Write-Host "Try: cargo clean" -ForegroundColor Yellow
    Set-Location ".."
    exit 1
}

$buildTime = ((Get-Date) - $buildStart).TotalSeconds
Write-Host "  Build completed in $([math]::Round($buildTime, 1))s" -ForegroundColor Green

Set-Location ".."

# Display results
Write-Host "========================================" -ForegroundColor Green
Write-Host "Build Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green

$fullPath = Join-Path (Get-Location) "service\$exePath"
if (Test-Path $fullPath) {
    $size = [math]::Round((Get-Item $fullPath).Length / 1MB, 2)
    Write-Host "Executable: $fullPath" -ForegroundColor Cyan
    Write-Host "Size: $size MB" -ForegroundColor Cyan
    Write-Host ""

    if ($Release) {
        Write-Host "Optimized release build ready!" -ForegroundColor Green
    }
    else {
        Write-Host "Debug build ready (use -Release for optimized build)" -ForegroundColor Yellow
    }
    Write-Host ""

    if ($Run) {
        Write-Host "Starting application..." -ForegroundColor Yellow
        Write-Host ""
        & $fullPath
    }
    else {
        Write-Host "To run: .\service\$exePath" -ForegroundColor White
        Write-Host "Or use: .\build.ps1 -Run" -ForegroundColor White
    }
    
    # Copy to output
    $OutputDir = Join-Path $scriptDir "output"
    if (-not (Test-Path $OutputDir)) {
        New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
    }
    
    $DestName = "framework_fan_control.exe"
    $DestPath = Join-Path $OutputDir $DestName
    
    Copy-Item $fullPath -Destination $DestPath -Force
    Write-Host ""
    Write-Host "Standalone EXE copied to:" -ForegroundColor Cyan
    Write-Host "  $DestPath" -ForegroundColor White
}
else {
    Write-Host "WARNING: Executable not found at expected location" -ForegroundColor Yellow
}