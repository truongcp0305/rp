
@echo off
REM === Lấy đường dẫn của file thực thi ===
set SCRIPT_PATH=%~dp0
set BINARY_PATH=%SCRIPT_PATH%rsnew.exe

copy "%SCRIPT_PATH%token.txt" "C:\temp\token.txt"
if %ERRORLEVEL% NEQ 0 (
    echo Failed to copy token.txt!
) else (
    echo token.txt copied successfully to C:\temp.
)

REM === Tên của Task Scheduler task ===
set TASK_NAME=MyAppOnLogin

REM === Tạo task chạy khi user đăng nhập ===
schtasks /Create ^
 /TN "%TASK_NAME%" ^
 /TR "\"%BINARY_PATH%\"" ^
 /SC ONLOGON ^
 /RL HIGHEST ^
 /F

REM === Chạy ứng dụng ngay lập tức (nếu muốn) ===
start "" "%BINARY_PATH%"
echo Task created and binary started.