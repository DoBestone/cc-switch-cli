//! 供应商操作命令实现

use anyhow::{bail, Result};
use ccswitch_core::{AppState, AppType, Provider, ProviderService};
use serde_json::json;
use std::io::{self, Write};

use crate::cli::AppTypeArg;
use crate::output::{print_error, print_info, print_success, print_warning, OutputContext};

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
) -> Result<()> {
    let state = AppState::init()?;
    let app_types = app.to_app_types();

    if app_types.len() > 1 {
        print_error("添加供应商时请指定具体的应用类型，例如: --app claude");
        bail!("未指定应用类型");
    }

    let app_type = app_types[0];

    // 从文件导入
    if let Some(file_path) = from_file {
        return add_from_file(&state, app_type, name, &file_path);
    }

    // 根据应用类型构建配置
    let settings_config = match app_type {
        AppType::Claude => {
            let api_key = api_key.ok_or_else(|| {
                print_error("Claude 供应商需要提供 --api-key");
                anyhow::anyhow!("缺少 API Key")
            })?;

            // 构建 env 对象
            let mut env = serde_json::Map::new();
            env.insert("ANTHROPIC_AUTH_TOKEN".to_string(), json!(api_key));
            env.insert(
                "ANTHROPIC_BASE_URL".to_string(),
                json!(base_url.unwrap_or_else(|| "https://api.anthropic.com".to_string())),
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
            let api_key = api_key.ok_or_else(|| {
                print_error("Codex 供应商需要提供 --api-key");
                anyhow::anyhow!("缺少 API Key")
            })?;

            let base_url = base_url.unwrap_or_else(|| "https://api.openai.com/v1".to_string());
            let model_name = model.unwrap_or_else(|| "gpt-4".to_string());

            let config = format!(
                r#"model_provider = "openai"
model = "{}"

[model_providers.openai]
name = "OpenAI"
base_url = "{}"
wire_api = "responses"
"#,
                model_name, base_url
            );

            let auth = format!(
                r#"[openai]
api_key = "{}"
"#,
                api_key
            );

            json!({
                "config": config,
                "auth": auth
            })
        }
        AppType::Gemini => {
            let api_key = api_key.ok_or_else(|| {
                print_error("Gemini 供应商需要提供 --api-key");
                anyhow::anyhow!("缺少 API Key")
            })?;

            let mut config = serde_json::Map::new();
            config.insert("apiKey".to_string(), json!(api_key));
            config.insert(
                "baseUrl".to_string(),
                json!(base_url.unwrap_or_else(|| "https://generativelanguage.googleapis.com".to_string())),
            );

            // 添加模型配置
            if let Some(m) = &model {
                config.insert("model".to_string(), json!(m));
            }

            json!(config)
        }
        AppType::OpenCode => {
            print_warning("OpenCode 供应商添加功能尚未完全实现");
            json!({})
        }
    };

    // 生成 ID
    let id = format!(
        "{}-{}",
        ccswitch_core::config::sanitize_name(name),
        chrono::Utc::now().timestamp()
    );

    let provider = Provider::new(id, name, settings_config);

    ProviderService::add(&state, app_type, provider)?;
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
