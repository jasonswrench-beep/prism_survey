@echo off
rem Runs detection over the benchmark folders, compares the results with the answer keys made in benchmark mode, and
rem writes a plain report to Documents\Prism Survey Benchmark (one line per article, then the totals against the
rem version 1 targets). The articles never leave this computer. Change the folders below if they move.
setlocal
cd /d "%~dp0"
set "F1=D:\Documents\Research Articles\Measures"
set "F2=D:\Documents\Research Articles\AI\AI Measures"
set "F3=D:\Documents\Research Articles\Sex\scales"
set "F4=D:\Documents\Comm Instruments"

echo Building Prism Survey's command-line tool (the first time takes a few minutes)...
cargo build --release -q -p prismsurvey-cli
if errorlevel 1 (
  echo The build failed. Rust is needed: https://rustup.rs
  pause
  exit /b 1
)
echo Running the benchmark. Scanned articles are read by text recognition, which takes a while.
target\release\prismsurvey.exe benchmark "%F1%" "%F2%" "%F3%" "%F4%"
if errorlevel 1 (
  pause
  exit /b 1
)
start "" "%USERPROFILE%\Documents\Prism Survey Benchmark"
pause
