# Framework Control - System Installer
# Installs the application to Program Files and creates Start Menu shortcuts

param(
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"

# Check for admin rights
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Host "ERROR: This installer requires Administrator privileges!" -ForegroundColor Red
    Write-Host "Please run PowerShell as Administrator and try again." -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Right-click PowerShell -> 'Run as Administrator'" -ForegroundColor Cyan
    pause
    exit 1
}

# Configuration
$AppName = "Framework Control"
$AppExeName = "framework-control.exe"
$InstallDir = "$env:ProgramFiles\FrameworkControl"
$StartMenuDir = "$env:ProgramData\Microsoft\Windows\Start Menu\Programs"
$DesktopShortcut = "$env:Public\Desktop\Framework Control.lnk"
$SourceDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$ServiceDir = Join-Path $SourceDir "service"

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "Framework Control - System Installer" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

if ($Uninstall) {
    Write-Host "UNINSTALLING Framework Control..." -ForegroundColor Yellow
    Write-Host ""

    # Stop any running instances
    Get-Process -Name "framework-control" -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 1

    # Remove shortcuts
    if (Test-Path "$StartMenuDir\Framework Control.lnk") {
        Remove-Item "$StartMenuDir\Framework Control.lnk" -Force
        Write-Host "✓ Removed Start Menu shortcut" -ForegroundColor Green
    }

    if (Test-Path $DesktopShortcut) {
        Remove-Item $DesktopShortcut -Force
        Write-Host "✓ Removed Desktop shortcut" -ForegroundColor Green
    }

    # Remove installation directory
    if (Test-Path $InstallDir) {
        Remove-Item $InstallDir -Recurse -Force
        Write-Host "✓ Removed installation directory" -ForegroundColor Green
    }

    Write-Host ""
    Write-Host "Framework Control has been uninstalled." -ForegroundColor Green
    Write-Host ""
    pause
    exit 0
}

# INSTALL MODE
Write-Host "Installing Framework Control..." -ForegroundColor Yellow
Write-Host ""

# Step 1: Build release version
Write-Host "[1/5] Building release version..." -ForegroundColor Cyan
Set-Location $ServiceDir

if (-not (Test-Path "target\release\$AppExeName")) {
    Write-Host "  Building optimized release binary..." -ForegroundColor Yellow
    cargo build --release --quiet

    if ($LASTEXITCODE -ne 0) {
        Write-Host ""
        Write-Host "ERROR: Build failed!" -ForegroundColor Red
        Set-Location $SourceDir
        pause
        exit 1
    }
}

$ExePath = "target\release\$AppExeName"
if (-not (Test-Path $ExePath)) {
    Write-Host "ERROR: Executable not found at $ExePath" -ForegroundColor Red
    Set-Location $SourceDir
    pause
    exit 1
}

$ExeSize = [math]::Round((Get-Item $ExePath).Length / 1MB, 2)
Write-Host "  ✓ Built successfully ($ExeSize MB)" -ForegroundColor Green

Set-Location $SourceDir

# Step 2: Create installation directory
Write-Host ""
Write-Host "[2/5] Creating installation directory..." -ForegroundColor Cyan
if (Test-Path $InstallDir) {
    Write-Host "  Removing old installation..." -ForegroundColor Yellow
    Get-Process -Name "framework-control" -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 1
    Remove-Item $InstallDir -Recurse -Force
}

New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
Write-Host "  ✓ Created: $InstallDir" -ForegroundColor Green

# Step 3: Copy files
Write-Host ""
Write-Host "[3/5] Installing files..." -ForegroundColor Cyan
Copy-Item "$ServiceDir\target\release\$AppExeName" "$InstallDir\" -Force
Copy-Item "$SourceDir\README.md" "$InstallDir\" -Force -ErrorAction SilentlyContinue
Copy-Item "$SourceDir\LICENSE" "$InstallDir\" -Force -ErrorAction SilentlyContinue
Write-Host "  ✓ Copied application files" -ForegroundColor Green

# Create config directory
$ConfigDir = "$InstallDir\config"
New-Item -ItemType Directory -Path $ConfigDir -Force | Out-Null

