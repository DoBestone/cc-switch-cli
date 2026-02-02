//! MCP 服务器服务模块
//!
//! 提供 MCP 服务器的业务逻辑，包括配置同步到各应用。

use indexmap::IndexMap;
use serde_json::{json, Value};
use std::fs;

use crate::app_config::AppType;
use crate::config::{
    get_claude_mcp_path, get_codex_config_dir, get_gemini_config_dir, get_opencode_config_dir,
    read_json_file, write_json_file, write_text_file,
};
use crate::error::AppError;
use crate::mcp::McpServer;
use crate::store::AppState;

/// MCP 服务器服务
pub struct McpService;

impl McpService {
    /// 列出所有 MCP 服务器
    pub fn list(state: &AppState) -> Result<IndexMap<String, McpServer>, AppError> {
        state.db.get_all_mcp_servers()
    }

    /// 获取单个 MCP 服务器
    pub fn get(state: &AppState, id: &str) -> Result<Option<McpServer>, AppError> {
        state.db.get_mcp_server(id)
    }

    /// 添加 MCP 服务器
    pub fn add(state: &AppState, server: McpServer) -> Result<(), AppError> {
        // 检查 ID 是否已存在
        if state.db.get_mcp_server(&server.id)?.is_some() {
            return Err(AppError::InvalidInput(format!(
                "MCP 服务器 '{}' 已存在",
                server.id
            )));
        }

        state.db.save_mcp_server(&server)?;

        // 同步到启用的应用
        Self::sync_to_apps(state, &server)?;

        Ok(())
    }

    /// 更新 MCP 服务器
    pub fn update(state: &AppState, server: McpServer) -> Result<(), AppError> {
        // 检查是否存在
        if state.db.get_mcp_server(&server.id)?.is_none() {
            return Err(AppError::InvalidInput(format!(
                "MCP 服务器 '{}' 不存在",
                server.id
            )));
        }

        state.db.save_mcp_server(&server)?;

        // 同步到所有应用
        Self::sync_all(state)?;

        Ok(())
    }

    /// 删除 MCP 服务器
    pub fn remove(state: &AppState, id: &str) -> Result<(), AppError> {
        // 检查是否存在
        let server = state.db.get_mcp_server(id)?;
        if server.is_none() {
            return Err(AppError::InvalidInput(format!(
                "MCP 服务器 '{}' 不存在",
                id
            )));
        }

        state.db.delete_mcp_server(id)?;

        // 同步到所有应用（移除该服务器）
        Self::sync_all(state)?;

        Ok(())
    }

    /// 切换 MCP 服务器的应用启用状态
    pub fn toggle(
        state: &AppState,
        id: &str,
        app: AppType,
        enable: bool,
    ) -> Result<(), AppError> {
        let mut server = state
            .db
            .get_mcp_server(id)?
            .ok_or_else(|| AppError::InvalidInput(format!("MCP 服务器 '{}' 不存在", id)))?;

        server.apps.set_enabled_for(&app, enable);
        state.db.update_mcp_server_apps(id, &server.apps)?;

        // 同步到该应用
        Self::sync_to_app(state, &app)?;

        Ok(())
    }

    /// 从应用导入 MCP 服务器配置
    pub fn import_from_app(state: &AppState, app: AppType) -> Result<Vec<String>, AppError> {
        let servers = Self::read_app_mcp_config(&app)?;
        let mut imported = Vec::new();

        for (id, config) in servers {
            // 检查是否已存在
            if state.db.get_mcp_server(&id)?.is_some() {
                continue;
            }

            let mut server = McpServer::new(&id, &id, config);
            server.apps.set_enabled_for(&app, true);

            state.db.save_mcp_server(&server)?;
            imported.push(id);
        }

        Ok(imported)
    }

    /// 同步所有 MCP 服务器到所有应用
    pub fn sync_all(state: &AppState) -> Result<(), AppError> {
        for app in AppType::all() {
            Self::sync_to_app(state, app)?;
        }
        Ok(())
    }

