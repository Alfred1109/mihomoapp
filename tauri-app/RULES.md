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

## 参考资料

- [MetaCubeX/meta-rules-dat](https://github.com/MetaCubeX/meta-rules-dat)
- [mihomo 文档](https://wiki.metacubex.one/)
- [Nyanpasu 配置示例](https://github.com/LibNyanpasu/clash-nyanpasu)
