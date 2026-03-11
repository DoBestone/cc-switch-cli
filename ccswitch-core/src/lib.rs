//! CC-Switch Core Library
//!
//! 核心业务逻辑库，提供 Claude Code、Codex、Gemini CLI 和 OpenCode 的配置管理功能。
//! 此库不依赖任何 GUI 框架（Tauri、WebView 等），可在 CLI、TUI 或服务端使用。
//!
//! # 架构设计
//!
//! ```text
//! ccswitch-core/
//! ├── lib.rs           - 公共 API 导出
//! ├── config.rs        - 配置文件路径和读写
//! ├── error.rs         - 统一错误类型
//! ├── provider.rs      - 供应商数据结构
//! ├── mcp.rs           - MCP 服务器数据结构
//! ├── app_config.rs    - 应用类型定义
//! ├── settings.rs      - 本地设置管理
//! ├── database/        - SQLite 数据持久化
//! │   ├── mod.rs
//! │   ├── schema.rs
//! │   ├── mcp.rs
//! │   └── dao/
//! └── services/        - 业务逻辑服务层
//!     ├── mod.rs
//!     ├── provider.rs
//!     ├── config.rs
//!     └── mcp.rs
//! ```
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use ccswitch_core::{AppState, AppType, ProviderService};
//!
//! fn main() -> anyhow::Result<()> {
//!     // 初始化应用状态
//!     let state = AppState::init()?;
//!
//!     // 列出所有 Claude 供应商
//!     let providers = ProviderService::list(&state, AppType::Claude)?;
//!     for (id, provider) in providers {
//!         println!("{}: {}", id, provider.name);
//!     }
//!
//!     // 获取当前供应商
//!     let current = ProviderService::current(&state, AppType::Claude)?;
//!     println!("当前供应商: {}", current);
//!
//!     // 切换供应商
//!     ProviderService::switch(&state, AppType::Claude, "provider-id")?;
//!
//!     Ok(())
//! }
//! ```

pub mod app_config;
pub mod config;
pub mod database;
pub mod error;
pub mod mcp;
pub mod openclaw_config;
pub mod prompt;
pub mod provider;
pub mod services;
pub mod settings;
pub mod skill;
pub mod store;

// 公共类型导出
pub use app_config::{AppType, McpApps, SkillApps};
pub use config::{
    get_app_config_dir, get_app_config_path, get_claude_config_dir, get_claude_mcp_path,
    get_claude_settings_path, get_codex_config_dir, get_codex_config_path, get_codex_auth_path,
    get_gemini_config_dir, get_gemini_settings_path, get_opencode_config_dir,
    get_openclaw_config_dir, get_openclaw_config_path, get_openclaw_providers_path,
    get_home_dir, get_database_path, read_json_file, write_json_file, write_text_file,
};
pub use database::Database;
pub use error::AppError;
pub use mcp::{McpServer, McpStdioConfig};
pub use openclaw_config::{
    OpenClawProviderConfig, OpenClawModelEntry, OpenClawDefaultModel,
    OpenClawAgentsDefaults, OpenClawEnvConfig, OpenClawToolsConfig,
    OpenClawHealthWarning, OpenClawWriteOutcome, OpenClawModelCatalogEntry,
    get_openclaw_config_path as get_openclaw_json_path,
    read_openclaw_config, write_openclaw_config,
    get_providers as get_openclaw_providers, set_provider as set_openclaw_provider,
    remove_provider as remove_openclaw_provider, get_typed_providers as get_openclaw_typed_providers,
    set_typed_provider as set_openclaw_typed_provider,
    get_default_model as get_openclaw_default_model, set_default_model as set_openclaw_default_model,
    get_agents_defaults, set_agents_defaults,
    get_env_config as get_openclaw_env_config, set_env_config as set_openclaw_env_config,
    get_tools_config as get_openclaw_tools_config, set_tools_config as set_openclaw_tools_config,
    get_model_catalog as get_openclaw_model_catalog, set_model_catalog as set_openclaw_model_catalog,
    add_model_to_catalog as add_openclaw_model_to_catalog, remove_model_from_catalog as remove_openclaw_model_from_catalog,
    scan_openclaw_config_health,
};
pub use prompt::Prompt;
pub use provider::{Provider, ProviderManager, ProviderMeta};
pub use skill::{Skill, SkillRepo};
pub use services::{
    ConfigService, EnvCheckerService, FailoverService, McpService, PromptService, ProxyService, ProviderService,
    SkillService, SpeedtestService, StreamCheckService, UsageStatsService, WebDavSyncService,
};
pub use services::failover::FailoverQueueItem;
pub use services::stream_check::{HealthStatus, HealthCheckResult, StreamCheckConfig};
pub use services::usage_stats::{
    UsageSummary, DailyStats, ProviderStats, ModelStats, UsageRecord, LimitStatus,
};
pub use services::webdav_sync::{WebDavSyncSettings, SyncStatus};
pub use settings::AppSettings;
pub use store::AppState;

/// 库版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 应用名称
pub const APP_NAME: &str = "cc-switch";
