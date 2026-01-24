# Mihomo Manager

一个基于 Tauri、React 和 Material-UI 构建的现代化 Mihomo 代理管理应用。

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20Linux%20%7C%20macOS-blue)](https://github.com)

## ✨ 功能特性

- 🚀 **跨平台支持** - Windows, Linux, macOS
- 🎨 **现代化界面** - Material-UI 设计，响应式布局
- ⚙️ **完整配置管理** - 可视化配置编辑器
- 🌐 **智能 DNS 分流** - 国内/国外域名自动分流
- 🛡️ **IP 分流规则** - GEOSITE + GEOIP 双重分流
- 🔒 **TUN 模式** - 系统级透明代理
- 📊 **实时监控** - 连接状态、流量统计
- 🔄 **自动备份** - 配置文件自动备份
- 🌍 **多语言** - 中文/英文界面

## 📖 文档

- **[完整文档](docs/README.md)** - 详细的使用和开发文档
- **[配置说明](docs/README.md#配置说明)** - DNS 和 IP 分流配置
- **[打包部署](docs/README.md#打包部署)** - 构建和发布指南
- **[故障排查](docs/README.md#故障排查)** - 常见问题解决方案

## 🚀 快速开始

```bash
# 1. 安装依赖
npm install

# 2. 准备 mihomo 二进制文件
npm run prepare:resources

# 3. 开发模式运行
npm run tauri:dev

# 4. 构建生产版本
npm run tauri:build
```

### 系统要求

**Windows**: Visual Studio 2019+ (C++ 工具), WebView2 Runtime

**Linux**:
```bash
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev libgtk-3-dev
```

## 📂 项目结构

```
tauri-app/
├── src/                    # React 前端
├── src-tauri/             # Tauri 后端
│   ├── src/              # Rust 源码
│   └── resources/        # mihomo 二进制文件
├── docs/                  # 完整文档
└── prepare-resources.*    # 资源准备脚本
```

## 🛠️ 开发命令

```bash
npm run dev                 # 前端开发服务器
npm run tauri:dev          # Tauri 开发模式
npm run tauri:build        # 构建应用
npm run prepare:resources  # 准备 mihomo 二进制
```

## 📝 许可证

MIT License - 详见 [LICENSE](LICENSE)

**依赖项**: mihomo (GPL-3.0), WinSW (MIT)

## 🔗 相关链接

- [Mihomo](https://github.com/MetaCubeX/mihomo) - 核心代理引擎
- [Tauri](https://tauri.app/) - 应用框架
- [Material-UI](https://mui.com/) - UI 组件库
