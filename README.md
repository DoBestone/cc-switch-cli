# CC-Switch CLI

纯命令行版本的 CC-Switch，用于在 Linux 服务器（无图形界面）上管理 Claude Code、Codex、Gemini CLI 等 AI 编程工具的供应商配置。

> **📢 致谢说明**  
> 本项目基于 [farion1231/cc-switch](https://github.com/farion1231/cc-switch) 进行二次开发。  
> 原项目是一个功能完善的图形界面工具，本项目将其重构为纯命令行版本，以适配 Linux 服务器环境。  
> 感谢原作者 [@farion1231](https://github.com/farion1231) 的优秀工作！

## 特性

- 🖥️ **纯 CLI** - 无 GUI 依赖，可在 SSH 会话中使用
- 🎮 **交互式菜单** - 新手友好的图形化菜单界面（支持高级 TUI 模式）
- 🔄 **供应商切换** - 快速切换不同的 API 供应商配置
- 📋 **多应用支持** - Claude Code、Codex CLI、Gemini CLI、OpenCode
- 🧪 **API 测试** - 验证 API Key 有效性和连接延迟
- 📦 **MCP 服务器管理** - 管理 Model Context Protocol 服务器
- 📝 **Prompts 管理** - 管理系统提示词
- 🧩 **Skills 扩展** - 从 GitHub 安装和管理 Skills
- 🌐 **代理支持** - 全局代理设置和自动扫描
- ⚡ **端点测速** - 测试 API 端点延迟
- 🔍 **环境检测** - 检测环境变量冲突
- 🔄 **批量操作** - 批量切换、测试、导出、导入、同步和编辑供应商
- 🚀 **自动更新** - 检测新版本并一键更新（支持预编译二进制）
- 💾 **单一可执行文件** - 编译后仅需一个二进制文件
- 🎨 **高级 TUI** - ratatui 构建的现代化终端界面（实验性）

## 安装

### 🚀 一键安装（推荐）

```bash
# 使用 curl
curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/install.sh | bash

# 或使用 wget
wget -qO- https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/install.sh | bash
```

安装脚本会自动：
1. 检测您的操作系统和架构
2. 下载预编译二进制（如有）
3. 如果没有预编译版本，自动安装 Rust 并从源码编译
4. 将 `cc-switch` 安装到 `/usr/local/bin`

### 🔄 更新到最新版

如果您已安装 cc-switch，可以使用以下方式更新：

```bash
# 方式一：使用内置命令更新（推荐）
cc-switch self-update

# 检查更新但不安装
cc-switch self-update --check

# 方式二：使用更新脚本（适用于旧版本或更新失败时）
curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/update.sh | bash

# 方式三：重新运行安装脚本
curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/install.sh | bash
```

**高级选项：**

```bash
# 强制重新安装（即使已是最新版）
CC_SWITCH_FORCE=1 bash <(curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/update.sh)

# 指定安装特定版本
CC_SWITCH_VERSION=1.0.2 bash <(curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/update.sh)

# 跳过 SHA256 校验（不推荐，仅在网络问题时使用）
CC_SWITCH_NO_VERIFY=1 bash <(curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/update.sh)
```

**更新特性：**
- ✅ 自动检测最新版本
- ✅ 智能下载预编译二进制（Linux 优先 musl 静态版本）
- ✅ SHA256 校验和验证，确保文件完整性
- ✅ 自动备份旧版本（带时间戳）
- ✅ 失败时自动回退到源码编译
- ✅ 支持 curl 或 wget 下载工具
- ✅ 跨平台支持（Linux/macOS/Windows）

### �📦 使用 Cargo 安装

```bash
# 直接从 crates.io 安装（需要先发布）
cargo install cc-switch

# 或从 Git 仓库安装
cargo install --git https://github.com/DoBestone/cc-switch-cli.git
```

### 🔧 从源码编译

```bash
# 克隆仓库
git clone https://github.com/DoBestone/cc-switch-cli.git
cd cc-switch-cli

# 编译 release 版本
cargo build --release

# 安装到系统路径
sudo cp target/release/cc-switch /usr/local/bin/
```

### 编译要求

- Rust 1.70.0+
- Linux / macOS / Windows

## 使用方法

### 🎨 交互式界面（推荐）

CC-Switch 提供两种交互式界面：

**简单菜单模式（默认）**
```bash
cc-switch
```
- 启动时显示精美的欢迎信息面板（类似 Claude CLI）
- 显示版本、当前供应商状态、工作目录
- 提供使用提示和快速操作建议
- 循环菜单，操作后自动返回
- 适合所有终端环境
- 键盘导航，简单易用

**高级 TUI 模式（实验性）**
```bash
cc-switch --tui
```
- 使用 ratatui 框架构建的现代化界面
- 分屏布局，实时预览
- Vim 风格快捷键（j/k/↑/↓）
- 数字快捷键快速跳转
- 更美观的视觉效果

> **提示**: TUI 模式需要支持 ANSI 转义序列的终端

### 🎮 交互式菜单（简单模式）

直接运行不带参数，进入交互式菜单：

```bash
cc-switch
```

菜单功能包括：
- **供应商管理**：列出、查看状态、切换、添加、编辑、测试、删除
- **扩展功能**：MCP 服务器、Prompts、Skills 管理
- **工具**：代理设置、端点测速、环境检测、查看配置

### 基本命令

```bash
# 显示帮助
cc-switch --help

# 列出所有供应商
cc-switch list

# 列出供应商并显示 API Key（脱敏）
cc-switch list --show-key

# 列出 Claude 供应商
cc-switch list --app claude

# 显示当前状态
cc-switch status

# 切换供应商
cc-switch use my-provider --app claude

# 显示配置路径
cc-switch config path
```

### 供应商管理

```bash
# 添加 Claude 供应商（自动测试 API Key）
cc-switch add my-provider --app claude --api-key "sk-xxx" --base-url "https://api.example.com"

# 添加时跳过 API 测试
cc-switch add my-provider --app claude --api-key "sk-xxx" --skip-test

# 编辑供应商
cc-switch edit my-provider --app claude --api-key "sk-new-xxx"
cc-switch edit my-provider --app claude --base-url "https://new-api.example.com"

# 测试供应商 API Key
cc-switch test my-provider --app claude

# 直接测试 API Key（不需要先添加）
cc-switch test --api-key "sk-xxx" --app claude

# 从文件导入
cc-switch add my-provider --app claude --from-file config.json

# 删除供应商
cc-switch remove my-provider --app claude

# 强制删除（跳过确认）
cc-switch remove my-provider --app claude -y
```

### 输出格式

```bash
# 表格格式（默认）
cc-switch list

# JSON 格式
cc-switch list -o json

# YAML 格式
cc-switch list -o yaml

# 禁用彩色输出
cc-switch list --no-color
```

### 扩展功能

#### MCP 服务器管理

```bash
# 列出 MCP 服务器
cc-switch mcp list

# 添加 MCP 服务器
cc-switch mcp add myserver --command "node" --args "server.js"

# 从应用导入
cc-switch mcp import
```

#### Prompts 管理

```bash
# 列出 Prompts
cc-switch prompt list

# 添加 Prompt
cc-switch prompt add --app claude --name "helper" --content "You are a helpful assistant"

# 从应用导入
cc-switch prompt import
```

#### Skills 管理

```bash
# 列出 Skills
cc-switch skill list

# 从 GitHub 安装
cc-switch skill install owner/repo

# 扫描本地目录
cc-switch skill scan

# 同步到所有应用
cc-switch skill sync
```

#### 代理设置

```bash
# 查看代理
cc-switch proxy get

# 设置代理
cc-switch proxy set http://127.0.0.1:7890

# 清除代理
cc-switch proxy clear

# 测试代理
cc-switch proxy test

# 扫描本地代理
cc-switch proxy scan
```

#### 工具命令

```bash
# 端点测速
cc-switch speedtest

# 环境变量检测
cc-switch env check
cc-switch env list
```

#### 自动更新

```bash
# 检测是否有新版本
cc-switch self-update --check

# 检测并执行更新
cc-switch self-update

# 强制重新安装最新版
cc-switch self-update --force

# 也可以使用 upgrade 别名
cc-switch upgrade --check
```

### 批量操作

批量操作功能允许您高效地管理多个供应商和应用配置。

#### 批量切换

将所有应用切换到同一个供应商：

```bash
# 批量切换所有应用到指定供应商
cc-switch batch switch "云雾API"
```

#### 批量测试

并发测试所有供应商的 API 连接性和延迟：

```bash
# 测试所有供应商
cc-switch batch test

# 只测试 Claude 供应商
cc-switch batch test --app claude

# 显示详细错误信息
cc-switch batch test --verbose

# 设置超时时间
cc-switch batch test --timeout 60
```

#### 批量导出/导入

导出和导入配置，用于备份或迁移：

```bash
# 导出所有供应商配置到 YAML 文件
cc-switch batch export backup.yaml

# 只导出 Claude 供应商
cc-switch batch export claude.yaml --app claude

# 导入配置
cc-switch batch import backup.yaml

# 导入并覆盖已存在的配置
cc-switch batch import backup.yaml --overwrite
```

#### 批量删除

删除多个供应商：

```bash
# 批量删除多个供应商（会提示确认）
cc-switch batch remove "供应商1" "供应商2" "供应商3"

# 跳过确认直接删除
cc-switch batch remove "供应商1" "供应商2" -y

# 删除所有应用中的指定供应商
cc-switch batch remove "临时API" --app all
```

#### 批量同步

将一个应用的配置同步到其他应用：

```bash
# 将 Claude 的供应商配置同步到 Codex 和 Gemini
cc-switch batch sync --from claude --to codex gemini

# 同步到所有其他应用
cc-switch batch sync --from claude --to all

# 覆盖已存在的配置
cc-switch batch sync --from claude --to all --overwrite
```

#### 批量编辑

批量修改供应商的配置字段：

```bash
# 修改所有供应商的 base-url
cc-switch batch edit base-url "https://api.example.com" --app all

# 只修改名称包含 "OpenAI" 的供应商的模型
cc-switch batch edit model "gpt-4o" --pattern "OpenAI"

# 修改小模型配置
cc-switch batch edit small-model "claude-haiku-4-20250514" --app claude
```

支持的字段：
- `base-url` (或 `base_url`, `baseUrl`)
- `model`
- `small-model` (或 `small_model`, `smallModel`)

## 配置文件位置

### Linux 服务器推荐

| 路径 | 说明 |
|------|------|
| `~/.cc-switch/` | CC-Switch 配置目录 |
| `~/.cc-switch/cc-switch.db` | SQLite 数据库 |
| `~/.cc-switch/settings.json` | 本地设置 |
| `~/.claude/` | Claude Code 配置 |
| `~/.codex/` | Codex CLI 配置 |
| `~/.gemini/` | Gemini CLI 配置 |

### 环境变量

可通过环境变量自定义路径：

```bash
export CCSWITCH_CONFIG_DIR="$HOME/.config/cc-switch"
export CCSWITCH_CLAUDE_CONFIG_DIR="$HOME/.config/claude"
```

支持 XDG Base Directory 规范：

```bash
export XDG_CONFIG_HOME="$HOME/.config"
# cc-switch 将使用 ~/.config/cc-switch/
```

## 项目结构

```
cc-switch-cli/
├── Cargo.toml              # 工作区配置
├── ccswitch-core/          # 核心库 (lib crate)
│   ├── src/
│   │   ├── lib.rs          # 公共 API
│   │   ├── app_config.rs   # 应用类型定义
│   │   ├── config.rs       # 配置文件处理
│   │   ├── database/       # SQLite 数据持久化
│   │   ├── error.rs        # 错误类型
│   │   ├── provider.rs     # 供应商数据结构
│   │   ├── services/       # 业务逻辑层
│   │   ├── settings.rs     # 本地设置
│   │   └── store.rs        # 应用状态
│   └── Cargo.toml
└── ccswitch-cli/           # CLI 工具 (bin crate)
    ├── src/
    │   ├── main.rs         # 入口
    │   ├── cli.rs          # clap 参数定义
    │   ├── commands/       # 命令实现
    │   └── output.rs       # 输出格式化
    └── Cargo.toml
```

## 与原项目的关系

本项目基于 [cc-switch](https://github.com/farion1231/cc-switch) 进行二次开发：

- 复用原项目的核心业务逻辑（供应商管理、配置同步等）
- 移除 Tauri/GUI 依赖
- 重构为纯 Rust CLI 工具
- 保持数据库格式兼容

## 功能对比

| 功能 | 命令行 | 交互式菜单 | 批量操作 |
|------|--------|------------|----------|
| 供应商管理 | ✅ | ✅ | ✅ |
| MCP 服务器 | ✅ | ✅ | ❌ |
| Prompts | ✅ | ✅ | ❌ |
| Skills | ✅ | ✅ | ❌ |
| 代理设置 | ✅ | ✅ | ❌ |
| 端点测速 | ✅ | ✅ | ❌ |
| 环境检测 | ✅ | ✅ | ❌ |
| 批量切换 | ✅ | ❌ | ✅ |
| 批量测试 | ✅ | ❌ | ✅ |
| 批量导出/导入 | ✅ | ❌ | ✅ |
| 批量同步 | ✅ | ❌ | ✅ |
| 批量编辑 | ✅ | ❌ | ✅ |

## 后续计划

- [x] ✅ TUI 支持 (使用 ratatui) - 高级 TUI 模式已实现
- [ ] 订阅同步功能
- [x] ✅ 配置导入导出 - 批量导出/导入已实现
- [x] ✅ MCP 服务器管理
- [x] ✅ 交互式菜单
- [x] ✅ 批量操作增强 - 完整的批量操作系统已实现

## 许可证

MIT License - 与原项目保持一致
