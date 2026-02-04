#!/usr/bin/env bash
# CC-Switch CLI 更新脚本
# 用法: curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/update.sh | bash
#
# 环境变量:
#   CC_SWITCH_VERSION - 指定安装版本 (默认: latest)
#   CC_SWITCH_FORCE   - 强制重新安装 (设为 1 启用)
#   CC_SWITCH_NO_VERIFY - 跳过 SHA256 校验 (不推荐)

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
NO_VERIFY="${CC_SWITCH_NO_VERIFY:-0}"
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
        Linux*)
            OS_TYPE="linux"
            EXE_EXT=""
            ;;
        Darwin*)
            OS_TYPE="darwin"
            EXE_EXT=""
            ;;
        CYGWIN*|MINGW*|MSYS*)
            OS_TYPE="windows"
            EXE_EXT=".exe"
            BINARY_NAME="${BINARY_NAME}.exe"
            ;;
        *)
            error "不支持的操作系统: $OS"
            ;;
    esac

    case "$ARCH" in
        x86_64|amd64)  ARCH_TYPE="x86_64" ;;
        arm64|aarch64) ARCH_TYPE="aarch64" ;;
        armv7l)        ARCH_TYPE="armv7" ;;
        i686|i386)
            warn "32位系统支持有限"
            ARCH_TYPE="x86_64"
            ;;
        *)
            error "不支持的架构: $ARCH"
            ;;
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
        warn "未检测到已安装的 cc-switch，将执行全新安装"
        # 默认安装目录
        if [ -w "/usr/local/bin" ]; then
            INSTALL_DIR="/usr/local/bin"
        elif [ -w "$HOME/.local/bin" ]; then
            INSTALL_DIR="$HOME/.local/bin"
            mkdir -p "$INSTALL_DIR"
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

    # 尝试使用 curl
    if command -v curl &> /dev/null; then
        LATEST_INFO=$(curl -fsSL "$GITHUB_API" 2>/dev/null) || {
            error "无法获取版本信息，请检查网络连接"
        }
    # 回退到 wget
    elif command -v wget &> /dev/null; then
        LATEST_INFO=$(wget -qO- "$GITHUB_API" 2>/dev/null) || {
            error "无法获取版本信息，请检查网络连接"
        }
    else
        error "需要 curl 或 wget 来下载文件"
    fi

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

# 下载文件（支持 curl 和 wget）
download_file() {
    local url="$1"
    local output="$2"

    if command -v curl &> /dev/null; then
        curl -fsSL -o "$output" "$url" 2>/dev/null
    elif command -v wget &> /dev/null; then
        wget -qO "$output" "$url" 2>/dev/null
    else
        error "需要 curl 或 wget 来下载文件"
    fi
}

