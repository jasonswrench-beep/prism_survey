@echo off
rem Imports every stored scale (.doc, .docx, .txt) in a folder you choose and writes, for each, the .pscale file, a
rem respondent copy and a researcher copy in Word, plus one report. Set the researcher copies beside your originals.
rem Results go to Documents\Prism Survey Import Check, one folder per run. Nothing is changed in the folder you choose.
setlocal
cd /d "%~dp0"

echo Choose the folder of stored scales...
for /f "usebackq delims=" %%F in (`powershell -NoProfile -STA -Command "Add-Type -AssemblyName System.Windows.Forms; $d = New-Object System.Windows.Forms.FolderBrowserDialog; $d.Description = 'Choose a folder of stored scales to check'; if ($d.ShowDialog() -eq 'OK') { $d.SelectedPath }"`) do set "FOLDER=%%F"
if not defined FOLDER (
  echo No folder was chosen.
  pause
  exit /b 1
)

echo.
echo Building Prism Survey's command-line tool (the first time takes a few minutes)...
cargo build --release -q -p prismsurvey-cli
if errorlevel 1 (
  echo The build failed. Rust is needed: https://rustup.rs
  pause
  exit /b 1
)

set "RECURSE="
choice /c YN /m "Include the folders inside it as well"
if errorlevel 2 set "RECURSE=--no-recurse"

echo.
echo Checking "%FOLDER%"...
target\release\prismsurvey.exe check-folder "%FOLDER%" %RECURSE%
if errorlevel 1 (
  pause
  exit /b 1
)
start "" "%USERPROFILE%\Documents\Prism Survey Import Check"
echo.
echo Done. Open the newest folder there and read "Import check report.txt" first.
pause
