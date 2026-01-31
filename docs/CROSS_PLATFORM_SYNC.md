# 🌍 跨平台配置一致性指南

本指南介绍如何确保 Mihomo Manager 在不同平台间保持配置一致性，避免因环境差异导致的配置问题。

## 🎯 解决方案概述

### **核心问题**
- 用户在不同平台上手动配置时容易出现不一致
- DNS优化、性能参数、路由规则等关键配置可能遗漏
- 订阅链接与本地配置的兼容性问题
- 配置迁移时的平台适配问题

### **解决方案**
我们提供了**三层配置一致性保障**：

1. **📱 GUI工具** - 图形界面一键标准化
2. **⌨️ 命令行工具** - 脚本化批量处理  
3. **🔧 API接口** - 程序化集成

## 📱 GUI工具使用

### **配置同步界面**

在应用中找到"配置同步"页面，可以进行：

#### **健康检查**
- 📊 配置健康评分（0-100分）
- ⚠️ 问题诊断和优化建议
- 🏥 实时配置状态监控

#### **一键操作**
```
🔧 标准化配置   - 基于现有配置优化
🔄 重置为模板   - 恢复标准配置模板  
📋 生成报告     - 详细配置分析报告
📤 导入配置     - 跨平台配置迁移
```

#### **操作结果**
- ✅ 配置变更记录
- ⚠️ 剩余问题提示  
- 💾 自动备份创建

### **使用流程**

1. **首次使用**：
   - 运行健康检查查看配置状态
   - 点击"标准化配置"一键优化
   - 查看操作结果和建议

2. **平台迁移**：
   - 导出原平台配置文件
   - 在新平台点击"导入配置"
   - 系统自动适配平台差异

3. **定期维护**：
   - 定期运行健康检查
   - 查看配置报告了解优化建议
   - 必要时重新标准化

## ⌨️ 命令行工具

### **安装要求**
```bash
# Python 3.6+
pip install pyyaml
```

### **工具位置**
```bash
# 项目目录下
./scripts/sync-config.py
```

### **基本用法**

#### **检查配置状态**
```bash
python sync-config.py --check
```

输出示例：
```
📊 配置状态检查
==================================================
平台: linux
配置文件: /home/user/.config/mihomo/config.yaml
文件存在: ✅
标准化状态: ❌
问题数量: 3

⚠️ 发现的问题:
  ❌ 未启用HTTP/3 DoH加速
  ❌ 缺少DNS智能分流配置
  ❌ 缺少国外流量代理规则
```

#### **标准化配置**
```bash
python sync-config.py --sync
```

输出示例：
```
🔄 开始标准化配置...
操作结果: ✅ 成功
备份创建: ✅
配置变更: 8项
剩余问题: 0项

🔧 配置变更:
  ⚡ 启用HTTP/3 DoH加速
  🎯 配置DNS智能分流策略
  📋 补充缺失路由规则: 5条
  🚀 禁用IPv6以提升解析速度
```

#### **重置为模板**
```bash
python sync-config.py --reset
```

#### **生成详细报告**
```bash
python sync-config.py --report
```

### **批量处理脚本**

```bash
#!/bin/bash
# 批量标准化多个用户配置

for user in /home/*; do
  if [ -d "$user/.config/mihomo" ]; then
    echo "处理用户: $(basename $user)"
    sudo -u $(basename $user) python sync-config.py --sync
  fi
done
```

## 🔧 API接口集成

### **Tauri Commands**

```typescript
// 检查配置状态
const status = await invoke<ConfigStatus>('check_config_status');

// 标准化配置
const result = await invoke<ConfigSyncResult>('standardize_config');

// 健康检查
const health = await invoke<HealthCheckResult>('config_health_check');

// 配置迁移
const migrationResult = await invoke<ConfigSyncResult>(
  'migrate_config_cross_platform', 
  { sourceConfig: configData }
);
```

### **数据结构**

```typescript
interface ConfigStatus {
  is_standardized: boolean;     // 是否已标准化
  version: string;              // 配置版本
  issues_count: number;         // 问题数量
  last_sync: string | null;     // 最后同步时间
  platform: string;            // 平台名称
}

interface ConfigSyncResult {
  success: boolean;             // 操作是否成功
  changes: string[];            // 配置变更列表
  issues: string[];             // 剩余问题列表
  backup_created: boolean;      // 是否创建备份
  config_path: string;          // 配置文件路径
}
```

## 🌍 跨平台配置路径

### **各平台配置目录**
| 平台 | 配置目录 | 备份目录 |
|------|----------|----------|
| **Linux** | `~/.config/mihomo/` | `~/.config/mihomo/backups/` |
| **Windows** | `%APPDATA%\mihomo\` | `%APPDATA%\mihomo\backups\` |
| **macOS** | `~/.config/mihomo/` | `~/.config/mihomo/backups/` |

### **配置文件结构**
```
mihomo/
├── config.yaml              # 主配置文件
├── config.example.yaml      # 示例配置（仅项目根目录）
└── backups/                 # 自动备份目录
    ├── config_20240131_120000.yaml
    └── config_backup_20240131_130000.yaml
