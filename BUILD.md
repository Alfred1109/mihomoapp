# 跨平台构建指南

本项目已配置支持 Windows 和 Linux 双平台安装包生成。

## 构建目标

- **Windows**: MSI 安装包 + NSIS 安装程序
- **Linux**: DEB 安装包 (Debian/Ubuntu)

## 平台适配说明

### Windows 平台
- ✅ 管理员权限支持 (app.manifest)
- ✅ Windows 服务管理 (sc.exe)
- ✅ 进程管理 (taskkill)
- ✅ 配置路径: `%APPDATA%\Roaming\mihomo`
- ✅ WebView2 自动下载引导

### Linux 平台
- ✅ systemd 服务管理
- ✅ 进程管理 (pkill)
- ✅ 配置路径: `~/.config/mihomo`
- ✅ 桌面入口文件 (.desktop)
- ✅ root 权限检查

## 构建步骤

### 在 Windows 上构建

```powershell
# 进入项目目录
cd tauri-app

# 安装依赖
npm install

# 构建 Windows 安装包 (MSI + NSIS)
npm run tauri:build

# 构建产物位置:
# tauri-app/src-tauri/target/release/bundle/msi/Mihomo Manager_0.1.0_x64_en-US.msi
# tauri-app/src-tauri/target/release/bundle/nsis/Mihomo Manager_0.1.0_x64-setup.exe
```

### 在 Linux 上构建

```bash
# 进入项目目录
cd tauri-app

# 安装依赖
npm install

# 安装系统依赖 (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install -y \
    libwebkit2gtk-4.1-dev \
    libjavascriptcoregtk-4.1-dev \
    libgtk-3-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev

# 构建 Linux 安装包 (DEB)
npm run tauri:build

# 构建产物位置:
# tauri-app/src-tauri/target/release/bundle/deb/mihomo-manager_0.1.0_amd64.deb
```

## 仅构建特定平台

如果只想构建特定格式，可以修改 `tauri.conf.json`:

```json
// 仅 Windows MSI
"targets": ["msi"]

// 仅 Windows NSIS
"targets": ["nsis"]

// 仅 Linux DEB
"targets": ["deb"]

// 全部格式
"targets": ["deb", "msi", "nsis"]
```

## 前置要求

### Windows
- Node.js 16+
- Rust 1.70+
- Visual Studio Build Tools (C++ 开发工具)
- WebView2 运行时 (自动安装)

### Linux
- Node.js 16+
- Rust 1.70+
- 系统开发库 (见上方安装命令)

## 开发模式

```bash
cd tauri-app
npm run tauri:dev
```

## 注意事项

1. **Windows 签名**: 生产环境建议配置代码签名证书
2. **管理员权限**: Windows 版本需要管理员权限运行 (TUN 模式需要)
3. **Linux 权限**: 某些功能需要 root 权限 (服务安装、TUN 模式)
4. **跨平台构建**: 只能在对应平台上构建该平台的安装包

## 已生成的安装包

- ✅ `mihomo-manager_0.1.0_amd64_restart_fixed.deb` (Linux)
- ⏳ Windows 安装包需要在 Windows 环境下执行构建

## 启动脚本

### Windows
- `start_admin.bat` - 批处理启动脚本
- `start_admin.ps1` - PowerShell 启动脚本

### Linux
- 通过桌面入口或命令行启动: `mihomoapp`
