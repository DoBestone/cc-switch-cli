//! 供应商数据结构模块
//!
//! 定义供应商、配置等核心数据结构。

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// 供应商结构体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    /// 唯一标识符
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 配置内容（JSON 格式）
    #[serde(rename = "settingsConfig")]
    pub settings_config: Value,
    /// 供应商网站 URL
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "websiteUrl")]
    pub website_url: Option<String>,
    /// 分类
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    /// 创建时间戳
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "createdAt")]
    pub created_at: Option<i64>,
    /// 排序索引
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "sortIndex")]
    pub sort_index: Option<usize>,
    /// 备注信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    /// 供应商元数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<ProviderMeta>,
    /// 图标名称
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// 图标颜色（Hex 格式）
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "iconColor")]
    pub icon_color: Option<String>,
    /// 是否加入故障转移队列
    #[serde(default)]
    #[serde(rename = "inFailoverQueue")]
    pub in_failover_queue: bool,
}

impl Provider {
    /// 创建新供应商
    pub fn new(id: impl Into<String>, name: impl Into<String>, settings_config: Value) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            settings_config,
            website_url: None,
            category: None,
            created_at: Some(chrono::Utc::now().timestamp()),
            sort_index: None,
            notes: None,
            meta: None,
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        }
    }

    /// 从现有 ID 创建供应商
    pub fn with_id(
        id: String,
        name: String,
        settings_config: Value,
        website_url: Option<String>,
    ) -> Self {
        Self {
            id,
            name,
            settings_config,
            website_url,
            category: None,
            created_at: None,
            sort_index: None,
            notes: None,
            meta: None,
            icon: None,
            icon_color: None,
            in_failover_queue: false,
        }
    }

    /// 获取 Base URL（从配置中提取）
    pub fn get_base_url(&self) -> Option<String> {
        // 尝试从 env 中获取
        if let Some(env) = self.settings_config.get("env") {
            // Claude
            if let Some(url) = env.get("ANTHROPIC_BASE_URL").and_then(|v| v.as_str()) {
                return Some(url.to_string());
            }
        }
        // 尝试从 config 字段获取 (Codex TOML)
        if let Some(config) = self.settings_config.get("config").and_then(|v| v.as_str()) {
            for line in config.lines() {
                if line.trim().starts_with("base_url") {
                    if let Some(url) = line.split('=').nth(1) {
                        return Some(url.trim().trim_matches('"').to_string());
                    }
                }
            }
        }
        None
    }

    /// 获取模型名称
    pub fn get_model(&self) -> Option<String> {
        if let Some(env) = self.settings_config.get("env") {
            if let Some(model) = env.get("ANTHROPIC_MODEL").and_then(|v| v.as_str()) {
                return Some(model.to_string());
            }
        }
        None
    }
}

impl Default for Provider {
    fn default() -> Self {
        Self::new("default", "Default", Value::Object(serde_json::Map::new()))
    }
}

/// 供应商管理器
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderManager {
    /// 供应商映射（保持插入顺序）
    pub providers: IndexMap<String, Provider>,
    /// 当前激活的供应商 ID
    pub current: String,
}

impl ProviderManager {
    /// 创建空的管理器
    pub fn new() -> Self {
        Self::default()
    }

    /// 获取当前供应商
    pub fn current_provider(&self) -> Option<&Provider> {
        self.providers.get(&self.current)
    }

    /// 添加供应商
    pub fn add(&mut self, provider: Provider) {
        let id = provider.id.clone();
        self.providers.insert(id.clone(), provider);
        if self.current.is_empty() {
            self.current = id;
        }
    }

    /// 删除供应商
    pub fn remove(&mut self, id: &str) -> Option<Provider> {
        self.providers.swap_remove(id)
    }

    /// 切换当前供应商
    pub fn switch(&mut self, id: &str) -> bool {
        if self.providers.contains_key(id) {
            self.current = id.to_string();
            true
        } else {
            false
        }
    }

    /// 获取供应商数量
    pub fn len(&self) -> usize {
        self.providers.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.providers.is_empty()
    }
}

/// 自定义端点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomEndpoint {
    pub url: String,
    pub added_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used: Option<i64>,
}

/// 用量查询脚本配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageScript {
    pub enabled: bool,
    pub language: String,
    pub code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "apiKey")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "baseUrl")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "accessToken")]
    pub access_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "userId")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "templateType")]
    pub template_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "autoQueryInterval")]
    pub auto_query_interval: Option<u64>,
}

/// 供应商单独的测试配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderTestConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(rename = "testModel", skip_serializing_if = "Option::is_none")]
    pub test_model: Option<String>,
    #[serde(rename = "timeoutSecs", skip_serializing_if = "Option::is_none")]
    pub timeout_secs: Option<u64>,
    #[serde(rename = "testPrompt", skip_serializing_if = "Option::is_none")]
    pub test_prompt: Option<String>,
    #[serde(
        rename = "degradedThresholdMs",
        skip_serializing_if = "Option::is_none"
    )]
    pub degraded_threshold_ms: Option<u64>,
    #[serde(rename = "maxRetries", skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
}

/// 供应商单独的代理配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderProxyConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(rename = "proxyType", skip_serializing_if = "Option::is_none")]
    pub proxy_type: Option<String>,
    #[serde(rename = "proxyHost", skip_serializing_if = "Option::is_none")]
    pub proxy_host: Option<String>,
    #[serde(rename = "proxyPort", skip_serializing_if = "Option::is_none")]
    pub proxy_port: Option<u16>,
    #[serde(rename = "proxyUsername", skip_serializing_if = "Option::is_none")]
    pub proxy_username: Option<String>,
    #[serde(rename = "proxyPassword", skip_serializing_if = "Option::is_none")]
    pub proxy_password: Option<String>,
}

/// 供应商元数据
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderMeta {
    /// 自定义端点列表
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub custom_endpoints: HashMap<String, CustomEndpoint>,
    /// 用量查询脚本配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage_script: Option<UsageScript>,
    /// 供应商单独的测试配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_config: Option<ProviderTestConfig>,
    /// 供应商单独的代理配置
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_config: Option<ProviderProxyConfig>,
}

/// 用量数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageData {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "planName")]
    pub plan_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isValid")]
    pub is_valid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "invalidMessage")]
    pub invalid_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

/// 用量查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<UsageData>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_provider_creation() {
        let provider = Provider::new("test-id", "Test Provider", json!({"key": "value"}));
        assert_eq!(provider.id, "test-id");
        assert_eq!(provider.name, "Test Provider");
        assert!(provider.created_at.is_some());
    }

    #[test]
    fn test_provider_manager() {
        let mut manager = ProviderManager::new();
        assert!(manager.is_empty());

        let provider = Provider::new("p1", "Provider 1", json!({}));
        manager.add(provider);
        assert_eq!(manager.len(), 1);
        assert_eq!(manager.current, "p1");

        let provider2 = Provider::new("p2", "Provider 2", json!({}));
        manager.add(provider2);
        assert_eq!(manager.len(), 2);

        assert!(manager.switch("p2"));
        assert_eq!(manager.current, "p2");

        assert!(!manager.switch("nonexistent"));
    }

    #[test]
    fn test_get_base_url() {
        let provider = Provider::new(
            "test",
            "Test",
            json!({
                "env": {
                    "ANTHROPIC_BASE_URL": "https://api.example.com"
                }
            }),
        );
        assert_eq!(
            provider.get_base_url(),
            Some("https://api.example.com".to_string())
        );
    }
}
