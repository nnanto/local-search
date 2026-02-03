@echo off
setlocal enabledelayedexpansion

:: ============================================================================
:: localsearch Installation Script for Windows (Batch File)
:: ============================================================================

:: Default parameters
set "INSTALL_DIR=%LOCALAPPDATA%\Programs\localsearch"
set "GITHUB_REPO=nnanto/localsearch"
set "SHOW_HELP=0"

:: Parse command line arguments
:parse_args
if "%~1"=="" goto end_parse
if /i "%~1"=="-h" set "SHOW_HELP=1"
if /i "%~1"=="--help" set "SHOW_HELP=1"
if /i "%~1"=="/?" set "SHOW_HELP=1"
if /i "%~1"=="-InstallDir" (
    set "INSTALL_DIR=%~2"
    shift
)
if /i "%~1"=="-GitHubRepo" (
    set "GITHUB_REPO=%~2"
    shift
)
shift
goto parse_args
:end_parse

:: Show help if requested
if "%SHOW_HELP%"=="1" (
    call :show_help
    exit /b 0
)

:: Check for administrator privileges
call :check_admin
if errorlevel 1 (
    echo [WARN] Not running as Administrator. Will attempt to add to system PATH anyway.
    echo [WARN] If PATH modification fails, please run as Administrator.
    echo.
)

:: Main installation
echo [INFO] Installing localsearch CLI tool...
echo [INFO] Installation directory: %INSTALL_DIR%
echo [INFO] GitHub repository: %GITHUB_REPO%
echo.

call :install_localsearch
if errorlevel 1 (
    echo [ERROR] Installation failed!
    exit /b 1
)

echo [INFO] Installation completed successfully!
exit /b 0

:: ============================================================================
:: Functions
:: ============================================================================

:show_help
echo localsearch Installation Script for Windows
echo.
echo Usage: install.bat [OPTIONS]
echo.
echo Options:
echo   -InstallDir DIR     Installation directory (default: %%LOCALAPPDATA%%\Programs\localsearch)
echo   -GitHubRepo REPO    GitHub repository (default: nnanto/localsearch)
echo   -h, --help, /?      Show this help message
echo.
echo Examples:
echo   install.bat
echo   install.bat -InstallDir "C:\Tools\localsearch"
echo.
echo Note: This script adds to the SYSTEM PATH (global). Administrator privileges are recommended.
exit /b 0

:check_admin
net session >nul 2>&1
if errorlevel 1 (
    exit /b 1
)
exit /b 0

:install_localsearch
set "ARCHIVE_NAME=localsearch-windows-x86_64.zip"
set "DOWNLOAD_URL=https://github.com/%GITHUB_REPO%/releases/latest/download/%ARCHIVE_NAME%"

echo [INFO] Download URL: %DOWNLOAD_URL%

:: Create temporary directory
set "TMP_DIR=%TEMP%\localsearch_%RANDOM%%RANDOM%"
mkdir "%TMP_DIR%" 2>nul

set "ARCHIVE_PATH=%TMP_DIR%\%ARCHIVE_NAME%"

:: Download archive
echo [INFO] Downloading localsearch...
curl -L -o "%ARCHIVE_PATH%" "%DOWNLOAD_URL%" --silent --show-error --fail
if errorlevel 1 (
    echo [ERROR] Failed to download localsearch
    call :cleanup "%TMP_DIR%"
    exit /b 1
)

:: Extract archive using PowerShell (compatible with older Windows versions)
echo [INFO] Extracting archive...
powershell -NoProfile -ExecutionPolicy Bypass -Command "Expand-Archive -Path '%ARCHIVE_PATH%' -DestinationPath '%TMP_DIR%' -Force"
if errorlevel 1 (
    echo [ERROR] Failed to extract archive
    call :cleanup "%TMP_DIR%"
    exit /b 1
)

:: Create install directory if it doesn't exist
if not exist "%INSTALL_DIR%" (
    echo [INFO] Creating installation directory: %INSTALL_DIR%
    mkdir "%INSTALL_DIR%"
)

:: Copy binary
set "BINARY_PATH=%TMP_DIR%\localsearch.exe"
set "TARGET_PATH=%INSTALL_DIR%\localsearch.exe"

if not exist "%BINARY_PATH%" (
    echo [ERROR] Binary not found in extracted archive
    call :cleanup "%TMP_DIR%"
    exit /b 1
)

echo [INFO] Installing to %INSTALL_DIR%...
copy /Y "%BINARY_PATH%" "%TARGET_PATH%" >nul
if errorlevel 1 (
    echo [ERROR] Failed to copy binary
    call :cleanup "%TMP_DIR%"
    exit /b 1
)

:: Add to PATH
call :add_to_path "%INSTALL_DIR%"

echo [INFO] localsearch installed successfully!
echo [INFO] Binary location: %TARGET_PATH%
echo [INFO] Added to SYSTEM PATH (global - available to all users)
echo [INFO] Try running: localsearch --help
echo [INFO] You may need to restart your terminal for PATH changes to take effect.

:: Cleanup
call :cleanup "%TMP_DIR%"
exit /b 0

:add_to_path
set "DIR_TO_ADD=%~1"

:: Check if directory is already in PATH
echo %PATH% | find /i "%DIR_TO_ADD%" >nul
if not errorlevel 1 (
    echo [INFO] %DIR_TO_ADD% is already in PATH.
    exit /b 0
)

echo [INFO] Adding %DIR_TO_ADD% to SYSTEM PATH (global)...

:: Get current system PATH
for /f "skip=2 tokens=3*" %%A in ('reg query "HKLM\SYSTEM\CurrentControlSet\Control\Session Manager\Environment" /v PATH 2^>nul') do set "SYSTEM_PATH=%%A %%B"

:: Remove trailing space if exists
if defined SYSTEM_PATH (
    set "SYSTEM_PATH=!SYSTEM_PATH:~0,-1!"
)

:: Add new directory to system PATH
if "!SYSTEM_PATH!"=="" (
    setx PATH "%DIR_TO_ADD%" /M >nul 2>&1
) else (
    setx PATH "!SYSTEM_PATH!;%DIR_TO_ADD!" /M >nul 2>&1
)

if errorlevel 1 (
    echo [ERROR] Failed to add to system PATH. Please run as Administrator or add manually: %DIR_TO_ADD%
    echo [INFO] To add manually, add this directory to System Environment Variables.
    exit /b 1
)

:: Update PATH for current session
set "PATH=%PATH%;%DIR_TO_ADD%"

echo [INFO] Added to SYSTEM PATH successfully!
echo [INFO] Please restart your terminal for changes to take effect.
exit /b 0

:cleanup
set "CLEANUP_DIR=%~1"
if exist "%CLEANUP_DIR%" (
    rd /s /q "%CLEANUP_DIR%" 2>nul
)
exit /b 0
 
