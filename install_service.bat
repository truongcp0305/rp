@echo off
REM install_service.bat

REM Check for admin privileges
net session >nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    echo Administrator privileges required!
    pause
    exit /b 1
)

echo Stopping existing process...
taskkill /F /IM rsnew.exe 2>NUL
if %ERRORLEVEL% EQU 0 (
    echo Process stopped successfully
) else (
    echo No running process found
)

@REM echo Creating required folders...
@REM if not exist ".\dbscan" (
@REM     mkdir "dbscan"
@REM     if %ERRORLEVEL% NEQ 0 (
@REM         echo Failed to create System32\dbscan folder!
@REM     )
@REM )

@REM if not exist ".\Log" (
@REM     mkdir ".\Log"
@REM     if %ERRORLEVEL% NEQ 0 (
@REM         echo Failed to create Temp folder!
@REM     )
@REM )

echo Removing existing service...
sc.exe delete AVS 2>NUL
if %ERRORLEVEL% EQU 0 (
    echo Service removed successfully
) else (
    echo No existing service found
)

echo Current directory: %~dp0

echo Creating service...
sc.exe create AVS binPath= "\"%~dp0rsnew.exe\"" start= auto
if %ERRORLEVEL% NEQ 0 (
    echo Failed to create service!
)

echo Starting service...
sc.exe start AVS
if %ERRORLEVEL% NEQ 0 (
    echo Failed to start service!
)

echo Service installation complete!