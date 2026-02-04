//! CLI 参数定义模块
//!
//! 使用 clap 定义命令行接口结构。

use clap::{Parser, Subcommand, ValueEnum};

/// CC-Switch - CLI 配置管理工具
///
/// 用于管理 Claude Code、Codex、Gemini CLI 等 AI 编程工具的供应商配置。
#[derive(Parser, Debug)]
#[command(
    name = "cc-switch",
    version,
    author,
    about = "🔄 AI CLI 配置管理工具 - 轻松切换 Claude/Codex/Gemini 供应商",
    long_about = r#"
╔══════════════════════════════════════════════════════════════════════╗
║              CC-Switch - AI CLI 配置管理工具                         ║
╚══════════════════════════════════════════════════════════════════════╝

管理 Claude Code、Codex、Gemini CLI 的供应商配置。
支持在 Linux 服务器上通过 SSH 直接操作，无需图形界面。

🚀 快速开始:
   cc-switch              进入交互式菜单（推荐新手使用）
   cc-switch list         查看所有供应商
   cc-switch status       查看当前状态
   cc-switch use <名称>   切换供应商

📖 详细帮助:
   cc-switch <命令> --help   查看命令详情
"#,
    after_help = r#"💡 提示: 直接运行 cc-switch 不带参数可进入交互式菜单"#
)]
pub struct Cli {
    /// 输出格式
    #[arg(
        short = 'o',
        long,
        value_enum,
        default_value = "table",
        global = true,
        help = "输出格式 (table, json, yaml)"
    )]
    pub format: OutputFormat,

    /// 禁用彩色输出
    #[arg(long, global = true, help = "禁用彩色输出")]
    pub no_color: bool,

    /// 显示详细信息
    #[arg(short, long, global = true, help = "显示详细信息")]
    pub verbose: bool,

    /// 使用高级 TUI 界面
    #[arg(long, global = true, help = "使用高级 TUI 界面（实验性功能）")]
    pub tui: bool,

    /// 子命令
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// 输出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    /// 表格格式（默认）
    Table,
    /// JSON 格式
    Json,
    /// YAML 格式
    Yaml,
}

/// 应用类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum AppTypeArg {
    /// Claude Code CLI
    Claude,
    /// Codex CLI
    Codex,
    /// Gemini CLI
    Gemini,
    /// OpenCode CLI
    Opencode,
    /// 所有应用
    All,
}

impl AppTypeArg {
    /// 转换为 core 库的 AppType
    pub fn to_app_types(&self) -> Vec<ccswitch_core::AppType> {
        match self {
            Self::Claude => vec![ccswitch_core::AppType::Claude],
            Self::Codex => vec![ccswitch_core::AppType::Codex],
            Self::Gemini => vec![ccswitch_core::AppType::Gemini],
            Self::Opencode => vec![ccswitch_core::AppType::OpenCode],
            Self::All => ccswitch_core::AppType::all().to_vec(),
        }
    }
}

impl Default for AppTypeArg {
    fn default() -> Self {
        Self::All
    }
}

/// 子命令定义
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 📋 列出所有供应商配置
    #[command(
        visible_alias = "ls",
        long_about = "列出所有已配置的供应商。\n\n示例:\n  cc-switch list              列出所有供应商\n  cc-switch list --app claude 只列出 Claude 供应商\n  cc-switch list --detail     显示详细配置信息\n  cc-switch list --show-key   显示 API Key（脱敏）"
    )]
    List {
        /// 筛选应用类型 (claude/codex/gemini/opencode/all)
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,

        /// 显示详细配置信息
        #[arg(short, long)]
        detail: bool,

        /// 显示 API Key（脱敏显示）
        #[arg(long, help = "显示 API Key（部分隐藏）")]
        show_key: bool,
    },

    /// 📊 显示当前使用的供应商状态
    #[command(
        long_about = "显示各应用当前正在使用的供应商。\n\n示例:\n  cc-switch status              查看所有应用状态\n  cc-switch status --app claude 只看 Claude 状态"
    )]
    Status {
        /// 筛选应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,
    },

    /// 🔄 切换到指定供应商
    #[command(
        visible_alias = "switch",
        long_about = "切换到指定的供应商配置。\n\n示例:\n  cc-switch use 云雾API              切换 Claude 到 '云雾API'\n  cc-switch use OpenAI --app codex   切换 Codex 到 'OpenAI'"
    )]
    Use {
        /// 供应商名称 (可通过 cc-switch list 查看)
        name: String,

        /// 指定应用类型
        #[arg(short, long, value_enum, default_value = "claude")]
        app: AppTypeArg,
    },

    /// ➕ 添加新供应商 (交互式: cc-switch 然后选 4)
    #[command(
        long_about = r#"添加新的供应商配置。

