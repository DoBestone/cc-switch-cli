//! 供应商服务模块
//!
//! 处理供应商的 CRUD 操作、切换和配置管理。

use indexmap::IndexMap;
use serde_json::Value;

use crate::app_config::AppType;
use crate::config::{
    get_claude_settings_path, get_codex_auth_path, get_codex_config_path,
    get_gemini_settings_path, read_json_file, write_json_file, write_text_file,
};
use crate::error::AppError;
use crate::provider::Provider;
use crate::settings;
use crate::store::AppState;

/// 供应商业务逻辑服务
pub struct ProviderService;

impl ProviderService {
    /// 列出指定应用类型的所有供应商
    pub fn list(state: &AppState, app_type: AppType) -> Result<IndexMap<String, Provider>, AppError> {
        state.db.get_all_providers(app_type.as_str())
    }

    /// 获取当前供应商 ID
    ///
    /// 优先从本地 settings 读取，fallback 到数据库
    pub fn current(state: &AppState, app_type: AppType) -> Result<String, AppError> {
        // OpenCode 使用累加模式，不存在"当前供应商"概念
        if app_type.is_additive_mode() {
            return Ok(String::new());
        }

        // 优先从本地设置读取
        if let Some(id) = settings::get_current_provider(&app_type) {
            // 验证该供应商是否存在
            let providers = state.db.get_all_providers(app_type.as_str())?;
            if providers.contains_key(&id) {
                return Ok(id);
            }
        }

        // Fallback 到数据库
        state
            .db
            .get_current_provider(app_type.as_str())
            .map(|opt| opt.unwrap_or_default())
    }

    /// 获取当前供应商详情
    pub fn current_provider(state: &AppState, app_type: AppType) -> Result<Option<Provider>, AppError> {
        let current_id = Self::current(state, app_type.clone())?;
        if current_id.is_empty() {
            return Ok(None);
        }

        let providers = state.db.get_all_providers(app_type.as_str())?;
        Ok(providers.get(&current_id).cloned())
    }

    /// 添加新供应商
    pub fn add(state: &AppState, app_type: AppType, provider: Provider) -> Result<bool, AppError> {
        // 验证配置
        Self::validate_provider_settings(&app_type, &provider)?;

        // 保存到数据库
        state.db.save_provider(app_type.as_str(), &provider)?;

        // 如果是累加模式，直接同步到 live 配置
        if app_type.is_additive_mode() {
            Self::write_live_snapshot(&app_type, &provider)?;
            return Ok(true);
        }

        // 检查是否需要设为当前供应商
        let current = state.db.get_current_provider(app_type.as_str())?;
        if current.is_none() {
            state
                .db
                .set_current_provider(app_type.as_str(), &provider.id)?;
            settings::set_current_provider(&app_type, Some(&provider.id))?;
            Self::write_live_snapshot(&app_type, &provider)?;
        }

        Ok(true)
    }

    /// 更新供应商
    pub fn update(state: &AppState, app_type: AppType, provider: Provider) -> Result<bool, AppError> {
        Self::validate_provider_settings(&app_type, &provider)?;

        // 保存到数据库
        state.db.save_provider(app_type.as_str(), &provider)?;

        // 如果是累加模式，直接更新 live 配置
        if app_type.is_additive_mode() {
            Self::write_live_snapshot(&app_type, &provider)?;
            return Ok(true);
        }

        // 如果是当前供应商，同步到 live 配置
        let current_id = Self::current(state, app_type.clone())?;
        if current_id == provider.id {
            Self::write_live_snapshot(&app_type, &provider)?;
        }

        Ok(true)
    }

    /// 删除供应商
    pub fn delete(state: &AppState, app_type: AppType, id: &str) -> Result<(), AppError> {
        // 累加模式可以随时删除
        if app_type.is_additive_mode() {
            state.db.delete_provider(app_type.as_str(), id)?;
            return Ok(());
        }

        // 检查是否为当前供应商
        let current_id = Self::current(state, app_type.clone())?;
        if current_id == id {
            return Err(AppError::Message(
                "无法删除当前正在使用的供应商".to_string(),
            ));
        }

        state.db.delete_provider(app_type.as_str(), id)
    }

    /// 切换到指定供应商
    pub fn switch(state: &AppState, app_type: AppType, id: &str) -> Result<(), AppError> {
        // 验证供应商存在
        let providers = state.db.get_all_providers(app_type.as_str())?;
        let provider = providers
            .get(id)
            .ok_or_else(|| AppError::ProviderNotFound(id.to_string()))?;

        // 更新本地设置
        settings::set_current_provider(&app_type, Some(id))?;

        // 更新数据库
        state.db.set_current_provider(app_type.as_str(), id)?;

        // 同步到 live 配置
        Self::write_live_snapshot(&app_type, provider)?;

        Ok(())
    }

