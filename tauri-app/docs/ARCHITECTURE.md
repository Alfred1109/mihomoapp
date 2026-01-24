# Mihomo Manager 架构分析与缺陷评估

## 📐 整体架构概览

### 技术栈

**前端层 (React + TypeScript)**:
- React 18.2 - UI 框架
- Material-UI 5.15 - 组件库
- Vite 4.4 - 构建工具
- i18next - 国际化

**后端层 (Rust + Tauri)**:
- Tauri 1.8 - 跨平台框架
- Tokio - 异步运行时
- Reqwest - HTTP 客户端
- Anyhow - 错误处理

**核心服务**:
- Mihomo - 代理核心引擎

---

## 🏗️ 架构层次图

```
┌─────────────────────────────────────────────────────────────┐
│                        前端层 (React)                        │
├─────────────────────────────────────────────────────────────┤
│  App.tsx (主应用)                                            │
│    ├─ Dashboard (仪表盘)                                     │
│    ├─ SubscriptionManager (订阅管理)                        │
│    ├─ ProxyManager (代理管理)                               │
│    ├─ ConfigManager (配置管理)                              │
│    └─ BackupManager (备份管理)                              │
└─────────────────────────────────────────────────────────────┘
                            ↕ Tauri IPC
┌─────────────────────────────────────────────────────────────┐
│                    Tauri Commands (main.rs)                  │
├─────────────────────────────────────────────────────────────┤
│  - start/stop_mihomo_service                                │
│  - get/save_mihomo_config                                   │
│  - get_proxies / switch_proxy                               │
│  - add/update/delete_subscription                           │
│  - create/restore_backup                                    │
│  - check_admin_privileges                                   │
│  - install/start/stop_mihomo_service_cmd                    │
└─────────────────────────────────────────────────────────────┘
                            ↕
┌─────────────────────────────────────────────────────────────┐
│                    业务逻辑层 (Rust Modules)                 │
├─────────────────────────────────────────────────────────────┤
│  mihomo.rs         │  config.rs        │  subscription.rs   │
│  - 进程管理         │  - 配置读写        │  - 订阅管理        │
│  - API 调用         │  - TUN 模式        │  - 配置生成        │
│  - 延迟测试         │  - 路径管理        │  - 更新订阅        │
│                    │                   │                   │
│  backup.rs         │  validator.rs     │                   │
│  - 备份创建         │  - 配置验证        │                   │
│  - 备份恢复         │  - 规则检查        │                   │
└─────────────────────────────────────────────────────────────┘
                            ↕
┌─────────────────────────────────────────────────────────────┐
│                    数据持久化层                              │
├─────────────────────────────────────────────────────────────┤
│  配置文件          │  订阅存储         │  备份文件          │
│  config.yaml       │  subscriptions.json│  backups/         │
│  (%APPDATA%/mihomo)│  (%APPDATA%/mihomo)│  (%APPDATA%/mihomo)│
└─────────────────────────────────────────────────────────────┘
                            ↕
┌─────────────────────────────────────────────────────────────┐
│                    外部服务层                                │
├─────────────────────────────────────────────────────────────┤
│  Mihomo 进程       │  订阅服务器        │  DNS 服务器        │
│  (localhost:9090)  │  (HTTP/HTTPS)     │  (DoH/UDP)        │
└─────────────────────────────────────────────────────────────┘
```

---

## 🔄 组件协作流程

### 1. 应用启动流程

```
App.tsx 初始化
    ↓
检查 Tauri 环境
    ↓
checkMihomoStatus() ──→ invoke('get_mihomo_service_status')
    ↓                        ↓
checkAdminPrivileges() ──→ invoke('check_admin_privileges')
    ↓                        ↓
设置定时器 (5秒轮询)    main.rs 处理命令
    ↓                        ↓
渲染 UI 组件            调用业务逻辑模块
```

### 2. 启动 Mihomo 流程

