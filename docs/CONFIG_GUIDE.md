# 🔧 配置指南

本指南详细说明 Mihomo Manager 的配置文件结构和使用方法。

## 📁 配置文件位置

### 系统配置文件位置
| 平台 | 配置目录 |
|------|----------|
| **Linux** | `~/.config/mihomo/` |
| **Windows** | `%APPDATA%\mihomo\` |
| **macOS** | `~/Library/Application Support/mihomo/` |

### 主要配置文件
- `config.yaml` - 主配置文件
- `config.example.yaml` - 示例配置模板（项目根目录）

## 🚀 快速开始

### 1. 复制示例配置
```bash
# Linux/macOS
cp config.example.yaml ~/.config/mihomo/config.yaml

# Windows (PowerShell)
Copy-Item config.example.yaml "$env:APPDATA\mihomo\config.yaml"
```

### 2. 修改配置
编辑 `config.yaml` 并替换示例内容：
- 代理服务器地址和端口
- 认证密码或密钥
- 订阅链接（如使用）

## 📋 配置文件结构

### 🌐 基础网络配置
```yaml
port: 7890                    # HTTP代理端口
socks-port: 7891             # SOCKS5代理端口
mixed-port: 7890             # 混合模式端口
allow-lan: false             # 局域网访问
mode: rule                   # 工作模式
external-controller: 127.0.0.1:9090  # API接口
```

### 🧠 DNS配置（重要！）
我们的DNS配置采用**三层架构**，提供最佳性能：

```yaml
dns:
  enable: true
  ipv6: false                # 禁用IPv6提升速度
  prefer-h3: true            # 启用HTTP/3加速
  
  # Layer 1: 快速启动 (50-100ms)
  default-nameserver:
    - 223.5.5.5
    - 119.29.29.29
    
  # Layer 2: 主要解析 (DoH加密)
  nameserver:
    - https://doh.pub/dns-query
    - https://dns.alidns.com/dns-query
    
  # Layer 3: 防污染备用
  fallback:
    - https://1.1.1.1/dns-query
    - https://dns.google/dns-query
    
  # 智能分流策略
  nameserver-policy:
    "geosite:cn,private,apple":
      - https://doh.pub/dns-query
    "geosite:geolocation-!cn":
      - https://1.1.1.1/dns-query
    "geosite:category-ads-all": "rcode://success"
```

### 🌐 代理配置示例

#### Shadowsocks
```yaml
- name: "my-ss-server"
  type: ss
  server: your-server.com
  port: 8388
  cipher: chacha20-ietf-poly1305
  password: "your-password"
```

#### VMess
```yaml
- name: "my-vmess-server"
  type: vmess
  server: your-server.com
  port: 443
  uuid: "your-uuid"
  alterId: 0
  cipher: auto
  network: ws
  tls: true
```

#### Trojan
```yaml
- name: "my-trojan-server"
  type: trojan
  server: your-server.com
  port: 443
  password: "your-password"
  sni: your-server.com
```

### 🎛️ 代理组配置

#### 手动选择组
```yaml
- name: "PROXY"
  type: select
  proxies:
    - "AUTO"
    - "DIRECT"
    - "my-ss-server"
    - "my-vmess-server"
```

#### 自动选择组
```yaml
- name: "AUTO"
  type: url-test
  proxies:
    - "my-ss-server"
    - "my-vmess-server"
  url: http://www.gstatic.com/generate_204
  interval: 300
  tolerance: 50
```

### 🎯 路由规则

规则按**优先级从高到低**匹配：

```yaml
rules:
  # 1. 本地网络直连
  - DOMAIN-SUFFIX,local,DIRECT
  - IP-CIDR,192.168.0.0/16,DIRECT,no-resolve
  
  # 2. 广告屏蔽
  - GEOSITE,category-ads-all,ADBLOCK
  
  # 3. 国内服务直连
  - GEOSITE,cn,DIRECT
  - GEOSITE,apple-cn,DIRECT
  
  # 4. 国外服务代理
  - GEOSITE,geolocation-!cn,PROXY
  
  # 5. 国内IP直连
  - GEOIP,CN,DIRECT,no-resolve
  
  # 6. 默认代理
  - MATCH,PROXY
