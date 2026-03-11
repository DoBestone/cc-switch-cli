//! Web 控制器模块
//!
//! 提供 Web UI 和 REST API 用于配置管理。

pub mod api;
pub mod auth;
pub mod frontend;

use axum::{
    middleware,
    routing::{get, post, put, delete},
    Router,
};
use tower_http::cors::{Any, CorsLayer};

use crate::web::auth::AuthState;

/// 创建 Web 服务器路由
pub fn create_router(username: &str, password: &str) -> Router {
    let auth_state = AuthState {
        username: username.to_string(),
        password: password.to_string(),
    };

    // 公开路由（不需要认证）
    let public_routes = Router::new()
        .route("/", get(frontend::index))
        .route("/api/login", post(auth::login));

    // 受保护的路由（需要认证）
    let protected_routes = Router::new()
        // 状态和供应商 API
        .route("/api/status", get(api::get_status))
        .route("/api/providers", get(api::list_providers))
        .route("/api/providers/app/:app", get(api::list_providers_by_app))
        .route("/api/providers/app/:app/:name", get(api::get_provider))
        .route("/api/providers/app/:app", post(api::add_provider))
        .route("/api/providers/app/:app/:name", put(api::update_provider))
        .route("/api/providers/app/:app/:name", delete(api::delete_provider))
        .route("/api/switch", post(api::switch_provider))
        .route("/api/test/:app/:name", post(api::test_provider))
        // 批量操作 API
        .route("/api/batch/switch", post(api::batch_switch))
        .route("/api/batch/test", post(api::batch_test))
        .route("/api/export", get(api::export_config))
        .route("/api/import", post(api::import_config))
        // MCP API
        .route("/api/mcp", get(api::list_mcp))
        .route("/api/mcp/app/:app", get(api::list_mcp_by_app))
        .route("/api/mcp/app/:app", post(api::add_mcp))
        .route("/api/mcp/:id", delete(api::delete_mcp))
        .route("/api/mcp/:id/toggle", post(api::toggle_mcp))
        // Prompt API
        .route("/api/prompts", get(api::list_prompts))
        .route("/api/prompts/app/:app", get(api::list_prompts_by_app))
        .route("/api/prompts/app/:app", post(api::add_prompt))
        .route("/api/prompts/app/:app/:id", delete(api::delete_prompt))
        // Skill API
        .route("/api/skills", get(api::list_skills))
        .route("/api/skills/app/:app", get(api::list_skills_by_app))
        .route("/api/skills/install", post(api::install_skill))
        .route("/api/skills/:id", delete(api::delete_skill))
        .route("/api/skills/:id/toggle", post(api::toggle_skill))
        // Proxy API
        .route("/api/proxy", get(api::get_proxy))
        .route("/api/proxy", post(api::set_proxy))
        .route("/api/proxy", delete(api::clear_proxy))
        .route("/api/proxy/test", post(api::test_proxy))
        // Failover API
        .route("/api/failover/app/:app", get(api::list_failover))
        .route("/api/failover/app/:app", post(api::add_failover))
        .route("/api/failover/app/:app/:id", delete(api::remove_failover))
        .route("/api/failover/app/:app/clear", post(api::clear_failover))
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
        // 添加认证中间件
        .layer(middleware::from_fn_with_state(auth_state.clone(), auth::auth_middleware));

    // 合并路由
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(auth_state)
        // CORS 支持
        .layer(CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any))
}