```
用户点击启动按钮
    ↓
Dashboard.tsx
    ↓
invoke('start_mihomo_service')
    ↓
main.rs::start_mihomo_service()
    ↓
mihomo.rs::start_mihomo()
    ↓
├─ 检查配置文件存在
├─ 查找 mihomo 二进制
│   ├─ 系统路径 (/usr/bin, /usr/local/bin)
│   └─ 应用目录 (resources/)
├─ 启动进程 (TokioCommand)
└─ 返回进程 ID
    ↓
更新 AppState
    ↓
前端显示运行状态
```

### 3. 配置管理流程

```
ConfigManager.tsx 加载
    ↓
invoke('get_mihomo_config')
    ↓
config.rs::load_config()
    ↓
├─ 获取配置路径 (get_config_path)
├─ 读取 YAML 文件
├─ 解析为 JSON
└─ 返回配置对象
    ↓
前端渲染配置表单
    ↓
用户修改配置
    ↓
invoke('save_mihomo_config', config)
    ↓
config.rs::save_config()
    ↓
├─ 创建备份 (backup.rs)
├─ 转换 JSON → YAML
├─ 写入文件
└─ 返回成功
    ↓
提示用户重启服务
```

### 4. 订阅更新流程

```
SubscriptionManager.tsx
    ↓
invoke('update_subscription', id)
    ↓
subscription.rs::update_subscription()
    ↓
├─ 加载订阅存储 (subscriptions.json)
├─ 构建 HTTP 请求
│   ├─ 设置 User-Agent
│   └─ 可选使用代理
├─ 下载订阅内容
├─ 解析 Base64 编码
├─ 解析 YAML 配置
├─ 提取代理节点
├─ 更新订阅信息
└─ 保存到存储
    ↓
invoke('generate_config_from_subscriptions', ids)
    ↓
subscription.rs::generate_config_from_subscriptions()
    ↓
├─ 合并多个订阅的代理
├─ 创建代理组
├─ 生成完整配置
└─ 保存为 config.yaml
    ↓
提示用户重启服务
```

### 5. 代理切换流程

```
ProxyManager.tsx
    ↓
invoke('get_proxies')
    ↓
mihomo.rs::get_proxies()
    ↓
HTTP GET http://127.0.0.1:9090/proxies
    ↓
Mihomo API 返回代理列表
    ↓
前端渲染代理组和节点
    ↓
用户选择节点
    ↓
invoke('switch_proxy', group, proxy)
    ↓
mihomo.rs::switch_proxy()
    ↓
HTTP PUT http://127.0.0.1:9090/proxies/{group}
    ↓
Mihomo 切换代理
    ↓
前端刷新代理状态
```

---

## ⚠️ 架构缺陷分析

### 🔴 严重缺陷

#### 1. **状态管理混乱**

**问题描述**:
- `AppState` 只在 Rust 后端维护，前端通过轮询同步
- 前端组件各自维护本地状态，容易不一致
- 没有统一的状态管理方案 (Redux/Zustand)

**影响**:
```rust
// main.rs - 后端状态
pub struct AppState {
    pub mihomo_running: bool,
    pub mihomo_process: Option<u32>,
}

// App.tsx - 前端状态
const [mihomoStatus, setMihomoStatus] = useState(false);

// Dashboard.tsx - 组件状态
const [isRunning, setIsRunning] = useState(false);
```

**问题**:
- 前端每 5 秒轮询一次，延迟高
- 多个组件重复请求相同数据
- 状态更新不及时，用户体验差

**建议**:
- 使用 Tauri Events 实现实时状态推送
- 前端使用 Zustand/Redux 统一状态管理
- 减少轮询，改用事件驱动

---

#### 2. **错误处理不完善**

**问题描述**:
- 大量使用 `unwrap()` 和 `expect()`，可能导致 panic
- 错误信息不够详细，难以调试
- 前端错误处理简单，用户体验差

**代码示例**:
```rust
// config.rs:229 - 危险的 unwrap
let config_dir = get_mihomo_config_dir()?;
Ok(config_dir.join("config.yaml"))

// subscription.rs:63 - 可能 panic
let mut storage = load_subscriptions().await.unwrap_or_default();
```

**影响**:
- 应用可能崩溃
- 错误难以追踪
- 用户看到的错误信息不友好

