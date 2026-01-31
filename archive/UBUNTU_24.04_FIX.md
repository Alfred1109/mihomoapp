# Ubuntu 24.04 依赖兼容性修复

## 问题描述

在 Ubuntu 24.04 上安装 DEB 包时出现以下错误：
```
mihomo-manager : 依赖: libwebkit2gtk-4.0-37 但无法安装它
```

## 原因分析

Ubuntu 24.04 使用新版本的 WebKit2GTK 库：
- **Ubuntu 24.04**: `libwebkit2gtk-4.1-0`
- **旧版本 Ubuntu**: `libwebkit2gtk-4.0-37`

Tauri 构建系统在某些情况下会自动添加多个版本的依赖，导致包含了 Ubuntu 24.04 不支持的旧版本依赖。

## 解决方案

### 方案1：使用替代依赖（已实施）

修改 `tauri-app/src-tauri/tauri.conf.json` 中的 `deb.depends` 配置：

```json
"deb": {
  "depends": [
    "libwebkit2gtk-4.1-0 | libwebkit2gtk-4.0-37",
    "libayatana-appindicator3-1",
    "libgtk-3-0"
  ],
  "files": {},
  "desktopTemplate": "mihomoapp.desktop"
}
```

使用 `|` 符号指定替代依赖：
- 优先使用 `libwebkit2gtk-4.1-0`（Ubuntu 24.04）
- 如果不可用则回退到 `libwebkit2gtk-4.0-37`（旧版本）

这样可以同时兼容新旧版本的 Ubuntu。

### 方案2：仅支持 Ubuntu 24.04

如果只需要支持 Ubuntu 24.04，可以只指定新版本依赖：

```json
"deb": {
  "depends": [
    "libwebkit2gtk-4.1-0",
    "libayatana-appindicator3-1",
    "libgtk-3-0"
  ]
}
```

## 重新构建

修改配置后，需要重新构建 DEB 包：

```bash
cd tauri-app
npm run tauri:build
```

新生成的 DEB 包位置：
```
tauri-app/src-tauri/target/release/bundle/deb/mihomo-manager_0.1.0_amd64.deb
```

## 验证

检查 DEB 包的依赖信息：

```bash
dpkg-deb --info mihomo-manager_0.1.0_amd64.deb | grep Depends
```

应该显示：
```
Depends: libwebkit2gtk-4.1-0 | libwebkit2gtk-4.0-37, libayatana-appindicator3-1, libgtk-3-0
```

## 安装测试

在 Ubuntu 24.04 上安装：

```bash
sudo dpkg -i mihomo-manager_0.1.0_amd64.deb
sudo apt-get install -f  # 自动安装缺失的依赖
```

## 相关文件

- `tauri-app/src-tauri/tauri.conf.json` - Tauri 配置文件
- `BUILD.md` - 构建文档