    /// 同步 MCP 服务器到指定应用
    pub fn sync_to_app(state: &AppState, app: &AppType) -> Result<(), AppError> {
        let servers = state.db.get_all_mcp_servers()?;

        // 筛选出为该应用启用的服务器
        let enabled_servers: IndexMap<String, Value> = servers
            .iter()
            .filter(|(_, s)| s.apps.is_enabled_for(app))
            .map(|(id, s)| (id.clone(), s.server_config.clone()))
            .collect();

        match app {
            AppType::Claude => Self::write_claude_mcp(&enabled_servers)?,
            AppType::Codex => Self::write_codex_mcp(&enabled_servers)?,
            AppType::Gemini => Self::write_gemini_mcp(&enabled_servers)?,
            AppType::OpenCode => Self::write_opencode_mcp(&enabled_servers)?,
        }

        Ok(())
    }

    /// 同步单个服务器到其启用的应用
    fn sync_to_apps(state: &AppState, server: &McpServer) -> Result<(), AppError> {
        for app in server.apps.enabled_apps() {
            Self::sync_to_app(state, &app)?;
        }
        Ok(())
    }

    // ===== 配置文件读取 =====

    /// 读取应用的 MCP 配置
    fn read_app_mcp_config(app: &AppType) -> Result<IndexMap<String, Value>, AppError> {
        match app {
            AppType::Claude => Self::read_claude_mcp(),
            AppType::Codex => Self::read_codex_mcp(),
            AppType::Gemini => Self::read_gemini_mcp(),
            AppType::OpenCode => Self::read_opencode_mcp(),
        }
    }

    /// 读取 Claude MCP 配置
    fn read_claude_mcp() -> Result<IndexMap<String, Value>, AppError> {
        let path = get_claude_mcp_path();
        if !path.exists() {
            return Ok(IndexMap::new());
        }

        let config: Value = read_json_file(&path)?;
        let servers = config
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<IndexMap<_, _>>()
            })
            .unwrap_or_default();

        Ok(servers)
    }

    /// 读取 Codex MCP 配置
    fn read_codex_mcp() -> Result<IndexMap<String, Value>, AppError> {
        let path = get_codex_config_dir().join("config.toml");
        if !path.exists() {
            return Ok(IndexMap::new());
        }

        let content = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
        let config: toml::Value = content
            .parse()
            .map_err(|e| AppError::toml(&path, e))?;

        let servers = config
            .get("mcp_servers")
            .and_then(|v| v.as_table())
            .map(|table| {
                table
                    .iter()
                    .map(|(k, v)| {
                        let json_value = toml_to_json(v);
                        (k.clone(), json_value)
                    })
                    .collect::<IndexMap<_, _>>()
            })
            .unwrap_or_default();

        Ok(servers)
    }

    /// 读取 Gemini MCP 配置
    fn read_gemini_mcp() -> Result<IndexMap<String, Value>, AppError> {
        let path = get_gemini_config_dir().join("settings.json");
        if !path.exists() {
            return Ok(IndexMap::new());
        }

        let config: Value = read_json_file(&path)?;
        let servers = config
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<IndexMap<_, _>>()
            })
            .unwrap_or_default();

        Ok(servers)
    }

    /// 读取 OpenCode MCP 配置
    fn read_opencode_mcp() -> Result<IndexMap<String, Value>, AppError> {
        let path = get_opencode_config_dir().join("opencode.json");
        if !path.exists() {
            return Ok(IndexMap::new());
        }

        let config: Value = read_json_file(&path)?;
        let servers = config
            .get("mcpServers")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<IndexMap<_, _>>()
            })
            .unwrap_or_default();

        Ok(servers)
    }

    // ===== 配置文件写入 =====

    /// 写入 Claude MCP 配置
    fn write_claude_mcp(servers: &IndexMap<String, Value>) -> Result<(), AppError> {
        let path = get_claude_mcp_path();

        // 读取现有配置或创建新配置
        let mut config: Value = if path.exists() {
            read_json_file(&path).unwrap_or_else(|_| json!({}))
        } else {
            json!({})
        };

        // 更新 mcpServers
        let mcp_servers: serde_json::Map<String, Value> = servers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        config["mcpServers"] = Value::Object(mcp_servers);

        write_json_file(&path, &config)
    }

    /// 写入 Codex MCP 配置
    fn write_codex_mcp(servers: &IndexMap<String, Value>) -> Result<(), AppError> {
        let path = get_codex_config_dir().join("config.toml");

        // 读取现有配置或创建新配置
        let mut config: toml::Value = if path.exists() {
            let content = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
            content.parse().unwrap_or_else(|_| toml::Value::Table(toml::map::Map::new()))
        } else {
            toml::Value::Table(toml::map::Map::new())
        };

        // 转换为 TOML 格式
        let mut mcp_table = toml::map::Map::new();
        for (id, server_config) in servers {
            mcp_table.insert(id.clone(), json_to_toml(server_config));
        }

        if let toml::Value::Table(ref mut table) = config {
            table.insert("mcp_servers".to_string(), toml::Value::Table(mcp_table));
        }

        let toml_str = toml::to_string_pretty(&config)
            .map_err(|e| AppError::Config(format!("TOML 序列化失败: {}", e)))?;

        write_text_file(&path, &toml_str)
    }

    /// 写入 Gemini MCP 配置
    fn write_gemini_mcp(servers: &IndexMap<String, Value>) -> Result<(), AppError> {
        let path = get_gemini_config_dir().join("settings.json");

        // 读取现有配置或创建新配置
        let mut config: Value = if path.exists() {
            read_json_file(&path).unwrap_or_else(|_| json!({}))
        } else {
            json!({})
        };

        // 更新 mcpServers
        let mcp_servers: serde_json::Map<String, Value> = servers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        config["mcpServers"] = Value::Object(mcp_servers);

        write_json_file(&path, &config)
    }

    /// 写入 OpenCode MCP 配置
    fn write_opencode_mcp(servers: &IndexMap<String, Value>) -> Result<(), AppError> {
        let path = get_opencode_config_dir().join("opencode.json");

        // 读取现有配置或创建新配置
        let mut config: Value = if path.exists() {
            read_json_file(&path).unwrap_or_else(|_| json!({}))
        } else {
            json!({})
        };

        // 更新 mcpServers
        let mcp_servers: serde_json::Map<String, Value> = servers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        config["mcpServers"] = Value::Object(mcp_servers);

        write_json_file(&path, &config)
    }
}

