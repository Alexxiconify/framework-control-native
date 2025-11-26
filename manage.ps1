param(
    [string]$Action = "build", # build, msi, run, clean
    [switch]$Release,
    [switch]$Run
)

$ErrorActionPreference = "Stop"
$ScriptDir = $PSScriptRoot
$ServiceDir = Join-Path $ScriptDir "service"
$OutputDir = Join-Path $ScriptDir "output"

# Ensure output dir exists
if (-not (Test-Path $OutputDir)) { New-Item -ItemType Directory -Path $OutputDir -Force | Out-Null }

function Init-VS {
    Write-Host "Initializing Visual Studio environment..." -ForegroundColor Cyan
    $vsDevCmdPath = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat"
    if (Test-Path $vsDevCmdPath) {
        $tempFile = [System.IO.Path]::GetTempFileName()
        cmd /c "`"$vsDevCmdPath`" -arch=amd64 -host_arch=amd64 && set > `"$tempFile`""
        Get-Content $tempFile | ForEach-Object {
            if ($_ -match '^([^=]+)=(.*)$') { Set-Item -Path "env:$($matches[1])" -Value $matches[2] }
        }
        Remove-Item $tempFile
    } else {
        Write-Host "VS Build Tools not found, using system environment." -ForegroundColor Yellow
    }
}

if ($Action -eq "build" -or $Action -eq "run") {
    Init-VS
    Push-Location $ServiceDir
    Write-Host "Building Rust application..." -ForegroundColor Cyan
    $cargoArgs = @("build")
    if ($Release) { $cargoArgs += "--release" }

    cargo @cargoArgs
    if ($LASTEXITCODE -ne 0) { Write-Error "Build failed"; exit 1 }

    $target = if ($Release) { "release" } else { "debug" }
    $exe = "target\$target\framework-control.exe"

    if (Test-Path $exe) {
        Copy-Item $exe -Destination "$OutputDir\framework-control.exe" -Force
        Write-Host "Build success! Output: $OutputDir\framework-control.exe" -ForegroundColor Green
    } else {
        Write-Error "Executable not found at $exe"
        exit 1
    }
    Pop-Location

    if ($Run) {
        Write-Host "Starting application..." -ForegroundColor Cyan
        & "$OutputDir\framework-control.exe"
    }
}
elseif ($Action -eq "msi") {
    Init-VS
    Push-Location $ServiceDir
    Write-Host "Building Release binary for MSI..." -ForegroundColor Cyan
    cargo build --release
    if ($LASTEXITCODE -ne 0) { Write-Error "Build failed"; exit 1 }

    $WixDir = Join-Path $ServiceDir "wix"
    if (-not (Test-Path $WixDir)) { New-Item -ItemType Directory -Path $WixDir -Force | Out-Null }

    # WiX Toolset lookup (simplified)
    $wixPath = $null
    $possiblePaths = @(
        "C:\Program Files (x86)\WiX Toolset v3.11\bin",
        "$env:WIX\bin"
    )
    foreach ($p in $possiblePaths) { if (Test-Path "$p\candle.exe") { $wixPath = $p; break } }

    if (-not $wixPath) { Write-Error "WiX Toolset not found. Please install it."; exit 1 }
    $env:PATH = "$wixPath;$env:PATH"

    # Generate WXS (simplified from build_msi.ps1)
    $ProductGuid = [guid]::NewGuid().ToString()
    $MainExeGuid = [guid]::NewGuid().ToString()
    $ServiceExePath = Resolve-Path "target\release\framework-control.exe"

    $WixSource = @"
<?xml version="1.0" encoding="UTF-8"?>
<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">
    <Product Id="$ProductGuid" Name="Framework Control Native" Language="1033" Version="0.4.3.0" Manufacturer="Framework Control" UpgradeCode="$( [guid]::NewGuid().ToString() )">
        <Package InstallerVersion="200" Compressed="yes" InstallScope="perMachine" />
        <MajorUpgrade DowngradeErrorMessage="A newer version is already installed." />
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
        </ComponentGroup>
    </Fragment>
    <Fragment>
        <DirectoryRef Id="ApplicationProgramsFolder">
            <Component Id="ApplicationShortcut" Guid="$( [guid]::NewGuid().ToString() )">
                <Shortcut Id="ApplicationStartMenuShortcut" Name="Framework Control" Description="Control your Framework Laptop" Target="[INSTALLFOLDER]framework-control.exe" WorkingDirectory="INSTALLFOLDER"/>
                <RemoveFolder Id="CleanUpShortCut" Directory="ApplicationProgramsFolder" On="uninstall"/>
                <RegistryValue Root="HKCU" Key="Software\FrameworkControl" Name="installed" Type="integer" Value="1" KeyPath="yes"/>
            </Component>
        </DirectoryRef>
    </Fragment>
</Wix>
"@
    $WixSource | Out-File "$WixDir\FrameworkControl.wxs" -Encoding UTF8

    Set-Location $WixDir
    & candle.exe "FrameworkControl.wxs" -ext WixUtilExtension
    & light.exe "FrameworkControl.wixobj" -ext WixUtilExtension -ext WixUIExtension -out "$OutputDir\FrameworkControlNative-0.4.3.msi" -sval

    Write-Host "MSI Created: $OutputDir\FrameworkControlNative-0.4.3.msi" -ForegroundColor Green
    Pop-Location
}
elseif ($Action -eq "clean") {
    Write-Host "Cleaning..." -ForegroundColor Yellow
    if (Test-Path $OutputDir) { Remove-Item $OutputDir -Recurse -Force }
    if (Test-Path "$ServiceDir\target") { Remove-Item "$ServiceDir\target" -Recurse -Force }
    Write-Host "Clean complete." -ForegroundColor Green
}