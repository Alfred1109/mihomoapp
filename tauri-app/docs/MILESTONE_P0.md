# P0 架构改进 Milestone

## 📋 概览

**目标**: 解决架构中的 3 个 P0 严重缺陷
**预计时间**: 1-2 周
**优先级**: 🔴 最高

---

## 🎯 改进目标

### 1. 状态管理混乱 → 统一状态管理
- ❌ 当前：前端每 5 秒轮询，多组件重复维护状态
- ✅ 目标：使用 Tauri Events 实时推送 + Zustand 统一管理

### 2. 进程管理不可靠 → 进程监控和自动重启
- ❌ 当前：启动后不再监控，崩溃无法检测
- ✅ 目标：实现 watchdog 持续监控 + 自动重启机制

### 3. 配置文件并发问题 → 安全的配置管理
- ❌ 当前：无文件锁，可能数据竞争和损坏
- ✅ 目标：文件锁保护 + 原子写入 + 配置管理器单例

---

## 📅 实施计划

### Phase 1: 统一状态管理 (3-4 天)

#### Task 1.1: 实现 Tauri Events 状态推送

**文件**: `backend/src/main.rs`, `backend/src/mihomo.rs`

**实现内容**:
```rust
// backend/src/events.rs (新建)
use tauri::Manager;
use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct MihomoStatusEvent {
    pub running: bool,
    pub process_id: Option<u32>,
    pub timestamp: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ConfigChangeEvent {
    pub config_path: String,
    pub timestamp: u64,
}

pub fn emit_mihomo_status(app: &tauri::AppHandle, status: MihomoStatusEvent) {
    app.emit_all("mihomo-status", status).ok();
}

pub fn emit_config_change(app: &tauri::AppHandle, event: ConfigChangeEvent) {
    app.emit_all("config-change", event).ok();
}
```

**修改点**:
1. 创建 `events.rs` 模块
2. 定义事件类型
3. 在状态变化时发送事件
4. 移除前端轮询逻辑

**验收标准**:
- ✅ 启动/停止 mihomo 时立即推送事件
- ✅ 前端实时收到状态更新（< 100ms）
- ✅ 移除所有 5 秒轮询代码

---

#### Task 1.2: 前端集成 Zustand 状态管理

**文件**: `src/store/appStore.ts` (新建), `src/App.tsx`

**实现内容**:
```typescript
// src/store/appStore.ts
import { create } from 'zustand';
import { listen } from '@tauri-apps/api/event';

interface MihomoStatus {
  running: boolean;
  processId: number | null;
  timestamp: number;
}

interface AppStore {
  // 状态
  mihomoStatus: MihomoStatus;
  isAdmin: boolean;
  adminCheckDone: boolean;
  
  // 操作
  setMihomoStatus: (status: MihomoStatus) => void;
  setIsAdmin: (isAdmin: boolean) => void;
  setAdminCheckDone: (done: boolean) => void;
  
  // 初始化监听
  initEventListeners: () => void;
}

export const useAppStore = create<AppStore>((set) => ({
  mihomoStatus: {
    running: false,
    processId: null,
    timestamp: 0,
  },
  isAdmin: false,
  adminCheckDone: false,
  
  setMihomoStatus: (status) => set({ mihomoStatus: status }),
  setIsAdmin: (isAdmin) => set({ isAdmin }),
  setAdminCheckDone: (done) => set({ adminCheckDone: done }),
  
  initEventListeners: () => {
    // 监听 mihomo 状态变化
    listen('mihomo-status', (event: any) => {
      set({
        mihomoStatus: {
          running: event.payload.running,
          processId: event.payload.process_id,
          timestamp: event.payload.timestamp,
        },
      });
    });
    
    // 监听配置变化
    listen('config-change', (event: any) => {
      console.log('Config changed:', event.payload);
    });
  },
}));
```

**修改点**:
1. 安装 `zustand`: `npm install zustand`
2. 创建全局状态 store
3. 在 `App.tsx` 中使用 store
4. 移除组件内部状态

**验收标准**:
- ✅ 所有组件使用统一的 store
- ✅ 状态变化自动同步到所有组件
- ✅ 无重复请求和状态不一致

---

### Phase 2: 进程监控和自动重启 (3-4 天)

