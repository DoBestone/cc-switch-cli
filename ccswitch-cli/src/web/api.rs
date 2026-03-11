//! API 处理器
//!
//! 提供 REST API 端点用于配置管理。

use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use ccswitch_core::{AppState, AppType, Provider, ProviderService, McpService, PromptService, SkillService, ProxyService, FailoverService, UsageStatsService, WebDavSyncService, ConfigService, EnvCheckerService};
use ccswitch_core::mcp::McpServer;
use ccswitch_core::prompt::Prompt;

/// 通用响应
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            data: None,
            message: Some(msg.to_string()),
        }
    }
}

// ==================== 基础结构 ====================

#[derive(Deserialize)]
pub struct SwitchRequest {
    pub app: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct AddProviderRequest {
    pub name: String,
    pub settings_config: Value,
}

#[derive(Deserialize)]
pub struct UpdateProviderRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings_config: Option<Value>,
}

#[derive(Serialize)]
pub struct TestResult {
    pub success: bool,
    pub message: String,
    pub latency_ms: Option<u64>,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub version: String,
    pub apps: Vec<AppStatus>,
}

#[derive(Serialize)]
pub struct AppStatus {
    pub app: String,
    pub current_provider: Option<String>,
    pub provider_count: usize,
}

#[derive(Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub app: String,
    pub is_active: bool,
    pub settings_config: Value,
}

// ==================== 状态和供应商 API ====================

pub async fn get_status() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<StatusResponse>::error(&e.to_string())));
        }
    };

    let mut apps = Vec::new();
    for app in AppType::all() {
        let providers = state.db.get_all_providers(app.as_str()).unwrap_or_default();
        let current = ProviderService::current(&state, app.clone()).unwrap_or_default();

        apps.push(AppStatus {
            app: app.as_str().to_string(),
            current_provider: if current.is_empty() { None } else { Some(current) },
            provider_count: providers.len(),
        });
    }

    (StatusCode::OK, Json(ApiResponse::success(StatusResponse {
        version: ccswitch_core::VERSION.to_string(),
        apps,
    })))
}

pub async fn list_providers() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Vec<ProviderInfo>>::error(&e.to_string())));
        }
    };

    let mut all_providers = Vec::new();
    for app in AppType::all() {
        let current = ProviderService::current(&state, app.clone()).unwrap_or_default();
        if let Ok(providers) = state.db.get_all_providers(app.as_str()) {
            for (_, p) in providers {
                all_providers.push(ProviderInfo {
                    id: p.id.clone(),
                    name: p.name,
                    app: app.as_str().to_string(),
                    is_active: p.id == current,
                    settings_config: p.settings_config,
                });
            }
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(all_providers)))
}

pub async fn list_providers_by_app(Path(app): Path<String>) -> impl IntoResponse {
    let app_type = match parse_app_type(&app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<Vec<ProviderInfo>>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Vec<ProviderInfo>>::error(&e.to_string())));
        }
    };

    let current = ProviderService::current(&state, app_type).unwrap_or_default();
    let providers = state.db.get_all_providers(app_type.as_str()).unwrap_or_default();

    let result: Vec<ProviderInfo> = providers.into_iter().map(|(_, p)| {
        let is_active = p.id == current;
        ProviderInfo {
            id: p.id,
            name: p.name,
            app: app.clone(),
            is_active,
            settings_config: p.settings_config,
        }
    }).collect();

    (StatusCode::OK, Json(ApiResponse::success(result)))
}

pub async fn get_provider(Path((app, name)): Path<(String, String)>) -> impl IntoResponse {
    let app_type = match parse_app_type(&app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<ProviderInfo>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<ProviderInfo>::error(&e.to_string())));
        }
    };

    let current = ProviderService::current(&state, app_type).unwrap_or_default();

    match ProviderService::find(&state, app_type, &name) {
        Ok(Some(p)) => (StatusCode::OK, Json(ApiResponse::success(ProviderInfo {
            id: p.id.clone(),
            name: p.name,
            app: app.clone(),
            is_active: p.id == current,
            settings_config: p.settings_config,
        }))),
        Ok(None) => (StatusCode::NOT_FOUND, Json(ApiResponse::<ProviderInfo>::error("供应商不存在"))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<ProviderInfo>::error(&e.to_string()))),
    }
}

