# Mihomo Manager - 管理员启动脚本
# 自动以管理员身份启动 Tauri 应用

$appPath = "d:\mihomo\tauri-app\src-tauri\target\debug\mihomoapp.exe"

# 检查应用文件是否存在
if (-not (Test-Path $appPath)) {
    Write-Host "错误: 找不到应用程序文件" -ForegroundColor Red
    Write-Host "路径: $appPath" -ForegroundColor Yellow
    Write-Host "请先编译应用程序: cd tauri-app/src-tauri && cargo build" -ForegroundColor Yellow
    Read-Host "按任意键退出"
    exit 1
}

# 检查是否已经以管理员身份运行
$currentPrincipal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
$isAdmin = $currentPrincipal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if ($isAdmin) {
    # 已经是管理员，直接启动应用
    Write-Host "以管理员身份启动 Mihomo Manager..." -ForegroundColor Green
    Start-Process -FilePath $appPath -Wait
} else {
    # 不是管理员，请求提权并重新启动
    Write-Host "请求管理员权限..." -ForegroundColor Yellow
    try {
        Start-Process PowerShell -ArgumentList "-ExecutionPolicy Bypass -File `"$PSCommandPath`"" -Verb RunAs
    } catch {
        Write-Host "无法获取管理员权限: $($_.Exception.Message)" -ForegroundColor Red
        Read-Host "按任意键退出"
        exit 1
    }
}