# Create default config
$DefaultConfig = @{
    fan = @{
        mode = "auto"
        curve = @(
            @(40, 20),
            @(50, 30),
            @(60, 40),
            @(70, 60),
            @(80, 80),
            @(90, 100)
        )
    }
    power = @{
        tdp_watts = 15
        thermal_limit = 80
    }
    battery = @{
        charge_limit = 80
    }
}

$DefaultConfig | ConvertTo-Json -Depth 10 | Set-Content "$ConfigDir\config.json" -Force
Write-Host "  ✓ Created default configuration" -ForegroundColor Green

# Step 4: Create Start Menu shortcut
Write-Host ""
Write-Host "[4/5] Creating shortcuts..." -ForegroundColor Cyan

$WshShell = New-Object -ComObject WScript.Shell

# Start Menu shortcut
$StartMenuShortcut = $WshShell.CreateShortcut("$StartMenuDir\Framework Control.lnk")
$StartMenuShortcut.TargetPath = "$InstallDir\$AppExeName"
$StartMenuShortcut.WorkingDirectory = $InstallDir
$StartMenuShortcut.Description = "Framework laptop control with fan curve editor"
$StartMenuShortcut.Save()
Write-Host "  ✓ Created Start Menu shortcut" -ForegroundColor Green

# Desktop shortcut (optional)
$CreateDesktop = Read-Host "  Create Desktop shortcut? (Y/n)"
if ($CreateDesktop -ne "n" -and $CreateDesktop -ne "N") {
    $DesktopShortcut = $WshShell.CreateShortcut("$env:Public\Desktop\Framework Control.lnk")
    $DesktopShortcut.TargetPath = "$InstallDir\$AppExeName"
    $DesktopShortcut.WorkingDirectory = $InstallDir
    $DesktopShortcut.Description = "Framework laptop control with fan curve editor"
    $DesktopShortcut.Save()
    Write-Host "  ✓ Created Desktop shortcut" -ForegroundColor Green
}

# Step 5: Add to PATH (optional)
Write-Host ""
Write-Host "[5/5] System integration..." -ForegroundColor Cyan

$AddToPath = Read-Host "  Add to system PATH? (Y/n)"
if ($AddToPath -ne "n" -and $AddToPath -ne "N") {
    $CurrentPath = [Environment]::GetEnvironmentVariable("Path", "Machine")
    if ($CurrentPath -notlike "*$InstallDir*") {
        [Environment]::SetEnvironmentVariable("Path", "$CurrentPath;$InstallDir", "Machine")
        Write-Host "  ✓ Added to system PATH" -ForegroundColor Green
        Write-Host "  (Restart your terminal to use 'framework-control' command)" -ForegroundColor Yellow
    } else {
        Write-Host "  ✓ Already in system PATH" -ForegroundColor Green
    }
}

# Summary
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "Installation Complete!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Installation Details:" -ForegroundColor Cyan
Write-Host "  Location: $InstallDir" -ForegroundColor White
Write-Host "  Size: $ExeSize MB" -ForegroundColor White
Write-Host "  Executable: $AppExeName" -ForegroundColor White
Write-Host ""
Write-Host "How to Launch:" -ForegroundColor Cyan
Write-Host "  1. Press Windows key and type 'Framework Control'" -ForegroundColor White
Write-Host "  2. Click the app in Start Menu" -ForegroundColor White
Write-Host "  3. Or run: $InstallDir\$AppExeName" -ForegroundColor White
Write-Host ""
Write-Host "Configuration:" -ForegroundColor Cyan
Write-Host "  Config file: $ConfigDir\config.json" -ForegroundColor White
Write-Host ""
Write-Host "To Uninstall:" -ForegroundColor Cyan
Write-Host "  Run: .\install_system.ps1 -Uninstall" -ForegroundColor White
Write-Host ""

$Launch = Read-Host "Launch Framework Control now? (Y/n)"
if ($Launch -ne "n" -and $Launch -ne "N") {
    Start-Process "$InstallDir\$AppExeName"
    Write-Host ""
    Write-Host "Framework Control is starting..." -ForegroundColor Green
}

Write-Host ""
Write-Host "Enjoy your Framework laptop control center! 🚀" -ForegroundColor Green
Write-Host ""