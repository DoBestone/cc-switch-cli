#!/usr/bin/env bash
# CC-Switch CLI 更新脚本
# 用法: curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/update.sh | bash
#
# 环境变量:
#   CC_SWITCH_VERSION - 指定安装版本 (默认: latest)
#   CC_SWITCH_FORCE   - 强制重新安装 (设为 1 启用)

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# 配置
VERSION="${CC_SWITCH_VERSION:-latest}"
FORCE="${CC_SWITCH_FORCE:-0}"
REPO="DoBestone/cc-switch-cli"
BINARY_NAME="cc-switch"
GITHUB_API="https://api.github.com/repos/${REPO}/releases/latest"

# 打印带颜色的消息
info() { echo -e "${BLUE}ℹ${NC} $1"; }
success() { echo -e "${GREEN}✓${NC} $1"; }
warn() { echo -e "${YELLOW}⚠${NC} $1"; }
error() { echo -e "${RED}✗${NC} $1"; exit 1; }

# 打印标题
print_header() {
    echo ""
    echo -e "${CYAN}╔════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║   ${BOLD}CC-Switch CLI 更新程序${NC}${CYAN}               ║${NC}"
    echo -e "${CYAN}╚════════════════════════════════════════╝${NC}"
    echo ""
}

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

# 查找当前安装的 cc-switch
find_current_install() {
    CURRENT_BIN=""
    CURRENT_VERSION=""
    INSTALL_DIR=""
    
    # 尝试找到当前安装
    if command -v "$BINARY_NAME" &> /dev/null; then
        CURRENT_BIN="$(command -v "$BINARY_NAME")"
        CURRENT_VERSION=$("$BINARY_NAME" --version 2>/dev/null | head -1 | awk '{print $2}' || echo "unknown")
        INSTALL_DIR="$(dirname "$CURRENT_BIN")"
        info "当前安装: $CURRENT_BIN"
        info "当前版本: v$CURRENT_VERSION"
    else
        warn "未检测到已安装的 cc-switch"
        # 默认安装目录
        if [ -w "/usr/local/bin" ]; then
            INSTALL_DIR="/usr/local/bin"
        else
            INSTALL_DIR="$HOME/.local/bin"
            mkdir -p "$INSTALL_DIR"
        fi
        info "将安装到: $INSTALL_DIR"
    fi
}

# 获取最新版本信息
get_latest_version() {
    info "正在检测最新版本..."
    
    LATEST_INFO=$(curl -fsSL "$GITHUB_API" 2>/dev/null) || error "无法获取版本信息，请检查网络连接"
    
    LATEST_VERSION=$(echo "$LATEST_INFO" | grep -o '"tag_name": *"[^"]*"' | head -1 | sed 's/.*"tag_name": *"v\?\([^"]*\)".*/\1/')
    
    if [ -z "$LATEST_VERSION" ]; then
        error "无法解析最新版本"
    fi
    
    info "最新版本: v$LATEST_VERSION"
}

# 比较版本号
compare_versions() {
    local v1="$1"
    local v2="$2"
    
    # 移除 v 前缀
    v1="${v1#v}"
    v2="${v2#v}"
    
    # 如果版本相同
    if [ "$v1" = "$v2" ]; then
        return 1  # 不需要更新
    fi
    
    # 简单比较（假设语义化版本）
    local IFS='.'
    read -ra V1_PARTS <<< "$v1"
    read -ra V2_PARTS <<< "$v2"
    
    for i in 0 1 2; do
        local p1="${V1_PARTS[$i]:-0}"
        local p2="${V2_PARTS[$i]:-0}"
        
        if [ "$p2" -gt "$p1" ] 2>/dev/null; then
            return 0  # 需要更新
        elif [ "$p2" -lt "$p1" ] 2>/dev/null; then
            return 1  # 当前版本更新
        fi
    done
    
    return 1  # 版本相同
}

# 下载并安装
download_and_install() {
    local version="$1"
    
    # 尝试多种二进制格式
    local VARIANTS=("${PLATFORM}" "${PLATFORM}-musl")
    
    for variant in "${VARIANTS[@]}"; do
        if [ "$version" = "latest" ]; then
            DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/${BINARY_NAME}-${variant}"
        else
            DOWNLOAD_URL="https://github.com/$REPO/releases/download/v${version}/${BINARY_NAME}-${variant}"
        fi
        
        info "尝试下载: ${BINARY_NAME}-${variant}"
        
        # 创建临时文件
        TMP_FILE=$(mktemp)
        
        # 下载
        if curl -fsSL -o "$TMP_FILE" "$DOWNLOAD_URL" 2>/dev/null; then
            # 验证下载
            if [ -s "$TMP_FILE" ]; then
                chmod +x "$TMP_FILE"
                
                # 验证二进制
                if "$TMP_FILE" --version &>/dev/null; then
                    # 成功，继续安装
                    install_binary "$TMP_FILE"
                    return 0
                fi
            fi
        fi
        
        rm -f "$TMP_FILE"
    done
    
    # 所有下载都失败
    warn "未找到适合当前平台的预编译二进制"
    
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
            info "已取消更新"
            exit 0
            ;;
    esac
}