# 下载并安装
download_and_install() {
    local version="$1"

    # 尝试多种二进制格式（Linux 优先 musl 静态链接版本）
    local VARIANTS=()
    if [ "$OS_TYPE" = "linux" ]; then
        VARIANTS=("${PLATFORM}-musl" "${PLATFORM}")
    else
        VARIANTS=("${PLATFORM}")
    fi

    for variant in "${VARIANTS[@]}"; do
        local binary_name="${BINARY_NAME%-*}"  # 移除可能的 .exe
        if [ "$OS_TYPE" = "windows" ]; then
            binary_name="${binary_name}.exe"
        fi

        local file_suffix=""
        if [ "$OS_TYPE" = "windows" ]; then
            file_suffix=".exe"
        fi

        if [ "$version" = "latest" ]; then
            DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/cc-switch-${variant}${file_suffix}"
            CHECKSUM_URL="https://github.com/$REPO/releases/latest/download/cc-switch-${variant}${file_suffix}.sha256"
        else
            DOWNLOAD_URL="https://github.com/$REPO/releases/download/v${version}/cc-switch-${variant}${file_suffix}"
            CHECKSUM_URL="https://github.com/$REPO/releases/download/v${version}/cc-switch-${variant}${file_suffix}.sha256"
        fi

        info "尝试下载: cc-switch-${variant}${file_suffix}"

        # 创建临时文件
        TMP_FILE=$(mktemp)

        # 下载
        if download_file "$DOWNLOAD_URL" "$TMP_FILE"; then
            # 验证下载
            if [ -s "$TMP_FILE" ]; then
                # 验证校验和
                if verify_checksum "$TMP_FILE" "$CHECKSUM_URL"; then
                    chmod +x "$TMP_FILE"

                    # 验证二进制（Unix 系统）
                    if [ "$OS_TYPE" != "windows" ]; then
                        if ! "$TMP_FILE" --version &>/dev/null; then
                            warn "二进制文件验证失败，尝试下一个变体"
                            rm -f "$TMP_FILE"
                            continue
                        fi
                    fi

                    # 成功，继续安装
                    install_binary "$TMP_FILE"
                    return 0
                else
                    warn "校验和验证失败，尝试下一个变体"
                    rm -f "$TMP_FILE"
                    continue
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
            echo "支持的平台："
            echo -e "  • Linux x86_64: ${GREEN}cc-switch-linux-x86_64${NC} 或 ${GREEN}cc-switch-linux-x86_64-musl${NC}"
            echo -e "  • Linux ARM64:  ${GREEN}cc-switch-linux-aarch64${NC} 或 ${GREEN}cc-switch-linux-aarch64-musl${NC}"
            echo -e "  • Linux ARMv7:  ${GREEN}cc-switch-linux-armv7${NC}"
            echo -e "  • macOS Intel:  ${GREEN}cc-switch-darwin-x86_64${NC}"
            echo -e "  • macOS ARM64:  ${GREEN}cc-switch-darwin-aarch64${NC}"
            echo -e "  • Windows x64:  ${GREEN}cc-switch-windows-x86_64.exe${NC}"
            echo ""
            echo "下载后手动安装:"
            if [ "$OS_TYPE" = "windows" ]; then
                echo -e "  ${GREEN}move cc-switch-windows-x86_64.exe C:\\Windows\\System32\\cc-switch.exe${NC}"
            else
                echo -e "  ${GREEN}chmod +x cc-switch-*${NC}"
                echo -e "  ${GREEN}sudo mv cc-switch-* /usr/local/bin/cc-switch${NC}"
            fi
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
        BACKUP_FILE="${CURRENT_BIN}.backup-$(date +%Y%m%d-%H%M%S)"
        info "备份当前版本到: $BACKUP_FILE"
        if [ -w "$CURRENT_BIN" ]; then
            cp "$CURRENT_BIN" "$BACKUP_FILE"
        else
            sudo cp "$CURRENT_BIN" "$BACKUP_FILE"
        fi
    fi

    # 安装新版本
    TARGET_PATH="$INSTALL_DIR/$BINARY_NAME"
    info "安装到: $TARGET_PATH"

    # Windows 特殊处理
    if [ "$OS_TYPE" = "windows" ]; then
        if [ -w "$INSTALL_DIR" ]; then
            cp "$TMP_FILE" "$TARGET_PATH"
        else
            # Windows 可能需要管理员权限
            cp "$TMP_FILE" "$TARGET_PATH" 2>/dev/null || {
                warn "需要管理员权限，请以管理员身份运行此脚本"
                exit 1
            }
        fi
    else
        if [ -w "$INSTALL_DIR" ]; then
            mv "$TMP_FILE" "$TARGET_PATH"
        else
            sudo mv "$TMP_FILE" "$TARGET_PATH"
        fi
        chmod +x "$TARGET_PATH"
    fi

    success "二进制文件已安装"
}

# 从源码编译
build_from_source() {
    info "准备从源码编译..."

    # 检查 git
    if ! command -v git &> /dev/null; then
        error "需要 git 来克隆仓库，请先安装 git"
    fi

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
    git clone --depth 1 "https://github.com/$REPO.git" cc-switch-cli || error "克隆仓库失败"
    cd cc-switch-cli

    info "编译 release 版本 (可能需要几分钟)..."
    cargo build --release || error "编译失败"

    # 安装
    local binary_path="target/release/cc-switch"
    if [ "$OS_TYPE" = "windows" ]; then
        binary_path="target/release/cc-switch.exe"
    fi

    TARGET_PATH="$INSTALL_DIR/$BINARY_NAME"
    if [ -w "$INSTALL_DIR" ]; then
        cp "$binary_path" "$TARGET_PATH"
    else
        sudo cp "$binary_path" "$TARGET_PATH"
    fi

    chmod +x "$TARGET_PATH" 2>/dev/null || true

    success "编译安装完成"
}

# 验证安装
verify_installation() {
    if [ -f "$INSTALL_DIR/$BINARY_NAME" ]; then
        NEW_VERSION=$("$INSTALL_DIR/$BINARY_NAME" --version 2>/dev/null | head -1 | awk '{print $2}' || echo "unknown")
    else
        error "安装失败：找不到二进制文件"
    fi

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
        if [ "$OS_TYPE" = "windows" ]; then
            echo "请将以下目录添加到系统 PATH:"
            echo -e "${CYAN}$INSTALL_DIR${NC}"
        else
            echo "请将以下内容添加到 ~/.bashrc 或 ~/.zshrc:"
            echo -e "${CYAN}export PATH=\"\$PATH:$INSTALL_DIR\"${NC}"
        fi
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

# 清理旧备份文件
cleanup_old_backups() {
    if [ -n "$INSTALL_DIR" ] && [ -d "$INSTALL_DIR" ]; then
        # 保留最近3个备份
        local backups=($(ls -t "$INSTALL_DIR"/*.backup* 2>/dev/null || true))
        local count=${#backups[@]}
        if [ $count -gt 3 ]; then
            info "清理旧备份文件 (保留最近3个)..."
            for ((i=3; i<$count; i++)); do
                rm -f "${backups[$i]}"
            done
        fi
    fi
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

    # 验证安装
    verify_installation

    # 清理旧备份
    cleanup_old_backups

    # 显示使用说明
    show_usage
}

# 运行
main "$@"
