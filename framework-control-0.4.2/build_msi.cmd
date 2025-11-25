@echo off
echo ========================================
echo Framework Control - MSI Build Script
echo ========================================
echo.

REM Initialize Visual Studio environment
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\VsDevCmd.bat" -arch=amd64 -host_arch=amd64

if %ERRORLEVEL% NEQ 0 (
    echo ERROR: Failed to initialize Visual Studio environment
    pause
    exit /b 1
)

echo.
echo [1/5] Building web UI...
cd web
call npm install
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: npm install failed
    pause
    exit /b 1
)

call npm run build
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: web build failed
    pause
    exit /b 1
)

echo.
echo [2/5] Building Rust service...
cd ..\service
cargo build --release
if %ERRORLEVEL% NEQ 0 (
    echo ERROR: cargo build failed
    pause
    exit /b 1
)

echo.
echo [3/5] Configuring MSI build...
set FRAMEWORK_CONTROL_PORT=8090
set FRAMEWORK_CONTROL_TOKEN=framework-control-secure-token-2024
set FRAMEWORK_CONTROL_ALLOWED_ORIGINS=http://127.0.0.1:8090,http://localhost:8090
set FRAMEWORK_CONTROL_UPDATE_REPO=framework-laptop/framework-control

echo Port: %FRAMEWORK_CONTROL_PORT%
echo Token: ***SET***

echo.
echo [4/5] Building MSI installer...

REM Backup and update WinSW XML
copy wix\FrameworkControlService.xml wix\FrameworkControlService.xml.bak

powershell -Command "$xml = Get-Content wix\FrameworkControlService.xml -Raw; $xml = $xml -replace '@FRAMEWORK_CONTROL_ALLOWED_ORIGINS@', '%FRAMEWORK_CONTROL_ALLOWED_ORIGINS%'; $xml = $xml -replace '@FRAMEWORK_CONTROL_TOKEN@', '%FRAMEWORK_CONTROL_TOKEN%'; $xml = $xml -replace '@FRAMEWORK_CONTROL_PORT@', '%FRAMEWORK_CONTROL_PORT%'; $xml = $xml -replace '@FRAMEWORK_CONTROL_UPDATE_REPO@', '%FRAMEWORK_CONTROL_UPDATE_REPO%'; Set-Content wix\FrameworkControlService.xml $xml -Encoding UTF8"

cargo wix --nocapture -v

REM Restore original XML
move /Y wix\FrameworkControlService.xml.bak wix\FrameworkControlService.xml

if %ERRORLEVEL% NEQ 0 (
    echo ERROR: MSI build failed
    pause
    exit /b 1
)

echo.
echo [5/5] Locating MSI package...
for /f "delims=" %%i in ('dir /b /o-d target\wix\*.msi 2^>nul') do set MSI_FILE=%%i

if defined MSI_FILE (
    echo.
    echo ========================================
    echo Build Complete!
    echo ========================================
    echo.
    echo MSI Location: %CD%\target\wix\%MSI_FILE%
    echo.
    echo To install, run as Administrator:
    echo   msiexec /i "%CD%\target\wix\%MSI_FILE%"
    echo.
) else (
    echo ERROR: MSI file not found
    pause
    exit /b 1
)

pause