💡 推荐使用交互式模式:
   运行 cc-switch 然后选择 "4. 添加供应商"

命令行示例:
  # Claude 供应商
  cc-switch add "我的API" --api-key "sk-xxx" --model "claude-sonnet-4-20250514"
  
  # 自定义 Base URL
  cc-switch add "代理API" --api-key "sk-xxx" --base-url "https://api.example.com"
  
  # Codex 供应商  
  cc-switch add "OpenAI" --app codex --api-key "sk-xxx" --model "gpt-4o"
  
注意：添加时会自动测试 API Key 有效性，使用 --skip-test 跳过测试"#
    )]
    Add {
        /// 供应商名称 (方便记忆的名字)
        name: String,

        /// 应用类型
        #[arg(short, long, value_enum, default_value = "claude")]
        app: AppTypeArg,

        /// API Key (必填)
        #[arg(long, help = "API Key (如 sk-ant-xxx)")]
        api_key: Option<String>,

        /// API Base URL (可选，用于代理)
        #[arg(long, help = "Base URL (如 https://api.example.com)")]
        base_url: Option<String>,

        /// 主模型名称
        #[arg(long, short = 'm', help = "主模型 (如 claude-sonnet-4-20250514)")]
        model: Option<String>,

        /// 小模型/快速模型名称
        #[arg(long, help = "小模型 (如 claude-haiku-4-20250514)")]
        small_model: Option<String>,

        /// 从文件导入完整配置
        #[arg(long, value_name = "FILE", help = "从 JSON 文件导入")]
        from_file: Option<String>,

        /// 跳过 API 测试
        #[arg(long, help = "跳过添加前的 API 测试")]
        skip_test: bool,
    },

    /// ✏️ 编辑供应商
    #[command(
        long_about = r#"编辑已有的供应商配置。

示例:
  # 修改 API Key
  cc-switch edit "云雾API" --api-key "sk-new-xxx"
  
  # 修改 Base URL
  cc-switch edit "云雾API" --base-url "https://new-api.example.com"
  
  # 修改多个字段
  cc-switch edit "云雾API" --api-key "sk-xxx" --model "claude-sonnet-4-20250514""#
    )]
    Edit {
        /// 供应商名称
        name: String,

        /// 应用类型
        #[arg(short, long, value_enum, default_value = "claude")]
        app: AppTypeArg,

        /// 新 API Key
        #[arg(long, help = "新的 API Key")]
        api_key: Option<String>,

        /// 新 Base URL
        #[arg(long, help = "新的 Base URL")]
        base_url: Option<String>,

        /// 新主模型
        #[arg(long, short = 'm', help = "新的主模型")]
        model: Option<String>,

        /// 新小模型
        #[arg(long, help = "新的小模型")]
        small_model: Option<String>,

        /// 新名称
        #[arg(long, help = "新的供应商名称")]
        new_name: Option<String>,
    },

    /// 🧪 测试供应商 API Key
    #[command(
        long_about = r#"测试供应商的 API Key 是否有效。