**建议**:
- 统一错误类型，使用 `thiserror` 定义错误枚举
- 所有错误都应该被捕获和处理
- 提供详细的错误上下文
- 前端显示友好的错误提示

---

#### 3. **进程管理不可靠**

**问题描述**:
- 进程启动后没有持续监控
- 进程意外退出无法检测和自动重启
- 停止进程时可能残留僵尸进程

**代码分析**:
```rust
// mihomo.rs:99 - 启动后不再监控
match TokioCommand::new(&mihomo_path)
    .args(["-f", &config_path])
    .spawn()
{
    Ok(mut cmd) => {
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        // 只检查一次，之后不再监控
        match cmd.try_wait() { ... }
    }
}
```

**问题**:
- Mihomo 进程崩溃后应用不知道
- 前端显示"运行中"但实际已停止
- 没有自动重启机制

**建议**:
- 使用进程监控线程持续检查
- 实现进程看门狗 (watchdog)
- 提供自动重启选项
- 记录进程退出原因

---

#### 4. **配置文件并发问题**

**问题描述**:
- 多个模块同时读写配置文件
- 没有文件锁机制
- 可能导致配置损坏

**代码示例**:
```rust
// config.rs - 没有锁保护
pub async fn save_config(config: serde_json::Value) -> Result<()> {
    let config_path = get_config_path()?;
    // 直接写入，没有锁
    fs::write(&config_path, yaml_content)?;
    Ok(())
}

// subscription.rs - 同时写入
pub async fn generate_config_from_subscriptions(...) -> Result<()> {
    // 也会写入 config.yaml
    crate::config::save_config(merged_config).await?;
}
```

**风险**:
- 订阅更新时用户手动保存配置 → 数据竞争
- 配置文件可能被截断或损坏
- 备份可能备份到损坏的配置

**建议**:
- 使用文件锁 (flock)
- 实现配置管理器单例
- 使用原子写入 (write to temp + rename)

---

### 🟡 中等缺陷

#### 5. **API 调用无超时控制**

**问题**:
```rust
// mihomo.rs:276 - 没有超时
let response = client
    .get("http://127.0.0.1:9090/proxies")
    .send()
    .await?;
```

**影响**:
- Mihomo 无响应时应用卡死
- 用户界面冻结

**建议**:
```rust
let response = client
    .get("http://127.0.0.1:9090/proxies")
    .timeout(Duration::from_secs(5))
    .send()
    .await?;
```

---

#### 6. **订阅更新无进度反馈**

**问题**:
- 下载大订阅文件时无进度显示
- 用户不知道是否卡住

**建议**:
- 使用流式下载并报告进度
- 前端显示进度条
- 使用 Tauri Events 推送进度

---

#### 7. **配置验证不完整**

**问题**:
```rust
// validator.rs - 验证不够严格
fn validate_dns(config: &serde_json::Value, warnings: &mut Vec<String>) {
    // 只检查基本字段，不验证值的有效性
    if let Some(nameservers) = dns.get("nameserver") {
        if nameservers.is_empty() {
            warnings.push("未配置DNS服务器".to_string());
        }
    }
}
```

**缺失验证**:
- DNS 服务器格式是否正确
- 端口号是否在有效范围
- IP-CIDR 格式是否正确
- 代理协议是否支持

**建议**:
- 使用正则表达式验证格式
- 验证端口范围 (1-65535)
- 检查必填字段完整性

---

#### 8. **资源泄漏风险**

**问题**:
```rust
// mihomo.rs - 进程句柄可能泄漏
pub async fn start_mihomo() -> Result<u32> {
    match TokioCommand::new(&mihomo_path).spawn() {
        Ok(mut cmd) => {
            // cmd 被移动，但没有保存句柄
            // 无法在后续停止进程
        }
    }
}
```

**影响**:
- 进程无法正确停止
- 可能残留僵尸进程

**建议**:
- 保存进程句柄到全局状态
- 实现 Drop trait 确保清理

---

### 🟢 轻微缺陷

#### 9. **日志系统缺失**

**问题**:
- 没有统一的日志框架
- 使用 `println!` 和 `eprintln!`
- 无法追踪问题

