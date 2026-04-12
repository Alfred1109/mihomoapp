# 🚀 安装配置指南

## 📋 概述

本指南提供了跨平台的详细安装步骤，包括Windows、macOS、Linux的构建和部署说明，以及常见问题的解决方案。

## 🎯 构建目标

- **跨平台兼容性**：支持Windows、macOS、Linux
- **一键部署**：简化安装流程
- **依赖管理**：自动处理平台特定依赖
- **性能优化**：针对不同平台优化配置

## 🖥️ 平台支持矩阵

| 平台 | 架构 | 状态 | 安装包格式 | 备注 |
|------|------|------|------------|------|
| **Windows** | x64 | ✅ 完全支持 | `.msi`, `.exe` | 推荐使用MSI安装包 |
| **Windows** | ARM64 | ⚠️ 实验性 | `.msi` | Windows 11 ARM设备 |
| **macOS** | Intel | ✅ 完全支持 | `.dmg`, `.app` | macOS 10.14+ |
| **macOS** | Apple Silicon | ✅ 完全支持 | `.dmg`, `.app` | 原生ARM64支持 |
| **Linux** | x64 | ✅ 完全支持 | `.deb`, `.rpm`, `.AppImage` | 主流发行版 |
| **Linux** | ARM64 | ✅ 完全支持 | `.deb`, `.AppImage` | 树莓派、服务器 |

## 🔧 开发环境要求

### 必需依赖
```bash
# Rust工具链
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# Node.js (推荐v18+)
# 通过 nvm 安装
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 18
nvm use 18

# Tauri CLI
npm install -g @tauri-apps/cli
```

### 平台特定依赖

#### Windows
```powershell
# Visual Studio Build Tools 2022
winget install Microsoft.VisualStudio.2022.BuildTools

# WebView2 Runtime (通常已预装)
winget install Microsoft.EdgeWebView2Runtime
```

#### macOS
```bash
# Xcode Command Line Tools
xcode-select --install

# 可选：完整 Xcode (用于iOS开发)
# 从 App Store 安装 Xcode
```

#### Linux (Ubuntu/Debian)
```bash
# 基础构建工具
sudo apt update
sudo apt install -y \
    build-essential \
    curl \
    wget \
    libssl-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    webkit2gtk-4.0-dev

# 可选：AppImage构建工具
sudo apt install -y fuse
```

#### Linux (CentOS/RHEL/Fedora)
```bash
# Fedora
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y \
    openssl-devel \
    gtk3-devel \
    libappindicator-gtk3-devel \
    librsvg2-devel \
    webkit2gtk3-devel

# CentOS/RHEL (需要EPEL仓库)
sudo yum groupinstall -y "Development Tools"
sudo yum install -y epel-release
sudo yum install -y openssl-devel gtk3-devel webkit2gtk3-devel
```

## 📦 快速安装

### 方法1：预构建版本 (推荐)
```bash
# 下载最新release
wget https://github.com/Alfred1109/mihomoapp/releases/latest/download/mihomo-manager-linux-x64.AppImage

# 添加执行权限
chmod +x mihomo-manager-linux-x64.AppImage

# 运行
./mihomo-manager-linux-x64.AppImage
```

### 方法2：从源码构建
```bash
# 克隆项目
git clone https://github.com/Alfred1109/mihomoapp.git
cd mihomoapp/tauri-app

# 安装依赖
npm install

# 开发模式运行
npm run tauri dev

# 构建生产版本
npm run tauri build
```

### 方法3：包管理器安装

#### Windows (Scoop)
```powershell
# 添加bucket
scoop bucket add extras
scoop install mihomo-manager
```

#### macOS (Homebrew)
```bash
# 添加tap
brew tap Alfred1109/mihomoapp
brew install mihomo-manager
```

#### Linux (Snap)
```bash
# 安装snap包
sudo snap install mihomo-manager
```

## 🔨 详细构建步骤

### 1. 环境准备
```bash
# 检查Rust版本
rustc --version  # 应该 >= 1.70.0

# 检查Node.js版本  
node --version   # 应该 >= 16.0.0

# 检查Tauri CLI
tauri --version  # 应该 >= 1.5.0
```

### 2. 项目克隆与设置
```bash
git clone https://github.com/Alfred1109/mihomoapp.git
cd mihomoapp/tauri-app

# 复制环境配置
cp .env.example .env

# 安装前端依赖
npm install
```

### 3. 开发环境运行
```bash
# 启动开发服务器（热重载）
npm run tauri dev

# 或者使用Tauri CLI
tauri dev
```

### 4. 生产构建
```bash
# 构建所有平台包
npm run build:all

# 或构建特定平台
npm run build:windows   # Windows MSI/EXE
npm run build:macos     # macOS DMG/APP  
npm run build:linux     # Linux DEB/AppImage
```