/// 将 TOML 值转换为 JSON 值
fn toml_to_json(value: &toml::Value) -> Value {
    match value {
        toml::Value::String(s) => Value::String(s.clone()),
        toml::Value::Integer(i) => Value::Number((*i).into()),
        toml::Value::Float(f) => {
            serde_json::Number::from_f64(*f)
                .map(Value::Number)
                .unwrap_or(Value::Null)
        }
        toml::Value::Boolean(b) => Value::Bool(*b),
        toml::Value::Array(arr) => Value::Array(arr.iter().map(toml_to_json).collect()),
        toml::Value::Table(table) => {
            let obj: serde_json::Map<String, Value> = table
                .iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect();
            Value::Object(obj)
        }
        toml::Value::Datetime(dt) => Value::String(dt.to_string()),
    }
}

/// 将 JSON 值转换为 TOML 值
fn json_to_toml(value: &Value) -> toml::Value {
    match value {
        Value::Null => toml::Value::String("".to_string()),
        Value::Bool(b) => toml::Value::Boolean(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                toml::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                toml::Value::Float(f)
            } else {
                toml::Value::String(n.to_string())
            }
        }
        Value::String(s) => toml::Value::String(s.clone()),
        Value::Array(arr) => toml::Value::Array(arr.iter().map(json_to_toml).collect()),
        Value::Object(obj) => {
            let table: toml::map::Map<String, toml::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), json_to_toml(v)))
                .collect();
            toml::Value::Table(table)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mcp_service_add_and_list() {
        let state = AppState::memory().unwrap();

        let server = McpServer::new("test-server", "Test Server", json!({"command": "npx"}));
        McpService::add(&state, server).unwrap();

        let servers = McpService::list(&state).unwrap();
        assert_eq!(servers.len(), 1);
        assert!(servers.contains_key("test-server"));
    }

    #[test]
    fn test_mcp_service_toggle() {
        let state = AppState::memory().unwrap();

        let server = McpServer::new("test-server", "Test Server", json!({"command": "npx"}));
        McpService::add(&state, server).unwrap();

        McpService::toggle(&state, "test-server", AppType::Claude, true).unwrap();

        let server = McpService::get(&state, "test-server").unwrap().unwrap();
        assert!(server.apps.claude);
        assert!(!server.apps.codex);
    }
}
