//! OpenClaw 配置文件读写模块
//!
//! 处理 `~/.openclaw/openclaw.json` 配置文件的读写操作（JSON5 格式）。
//! OpenClaw 使用累加式供应商管理，所有供应商配置共存于同一配置文件中。

use crate::config::{atomic_write, get_app_config_dir, get_openclaw_config_dir};
use crate::error::AppError;
use chrono::Local;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

const OPENCLAW_TOOLS_PROFILES: &[&str] = &["minimal", "coding", "messaging", "full"];

// ============================================================================
// Path Functions
// ============================================================================

/// 获取 OpenClaw 配置文件路径
///
/// 返回 `~/.openclaw/openclaw.json`
pub fn get_openclaw_config_path() -> PathBuf {
    get_openclaw_config_dir().join("openclaw.json")
}

fn default_openclaw_config_value() -> Value {
    json!({
        "models": {
            "mode": "merge",
            "providers": {}
        }
    })
}

fn openclaw_write_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

// ============================================================================
// Type Definitions
// ============================================================================

/// OpenClaw 健康检查警告
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OpenClawHealthWarning {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

/// OpenClaw 写入结果
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct OpenClawWriteOutcome {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<OpenClawHealthWarning>,
}

/// OpenClaw 供应商配置（对应 models.providers 中的条目）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenClawProviderConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<OpenClawModelEntry>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl OpenClawProviderConfig {
    /// 创建新的供应商配置
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            base_url: Some(base_url.into()),
            api_key: Some(api_key.into()),
            api: None,
            models: Vec::new(),
            headers: HashMap::new(),
            extra: HashMap::new(),
        }
    }

    /// 添加模型
    pub fn with_model(mut self, model: OpenClawModelEntry) -> Self {
        self.models.push(model);
        self
    }
}

/// OpenClaw 模型条目
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenClawModelEntry {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<OpenClawModelCost>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl OpenClawModelEntry {
    /// 创建新的模型条目
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: None,
            alias: None,
            cost: None,
            context_window: None,
            extra: HashMap::new(),
        }
    }

    /// 设置名称
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }
}

/// OpenClaw 模型成本配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawModelCost {
    pub input: f64,
    pub output: f64,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// OpenClaw 默认模型配置（agents.defaults.model）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawDefaultModel {
    pub primary: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fallbacks: Vec<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl OpenClawDefaultModel {
    /// 创建新的默认模型配置
    pub fn new(primary: impl Into<String>) -> Self {
        Self {
            primary: primary.into(),
            fallbacks: Vec::new(),
            extra: HashMap::new(),
        }
    }
}

/// OpenClaw 模型目录条目（agents.defaults.models 中的值）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawModelCatalogEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// OpenClaw agents.defaults 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawAgentsDefaults {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<OpenClawDefaultModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub models: Option<HashMap<String, OpenClawModelCatalogEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// OpenClaw env 配置（openclaw.json 的 env 节点）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawEnvConfig {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub vars: HashMap<String, Value>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub shell_env: HashMap<String, Value>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl Default for OpenClawEnvConfig {
    fn default() -> Self {
        Self {
            vars: HashMap::new(),
            shell_env: HashMap::new(),
            extra: HashMap::new(),
        }
    }
}

/// OpenClaw tools 配置（openclaw.json 的 tools 节点）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenClawToolsConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl Default for OpenClawToolsConfig {
    fn default() -> Self {
        Self {
            profile: None,
            allow: Vec::new(),
            deny: Vec::new(),
            extra: HashMap::new(),
        }
    }
}

// ============================================================================
// Core Read/Write Functions
// ============================================================================

/// 读取 OpenClaw 配置文件
///
/// 支持 JSON5 格式，返回完整的配置 JSON 对象
pub fn read_openclaw_config() -> Result<Value, AppError> {
    let path = get_openclaw_config_path();
    if !path.exists() {
        return Ok(default_openclaw_config_value());
    }

    let content = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
    json5::from_str(&content)
        .map_err(|e| AppError::Config(format!("Failed to parse OpenClaw config as JSON5: {e}")))
}

/// 写入 OpenClaw 配置文件
pub fn write_openclaw_config(config: &Value) -> Result<OpenClawWriteOutcome, AppError> {
    let _guard = openclaw_write_lock().lock().map_err(|e| AppError::Lock(e.to_string()))?;

    let path = get_openclaw_config_path();

    // 创建备份
    let backup_path = if path.exists() {
        let content = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
        Some(create_openclaw_backup(&content)?)
    } else {
        None
    };

    // 确保目录存在
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    // 写入配置
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| AppError::JsonSerialize { source: e })?;
    atomic_write(&path, content.as_bytes())?;

    // 健康检查
    let warnings = scan_openclaw_health_from_value(config);

    Ok(OpenClawWriteOutcome {
        backup_path: backup_path.map(|p| p.display().to_string()),
        warnings,
    })
}

