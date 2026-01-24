#!/bin/bash
# Linux/macOS 资源准备脚本
# 用于下载和准备 mihomo 二进制文件

set -e

MIHOMO_VERSION="${1:-v1.18.10}"
RESOURCES_DIR="backend/resources"

echo "=== Mihomo 资源准备脚本 (Linux/macOS) ==="
echo ""

# 创建资源目录
mkdir -p "$RESOURCES_DIR"
echo "✓ 创建资源目录: $RESOURCES_DIR"

# 检测系统架构
ARCH=$(uname -m)
case $ARCH in
    x86_64)
        MIHOMO_ARCH="amd64"
        ;;
    aarch64|arm64)
        MIHOMO_ARCH="arm64"
        ;;
    *)
        echo "✗ 不支持的架构: $ARCH"
        exit 1
        ;;
esac

# 检测操作系统
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
case $OS in
    linux)
        MIHOMO_OS="linux"
        ;;
    darwin)
        MIHOMO_OS="darwin"
        ;;
    *)
        echo "✗ 不支持的操作系统: $OS"
        exit 1
        ;;
esac

# 下载 mihomo
echo "正在下载 mihomo $MIHOMO_VERSION ($MIHOMO_OS-$MIHOMO_ARCH)..."
MIHOMO_URL="https://github.com/MetaCubeX/mihomo/releases/download/$MIHOMO_VERSION/mihomo-${MIHOMO_OS}-${MIHOMO_ARCH}-${MIHOMO_VERSION}.gz"
MIHOMO_FILE="mihomo-${MIHOMO_OS}-${MIHOMO_ARCH}.gz"

if command -v wget &> /dev/null; then
    wget -q --show-progress "$MIHOMO_URL" -O "$MIHOMO_FILE" || {
        echo "✗ 下载失败，请检查版本号或网络连接"
        exit 1
    }
elif command -v curl &> /dev/null; then
    curl -L --progress-bar "$MIHOMO_URL" -o "$MIHOMO_FILE" || {
        echo "✗ 下载失败，请检查版本号或网络连接"
        exit 1
    }
else
    echo "✗ 未找到 wget 或 curl，请安装其中之一"
    exit 1
fi

echo "✓ 下载完成"

# 解压
echo "正在解压..."
gunzip -f "$MIHOMO_FILE"
MIHOMO_BINARY="${MIHOMO_FILE%.gz}"

# 设置执行权限并移动
chmod +x "$MIHOMO_BINARY"
mv "$MIHOMO_BINARY" "$RESOURCES_DIR/mihomo"
echo "✓ mihomo 已放置到 $RESOURCES_DIR/mihomo"

# 验证文件
echo ""
echo "=== 验证资源文件 ==="

if [ -f "$RESOURCES_DIR/mihomo" ]; then
    SIZE=$(du -h "$RESOURCES_DIR/mihomo" | cut -f1)
    FILE_SIZE=$(stat -f%z "$RESOURCES_DIR/mihomo" 2>/dev/null || stat -c%s "$RESOURCES_DIR/mihomo" 2>/dev/null)
    
    if [ "$FILE_SIZE" -gt 20000000 ]; then
        echo "✓ mihomo - $SIZE"
        
        # 测试执行
        if "$RESOURCES_DIR/mihomo" -v &>/dev/null; then
            echo "✓ mihomo 可执行"
        else
            echo "⚠ mihomo 可能无法执行，请检查"
        fi
    else
        echo "✗ mihomo - $SIZE (文件太小，可能损坏)"
        exit 1
    fi
else
    echo "✗ mihomo - 未找到"
    exit 1
fi

echo ""
echo "=== 所有资源文件准备完成！ ==="
echo "现在可以运行: npm run tauri build"
