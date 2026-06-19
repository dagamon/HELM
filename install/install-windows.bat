@echo off
setlocal

:: HELM Windows Service Installer (Rust backend, via NSSM)
:: Requires: NSSM (https://nssm.cc/), Rust toolchain (rustup), Node.js >= 20

set SERVICE_NAME=HELM
set HELM_DIR=%~dp0..
for %%I in ("%HELM_DIR%") do set HELM_DIR=%%~fI

:: Find cargo
where cargo >nul 2>&1 || (
    echo ERROR: cargo not found. Install Rust via https://rustup.rs/
    exit /b 1
)

:: Find npm
where npm >nul 2>&1 || (
    echo ERROR: npm not found. Install Node.js ^>= 20 from https://nodejs.org/
    exit /b 1
)

:: Find NSSM
where nssm >nul 2>&1 || (
    if exist "%~dp0nssm.exe" (
        set "PATH=%~dp0;%PATH%"
    ) else (
        echo ERROR: nssm.exe not found. Download from https://nssm.cc/
        exit /b 1
    )
)

echo.
echo === HELM Service Installer (Rust) ===
echo HELM Dir: %HELM_DIR%
echo.

:: Stop existing service so cargo can overwrite the running binary
nssm status %SERVICE_NAME% >nul 2>&1 && (
    echo Stopping existing %SERVICE_NAME% service...
    nssm stop %SERVICE_NAME% >nul 2>&1
)

:: Build Rust backend
echo Building helm.exe (cargo build --release)...
pushd "%HELM_DIR%\helm-rs"
cargo build --release -p helm-bin
if errorlevel 1 (
    popd
    echo ERROR: cargo build failed.
    exit /b 1
)
popd

set BIN_SRC=%HELM_DIR%\helm-rs\target\release\helm.exe
if not exist "%BIN_SRC%" (
    echo ERROR: built binary missing at %BIN_SRC%
    exit /b 1
)

:: Build frontend if static/ empty
if not exist "%HELM_DIR%\static\index.html" (
    echo Building frontend...
    pushd "%HELM_DIR%\client"
    call npm install --silent
    call npm run build
    if errorlevel 1 (
        popd
        echo ERROR: frontend build failed.
        exit /b 1
    )
    popd
) else (
    echo Frontend already built, skipping.
)

:: Ensure data dir exists
if not exist "%HELM_DIR%\data" mkdir "%HELM_DIR%\data"

:: Scaffold .env
if not exist "%HELM_DIR%\.env" (
    > "%HELM_DIR%\.env" echo HOST=127.0.0.1
    >> "%HELM_DIR%\.env" echo PORT=7010
    >> "%HELM_DIR%\.env" echo DB_PATH=./data/dashboard.db
    >> "%HELM_DIR%\.env" echo DASHBOARD_PIN=
    echo Created default .env
)

:: Remove existing service if present (was stopped above)
nssm status %SERVICE_NAME% >nul 2>&1 && (
    echo Removing existing %SERVICE_NAME% service...
    nssm remove %SERVICE_NAME% confirm
)

:: Install service pointed at the Rust binary
echo Installing %SERVICE_NAME% service...
nssm install %SERVICE_NAME% "%BIN_SRC%"
nssm set %SERVICE_NAME% AppDirectory "%HELM_DIR%"
nssm set %SERVICE_NAME% Start SERVICE_AUTO_START
nssm set %SERVICE_NAME% AppStdout "%HELM_DIR%\data\helm-stdout.log"
nssm set %SERVICE_NAME% AppStderr "%HELM_DIR%\data\helm-stderr.log"
nssm set %SERVICE_NAME% AppRotateFiles 1
nssm set %SERVICE_NAME% AppRotateBytes 1048576
nssm set %SERVICE_NAME% Description "HELM Service Control Dashboard (Rust)"

:: Start service
echo Starting %SERVICE_NAME%...
nssm start %SERVICE_NAME%

echo.
echo === Done! ===
echo HELM running at http://127.0.0.1:7010
echo Service name: %SERVICE_NAME%
echo.
echo Management:
echo   nssm status HELM
echo   nssm stop HELM
echo   nssm start HELM
echo   nssm restart HELM
echo   nssm remove HELM confirm
echo.

endlocal