示例:
  cc-switch test "云雾API"                   测试指定供应商
  cc-switch test "云雾API" --app claude      测试 Claude 供应商
  cc-switch test --api-key "sk-xxx"          直接测试 API Key
  cc-switch test --api-key "sk-xxx" --base-url "https://api.example.com""#
    )]
    Test {
        /// 供应商名称（可选，与 --api-key 二选一）
        name: Option<String>,

        /// 应用类型
        #[arg(short, long, value_enum, default_value = "claude")]
        app: AppTypeArg,

        /// 直接测试 API Key
        #[arg(long, help = "要测试的 API Key")]
        api_key: Option<String>,

        /// Base URL（配合 --api-key 使用）
        #[arg(long, help = "Base URL")]
        base_url: Option<String>,

        /// 测试模型
        #[arg(long, help = "测试使用的模型")]
        model: Option<String>,

        /// 超时时间（秒）
        #[arg(long, default_value = "30")]
        timeout: u64,
    },

    /// ❌ 删除供应商
    #[command(
        visible_alias = "rm",
        long_about = "删除指定的供应商配置。\n\n示例:\n  cc-switch remove 云雾API    删除名为 '云雾API' 的供应商\n  cc-switch rm 云雾API -y     跳过确认直接删除"
    )]
    Remove {
        /// 要删除的供应商名称
        name: String,

        /// 指定应用类型
        #[arg(short, long, value_enum, default_value = "claude")]
        app: AppTypeArg,

        /// 跳过确认直接删除
        #[arg(short = 'y', long, help = "跳过确认")]
        yes: bool,
    },

    /// 🔄 更新订阅/刷新配置
    #[command(
        long_about = "更新订阅或刷新配置。\n\n示例:\n  cc-switch update              更新所有订阅\n  cc-switch update --app claude 只更新 Claude"
    )]
    Update {
        /// 指定应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,
    },

    /// 📤 导出配置到文件
    #[command(
        long_about = "导出供应商配置到文件。\n\n示例:\n  cc-switch export                       导出到终端 (JSON)\n  cc-switch export -o backup.json        导出到文件\n  cc-switch export --format yaml -o cfg  导出为 YAML"
    )]
    Export {
        /// 导出格式
        #[arg(short, long, value_enum, default_value = "json", help = "格式: json/yaml/toml")]
        format: ExportFormatArg,

        /// 输出文件路径
        #[arg(short, long, value_name = "FILE")]
        out: Option<String>,

        /// 指定应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,
    },

    /// 📥 从文件导入配置
    #[command(
        long_about = "从配置文件导入供应商。\n\n示例:\n  cc-switch import backup.json           导入配置文件\n  cc-switch import cfg.yaml --app claude 只导入 Claude"
    )]
    Import {
        /// 配置文件路径
        file: String,

        /// 指定应用类型
        #[arg(short, long, value_enum)]
        app: Option<AppTypeArg>,
    },

    /// ⚙️ 配置管理
    #[command(
        long_about = "管理 cc-switch 和各应用的配置。\n\n示例:\n  cc-switch config path    显示配置文件路径\n  cc-switch config check   检查配置状态"
    )]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// 📦 MCP 服务器管理
    #[command(
        long_about = "管理 MCP (Model Context Protocol) 服务器配置。\n\n示例:\n  cc-switch mcp list                列出所有 MCP 服务器\n  cc-switch mcp add my-server --command npx --args \"-y\" \"@test/server\"\n  cc-switch mcp toggle my-server --app claude --enable"
    )]
    Mcp {
        #[command(subcommand)]
        action: McpAction,
    },

    /// 📝 Prompt 管理
    #[command(
        long_about = "管理各应用的系统提示词 (CLAUDE.md, AGENTS.md 等)。\n\n示例:\n  cc-switch prompt list --app claude    列出 Claude 的 Prompts\n  cc-switch prompt add \"My Prompt\" --app claude --content \"# My Prompt\"\n  cc-switch prompt enable my-prompt --app claude"
    )]
    Prompt {
        #[command(subcommand)]
        action: PromptAction,
    },

    /// 🌐 代理设置
    #[command(
        long_about = "管理全局代理设置。\n\n示例:\n  cc-switch proxy get              查看当前代理\n  cc-switch proxy set http://127.0.0.1:7890\n  cc-switch proxy test             测试代理连接\n  cc-switch proxy scan             扫描本地代理"
    )]
    Proxy {
        #[command(subcommand)]
        action: ProxyAction,
    },

    /// ⚡ 端点测速
    #[command(
        visible_alias = "speed",
        long_about = "测试 API 端点的延迟。\n\n示例:\n  cc-switch speedtest                    测试默认端点\n  cc-switch speedtest https://api.example.com\n  cc-switch speedtest --timeout 5"
    )]
    Speedtest {
        /// 要测试的 URL 列表
        #[arg(num_args = 0..)]
        urls: Vec<String>,

        /// 超时时间（秒）
        #[arg(long, default_value = "10")]
        timeout: u64,

        /// 使用全局代理
        #[arg(long)]
        proxy: bool,
    },

    /// 🔍 环境变量检测
    #[command(
        long_about = "检测可能与 AI CLI 工具冲突的环境变量。\n\n示例:\n  cc-switch env check              检查所有应用\n  cc-switch env check --app claude 只检查 Claude\n  cc-switch env list               列出相关环境变量"
    )]
    Env {
        #[command(subcommand)]
        action: EnvAction,
    },

    /// 🧩 Skills 管理
    #[command(
        long_about = "管理各应用的 Skills 扩展。\n\n示例:\n  cc-switch skill list                列出所有 Skills\n  cc-switch skill install owner/repo  从 GitHub 安装 Skill\n  cc-switch skill toggle my-skill --app claude --enable"
    )]
    Skill {
        #[command(subcommand)]
        action: SkillAction,
    },

    /// 🔄 检测更新/自动更新
    #[command(
        name = "self-update",
        visible_alias = "upgrade",
        long_about = r#"检测新版本并自动更新 cc-switch CLI 工具。

