# Framework Control - MSI Installer Builder
# Creates a Windows Installer (MSI) with service installation
param(
    [switch]$Clean
)
$ErrorActionPreference = "Stop"
Write-Host ""
Write-Host "======================================" -ForegroundColor Cyan
Write-Host "Framework Control - MSI Builder" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$ServiceDir = Join-Path $ProjectRoot "service"
$WixDir = Join-Path $ServiceDir "wix"
$OutputDir = Join-Path $ProjectRoot "output"
# Clean if requested
if ($Clean) {
    Write-Host "Cleaning previous builds..." -ForegroundColor Yellow
    if (Test-Path $OutputDir) {
        Remove-Item $OutputDir -Recurse -Force
    }
    if (Test-Path "$ServiceDir\target") {
        Remove-Item "$ServiceDir\target" -Recurse -Force
    }
    Write-Host "[OK] Clean complete" -ForegroundColor Green
    Write-Host ""
}
# Check for WiX Toolset
Write-Host "[1/5] Checking for WiX Toolset..." -ForegroundColor Cyan
$wixPath = $null
$possiblePaths = @(
    "C:\Program Files (x86)\WiX Toolset v3.11\bin",
    "C:\Program Files (x86)\WiX Toolset v3.14\bin",
    "$env:WIX\bin"
)
foreach ($path in $possiblePaths) {
    if (Test-Path "$path\candle.exe") {
        $wixPath = $path
        break
    }
}
if (-not $wixPath) {
    Write-Host "[ERROR] WiX Toolset not found!" -ForegroundColor Red
    Write-Host ""
    Write-Host "Please install WiX Toolset from:" -ForegroundColor Yellow
    Write-Host "https://wixtoolset.org/releases/" -ForegroundColor Cyan
    Write-Host ""
    Write-Host "Or install via command:" -ForegroundColor Yellow
    Write-Host "winget install WiXToolset.WiX" -ForegroundColor White
    Write-Host ""
    exit 1
}
Write-Host "  [OK] WiX found: $wixPath" -ForegroundColor Green
# Build release version
Write-Host ""
Write-Host "[2/5] Building release version..." -ForegroundColor Cyan
Set-Location $ServiceDir
if (-not (Test-Path "target\release\framework-control.exe")) {
    Write-Host "  Building optimized binary..." -ForegroundColor Yellow
    cargo build --release --quiet
    if ($LASTEXITCODE -ne 0) {
        Write-Host "[ERROR] Build failed!" -ForegroundColor Red
        Set-Location $ProjectRoot
        exit 1
    }
}
$ExePath = "target\release\framework-control.exe"
$ExeSize = [math]::Round((Get-Item $ExePath).Length / 1MB, 2)
Write-Host "  [OK] Release build complete" -ForegroundColor Green
Set-Location $ProjectRoot
# Create WiX directory and files
Write-Host ""
Write-Host "[3/5] Creating WiX configuration..." -ForegroundColor Cyan
if (-not (Test-Path $WixDir)) {
    New-Item -ItemType Directory -Path $WixDir -Force | Out-Null
}
# Generate new GUIDs
$ProductGuid = [guid]::NewGuid().ToString()
$MainExeGuid = [guid]::NewGuid().ToString()
$ConfigDirGuid = [guid]::NewGuid().ToString()
$ReadmeGuid = [guid]::NewGuid().ToString()
$LicenseGuid = [guid]::NewGuid().ToString()
$MenuDirGuid = [guid]::NewGuid().ToString()
$ServiceExePath = Resolve-Path "$ServiceDir\target\release\framework-control.exe"
$ReadmePath = Resolve-Path "$ProjectRoot\README.md"
$LicensePath = Resolve-Path "$ProjectRoot\LICENSE"
# Create WiX source file
$WixSource = @"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
    <Product Id="$ProductGuid" Name="Framework Control" Language="1033" Version="0.4.2.0" Manufacturer="Framework Control" UpgradeCode="PUT-GUID-HERE">
        <Package InstallerVersion="200" Compressed="yes" InstallScope="perMachine" />

        <MajorUpgrade DowngradeErrorMessage="A newer version of [ProductName] is already installed." />
        <MediaTemplate EmbedCab="yes" />

        <Feature Id="ProductFeature" Title="Framework Control" Level="1">
            <ComponentGroupRef Id="ProductComponents" />
            <ComponentRef Id="ApplicationShortcut" />
        </Feature>
    </Product>

    <Fragment>
        <Directory Id="TARGETDIR" Name="SourceDir">
            <Directory Id="ProgramFilesFolder">
                <Directory Id="INSTALLFOLDER" Name="FrameworkControl" />
            </Directory>
            <Directory Id="ProgramMenuFolder">
                <Directory Id="ApplicationProgramsFolder" Name="Framework Control"/>
            </Directory>
        </Directory>
    </Fragment>

    <Fragment>
        <ComponentGroup Id="ProductComponents" Directory="INSTALLFOLDER">
            <Component Id="ProductComponent" Guid="$MainExeGuid">
                <File Source="$ServiceExePath" Id="FrameworkControlEXE" KeyPath="yes" />
                <ServiceInstall Id="ServiceInstaller" Type="ownProcess" Name="FrameworkControl" DisplayName="Framework Control Service" Description="Manages Framework Laptop hardware settings" Start="auto" Account="LocalSystem" ErrorControl="normal" />
                <ServiceControl Id="StartService" Start="install" Stop="both" Remove="uninstall" Name="FrameworkControl" Wait="yes" />
            </Component>
            <Component Id="Readme" Guid="$ReadmeGuid">
                <File Source="$ReadmePath" Id="ReadmeMD" />
            </Component>
            <Component Id="License" Guid="$LicenseGuid">
                <File Source="$LicensePath" Id="LicenseFile" />
            </Component>
        </ComponentGroup>
    </Fragment>

    <Fragment>
        <DirectoryRef Id="ApplicationProgramsFolder">
            <Component Id="ApplicationShortcut" Guid="$MenuDirGuid">
                <Shortcut Id="ApplicationStartMenuShortcut" Name="Framework Control" Description="Control your Framework Laptop" Target="[INSTALLFOLDER]framework-control.exe" WorkingDirectory="INSTALLFOLDER"/>
                <RemoveFolder Id="CleanUpShortCut" Directory="ApplicationProgramsFolder" On="uninstall"/>
                <RegistryValue Root="HKCU" Key="Software\FrameworkControl" Name="installed" Type="integer" Value="1" KeyPath="yes"/>
            </Component>
        </DirectoryRef>
    </Fragment>
