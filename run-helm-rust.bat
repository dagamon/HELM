@echo off
setlocal EnableExtensions

set "ROOT=%~dp0"
if "%ROOT:~-1%"=="\" set "ROOT=%ROOT:~0,-1%"

if not exist "%ROOT%\data" mkdir "%ROOT%\data"

set "DB_SRC=%ROOT%\data\dashboard.db"
set "DB_RS=%ROOT%\data\dashboard-rs.db"

if not exist "%DB_RS%" (
    if exist "%DB_SRC%" (
        echo Creating Rust DB copy: "%DB_RS%"
        copy /Y "%DB_SRC%" "%DB_RS%" >nul
        if errorlevel 1 (
            echo ERROR: failed to create "%DB_RS%".
            exit /b 1
        )
    ) else (
        echo NOTE: "%DB_SRC%" not found. Rust HELM will initialize a new DB at "%DB_RS%".
    )
)

if "%HOST%"=="" set "HOST=127.0.0.1"
if "%PORT%"=="" set "PORT=7011"
if "%DB_PATH%"=="" set "DB_PATH=%DB_RS%"
if "%DASHBOARD_PIN%"=="" set "DASHBOARD_PIN="

where cargo >nul 2>&1
if errorlevel 1 (
    echo ERROR: cargo not found in PATH. Install Rust toolchain first.
    exit /b 1
)

echo.
echo Starting HELM Rust...
echo HOST=%HOST%
echo PORT=%PORT%
echo DB_PATH=%DB_PATH%
echo.

pushd "%ROOT%"
cargo run --manifest-path "%ROOT%\helm-rs\Cargo.toml" -p helm-bin
set "EXIT_CODE=%ERRORLEVEL%"
popd

exit /b %EXIT_CODE%
