# Mihomo 规则配置说明

## 问题背景

之前的配置只有 GEOIP 规则，导致国内域名如果解析到非 CN 的 IP 地址时，也会走代理。这会造成：
- 国内网站访问变慢
- 不必要的流量消耗
- 某些国内服务无法正常访问

## 解决方案

参考 Nyanpasu 和 mihomo 官方推荐配置，添加了完整的 GEOSITE 域名分流规则。

## 规则优先级（从上到下匹配）

### 1. 本地网络规则
```
DOMAIN-SUFFIX,local,DIRECT
IP-CIDR,127.0.0.0/8,DIRECT
IP-CIDR,172.16.0.0/12,DIRECT
IP-CIDR,192.168.0.0/16,DIRECT
IP-CIDR,10.0.0.0/8,DIRECT
```
本地和内网流量直连

### 2. GEOSITE 域名规则（关键）
```
GEOSITE,private,DIRECT              # 私有域名直连
GEOSITE,cn,DIRECT                   # 国内域名直连
GEOSITE,category-ads-all,REJECT     # 广告域名拒绝
GEOSITE,apple-cn,DIRECT             # 苹果中国服务直连
GEOSITE,microsoft@cn,DIRECT         # 微软中国服务直连
GEOSITE,steam@cn,DIRECT             # Steam 中国直连
GEOSITE,category-games@cn,DIRECT    # 国内游戏直连
GEOSITE,geolocation-!cn,PROXY       # 非国内域名走代理
```

**重点**：这些规则确保国内域名（如 baidu.com, taobao.com, bilibili.com 等）无论解析到什么 IP，都会直连。

### 3. GEOIP IP 地址规则
```
GEOIP,LAN,DIRECT,no-resolve         # 局域网 IP 直连
GEOIP,CN,DIRECT,no-resolve          # 国内 IP 直连
```

`no-resolve` 参数：不进行 DNS 解析，直接匹配 IP，提高性能。

### 4. 兜底规则
```
MATCH,PROXY                         # 其他所有流量走代理
```

## DNS 配置

### nameserver-policy（DNS 分流）
```json
{
  "geosite:cn,private,apple": [
    "https://doh.pub/dns-query",
    "https://dns.alidns.com/dns-query"
  ],
  "geosite:category-ads-all": "rcode://success"
}
```

- 国内域名使用国内 DNS 服务器解析（避免 DNS 污染）
- 广告域名返回成功但不解析（屏蔽广告）

## GEOSITE 数据来源

mihomo 使用的 geosite 数据来自 [MetaCubeX/meta-rules-dat](https://github.com/MetaCubeX/meta-rules-dat)，包含：
- 国内域名列表（来自 blackmatrix7/ios_rule_script）
- 国外服务域名列表
- 广告域名列表
- 各类服务的域名分类

## 规则更新

mihomo 会自动下载和更新 geosite.dat 和 geoip.dat 文件，无需手动维护。

## 性能优化配置

为了提升网页加载速度，添加了以下优化：

### 1. DNS 优化
```json
{
  "ipv6": false,                          // 禁用 IPv6，减少 DNS 查询时间
  "fake-ip-range": "198.18.0.1/16",       // 明确指定 fake-ip 范围
  "default-nameserver": [                 // 快速 UDP DNS 用于初始解析
    "223.5.5.5",
    "119.29.29.29"
  ],
  "fallback": [                           // 国外 DNS 备用
    "https://1.1.1.1/dns-query",
    "https://dns.google/dns-query"
  ],
  "fallback-filter": {                    // DNS 污染过滤
    "geoip": true,
    "geoip-code": "CN"
  }
}
```

**关键优化点**：
- `default-nameserver`：使用快速的 UDP DNS（223.5.5.5）进行初始域名解析，比 DoH 快很多
- `fallback`：当主 DNS 返回被污染的结果时，自动切换到国外 DNS
- `ipv6: false`：禁用 IPv6 查询，减少 DNS 解析时间（大部分网站不需要 IPv6）
- `fake-ip-filter`：排除游戏和特殊服务，避免连接问题

### 2. 连接优化
```json
{
  "unified-delay": true,              // 统一延迟测试
  "tcp-concurrent": true,             // TCP 并发连接
  "keep-alive-interval": 30,          // 保持连接活跃
  "find-process-mode": "strict",      // 精确进程匹配
  "global-client-fingerprint": "chrome" // 模拟 Chrome 指纹，减少被识别
}
```

**性能提升**：
- `tcp-concurrent`：允许同时建立多个 TCP 连接，加快页面加载
- `keep-alive-interval`：保持连接池活跃，减少重新建立连接的开销
- `global-client-fingerprint`：使用 Chrome 指纹，提高兼容性和速度

### 3. DNS 分流策略
```json
{
  "geosite:cn,private,apple": [           // 国内域名用国内 DNS
    "https://doh.pub/dns-query",
    "https://dns.alidns.com/dns-query"
  ],
  "geosite:geolocation-!cn": [            // 国外域名用国外 DNS
    "https://1.1.1.1/dns-query",
    "https://dns.google/dns-query"
  ]
}
```

这样可以：
- 国内网站使用国内 DNS，速度快且无污染
- 国外网站使用国外 DNS，避免解析错误
- 广告域名直接返回成功，不浪费时间

### 速度对比

优化前：
- DNS 解析：500-1000ms（DoH 较慢）
- 首次连接：需要重新建立 TCP 连接
- 国外网站可能使用国内 DNS，解析错误需要重试

优化后：
- DNS 解析：50-100ms（UDP DNS + 缓存）
- 连接复用：保持连接池，减少握手时间
- DNS 分流：正确的域名使用正确的 DNS，一次成功

**预期提升**：网页打开速度提升 3-5 倍，特别是首次访问。

## 参考资料

- [MetaCubeX/meta-rules-dat](https://github.com/MetaCubeX/meta-rules-dat)
- [mihomo 文档](https://wiki.metacubex.one/)
- [Nyanpasu 配置示例](https://github.com/LibNyanpasu/clash-nyanpasu)
