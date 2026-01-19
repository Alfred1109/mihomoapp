@echo off
title Mihomo Manager - 开发模式管理员启动
echo.
echo ========================================
echo   Mihomo Manager - 开发模式启动脚本
echo ========================================
echo.

cd /d "d:\mihomo\tauri-app"

echo [信息] 检查前端依赖...
if not exist "node_modules" (
    echo [警告] 前端依赖未安装，正在安装...
    npm install
    if %ERRORLEVEL% NEQ 0 (
        echo [错误] 前端依赖安装失败
        pause
        exit /b 1
    )
)

echo [信息] 检查 Rust 编译...
cd src-tauri
if not exist "target\debug\mihomoapp.exe" (
    echo [警告] 应用未编译，正在编译...
    cargo build
    if %ERRORLEVEL% NEQ 0 (
        echo [错误] Rust 编译失败
        pause
        exit /b 1
    )
)

echo [信息] 正在以管理员身份启动开发版本...
echo.

REM 使用 PowerShell 以管理员身份启动应用
powershell -Command "Start-Process 'target\debug\mihomoapp.exe' -Verb RunAs"

if %ERRORLEVEL% EQU 0 (
    echo [成功] 开发版本已启动
    echo.
    echo 提示: 如果需要热重载，请使用: npm run tauri:dev
) else (
    echo [错误] 启动失败
    pause
)

echo.
timeout /t 2 /nobreak >nul
