//! 供应商操作命令实现

use anyhow::{bail, Result};
use ccswitch_core::{AppState, AppType, Provider, ProviderService};
use serde_json::json;
use std::io::{self, Write};
use std::time::Duration;

use crate::cli::AppTypeArg;
use crate::output::{print_error, print_info, print_success, print_warning, OutputContext, mask_api_key};

/// 切换供应商
pub fn switch(_ctx: &OutputContext, name: &str, app: AppTypeArg) -> Result<()> {
    let state = AppState::init()?;
    let app_types = app.to_app_types();

    // 如果选择了 "all"，需要明确指定应用
    if app_types.len() > 1 {
        print_error("切换供应商时请指定具体的应用类型，例如: --app claude");
        bail!("未指定应用类型");
    }

    let app_type = app_types[0];

    // 查找供应商
    let provider = ProviderService::find(&state, app_type, name)?;

    match provider {
        Some(p) => {
            ProviderService::switch(&state, app_type, &p.id)?;
            print_success(&format!(
                "已切换到供应商: {} ({})",
                p.name,
                app_type.display_name()
            ));

            // 显示供应商信息
            if let Some(url) = p.get_base_url() {
                print_info(&format!("Base URL: {}", url));
            }

            Ok(())
        }
        None => {
            print_error(&format!("未找到供应商: {}", name));

            // 显示可用的供应商
            let providers = ProviderService::list(&state, app_type)?;
            if !providers.is_empty() {
                print_info("可用的供应商:");
                for (id, p) in providers {
                    println!("  - {} ({})", p.name, id);
                }
            }

            bail!("供应商不存在");
        }
    }
}

/// 添加供应商
pub fn add(
    _ctx: &OutputContext,
    name: &str,
    app: AppTypeArg,
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    small_model: Option<String>,
    from_file: Option<String>,
    skip_test: bool,
) -> Result<()> {
    let state = AppState::init()?;
    let app_types = app.to_app_types();

    if app_types.len() > 1 {
        print_error("添加供应商时请指定具体的应用类型，例如: --app claude");
        bail!("未指定应用类型");
    }

    let app_type = app_types[0].clone();

    // 从文件导入
    if let Some(file_path) = from_file {
        return add_from_file(&state, app_type, name, &file_path);
    }

    // 根据应用类型构建配置
    let settings_config = match app_type {
        AppType::Claude => {
            let api_key_val = api_key.clone().ok_or_else(|| {
                print_error("Claude 供应商需要提供 --api-key");
                anyhow::anyhow!("缺少 API Key")
            })?;

            // 构建 env 对象
            let mut env = serde_json::Map::new();
            env.insert("ANTHROPIC_AUTH_TOKEN".to_string(), json!(api_key_val));
            env.insert(
                "ANTHROPIC_BASE_URL".to_string(),
                json!(base_url.clone().unwrap_or_else(|| "https://api.anthropic.com".to_string())),
            );

            // 添加模型配置
            if let Some(m) = &model {
                env.insert("ANTHROPIC_MODEL".to_string(), json!(m));
            }
            if let Some(sm) = &small_model {
                env.insert("ANTHROPIC_SMALL_FAST_MODEL".to_string(), json!(sm));
            }

            json!({ "env": env })
        }
        AppType::Codex => {
            let api_key_val = api_key.clone().ok_or_else(|| {
                print_error("Codex 供应商需要提供 --api-key");
                anyhow::anyhow!("缺少 API Key")
            })?;

            let base_url_val = base_url.clone().unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            let model_name = model.clone().unwrap_or_else(|| "gpt-4".to_string());

            let config = format!(
                r#"model_provider = "openai"
model = "{}"

[model_providers.openai]
name = "OpenAI"
base_url = "{}"
wire_api = "responses"
"#,
                model_name, base_url_val
            );

            let auth = format!(
                r#"[openai]
api_key = "{}"
"#,
                api_key_val
            );

            json!({
                "config": config,
                "auth": auth
            })
        }
        AppType::Gemini => {
            let api_key_val = api_key.clone().ok_or_else(|| {
                print_error("Gemini 供应商需要提供 --api-key");
                anyhow::anyhow!("缺少 API Key")
            })?;

            let mut config = serde_json::Map::new();
            config.insert("apiKey".to_string(), json!(api_key_val));
            config.insert(
                "baseUrl".to_string(),
                json!(base_url.clone().unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string())),
            );

            // 添加模型配置
            if let Some(m) = &model {
                config.insert("model".to_string(), json!(m));
            }

            json!(config)
        }
        AppType::OpenCode | AppType::OpenClaw => {
            print_warning("该应用供应商添加功能尚未完全实现");
            json!({})
        }
    };

    // 添加前测试 API Key
    if !skip_test && api_key.is_some() {
        print_info("正在测试 API Key 有效性...");

        let test_result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(test_api_key(
                &app_type,
                api_key.as_ref().unwrap(),
                base_url.as_deref(),
                model.as_deref(),
                30,
            ));

        match test_result {
            Ok(true) => {
                print_success("API Key 测试通过!");
            }
            Ok(false) => {
                print_warning("API Key 测试未返回预期结果，但仍将继续添加");
            }
            Err(e) => {
                print_error(&format!("API Key 测试失败: {}", e));
                print!("是否仍要添加此供应商? [y/N] ");
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !input.trim().eq_ignore_ascii_case("y") {
                    print_info("已取消");
                    return Ok(());
                }
            }
        }
    }

    // 生成 ID
    let id = format!(
        "{}-{}",
        ccswitch_core::config::sanitize_name(name),
        chrono::Utc::now().timestamp()
    );

    let provider = Provider::new(id, name, settings_config);

    ProviderService::add(&state, app_type.clone(), provider)?;
    print_success(&format!(
        "已添加供应商: {} ({})",
        name,
        app_type.display_name()
    ));

    Ok(())
}

