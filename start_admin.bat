@echo off
title Mihomo Manager - 管理员启动
echo.
echo ========================================
echo    Mihomo Manager - 管理员启动脚本
echo ========================================
echo.

set "APP_PATH=d:\mihomo\tauri-app\src-tauri\target\debug\mihomoapp.exe"

REM 检查应用文件是否存在
if not exist "%APP_PATH%" (
    echo [错误] 找不到应用程序文件
    echo 路径: %APP_PATH%
    echo.
    echo 请先编译应用程序:
    echo   cd tauri-app/src-tauri
    echo   cargo build
    echo.
    pause
    exit /b 1
)

echo [信息] 正在以管理员身份启动 Mihomo Manager...
echo.

REM 使用 PowerShell 以管理员身份启动应用
powershell -Command "Start-Process '%APP_PATH%' -Verb RunAs"

if %ERRORLEVEL% EQU 0 (
    echo [成功] 应用程序已启动
) else (
    echo [错误] 启动失败，错误代码: %ERRORLEVEL%
    echo.
    echo 可能的原因:
    echo - 用户取消了 UAC 提示
    echo - 应用程序文件损坏
    echo - 系统权限问题
    echo.
    pause
)

echo.
echo 脚本执行完成
timeout /t 3 /nobreak >nul
