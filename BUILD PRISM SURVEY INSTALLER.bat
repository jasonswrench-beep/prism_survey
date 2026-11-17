@echo off
rem Builds the Windows installer (unsigned) into app\src-tauri\target\release\bundle\nsis, with its SHA-256 beside it.
rem The first build takes a while. For a signed installer see the header of app\BUILD-INSTALLER.ps1.
setlocal
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0app\BUILD-INSTALLER.ps1"
if errorlevel 1 (
  echo.
  echo The installer was not built; the messages above say why.
  pause
  exit /b 1
)
echo.
echo The installer is in app\src-tauri\target\release\bundle\nsis.
pause
