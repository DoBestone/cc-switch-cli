//! 服务层模块
//!
//! 提供业务逻辑服务，包括供应商管理、配置同步、MCP 服务器管理、Prompt 管理等。

pub mod config;
pub mod env_checker;
pub mod global_proxy;
pub mod mcp;
pub mod prompt;
pub mod provider;
pub mod skill;
pub mod speedtest;

pub use config::ConfigService;
pub use env_checker::EnvCheckerService;
pub use global_proxy::ProxyService;
pub use mcp::McpService;
pub use prompt::PromptService;
pub use provider::ProviderService;
pub use skill::SkillService;
pub use speedtest::SpeedtestService;