#### Task 2.1: 实现进程监控 Watchdog

**文件**: `backend/src/watchdog.rs` (新建)

**实现内容**:
```rust
// backend/src/watchdog.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};
use sysinfo::{System, SystemExt, ProcessExt, Pid};
use tauri::Manager;

pub struct ProcessWatchdog {
    process_id: Arc<RwLock<Option<u32>>>,
    auto_restart: Arc<RwLock<bool>>,
    app_handle: tauri::AppHandle,
}

impl ProcessWatchdog {
    pub fn new(app_handle: tauri::AppHandle) -> Self {
        Self {
            process_id: Arc::new(RwLock::new(None)),
            auto_restart: Arc::new(RwLock::new(true)),
            app_handle,
        }
    }
    
    pub async fn set_process(&self, pid: u32) {
        let mut process_id = self.process_id.write().await;
        *process_id = Some(pid);
    }
    
    pub async fn clear_process(&self) {
        let mut process_id = self.process_id.write().await;
        *process_id = None;
    }
    
    pub async fn set_auto_restart(&self, enabled: bool) {
        let mut auto_restart = self.auto_restart.write().await;
        *auto_restart = enabled;
    }
    
    pub async fn start_monitoring(&self) {
        let process_id = self.process_id.clone();
        let auto_restart = self.auto_restart.clone();
        let app_handle = self.app_handle.clone();
        
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3));
            let mut system = System::new_all();
            
            loop {
                interval.tick().await;
                
                let pid = {
                    let pid_lock = process_id.read().await;
                    *pid_lock
                };
                
                if let Some(pid) = pid {
                    system.refresh_processes();
                    
                    // 检查进程是否存在
                    let process_exists = system.process(Pid::from(pid as usize)).is_some();
                    
                    if !process_exists {
                        tracing::warn!("Mihomo process {} not found, may have crashed", pid);
                        
                        // 清除进程 ID
                        {
                            let mut pid_lock = process_id.write().await;
                            *pid_lock = None;
                        }
                        
                        // 发送进程停止事件
                        crate::events::emit_mihomo_status(
                            &app_handle,
                            crate::events::MihomoStatusEvent {
                                running: false,
                                process_id: None,
                                timestamp: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs(),
                            },
                        );
                        
                        // 检查是否需要自动重启
                        let should_restart = {
                            let restart_lock = auto_restart.read().await;
                            *restart_lock
                        };
                        
                        if should_restart {
                            tracing::info!("Auto-restarting mihomo...");
                            
                            // 等待 2 秒后重启
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            
                            // 调用启动函数
                            match crate::mihomo::start_mihomo().await {
                                Ok(new_pid) => {
                                    tracing::info!("Mihomo restarted with PID: {}", new_pid);
                                    let mut pid_lock = process_id.write().await;
                                    *pid_lock = Some(new_pid);
                                    
                                    // 发送重启成功事件
                                    crate::events::emit_mihomo_status(
                                        &app_handle,
                                        crate::events::MihomoStatusEvent {
                                            running: true,
                                            process_id: Some(new_pid),
                                            timestamp: std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap()
                                                .as_secs(),
                                        },
                                    );
                                }
                                Err(e) => {
                                    tracing::error!("Failed to restart mihomo: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}
```

**依赖添加**:
```toml
# Cargo.toml
[dependencies]
sysinfo = "0.30"
tracing = "0.1"
tracing-subscriber = "0.3"
```

**修改点**:
1. 创建 `watchdog.rs` 模块
2. 在 `main.rs` 中初始化 watchdog
3. 启动/停止时更新 watchdog 状态
4. 添加日志记录

**验收标准**:
- ✅ 每 3 秒检查一次进程状态
- ✅ 进程崩溃后 5 秒内检测到
- ✅ 自动重启成功率 > 95%
- ✅ 发送状态变化事件

---

#### Task 2.2: 添加进程自动重启配置

**文件**: `src/components/Dashboard.tsx`