</Wix>
"@.Replace("PUT-GUID-HERE", [guid]::NewGuid().ToString())
$WixSource | Out-File "$WixDir\FrameworkControl.wxs" -Encoding UTF8
Write-Host "  [OK] WiX configuration created" -ForegroundColor Green
# Compile WiX
Write-Host ""
Write-Host "[4/5] Compiling installer..." -ForegroundColor Cyan
$env:PATH = "$wixPath;$env:PATH"
Set-Location $WixDir
# Candle (compile)
Write-Host "  Compiling .wxs to .wixobj..." -ForegroundColor Yellow
& candle.exe "FrameworkControl.wxs" -ext WixUtilExtension
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] WiX compilation failed!" -ForegroundColor Red
    Set-Location $ProjectRoot
    exit 1
}
# Light (link)
Write-Host "  Linking to create .msi..." -ForegroundColor Yellow
if (-not (Test-Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null
}
& light.exe "FrameworkControl.wixobj" -ext WixUtilExtension -ext WixUIExtension -out "$OutputDir\FrameworkControl-0.4.2.msi" -sval
if ($LASTEXITCODE -ne 0) {
    Write-Host "[ERROR] WiX linking failed!" -ForegroundColor Red
    Set-Location $ProjectRoot
    exit 1
}
Write-Host "  [OK] MSI created successfully" -ForegroundColor Green
Set-Location $ProjectRoot
# Summary
Write-Host "[5/5] Complete!" -ForegroundColor Cyan
$MsiPath = Resolve-Path "$OutputDir\FrameworkControl-0.4.2.msi"
$MsiSize = [math]::Round((Get-Item $MsiPath).Length / 1MB, 2)
Write-Host ""
Write-Host "========================================" -ForegroundColor Green
Write-Host "MSI Installer Created Successfully!" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
Write-Host ""
Write-Host "Installer Details:" -ForegroundColor Cyan
Write-Host "  Location: $MsiPath" -ForegroundColor White
Write-Host "  Size: $MsiSize MB" -ForegroundColor White
Write-Host "  Version: 0.4.2" -ForegroundColor White
Write-Host ""
Write-Host "Whats Included:" -ForegroundColor Cyan
Write-Host "  [OK] Framework Control executable" -ForegroundColor White
Write-Host "  [OK] Windows Service (auto-start)" -ForegroundColor White
Write-Host "  [OK] Start Menu shortcut" -ForegroundColor White
Write-Host "  [OK] Fan curve service (always running)" -ForegroundColor White
Write-Host ""
Write-Host "To Install:" -ForegroundColor Yellow
Write-Host "  1. Double-click: FrameworkControl-0.4.2.msi" -ForegroundColor White
Write-Host "  2. Follow installation wizard" -ForegroundColor White
Write-Host "  3. Service starts automatically" -ForegroundColor White
Write-Host ""
Write-Host "After Installation:" -ForegroundColor Cyan
Write-Host "  * Service: Running in background" -ForegroundColor White
Write-Host "  * GUI: Windows Key -> Framework Control" -ForegroundColor White
Write-Host "  * Location: C:\Program Files\FrameworkControl\" -ForegroundColor White
Write-Host ""
Write-Host "Opening output folder..." -ForegroundColor Yellow
Start-Process $OutputDir
Write-Host ""