示例:
  cc-switch self-update           检测并执行更新
  cc-switch self-update --check   仅检测，不更新
  cc-switch self-update --force   强制重新安装最新版

更新方式:
  1. 优先下载 GitHub Release 预编译二进制
  2. 回退使用 cargo install --git 从源码编译"#
    )]
    SelfUpdate {
        #[command(subcommand)]
        action: Option<SelfUpdateAction>,

        /// 仅检测是否有更新
        #[arg(long, short = 'c', help = "仅检测，不执行更新")]
        check: bool,

        /// 强制更新（即使已是最新版）
        #[arg(long, short = 'f', help = "强制重新安装")]
        force: bool,
    },

    /// 🔄 批量操作命令
    #[command(
        long_about = "批量操作多个供应商或应用。\n\n支持批量切换、测试、导出、导入、同步等操作。"
    )]
    Batch {
        #[command(subcommand)]
        action: BatchAction
    },

    /// ℹ️ 显示版本信息
    Version,
}

/// 批量操作子命令
#[derive(Subcommand, Debug)]
pub enum BatchAction {
    /// 🔄 批量切换所有应用到指定供应商
    #[command(
        visible_alias = "use",
        long_about = "将所有应用（Claude, Codex, Gemini）切换到同一个供应商。\n\n示例:\n  cc-switch batch switch 云雾API"
    )]
    Switch {
        /// 供应商名称
        name: String,
    },

    /// 🧪 批量测试所有供应商 API
    #[command(
        long_about = "并发测试所有或指定应用的供应商 API。\n\n示例:\n  cc-switch batch test              测试所有供应商\n  cc-switch batch test --app claude 只测试 Claude 供应商\n  cc-switch batch test --verbose    显示详细错误信息"
    )]
    Test {
        /// 筛选应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,

        /// 超时时间（秒）
        #[arg(short, long, default_value = "30")]
        timeout: u64,

        /// 显示详细错误信息
        #[arg(short, long)]
        verbose: bool,
    },

    /// 📤 批量导出配置到文件
    #[command(
        long_about = "导出所有供应商配置到 YAML 文件。\n\n示例:\n  cc-switch batch export backup.yaml              导出所有应用\n  cc-switch batch export claude.yaml --app claude 只导出 Claude"
    )]
    Export {
        /// 输出文件路径
        output: String,

        /// 筛选应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,
    },

    /// 📥 批量导入配置从文件
    #[command(
        long_about = "从 YAML 文件导入供应商配置。\n\n示例:\n  cc-switch batch import backup.yaml            导入配置\n  cc-switch batch import backup.yaml --overwrite 覆盖已存在的配置"
    )]
    Import {
        /// 输入文件路径
        input: String,

        /// 覆盖已存在的配置
        #[arg(long)]
        overwrite: bool,
    },

    /// ❌ 批量删除供应商
    #[command(
        visible_alias = "rm",
        long_about = "批量删除多个供应商。\n\n示例:\n  cc-switch batch remove 供应商1 供应商2 供应商3\n  cc-switch batch remove -y 供应商1         跳过确认"
    )]
    Remove {
        /// 要删除的供应商名称列表
        #[arg(required = true, num_args = 1..)]
        names: Vec<String>,

        /// 筛选应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,

        /// 跳过确认直接删除
        #[arg(short = 'y', long)]
        force: bool,
    },

    /// 🔄 批量同步配置（从一个应用复制到其他应用）
    #[command(
        long_about = "将一个应用的所有供应商配置同步到其他应用。\n\n示例:\n  cc-switch batch sync --from claude --to codex,gemini\n  cc-switch batch sync --from claude --to all --overwrite"
    )]
    Sync {
        /// 源应用类型
        #[arg(long, value_enum, required = true)]
        from: AppTypeArg,

        /// 目标应用类型（逗号分隔或 'all'）
        #[arg(long, value_delimiter = ',', num_args = 1.., required = true)]
        to: Vec<AppTypeArg>,

        /// 覆盖已存在的配置
        #[arg(long)]
        overwrite: bool,
    },

    /// ✏️ 批量编辑配置字段
    #[command(
        long_about = "批量修改供应商的指定字段。\n\n示例:\n  cc-switch batch edit base-url https://api.example.com --app all\n  cc-switch batch edit model gpt-4 --pattern OpenAI"
    )]
    Edit {
        /// 要修改的字段名 (base-url, model, small-model)
        field: String,

        /// 新值
        value: String,

        /// 筛选应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,

        /// 只修改名称匹配此模式的供应商
        #[arg(long)]
        pattern: Option<String>,
    },
}