**实现内容**:
```typescript
// Dashboard.tsx 添加自动重启开关
import { Switch, FormControlLabel } from '@mui/material';

const [autoRestart, setAutoRestart] = useState(true);

const handleAutoRestartChange = async (event: React.ChangeEvent<HTMLInputElement>) => {
  const enabled = event.target.checked;
  setAutoRestart(enabled);
  
  try {
    await invoke('set_auto_restart', { enabled });
    showNotification(
      enabled ? '已启用自动重启' : '已禁用自动重启',
      'success'
    );
  } catch (error) {
    showNotification(`设置失败: ${error}`, 'error');
  }
};

// UI 组件
<FormControlLabel
  control={
    <Switch
      checked={autoRestart}
      onChange={handleAutoRestartChange}
      color="primary"
    />
  }
  label="进程崩溃时自动重启"
/>
```

**验收标准**:
- ✅ 用户可以开关自动重启
- ✅ 设置持久化保存
- ✅ 崩溃时根据设置决定是否重启

---

### Phase 3: 安全的配置管理 (3-4 天)

#### Task 3.1: 实现配置文件锁机制

**文件**: `backend/src/config_manager.rs` (新建)

**实现内容**:
```rust
// backend/src/config_manager.rs
use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::PathBuf;
use anyhow::{Result, Context};
use fs2::FileExt;
use std::fs::File;

pub struct ConfigManager {
    config_path: PathBuf,
    lock: Arc<RwLock<()>>,
}

impl ConfigManager {
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            config_path,
            lock: Arc::new(RwLock::new(())),
        }
    }
    
    /// 读取配置（带锁）
    pub async fn read_config(&self) -> Result<serde_json::Value> {
        let _guard = self.lock.read().await;
        
        let file = File::open(&self.config_path)
            .context("Failed to open config file")?;
        
        // 获取共享锁
        file.lock_shared()
            .context("Failed to acquire shared lock")?;
        
        let content = std::fs::read_to_string(&self.config_path)
            .context("Failed to read config file")?;
        
        let yaml_value: serde_yaml::Value = serde_yaml::from_str(&content)
            .context("Failed to parse YAML")?;
        
        let json_value = serde_json::to_value(yaml_value)
            .context("Failed to convert to JSON")?;
        
        // 锁会在 file drop 时自动释放
        file.unlock().ok();
        
        Ok(json_value)
    }
    
    /// 写入配置（带锁 + 原子写入）
    pub async fn write_config(&self, config: serde_json::Value) -> Result<()> {
        let _guard = self.lock.write().await;
        
        // 先创建备份
        self.create_backup().await?;
        
        // 转换为 YAML
        let yaml_value: serde_yaml::Value = serde_json::from_value(config)
            .context("Failed to convert from JSON")?;
        
        let yaml_content = serde_yaml::to_string(&yaml_value)
            .context("Failed to serialize YAML")?;
        
        // 原子写入：先写临时文件，再重命名
        let temp_path = self.config_path.with_extension("yaml.tmp");
        
        {
            let temp_file = File::create(&temp_path)
                .context("Failed to create temp file")?;
            
            // 获取独占锁
            temp_file.lock_exclusive()
                .context("Failed to acquire exclusive lock")?;
            
            std::fs::write(&temp_path, yaml_content)
                .context("Failed to write temp file")?;
            
            // 同步到磁盘
            temp_file.sync_all()
                .context("Failed to sync temp file")?;
            
            temp_file.unlock().ok();
        }
        
        // 原子重命名
        std::fs::rename(&temp_path, &self.config_path)
            .context("Failed to rename temp file")?;
        
        tracing::info!("Config saved successfully: {:?}", self.config_path);
        
        Ok(())
    }
    
    /// 创建备份
    async fn create_backup(&self) -> Result<()> {
        if !self.config_path.exists() {
            return Ok(());
        }
        
        let backup_dir = self.config_path.parent()
            .context("Failed to get parent dir")?
            .join("backups");
        
        std::fs::create_dir_all(&backup_dir)
            .context("Failed to create backup dir")?;
        
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        let backup_path = backup_dir.join(format!("config_{}.yaml", timestamp));
        
        std::fs::copy(&self.config_path, &backup_path)
            .context("Failed to create backup")?;
        
        tracing::info!("Backup created: {:?}", backup_path);
        
        Ok(())
    }
}

// 全局单例
lazy_static::lazy_static! {
    static ref CONFIG_MANAGER: Arc<RwLock<Option<ConfigManager>>> = Arc::new(RwLock::new(None));
}

pub async fn init_config_manager(config_path: PathBuf) {
    let mut manager = CONFIG_MANAGER.write().await;
    *manager = Some(ConfigManager::new(config_path));
}

pub async fn get_config_manager() -> Result<Arc<RwLock<Option<ConfigManager>>>> {
    Ok(CONFIG_MANAGER.clone())
}
```

