//! MCP 服务器数据结构模块
//!
//! 定义 MCP (Model Context Protocol) 服务器的数据结构。

use serde::{Deserialize, Serialize};

use crate::app_config::McpApps;

/// MCP 服务器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    /// 唯一标识符
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 服务器配置 (JSON 格式，包含 command, args, env 等)
    pub server_config: serde_json::Value,
    /// 应用启用状态
    #[serde(default)]
    pub apps: McpApps,
    /// 描述
    pub description: Option<String>,
    /// 主页链接
    pub homepage: Option<String>,
    /// 文档链接
    pub docs: Option<String>,
    /// 标签列表
    #[serde(default)]
    pub tags: Vec<String>,
    /// 创建时间 (Unix 时间戳)
    pub created_at: Option<i64>,
    /// 排序索引
    pub sort_index: Option<usize>,
}

impl McpServer {
    /// 创建新的 MCP 服务器配置
    pub fn new(id: impl Into<String>, name: impl Into<String>, server_config: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            server_config,
            apps: McpApps::default(),
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
            created_at: Some(chrono::Utc::now().timestamp()),
            sort_index: None,
        }
    }

    /// 设置描述
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// 设置主页链接
    pub fn with_homepage(mut self, homepage: impl Into<String>) -> Self {
        self.homepage = Some(homepage.into());
        self
    }

    /// 设置标签
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// 检查是否为任何应用启用
    pub fn is_enabled_for_any(&self) -> bool {
        !self.apps.is_empty()
    }

    /// 获取启用的应用列表字符串
    pub fn enabled_apps_str(&self) -> String {
        let apps = self.apps.enabled_apps();
        if apps.is_empty() {
            "无".to_string()
        } else {
            apps.iter()
                .map(|a| a.display_name())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
}

/// MCP 服务器的 stdio 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStdioConfig {
    /// 执行命令
    pub command: String,
    /// 命令参数
    #[serde(default)]
    pub args: Vec<String>,
    /// 环境变量
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
}

impl McpStdioConfig {
    /// 创建新的 stdio 配置
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            env: std::collections::HashMap::new(),
        }
    }

    /// 添加参数
    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// 添加环境变量
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// 转换为 JSON Value
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "command": self.command,
            "args": self.args,
            "env": self.env
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_mcp_server_new() {
        let server = McpServer::new("test-server", "Test Server", json!({"command": "npx"}));
        assert_eq!(server.id, "test-server");
        assert_eq!(server.name, "Test Server");
        assert!(server.apps.is_empty());
    }

    #[test]
    fn test_mcp_stdio_config() {
        let config = McpStdioConfig::new("npx")
            .with_args(vec!["-y".to_string(), "@test/server".to_string()])
            .with_env("API_KEY", "test-key");

        assert_eq!(config.command, "npx");
        assert_eq!(config.args.len(), 2);
        assert_eq!(config.env.get("API_KEY"), Some(&"test-key".to_string()));
    }
}