/// 从文件添加供应商
fn add_from_file(
    state: &AppState,
    app_type: AppType,
    name: &str,
    file_path: &str,
) -> Result<()> {
    let content = std::fs::read_to_string(file_path)?;

    let settings_config: serde_json::Value = if file_path.ends_with(".json") {
        serde_json::from_str(&content)?
    } else if file_path.ends_with(".yaml") || file_path.ends_with(".yml") {
        serde_yaml::from_str(&content)?
    } else if file_path.ends_with(".toml") {
        // 对于 TOML，包装成 JSON
        json!({ "config": content })
    } else {
        // 尝试自动检测
        serde_json::from_str(&content)
            .or_else(|_| serde_yaml::from_str(&content).map_err(anyhow::Error::from))?
    };

    let id = format!(
        "{}-{}",
        ccswitch_core::config::sanitize_name(name),
        chrono::Utc::now().timestamp()
    );

    let provider = Provider::new(id, name, settings_config);

    ProviderService::add(state, app_type, provider)?;
    print_success(&format!(
        "已从文件导入供应商: {} ({})",
        name,
        app_type.display_name()
    ));

    Ok(())
}

/// 删除供应商
pub fn remove(_ctx: &OutputContext, name: &str, app: AppTypeArg, yes: bool) -> Result<()> {
    let state = AppState::init()?;
    let app_types = app.to_app_types();

    if app_types.len() > 1 {
        print_error("删除供应商时请指定具体的应用类型，例如: --app claude");
        bail!("未指定应用类型");
    }

    let app_type = app_types[0];

    // 查找供应商
    let provider = ProviderService::find(&state, app_type, name)?;

    match provider {
        Some(p) => {
            // 确认删除
            if !yes {
                print!("确定要删除供应商 \"{}\"? [y/N] ", p.name);
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !input.trim().eq_ignore_ascii_case("y") {
                    print_info("已取消");
                    return Ok(());
                }
            }

            match ProviderService::delete(&state, app_type, &p.id) {
                Ok(_) => {
                    print_success(&format!(
                        "已删除供应商: {} ({})",
                        p.name,
                        app_type.display_name()
                    ));
                }
                Err(e) => {
                    print_error(&format!("删除失败: {}", e));
                    bail!("删除失败");
                }
            }

            Ok(())
        }
        None => {
            print_error(&format!("未找到供应商: {}", name));
            bail!("供应商不存在");
        }
    }
}