```

## 🔄 订阅配置

### 📋 订阅兼容性设计

我们的配置采用**标准代理组命名**，确保与主流订阅服务兼容：

| 标准名称 | 用途 | 订阅兼容性 |
|----------|------|------------|
| `PROXY` | 主代理组 | ✅ 通用标准 |
| `AUTO` | 自动选择 | ✅ 主流支持 |
| `DIRECT` | 直连 | ✅ 内置策略 |
| `ADBLOCK` | 广告屏蔽 | ✅ 可选配置 |

### 🔗 使用订阅链接

#### 1. 应用内设置（推荐）
- 在应用的订阅管理页面添加订阅URL
- 应用会自动下载并合并代理节点
- 保留DNS优化和路由规则配置

#### 2. 手动配置
```yaml
# 在proxy-providers中添加订阅
proxy-providers:
  my-subscription:
    type: http
    url: "https://your-subscription-url"
    interval: 3600
    path: ./profiles/subscription.yaml
    health-check:
      enable: true
      url: http://www.gstatic.com/generate_204
      interval: 600

# 代理组引用订阅
proxy-groups:
  - name: "PROXY"
    type: select
    use:
      - my-subscription
    proxies:
      - "AUTO"
      - "DIRECT"
      
  - name: "AUTO"
    type: url-test
    use:
      - my-subscription
    url: http://www.gstatic.com/generate_204
    interval: 300
```

### ⚠️ 订阅注意事项

1. **代理组冲突处理**：
   - 订阅可能包含自己的代理组定义
   - 我们的规则会优先引用标准名称（PROXY, AUTO等）
   - 如订阅使用不同命名，需手动调整rules中的引用

2. **配置合并原则**：
   - DNS配置：以我们的优化配置为准
   - 代理节点：以订阅提供的为准
   - 路由规则：以我们的智能分流为准
   - 代理组：可灵活组合使用

## 🛠️ 高级配置

### TUN模式
```yaml
tun:
  enable: true               # 启用系统级代理
  stack: system             # 网络栈类型
  auto-route: true          # 自动路由
  auto-detect-interface: true
```

### 性能优化
```yaml
# 已包含在示例配置中
unified-delay: true         # 统一延迟测试
tcp-concurrent: true        # TCP并发
keep-alive-interval: 30     # 保活间隔
find-process-mode: strict   # 进程检测
```

## 🔧 故障排除

### 常见问题

#### 1. DNS解析慢
- 检查 `dns.ipv6: false` 是否设置
- 确认 `dns.prefer-h3: true` 已启用
- 验证 `nameserver-policy` 配置正确

#### 2. 国内网站打不开
- 检查规则中是否有 `GEOSITE,cn,DIRECT`
- 确认 `GEOIP,CN,DIRECT,no-resolve` 配置
- 验证本地网络规则优先级

#### 3. 代理连接失败
- 检查服务器地址、端口是否正确
- 验证密码、UUID等认证信息
- 测试网络连通性

#### 4. TUN模式无法启用
- 确保以管理员权限运行
- 检查系统是否支持TUN设备
- 验证防火墙设置

### 日志分析
```yaml
log-level: debug            # 启用详细日志
```

查看日志文件位置：
- Linux: `~/.config/mihomo/logs/`
- Windows: `%APPDATA%\mihomo\logs\`
- macOS: `~/Library/Logs/mihomo/`

## 📖 参考资源

- [Mihomo 官方文档](https://wiki.metacubex.one/)
- [Clash Meta 配置指南](https://github.com/MetaCubeX/mihomo)
- [GeoSite 规则列表](https://github.com/v2fly/domain-list-community)
- [GeoIP 数据库](https://github.com/v2fly/geoip)

## ⚠️ 注意事项

1. **配置安全**：
   - 不要在公共仓库中提交包含真实服务器信息的配置
   - 定期更换密码和密钥
   - 使用强加密算法

2. **性能优化**：
   - 根据网络环境调整DNS服务器
   - 合理设置代理组和规则
   - 监控资源使用情况

3. **法律合规**：
   - 遵守当地法律法规
   - 仅用于合法用途
   - 尊重服务条款
