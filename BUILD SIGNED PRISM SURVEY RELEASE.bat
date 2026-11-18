@echo off
rem Builds the signed release of Prism Survey: the installer, code-signed and timestamped, then signed with the update key
rem so the Help tab's Check for updates offers it, with latest.json (and previous.json) beside it. docs\UPDATES.md has the steps.
rem It asks for the update key's password. It needs the signing settings (OPENSTAGE_SIGN_*) set in Windows, and builds only
rem from a tagged commit with nothing uncommitted.
setlocal
cd /d "%~dp0"
if not defined OPENSTAGE_SIGN_TOOLS (
  echo OPENSTAGE_SIGN_TOOLS is not set: it names the folder holding the suite's sign_windows.py and check_signatures.py.
  echo Set it in Windows ^(Edit environment variables for your account^), then run this again.
  pause
  exit /b 1
)
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0app\BUILD-INSTALLER.ps1" -Sign -Updates
if errorlevel 1 (
  echo.
  echo The release was not built; the messages above say why.
  pause
  exit /b 1
)
echo.
echo The signed installer, its SHA-256 and latest.json are in app\src-tauri\target\release\bundle\nsis.
pause
