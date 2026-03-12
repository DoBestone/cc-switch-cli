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
NO_VERIFY="${CC_SWITCH_NO_VERIFY:-0}"

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

# 验证 SHA256 校验和
verify_checksum() {
    local file="$1"
    local checksum_url="$2"

    if [ "$NO_VERIFY" = "1" ]; then
        warn "跳过 SHA256 校验（不推荐）"
        return 0
    fi

    info "验证 SHA256 校验和..."

    # 下载校验和文件
    local checksum_file=$(mktemp)
    if command -v curl &> /dev/null; then
        curl -fsSL -o "$checksum_file" "$checksum_url" 2>/dev/null || {
            warn "无法下载校验和文件，跳过验证"
            rm -f "$checksum_file"
            return 0
        }
    elif command -v wget &> /dev/null; then
        wget -qO "$checksum_file" "$checksum_url" 2>/dev/null || {
            warn "无法下载校验和文件，跳过验证"
            rm -f "$checksum_file"
            return 0
        }
    else
        warn "需要 curl 或 wget 来验证校验和"
        return 0
    fi

    # 读取期望的校验和
    local expected_sum=$(cat "$checksum_file" | awk '{print $1}')
    rm -f "$checksum_file"

    # 计算实际校验和
    local actual_sum=""
    if command -v sha256sum &> /dev/null; then
        actual_sum=$(sha256sum "$file" | awk '{print $1}')
    elif command -v shasum &> /dev/null; then
        actual_sum=$(shasum -a 256 "$file" | awk '{print $1}')
    else
        warn "未找到 sha256sum 或 shasum，跳过验证"
        return 0
    fi

    # 比较校验和
    if [ "$expected_sum" = "$actual_sum" ]; then
        success "SHA256 校验通过"
        return 0
    else
        error "SHA256 校验失败！文件可能已损坏或被篡改"
        return 1
    fi
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

    # 尝试多种二进制格式（Linux 优先 musl 静态链接版本）
    local VARIANTS=()
    if [ "$OS_TYPE" = "linux" ]; then
        VARIANTS=("${PLATFORM}-musl" "${PLATFORM}")
    else
        VARIANTS=("${PLATFORM}")
    fi

    for variant in "${VARIANTS[@]}"; do
        local file_suffix=""
        if [ "$OS_TYPE" = "windows" ]; then
            file_suffix=".exe"
        fi

        if [ "$VERSION" = "latest" ]; then
            DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/${BINARY_NAME}-${variant}${file_suffix}"
            CHECKSUM_URL="https://github.com/$REPO/releases/latest/download/${BINARY_NAME}-${variant}${file_suffix}.sha256"
        else
            DOWNLOAD_URL="https://github.com/$REPO/releases/download/$VERSION/${BINARY_NAME}-${variant}${file_suffix}"
            CHECKSUM_URL="https://github.com/$REPO/releases/download/$VERSION/${BINARY_NAME}-${variant}${file_suffix}.sha256"
        fi

        info "尝试: ${BINARY_NAME}-${variant}${file_suffix}"

        # 尝试下载
        if curl -fsSL --head "$DOWNLOAD_URL" &> /dev/null; then
            TMP_FILE=$(mktemp)
            if curl -fsSL -o "$TMP_FILE" "$DOWNLOAD_URL" 2>/dev/null; then
                # 验证 SHA256 校验和
                if verify_checksum "$TMP_FILE" "$CHECKSUM_URL"; then
                    chmod +x "$TMP_FILE"

                    # 验证二进制文件
                    if [ -s "$TMP_FILE" ] && file "$TMP_FILE" | grep -q "executable"; then
                        # 安装到目标目录
                        if [ -w "$INSTALL_DIR" ]; then
                            mv "$TMP_FILE" "$INSTALL_DIR/$BINARY_NAME"
                        else
                            sudo mv "$TMP_FILE" "$INSTALL_DIR/$BINARY_NAME"
                        fi

                        success "已下载并安装预编译二进制 (${variant})"
                        return 0
                    fi
                fi
            fi
            rm -f "$TMP_FILE"
        fi
    done

    warn "未找到适合当前平台的预编译二进制"
    return 1
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
    
    # 尝试下载预编译二进制
    if download_binary; then
        verify_installation
        show_usage
        exit 0
    fi
    
    # 询问用户是否要从源码编译
    echo ""
    warn "未找到预编译二进制文件"
    echo ""
    echo "可选方案:"
    echo -e "  ${GREEN}1.${NC} 从源码编译 (需要 Rust 工具链，约需 2GB 内存)"
    echo -e "  ${GREEN}2.${NC} 手动下载预编译版本"
    echo -e "  ${GREEN}3.${NC} 退出"
    echo ""
    
    read -p "请选择 [1/2/3]: " choice
    
    case "$choice" in
        1)
            build_from_source
            verify_installation
            show_usage
            ;;
        2)
            echo ""
            echo "请访问 GitHub Releases 页面下载:"
            echo -e "  ${CYAN}https://github.com/$REPO/releases/latest${NC}"
            echo ""
            echo "下载后手动安装:"
            echo -e "  ${GREEN}chmod +x cc-switch-*${NC}"
            echo -e "  ${GREEN}sudo mv cc-switch-* /usr/local/bin/cc-switch${NC}"
            echo ""
            exit 0
            ;;
        *)
            info "已取消安装"
            exit 0
            ;;
    esac
}

# 运行
main "$@"