/// 更新配置
pub fn update(_ctx: &OutputContext, app: AppTypeArg) -> Result<()> {
    let app_types = app.to_app_types();

    for app_type in app_types {
        print_info(&format!("正在更新 {} 配置...", app_type.display_name()));
        // TODO: 实现更新逻辑（同步订阅等）
    }

    print_warning("更新功能尚未完全实现");

    Ok(())
}

/// 编辑供应商
pub fn edit(
    _ctx: &OutputContext,
    name: &str,
    app: AppTypeArg,
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    small_model: Option<String>,
    new_name: Option<String>,
) -> Result<()> {
    let state = AppState::init()?;
    let app_types = app.to_app_types();

    if app_types.len() > 1 {
        print_error("编辑供应商时请指定具体的应用类型，例如: --app claude");
        bail!("未指定应用类型");
    }

    let app_type = app_types[0].clone();

    // 查找供应商
    let provider = ProviderService::find(&state, app_type.clone(), name)?;

    match provider {
        Some(mut p) => {
            let old_name = p.name.clone();

            // 更新名称
            if let Some(ref n) = new_name {
                p.name = n.clone();
            }

            // 根据应用类型更新配置
            match app_type {
                AppType::Claude => {
                    if let Some(env) = p.settings_config.get_mut("env") {
                        if let Some(obj) = env.as_object_mut() {
                            if let Some(key) = &api_key {
                                obj.insert("ANTHROPIC_AUTH_TOKEN".to_string(), json!(key));
                            }
                            if let Some(url) = &base_url {
                                obj.insert("ANTHROPIC_BASE_URL".to_string(), json!(url));
                            }
                            if let Some(m) = &model {
                                obj.insert("ANTHROPIC_MODEL".to_string(), json!(m));
                            }
                            if let Some(sm) = &small_model {
                                obj.insert("ANTHROPIC_SMALL_FAST_MODEL".to_string(), json!(sm));
                            }
                        }
                    }
                }
                AppType::Codex => {
                    // Codex 使用 TOML 格式，需要重建配置
                    if api_key.is_some() || base_url.is_some() || model.is_some() {
                        let current_config = p.settings_config.get("config")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        // 提取当前值
                        let mut current_base_url = "https://api.openai.com/v1".to_string();
                        let mut current_model = "gpt-4".to_string();
                        for line in current_config.lines() {
                            if line.trim().starts_with("base_url") {
                                if let Some(v) = line.split('=').nth(1) {
                                    current_base_url = v.trim().trim_matches('"').to_string();
                                }
                            }
                            if line.trim().starts_with("model =") {
                                if let Some(v) = line.split('=').nth(1) {
                                    current_model = v.trim().trim_matches('"').to_string();
                                }
                            }
                        }

                        let base_url_updated = base_url.is_some();
                        let model_updated = model.is_some();
                        let new_base_url = base_url.unwrap_or(current_base_url);
                        let new_model = model.unwrap_or(current_model);

                        let config = format!(
                            r#"model_provider = "openai"
model = "{}"

[model_providers.openai]
name = "OpenAI"
base_url = "{}"
wire_api = "responses"
"#,
                            new_model, new_base_url
                        );

                        if let Some(obj) = p.settings_config.as_object_mut() {
                            obj.insert("config".to_string(), json!(config));
                        }

                        if let Some(key) = &api_key {
                            let auth = format!(
                                r#"[openai]
api_key = "{}"
"#,
                                key
                            );
                            if let Some(obj) = p.settings_config.as_object_mut() {
                                obj.insert("auth".to_string(), json!(auth));
                            }
                        }

                        // 保存更新
                        ProviderService::update(&state, app_type.clone(), p)?;
                        print_success(&format!(
                            "已更新供应商: {} ({})",
                            new_name.as_ref().map(|n| n.as_str()).unwrap_or(&old_name),
                            app_type.display_name()
                        ));

                        // 显示更新的字段
                        if api_key.is_some() {
                            print_info("  - API Key 已更新");
                        }
                        if base_url_updated {
                            print_info("  - Base URL 已更新");
                        }
                        if model_updated {
                            print_info("  - 模型 已更新");
                        }
                        
                        return Ok(());
                    }
                }
                AppType::Gemini => {
                    if let Some(obj) = p.settings_config.as_object_mut() {
                        if let Some(key) = &api_key {
                            obj.insert("apiKey".to_string(), json!(key));
                        }
                        if let Some(url) = &base_url {
                            obj.insert("baseUrl".to_string(), json!(url));
                        }
                        if let Some(m) = &model {
                            obj.insert("model".to_string(), json!(m));
                        }
                    }
                }
                AppType::OpenCode | AppType::OpenClaw => {
                    print_warning("该应用供应商编辑功能尚未完全实现");
                }
            }

            // 保存更新
            ProviderService::update(&state, app_type.clone(), p)?;
            print_success(&format!(
                "已更新供应商: {} ({})",
                new_name.as_ref().map(|n| n.as_str()).unwrap_or(&old_name),
                app_type.display_name()
            ));

            // 显示更新的字段
            if api_key.is_some() {
                print_info("  - API Key 已更新");
            }
            if base_url.is_some() {
                print_info("  - Base URL 已更新");
            }
            if model.is_some() {
                print_info("  - 模型 已更新");
            }
            if small_model.is_some() {
                print_info("  - 小模型 已更新");
            }

            Ok(())
        }
        None => {
            print_error(&format!("未找到供应商: {}", name));

            // 显示可用的供应商
            let providers = ProviderService::list(&state, app_type)?;
            if !providers.is_empty() {
                print_info("可用的供应商:");
                for (id, p) in providers {
                    println!("  - {} ({})", p.name, id);
                }
            }

            bail!("供应商不存在");
        }
    }
}

