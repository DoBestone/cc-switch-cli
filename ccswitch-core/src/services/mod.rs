//! 服务层模块
//!
//! 提供业务逻辑服务，包括供应商管理、配置同步、MCP 服务器管理、Prompt 管理等。

pub mod config;
pub mod env_checker;
pub mod failover;
pub mod global_proxy;
pub mod mcp;
pub mod prompt;
pub mod provider;
pub mod skill;
pub mod speedtest;
pub mod stream_check;
pub mod usage_stats;
pub mod webdav_sync;

pub use config::ConfigService;
pub use env_checker::EnvCheckerService;
pub use failover::FailoverService;
pub use global_proxy::ProxyService;
pub use mcp::McpService;
pub use prompt::PromptService;
pub use provider::ProviderService;
pub use skill::SkillService;
pub use speedtest::SpeedtestService;
pub use stream_check::StreamCheckService;
pub use usage_stats::UsageStatsService;
pub use webdav_sync::WebDavSyncService;
