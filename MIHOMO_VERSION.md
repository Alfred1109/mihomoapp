# Mihomo 内核版本信息

## 当前版本

**版本**: Mihomo Meta v1.19.19  
**编译时间**: 2026年1月16日  
**编译器**: Go 1.25.6  
**支持特性**: with_gvisor

## 平台支持

### Linux (amd64)
- 二进制文件位置: `/usr/local/bin/mihomo`
- 版本: v1.19.19
- 架构: amd64

### Windows (amd64)
- 二进制文件位置: `tauri-app/src-tauri/resources/mihomo.exe`
- 版本: v1.19.19
- 架构: amd64

## 更新历史

### v1.19.19 (2026-01-24)
- 从 v1.18.8 更新到 v1.19.19
- 更新了 Linux 和 Windows 平台的 mihomo 内核
- 支持最新的功能和修复

### v1.18.8 (初始版本)
- 项目初始使用的 mihomo 版本

## 如何更新 mihomo 内核

### 自动更新（推荐）
项目已包含最新版本的 mihomo 内核，无需手动更新。

### 手动更新

#### Linux 平台
```bash
# 下载最新版本
wget https://github.com/MetaCubeX/mihomo/releases/download/v1.19.19/mihomo-linux-amd64-v1.19.19.gz

# 解压
gunzip mihomo-linux-amd64-v1.19.19.gz

# 停止服务
sudo systemctl stop mihomo.service

# 替换二进制文件
sudo cp mihomo-linux-amd64-v1.19.19 /usr/local/bin/mihomo
sudo chmod +x /usr/local/bin/mihomo

# 重启服务
sudo systemctl start mihomo.service
```

#### Windows 平台
1. 从 [Mihomo Releases](https://github.com/MetaCubeX/mihomo/releases) 下载最新的 Windows 版本
2. 解压 `mihomo-windows-amd64-v1.19.19.zip`
3. 替换 `tauri-app/src-tauri/resources/mihomo.exe`
4. 重新构建应用

## 版本检查

### Linux
```bash
mihomo -v
```

### Windows
```cmd
mihomo.exe -v
```

## 官方资源

- **GitHub 仓库**: https://github.com/MetaCubeX/mihomo
- **发布页面**: https://github.com/MetaCubeX/mihomo/releases
- **文档**: https://wiki.metacubex.one/

## 注意事项

1. 更新内核前请备份配置文件
2. 更新后建议测试所有功能是否正常
3. 如遇到问题，可以回退到之前的版本
4. Windows 版本更新需要重新构建应用安装包