/// 测试 API 命令
pub async fn test_api(
    _ctx: &OutputContext,
    name: Option<String>,
    app: AppTypeArg,
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    timeout: u64,
) -> Result<()> {
    let app_types = app.to_app_types();

    if app_types.len() > 1 {
        print_error("测试 API 时请指定具体的应用类型，例如: --app claude");
        bail!("未指定应用类型");
    }

    let app_type = app_types[0].clone();

    // 获取测试参数
    let (test_key, test_url, test_model) = if let Some(key) = api_key {
        // 直接使用传入的参数
        let url = base_url.unwrap_or_else(|| get_default_base_url(&app_type));
        let model = model.unwrap_or_else(|| get_default_model(&app_type));
        (key, url, model)
    } else if let Some(provider_name) = name {
        // 从供应商获取
        let state = AppState::init()?;
        let provider = ProviderService::find(&state, app_type.clone(), &provider_name)?;

        match provider {
            Some(p) => {
                let (key, url) = ProviderService::extract_credentials(&p, &app_type)?;
                let model = model.unwrap_or_else(|| p.get_model().unwrap_or_else(|| get_default_model(&app_type)));

                print_info(&format!("测试供应商: {} ({})", p.name, mask_api_key(&key)));

                (key, url, model)
            }
            None => {
                print_error(&format!("未找到供应商: {}", provider_name));
                bail!("供应商不存在");
            }
        }
    } else {
        print_error("请指定供应商名称或 --api-key 参数");
        bail!("缺少必需参数");
    };

    if test_key.is_empty() {
        print_error("未找到有效的 API Key");
        bail!("API Key 为空");
    }

    println!("\n🧪 API 测试\n");
    println!("  应用类型: {}", app_type.display_name());
    println!("  Base URL: {}", test_url);
    println!("  模型: {}", test_model);
    println!("  API Key: {}", mask_api_key(&test_key));
    println!();

    print_info("正在测试...");

    match test_api_key(&app_type, &test_key, Some(&test_url), Some(&test_model), timeout).await {
        Ok(true) => {
            print_success("✓ API 测试通过！");
            Ok(())
        }
        Ok(false) => {
            print_warning("⚠ API 测试未返回预期结果");
            Ok(())
        }
        Err(e) => {
            print_error(&format!("✗ API 测试失败: {}", e));
            bail!("API 测试失败");
        }
    }
}

