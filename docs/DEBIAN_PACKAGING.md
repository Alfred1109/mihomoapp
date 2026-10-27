# Debian 打包依赖检查

当前版本使用 Tauri 1.x，对应 WebKitGTK 4.0、JavaScriptCoreGTK 4.0 和 libsoup2。WebKitGTK 4.1 不能替换 4.0 的 ELF ABI；安装包依赖中不得用 `libwebkit2gtk-4.1-0 | libwebkit2gtk-4.0-37` 掩盖实际链接要求。当前未完成 Ubuntu 24.04 原生 WebKitGTK 4.1 支持。既有安装包可能与当前锁定源码的构建环境不同，检查始终以包内真实 ELF 为准，不能仅凭文件名或版本号推断链接 ABI。

## 构建与检查

在具有匹配开发库的构建环境中运行：

```bash
cd tauri-app
npm run tauri:build:deb
```

该命令保留 Tauri 自动计算的 `Depends`。打包完成后，`scripts/check-deb.py` 使用 `dpkg-deb -x` 只读解包，不执行安装脚本、不安装软件包、不重写原包。它读取 ELF `NEEDED`，核对 WebKit/JavaScriptCore/libsoup 的准确 ABI，然后用构建环境中的 `dpkg-shlibdeps` 计算最低依赖，检查包中的声明能否保证这些要求。缺库、包映射缺失、版本约束不足和可绕过真实 ABI 的替代依赖均返回失败。

2026-10-28 的隔离 Debian 构建中，Tauri 1.x 自动生成的元数据仅覆盖部分 GTK/WebKit 依赖，未满足包内 ELF 经 `dpkg-shlibdeps` 计算的最低要求。因此 `tauri.conf.json` 的 `bundle.deb.depends` 补充以下实测依赖，同时保留托盘所需的 `libayatana-appindicator3-1`：

| 运行库 | 最低版本 |
|---|---|
| `libc6` | `2.34` |
| `libcairo2` | `1.10.0` |
| `libgcc-s1` | `4.2` |
| `libgdk-pixbuf-2.0-0` | `2.36.9` |
| `libglib2.0-0` | `2.35.8` |
| `libgtk-3-0` | `3.21.4` |
| `libjavascriptcoregtk-4.0-18` | 构建器未给出版本下限；保留准确 ABI 包名 |
| `libsoup2.4-1` | `2.4.0` |
| `libssl3` | `3.0.0` |
| `libwebkit2gtk-4.0-37` | `2.21.1` |

这些补充声明对应本次真实构建的链接结果，不是跨发行版兼容声明。每次重新构建仍须通过原有只读检查；升级 Rust/Tauri、切换构建镜像或改变原生依赖后，应以新包的 ELF 和 `dpkg-shlibdeps` 结果重新核对，不能删除版本约束或弱化检查以放行安装包。

检查工具需要 Python 3、`dpkg-deb`、`readelf`、`dpkg-shlibdeps` 及匹配的系统库和包元数据。可单独检查一个包或目录：

```bash
python3 scripts/check-deb.py /path/to/application.deb
python3 scripts/check-deb.py /path/to/deb-directory
```

旧的 `tauri-app/fix-deb-depends.sh` 仅作为兼容入口转交同一只读检查，已不再修改 Depends 或重打包。若构建器生成的元数据不足，检查会保留原包并报错；应修复依赖生成来源，再重新构建。

此检查不能替代干净目标系统中的安装、桌面启动、托盘、权限及代理功能验收，也不会证明未检测的 `dlopen` 插件依赖齐全。`libayatana-appindicator3-1` 等额外托盘依赖仍保留在 Tauri 配置中。

## 回归检查

```bash
python3 -B scripts/test_check_deb.py
```

测试只在临时目录生成包，验证真实 ELF 检查、错误依赖拒绝、版本要求及原包摘要不变，不安装测试包。