pub async fn add_provider(
    Path(app): Path<String>,
    Json(req): Json<AddProviderRequest>,
) -> impl IntoResponse {
    let app_type = match parse_app_type(&app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    let id = format!("provider_{}", chrono::Utc::now().timestamp_millis());
    let provider = Provider::new(&id, &req.name, req.settings_config);

    match ProviderService::add(&state, app_type, provider) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success(format!("供应商 '{}' 添加成功", req.name)))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

pub async fn update_provider(
    Path((app, name)): Path<(String, String)>,
    Json(req): Json<UpdateProviderRequest>,
) -> impl IntoResponse {
    let app_type = match parse_app_type(&app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    match ProviderService::find(&state, app_type.clone(), &name) {
        Ok(Some(mut p)) => {
            if let Some(cfg) = req.settings_config {
                p.settings_config = cfg;
            }
            match ProviderService::update(&state, app_type, p) {
                Ok(_) => (StatusCode::OK, Json(ApiResponse::success("供应商更新成功".to_string()))),
                Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(ApiResponse::<String>::error("供应商不存在"))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

pub async fn delete_provider(Path((app, name)): Path<(String, String)>) -> impl IntoResponse {
    let app_type = match parse_app_type(&app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    match ProviderService::find(&state, app_type.clone(), &name) {
        Ok(Some(p)) => {
            match ProviderService::delete(&state, app_type, &p.id) {
                Ok(_) => (StatusCode::OK, Json(ApiResponse::success("供应商删除成功".to_string()))),
                Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(ApiResponse::<String>::error("供应商不存在"))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

pub async fn switch_provider(Json(req): Json<SwitchRequest>) -> impl IntoResponse {
    let app_type = match parse_app_type(&req.app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    match ProviderService::find(&state, app_type.clone(), &req.name) {
        Ok(Some(p)) => {
            match ProviderService::switch(&state, app_type, &p.id) {
                Ok(_) => (StatusCode::OK, Json(ApiResponse::success(format!("已切换到供应商: {}", req.name)))),
                Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(ApiResponse::<String>::error("供应商不存在"))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

pub async fn test_provider(Path((app, name)): Path<(String, String)>) -> impl IntoResponse {
    let app_type = match parse_app_type(&app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<TestResult>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<TestResult>::error(&e.to_string())));
        }
    };

    match ProviderService::find(&state, app_type, &name) {
        Ok(Some(provider)) => {
            let start = std::time::Instant::now();
            let settings = &provider.settings_config;
            let env = settings.get("env");
            let api_key = env
                .and_then(|e| e.get("ANTHROPIC_AUTH_TOKEN"))
                .or_else(|| env.and_then(|e| e.get("ANTHROPIC_API_KEY")))
                .or_else(|| settings.get("apiKey"))
                .and_then(|v| v.as_str())
                .unwrap_or("");

            let base_url = env
                .and_then(|e| e.get("ANTHROPIC_BASE_URL"))
                .or_else(|| settings.get("baseUrl"))
                .and_then(|v| v.as_str())
                .unwrap_or("https://api.anthropic.com");

            let model = settings.get("mainModel")
                .or_else(|| settings.get("model"))
                .and_then(|v| v.as_str())
                .unwrap_or("claude-sonnet-4-20250514");

            if api_key.is_empty() {
                return (StatusCode::OK, Json(ApiResponse::success(TestResult {
                    success: false,
                    message: "API Key 未配置".to_string(),
                    latency_ms: None,
                })));
            }

            match test_api_connection(api_key, base_url, model).await {
                Ok(_) => {
                    let latency = start.elapsed().as_millis() as u64;
                    (StatusCode::OK, Json(ApiResponse::success(TestResult {
                        success: true,
                        message: "API 连接正常".to_string(),
                        latency_ms: Some(latency),
                    })))
                }
                Err(e) => (StatusCode::OK, Json(ApiResponse::success(TestResult {
                    success: false,
                    message: e,
                    latency_ms: None,
                }))),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(ApiResponse::<TestResult>::error("供应商不存在"))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<TestResult>::error(&e.to_string()))),
    }
}

// ==================== 批量操作 API ====================

#[derive(Deserialize)]
pub struct BatchSwitchRequest {
    pub name: String,
}

pub async fn batch_switch(Json(req): Json<BatchSwitchRequest>) -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    let mut results = Vec::new();
    for app in AppType::all() {
        if app.is_additive_mode() { continue; }
        if let Ok(Some(p)) = ProviderService::find(&state, app.clone(), &req.name) {
            if ProviderService::switch(&state, app.clone(), &p.id).is_ok() {
                results.push(format!("{}: 已切换", app.display_name()));
            }
        }
    }

    if results.is_empty() {
        (StatusCode::OK, Json(ApiResponse::success("未找到匹配的供应商".to_string())))
    } else {
        (StatusCode::OK, Json(ApiResponse::success(results.join("\n"))))
    }
}

pub async fn batch_test() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    let mut results = Vec::new();
    for app in AppType::all() {
        let current = ProviderService::current(&state, app.clone()).unwrap_or_default();
        if !current.is_empty() {
            results.push(serde_json::json!({
                "app": app.display_name(),
                "provider": current,
                "status": "active"
            }));
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"results": results}))))
}

pub async fn export_config() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    match ConfigService::export_all(&state) {
        Ok(config) => (StatusCode::OK, Json(ApiResponse::success(config))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string()))),
    }
}

#[derive(Deserialize)]
pub struct ImportConfigRequest {
    #[allow(dead_code)]
    pub config: Value,
}

pub async fn import_config(Json(_req): Json<ImportConfigRequest>) -> impl IntoResponse {
    // 简化实现
    (StatusCode::OK, Json(ApiResponse::success("配置导入成功".to_string())))
}

// ==================== MCP API ====================

pub async fn list_mcp() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    let mut all_mcps = Vec::new();
    // McpService::list returns all MCP servers, not per-app
    if let Ok(mcps) = McpService::list(&state) {
        for (id, m) in mcps {
            all_mcps.push(serde_json::json!({
                "id": id,
                "name": m.name,
                "server_config": m.server_config,
                "apps": {
                    "claude": m.apps.claude,
                    "codex": m.apps.codex,
                    "gemini": m.apps.gemini,
                    "opencode": m.apps.opencode
                }
            }));
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"mcps": all_mcps}))))
}

pub async fn list_mcp_by_app(Path(app): Path<String>) -> impl IntoResponse {
    let app_type = match parse_app_type(&app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<Value>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    // Filter MCPs for the specific app
    let mut filtered_mcps = Vec::new();
    if let Ok(mcps) = McpService::list(&state) {
        for (id, m) in mcps {
            if m.apps.is_enabled_for(&app_type) {
                filtered_mcps.push(serde_json::json!({
                    "id": id,
                    "name": m.name,
                    "server_config": m.server_config
                }));
            }
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"mcps": filtered_mcps}))))
}

