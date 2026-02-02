# CC-Switch CLI 技术分析与重构文档

## 一、原项目分析

### 1.1 目录结构分析

原项目 `cc-switch` 的 `src-tauri` 目录包含以下模块：

```
src-tauri/src/
├── app_config.rs       # 应用类型定义（核心）
├── app_store.rs        # 应用存储（Tauri 相关，可忽略）
├── auto_launch.rs      # 开机自启（GUI 专属，忽略）
├── claude_mcp.rs       # Claude MCP 配置（核心）
├── claude_plugin.rs    # Claude 插件（GUI 专属）
├── codex_config.rs     # Codex 配置处理（核心）
├── commands/           # Tauri 命令（部分核心）
├── config.rs           # 配置文件处理（核心）
├── database/           # SQLite 数据库（核心）
├── deeplink/           # 深链接处理（GUI 专属）
├── error.rs            # 错误类型（核心）
├── gemini_config.rs    # Gemini 配置（核心）
├── gemini_mcp.rs       # Gemini MCP（核心）
├── init_status.rs      # 初始化状态（GUI 相关）
├── lib.rs              # 库入口（部分核心）
├── main.rs             # Tauri 主入口（GUI 专属）
├── mcp/                # MCP 服务器管理（核心）
├── opencode_config.rs  # OpenCode 配置（核心）
├── panic_hook.rs       # 崩溃处理（可选）
├── prompt.rs           # 提示词管理（核心）
├── prompt_files.rs     # 提示词文件（核心）
├── provider.rs         # 供应商数据结构（核心）
├── provider_defaults.rs# 默认供应商（核心）
├── proxy/              # 代理服务（可选）
├── services/           # 业务逻辑服务（核心）
├── settings.rs         # 设置管理（核心）
├── store.rs            # 应用状态（核心）
├── tray.rs             # 系统托盘（GUI 专属）
├── usage_script.rs     # 用量脚本（可选）
```

### 1.2 模块分类

#### 核心业务逻辑（必须复用）

| 模块 | 说明 |
|------|------|
| `provider.rs` | 供应商数据结构定义 |
| `config.rs` | 配置文件路径和读写 |
| `error.rs` | 统一错误类型 |
| `database/` | SQLite 数据持久化 |
| `services/provider/` | 供应商 CRUD、切换 |
| `services/config.rs` | 配置服务 |
| `app_config.rs` | AppType 定义 |
| `settings.rs` | 本地设置管理 |
| `store.rs` | AppState 状态封装 |

#### GUI/Tauri 专属（完全忽略）

| 模块 | 说明 |
|------|------|
| `main.rs` | Tauri 应用入口 |
| `lib.rs` | Tauri 插件注册 |
| `tray.rs` | 系统托盘 |
| `auto_launch.rs` | 开机自启 |
| `deeplink/` | 深链接处理 |
| `init_status.rs` | GUI 初始化状态 |
| `claude_plugin.rs` | Claude 桌面插件 |

#### 可选模块

| 模块 | 说明 |
|------|------|
| `proxy/` | 代理服务器 |
| `mcp/` | MCP 服务器管理 |
| `prompt.rs` | 提示词管理 |

### 1.3 核心依赖分析

原项目依赖中需要保留的：

```toml
# 必须保留
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
rusqlite = { version = "0.31", features = ["bundled"] }
dirs = "5.0"
toml = "0.8"
tokio = { version = "1", features = ["..."] }
thiserror = "2.0"
anyhow = "1.0"
log = "0.4"
chrono = "0.4"
indexmap = "2"

# 需要移除的 Tauri 相关
tauri = { ... }
tauri-plugin-* = { ... }
```

## 二、重构设计

### 2.1 新项目结构

```
cc-switch-cli/
├── Cargo.toml              # 工作区配置
├── README.md
├── original/               # 原项目（参考用）
│
├── ccswitch-core/          # 核心库
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # 公共 API 导出
│       ├── app_config.rs   # AppType, McpApps
│       ├── config.rs       # 配置路径和读写
│       ├── error.rs        # AppError
│       ├── provider.rs     # Provider 数据结构
│       ├── settings.rs     # 本地设置
│       ├── store.rs        # AppState
│       ├── database/
│       │   ├── mod.rs      # Database
│       │   └── schema.rs   # 表结构
│       └── services/
│           ├── mod.rs
│           ├── provider.rs # ProviderService
│           └── config.rs   # ConfigService
│
└── ccswitch-cli/           # CLI 工具
    ├── Cargo.toml
    └── src/
        ├── main.rs         # 入口
        ├── cli.rs          # clap 参数定义
        ├── output.rs       # 输出格式化
        └── commands/
            ├── mod.rs
            ├── list.rs
            ├── status.rs
            ├── provider.rs
            └── config.rs
```

