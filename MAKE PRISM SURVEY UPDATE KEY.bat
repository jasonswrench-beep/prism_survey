@echo off
rem Makes the key that signs Prism Survey updates. Run it once, by double-click. It asks for a password twice.
rem The secret key stays in your profile folder (.prismsurvey-update.key); only its public half goes into the program.
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File tools\make-update-key.ps1
echo.
pause
