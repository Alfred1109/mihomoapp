# Mihomo Manager Architecture

## Overview

当前项目是一个 `Tauri + Rust + React` 的桌面应用，负责管理 Mihomo 内核、订阅、运行时配置和系统服务。

代码已经完成一轮结构收口，核心设计可以概括为：

- 前端采用 `App shell + feature components + Zustand event store`
- 后端采用 `main.rs 负责装配 + commands 模块负责 Tauri 命令 + 业务模块负责核心能力`
- 配置分为 `base config` 和 `runtime config`
- 前后端通过 `Tauri invoke + Tauri events` 同步状态

## Directory Layout

```text
tauri-app/
├── frontend/
│   ├── components/
│   │   ├── Dashboard.tsx
│   │   ├── ServiceControl.tsx
│   │   ├── SystemStatusCard.tsx
│   │   ├── IPInfoCard.tsx
│   │   ├── SubscriptionManager.tsx
│   │   ├── ProxyManager.tsx
│   │   └── ConfigManager.tsx
│   ├── store/
│   │   └── appStore.ts
│   ├── hooks/
│   ├── utils/
│   ├── i18n/
│   ├── App.tsx
│   └── main.tsx
├── backend/
│   ├── src/
│   │   ├── commands/
│   │   │   ├── config_subscription.rs
│   │   │   ├── runtime_system.rs
│   │   │   ├── service.rs
│   │   │   └── mod.rs
│   │   ├── main.rs
│   │   ├── mihomo.rs
│   │   ├── subscription.rs
│   │   ├── config.rs
│   │   ├── base_config.rs
│   │   ├── config_manager.rs
│   │   ├── backup.rs
│   │   ├── validator.rs
│   │   ├── events.rs
│   │   └── watchdog.rs
│   ├── resources/
│   └── tauri.conf.json
├── docs/
├── prepare-resources.sh
├── prepare-resources.ps1
└── package.json
```

## Frontend

### UI structure

- `frontend/App.tsx` 是应用壳层，负责页签、通知、语言切换、全局事件初始化
- `Dashboard` 聚合服务状态、系统状态和 IP 信息
- `SubscriptionManager`、`ProxyManager`、`ConfigManager` 分别负责三块核心业务

### State model

前端不是 Redux 风格的大一统 store，而是“轻全局 + 组件自治”的模式：

- `frontend/store/appStore.ts`
  - 保存 `mihomoStatus`
  - 保存管理员权限状态
  - 监听后端事件并记录 `lastConfigChange` / `lastProxyChange` / `lastSubscriptionUpdate`
- 各业务组件仍各自维护本地数据
- 当 store 中的事件时间戳变化时，页面会自动触发刷新

这意味着当前前端已经不是纯轮询架构，但也不是完全中心化的数据流。

### Runtime flow

应用启动时：

1. `App.tsx` 初始化 Tauri 事件监听
2. 读取 Mihomo 运行状态与管理员权限
3. 渲染页签和业务组件
4. 后续通过 store 中的事件时间戳驱动局部刷新

## Backend

### Composition root

`backend/src/main.rs` 现在主要负责：

- Tauri 应用构建
- 系统托盘菜单
- 窗口关闭/隐藏行为
- `watchdog` 初始化
- `ConfigManager` 初始化
- 命令注册

业务命令实现已经拆出 `main.rs`。

### Command modules

#### `commands/config_subscription.rs`

负责配置与订阅命令：

- 读取/保存运行时配置
- 订阅的增删改查
- 订阅导入导出与恢复
- 基础配置导入导出
- 运行时配置重生成
- 配置/订阅相关事件发射

#### `commands/service.rs`

负责服务与应用设置：

- 安装/启动/停止/重启/卸载 Mihomo 服务
- 开机自启
- 静默启动
- 服务状态查询
- Windows `winsw` 辅助逻辑

#### `commands/runtime_system.rs`

负责运行时状态与系统能力：

- Mihomo 运行状态
- 代理列表与切换
- 自动重启设置
- 节点测速
- 当前出口 IP 查询
- 配置校验
- 配置备份管理
- 二进制检查
- 管理员权限检查与提权重启

### Core business modules

#### `mihomo.rs`

- Mihomo 进程启动/停止
- 调用本地 Mihomo HTTP API
- 代理列表读取
- 代理切换
- 节点测速
- 默认运行时配置生成

#### `subscription.rs`

- 订阅存储
- 订阅拉取与解析
- 代理节点提取
- 多订阅合并
- 运行时配置生成

#### `config.rs`

- 运行时配置加载/保存
- 配置升级
- TUN 开关同步
- 默认运行时配置创建

#### `base_config.rs`

- 基础配置读取/保存
- 基础配置重置
- 基础配置与节点列表合并

#### `config_manager.rs`

- 配置文件锁
- 原子写入
- 自动备份与过期清理

#### `watchdog.rs`

- 监控 Mihomo API 健康状态
- 推送 `mihomo-status` 事件
- 自动重启已崩溃服务

## Event Model

后端通过 `backend/src/events.rs` 发出几个核心事件：

- `mihomo-status`
- `config-change`
- `proxy-change`
- `subscription-update`

前端 `appStore` 在启动时订阅这些事件，并将其映射为全局时间戳或状态对象。业务组件依赖这些时间戳决定是否重新拉取数据。

## Configuration model

项目有两层配置：

### Base config

- 持久化“用户期望的基础规则”
- 不直接包含完整订阅节点
- 由 `base_config.rs` 管理

### Runtime config

- 真正交给 Mihomo 运行的配置
- 由基础配置 + 当前有效订阅节点合并生成
- 由 `config.rs` / `subscription.rs` 管理

这种分层让“重置基础配置”和“重新生成运行时配置”可以分开处理。

## Startup sequence

```text
main.rs
  ├─ init ConfigManager
  ├─ init watchdog
  ├─ register commands
  ├─ setup tray and window behavior
  └─ launch Tauri app

frontend/App.tsx
  ├─ init event listeners
  ├─ read service/admin status
  ├─ render tabs
  └─ mount feature components
```

## Current tradeoffs

当前架构的优点：

- `main.rs` 已经从超大入口文件收缩成装配层
- 命令层职责按业务拆开，后续继续扩展成本更低
- 前端已具备基本事件驱动刷新能力
- 配置与订阅逻辑有清晰边界

当前仍然保留的现实取舍：

- 前端业务数据还没有完全收敛进统一 store
- 托盘动作仍直接写在 `main.rs`
- 部分后端命令之间仍有重复错误格式化逻辑
- 自动化测试仍然偏少

## Suggested next steps

- 为 `commands` 模块补最小 smoke test
- 将托盘动作继续抽成 helper 或 tray module
- 补一份命令到前端页面的映射文档
- 为前端状态流引入更统一的数据刷新约定
