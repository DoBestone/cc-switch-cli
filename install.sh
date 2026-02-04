#!/usr/bin/env bash
# CC-Switch CLI 一键安装脚本
# 用法: curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/install.sh | bash

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 版本
VERSION="${CC_SWITCH_VERSION:-latest}"
REPO="DoBestone/cc-switch-cli"
INSTALL_DIR="${CC_SWITCH_INSTALL_DIR:-/usr/local/bin}"
BINARY_NAME="cc-switch"

# 打印带颜色的消息
info() { echo -e "${BLUE}ℹ${NC} $1"; }
success() { echo -e "${GREEN}✓${NC} $1"; }
warn() { echo -e "${YELLOW}⚠${NC} $1"; }
error() { echo -e "${RED}✗${NC} $1"; exit 1; }

# 检测操作系统和架构
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"
    
    case "$OS" in
        Linux*)   OS_TYPE="linux" ;;
        Darwin*)  OS_TYPE="darwin" ;;
        CYGWIN*|MINGW*|MSYS*) OS_TYPE="windows" ;;
        *)        error "不支持的操作系统: $OS" ;;
    esac
    
    case "$ARCH" in
        x86_64|amd64)  ARCH_TYPE="x86_64" ;;
        arm64|aarch64) ARCH_TYPE="aarch64" ;;
        armv7l)        ARCH_TYPE="armv7" ;;
        *)             error "不支持的架构: $ARCH" ;;
    esac
    
    PLATFORM="${OS_TYPE}-${ARCH_TYPE}"
    info "检测到平台: $PLATFORM"
}

# 检测包管理器并安装依赖
install_dependencies() {
    if command -v cargo &> /dev/null; then
        info "已安装 Rust/Cargo"
        return 0
    fi
    
    warn "未检测到 Rust，正在安装..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    success "Rust 安装完成"
}

# 从 GitHub 下载预编译二进制（如果有）
download_binary() {
    info "尝试下载预编译二进制..."
    
    if [ "$VERSION" = "latest" ]; then
        DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/${BINARY_NAME}-${PLATFORM}"
    else
        DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/${BINARY_NAME}-${PLATFORM}"
    fi
    
    # 尝试下载
    if curl -fsSL --head "$DOWNLOAD_URL" &> /dev/null; then
        TMP_FILE=$(mktemp)
        curl -fsSL -o "$TMP_FILE" "$DOWNLOAD_URL" || return 1
        chmod +x "$TMP_FILE"
        
        # 安装到目标目录
        if [ -w "$INSTALL_DIR" ]; then
            mv "$TMP_FILE" "$INSTALL_DIR/$BINARY_NAME"
        else
            sudo mv "$TMP_FILE" "$INSTALL_DIR/$BINARY_NAME"
        fi
        
        success "已下载并安装预编译二进制"
        return 0
    else
        warn "未找到预编译二进制，将从源码编译"
        return 1
    fi
}

# 从源码编译安装
build_from_source() {
    info "从源码编译安装..."
    
    # 安装依赖
    install_dependencies
    
    # 克隆仓库
    TMP_DIR=$(mktemp -d)
    cd "$TMP_DIR"
    
    info "克隆仓库..."
    git clone --depth 1 "https://github.com/$REPO.git" cc-switch-cli
    cd cc-switch-cli
    
    info "编译 release 版本 (可能需要几分钟)..."
    cargo build --release
    
    # 安装二进制
    if [ -w "$INSTALL_DIR" ]; then
        cp "target/release/$BINARY_NAME" "$INSTALL_DIR/"
    else
        sudo cp "target/release/$BINARY_NAME" "$INSTALL_DIR/"
    fi
    
    # 清理
    cd /
    rm -rf "$TMP_DIR"
    
    success "编译安装完成"
}

# 验证安装
verify_installation() {
    if command -v "$BINARY_NAME" &> /dev/null; then
        VERSION_INFO=$("$BINARY_NAME" --version 2>/dev/null || echo "unknown")
        success "安装成功: $VERSION_INFO"
    else
        # 检查是否在 PATH 中
        if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
            warn "$INSTALL_DIR 不在 PATH 中"
            echo ""
            echo "请将以下内容添加到 ~/.bashrc 或 ~/.zshrc:"
            echo -e "${CYAN}export PATH=\"\$PATH:$INSTALL_DIR\"${NC}"
            echo ""
        else
            error "安装失败"
        fi
    fi
}

# 显示使用说明
show_usage() {
    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║   CC-Switch 安装完成!                  ║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"
    echo ""
    echo "快速开始:"
    echo -e "  ${GREEN}cc-switch${NC}           进入交互式菜单"
    echo -e "  ${GREEN}cc-switch list${NC}      查看所有供应商"
    echo -e "  ${GREEN}cc-switch --help${NC}    查看帮助"
    echo ""
    echo "更多信息请访问: https://github.com/$REPO"
    echo ""
}

# 主函数
main() {
    echo ""
    echo -e "${CYAN}🔄 CC-Switch CLI 安装程序${NC}"
    echo ""
    
    detect_platform
    
    # 尝试下载预编译二进制，失败则从源码编译
    if ! download_binary; then
        build_from_source
    fi
    
    verify_installation
    show_usage
}

# 运行
main "$@"