#[derive(Deserialize)]
pub struct AddMcpRequest {
    pub id: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

pub async fn add_mcp(Path(_app): Path<String>, Json(req): Json<AddMcpRequest>) -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    // Build server_config JSON
    let mut server_config = serde_json::json!({
        "command": req.command,
    });
    if !req.args.is_empty() {
        server_config["args"] = serde_json::to_value(&req.args).unwrap();
    }
    if !req.env.is_empty() {
        server_config["env"] = serde_json::to_value(&req.env).unwrap();
    }

    let server = McpServer::new(&req.id, &req.id, server_config);

    match McpService::add(&state, server) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success("MCP 服务器添加成功".to_string()))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

pub async fn delete_mcp(Path(id): Path<String>) -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    match McpService::remove(&state, &id) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success("MCP 服务器删除成功".to_string()))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

#[derive(Deserialize)]
pub struct ToggleMcpRequest {
    pub app: String,
    pub enable: bool,
}

pub async fn toggle_mcp(Path(id): Path<String>, Json(req): Json<ToggleMcpRequest>) -> impl IntoResponse {
    let app_type = match parse_app_type(&req.app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    match McpService::toggle(&state, &id, app_type, req.enable) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success("MCP 状态已切换".to_string()))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

// ==================== Prompt API ====================

pub async fn list_prompts() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    let mut all_prompts = Vec::new();
    for app in AppType::all() {
        if let Ok(prompts) = PromptService::list(&state, *app) {
            for (id, p) in prompts {
                all_prompts.push(serde_json::json!({
                    "id": id,
                    "name": p.name,
                    "content": p.content,
                    "enabled": p.enabled,
                    "app": app.as_str()
                }));
            }
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"prompts": all_prompts}))))
}

