# Mihomo 资源版本与准备

项目默认资源版本为 **Mihomo v1.19.33**。唯一版本与摘要来源为
[`tauri-app/resource-versions.json`](tauri-app/resource-versions.json)，其中 SHA256
固定自 [Mihomo 官方 v1.19.33 GitHub Release API](https://api.github.com/repos/MetaCubeX/mihomo/releases/tags/v1.19.33)
的对应资产 `digest` 字段。

资源准备入口共用 `tauri-app/prepare-resources.mjs`，需要 Node.js 18 或以上版本。
入口按自身目录定位资源，不依赖调用时的工作目录。

```bash
# Linux/macOS：准备当前平台和架构的 Mihomo
bash tauri-app/prepare-resources.sh

# 显式选择其他正式版本；该版本必须在官方 Release API 提供对应资产 SHA256
bash tauri-app/prepare-resources.sh v1.19.33
```

```powershell
# Windows x64：准备 Mihomo，并保留已有 WinSW
powershell -ExecutionPolicy Bypass -File tauri-app/prepare-resources.ps1

# 首次下载或更新 WinSW：必须传入从可信独立渠道预先确认的 SHA256
# 请用真实摘要替换 <trusted-sha256>；不要把本次下载自算的摘要当作来源验证。
powershell -ExecutionPolicy Bypass -File tauri-app/prepare-resources.ps1 `
  -MihomoVersion v1.19.33 -WinswVersion v2.12.0 -WinswSha256 '<trusted-sha256>'
```

WinSW v2.12.0 的[官方发布](https://github.com/winsw/winsw/releases/tag/v2.12.0)
未提供 SHA256 摘要，因此脚本不会无校验地下载和替换它。已有 `winsw.exe` 会保留，
**不代表本次已验证其来源或完整性**；缺失且没有提供可信摘要时，脚本失败退出。

所有下载均先存入资源目录内的临时目录；Mihomo 压缩包必须通过可信 SHA256 校验，
解压后的程序必须能执行 `-v`，且报告的版本、平台和架构必须与请求一致。
全部请求资源准备完成后才替换原文件；下载、校验、解压或执行验证失败会保留原资源并返回非零退出码。
脚本只准备构建资源，不安装、启动或停止系统代理服务。

| 资源 | 路径 | 准备脚本目标 |
|---|---|---|
| Mihomo Linux/macOS | `tauri-app/backend/resources/mihomo` | 当前系统的 x64/ARM64 |
| Mihomo Windows | `tauri-app/backend/resources/mihomo.exe` | Windows x64 |
| WinSW | `tauri-app/backend/resources/winsw.exe` | Windows x64 |

脚本提供资源准备能力不代表该平台的桌面应用已通过构建、安装及网络行为验收。
切换操作系统或架构后，应重新准备对应资源，再构建应用。

## 升级兼容与运行边界

v1.19.27 移除了 `global-client-fingerprint`。应用会在配置生成或保存时将旧的全局默认值转换为对应节点的 `client-fingerprint`，并保留节点中显式设置的值；代理提供者使用经过验证的覆盖表达式保留相同的默认行为。旧运行配置不会因为启动应用或普通读取而被改写；用户明确启动核心时，如需迁移，应用先在独立目录用所选核心校验候选，再备份并原子替换原配置。校验失败时不改写原配置。

v1.19.28 起，provider 节点不再出现在 `/proxies` 顶层。应用同时读取 provider 数据，按来源区分测速；如果不同来源含同名节点，核心的分组接口无法指明实际来源，界面会标明歧义。

Linux 和 Windows 的项目资源已经升级；资源准备不修改操作系统中的 `/usr/bin/mihomo` 或已安装的系统服务。应用启动与新服务安装优先使用项目打包资源。已安装服务启动或重启前会核对实际核心和配置路径；如仍绑定旧路径，需重新安装本项目服务后才能应用新版。请先确认现有网络运行情况，重新安装服务属于单独的实际运行操作。
