@echo off
REM === Get the full path of the current script ===
set SCRIPT_PATH=%~dp0
set BINARY_PATH=%SCRIPT_PATH%rsnew.exe

REM === Set registry key name for startup ===
set REG_NAME=MyAppOnLogin

REM === Add to HKCU\...\Run (current user startup) ===
reg add "HKCU\Software\Microsoft\Windows\CurrentVersion\Run" /v %REG_NAME% /t REG_SZ /d "%BINARY_PATH%" /f

REM === Run the binary immediately ===
start "" "%BINARY_PATH%"
echo Binary started.