**建议**:
- 使用 `tracing` 或 `log` crate
- 实现日志文件轮转
- 提供日志查看界面

---

#### 10. **性能优化不足**

**问题**:
- 前端每 5 秒轮询状态
- 代理列表每次全量获取
- 没有缓存机制

**建议**:
- 使用 WebSocket 或 Tauri Events
- 实现增量更新
- 添加本地缓存

---

#### 11. **测试覆盖率为 0**

**问题**:
- 没有单元测试
- 没有集成测试
- 代码质量难以保证

**建议**:
- 为核心模块添加单元测试
- 使用 `cargo test`
- 前端使用 Jest/Vitest

---

#### 12. **国际化不完整**

**问题**:
```tsx
// App.tsx - 混合使用中英文
<Tab label="备份管理" />  // 硬编码中文
<Tab label={t('app.config')} />  // 使用 i18n
```

**建议**:
- 所有文本都使用 i18n
- 提供完整的翻译文件

---

## 🎯 架构改进建议

### 短期改进 (1-2 周)

1. **添加统一状态管理**
   - 前端使用 Zustand
   - 减少重复请求

2. **实现事件驱动通信**
   - 使用 Tauri Events
   - 替代轮询机制

3. **完善错误处理**
   - 定义错误枚举
   - 统一错误格式

4. **添加进程监控**
   - 实现 watchdog
   - 自动重启机制

### 中期改进 (1-2 月)

1. **重构配置管理**
   - 实现配置管理器单例
   - 添加文件锁
   - 原子写入

2. **添加日志系统**
   - 集成 tracing
   - 日志文件管理
   - 日志查看界面

3. **完善测试**
   - 单元测试覆盖率 > 60%
   - 集成测试
   - E2E 测试

### 长期改进 (3-6 月)

1. **架构重构**
   - 引入 DDD (领域驱动设计)
   - 分离业务逻辑和基础设施
   - 使用依赖注入

2. **性能优化**
   - 实现缓存层
   - 优化 API 调用
   - 减少内存占用

3. **功能增强**
   - 插件系统
   - 自定义规则编辑器
   - 流量统计可视化

---

## 📊 架构评分

| 维度 | 评分 | 说明 |
|------|------|------|
| **可维护性** | 6/10 | 模块划分清晰，但缺乏文档和测试 |
| **可扩展性** | 5/10 | 添加新功能需要修改多处代码 |
| **可靠性** | 4/10 | 错误处理不完善，进程管理不可靠 |
| **性能** | 6/10 | 基本满足需求，但有优化空间 |
| **安全性** | 7/10 | 基本安全，但配置文件无加密 |
| **用户体验** | 7/10 | 界面友好，但状态同步有延迟 |

**总体评分**: **5.8/10** (及格，但需要改进)

---

## 🔍 关键问题优先级

### P0 (立即修复)
1. ❌ 状态管理混乱 - 影响用户体验
2. ❌ 进程管理不可靠 - 可能导致服务异常
3. ❌ 配置文件并发问题 - 可能损坏数据

### P1 (尽快修复)
4. ⚠️ 错误处理不完善 - 影响调试和稳定性
5. ⚠️ API 调用无超时 - 可能导致卡死
6. ⚠️ 资源泄漏风险 - 长期运行问题

### P2 (计划修复)
7. 📝 日志系统缺失 - 影响问题追踪
8. 📝 测试覆盖率为 0 - 影响代码质量
9. 📝 配置验证不完整 - 用户体验

### P3 (优化改进)
10. 💡 性能优化不足 - 可以更好
11. 💡 订阅更新无进度 - 用户体验
12. 💡 国际化不完整 - 完善度

---

## 📝 总结

**优点**:
- ✅ 模块划分清晰，职责明确
- ✅ 使用现代技术栈
- ✅ 跨平台支持良好
- ✅ 基本功能完整

**缺点**:
- ❌ 状态管理和错误处理需要改进
- ❌ 进程管理不够可靠
- ❌ 缺少测试和日志
- ❌ 性能优化空间大

**建议**:
优先解决 P0 级别的问题，然后逐步改进其他方面。整体架构基础良好，通过持续重构和优化，可以成为一个高质量的应用。