    /// 按名称或 ID 查找供应商
    pub fn find(
        state: &AppState,
        app_type: AppType,
        name_or_id: &str,
    ) -> Result<Option<Provider>, AppError> {
        let providers = state.db.get_all_providers(app_type.as_str())?;

        // 先按 ID 精确匹配
        if let Some(provider) = providers.get(name_or_id) {
            return Ok(Some(provider.clone()));
        }

        // 按名称模糊匹配
        let name_lower = name_or_id.to_lowercase();
        for provider in providers.values() {
            if provider.name.to_lowercase() == name_lower {
                return Ok(Some(provider.clone()));
            }
        }

        // 按名称前缀匹配
        for provider in providers.values() {
            if provider.name.to_lowercase().starts_with(&name_lower) {
                return Ok(Some(provider.clone()));
            }
        }

        Ok(None)
    }

    /// 验证供应商配置
    fn validate_provider_settings(app_type: &AppType, provider: &Provider) -> Result<(), AppError> {
        match app_type {
            AppType::Claude => Self::validate_claude_settings(provider),
            AppType::Codex => Self::validate_codex_settings(provider),
            AppType::Gemini => Self::validate_gemini_settings(provider),
            AppType::OpenCode => Ok(()), // OpenCode 验证较宽松
        }
    }