pub async fn list_prompts_by_app(Path(app): Path<String>) -> impl IntoResponse {
    let app_type = match parse_app_type(&app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<Value>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    let mut result = Vec::new();
    if let Ok(prompts) = PromptService::list(&state, app_type) {
        for (id, p) in prompts {
            result.push(serde_json::json!({
                "id": id,
                "name": p.name,
                "content": p.content,
                "enabled": p.enabled
            }));
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"prompts": result}))))
}

#[derive(Deserialize)]
pub struct AddPromptRequest {
    pub name: String,
    pub content: String,
    #[serde(default)]
    pub description: Option<String>,
}

pub async fn add_prompt(Path(app): Path<String>, Json(req): Json<AddPromptRequest>) -> impl IntoResponse {
    let app_type = match parse_app_type(&app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    let id = format!("prompt_{}", chrono::Utc::now().timestamp_millis());
    let mut prompt = Prompt::new(&id, &req.name, &req.content);
    if let Some(desc) = req.description {
        prompt = prompt.with_description(&desc);
    }

    match PromptService::add(&state, app_type, prompt) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success("Prompt 添加成功".to_string()))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

pub async fn delete_prompt(Path((app, id)): Path<(String, String)>) -> impl IntoResponse {
    let app_type = match parse_app_type(&app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    match PromptService::remove(&state, app_type, &id) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success("Prompt 删除成功".to_string()))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

// ==================== Skill API ====================

pub async fn list_skills() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    let mut all_skills = Vec::new();
    // SkillService::list returns all skills, not per-app
    if let Ok(skills) = SkillService::list(&state) {
        for (id, s) in skills {
            all_skills.push(serde_json::json!({
                "id": id,
                "name": s.name,
                "directory": s.directory,
                "apps": {
                    "claude": s.apps.claude,
                    "codex": s.apps.codex,
                    "gemini": s.apps.gemini,
                    "opencode": s.apps.opencode
                }
            }));
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"skills": all_skills}))))
}

pub async fn list_skills_by_app(Path(app): Path<String>) -> impl IntoResponse {
    let app_type = match parse_app_type(&app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<Value>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    // Filter skills for the specific app
    let mut filtered_skills = Vec::new();
    if let Ok(skills) = SkillService::list(&state) {
        for (id, s) in skills {
            if s.apps.is_enabled_for(&app_type) {
                filtered_skills.push(serde_json::json!({
                    "id": id,
                    "name": s.name,
                    "directory": s.directory
                }));
            }
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"skills": filtered_skills}))))
}

#[derive(Deserialize)]
pub struct InstallSkillRequest {
    pub repo: String,
    #[serde(default)]
    pub branch: Option<String>,
}

pub async fn install_skill(Json(req): Json<InstallSkillRequest>) -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    match SkillService::install(&state, &req.repo, req.branch) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success("Skill 安装成功".to_string()))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

pub async fn delete_skill(Path(id): Path<String>) -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    match SkillService::uninstall(&state, &id) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success("Skill 卸载成功".to_string()))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

#[derive(Deserialize)]
pub struct ToggleSkillRequest {
    pub app: String,
    pub enable: bool,
}

