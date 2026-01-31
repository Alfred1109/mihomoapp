# 跨平台支持说明

## 概述

Mihomo Manager 已完整适配 Windows 和 Linux 双平台，包括安装包生成和平台特定功能。

## 安装包支持

### ✅ Windows
- **MSI 安装包** - Windows Installer 标准格式
- **NSIS 安装程序** - 自定义安装向导
- **WebView2** - 自动下载引导程序

### ✅ Linux
- **DEB 包** - Debian/Ubuntu 系统
- **依赖自动声明** - 包含所有必需的系统库

## 平台特定功能适配

### 权限管理

| 功能 | Windows | Linux |
|------|---------|-------|
| 管理员检查 | PowerShell + WindowsPrincipal | libc::geteuid() |
| 提权重启 | RunAs Verb | pkexec/sudo |
| 清单文件 | app.manifest (requireAdministrator) | - |

### 服务管理

| 功能 | Windows | Linux |
|------|---------|-------|
| 服务安装 | sc.exe create | systemctl + .service 文件 |
| 服务启动 | sc.exe start / net start | systemctl start |
| 服务停止 | sc.exe stop / net stop | systemctl stop |
| 服务卸载 | sc.exe delete | systemctl disable + rm |
| 状态查询 | sc.exe query | systemctl is-active |

### 进程管理

| 功能 | Windows | Linux |
|------|---------|-------|
| 终止进程 | taskkill /F /IM mihomo.exe | pkill mihomo |

### 配置路径

| 平台 | 配置目录 | 备份目录 |
|------|----------|----------|
| Windows | `%APPDATA%\Roaming\mihomo` | `%APPDATA%\Roaming\mihomo\backups` |
| Linux | `~/.config/mihomo` | `~/.config/mihomo/backups` |

### 桌面集成

| 功能 | Windows | Linux |
|------|---------|-------|
| 系统托盘 | ✅ 支持 | ✅ 支持 |
| 桌面快捷方式 | ✅ 安装时创建 | ✅ .desktop 文件 |
| 开始菜单 | ✅ 自动添加 | ✅ 应用程序菜单 |

## 代码适配

所有平台特定代码使用条件编译：

```rust
#[cfg(target_os = "windows")]
{
    // Windows 特定代码
}

#[cfg(not(target_os = "windows"))]
{
    // Linux/Unix 特定代码
}
```

### 适配的模块

- `main.rs` - 权限检查、服务管理、重启机制
- `mihomo.rs` - 进程管理
- `backup.rs` - 配置路径
- `app.manifest` - Windows 管理员权限
- `mihomoapp.desktop` - Linux 桌面入口

## 构建配置

### tauri.conf.json

```json
{
  "bundle": {
    "targets": ["deb", "msi", "nsis"],
    "deb": {
      "depends": [
        "libwebkit2gtk-4.1-0",
        "libjavascriptcoregtk-4.1-0",
        "libgtk-3-0",
        "libayatana-appindicator3-1"
      ]
    },
    "windows": {
      "webviewInstallMode": {
        "type": "downloadBootstrapper"
      }
    }
  }
}
```

## 测试状态

| 平台 | 编译 | 打包 | 运行 | 服务模式 | TUN 模式 |
|------|------|------|------|----------|----------|
| Windows 10/11 | ✅ | ⏳ | ⏳ | ⏳ | ⏳ |
| Ubuntu 20.04+ | ✅ | ✅ | ⏳ | ⏳ | ⏳ |
| Debian 11+ | ✅ | ✅ | ⏳ | ⏳ | ⏳ |

说明：
- ✅ 已验证
- ⏳ 待测试
- ❌ 不支持

## 已知限制

1. **跨平台构建**: 必须在对应平台上构建该平台的安装包
2. **Windows 签名**: 未配置代码签名，可能触发 SmartScreen 警告
3. **macOS**: 代码已包含 .icns 图标，但未配置 macOS 打包目标

## 下一步计划

- [ ] 在 Windows 环境测试 MSI/NSIS 安装包生成
- [ ] 配置代码签名证书
- [ ] 添加 AppImage 支持（通用 Linux 格式）
- [ ] 考虑添加 macOS 支持
