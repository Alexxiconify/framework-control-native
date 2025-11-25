# Framework Control - Quick Install Script
# This script checks prerequisites and builds the project

param(
    [int]$Port = 8090,
    [string]$Token = "",
    [switch]$SkipMsi,
    [switch]$Help
)

if ($Help) {
    Write-Host @"
Framework Control - Installation Script

Usage: .\install.ps1 [-Port <port>] [-Token <token>] [-SkipMsi] [-Help]

Parameters:
  -Port      : HTTP port for the service (default: 8090)
  -Token     : Authentication token (optional but recommended)
  -SkipMsi   : Only build executables, don't create MSI
  -Help      : Show this help message

Examples:
  .\install.ps1
  .\install.ps1 -Port 8091 -Token "my-secret-token"
  .\install.ps1 -SkipMsi

"@
    exit 0
}

$ErrorActionPreference = "Stop"
$repoRoot = Split-Path -Parent $PSScriptRoot

Write-Host "================================" -ForegroundColor Cyan
Write-Host "Framework Control - Installation" -ForegroundColor Cyan
Write-Host "================================" -ForegroundColor Cyan
Write-Host ""

# Check prerequisites
Write-Host "[1/7] Checking prerequisites..." -ForegroundColor Yellow

# Check Rust
Write-Host "  Checking Rust installation..." -NoNewline
try {
    $rustVersion = cargo --version 2>&1
    Write-Host " OK ($rustVersion)" -ForegroundColor Green
} catch {
    Write-Host " FAILED" -ForegroundColor Red
    Write-Host ""
    Write-Host "Rust is not installed. Please install it from: https://rustup.rs/" -ForegroundColor Red
    exit 1
}

# Check for MSVC linker (required for Rust on Windows)
Write-Host "  Checking MSVC Build Tools..." -NoNewline
$linkPath = Get-Command link.exe -ErrorAction SilentlyContinue
if ($linkPath -and $linkPath.Source -match "Microsoft Visual Studio") {
    Write-Host " OK" -ForegroundColor Green
} else {
    Write-Host " NOT FOUND" -ForegroundColor Red
    Write-Host ""
    Write-Host "Visual Studio Build Tools are required for compiling Rust on Windows." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Please install one of the following:" -ForegroundColor Yellow
    Write-Host "  1. Visual Studio Build Tools: https://visualstudio.microsoft.com/downloads/" -ForegroundColor White
    Write-Host "     (Select 'Desktop development with C++' during installation)" -ForegroundColor White
    Write-Host "  2. Visual Studio Community: https://visualstudio.microsoft.com/vs/community/" -ForegroundColor White
    Write-Host "     (Select 'Desktop development with C++' during installation)" -ForegroundColor White
    Write-Host ""
    Write-Host "After installation, restart your terminal and run this script again." -ForegroundColor Yellow
    exit 1
}

# Check Node.js
Write-Host "  Checking Node.js installation..." -NoNewline
try {
    $nodeVersion = node --version 2>&1
    if ($LASTEXITCODE -ne 0) { throw }
    Write-Host " OK ($nodeVersion)" -ForegroundColor Green
} catch {
    Write-Host " FAILED" -ForegroundColor Red
    Write-Host ""
    Write-Host "Node.js is not installed. Please install it from: https://nodejs.org/" -ForegroundColor Red
    exit 1
}

# Check npm
Write-Host "  Checking npm installation..." -NoNewline
try {
    $npmVersion = npm --version 2>&1
    if ($LASTEXITCODE -ne 0) { throw }
    Write-Host " OK ($npmVersion)" -ForegroundColor Green
} catch {
    Write-Host " FAILED" -ForegroundColor Red
    Write-Host ""
    Write-Host "npm is not installed. Please install Node.js from: https://nodejs.org/" -ForegroundColor Red
    exit 1
}

if (-not $SkipMsi) {
    # Check cargo-wix
    Write-Host "  Checking cargo-wix..." -NoNewline
    try {
        $wixVersion = cargo wix --version 2>&1
        if ($LASTEXITCODE -ne 0) { throw }
        Write-Host " OK" -ForegroundColor Green
    } catch {
        Write-Host " NOT FOUND" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "  Installing cargo-wix..." -ForegroundColor Yellow
        cargo install cargo-wix
        if ($LASTEXITCODE -ne 0) {
            Write-Host "  Failed to install cargo-wix" -ForegroundColor Red
            exit 1
        }
        Write-Host "  cargo-wix installed successfully" -ForegroundColor Green
    }

    # Check WiX Toolset
    Write-Host "  Checking WiX Toolset..." -NoNewline
    $wixPath = Get-Command candle.exe -ErrorAction SilentlyContinue
    if ($wixPath) {
        Write-Host " OK" -ForegroundColor Green
    } else {
        Write-Host " NOT FOUND" -ForegroundColor Yellow
        Write-Host ""
        Write-Host "  WiX Toolset is required for creating MSI packages." -ForegroundColor Yellow
        Write-Host "  Install it with: choco install wixtoolset" -ForegroundColor Yellow
        Write-Host "  Or download from: https://wixtoolset.org/" -ForegroundColor Yellow
        Write-Host ""
        $response = Read-Host "Continue without creating MSI? (y/n)"
        if ($response -ne "y") {
            exit 1
        }
        $SkipMsi = $true
    }
}

Write-Host ""