# 安装二进制文件
install_binary() {
    local TMP_FILE="$1"
    # 备份旧版本
    if [ -n "$CURRENT_BIN" ] && [ -f "$CURRENT_BIN" ]; then
        BACKUP_FILE="${CURRENT_BIN}.backup"
        info "备份当前版本到: $BACKUP_FILE"
        cp "$CURRENT_BIN" "$BACKUP_FILE"
    fi
    
    # 安装新版本
    TARGET_PATH="$INSTALL_DIR/$BINARY_NAME"
    info "安装到: $TARGET_PATH"
    
    if [ -w "$INSTALL_DIR" ]; then
        mv "$TMP_FILE" "$TARGET_PATH"
    else
        sudo mv "$TMP_FILE" "$TARGET_PATH"
    fi
    
    chmod +x "$TARGET_PATH"
    
    success "二进制文件已安装"
}

# 从源码编译
build_from_source() {
    info "准备从源码编译..."
    
    # 检查 Rust
    if ! command -v cargo &> /dev/null; then
        warn "未检测到 Rust，正在安装..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
    fi
    
    # 克隆并编译
    TMP_DIR=$(mktemp -d)
    trap "rm -rf '$TMP_DIR'" EXIT
    
    cd "$TMP_DIR"
    info "克隆仓库..."
    git clone --depth 1 "https://github.com/$REPO.git" cc-switch-cli
    cd cc-switch-cli
    
    info "编译 release 版本 (可能需要几分钟)..."
    cargo build --release
    
    # 安装
    TARGET_PATH="$INSTALL_DIR/$BINARY_NAME"
    if [ -w "$INSTALL_DIR" ]; then
        cp "target/release/$BINARY_NAME" "$TARGET_PATH"
    else
        sudo cp "target/release/$BINARY_NAME" "$TARGET_PATH"
    fi
    
    chmod +x "$TARGET_PATH"
    
    success "编译安装完成"
}

# 验证安装
verify_installation() {
    NEW_VERSION=$("$INSTALL_DIR/$BINARY_NAME" --version 2>/dev/null | head -1 | awk '{print $2}' || echo "unknown")
    
    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║         ${BOLD}✓ 更新成功!${NC}${GREEN}                    ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════╝${NC}"
    echo ""
    
    if [ -n "$CURRENT_VERSION" ] && [ "$CURRENT_VERSION" != "unknown" ]; then
        echo -e "  版本变更: ${YELLOW}v${CURRENT_VERSION}${NC} → ${GREEN}v${NEW_VERSION}${NC}"
    else
        echo -e "  已安装版本: ${GREEN}v${NEW_VERSION}${NC}"
    fi
    
    echo ""
    
    # 检查 PATH
    if ! command -v "$BINARY_NAME" &> /dev/null; then
        warn "$INSTALL_DIR 不在 PATH 中"
        echo ""
        echo "请将以下内容添加到 ~/.bashrc 或 ~/.zshrc:"
        echo -e "${CYAN}export PATH=\"\$PATH:$INSTALL_DIR\"${NC}"
        echo ""
    fi
}

# 显示使用说明
show_usage() {
    echo "快速开始:"
    echo -e "  ${GREEN}cc-switch${NC}              进入交互式菜单"
    echo -e "  ${GREEN}cc-switch list${NC}         查看所有供应商"
    echo -e "  ${GREEN}cc-switch self-update${NC}  检测更新"
    echo ""
    echo "更多信息请访问: https://github.com/$REPO"
    echo ""
}

# 迁移旧版本配置 (0.1.0 -> 0.2.0+)
migrate_config() {
    if [ -z "$CURRENT_VERSION" ]; then
        return
    fi
    
    # 检查是否是 0.1.x 版本
    case "$CURRENT_VERSION" in
        0.1.*)
            info "检测到 v0.1.x 版本，检查配置迁移..."
            
            CONFIG_DIR="$HOME/.cc-switch"
            OLD_DB="$CONFIG_DIR/cc-switch.db"
            
            if [ -f "$OLD_DB" ]; then
                info "配置文件兼容，无需迁移"
            fi
            ;;
    esac
}

# 主函数
main() {
    print_header
    
    detect_platform
    find_current_install
    get_latest_version
    
    # 检查是否需要更新
    NEEDS_UPDATE=0
    if [ "$FORCE" = "1" ]; then
        info "强制更新模式"
        NEEDS_UPDATE=1
    elif [ -z "$CURRENT_VERSION" ] || [ "$CURRENT_VERSION" = "unknown" ]; then
        NEEDS_UPDATE=1
    elif compare_versions "$CURRENT_VERSION" "$LATEST_VERSION"; then
        NEEDS_UPDATE=1
    fi
    
    if [ "$NEEDS_UPDATE" = "0" ]; then
        echo ""
        success "已是最新版本 (v$CURRENT_VERSION)"
        echo ""
        echo "如需强制重新安装，请运行:"
        echo -e "  ${CYAN}CC_SWITCH_FORCE=1 bash <(curl -fsSL https://raw.githubusercontent.com/$REPO/main/update.sh)${NC}"
        echo ""
        exit 0
    fi
    
    echo ""
    if [ -n "$CURRENT_VERSION" ] && [ "$CURRENT_VERSION" != "unknown" ]; then
        info "准备更新: v$CURRENT_VERSION → v$LATEST_VERSION"
    else
        info "准备安装: v$LATEST_VERSION"
    fi
    echo ""
    
    # 执行更新
    download_and_install "$LATEST_VERSION"
    
    # 迁移配置
    migrate_config
    
    # 验证安装
    verify_installation
    
    # 显示使用说明
    show_usage
}

# 运行
main "$@"