/// 对现有 OpenClaw 配置做健康检查
pub fn scan_openclaw_config_health() -> Result<Vec<OpenClawHealthWarning>, AppError> {
    let path = get_openclaw_config_path();
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&path).map_err(|e| AppError::io(&path, e))?;
    match json5::from_str::<Value>(&content) {
        Ok(config) => Ok(scan_openclaw_health_from_value(&config)),
        Err(err) => Ok(vec![OpenClawHealthWarning {
            code: "config_parse_failed".to_string(),
            message: format!("OpenClaw config could not be parsed as JSON5: {err}"),
            path: Some(path.display().to_string()),
        }]),
    }
}

fn scan_openclaw_health_from_value(config: &Value) -> Vec<OpenClawHealthWarning> {
    let mut warnings = Vec::new();

    // 检查 tools.profile
    if let Some(profile) = config
        .get("tools")
        .and_then(|tools| tools.get("profile"))
        .and_then(Value::as_str)
    {
        if !OPENCLAW_TOOLS_PROFILES.contains(&profile) {
            warnings.push(OpenClawHealthWarning {
                code: "invalid_tools_profile".to_string(),
                message: format!("tools.profile uses unsupported value '{profile}'."),
                path: Some("tools.profile".to_string()),
            });
        }
    }

    // 检查 legacy timeout
    if config
        .get("agents")
        .and_then(|agents| agents.get("defaults"))
        .and_then(|defaults| defaults.get("timeout"))
        .is_some()
    {
        warnings.push(OpenClawHealthWarning {
            code: "legacy_agents_timeout".to_string(),
            message: "agents.defaults.timeout is deprecated; use agents.defaults.timeoutSeconds."
                .to_string(),
            path: Some("agents.defaults.timeout".to_string()),
        });
    }

    // 检查 env.vars 格式
    if let Some(value) = config.get("env").and_then(|env| env.get("vars")) {
        if !value.is_object() {
            warnings.push(OpenClawHealthWarning {
                code: "stringified_env_vars".to_string(),
                message: "env.vars should be an object.".to_string(),
                path: Some("env.vars".to_string()),
            });
        }
    }

    warnings
}

fn create_openclaw_backup(source: &str) -> Result<PathBuf, AppError> {
    let backup_dir = get_app_config_dir().join("backups").join("openclaw");
    fs::create_dir_all(&backup_dir).map_err(|e| AppError::io(&backup_dir, e))?;

    let base_id = format!("openclaw_{}", Local::now().format("%Y%m%d_%H%M%S"));
    let mut filename = format!("{base_id}.json");
    let mut backup_path = backup_dir.join(&filename);
    let mut counter = 1;

    while backup_path.exists() {
        filename = format!("{base_id}_{counter}.json");
        backup_path = backup_dir.join(&filename);
        counter += 1;
    }

    atomic_write(&backup_path, source.as_bytes())?;
    cleanup_openclaw_backups(&backup_dir)?;
    Ok(backup_path)
}

fn cleanup_openclaw_backups(dir: &PathBuf) -> Result<(), AppError> {
    let retain = 10; // 保留最近 10 个备份
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(|e| AppError::io(dir, e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|ext| ext == "json" || ext == "json5")
                .unwrap_or(false)
        })
        .collect();

    if entries.len() <= retain {
        return Ok(());
    }

    entries.sort_by_key(|entry| entry.metadata().and_then(|m| m.modified()).ok());
    let remove_count = entries.len().saturating_sub(retain);
    for entry in entries.into_iter().take(remove_count) {
        let _ = fs::remove_file(entry.path());
    }

    Ok(())
}

// ============================================================================
// Provider Functions
// ============================================================================