# Install web dependencies
Write-Host "[2/7] Installing web dependencies..." -ForegroundColor Yellow
Push-Location "$repoRoot\web"
try {
    npm install
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Failed to install web dependencies" -ForegroundColor Red
        exit 1
    }
    Write-Host "  Web dependencies installed successfully" -ForegroundColor Green
} finally {
    Pop-Location
}

Write-Host ""

# Build web UI
Write-Host "[3/7] Building web UI..." -ForegroundColor Yellow
Push-Location "$repoRoot\web"
try {
    npm run build
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Failed to build web UI" -ForegroundColor Red
        exit 1
    }
    Write-Host "  Web UI built successfully" -ForegroundColor Green
} finally {
    Pop-Location
}

Write-Host ""

# Build service
Write-Host "[4/7] Building service (release mode)..." -ForegroundColor Yellow
Push-Location "$repoRoot\service"
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Failed to build service" -ForegroundColor Red
        exit 1
    }
    Write-Host "  Service built successfully" -ForegroundColor Green
} finally {
    Pop-Location
}

Write-Host ""

if (-not $SkipMsi) {
    # Create environment file for MSI build
    Write-Host "[5/7] Configuring MSI build..." -ForegroundColor Yellow

    if (-not $Token) {
        Write-Host "  Warning: No authentication token specified. Service will run without authentication." -ForegroundColor Yellow
        Write-Host "  It is recommended to set a token for security." -ForegroundColor Yellow
        $Token = ""
    }

    $env:FRAMEWORK_CONTROL_PORT = $Port.ToString()
    $env:FRAMEWORK_CONTROL_TOKEN = $Token
    $env:FRAMEWORK_CONTROL_ALLOWED_ORIGINS = "http://127.0.0.1:$Port,http://localhost:$Port"
    $env:FRAMEWORK_CONTROL_UPDATE_REPO = "framework-laptop/framework-control"  # Update this

    Write-Host "  Port: $Port" -ForegroundColor Cyan
    Write-Host "  Token: $(if($Token){'***SET***'}else{'NOT SET'})" -ForegroundColor Cyan

    Write-Host ""

    # Build MSI
    Write-Host "[6/7] Building MSI installer..." -ForegroundColor Yellow
    Push-Location "$repoRoot\service"
    try {
        # Update WinSW XML with environment variables
        $xmlPath = "wix\FrameworkControlService.xml"
        if (Test-Path $xmlPath) {
            $xml = Get-Content $xmlPath -Raw
            $xml = $xml -replace '@FRAMEWORK_CONTROL_ALLOWED_ORIGINS@', $env:FRAMEWORK_CONTROL_ALLOWED_ORIGINS
            $xml = $xml -replace '@FRAMEWORK_CONTROL_TOKEN@', $env:FRAMEWORK_CONTROL_TOKEN
            $xml = $xml -replace '@FRAMEWORK_CONTROL_PORT@', $env:FRAMEWORK_CONTROL_PORT
            $xml = $xml -replace '@FRAMEWORK_CONTROL_UPDATE_REPO@', $env:FRAMEWORK_CONTROL_UPDATE_REPO

            # Backup original
            $backupPath = "$xmlPath.backup"
            Copy-Item $xmlPath $backupPath -Force

            Set-Content $xmlPath $xml -Encoding UTF8
        }

        cargo wix --nocapture -v

        # Restore original XML
        if (Test-Path "$xmlPath.backup") {
            Move-Item "$xmlPath.backup" $xmlPath -Force
        }

        if ($LASTEXITCODE -ne 0) {
            Write-Host "Failed to build MSI" -ForegroundColor Red
            exit 1
        }
        Write-Host "  MSI built successfully" -ForegroundColor Green
    } finally {
        Pop-Location
    }

    Write-Host ""
    Write-Host "[7/7] Locating MSI package..." -ForegroundColor Yellow
    $msiPath = Get-ChildItem "$repoRoot\service\target\wix\*.msi" | Sort-Object LastWriteTime -Descending | Select-Object -First 1

    if ($msiPath) {
        Write-Host ""
        Write-Host "================================" -ForegroundColor Green
        Write-Host "Build Complete!" -ForegroundColor Green
        Write-Host "================================" -ForegroundColor Green
        Write-Host ""
        Write-Host "MSI Location: $($msiPath.FullName)" -ForegroundColor Cyan
        Write-Host "MSI Size: $([math]::Round($msiPath.Length/1MB, 2)) MB" -ForegroundColor Cyan
        Write-Host ""
        Write-Host "To install, run as Administrator:" -ForegroundColor Yellow
        Write-Host "  msiexec /i `"$($msiPath.FullName)`"" -ForegroundColor White
        Write-Host ""
    } else {
        Write-Host "Warning: MSI file not found in target/wix directory" -ForegroundColor Yellow
    }
} else {
    Write-Host "[5/7] Skipping MSI creation" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "================================" -ForegroundColor Green
    Write-Host "Build Complete!" -ForegroundColor Green
    Write-Host "================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "Executable Location: $repoRoot\service\target\release\framework-control-service.exe" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "To run manually:" -ForegroundColor Yellow
    Write-Host "  `$env:FRAMEWORK_CONTROL_PORT = $Port" -ForegroundColor White
    Write-Host "  `$env:FRAMEWORK_CONTROL_TOKEN = `"your-token`"" -ForegroundColor White
    Write-Host "  .\service\target\release\framework-control-service.exe" -ForegroundColor White
    Write-Host ""
}

Write-Host "For more information, see README.md" -ForegroundColor Cyan