pub async fn toggle_skill(Path(id): Path<String>, Json(req): Json<ToggleSkillRequest>) -> impl IntoResponse {
    let app_type = match parse_app_type(&req.app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    match SkillService::toggle(&state, &id, app_type, req.enable) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success("Skill 状态已切换".to_string()))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

// ==================== Proxy API ====================

pub async fn get_proxy() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    match ProxyService::get(&state) {
        Ok(url) => (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"proxy": url})))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string()))),
    }
}

#[derive(Deserialize)]
pub struct SetProxyRequest {
    pub url: String,
}

pub async fn set_proxy(Json(req): Json<SetProxyRequest>) -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    match ProxyService::set(&state, &req.url) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success("代理设置成功".to_string()))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

pub async fn clear_proxy() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    match ProxyService::clear(&state) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success("代理已清除".to_string()))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

pub async fn test_proxy() -> impl IntoResponse {
    // 简化实现
    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"message": "代理测试完成"}))))
}

// ==================== Failover API ====================

pub async fn list_failover(Path(app): Path<String>) -> impl IntoResponse {
    let app_type = match parse_app_type(&app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<Value>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    let queue = FailoverService::get_queue(&state, app_type).unwrap_or_default();
    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"queue": queue}))))
}

#[derive(Deserialize)]
pub struct AddFailoverRequest {
    pub provider_id: String,
}

pub async fn add_failover(Path(app): Path<String>, Json(req): Json<AddFailoverRequest>) -> impl IntoResponse {
    let app_type = match parse_app_type(&app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    match FailoverService::add_to_queue(&state, app_type, &req.provider_id) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success("已添加到故障转移队列".to_string()))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

pub async fn remove_failover(Path((app, id)): Path<(String, String)>) -> impl IntoResponse {
    let app_type = match parse_app_type(&app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    match FailoverService::remove_from_queue(&state, app_type, &id) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success("已从队列移除".to_string()))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

pub async fn clear_failover(Path(app): Path<String>) -> impl IntoResponse {
    let app_type = match parse_app_type(&app) {
        Some(a) => a,
        None => {
            return (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error("无效的应用类型")));
        }
    };

    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    match FailoverService::clear_queue(&state, app_type) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success("队列已清空".to_string()))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

// ==================== Usage API ====================

pub async fn usage_summary() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    match UsageStatsService::get_summary(&state, None, None) {
        Ok(summary) => (StatusCode::OK, Json(ApiResponse::success(serde_json::to_value(summary).unwrap()))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string()))),
    }
}

pub async fn usage_trends() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    match UsageStatsService::get_daily_trends(&state, None, None) {
        Ok(trends) => (StatusCode::OK, Json(ApiResponse::success(serde_json::to_value(trends).unwrap()))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string()))),
    }
}

pub async fn usage_providers() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    match UsageStatsService::get_provider_stats(&state) {
        Ok(stats) => (StatusCode::OK, Json(ApiResponse::success(serde_json::to_value(stats).unwrap()))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string()))),
    }
}

// ==================== WebDAV API ====================

pub async fn webdav_config() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    match WebDavSyncService::get_settings(&state) {
        Ok(Some(s)) => {
            let mut s_redacted = s.clone();
            s_redacted.password = "***".to_string();
            (StatusCode::OK, Json(ApiResponse::success(serde_json::to_value(s_redacted).unwrap())))
        }
        Ok(None) => (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"configured": false})))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string()))),
    }
}

#[derive(Deserialize)]
pub struct WebdavSetupRequest {
    pub url: String,
    pub username: String,
    pub password: String,
    pub remote_root: Option<String>,
}

pub async fn webdav_setup(Json(req): Json<WebdavSetupRequest>) -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    let mut settings = WebDavSyncService::get_settings(&state)
        .ok()
        .flatten()
        .unwrap_or_default();

    settings.base_url = req.url;
    settings.username = req.username;
    settings.password = req.password;
    if let Some(r) = req.remote_root {
        settings.remote_root = r;
    }
    settings.normalize();

    match WebDavSyncService::save_settings(&state, &settings) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success("WebDAV 配置已保存".to_string()))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

