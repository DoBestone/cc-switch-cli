# CC-Switch CLI

纯命令行版本的 CC-Switch，用于在 Linux 服务器（无图形界面）上管理 Claude Code、Codex、Gemini CLI 等 AI 编程工具的供应商配置。

> **📢 致谢说明**  
> 本项目基于 [farion1231/cc-switch](https://github.com/farion1231/cc-switch) 进行二次开发。  
> 原项目是一个功能完善的图形界面工具，本项目将其重构为纯命令行版本，以适配 Linux 服务器环境。  
> 感谢原作者 [@farion1231](https://github.com/farion1231) 的优秀工作！

## 特性

- 🖥️ **纯 CLI** - 无 GUI 依赖，可在 SSH 会话中使用
- 🔄 **供应商切换** - 快速切换不同的 API 供应商配置
- 📋 **多应用支持** - Claude Code、Codex CLI、Gemini CLI、OpenCode
- 📦 **单一可执行文件** - 编译后仅需一个二进制文件
- 🔧 **可扩展** - 代码结构清晰，便于后续增加 TUI 支持

## 安装

### 从源码编译

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

### 基本命令

```bash
# 显示帮助
cc-switch --help

# 列出所有供应商
cc-switch list

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
# 添加 Claude 供应商
cc-switch add my-provider --app claude --api-key "sk-xxx" --base-url "https://api.example.com"

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

## 后续计划

- [ ] TUI 支持 (使用 ratatui)
- [ ] 订阅同步功能
- [ ] 配置导入导出
- [ ] MCP 服务器管理
- [ ] 批量操作支持

## 许可证

MIT License - 与原项目保持一致
