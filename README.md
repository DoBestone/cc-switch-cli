# CC-Switch CLI

[![Release](https://img.shields.io/github/v/release/DoBestone/cc-switch-cli?include_prereleases)](https://github.com/DoBestone/cc-switch-cli/releases)
[![License](https://img.shields.io/github/license/DoBestone/cc-switch-cli)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-blue)](https://github.com/DoBestone/cc-switch-cli/releases)

纯命令行版本的 CC-Switch，用于在 Linux 服务器（无图形界面）上管理 Claude Code、Codex、Gemini CLI、OpenClaw 等 AI 编程工具的供应商配置。

> **📢 致谢说明**
> 本项目基于 [farion1231/cc-switch](https://github.com/farion1231/cc-switch) 进行二次开发。
> 原项目是一个功能完善的图形界面工具，本项目将其重构为纯命令行版本，以适配 Linux 服务器环境。
> 感谢原作者 [@farion1231](https://github.com/farion1231) 的优秀工作！

## ✨ 特性

### 核心功能
- 🖥️ **纯 CLI** - 无 GUI 依赖，可在 SSH 会话中使用
- 🎮 **交互式菜单** - 新手友好的图形化菜单界面（支持高级 TUI 模式）
- 🔄 **供应商切换** - 快速切换不同的 API 供应商配置
- 📋 **多应用支持** - Claude Code、Codex CLI、Gemini CLI、OpenCode、OpenClaw

### 供应商管理
- 🧪 **API 测试** - 验证 API Key 有效性和连接延迟
- 📦 **MCP 服务器管理** - 管理 Model Context Protocol 服务器
- 📝 **Prompts 管理** - 管理系统提示词（CLAUDE.md 等）
- 🧩 **Skills 扩展** - 从 GitHub 安装和管理 Skills
- 🔄 **批量操作** - 批量切换、测试、导出、导入、同步和编辑供应商
- 🔥 **故障转移** - 配置备用供应商，主供应商失败时自动切换

### 网络与代理
- 🌐 **代理支持** - 全局代理设置和自动扫描
- ⚡ **端点测速** - 测试 API 端点延迟
- 🔍 **环境检测** - 检测环境变量冲突

### 云端与统计
- ☁️ **WebDAV 同步** - 配置云端同步，多设备配置共享
- 📊 **使用量统计** - 查看 API 使用量和限额管理

### Web 与更新
- 🌐 **Web 控制器** - 通过浏览器管理配置（带身份验证）
- 🚀 **智能更新** - 自动检测新版本，支持大/中/小版本更新策略
  - 🔴 大版本更新：强制提示，需用户确认
  - 🟡 中版本更新：推荐更新
  - 🟢 小版本更新：选择性更新

### 其他
- 💾 **单一可执行文件** - 编译后仅需一个二进制文件
- 🛡️ **SHA256 校验** - 下载文件完整性验证

## 📥 安装

### 🚀 一键安装（推荐）

```bash
# 使用 curl
curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/install.sh | bash

# 或使用 wget
wget -qO- https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/install.sh | bash
```

安装脚本会自动：
1. 检测您的操作系统和架构
2. 下载预编译二进制（支持 SHA256 校验）
3. 如果没有预编译版本，自动安装 Rust 并从源码编译
4. 将 `cc-switch` 安装到 `/usr/local/bin`

### 🔄 更新到最新版

```bash
# 方式一：使用内置命令更新（推荐）
cc-switch self-update

# 检查更新但不安装
cc-switch self-update --check

# 强制重新安装最新版
cc-switch self-update --force

# 方式二：使用更新脚本
curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/update.sh | bash
```

**高级选项：**

```bash
# 指定安装特定版本
CC_SWITCH_VERSION=1.2.3 bash <(curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/update.sh)

# 强制重新安装
CC_SWITCH_FORCE=1 bash <(curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/update.sh)

# 跳过 SHA256 校验（不推荐）
CC_SWITCH_NO_VERIFY=1 bash <(curl -fsSL https://raw.githubusercontent.com/DoBestone/cc-switch-cli/main/update.sh)
```

### 📦 支持的平台

| 平台 | 架构 | 文件 |
|------|------|------|
| Linux | x86_64 | `cc-switch-linux-x86_64` / `cc-switch-linux-x86_64-musl` |
| Linux | ARM64 | `cc-switch-linux-aarch64` / `cc-switch-linux-aarch64-musl` |
| Linux | ARMv7 | `cc-switch-linux-armv7` |
| macOS | Intel | `cc-switch-darwin-x86_64` |
| macOS | Apple Silicon | `cc-switch-darwin-aarch64` |
| Windows | x86_64 | `cc-switch-windows-x86_64.exe` |

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

**编译要求**: Rust 1.70.0+

## 📖 使用方法

### 🎨 交互式界面（推荐）

```bash
# 简单菜单模式（默认）
cc-switch

# 高级 TUI 模式
cc-switch --tui
```

**启动时自动检查版本更新**，有新版本时会显示提示。

### 基本命令

```bash
# 显示帮助
cc-switch --help

# 列出所有供应商
cc-switch list

# 显示当前状态
cc-switch status

# 切换供应商
cc-switch use my-provider --app claude

# 添加供应商
cc-switch add my-provider --app claude --api-key "sk-xxx" --base-url "https://api.example.com"

# 测试供应商
cc-switch test my-provider --app claude

# 删除供应商
cc-switch remove my-provider --app claude
```

### 批量操作

```bash
# 批量切换所有应用
cc-switch batch switch "云雾API"

# 批量测试所有供应商
cc-switch batch test

# 批量导出配置
cc-switch batch export backup.yaml

# 批量导入配置
cc-switch batch import backup.yaml

# 批量同步（从一个应用到其他应用）
cc-switch batch sync --from claude --to codex gemini

# 批量编辑
cc-switch batch edit base-url "https://api.example.com" --app all
```

### 故障转移

```bash
# 查看故障转移队列
cc-switch failover list --app claude

# 添加备用供应商
cc-switch failover add backup-api --app claude

# 移除备用供应商
cc-switch failover remove backup-api --app claude
```

### 使用量统计

```bash
# 查看使用量汇总
cc-switch usage summary

# 查看趋势
cc-switch usage trends --days 7

# 设置限额
cc-switch usage set-limit my-api --daily 10 --monthly 100
```

### WebDAV 云同步

```bash
# 配置 WebDAV
cc-switch webdav setup --url https://dav.example.com --username user --password pass

# 测试连接
cc-switch webdav test

# 上传配置
cc-switch webdav upload

# 下载配置
cc-switch webdav download
```

### Web 控制器

```bash
# 启动 Web 控制器（默认端口 8000）
cc-switch web

# 自定义端口和登录信息
cc-switch web --port 3000 --user admin --pass secret123

# 仅本地访问
cc-switch web --host 127.0.0.1
```

### OpenClaw 配置

```bash
# 列出 OpenClaw 供应商
cc-switch openclaw list

# 添加供应商
cc-switch openclaw add my-api --base-url https://api.example.com --api-key sk-xxx

# 设置默认模型
cc-switch openclaw default-model --primary gpt-4

# 健康检查
cc-switch openclaw health --fix
```

## 📁 配置文件位置

| 路径 | 说明 |
|------|------|
| `~/.cc-switch/` | CC-Switch 配置目录 |
| `~/.cc-switch/cc-switch.db` | SQLite 数据库 |
| `~/.cc-switch/settings.json` | 本地设置 |
| `~/.claude/` | Claude Code 配置 |
| `~/.codex/` | Codex CLI 配置 |
| `~/.gemini/` | Gemini CLI 配置 |
| `~/.opencode/` | OpenCode 配置 |
| `~/.openclaw/` | OpenClaw 配置 |

## 🔄 版本更新策略

CC-Switch 采用智能版本更新策略：

| 版本类型 | 示例 | 行为 |
|---------|------|------|
| 🔴 大版本 | `1.x.x` → `2.x.x` | 强制提示，需用户确认（可能有不兼容变更） |
| 🟡 中版本 | `1.1.x` → `1.2.x` | 推荐更新，默认提示 |
| 🟢 小版本 | `1.1.1` → `1.1.2` | 选择性更新，静默提示 |

## 📊 功能对比

| 功能 | 命令行 | 交互式菜单 | 批量操作 |
|------|--------|------------|----------|
| 供应商管理 | ✅ | ✅ | ✅ |
| MCP 服务器 | ✅ | ✅ | ❌ |
| Prompts | ✅ | ✅ | ❌ |
| Skills | ✅ | ✅ | ❌ |
| 代理设置 | ✅ | ✅ | ❌ |
| 端点测速 | ✅ | ✅ | ❌ |
| 环境检测 | ✅ | ✅ | ❌ |
| 故障转移 | ✅ | ❌ | ✅ |
| 使用量统计 | ✅ | ❌ | ❌ |
| WebDAV 同步 | ✅ | ❌ | ❌ |
| Web 控制器 | ✅ | ❌ | ❌ |
| 批量切换/测试/导出/导入/同步/编辑 | ✅ | ❌ | ✅ |

## 🛠️ 项目结构

```
cc-switch-cli/
├── Cargo.toml              # 工作区配置
├── ccswitch-core/          # 核心库 (lib crate)
│   └── src/
│       ├── lib.rs          # 公共 API
│       ├── config.rs       # 配置文件处理
│       ├── database/       # SQLite 数据持久化
│       ├── provider.rs     # 供应商数据结构
│       └── services/       # 业务逻辑层
└── ccswitch-cli/           # CLI 工具 (bin crate)
    └── src/
        ├── main.rs         # 入口
        ├── cli.rs          # clap 参数定义
        ├── commands/       # 命令实现
        ├── interactive.rs  # 交互式菜单
        └── tui.rs          # 高级 TUI
```

## 📝 更新日志

### v1.2.3 (2026-03-12)
- ✨ 增强版本检查系统，支持大/中/小版本更新策略
- ✨ 添加启动时自动版本检查
- ✨ 增强 GitHub API 限流处理
- ✨ 为 install.sh 添加 SHA256 校验支持
- 🐛 修复版本解析问题，支持预发布版本

### v1.2.0 - v1.2.2
- ✨ 添加 Web 控制器（带身份验证）
- ✨ 添加 WebDAV 云同步
- ✨ 添加使用量统计
- ✨ 添加故障转移功能
- ✨ 添加 OpenClaw 支持
- 🐛 多项 Bug 修复

[查看完整更新日志](https://github.com/DoBestone/cc-switch-cli/releases)

## 📜 许可证

MIT License - 与原项目保持一致

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

---

**Star ⭐ 本项目以获取最新更新！**