    fn validate_claude_settings(provider: &Provider) -> Result<(), AppError> {
        let env = provider.settings_config.get("env");

        // 检查必需的认证字段
        let has_api_key = env
            .and_then(|e| e.get("ANTHROPIC_API_KEY"))
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        let has_auth_token = env
            .and_then(|e| e.get("ANTHROPIC_AUTH_TOKEN"))
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        if !has_api_key && !has_auth_token {
            return Err(AppError::InvalidInput(
                "Claude 供应商需要配置 ANTHROPIC_API_KEY 或 ANTHROPIC_AUTH_TOKEN".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_codex_settings(provider: &Provider) -> Result<(), AppError> {
        // Codex 使用 TOML 配置，检查 auth 字段
        let config = provider
            .settings_config
            .get("config")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let auth = provider
            .settings_config
            .get("auth")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // 需要在 config 或 auth 中有认证信息
        let has_auth_in_config = config.contains("api_key") || config.contains("access_token");
        let has_auth_file = !auth.is_empty();

        if !has_auth_in_config && !has_auth_file {
            return Err(AppError::InvalidInput(
                "Codex 供应商需要配置 auth 认证信息".to_string(),
            ));
        }

        Ok(())
    }

    fn validate_gemini_settings(provider: &Provider) -> Result<(), AppError> {
        // 检查 API Key
        let has_api_key = provider
            .settings_config
            .get("apiKey")
            .and_then(|v| v.as_str())
            .map(|s| !s.is_empty())
            .unwrap_or(false);

        if !has_api_key {
            return Err(AppError::InvalidInput(
                "Gemini 供应商需要配置 apiKey".to_string(),
            ));
        }

        Ok(())
    }

    /// 将供应商配置写入 live 文件
    fn write_live_snapshot(app_type: &AppType, provider: &Provider) -> Result<(), AppError> {
        match app_type {
            AppType::Claude => Self::write_claude_live(provider),
            AppType::Codex => Self::write_codex_live(provider),
            AppType::Gemini => Self::write_gemini_live(provider),
            AppType::OpenCode => Self::write_opencode_live(provider),
        }
    }

    fn write_claude_live(provider: &Provider) -> Result<(), AppError> {
        let path = get_claude_settings_path();
        write_json_file(&path, &provider.settings_config)
    }

    fn write_codex_live(provider: &Provider) -> Result<(), AppError> {
        // 写入 config.toml
        if let Some(config) = provider.settings_config.get("config").and_then(|v| v.as_str()) {
            let path = get_codex_config_path();
            write_text_file(&path, config)?;
        }

        // 写入 auth.toml
        if let Some(auth) = provider.settings_config.get("auth").and_then(|v| v.as_str()) {
            let path = get_codex_auth_path();
            write_text_file(&path, auth)?;
        }

        Ok(())
    }

    fn write_gemini_live(provider: &Provider) -> Result<(), AppError> {
        let path = get_gemini_settings_path();
        write_json_file(&path, &provider.settings_config)
    }

    fn write_opencode_live(_provider: &Provider) -> Result<(), AppError> {
        // OpenCode 使用累加模式，需要单独处理
        // TODO: 实现 OpenCode 配置写入
        Ok(())
    }

    /// 从 live 配置读取设置
    pub fn read_live_settings(app_type: AppType) -> Result<Value, AppError> {
        match app_type {
            AppType::Claude => {
                let path = get_claude_settings_path();
                if path.exists() {
                    read_json_file(&path)
                } else {
                    Ok(Value::Object(serde_json::Map::new()))
                }
            }
            AppType::Codex => {
                let config_path = get_codex_config_path();
                let auth_path = get_codex_auth_path();

                let config = if config_path.exists() {
                    std::fs::read_to_string(&config_path)
                        .map_err(|e| AppError::io(&config_path, e))?
                } else {
                    String::new()
                };

                let auth = if auth_path.exists() {
                    std::fs::read_to_string(&auth_path).map_err(|e| AppError::io(&auth_path, e))?
                } else {
                    String::new()
                };

                Ok(serde_json::json!({
                    "config": config,
                    "auth": auth
                }))
            }
            AppType::Gemini => {
                let path = get_gemini_settings_path();
                if path.exists() {
                    read_json_file(&path)
                } else {
                    Ok(Value::Object(serde_json::Map::new()))
                }
            }
            AppType::OpenCode => {
                // TODO: 实现 OpenCode 配置读取
                Ok(Value::Object(serde_json::Map::new()))
            }
        }
    }

    /// 提取凭据信息
    pub fn extract_credentials(
        provider: &Provider,
        app_type: &AppType,
    ) -> Result<(String, String), AppError> {
        match app_type {
            AppType::Claude => {
                let env = provider.settings_config.get("env");

                let api_key = env
                    .and_then(|e| e.get("ANTHROPIC_AUTH_TOKEN"))
                    .or_else(|| env.and_then(|e| e.get("ANTHROPIC_API_KEY")))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let base_url = env
                    .and_then(|e| e.get("ANTHROPIC_BASE_URL"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("https://api.anthropic.com")
                    .to_string();

                Ok((api_key, base_url))
            }
            AppType::Codex => {
                // 从 TOML 配置中提取
                let config = provider
                    .settings_config
                    .get("config")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let mut api_key = String::new();
                let mut base_url = String::new();

                for line in config.lines() {
                    let line = line.trim();
                    if line.starts_with("api_key") {
                        if let Some(value) = line.split('=').nth(1) {
                            api_key = value.trim().trim_matches('"').to_string();
                        }
                    } else if line.starts_with("base_url") {
                        if let Some(value) = line.split('=').nth(1) {
                            base_url = value.trim().trim_matches('"').to_string();
                        }
                    }
                }

                Ok((api_key, base_url))
            }
            AppType::Gemini => {
                let api_key = provider
                    .settings_config
                    .get("apiKey")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                let base_url = provider
                    .settings_config
                    .get("baseUrl")
                    .and_then(|v| v.as_str())
                    .unwrap_or("https://generativelanguage.googleapis.com")
                    .to_string();

                Ok((api_key, base_url))
            }
            AppType::OpenCode => Ok((String::new(), String::new())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_list_providers() {
        let state = AppState::memory().unwrap();

        let provider = Provider::new(
            "p1",
            "Provider 1",
            json!({
                "env": {
                    "ANTHROPIC_AUTH_TOKEN": "test-token"
                }
            }),
        );
        state.db.save_provider("claude", &provider).unwrap();

        let providers = ProviderService::list(&state, AppType::Claude).unwrap();
        assert_eq!(providers.len(), 1);
    }

    #[test]
    fn test_find_provider() {
        let state = AppState::memory().unwrap();

        let provider = Provider::new(
            "my-provider",
            "My Test Provider",
            json!({
                "env": {
                    "ANTHROPIC_AUTH_TOKEN": "test-token"
                }
            }),
        );
        state.db.save_provider("claude", &provider).unwrap();

        // 按 ID 查找
        let found = ProviderService::find(&state, AppType::Claude, "my-provider").unwrap();
        assert!(found.is_some());

        // 按名称查找
        let found = ProviderService::find(&state, AppType::Claude, "My Test Provider").unwrap();
        assert!(found.is_some());

        // 按前缀查找
        let found = ProviderService::find(&state, AppType::Claude, "My Test").unwrap();
        assert!(found.is_some());
    }
}