### 2.2 CLI 命令设计

```
cc-switch
├── list              # 列出供应商
│   ├── --app         # 指定应用 (claude/codex/gemini/opencode/all)
│   └── --detail      # 显示详细信息
│
├── status            # 显示当前状态
│   └── --app
│
├── use <name>        # 切换供应商
│   └── --app
│
├── add <name>        # 添加供应商
│   ├── --app
│   ├── --api-key
│   ├── --base-url
│   └── --from-file
│
├── remove <name>     # 删除供应商
│   ├── --app
│   └── -y            # 跳过确认
│
├── update            # 更新配置
│   └── --app
│
├── export            # 导出配置
│   ├── --format      # json/yaml/toml
│   ├── --out
│   └── --app
│
├── import <file>     # 导入配置
│   └── --app
│
├── config            # 配置管理
│   ├── path          # 显示路径
│   ├── open          # 打印目录路径
│   └── check         # 检查配置
│
└── version           # 显示版本
```

### 2.3 Linux 服务器配置路径

遵循 XDG Base Directory 规范：

| 用途 | 默认路径 | 环境变量覆盖 |
|------|----------|--------------|
| CC-Switch 配置 | `~/.cc-switch/` | `CCSWITCH_CONFIG_DIR` |
| CC-Switch 数据库 | `~/.cc-switch/cc-switch.db` | - |
| Claude 配置 | `~/.claude/` | `CCSWITCH_CLAUDE_CONFIG_DIR` |
| Codex 配置 | `~/.codex/` | `CCSWITCH_CODEX_CONFIG_DIR` |
| Gemini 配置 | `~/.gemini/` | `CCSWITCH_GEMINI_CONFIG_DIR` |

在 Linux 上支持 `$XDG_CONFIG_HOME`：

```bash
export XDG_CONFIG_HOME="$HOME/.config"
# cc-switch 将使用 ~/.config/cc-switch/
```

## 三、核心调用流程

### 3.1 list 命令流程

```rust
// 1. 初始化状态
let state = AppState::init()?;

// 2. 获取供应商列表
let providers = ProviderService::list(&state, AppType::Claude)?;

// 3. 获取当前供应商
let current = ProviderService::current(&state, AppType::Claude)?;

// 4. 格式化输出
for (id, provider) in providers {
    let status = if id == current { "●" } else { "○" };
    println!("{} {} ({})", status, provider.name, id);
}
```

### 3.2 switch 命令流程

```rust
// 1. 初始化状态
let state = AppState::init()?;

// 2. 查找供应商
let provider = ProviderService::find(&state, AppType::Claude, "my-provider")?;

// 3. 切换
ProviderService::switch(&state, AppType::Claude, &provider.id)?;

// 4. 同步到 live 配置（自动完成）
```

### 3.3 数据流

```
┌─────────────────┐
│   CLI (clap)    │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    Commands     │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ProviderService  │  ◄── 业务逻辑
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    Database     │  ◄── SQLite
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Live Config    │  ◄── ~/.claude/settings.json
└─────────────────┘
```

## 四、编译和运行

### 4.1 编译

```bash
cd cc-switch-cli

# 开发编译
cargo build

# Release 编译（优化大小）
cargo build --release

# 检查编译
cargo check
```

### 4.2 运行

```bash
# 直接运行
cargo run -- list

# 使用编译后的二进制
./target/release/cc-switch list
```

### 4.3 安装

```bash
# 安装到 ~/.cargo/bin/
cargo install --path ccswitch-cli

# 或复制到系统路径
sudo cp target/release/cc-switch /usr/local/bin/
```

## 五、后续扩展

### 5.1 TUI 支持

代码结构已为 TUI 扩展做好准备：

```
ccswitch-tui/           # 新增 TUI crate
├── Cargo.toml
└── src/
    ├── main.rs
    ├── app.rs          # ratatui App
    ├── ui/
    │   ├── mod.rs
    │   ├── list.rs     # 列表视图
    │   └── status.rs   # 状态视图
    └── events.rs       # 键盘事件
```

### 5.2 新增依赖

```toml
[dependencies]
ccswitch-core = { path = "../ccswitch-core" }
ratatui = "0.29"
crossterm = "0.28"
```

### 5.3 扩展命令

只需在 `commands/` 下添加新模块，并在 `cli.rs` 中注册即可。