/// 配置操作子命令
#[derive(Subcommand, Debug)]
pub enum ConfigAction {
    /// 📁 显示配置文件路径
    Path {
        /// 指定应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,
    },

    /// 📂 打开配置目录
    Open {
        /// 指定应用类型
        #[arg(short, long, value_enum)]
        app: Option<AppTypeArg>,
    },

    /// ✅ 检查配置状态
    Check {
        /// 指定应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,
    },
}

/// 导出格式
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ExportFormatArg {
    Json,
    Yaml,
    Toml,
}

/// MCP 操作子命令
#[derive(Subcommand, Debug)]
pub enum McpAction {
    /// 📋 列出所有 MCP 服务器
    #[command(visible_alias = "ls")]
    List {
        /// 筛选应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,

        /// 显示详细配置
        #[arg(short, long)]
        detail: bool,
    },

    /// ➕ 添加 MCP 服务器
    Add {
        /// 服务器 ID
        id: String,

        /// 执行命令
        #[arg(long)]
        command: String,

        /// 命令参数
        #[arg(long, num_args = 1..)]
        args: Vec<String>,

        /// 环境变量 (格式: KEY=VALUE)
        #[arg(long, short, num_args = 1..)]
        env: Vec<String>,

        /// 显示名称
        #[arg(long)]
        name: Option<String>,

        /// 描述
        #[arg(long)]
        description: Option<String>,
    },

    /// ✏️ 更新 MCP 服务器
    Update {
        /// 服务器 ID
        id: String,

        /// 新名称
        #[arg(long)]
        name: Option<String>,

        /// 新命令
        #[arg(long)]
        command: Option<String>,

        /// 新参数
        #[arg(long, num_args = 1..)]
        args: Option<Vec<String>>,

        /// 新描述
        #[arg(long)]
        description: Option<String>,
    },

    /// ❌ 删除 MCP 服务器
    #[command(visible_alias = "rm")]
    Remove {
        /// 服务器 ID
        id: String,

        /// 跳过确认
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// 🔄 切换应用启用状态
    Toggle {
        /// 服务器 ID
        id: String,

        /// 应用类型
        #[arg(short, long, value_enum)]
        app: AppTypeArg,

        /// 启用
        #[arg(long, conflicts_with = "disable")]
        enable: bool,

        /// 禁用
        #[arg(long, conflicts_with = "enable")]
        disable: bool,
    },

    /// 📥 从应用导入 MCP 配置
    Import {
        /// 从指定应用导入
        #[arg(long, value_enum)]
        from: Option<AppTypeArg>,
    },

    /// 🔍 显示 MCP 服务器详情
    Show {
        /// 服务器 ID
        id: String,
    },
}

/// Prompt 操作子命令
#[derive(Subcommand, Debug)]
pub enum PromptAction {
    /// 📋 列出所有 Prompts
    #[command(visible_alias = "ls")]
    List {
        /// 筛选应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,
    },