**依赖添加**:
```toml
# Cargo.toml
[dependencies]
fs2 = "0.4"  # 文件锁
lazy_static = "1.4"
chrono = "0.4"
```

**修改点**:
1. 创建 `config_manager.rs` 模块
2. 替换 `config.rs` 中的直接读写
3. 在 `main.rs` 中初始化管理器
4. 所有配置操作通过管理器

**验收标准**:
- ✅ 所有配置读写都通过管理器
- ✅ 并发读写不会导致数据损坏
- ✅ 写入失败时配置文件不受影响
- ✅ 自动创建备份

---

#### Task 3.2: 重构现有配置操作

**文件**: `backend/src/config.rs`, `backend/src/subscription.rs`

**修改内容**:
```rust
// config.rs - 使用配置管理器
pub async fn load_config() -> Result<serde_json::Value> {
    let manager_lock = crate::config_manager::get_config_manager().await?;
    let manager_opt = manager_lock.read().await;
    let manager = manager_opt.as_ref()
        .context("Config manager not initialized")?;
    
    manager.read_config().await
}

pub async fn save_config(config: serde_json::Value) -> Result<()> {
    let manager_lock = crate::config_manager::get_config_manager().await?;
    let manager_opt = manager_lock.read().await;
    let manager = manager_opt.as_ref()
        .context("Config manager not initialized")?;
    
    manager.write_config(config).await
}
```

**验收标准**:
- ✅ 所有模块使用统一的配置管理器
- ✅ 移除所有直接文件操作
- ✅ 测试并发场景无问题

---

### Phase 4: 统一错误处理 (2-3 天)

#### Task 4.1: 定义错误枚举

**文件**: `backend/src/error.rs` (新建)

**实现内容**:
```rust
// backend/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("配置错误: {0}")]
    ConfigError(String),
    
    #[error("进程错误: {0}")]
    ProcessError(String),
    
    #[error("网络错误: {0}")]
    NetworkError(String),
    
    #[error("文件系统错误: {0}")]
    FileSystemError(String),
    
    #[error("权限错误: {0}")]
    PermissionError(String),
    
    #[error("验证错误: {0}")]
    ValidationError(String),
    
    #[error("订阅错误: {0}")]
    SubscriptionError(String),
    
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("YAML 错误: {0}")]
    YamlError(#[from] serde_yaml::Error),
    
    #[error("JSON 错误: {0}")]
    JsonError(#[from] serde_json::Error),
    
    #[error("HTTP 错误: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("其他错误: {0}")]
    Other(#[from] anyhow::Error),
}

pub type AppResult<T> = Result<T, AppError>;

impl From<AppError> for String {
    fn from(error: AppError) -> Self {
        error.to_string()
    }
}
```

**依赖添加**:
```toml
# Cargo.toml
[dependencies]
thiserror = "1.0"
```

**修改点**:
1. 创建 `error.rs` 模块
2. 替换所有 `Result<T, String>` 为 `AppResult<T>`
3. 使用 `?` 操作符传播错误
4. 提供详细的错误上下文

**验收标准**:
- ✅ 所有错误都有明确的类型
- ✅ 错误信息包含足够的上下文
- ✅ 前端显示友好的错误提示

---

### Phase 5: 测试和验证 (2-3 天)

#### Task 5.1: 编写单元测试

**文件**: `backend/src/config_manager.rs`, `backend/src/watchdog.rs`

