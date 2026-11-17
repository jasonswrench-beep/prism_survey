@echo off
rem Starts the Prism Survey window from the source (development build). BUILD PRISM SURVEY INSTALLER.bat makes the
rem installer. The first start takes a few minutes while the window's code is compiled.
setlocal
cd /d "%~dp0app\ui"
if not exist node_modules call npm ci
call npm run tauri dev
if errorlevel 1 pause
