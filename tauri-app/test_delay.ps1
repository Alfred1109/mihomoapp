# 测试mihomo节点延迟
Write-Host "测试节点延迟..." -ForegroundColor Cyan

# 获取所有代理节点
$proxies = Invoke-RestMethod -Uri "http://127.0.0.1:9090/proxies" -Method Get

# 测试几个节点
$testNodes = @("🇭🇰 Hong Kong 01", "🇸🇬 Singapore 01", "🇺🇸 United States 01")

foreach ($nodeName in $testNodes) {
    if ($proxies.proxies.$nodeName) {
        Write-Host "`n测试节点: $nodeName" -ForegroundColor Yellow
        
        # 使用Cloudflare测速
        try {
            $result = Invoke-RestMethod -Uri "http://127.0.0.1:9090/proxies/$nodeName/delay?timeout=5000&url=http://cp.cloudflare.com" -Method Get
            Write-Host "  Cloudflare延迟: $($result.delay)ms" -ForegroundColor Green
        } catch {
            Write-Host "  Cloudflare测速失败" -ForegroundColor Red
        }
        
        # 使用gstatic测速对比
        try {
            $result2 = Invoke-RestMethod -Uri "http://127.0.0.1:9090/proxies/$nodeName/delay?timeout=5000&url=http://www.gstatic.com/generate_204" -Method Get
            Write-Host "  gstatic延迟: $($result2.delay)ms" -ForegroundColor Green
        } catch {
            Write-Host "  gstatic测速失败" -ForegroundColor Red
        }
    }
}

Write-Host "`n测试完成！" -ForegroundColor Cyan
