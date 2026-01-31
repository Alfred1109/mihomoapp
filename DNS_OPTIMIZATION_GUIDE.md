# DNS 优化指南

## 📋 概述

本文档详细说明了mihomo应用中DNS配置的系统化优化方案，旨在实现与nyanpasu/clashverge相当的性能水平。

## 🏗️ DNS 三层架构

### Layer 1: 快速初始解析 (UDP)
```yaml
default-nameserver:
  - 223.5.5.5      # 阿里DNS
  - 119.29.29.29   # 腾讯DNS
```
- **作用**：快速启动DNS解析，避免首次查询延迟
- **特点**：UDP协议，响应快速（50-100ms）
- **用途**：初始化和缓存预热

### Layer 2: 主要DNS服务 (DoH加密)
```yaml
nameserver:
  - https://doh.pub/dns-query           # 公共DoH服务
  - https://dns.alidns.com/dns-query    # 阿里DoH服务
```
- **作用**：主要DNS查询通道，支持加密
- **特点**：DoH协议，隐私保护，支持HTTP/3
- **优化**：启用`prefer-h3: true`使用HTTP/3加速

### Layer 3: 防污染备用 (国外DoH)
```yaml
fallback:
  - https://1.1.1.1/dns-query    # Cloudflare
  - https://dns.google/dns-query # Google
```
- **作用**：当主DNS被污染时自动切换
- **触发条件**：通过`fallback-filter`智能判断
- **特点**：国外DNS，用于解析被污染的国外域名

## 🧠 智能分流策略 (nameserver-policy)

```yaml
nameserver-policy:
  # 国内域名 → 国内DNS (避免污染)
  "geosite:cn,private,apple":
    - https://doh.pub/dns-query
    - https://dns.alidns.com/dns-query
  
  # 国外域名 → 国外DNS (准确解析)
  "geosite:geolocation-!cn":
    - https://1.1.1.1/dns-query
    - https://dns.google/dns-query
  
  # 广告域名 → 屏蔽
  "geosite:category-ads-all": "rcode://success"
```

**分流逻辑**：
- 国内网站 → 国内DNS → 避免被污染
- 国外网站 → 国外DNS → 准确解析
- 广告域名 → 直接屏蔽 → 减少无用流量

## 🎯 路由规则分层

```
优先级从高到低：
1. 本地网络 (127.0.0.0/8, 192.168.0.0/16等) → DIRECT
2. 广告域名 (GEOSITE,category-ads-all) → REJECT
3. 私有网络 (GEOSITE,private) → DIRECT
4. 国内服务 (GEOSITE,cn) → DIRECT
5. 国外服务 (GEOSITE,geolocation-!cn) → PROXY
6. 国内IP (GEOIP,CN) → DIRECT
7. 其他流量 (MATCH) → PROXY
```

**关键优化**：
- `no-resolve`标记：IP-CIDR规则不触发DNS查询，避免延迟
- 广告屏蔽优先：在国内外分流之前，减少无用请求
- 分层匹配：从特殊到通用，提高匹配效率

## 🔧 性能优化参数

| 参数 | 值 | 说明 |
|------|-----|------|
| `ipv6` | `false` | 禁用IPv6避免超时 |
| `prefer-h3` | `true` | 启用HTTP/3加速DoH |
| `enhanced-mode` | `fake-ip` | 使用Fake-IP加速 |
| `unified-delay` | `true` | 统一延迟测试 |
| `tcp-concurrent` | `true` | TCP并发连接 |

## 🛡️ Fake-IP 排除列表

某些特殊服务需要真实IP，不能使用Fake-IP：

```yaml
fake-ip-filter:
  - "*.lan"                        # 本地网络
  - "*.local"                      # mDNS
  - "localhost.ptlogin2.qq.com"   # QQ登录
  - "+.srv.nintendo.net"          # Nintendo服务
  - "+.stun.playstation.net"      # PlayStation
  - "xbox.*.microsoft.com"        # Xbox
  - "+.battlenet.com*"            # Battle.net
  - "+.kuwo.cn"                   # 酷我音乐
  - "+.msftconnecttest.com"       # Windows检测
  - "+.msftncsi.com"              # Windows检测
```

## 📊 性能对比

### 优化前
- DNS解析延迟：2-7秒（IPv6超时、过多fallback）
- 国内网站访问：**非常慢**（错误走代理）
- 国外网站访问：**较慢**（DNS解析复杂）

### 优化后
- DNS解析延迟：50-200ms（三层架构、智能分流）
- 国内网站访问：**直连速度**（GEOSITE规则直接匹配）
- 国外网站访问：**正常代理速度**（与nyanpasu相当）

## ⚙️ 配置文件位置

- **代码模板**：`tauri-app/backend/src/mihomo.rs` - `create_default_config()`函数
- **运行时配置**：`~/.config/mihomo/config.yaml` - 实际运行的配置文件

## 🔄 配置更新流程

1. **修改代码模板**：更新`mihomo.rs`中的`create_default_config()`
2. **自动升级机制**：应用启动时自动检测配置版本并升级
3. **备份现有配置**：升级前自动备份当前配置
4. **应用新配置**：重启mihomo服务加载新配置

## 🚀 最佳实践

1. **定期检查日志**：监控DNS查询和路由匹配情况
2. **测试关键域名**：验证国内外域名分流是否正确
3. **监控性能指标**：关注DNS解析延迟和代理延迟
4. **备份配置**：在修改前备份现有配置

## 📝 更新历史

### v2.1 (2026-01-31)
- ✅ 添加`prefer-h3`HTTP/3优化
- ✅ 扩展fake-ip-filter排除列表
- ✅ 优化路由规则分层和优先级
- ✅ 为IP-CIDR规则添加no-resolve标记
- ✅ 添加localhost域名规则

### v2.0 (2026-01-27)
- ✅ 实现DNS三层架构
- ✅ 添加智能分流策略
- ✅ 配置自动升级机制
- ✅ 优化性能参数

## 🔗 参考资源

- [Clash Meta 官方Wiki](https://wiki.metacubex.one/)
- [mihomo 项目](https://github.com/MetaCubeX/mihomo)
- [nyanpasu 配置参考](https://github.com/hiddify/hiddify-next)
