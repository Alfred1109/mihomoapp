# 项目结构重构计划

## 目标
- `src` → `frontend`
- `src-tauri` → `backend`

## 需要修改的文件

### 1. tauri.conf.json (backend/tauri.conf.json)
- `distDir`: "../dist" (保持不变，因为 dist 在根目录)

### 2. vite.config.ts
- `ignored`: ["**/src-tauri/**"] → ["**/backend/**"]

### 3. package.json
- `main`: "src/main.tsx" → "frontend/main.tsx"

### 4. index.html
- 检查是否有引用 src 的路径

## 执行步骤
1. 修改配置文件
2. 重命名文件夹
3. 验证编译
