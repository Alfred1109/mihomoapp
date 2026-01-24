# Windows 资源准备脚本
# 用于下载和准备 mihomo 和 winsw 二进制文件

param(
    [string]$MihomoVersion = "v1.18.10",
    [string]$WinswVersion = "v2.12.0"
)

$ErrorActionPreference = "Stop"
$ResourcesDir = "backend\resources"

Write-Host "=== Mihomo 资源准备脚本 (Windows) ===" -ForegroundColor Cyan
Write-Host ""

# 创建资源目录
if (-not (Test-Path $ResourcesDir)) {
    New-Item -ItemType Directory -Force -Path $ResourcesDir | Out-Null
    Write-Host "✓ 创建资源目录: $ResourcesDir" -ForegroundColor Green
}

# 下载 mihomo Windows 版本
Write-Host "正在下载 mihomo $MihomoVersion (Windows AMD64)..." -ForegroundColor Yellow
$mihomoUrl = "https://github.com/MetaCubeX/mihomo/releases/download/$MihomoVersion/mihomo-windows-amd64-compatible-$MihomoVersion.zip"
$mihomoZip = "mihomo-windows.zip"

try {
    Invoke-WebRequest -Uri $mihomoUrl -OutFile $mihomoZip -UseBasicParsing
    Write-Host "✓ 下载完成" -ForegroundColor Green
    
    # 解压
    Write-Host "正在解压..." -ForegroundColor Yellow
    Expand-Archive -Path $mihomoZip -DestinationPath "temp-mihomo" -Force
    
    # 查找 exe 文件
    $exeFile = Get-ChildItem -Path "temp-mihomo" -Filter "*.exe" -Recurse | Select-Object -First 1
    if ($exeFile) {
        Move-Item -Path $exeFile.FullName -Destination "$ResourcesDir\mihomo.exe" -Force
        Write-Host "✓ mihomo.exe 已放置到 $ResourcesDir" -ForegroundColor Green
    } else {
        throw "未找到 mihomo.exe 文件"
    }
    
    # 清理临时文件
    Remove-Item $mihomoZip -Force
    Remove-Item "temp-mihomo" -Recurse -Force
    
} catch {
    Write-Host "✗ 下载 mihomo 失败: $_" -ForegroundColor Red
    Write-Host "请手动下载并放置到 $ResourcesDir\mihomo.exe" -ForegroundColor Yellow
}

# 下载 WinSW
Write-Host ""
Write-Host "正在下载 WinSW $WinswVersion..." -ForegroundColor Yellow
$winswUrl = "https://github.com/winsw/winsw/releases/download/$WinswVersion/WinSW-x64.exe"

try {
    Invoke-WebRequest -Uri $winswUrl -OutFile "$ResourcesDir\winsw.exe" -UseBasicParsing
    Write-Host "✓ winsw.exe 已下载" -ForegroundColor Green
} catch {
    Write-Host "✗ 下载 WinSW 失败: $_" -ForegroundColor Red
    Write-Host "请手动下载并放置到 $ResourcesDir\winsw.exe" -ForegroundColor Yellow
}

# 验证文件
Write-Host ""
Write-Host "=== 验证资源文件 ===" -ForegroundColor Cyan

$files = @(
    @{Name = "mihomo.exe"; MinSize = 30MB},
    @{Name = "winsw.exe"; MinSize = 10MB}
)

$allValid = $true
foreach ($file in $files) {
    $path = Join-Path $ResourcesDir $file.Name
    if (Test-Path $path) {
        $size = (Get-Item $path).Length
        $sizeMB = [math]::Round($size / 1MB, 2)
        if ($size -gt $file.MinSize) {
            Write-Host "✓ $($file.Name) - ${sizeMB}MB" -ForegroundColor Green
        } else {
            Write-Host "✗ $($file.Name) - ${sizeMB}MB (文件太小，可能损坏)" -ForegroundColor Red
            $allValid = $false
        }
    } else {
        Write-Host "✗ $($file.Name) - 未找到" -ForegroundColor Red
        $allValid = $false
    }
}

Write-Host ""
if ($allValid) {
    Write-Host "=== 所有资源文件准备完成！ ===" -ForegroundColor Green
    Write-Host "现在可以运行: npm run tauri build" -ForegroundColor Cyan
} else {
    Write-Host "=== 部分资源文件缺失或损坏 ===" -ForegroundColor Red
    Write-Host "请检查上述错误并手动下载缺失的文件" -ForegroundColor Yellow
}
