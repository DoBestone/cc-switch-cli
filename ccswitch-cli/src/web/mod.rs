//! Web 控制器模块
//!
//! 提供 Web UI 和 REST API 用于配置管理。

pub mod api;
pub mod frontend;

use axum::{
    routing::{get, post, put, delete},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

/// 创建 Web 服务器路由
pub fn create_router() -> Router {
    Router::new()
        // 静态页面
        .route("/", get(frontend::index))
        // 状态和供应商 API
        .route("/api/status", get(api::get_status))
        .route("/api/providers", get(api::list_providers))
        .route("/api/providers/:app", get(api::list_providers_by_app))
        .route("/api/providers/:app/:name", get(api::get_provider))
        .route("/api/providers/:app", post(api::add_provider))
        .route("/api/providers/:app/:name", put(api::update_provider))
        .route("/api/providers/:app/:name", delete(api::delete_provider))
        .route("/api/switch", post(api::switch_provider))
        .route("/api/test/:app/:name", post(api::test_provider))
        // 批量操作 API
        .route("/api/batch/switch", post(api::batch_switch))
        .route("/api/batch/test", post(api::batch_test))
        .route("/api/export", get(api::export_config))
        .route("/api/import", post(api::import_config))
        // MCP API
        .route("/api/mcp", get(api::list_mcp))
        .route("/api/mcp/:app", get(api::list_mcp_by_app))
        .route("/api/mcp/:app", post(api::add_mcp))
        .route("/api/mcp/:app/:id", delete(api::delete_mcp))
        .route("/api/mcp/:app/:id/toggle", post(api::toggle_mcp))
        // Prompt API
        .route("/api/prompts", get(api::list_prompts))
        .route("/api/prompts/:app", get(api::list_prompts_by_app))
        .route("/api/prompts/:app", post(api::add_prompt))
        .route("/api/prompts/:app/:id", delete(api::delete_prompt))
        // Skill API
        .route("/api/skills", get(api::list_skills))
        .route("/api/skills/:app", get(api::list_skills_by_app))
        .route("/api/skills/install", post(api::install_skill))
        .route("/api/skills/:id", delete(api::delete_skill))
        .route("/api/skills/:id/toggle", post(api::toggle_skill))
        // Proxy API
        .route("/api/proxy", get(api::get_proxy))
        .route("/api/proxy", post(api::set_proxy))
        .route("/api/proxy", delete(api::clear_proxy))
        .route("/api/proxy/test", post(api::test_proxy))
        // Failover API
        .route("/api/failover/:app", get(api::list_failover))
        .route("/api/failover/:app", post(api::add_failover))
        .route("/api/failover/:app/:id", delete(api::remove_failover))
        .route("/api/failover/:app/clear", post(api::clear_failover))
        // Usage API
        .route("/api/usage/summary", get(api::usage_summary))
        .route("/api/usage/trends", get(api::usage_trends))
        .route("/api/usage/providers", get(api::usage_providers))
        // WebDAV API
        .route("/api/webdav/config", get(api::webdav_config))
        .route("/api/webdav/setup", post(api::webdav_setup))
        .route("/api/webdav/test", post(api::webdav_test))
        .route("/api/webdav/upload", post(api::webdav_upload))
        .route("/api/webdav/download", get(api::webdav_download))
        // 环境检测 API
        .route("/api/env/check", get(api::env_check))
        .route("/api/env/list", get(api::env_list))
        // 配置路径 API
        .route("/api/config/paths", get(api::config_paths))
        // CORS 支持
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
}