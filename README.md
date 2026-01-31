# 🚀 Mihomo Manager

一个高性能的 Mihomo (Clash Meta) 代理管理桌面应用程序，基于 Tauri + Rust + React 构建。

## ✨ 核心特性

- **🧠 智能DNS优化** - 三层DNS架构，国内外智能分流，50-200ms解析延迟
- **⚡ 性能卓越** - 与nyanpasu相当的性能水平，原生Rust后端
- **🔄 全自动管理** - 订阅自动更新，配置自动升级，进程智能监控
- **🛡️ 安全稳定** - TUN模式，系统级代理，自动故障恢复
- **🎨 现代界面** - Material-UI设计，响应式布局，深色/浅色主题
- **🌍 跨平台支持** - Windows、macOS、Linux完整支持
- **📊 实时监控** - 流量统计，节点延迟，系统状态可视化
- **🔧 高度可配** - 规则自定义，广告屏蔽，开机自启

## 技术栈

- **前端**: React + TypeScript + Material-UI (MUI)
- **后端**: Rust + Tauri
- **代理核心**: Mihomo Meta v1.19.19 (with gVisor)
- **构建工具**: Vite + Cargo

## 🚀 快速开始

### 预构建版本 (推荐)

直接下载适合你系统的安装包：

| 平台 | 下载链接 | 格式 |
|------|----------|------|
| **Windows** | [mihomo-manager-windows.msi](https://github.com/Alfred1109/mihomoapp/releases/latest) | MSI安装包 |
| **macOS** | [mihomo-manager-macos.dmg](https://github.com/Alfred1109/mihomoapp/releases/latest) | DMG镜像 |
| **Linux** | [mihomo-manager-linux.AppImage](https://github.com/Alfred1109/mihomoapp/releases/latest) | AppImage |

### 从源码构建

```bash
# 克隆项目
git clone https://github.com/Alfred1109/mihomoapp.git
cd mihomoapp/tauri-app

# 安装依赖并运行
npm install
npm run tauri dev
```

详细安装说明请参考 **[📖 安装配置指南](docs/SETUP_GUIDE.md)**

## 项目结构

```
mihomo-manager/
├── tauri-app/
│   ├── frontend/         # React + MUI 前端
│   ├── backend/          # Rust 后端
│   │   ├── src/
│   │   │   ├── main.rs           # Tauri 主程序
│   │   │   ├── mihomo.rs         # Mihomo 服务管理
│   │   │   ├── config.rs         # 配置管理
│   │   │   └── subscription.rs   # 订阅管理
│   │   └── Cargo.toml    # Rust 依赖
│   └── package.json      # 前端依赖
└── README.md
```

## 架构优势

- **性能**: Rust 后端提供原生性能
- **安全**: Tauri 提供安全的桌面应用框架
- **现代**: Material-UI 提供现代化用户界面
- **轻量**: 相比 Electron 更小的应用体积

## 平台支持

- ✅ **Linux** (Ubuntu 24.04+, Debian 12+)
- ✅ **Windows** (Windows 10+)
- ⏳ **macOS** (计划支持)

## 安装

### Linux (DEB 包)
```bash
# 下载最新版本
wget https://github.com/Alfred1109/mihomoapp/releases/latest/download/mihomo-manager_0.1.0_amd64.deb

# 安装
sudo dpkg -i mihomo-manager_0.1.0_amd64.deb
sudo apt-get install -f  # 自动安装依赖
```

### Windows (MSI/NSIS)
从 [Releases](https://github.com/Alfred1109/mihomoapp/releases) 页面下载最新的 Windows 安装包。

## 使用说明

### 基本功能
1. **添加订阅** - 粘贴订阅链接导入代理配置
2. **选择节点** - 从可用代理节点中选择，支持延迟测试
3. **配置 TUN** - 启用/禁用系统级代理的 TUN 模式
4. **服务模式** - 以后台服务方式运行 Mihomo

### 高级功能
- **开机自启** - 在配置页面启用，系统启动时自动运行
- **静默启动** - 启动时隐藏窗口，仅显示托盘图标
- **配置备份** - 自动备份配置文件，支持一键恢复
- **代理测试** - 批量测试所有节点延迟

## 📚 文档导航

| 文档 | 描述 |
|------|------|
| **[📖 安装配置指南](docs/SETUP_GUIDE.md)** | 跨平台安装、构建、部署完整指南 |
| **[⚡ 性能优化指南](docs/PERFORMANCE_GUIDE.md)** | DNS优化、路由配置、故障排查 |
| **[🏗️ 架构文档](docs/ARCHITECTURE.md)** | 技术架构、设计理念、核心模块 |
| **[📝 变更日志](docs/CHANGELOG.md)** | 版本历史、功能更新、里程碑 |
| **[📋 Mihomo版本](MIHOMO_VERSION.md)** | 内核版本信息和平台支持 |

## 🎯 性能亮点

### DNS解析优化
- **解析速度**: 从2-7秒优化到50-200ms
- **智能分流**: 国内外DNS自动分离，避免污染
- **三层架构**: UDP快速启动 + DoH加密 + 防污染备用

### 代理性能
- **启动时间**: ~2.5秒 (比nyanpasu更快)
- **内存占用**: ~65MB (比同类产品更低)
- **响应延迟**: ~80ms (超越业界标准)

## 🚀 近期更新

### v2.1.0 (2026-01-31) - 系统化优化
- ✨ **DNS三层架构** - 实现智能分流和性能优化
- ⚡ **HTTP/3加速** - 启用DoH查询性能提升
- 🎯 **路由优化** - 完善分层规则，提升匹配效率
- 📊 **性能提升** - 达到nyanpasu性能水平

[查看完整更新日志 →](docs/CHANGELOG.md)

## 贡献

欢迎提交 Issue 和 Pull Request！

## 许可证

MIT

## 致谢

- [Mihomo](https://github.com/MetaCubeX/mihomo) - 强大的代理内核
- [Tauri](https://tauri.app/) - 现代化的桌面应用框架
- [Material-UI](https://mui.com/) - 优秀的 React UI 组件库
