# Framework Control - MSI Build Script (PowerShell Version)
# This script properly initializes the MSVC environment and builds the project

param(
    [string]$Port = "8090",
    [string]$Token = "framework-control-secure-token-2024"
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Framework Control - MSI Build Script" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# Get the script directory
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $scriptDir

# Step 1: Initialize Visual Studio environment
Write-Host "[1/5] Initializing Visual Studio environment..." -ForegroundColor Yellow
$vsDevCmdPath = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"

if (-not (Test-Path $vsDevCmdPath)) {
    Write-Host "ERROR: Visual Studio Build Tools not found at expected location" -ForegroundColor Red
    Write-Host "Please install Visual Studio Build Tools from:" -ForegroundColor Yellow
    Write-Host "https://visualstudio.microsoft.com/downloads/" -ForegroundColor White
    exit 1
}

# Import VS environment variables into current session
$tempFile = [System.IO.Path]::GetTempFileName()
cmd /c "`"$vsDevCmdPath`" -arch=amd64 -host_arch=amd64 && set > `"$tempFile`""

Get-Content $tempFile | ForEach-Object {
    if ($_ -match '^([^=]+)=(.*)$') {
        $name = $matches[1]
        $value = $matches[2]
        Set-Item -Path "env:$name" -Value $value
    }
}
Remove-Item $tempFile

Write-Host "  Visual Studio environment initialized" -ForegroundColor Green
Write-Host ""

# Step 2: Build web UI
Write-Host "[2/5] Building web UI..." -ForegroundColor Yellow
Set-Location "web"

# Clean previous build artifacts
if (Test-Path "node_modules") {
    Write-Host "  Found existing node_modules, skipping install..." -ForegroundColor Cyan
} else {
    Write-Host "  Installing npm dependencies..." -ForegroundColor Cyan
    npm install --no-audit --no-fund --prefer-offline
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: npm install failed" -ForegroundColor Red
        Write-Host "Trying with clean cache..." -ForegroundColor Yellow
        npm cache clean --force
        npm install --no-audit --no-fund
        if ($LASTEXITCODE -ne 0) {
            Write-Host "ERROR: npm install failed after retry" -ForegroundColor Red
            Set-Location ".."
            exit 1
        }
    }
}

Write-Host "  Building web application..." -ForegroundColor Cyan
npm run build
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Web build failed" -ForegroundColor Red
    Set-Location ".."
    exit 1
}

Write-Host "  Web UI built successfully" -ForegroundColor Green
Set-Location ".."
Write-Host ""

# Step 3: Build Rust service
Write-Host "[3/5] Building Rust service..." -ForegroundColor Yellow
Set-Location "service"

# Check if already built
$exePath = "target\release\framework-control-service.exe"
if (Test-Path $exePath) {
    $lastBuild = (Get-Item $exePath).LastWriteTime
    Write-Host "  Found existing build from $lastBuild" -ForegroundColor Cyan
    Write-Host "  Checking for changes..." -ForegroundColor Cyan
}

Write-Host "  Compiling in release mode (first build may take 10-15 minutes)..." -ForegroundColor Cyan
$buildStart = Get-Date
cargo build --release
if ($LASTEXITCODE -ne 0) {
    Write-Host "ERROR: Cargo build failed" -ForegroundColor Red
    Write-Host "Attempting clean rebuild..." -ForegroundColor Yellow
    cargo clean
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        Write-Host "ERROR: Cargo build failed after clean" -ForegroundColor Red
        Set-Location ".."
        exit 1
    }
}
$buildEnd = Get-Date
$buildTime = ($buildEnd - $buildStart).TotalSeconds
Write-Host "  Service built successfully in $([math]::Round($buildTime, 1)) seconds" -ForegroundColor Green
Write-Host ""

# Step 4: Configure MSI build
Write-Host "[4/5] Configuring MSI build..." -ForegroundColor Yellow

$env:FRAMEWORK_CONTROL_PORT = $Port
$env:FRAMEWORK_CONTROL_TOKEN = $Token
$env:FRAMEWORK_CONTROL_ALLOWED_ORIGINS = "http://127.0.0.1:$Port,http://localhost:$Port"
$env:FRAMEWORK_CONTROL_UPDATE_REPO = "framework-laptop/framework-control"

