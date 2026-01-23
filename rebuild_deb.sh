#!/bin/bash

# Ubuntu 24.04 兼容 DEB 包重新构建脚本

set -e

echo "========================================="
echo "重新构建 Mihomo Manager DEB 包"
echo "========================================="

# 进入 tauri-app 目录
cd tauri-app

# 清理旧的构建产物
echo "清理旧的构建产物..."
rm -rf src-tauri/target/release/bundle/deb

# 构建新的 DEB 包
echo "开始构建..."
npm run tauri:build

# 检查构建结果
DEB_FILE="src-tauri/target/release/bundle/deb/mihomo-manager_0.1.0_amd64.deb"

if [ -f "$DEB_FILE" ]; then
    echo ""
    echo "========================================="
    echo "构建成功！"
    echo "========================================="
    echo "DEB 包位置: $DEB_FILE"
    echo ""
    echo "依赖信息："
    dpkg-deb --info "$DEB_FILE" | grep "Depends:"
    echo ""
    echo "安装命令："
    echo "  sudo dpkg -i $DEB_FILE"
    echo "  sudo apt-get install -f"
    echo ""
    
    # 复制到项目根目录
    echo "复制到项目根目录..."
    cp "$DEB_FILE" ../mihomo-manager_0.1.0_amd64.deb
    echo "已复制到: mihomo-manager_0.1.0_amd64.deb"
else
    echo ""
    echo "========================================="
    echo "构建失败！"
    echo "========================================="
    exit 1
fi