/// 获取所有供应商配置（原始 JSON）
pub fn get_providers() -> Result<Map<String, Value>, AppError> {
    let config = read_openclaw_config()?;
    Ok(config
        .get("models")
        .and_then(|m| m.get("providers"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default())
}

/// 获取单个供应商配置
pub fn get_provider(id: &str) -> Result<Option<Value>, AppError> {
    Ok(get_providers()?.get(id).cloned())
}

/// 设置供应商配置
pub fn set_provider(id: &str, provider_config: Value) -> Result<OpenClawWriteOutcome, AppError> {
    let mut config = read_openclaw_config()?;
    ensure_object_path(&mut config, &["models", "providers"]);

    if let Some(providers) = config
        .get_mut("models")
        .and_then(|m| m.get_mut("providers"))
        .and_then(|p| p.as_object_mut())
    {
        providers.insert(id.to_string(), provider_config);
    }

    write_openclaw_config(&config)
}

/// 删除供应商配置
pub fn remove_provider(id: &str) -> Result<OpenClawWriteOutcome, AppError> {
    let mut config = read_openclaw_config()?;
    let mut removed = false;

    if let Some(providers) = config
        .get_mut("models")
        .and_then(|models| models.get_mut("providers"))
        .and_then(Value::as_object_mut)
    {
        removed = providers.remove(id).is_some();
    }

    if !removed {
        return Ok(OpenClawWriteOutcome::default());
    }

    write_openclaw_config(&config)
}

/// 获取所有供应商配置（类型化）
pub fn get_typed_providers() -> Result<IndexMap<String, OpenClawProviderConfig>, AppError> {
    let providers = get_providers()?;
    let mut result = IndexMap::new();

    for (id, value) in providers {
        match serde_json::from_value::<OpenClawProviderConfig>(value.clone()) {
            Ok(config) => {
                result.insert(id, config);
            }
            Err(e) => {
                log::warn!("Failed to parse OpenClaw provider '{id}': {e}");
            }
        }
    }

    Ok(result)
}

/// 设置供应商配置（类型化）
pub fn set_typed_provider(
    id: &str,
    config: &OpenClawProviderConfig,
) -> Result<OpenClawWriteOutcome, AppError> {
    let value = serde_json::to_value(config).map_err(|e| AppError::JsonSerialize { source: e })?;
    set_provider(id, value)
}

// ============================================================================
// Agents Configuration Functions
// ============================================================================

/// 读取默认模型配置（agents.defaults.model）
pub fn get_default_model() -> Result<Option<OpenClawDefaultModel>, AppError> {
    let config = read_openclaw_config()?;

    let Some(model_value) = config
        .get("agents")
        .and_then(|a| a.get("defaults"))
        .and_then(|d| d.get("model"))
    else {
        return Ok(None);
    };

    let model = serde_json::from_value(model_value.clone())
        .map_err(|e| AppError::Config(format!("Failed to parse agents.defaults.model: {e}")))?;
    Ok(Some(model))
}

/// 设置默认模型配置
pub fn set_default_model(model: &OpenClawDefaultModel) -> Result<OpenClawWriteOutcome, AppError> {
    let mut config = read_openclaw_config()?;
    ensure_object_path(&mut config, &["agents", "defaults"]);

    let model_value =
        serde_json::to_value(model).map_err(|e| AppError::JsonSerialize { source: e })?;

    if let Some(defaults) = config
        .get_mut("agents")
        .and_then(|a| a.as_object_mut())
        .and_then(|obj| obj.get_mut("defaults"))
        .and_then(|d| d.as_object_mut())
    {
        defaults.insert("model".to_string(), model_value);
    }

    write_openclaw_config(&config)
}

/// 获取完整的 agents.defaults 配置
pub fn get_agents_defaults() -> Result<Option<OpenClawAgentsDefaults>, AppError> {
    let config = read_openclaw_config()?;

    let Some(defaults_value) = config.get("agents").and_then(|a| a.get("defaults")) else {
        return Ok(None);
    };

    let defaults = serde_json::from_value(defaults_value.clone())
        .map_err(|e| AppError::Config(format!("Failed to parse agents.defaults: {e}")))?;
    Ok(Some(defaults))
}

/// 设置完整的 agents.defaults 配置
pub fn set_agents_defaults(defaults: &OpenClawAgentsDefaults) -> Result<OpenClawWriteOutcome, AppError> {
    let mut config = read_openclaw_config()?;
    ensure_object_path(&mut config, &["agents"]);

    let defaults_value =
        serde_json::to_value(defaults).map_err(|e| AppError::JsonSerialize { source: e })?;

    if let Some(agents) = config.get_mut("agents").and_then(|a| a.as_object_mut()) {
        agents.insert("defaults".to_string(), defaults_value);
    }

    write_openclaw_config(&config)
}

// ============================================================================
// Env Configuration
// ============================================================================

/// 读取 env 配置
pub fn get_env_config() -> Result<OpenClawEnvConfig, AppError> {
    let config = read_openclaw_config()?;

    let Some(env_value) = config.get("env") else {
        return Ok(OpenClawEnvConfig::default());
    };

    serde_json::from_value(env_value.clone())
        .map_err(|e| AppError::Config(format!("Failed to parse env config: {e}")))
}

/// 设置 env 配置
pub fn set_env_config(env: &OpenClawEnvConfig) -> Result<OpenClawWriteOutcome, AppError> {
    let mut config = read_openclaw_config()?;
    let value = serde_json::to_value(env).map_err(|e| AppError::JsonSerialize { source: e })?;

    if let Some(obj) = config.as_object_mut() {
        obj.insert("env".to_string(), value);
    }

    write_openclaw_config(&config)
}

// ============================================================================
// Tools Configuration
// ============================================================================

/// 读取 tools 配置
pub fn get_tools_config() -> Result<OpenClawToolsConfig, AppError> {
    let config = read_openclaw_config()?;

    let Some(tools_value) = config.get("tools") else {
        return Ok(OpenClawToolsConfig::default());
    };

    serde_json::from_value(tools_value.clone())
        .map_err(|e| AppError::Config(format!("Failed to parse tools config: {e}")))
}

/// 设置 tools 配置
pub fn set_tools_config(tools: &OpenClawToolsConfig) -> Result<OpenClawWriteOutcome, AppError> {
    let mut config = read_openclaw_config()?;
    let value = serde_json::to_value(tools).map_err(|e| AppError::JsonSerialize { source: e })?;

    if let Some(obj) = config.as_object_mut() {
        obj.insert("tools".to_string(), value);
    }

    write_openclaw_config(&config)
}

// ============================================================================
// Model Catalog Configuration
// ============================================================================

/// 获取模型目录配置 (agents.defaults.models)
pub fn get_model_catalog() -> Result<Option<HashMap<String, OpenClawModelCatalogEntry>>, AppError> {
    let config = read_openclaw_config()?;

    let Some(models_value) = config
        .get("agents")
        .and_then(|a| a.get("defaults"))
        .and_then(|d| d.get("models"))
    else {
        return Ok(None);
    };

    let models = serde_json::from_value(models_value.clone())
        .map_err(|e| AppError::Config(format!("Failed to parse agents.defaults.models: {e}")))?;
    Ok(Some(models))
}

/// 设置模型目录配置 (agents.defaults.models)
pub fn set_model_catalog(
    catalog: &HashMap<String, OpenClawModelCatalogEntry>,
) -> Result<OpenClawWriteOutcome, AppError> {
    let mut config = read_openclaw_config()?;
    ensure_object_path(&mut config, &["agents", "defaults"]);

    let catalog_value =
        serde_json::to_value(catalog).map_err(|e| AppError::JsonSerialize { source: e })?;

    if let Some(defaults) = config
        .get_mut("agents")
        .and_then(|a| a.as_object_mut())
        .and_then(|obj| obj.get_mut("defaults"))
        .and_then(|d| d.as_object_mut())
    {
        defaults.insert("models".to_string(), catalog_value);
    }

    write_openclaw_config(&config)
}

/// 添加单个模型到目录
pub fn add_model_to_catalog(
    model_id: &str,
    entry: &OpenClawModelCatalogEntry,
) -> Result<OpenClawWriteOutcome, AppError> {
    let mut catalog = get_model_catalog()?.unwrap_or_default();
    catalog.insert(model_id.to_string(), entry.clone());
    set_model_catalog(&catalog)
}

/// 从目录移除模型
pub fn remove_model_from_catalog(model_id: &str) -> Result<OpenClawWriteOutcome, AppError> {
    let mut catalog = get_model_catalog()?.unwrap_or_default();
    catalog.remove(model_id);
    set_model_catalog(&catalog)
}

// ============================================================================
// Helper Functions
// ============================================================================

fn ensure_object_path(config: &mut Value, path: &[&str]) {
    if path.is_empty() {
        return;
    }

    // 确保根是对象
    if !config.is_object() {
        *config = Value::Object(Map::new());
    }

    // 逐级创建路径
    let mut current = config;
    for key in path {
        let obj = current.as_object_mut().unwrap();
        current = obj
            .entry(key.to_string())
            .or_insert_with(|| Value::Object(Map::new()));

        if !current.is_object() {
            *current = Value::Object(Map::new());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model_new() {
        let model = OpenClawDefaultModel::new("provider/model");
        assert_eq!(model.primary, "provider/model");
        assert!(model.fallbacks.is_empty());
    }

    #[test]
    fn test_provider_config_new() {
        let config = OpenClawProviderConfig::new("https://api.example.com", "sk-xxx");
        assert_eq!(config.base_url, Some("https://api.example.com".to_string()));
        assert_eq!(config.api_key, Some("sk-xxx".to_string()));
    }

    #[test]
    fn test_health_warnings() {
        let config = json!({
            "tools": { "profile": "invalid" },
            "agents": { "defaults": { "timeout": 30 } }
        });

        let warnings = scan_openclaw_health_from_value(&config);
        assert!(!warnings.is_empty());
        assert!(warnings.iter().any(|w| w.code == "invalid_tools_profile"));
        assert!(warnings.iter().any(|w| w.code == "legacy_agents_timeout"));
    }
}