/// 测试 API Key 有效性
async fn test_api_key(
    app_type: &AppType,
    api_key: &str,
    base_url: Option<&str>,
    model: Option<&str>,
    timeout_secs: u64,
) -> Result<bool> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout_secs))
        .build()?;

    match app_type {
        AppType::Claude => {
            let url = format!(
                "{}/v1/messages",
                base_url.unwrap_or("https://api.anthropic.com")
            );
            let model_name = model.unwrap_or("claude-sonnet-4-20250514");

            let response: reqwest::Response = client
                .post(&url)
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01")
                .header("content-type", "application/json")
                .json(&json!({
                    "model": model_name,
                    "max_tokens": 10,
                    "messages": [{"role": "user", "content": "hi"}]
                }))
                .send()
                .await?;

            let status = response.status();
            if status.is_success() {
                Ok(true)
            } else if status.as_u16() == 401 {
                bail!("API Key 无效或已过期");
            } else if status.as_u16() == 403 {
                bail!("权限不足");
            } else if status.as_u16() == 429 {
                // Rate limit 说明 key 是有效的
                Ok(true)
            } else {
                let body: String = response.text().await.unwrap_or_default();
                bail!("HTTP {}: {}", status, body);
            }
        }
        AppType::Codex | AppType::OpenCode | AppType::OpenClaw => {
            // OpenAI 兼容 API
            let url = format!(
                "{}/chat/completions",
                base_url.unwrap_or("https://api.openai.com/v1")
            );
            let model_name = model.unwrap_or("gpt-4");

            let response: reqwest::Response = client
                .post(&url)
                .header("Authorization", format!("Bearer {}", api_key))
                .header("content-type", "application/json")
                .json(&json!({
                    "model": model_name,
                    "max_tokens": 10,
                    "messages": [{"role": "user", "content": "hi"}]
                }))
                .send()
                .await?;

            let status = response.status();
            if status.is_success() {
                Ok(true)
            } else if status.as_u16() == 401 {
                bail!("API Key 无效或已过期");
            } else if status.as_u16() == 429 {
                Ok(true)
            } else {
                let body: String = response.text().await.unwrap_or_default();
                bail!("HTTP {}: {}", status, body);
            }
        }
        AppType::Gemini => {
            let base = base_url.unwrap_or("https://generativelanguage.googleapis.com");
            let model_name = model.unwrap_or("gemini-1.5-flash");
            let url = format!(
                "{}/v1beta/models/{}:generateContent?key={}",
                base, model_name, api_key
            );

            let response: reqwest::Response = client
                .post(&url)
                .header("content-type", "application/json")
                .json(&json!({
                    "contents": [{"parts": [{"text": "hi"}]}]
                }))
                .send()
                .await?;

            let status = response.status();
            if status.is_success() {
                Ok(true)
            } else if status.as_u16() == 400 {
                let body: String = response.text().await.unwrap_or_default();
                if body.contains("API_KEY_INVALID") {
                    bail!("API Key 无效");
                }
                bail!("请求错误: {}", body);
            } else if status.as_u16() == 429 {
                Ok(true)
            } else {
                let body: String = response.text().await.unwrap_or_default();
                bail!("HTTP {}: {}", status, body);
            }
        }
    }
}

/// 获取默认 Base URL
fn get_default_base_url(app_type: &AppType) -> String {
    match app_type {
        AppType::Claude => "https://api.anthropic.com".to_string(),
        AppType::Codex | AppType::OpenCode | AppType::OpenClaw => "https://api.openai.com/v1".to_string(),
        AppType::Gemini => "https://generativelanguage.googleapis.com".to_string(),
    }
}

/// 获取默认模型
fn get_default_model(app_type: &AppType) -> String {
    match app_type {
        AppType::Claude => "claude-sonnet-4-20250514".to_string(),
        AppType::Codex | AppType::OpenCode | AppType::OpenClaw => "gpt-4".to_string(),
        AppType::Gemini => "gemini-1.5-flash".to_string(),
    }
}