**测试内容**:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_concurrent_config_read() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.yaml");
        
        // 创建测试配置
        std::fs::write(&config_path, "port: 7890").unwrap();
        
        let manager = ConfigManager::new(config_path);
        
        // 并发读取
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let manager = manager.clone();
                tokio::spawn(async move {
                    manager.read_config().await.unwrap()
                })
            })
            .collect();
        
        for handle in handles {
            handle.await.unwrap();
        }
    }
    
    #[tokio::test]
    async fn test_atomic_write() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("config.yaml");
        
        let manager = ConfigManager::new(config_path.clone());
        
        let config = serde_json::json!({
            "port": 7890,
            "socks-port": 7891,
        });
        
        manager.write_config(config.clone()).await.unwrap();
        
        let read_config = manager.read_config().await.unwrap();
        assert_eq!(config, read_config);
    }
}
```

**验收标准**:
- ✅ 所有核心功能有单元测试
- ✅ 测试覆盖率 > 60%
- ✅ 所有测试通过

---

#### Task 5.2: 集成测试

**测试场景**:
1. ✅ 启动 mihomo → 检测状态 → 停止 mihomo
2. ✅ 启动 mihomo → 手动 kill 进程 → 自动重启
3. ✅ 并发修改配置 → 验证数据完整性
4. ✅ 订阅更新时保存配置 → 无冲突
5. ✅ 前端状态实时更新 → 无延迟

**验收标准**:
- ✅ 所有场景测试通过
- ✅ 无数据损坏
- ✅ 无进程泄漏

---

## 📊 进度跟踪

| Phase | 任务 | 预计时间 | 状态 | 负责人 |
|-------|------|---------|------|--------|
| 1 | Tauri Events 状态推送 | 2天 | 🔲 待开始 | - |
| 1 | Zustand 状态管理 | 1天 | 🔲 待开始 | - |
| 2 | 进程监控 Watchdog | 2天 | 🔲 待开始 | - |
| 2 | 自动重启配置 | 1天 | 🔲 待开始 | - |
| 3 | 配置文件锁机制 | 2天 | 🔲 待开始 | - |
| 3 | 重构配置操作 | 1天 | 🔲 待开始 | - |
| 4 | 统一错误处理 | 2天 | 🔲 待开始 | - |
| 5 | 单元测试 | 1天 | 🔲 待开始 | - |
| 5 | 集成测试 | 1天 | 🔲 待开始 | - |

**总计**: 13 天

---

## ✅ 验收标准

### 功能验收
- [ ] 前端状态实时更新（< 100ms 延迟）
- [ ] 进程崩溃后 5 秒内检测并重启
- [ ] 并发配置操作无数据损坏
- [ ] 所有错误都有友好提示

### 性能验收
- [ ] CPU 占用 < 5%（空闲时）
- [ ] 内存占用 < 100MB
- [ ] 状态更新延迟 < 100ms

### 质量验收
- [ ] 单元测试覆盖率 > 60%
- [ ] 所有测试通过
- [ ] 无编译警告
- [ ] 代码通过 clippy 检查

---

## 🔧 开发环境准备

### 依赖安装
```bash
# 前端
cd tauri-app
npm install zustand

# 后端
cd backend
cargo add sysinfo fs2 lazy_static chrono thiserror tracing tracing-subscriber
```

### 开发工具
```bash
# Rust 格式化和检查
cargo fmt
cargo clippy

# 运行测试
cargo test

# 前端开发
npm run tauri:dev
```

---

## 📝 注意事项

### 风险点
1. **状态迁移风险**: 前端状态管理改动较大，需要充分测试
2. **进程监控性能**: watchdog 不应占用过多资源
3. **文件锁兼容性**: Windows 和 Linux 的文件锁行为可能不同
4. **向后兼容**: 确保旧配置文件仍然可用

### 回滚方案
1. 保留旧代码的 git 分支
2. 每个 Phase 完成后创建 tag
3. 出现问题可快速回滚到上一个稳定版本

### 沟通计划
- 每日站会：同步进度和问题
- Phase 完成后：代码 Review
- 最终验收：完整功能演示

---

## 📚 参考资料

- [Tauri Events 文档](https://tauri.app/v1/guides/features/events)
- [Zustand 文档](https://github.com/pmndrs/zustand)
- [fs2 文件锁文档](https://docs.rs/fs2/)
- [thiserror 文档](https://docs.rs/thiserror/)
- [sysinfo 进程监控](https://docs.rs/sysinfo/)

---

**创建时间**: 2026-01-24
**最后更新**: 2026-01-24
**版本**: v1.0