### 5. 构建输出
构建完成后，安装包位于：
```
tauri-app/src-tauri/target/release/bundle/
├── deb/           # Linux DEB包
├── appimage/      # Linux AppImage
├── msi/           # Windows MSI安装包
├── nsis/          # Windows NSIS安装包
└── macos/         # macOS应用包
```

## 🐛 常见问题解决

### Ubuntu 24.04 依赖兼容性问题

**问题描述**：
Ubuntu 24.04使用了新的软件包版本，可能导致某些依赖不兼容。

**解决方案**：
```bash
# 更新软件源
sudo apt update

# 安装兼容性库
sudo apt install -y \
    libwebkit2gtk-4.0-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev

# 如果仍有问题，尝试从源码安装webkit2gtk
sudo apt build-dep webkit2gtk-4.0
```

**替代方案**：
```bash
# 使用Flatpak版本
flatpak install flathub com.github.alfred1109.mihomoapp

# 使用AppImage版本（推荐）
wget https://github.com/Alfred1109/mihomoapp/releases/latest/download/mihomo-manager-linux-x64.AppImage
chmod +x mihomo-manager-linux-x64.AppImage
```

### Windows构建问题

**问题1：缺少Visual Studio Build Tools**
```powershell
# 安装必需的构建工具
winget install Microsoft.VisualStudio.2022.BuildTools
# 或者安装完整的Visual Studio Community
winget install Microsoft.VisualStudio.2022.Community
```

**问题2：WebView2相关错误**
```powershell
# 手动安装WebView2 Runtime
winget install Microsoft.EdgeWebView2Runtime
# 或从微软官网下载：https://go.microsoft.com/fwlink/p/?LinkId=2124703
```

### macOS构建问题

**问题1：代码签名错误**
```bash
# 禁用代码签名（仅开发环境）
export TAURI_SKIP_DEVTOOLS_INSTALL=true
export CSC_IDENTITY_AUTO_DISCOVERY=false
```

**问题2：权限问题**
```bash
# 给予必要权限
sudo spctl --master-disable  # 临时禁用Gatekeeper
# 构建完成后重新启用
sudo spctl --master-enable
```

### 网络连接问题

**问题：依赖下载失败**
```bash
# 配置npm镜像
npm config set registry https://registry.npmmirror.com/

# 配置Rust镜像
export RUSTUP_DIST_SERVER="https://rsproxy.cn"
export RUSTUP_UPDATE_ROOT="https://rsproxy.cn/rustup"
```

### 构建性能优化

**问题：构建速度慢**
```bash
# 启用Rust增量编译
export CARGO_INCREMENTAL=1

# 使用更多CPU核心
export CARGO_BUILD_JOBS=8

# 使用本地缓存
export CARGO_TARGET_DIR="target"
```

## 🚀 部署配置

### 开发环境配置
```bash
# 复制开发配置
cp tauri-app/.env.example tauri-app/.env.development

# 编辑配置文件
nano tauri-app/.env.development
```

### 生产环境配置
```bash
# 生产环境变量
export TAURI_ENV=production
export RUST_LOG=info
export NODE_ENV=production
```

### 自动部署脚本
```bash
#!/bin/bash
# deploy.sh - 自动部署脚本

set -e

echo "🚀 开始构建部署..."

# 检查环境
node --version
rustc --version
tauri --version

# 安装依赖
npm ci

# 运行测试
npm test

# 构建应用
npm run tauri build

# 创建安装包
echo "📦 创建安装包..."
mkdir -p dist/
cp src-tauri/target/release/bundle/*/*.{msi,deb,dmg,AppImage} dist/ 2>/dev/null || true

echo "✅ 部署完成！"
ls -la dist/
```

## 📊 构建配置选项

### Tauri配置 (tauri.conf.json)
```json
{
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devPath": "http://localhost:1420",
    "distDir": "../dist",
    "withGlobalTauri": false
  },
  "package": {
    "productName": "Mihomo Manager",
    "version": "0.0.0"
  },
  "tauri": {
    "bundle": {
      "active": true,
      "targets": ["msi", "deb", "appimage", "dmg"],
      "identifier": "com.alfred1109.mihomoapp",
      "icon": [
        "icons/32x32.png",
        "icons/128x128.png",
        "icons/icon.icns",
        "icons/icon.ico"
      ]
    }
  }
}
```

## 🔗 相关资源

- **项目仓库**：https://github.com/Alfred1109/mihomoapp
- **问题反馈**：https://github.com/Alfred1109/mihomoapp/issues
- **Tauri文档**：https://tauri.app/
- **Rust文档**：https://doc.rust-lang.org/

## 📝 维护说明

### 依赖更新
```bash
# 更新Rust工具链
rustup update

# 更新Node.js依赖
npm update

# 更新Tauri CLI
npm install -g @tauri-apps/cli@latest
```

### 安全检查
```bash
# Rust安全审计
cargo audit

# Node.js安全审计  
npm audit

# 修复安全问题
npm audit fix
```

---

**更新时间**: 2026-01-31  
**维护者**: Alfred1109  
**支持版本**: v2.1+
