# 对比不同测速URL的延迟差异
Write-Host "=== 延迟对比测试 ===" -ForegroundColor Cyan

$testUrls = @{
    "Cloudflare" = "http://cp.cloudflare.com"
    "Google" = "http://www.gstatic.com/generate_204"
    "Cloudflare DNS" = "http://1.1.1.1"
}

$nodeName = "🇭🇰 Hong Kong 01"

Write-Host "`n测试节点: $nodeName`n" -ForegroundColor Yellow

foreach ($name in $testUrls.Keys) {
    $url = $testUrls[$name]
    Write-Host "测试 $name ($url)..." -ForegroundColor White
    
    try {
        $result = Invoke-RestMethod -Uri "http://127.0.0.1:9090/proxies/$nodeName/delay?timeout=5000&url=$url" -Method Get -ErrorAction Stop
        Write-Host "  延迟: $($result.delay)ms" -ForegroundColor Green
    } catch {
        Write-Host "  测速失败: $($_.Exception.Message)" -ForegroundColor Red
    }
    Start-Sleep -Milliseconds 500
}

Write-Host "`n=== 直连测试（不走代理）===" -ForegroundColor Cyan
foreach ($name in $testUrls.Keys) {
    $url = $testUrls[$name]
    Write-Host "直连 $name..." -ForegroundColor White
    
    try {
        $time = Measure-Command { 
            Invoke-WebRequest -Uri $url -UseBasicParsing -TimeoutSec 3 | Out-Null 
        }
        Write-Host "  延迟: $([int]$time.TotalMilliseconds)ms" -ForegroundColor Green
    } catch {
        Write-Host "  请求失败" -ForegroundColor Red
    }
}

Write-Host "`n测试完成！" -ForegroundColor Cyan
