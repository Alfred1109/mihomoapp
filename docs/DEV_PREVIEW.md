# 本地界面预览

在 Linux 项目根目录运行：

```bash
cd tauri-app
npm ci
cd ..
./start.sh
# 可选：./start.sh --port 11420
./stop.sh
```

`start.sh` 等待 Vite 就绪后输出地址，默认仅监听 `127.0.0.1:1420`。重复启动复用本项目已经运行的预览；端口被其他进程占用时直接报错。日志和进程状态位于 `.runtime/`，不纳入 Git。

`stop.sh` 校验进程启动时间、工作目录、完整 Vite 入口、随机进程标记、用户和进程组后，才停止此脚本启动的预览。PID 归属不符或状态文件损坏时保留现场并报错，不按进程名或端口终止程序。

这些脚本只管理前端开发预览，不启动 Tauri 桌面程序、Mihomo 或系统服务。浏览器预览不具备 Tauri 后端能力，也不能替代桌面代理操作的验收。其他平台可在 `tauri-app` 使用 `npm run dev`。

# YAML 诊断示例

示例与桌面程序共用库中的 YAML 转换实现，不依赖固定的用户配置路径：

```bash
cd tauri-app/backend
cargo run --example test_yaml_fixed -- /path/to/config.yaml
```