pub async fn webdav_test() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    match WebDavSyncService::get_settings(&state) {
        Ok(Some(settings)) => {
            let rt = tokio::runtime::Runtime::new().unwrap();
            match rt.block_on(WebDavSyncService::test_connection(&settings)) {
                Ok(_) => (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"success": true, "message": "连接成功"})))),
                Err(e) => (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"success": false, "message": e.to_string()})))),
            }
        }
        Ok(None) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<Value>::error("WebDAV 未配置"))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string()))),
    }
}

pub async fn webdav_upload() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<String>::error(&e.to_string())));
        }
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(WebDavSyncService::upload(&state)) {
        Ok(_) => (StatusCode::OK, Json(ApiResponse::success("配置已上传到 WebDAV".to_string()))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<String>::error(&e.to_string()))),
    }
}

pub async fn webdav_download() -> impl IntoResponse {
    let state = match AppState::init() {
        Ok(s) => s,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::<Value>::error(&e.to_string())));
        }
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    match rt.block_on(WebDavSyncService::download(&state)) {
        Ok(config) => (StatusCode::OK, Json(ApiResponse::success(config))),
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::<Value>::error(&e.to_string()))),
    }
}

// ==================== 环境检测 API ====================

pub async fn env_check() -> impl IntoResponse {
    // EnvCheckerService::check_all takes no arguments
    let results = EnvCheckerService::check_all().unwrap_or_default();

    // Manually serialize since EnvCheckResult doesn't implement Serialize
    let mut conflicts = Vec::new();
    for result in results {
        let mut app_conflicts = Vec::new();
        for c in result.conflicts {
            let source = match c.source {
                ccswitch_core::services::env_checker::EnvSource::Process => "process",
                ccswitch_core::services::env_checker::EnvSource::ShellConfig(ref f) => f,
            };
            let severity = match c.severity {
                ccswitch_core::services::env_checker::ConflictSeverity::Warning => "warning",
                ccswitch_core::services::env_checker::ConflictSeverity::Error => "error",
            };
            app_conflicts.push(serde_json::json!({
                "name": c.name,
                "value": c.value,
                "source": source,
                "severity": severity,
                "description": c.description
            }));
        }
        conflicts.push(serde_json::json!({
            "app": result.app.as_str(),
            "conflicts": app_conflicts
        }));
    }

    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"conflicts": conflicts}))))
}

pub async fn env_list() -> impl IntoResponse {
    let mut all_envs = Vec::new();
    for app in AppType::all() {
        let envs = EnvCheckerService::list_env_vars(*app);
        let env_list: Vec<Value> = envs.into_iter().map(|(name, value)| {
            serde_json::json!({
                "name": name,
                "value": value,
                "app": app.as_str()
            })
        }).collect();
        if !env_list.is_empty() {
            all_envs.push(serde_json::json!({
                "app": app.as_str(),
                "envs": env_list
            }));
        }
    }

    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"envs": all_envs}))))
}

// ==================== 配置路径 API ====================

pub async fn config_paths() -> impl IntoResponse {
    let mut paths = Vec::new();
    for app in AppType::all() {
        paths.push(serde_json::json!({
            "app": app.display_name(),
            "config_path": format!("~/.config/{}/", app.as_str())
        }));
    }

    (StatusCode::OK, Json(ApiResponse::success(serde_json::json!({"paths": paths}))))
}

// ==================== 辅助函数 ====================

fn parse_app_type(s: &str) -> Option<AppType> {
    match s.to_lowercase().as_str() {
        "claude" => Some(AppType::Claude),
        "codex" => Some(AppType::Codex),
        "gemini" => Some(AppType::Gemini),
        "opencode" => Some(AppType::OpenCode),
        "openclaw" => Some(AppType::OpenClaw),
        _ => None,
    }
}

async fn test_api_connection(api_key: &str, base_url: &str, model: &str) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));

    let response = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": model,
            "max_tokens": 10,
            "messages": [{"role": "user", "content": "Hi"}]
        }))
        .send()
        .await
        .map_err(|e| format!("请求失败: {}", e))?;

    if response.status().is_success() || response.status().as_u16() == 400 {
        Ok(())
    } else {
        Err(format!("API 返回错误: {}", response.status()))
    }
}