    /// ➕ 添加 Prompt
    Add {
        /// Prompt 名称
        name: String,

        /// 应用类型
        #[arg(short, long, value_enum)]
        app: AppTypeArg,

        /// Prompt 内容
        #[arg(long)]
        content: Option<String>,

        /// 从文件读取内容
        #[arg(long, value_name = "FILE")]
        file: Option<String>,

        /// 描述
        #[arg(long)]
        description: Option<String>,
    },

    /// ✏️ 更新 Prompt
    Update {
        /// Prompt ID
        id: String,

        /// 应用类型
        #[arg(short, long, value_enum)]
        app: AppTypeArg,

        /// 新名称
        #[arg(long)]
        name: Option<String>,

        /// 新内容
        #[arg(long)]
        content: Option<String>,

        /// 新描述
        #[arg(long)]
        description: Option<String>,
    },

    /// ❌ 删除 Prompt
    #[command(visible_alias = "rm")]
    Remove {
        /// Prompt ID
        id: String,

        /// 应用类型
        #[arg(short, long, value_enum)]
        app: AppTypeArg,

        /// 跳过确认
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// ✅ 启用 Prompt
    Enable {
        /// Prompt ID
        id: String,

        /// 应用类型
        #[arg(short, long, value_enum)]
        app: AppTypeArg,
    },

    /// 🔍 显示 Prompt 详情
    Show {
        /// Prompt ID
        id: String,

        /// 应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,
    },

    /// 📥 从应用导入 Prompt
    Import {
        /// 应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,
    },
}

/// 代理操作子命令
#[derive(Subcommand, Debug)]
pub enum ProxyAction {
    /// 🔍 查看当前代理设置
    Get,

    /// ⚙️ 设置全局代理
    Set {
        /// 代理 URL (http://host:port 或 socks5://host:port)
        url: String,
    },

    /// ❌ 清除代理设置
    Clear,

    /// 🧪 测试代理连接
    Test {
        /// 指定代理 URL（不指定则使用当前设置）
        #[arg(long)]
        url: Option<String>,
    },

    /// 🔍 扫描本地代理
    Scan,
}

/// 环境变量操作子命令
#[derive(Subcommand, Debug)]
pub enum EnvAction {
    /// 🔍 检查环境变量冲突
    Check {
        /// 筛选应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,
    },

    /// 📋 列出相关环境变量
    #[command(visible_alias = "ls")]
    List {
        /// 筛选应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,
    },
}

/// Skill 操作子命令
#[derive(Subcommand, Debug)]
pub enum SkillAction {
    /// 📋 列出所有 Skills
    #[command(visible_alias = "ls")]
    List {
        /// 筛选应用类型
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,

        /// 显示详细信息
        #[arg(short, long)]
        detail: bool,
    },

    /// 📥 从 GitHub 安装 Skill
    Install {
        /// GitHub 仓库 (格式: owner/name)
        repo: String,

        /// 分支名称
        #[arg(long, default_value = "main")]
        branch: Option<String>,

        /// 安装后启用的应用
        #[arg(short, long, value_enum)]
        app: Option<AppTypeArg>,
    },

    /// ❌ 卸载 Skill
    #[command(visible_alias = "rm")]
    Uninstall {
        /// Skill ID
        id: String,

        /// 跳过确认
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// 🔄 切换应用启用状态
    Toggle {
        /// Skill ID
        id: String,

        /// 应用类型
        #[arg(short, long, value_enum)]
        app: AppTypeArg,

        /// 启用
        #[arg(long, conflicts_with = "disable")]
        enable: bool,

        /// 禁用
        #[arg(long, conflicts_with = "enable")]
        disable: bool,
    },

    /// 🔍 扫描本地 Skills 目录
    Scan,

    /// 🔄 同步 Skills 到所有应用
    Sync,

    /// 🔍 显示 Skill 详情
    Show {
        /// Skill ID
        id: String,
    },
}

/// 自动更新操作子命令
#[derive(Subcommand, Debug)]
pub enum SelfUpdateAction {
    /// 🔍 检测是否有新版本
    Check,
    
    /// ⬆️ 执行更新
    Run {
        /// 强制重新安装
        #[arg(long, short = 'f')]
        force: bool,
    },
}
