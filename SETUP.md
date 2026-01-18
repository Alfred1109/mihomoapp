# Mihomo Manager 安装设置指南

## 系统要求

- Node.js 16.0 或更高版本
- npm 或 yarn 包管理器
- Windows 10/11 (支持其他平台但未全面测试)
- 管理员权限 (TUN 模式需要)

## 安装步骤

### 1. 安装 Mihomo 核心

首先需要安装 mihomo 代理核心：

**Windows:**
```bash
# 下载最新版本
# 访问 https://github.com/MetaCubeX/mihomo/releases
# 下载适合您系统的版本，如 mihomo-windows-amd64.zip

# 解压到系统路径
# 将 mihomo.exe 放置到 PATH 环境变量中的目录
# 或创建专门目录如 C:\Program Files\mihomo\
```

**验证安装:**
```bash
mihomo -v
```

### 2. 克隆项目

```bash
git clone <repository-url>
cd mihomo-manager
```

### 3. 安装依赖

```bash
# 安装所有依赖
npm run install-deps

# 或者分别安装
cd server && npm install
cd ../client && npm install
```

### 4. 配置环境

在 `server` 目录创建环境配置：

```bash
# server/.env (可选)
PORT=3001
NODE_ENV=development
```

### 5. 启动应用

**开发模式 (推荐初次使用):**
```bash
npm run dev
```

**生产模式:**
```bash
# 构建前端
npm run build

# 启动后端
npm start
```

## 使用说明

### 首次使用

1. **访问应用**: 打开浏览器访问 `http://localhost:3000`

2. **添加订阅**: 
   - 点击 "Subscriptions" 标签
   - 添加您的代理订阅链接
   - 生成配置文件

3. **启动服务**: 
   - 在 Dashboard 中点击 "Start" 按钮
   - 服务将在后台运行

4. **配置代理**: 
   - 在 "Proxies" 标签中选择和切换节点
   - 在 "Config" 标签中调整设置

### TUN 模式设置

TUN 模式提供系统级透明代理：

1. **启用 TUN 模式**:
   - 确保以管理员身份运行
   - 在 Config 页面启用 TUN 模式
   - 保存配置并重启服务

2. **网络设置**:
   - TUN 模式会自动配置系统路由
   - 无需手动设置浏览器代理

### 端口说明

默认端口配置：
- **Web 管理界面**: 3000
- **API 服务**: 3001
- **HTTP 代理**: 7890
- **SOCKS 代理**: 7891
- **控制 API**: 9090
- **DNS 服务**: 53 (TUN 模式)

## 常见问题

### Q: 无法启动 mihomo 服务
A: 检查以下项目：
- mihomo 二进制文件是否正确安装
- 配置文件是否存在且格式正确
- 端口是否被其他程序占用
- 是否有足够的权限

### Q: TUN 模式无法启用
A: TUN 模式需要：
- 管理员权限
- Windows 10/11 支持
- 确保没有其他 VPN 软件冲突

### Q: 订阅链接解析失败
A: 检查：
- 订阅链接是否有效
- 网络连接是否正常
- User-Agent 设置是否正确

### Q: 代理连接失败
A: 排查步骤：
1. 检查节点是否可用
2. 验证防火墙设置
3. 查看系统日志
4. 测试不同节点

## 高级配置

### 自定义配置文件

配置文件位置：
- Windows: `%USERPROFILE%\.config\mihomo\config.yaml`
- 或项目目录: `server\config\config.yaml`

### 服务模式运行

**Windows 服务安装** (需要管理员权限):
```bash
# 使用 NSSM 或类似工具将应用注册为 Windows 服务
# 下载 NSSM: https://nssm.cc/

nssm install "Mihomo Manager" "node" "path\to\server\index.js"
nssm set "Mihomo Manager" Start SERVICE_AUTO_START
nssm start "Mihomo Manager"
```

### 数据备份

重要数据位置：
- 订阅数据: `server/data/subscriptions.json`
- 配置文件: `server/config/config.yaml`
- 配置备份: `server/config/config-backup-*.yaml`

建议定期备份这些文件。

## 安全注意事项

1. **管理员权限**: 仅在需要 TUN 模式时使用
2. **网络安全**: 不要在公共网络上开放 LAN 访问
3. **配置安全**: 避免在配置中硬编码敏感信息
4. **订阅安全**: 只使用可信的订阅源

## 故障排除

### 日志查看

应用日志位置：
- 浏览器开发者控制台 (前端)
- 命令行输出 (后端)
- 系统事件查看器 (Windows 服务)

### 重置配置

如需重置到默认设置：
1. 停止 mihomo 服务
2. 删除配置文件目录
3. 重启应用
4. 重新配置

### 联系支持

遇到问题时：
1. 查看日志文件
2. 检查网络连接
3. 验证配置文件格式
4. 提供详细错误信息

---

**注意**: 请确保遵守当地法律法规使用代理服务。