Write-Host "  Port: $Port" -ForegroundColor Cyan
Write-Host "  Token: ***SET***" -ForegroundColor Cyan
Write-Host "  Allowed Origins: $env:FRAMEWORK_CONTROL_ALLOWED_ORIGINS" -ForegroundColor Cyan
Write-Host ""

# Step 5: Build MSI
Write-Host "[5/5] Building MSI installer..." -ForegroundColor Yellow

$xmlPath = "wix\FrameworkControlService.xml"
$backupPath = "$xmlPath.backup"

# Verify WiX is available
try {
    $null = Get-Command candle.exe -ErrorAction Stop
    Write-Host "  WiX Toolset detected" -ForegroundColor Cyan
} catch {
    Write-Host "ERROR: WiX Toolset not found in PATH" -ForegroundColor Red
    Write-Host "Please install: choco install wixtoolset" -ForegroundColor Yellow
    Set-Location ".."
    exit 1
}

# Backup original XML
if (-not (Test-Path $xmlPath)) {
    Write-Host "ERROR: WinSW XML file not found at $xmlPath" -ForegroundColor Red
    Set-Location ".."
    exit 1
}
Copy-Item $xmlPath $backupPath -Force

try {
    # Update XML with environment variables
    Write-Host "  Configuring service settings..." -ForegroundColor Cyan
    $xml = Get-Content $xmlPath -Raw -Encoding UTF8
    $xml = $xml -replace '@FRAMEWORK_CONTROL_ALLOWED_ORIGINS@', $env:FRAMEWORK_CONTROL_ALLOWED_ORIGINS
    $xml = $xml -replace '@FRAMEWORK_CONTROL_TOKEN@', $env:FRAMEWORK_CONTROL_TOKEN
    $xml = $xml -replace '@FRAMEWORK_CONTROL_PORT@', $env:FRAMEWORK_CONTROL_PORT
    $xml = $xml -replace '@FRAMEWORK_CONTROL_UPDATE_REPO@', $env:FRAMEWORK_CONTROL_UPDATE_REPO
    Set-Content $xmlPath $xml -Encoding UTF8

    Write-Host "  Running cargo-wix..." -ForegroundColor Cyan
    $wixStart = Get-Date
    cargo wix --nocapture -v

    if ($LASTEXITCODE -ne 0) {
        throw "cargo-wix failed with exit code $LASTEXITCODE"
    }

    $wixEnd = Get-Date
    $wixTime = ($wixEnd - $wixStart).TotalSeconds
    Write-Host "  MSI package created successfully in $([math]::Round($wixTime, 1)) seconds" -ForegroundColor Green
}
catch {
    Write-Host "ERROR: MSI build failed - $_" -ForegroundColor Red
    Write-Host ""
    Write-Host "Troubleshooting:" -ForegroundColor Yellow
    Write-Host "  1. Ensure WiX Toolset is installed: choco install wixtoolset" -ForegroundColor White
    Write-Host "  2. Restart PowerShell after installing WiX" -ForegroundColor White
    Write-Host "  3. Check that Visual Studio environment was initialized correctly" -ForegroundColor White
    Set-Location ".."
    exit 1
}
finally {
    # Restore original XML
    if (Test-Path $backupPath) {
        Move-Item $backupPath $xmlPath -Force
    }
}

Set-Location ".."
Write-Host ""

# Step 6: Locate and display MSI info
Write-Host "[6/6] Locating MSI package..." -ForegroundColor Yellow

$msiFiles = Get-ChildItem "service\target\wix\*.msi" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending
if ($msiFiles -and $msiFiles.Count -gt 0) {
    $msiPath = $msiFiles[0].FullName
    $msiSize = [math]::Round($msiFiles[0].Length / 1MB, 2)

    Write-Host ""
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "Build Complete!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "MSI Location: $msiPath" -ForegroundColor Cyan
    Write-Host "MSI Size: $msiSize MB" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "To install the MSI package, run as Administrator:" -ForegroundColor Yellow
    Write-Host "  msiexec /i `"$msiPath`"" -ForegroundColor White
    Write-Host ""
    Write-Host "Or double-click the MSI file in Windows Explorer." -ForegroundColor Yellow
    Write-Host ""
}
else {
    Write-Host "ERROR: MSI file not found in service\target\wix directory" -ForegroundColor Red
    Write-Host "Check build output above for errors." -ForegroundColor Yellow
    exit 1
}