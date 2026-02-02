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
        long_about = "列出所有已配置的供应商。\n\n示例:\n  cc-switch list              列出所有供应商\n  cc-switch list --app claude 只列出 Claude 供应商\n  cc-switch list --detail     显示详细配置信息"
    )]
    List {
        /// 筛选应用类型 (claude/codex/gemini/opencode/all)
        #[arg(short, long, value_enum, default_value = "all")]
        app: AppTypeArg,

        /// 显示详细配置信息
        #[arg(short, long)]
        detail: bool,
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
  cc-switch add "OpenAI" --app codex --api-key "sk-xxx" --model "gpt-4o""#
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

    /// ℹ️ 显示版本信息
    Version,
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