```

## 📋 标准化配置内容

### **DNS优化配置**
```yaml
dns:
  enable: true                 # ✅ 启用DNS
  ipv6: false                  # ✅ 禁用IPv6提升速度
  prefer-h3: true              # ✅ 启用HTTP/3加速
  enhanced-mode: fake-ip       # ✅ Fake-IP模式
  
  # Layer 1: 快速初始DNS (50-100ms)
  default-nameserver:
    - 223.5.5.5                # 阿里DNS
    - 119.29.29.29             # 腾讯DNS
  
  # Layer 2: 主要DoH DNS (加密)
  nameserver:
    - https://doh.pub/dns-query
    - https://dns.alidns.com/dns-query
  
  # Layer 3: 防污染备用DNS
  fallback:
    - https://1.1.1.1/dns-query
    - https://dns.google/dns-query
  
  # 智能分流策略
  nameserver-policy:
    "geosite:cn,private,apple":
      - https://doh.pub/dns-query
    "geosite:geolocation-!cn":
      - https://1.1.1.1/dns-query
    "geosite:category-ads-all": "rcode://success"
```

### **性能优化配置**
```yaml
# ⚡ 性能优化参数
unified-delay: true            # 统一延迟测试
tcp-concurrent: true           # TCP并发连接  
keep-alive-interval: 30        # Keep-Alive间隔
find-process-mode: strict      # 严格进程匹配
global-client-fingerprint: chrome # 客户端指纹
```

### **路由规则优化**
```yaml
rules:
  # 🏠 本地网络直连
  - DOMAIN-SUFFIX,local,DIRECT
  - IP-CIDR,192.168.0.0/16,DIRECT,no-resolve
  
  # 🛡️ 广告屏蔽
  - GEOSITE,category-ads-all,ADBLOCK
  
  # 🏪 国内服务直连
  - GEOSITE,cn,DIRECT
  - GEOSITE,apple-cn,DIRECT
  
  # 🌍 国外服务代理
  - GEOSITE,geolocation-!cn,PROXY
  
  # 🇨🇳 国内IP直连
  - GEOIP,CN,DIRECT,no-resolve
  
  # ⚡ 默认代理
  - MATCH,PROXY
```

## 🔍 问题诊断

### **常见问题及解决方案**

#### **配置文件不存在**
```bash
# 创建默认配置
python sync-config.py --reset
```

#### **DNS解析慢**
- ✅ 检查IPv6是否已禁用
- ✅ 确认HTTP/3加速已启用
- ✅ 验证智能分流配置

#### **国内网站打不开**
- ✅ 检查`GEOSITE,cn,DIRECT`规则
- ✅ 确认`GEOIP,CN,DIRECT`配置
- ✅ 验证本地网络规则

#### **代理组不兼容**
- ✅ 使用标准代理组名称（PROXY/AUTO/DIRECT）
- ✅ 检查订阅链接代理组命名
- ✅ 手动调整规则引用

### **健康检查评分标准**

| 分数范围 | 状态 | 说明 |
|----------|------|------|
| **90-100** | 🟢 优秀 | 配置完美，无需优化 |
| **70-89** | 🟡 良好 | 少量问题，建议优化 |
| **50-69** | 🟠 一般 | 多个问题，需要标准化 |
| **0-49** | 🔴 较差 | 严重问题，建议重置 |

## 🚀 最佳实践

### **配置管理流程**
1. **新环境部署**：
   - 使用标准化工具创建配置
   - 导入订阅或手动配置代理
   - 运行健康检查验证

2. **定期维护**：
   - 每月运行配置健康检查
   - 及时应用标准化建议
   - 保持配置文件版本更新

3. **跨平台迁移**：
   - 导出源平台完整配置
   - 使用迁移工具自动适配
   - 验证目标平台功能正常

### **自动化建议**

```bash
# 定时任务 - 每周配置检查
0 2 * * 1 /path/to/sync-config.py --check | logger -t mihomo-config

# 系统启动时自动标准化
@reboot /path/to/sync-config.py --sync --quiet
```

## 📞 技术支持

遇到问题时的排查顺序：

1. 🔍 **运行健康检查** - 了解具体问题
2. 📋 **查看生成报告** - 获得详细诊断
3. 🔧 **尝试标准化** - 自动修复大部分问题
4. 🔄 **重置为模板** - 解决复杂配置冲突
5. 📖 **查阅文档** - 了解配置原理
6. 💬 **社区求助** - 获得人工支持

通过这套完整的跨平台一致性方案，用户无论在哪个平台上使用 Mihomo Manager，都能获得统一、优化的配置体验！🎉
