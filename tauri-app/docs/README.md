# Mihomo Manager 完整文档

一个基于 Tauri、React 和 Material-UI 构建的现代化 Mihomo 代理管理应用。

## 📑 目录

- [功能特性](#功能特性)
- [快速开始](#快速开始)
- [开发指南](#开发指南)
- [打包部署](#打包部署)
- [配置说明](#配置说明)
- [故障排查](#故障排查)

---

## 🚀 功能特性

- ✅ **跨平台支持**: Windows, Linux, macOS
- 🎨 **现代化界面**: Material-UI 设计，响应式布局
- ⚙️ **完整配置管理**: 可视化配置编辑器
- 🌐 **智能 DNS 分流**: 国内/国外域名自动分流
- 🛡️ **IP 分流规则**: GEOSITE + GEOIP 双重分流
- 🔒 **TUN 模式**: 系统级透明代理
- 📊 **实时监控**: 连接状态、流量统计
- 🔄 **自动备份**: 配置文件自动备份
- 🌍 **多语言**: 中文/英文界面

---

## 🎯 快速开始

### 系统要求

**通用要求**:
- Node.js 16+
- Rust 1.70+
- npm 或 yarn

**Windows**:
- Visual Studio 2019+ (C++ 工具)
- WebView2 Runtime

**Linux**:
```bash
# Ubuntu/Debian
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev libgtk-3-dev

# Fedora
sudo dnf install webkit2gtk4.1-devel libappindicator-gtk3-devel gtk3-devel
```

### 安装步骤

```bash
# 1. 克隆仓库
git clone <repository-url>
cd mihomo/tauri-app

# 2. 安装依赖
npm install

# 3. 准备 mihomo 二进制文件（自动）
npm run prepare:resources

# 4. 开发模式运行
npm run tauri:dev

# 5. 构建生产版本
npm run tauri:build
```

---

## 💻 开发指南

### 项目结构

```
tauri-app/
├── src/                          # React 前端
│   ├── components/
│   │   ├── Dashboard.tsx        # 仪表盘
│   │   ├── ConfigManager.tsx    # 配置管理
│   │   ├── ProxyManager.tsx     # 代理管理
│   │   └── ConnectionMonitor.tsx # 连接监控
│   ├── App.tsx
│   └── main.tsx
├── backend/                    # Tauri 后端
│   ├── src/
│   │   ├── main.rs              # 主程序入口
│   │   ├── config.rs            # 配置管理
│   │   ├── mihomo.rs            # mihomo 核心逻辑
│   │   ├── backup.rs            # 备份管理
│   │   └── validator.rs         # 配置验证
│   ├── resources/               # 打包资源
│   │   ├── mihomo              # Linux 二进制
│   │   ├── mihomo.exe          # Windows 二进制
│   │   └── winsw.exe           # Windows 服务管理
│   └── tauri.conf.json         # Tauri 配置
├── prepare-resources.ps1        # Windows 资源准备
├── prepare-resources.sh         # Linux 资源准备
└── package.json
```

### 开发命令

```bash
# 前端开发
npm run dev              # 启动 Vite 开发服务器

# Tauri 开发
npm run tauri:dev        # 启动完整应用（热重载）

# 构建
npm run build            # 构建前端
npm run tauri:build      # 构建完整应用

# 资源管理
npm run prepare:resources  # 下载 mihomo 二进制
```

### 配置文件位置

**Windows**:
- 配置: `%APPDATA%\mihomo\config.yaml`
- 备份: `%APPDATA%\mihomo\backups\`

**Linux**:
- 配置: `~/.config/mihomo/config.yaml`
- 备份: `~/.config/mihomo/backups/`

---

## 📦 打包部署

### 准备二进制文件

#### 自动准备（推荐）

```bash
npm run prepare:resources
```

脚本会自动：
- 检测操作系统和架构
- 下载对应版本的 mihomo
- 下载 WinSW (Windows)
- 验证文件完整性

#### 手动准备

从 [mihomo releases](https://github.com/MetaCubeX/mihomo/releases) 下载：

**Windows**:
```powershell
# 下载并放置到 backend/resources/
mihomo.exe    # mihomo 核心
winsw.exe     # Windows 服务管理
```

**Linux**:
```bash
# 下载并放置到 backend/resources/
mihomo        # mihomo 核心（需要执行权限）
chmod +x backend/resources/mihomo
```

### Tauri 打包配置

`tauri.conf.json` 中的关键配置：

```json
{
  "bundle": {
    "resources": [
      "resources/mihomo",
      "resources/mihomo.exe",
      "resources/winsw.exe"
    ],
    "externalBin": [
      "resources/mihomo",
      "resources/mihomo.exe"
    ],
    "targets": ["deb", "msi", "nsis"]
  }
}
```

**配置说明**:
- `resources`: 打包到应用资源目录
- `externalBin`: 标记为可执行文件，Tauri 自动处理权限
- `targets`: 构建目标格式

### 二进制文件查找逻辑

应用启动时按优先级查找 mihomo：

**Linux**:
1. `/usr/local/bin/mihomo` (系统全局)
2. `/usr/bin/mihomo` (系统全局)
3. `/opt/mihomo/mihomo` (独立安装)
4. `{app_dir}/mihomo` (应用目录)
5. `{app_dir}/resources/mihomo` (打包位置)

**Windows**:
1. `{app_dir}\mihomo.exe` (应用目录)
2. `{app_dir}\resources\mihomo.exe` (打包位置)

### 构建安装包

```bash
npm run tauri:build
```

**输出位置**:
- **Windows**: `backend/target/release/bundle/msi/` 或 `nsis/`
- **Linux**: `backend/target/release/bundle/deb/`

### 安装后的文件位置

**Windows**:
- 应用: `C:\Program Files\Mihomo Manager\`
- 二进制: `C:\Program Files\Mihomo Manager\resources\mihomo.exe`

**Linux**:
- 应用: `/usr/bin/mihomo-manager`
- 二进制: `/usr/lib/mihomo-manager/resources/mihomo`

---

## ⚙️ 配置说明

### DNS 配置架构

#### 三层 DNS 解析

```
1. default-nameserver (UDP DNS)
   └─ 223.5.5.5, 119.29.29.29
   └─ 快速初始解析 (50-100ms)

2. nameserver (DoH 加密)
   └─ https://doh.pub/dns-query
   └─ https://dns.alidns.com/dns-query
   └─ 主要 DNS 服务器

3. fallback (防污染)
   └─ https://1.1.1.1/dns-query
   └─ https://dns.google/dns-query
   └─ 污染结果时自动切换
```

#### DNS 分流策略

```json
{
  "nameserver-policy": {
    "geosite:cn,private,apple": [
      "https://doh.pub/dns-query",
      "https://dns.alidns.com/dns-query"
    ],
    "geosite:geolocation-!cn": [
      "https://1.1.1.1/dns-query",
      "https://dns.google/dns-query"
    ],
    "geosite:category-ads-all": "rcode://success"
  }
}
```

**分流说明**:
- 国内域名 → 国内 DNS (避免污染)
- 国外域名 → 国外 DNS (准确解析)
- 广告域名 → 返回成功但不解析 (屏蔽)

#### DNS 优化配置

```json
{
  "ipv6": false,                    // 禁用 IPv6，减少查询
  "enhanced-mode": "fake-ip",       // Fake-IP 模式加速
  "fake-ip-range": "198.18.0.1/16", // Fake-IP 范围
  "fake-ip-filter": [               // 排除特殊服务
    "*.lan",
    "*.local",
    "+.stun.playstation.net",
    "+.battlenet.com"
  ],
  "fallback-filter": {
    "geoip": true,                  // 启用 GeoIP 过滤
    "geoip-code": "CN",             // 中国 IP 判定
    "ipcidr": ["240.0.0.0/4"]       // 过滤保留 IP
  }
}
```

### IP 分流规则

#### 四层规则优先级

```
1. 本地网络 (7条)
   └─ 局域网、私有 IP 直连

2. GEOSITE 域名 (8条)
   └─ 基于域名的精确分流

3. GEOIP 地址 (2条)
   └─ 基于 IP 地理位置分流

4. 兜底规则 (1条)
   └─ 其他流量默认处理
```

#### 详细规则列表

**1. 本地网络规则**
```yaml
- DOMAIN-SUFFIX,local,DIRECT
- IP-CIDR,127.0.0.0/8,DIRECT       # 本地回环
- IP-CIDR,172.16.0.0/12,DIRECT     # 私有网络 A
- IP-CIDR,192.168.0.0/16,DIRECT    # 私有网络 B
- IP-CIDR,10.0.0.0/8,DIRECT        # 私有网络 C
- IP-CIDR,17.0.0.0/8,DIRECT        # Apple 私有
- IP-CIDR,100.64.0.0/10,DIRECT     # 运营商 NAT
```

**2. GEOSITE 域名规则**
```yaml
- GEOSITE,private,DIRECT           # 私有域名
- GEOSITE,cn,DIRECT                # 国内域名
- GEOSITE,category-ads-all,REJECT  # 广告域名
- GEOSITE,apple-cn,DIRECT          # 苹果中国
- GEOSITE,microsoft@cn,DIRECT      # 微软中国
- GEOSITE,steam@cn,DIRECT          # Steam 中国
- GEOSITE,category-games@cn,DIRECT # 国内游戏
- GEOSITE,geolocation-!cn,PROXY    # 国外域名
```

**3. GEOIP 地址规则**
```yaml
- GEOIP,LAN,DIRECT,no-resolve      # 局域网 IP
- GEOIP,CN,DIRECT,no-resolve       # 国内 IP
```

**4. 兜底规则**
```yaml
- MATCH,PROXY                       # 其他流量
```

#### 规则说明

**GEOSITE 数据来源**:
- [MetaCubeX/meta-rules-dat](https://github.com/MetaCubeX/meta-rules-dat)
- 包含国内外域名、广告域名等分类
- mihomo 自动更新，无需手动维护

**no-resolve 参数**:
- 不进行 DNS 解析，直接匹配 IP
- 提高性能，避免重复解析

**规则优势**:
- 域名优先于 IP，确保精确分流
- 国内域名无论解析到什么 IP 都直连
- 避免 CDN 导致的误判

### 性能优化配置

#### 连接优化

```json
{
  "unified-delay": true,              // 统一延迟测试
  "tcp-concurrent": true,             // TCP 并发连接
  "keep-alive-interval": 30,          // 保持连接活跃
  "find-process-mode": "strict",      // 精确进程匹配
  "global-client-fingerprint": "chrome" // Chrome 指纹
}
```

**性能提升**:
- `tcp-concurrent`: 同时建立多个连接，加快页面加载
- `keep-alive-interval`: 连接池复用，减少握手开销
- `global-client-fingerprint`: 提高兼容性

#### 速度对比

**优化前**:
- DNS 解析: 500-1000ms (DoH 较慢)
- 首次连接: 需要重新建立 TCP
- 可能使用错误的 DNS，需要重试

**优化后**:
- DNS 解析: 50-100ms (UDP DNS + 缓存)
- 连接复用: 保持连接池
- DNS 分流: 一次成功

**预期提升**: 网页打开速度提升 3-5 倍

### TUN 模式

#### 功能说明

- 系统级透明代理
- 自动路由配置
- DNS 劫持支持
- 无需手动配置系统代理

#### 配置示例

```json
{
  "tun": {
    "enable": false,
    "stack": "system",
    "auto-route": true,
    "auto-detect-interface": true,
    "dns-hijack": ["any:53"],
    "mtu": 1500
  }
}
```

#### 权限要求

**Windows**:
- 需要管理员权限运行应用
- 应用会提示提升权限

**Linux**:
```bash
# 方法 1: 使用 sudo
sudo mihomo-manager

# 方法 2: 设置 capabilities (推荐)
sudo setcap cap_net_admin,cap_net_bind_service=+ep /path/to/mihomo
```

---

## 🔧 故障排查

### 常见问题

#### 1. 找不到 mihomo 二进制文件

**错误信息**: "未找到 mihomo.exe 文件"

**解决方案**:
```bash
# 自动下载
npm run prepare:resources

# 手动检查
ls backend/resources/
# 应该看到 mihomo 或 mihomo.exe
```

#### 2. 编译错误

```bash
# 清理并重新构建
cd backend
cargo clean
cd ..
npm run tauri:build
```

#### 3. TUN 模式无法启用

**Windows**:
- 右键应用 → "以管理员身份运行"
- 或使用应用内的"以管理员身份重启"功能

**Linux**:
```bash
# 临时方案
sudo mihomo-manager

# 永久方案（推荐）
sudo setcap cap_net_admin,cap_net_bind_service=+ep \
  /usr/lib/mihomo-manager/resources/mihomo
```

#### 4. 权限被拒绝 (Linux)

```bash
# 检查文件权限
ls -l backend/resources/mihomo

# 添加执行权限
chmod +x backend/resources/mihomo
```

#### 5. DNS 解析失败

**检查配置**:
- 确认 DNS 配置正确
- 检查网络连接
- 尝试切换 DNS 服务器

**测试 DNS**:
```bash
# Windows
nslookup google.com 127.0.0.1

# Linux
dig @127.0.0.1 google.com
```

#### 6. 代理连接失败

**排查步骤**:
1. 检查 mihomo 是否运行
2. 检查代理配置是否正确
3. 查看 mihomo 日志
4. 测试代理节点延迟

#### 7. 配置文件损坏

**恢复方案**:
```bash
# 应用会自动备份配置
# 备份位置:
# Windows: %APPDATA%\mihomo\backups\
# Linux: ~/.config/mihomo/backups/

# 手动恢复
cp backup-file.yaml config.yaml
```

### 日志查看

**应用日志**:
- Windows: 应用控制台输出
- Linux: 终端输出或 systemd 日志

**mihomo 日志**:
- 通过应用界面查看
- 或直接查看 mihomo 输出

### 性能问题

**网速慢**:
1. 检查代理节点延迟
2. 切换到更快的节点
3. 检查 DNS 配置
4. 确认规则分流正确

**内存占用高**:
1. 检查连接数
2. 重启 mihomo 服务
3. 清理缓存

---

## 📚 参考资料

### 官方文档
- [Mihomo 官方仓库](https://github.com/MetaCubeX/mihomo)
- [Mihomo Wiki](https://wiki.metacubex.one/)
- [Tauri 文档](https://tauri.app/)
- [Material-UI 文档](https://mui.com/)

### 规则数据
- [MetaCubeX/meta-rules-dat](https://github.com/MetaCubeX/meta-rules-dat)
- [blackmatrix7/ios_rule_script](https://github.com/blackmatrix7/ios_rule_script)

### 相关项目
- [Clash Nyanpasu](https://github.com/LibNyanpasu/clash-nyanpasu)
- [Clash Verge](https://github.com/zzzgydi/clash-verge)

---

## 📄 许可证

本项目采用 MIT 许可证。

**依赖项许可证**:
- mihomo: GPL-3.0
- WinSW: MIT
- Tauri: MIT/Apache-2.0
- Material-UI: MIT

---

## 🤝 贡献

欢迎贡献代码、报告问题或提出建议！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

---

## 📞 支持

如有问题，请通过以下方式获取帮助：
- 提交 GitHub Issue
- 查看文档和 FAQ
- 参考官方 Wiki

---

**最后更新**: 2026-01-24
