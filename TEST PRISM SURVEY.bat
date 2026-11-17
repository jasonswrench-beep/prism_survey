@echo off
rem Runs every automatic check: the engine's tests, then the interface's type check and build.
setlocal
cd /d "%~dp0"

echo Engine tests...
cargo test --workspace
if errorlevel 1 goto failed

echo.
echo Interface type check and build...
pushd app\ui
if not exist node_modules call npm ci
call npm run build
if errorlevel 1 (
  popd
  goto failed
)
popd

echo.
echo All checks passed.
pause
exit /b 0

:failed
echo.
echo Something failed; the messages above say what.
pause
exit /b 1
