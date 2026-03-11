//! OpenClaw 配置管理命令
//!
//! 提供管理 OpenClaw 特有配置的命令，包括供应商、默认模型、环境变量等。

use anyhow::{anyhow, Result};
use serde_json::json;

use ccswitch_core::{
    get_openclaw_config_path, get_openclaw_providers_path,
    read_openclaw_config, write_openclaw_config,
    get_openclaw_providers, remove_openclaw_provider,
    get_openclaw_typed_providers, set_openclaw_typed_provider,
    get_openclaw_default_model, set_openclaw_default_model,
    get_agents_defaults, set_agents_defaults,
    get_openclaw_env_config, set_openclaw_env_config,
    get_openclaw_tools_config, set_openclaw_tools_config,
    get_openclaw_model_catalog,
    add_openclaw_model_to_catalog, remove_openclaw_model_from_catalog,
    scan_openclaw_config_health,
    OpenClawProviderConfig, OpenClawModelEntry, OpenClawDefaultModel, OpenClawModelCatalogEntry,
};

use crate::output::{print_success, print_info, print_warning};
use crate::cli::OutputFormat;

/// 列出 OpenClaw 供应商
pub fn list_providers(ctx: &crate::output::OutputContext, detail: bool) -> Result<()> {
    let providers = get_openclaw_typed_providers()?;

    if providers.is_empty() {
        print_success("没有配置 OpenClaw 供应商");
        return Ok(());
    }

    if detail {
        print_json_output(ctx, &providers)?;
    } else {
        let table_data: Vec<Vec<String>> = providers
            .iter()
            .map(|(id, config)| {
                let model_count = config.models.len();
                vec![
                    id.clone(),
                    config.base_url.as_deref().unwrap_or("-").to_string(),
                    config.api.as_deref().unwrap_or("-").to_string(),
                    model_count.to_string(),
                ]
            })
            .collect();

        print_table_output(ctx, &["ID", "Base URL", "API Type", "Models"], table_data);
    }

    Ok(())
}

/// 添加 OpenClaw 供应商
pub fn add_provider(
    _ctx: &crate::output::OutputContext,
    id: &str,
    base_url: Option<String>,
    api_key: Option<String>,
    api: Option<String>,
    models: Vec<String>,
) -> Result<()> {
    // 检查是否已存在
    let existing = get_openclaw_providers()?;
    if existing.get(id).is_some() {
        return Err(anyhow!("供应商 '{}' 已存在，请使用 update 命令修改", id));
    }

    // 构建模型列表
    let model_entries: Vec<OpenClawModelEntry> = models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let parts: Vec<&str> = m.splitn(2, ':').collect();
            let model_id = parts[0].to_string();
            let model_name = parts.get(1).map(|s| s.to_string());
            let final_id = if model_name.is_some() {
                model_id.clone()
            } else if i == 0 {
                format!("default-{}", m)
            } else {
                format!("model-{}", i)
            };
            OpenClawModelEntry {
                id: final_id,
                name: model_name.or(Some(model_id)),
                alias: None,
                context_window: None,
                cost: None,
                extra: Default::default(),
            }
        })
        .collect();

    let config = OpenClawProviderConfig {
        base_url,
        api_key,
        api,
        models: model_entries,
        headers: Default::default(),
        extra: Default::default(),
    };

    set_openclaw_typed_provider(id, &config)?;
    print_success(&format!("已添加供应商 '{}'", id));

    Ok(())
}

/// 更新 OpenClaw 供应商
pub fn update_provider(
    _ctx: &crate::output::OutputContext,
    id: &str,
    base_url: Option<String>,
    api_key: Option<String>,
    api: Option<String>,
    add_models: Vec<String>,
    remove_models: Vec<String>,
) -> Result<()> {
    let mut providers = get_openclaw_typed_providers()?;
    let config = providers
        .get_mut(id)
        .ok_or_else(|| anyhow!("供应商 '{}' 不存在", id))?;

    if let Some(url) = base_url {
        config.base_url = Some(url);
    }
    if let Some(key) = api_key {
        config.api_key = Some(key);
    }
    if let Some(a) = api {
        config.api = Some(a);
    }

    // 添加模型
    for m in add_models {
        let parts: Vec<&str> = m.splitn(2, ':').collect();
        let model_id = parts[0].to_string();
        let model_name = parts.get(1).map(|s| s.to_string());
        config.models.push(OpenClawModelEntry {
            id: model_id.clone(),
            name: model_name.or(Some(model_id)),
            alias: None,
            context_window: None,
            cost: None,
            extra: Default::default(),
        });
    }

    // 移除模型
    if !remove_models.is_empty() {
        config.models.retain(|m| !remove_models.contains(&m.id));
    }

    set_openclaw_typed_provider(id, config)?;
    print_success(&format!("已更新供应商 '{}'", id));

    Ok(())
}

/// 删除 OpenClaw 供应商
pub fn remove_provider(_ctx: &crate::output::OutputContext, id: &str, yes: bool) -> Result<()> {
    let providers = get_openclaw_providers()?;
    if providers.get(id).is_none() {
        return Err(anyhow!("供应商 '{}' 不存在", id));
    }

    if !yes {
        confirm_action(&format!("确定要删除供应商 '{}' 吗?", id))?;
    }

    remove_openclaw_provider(id)?;
    print_success(&format!("已删除供应商 '{}'", id));

    Ok(())
}

/// 显示供应商详情
pub fn show_provider(ctx: &crate::output::OutputContext, id: &str) -> Result<()> {
    let providers = get_openclaw_typed_providers()?;
    let config = providers
        .get(id)
        .ok_or_else(|| anyhow!("供应商 '{}' 不存在", id))?;

    print_json_output(ctx, &json!({
        "id": id,
        "config": config
    }))?;

    Ok(())
}

/// 获取默认模型配置
pub fn get_default_model_cmd(ctx: &crate::output::OutputContext) -> Result<()> {
    let default_model = get_openclaw_default_model()?;

    if let Some(model) = default_model {
        print_json_output(ctx, &model)?;
    } else {
        print_success("未配置默认模型");
    }

    Ok(())
}

/// 设置默认模型配置
pub fn set_default_model_cmd(
    _ctx: &crate::output::OutputContext,
    primary: Option<String>,
    fallbacks: Vec<String>,
) -> Result<()> {
    let primary_model = primary.ok_or_else(|| anyhow!("必须指定 --primary 参数"))?;

    let current = OpenClawDefaultModel {
        primary: primary_model,
        fallbacks,
        extra: Default::default(),
    };

    set_openclaw_default_model(&current)?;
    print_success("已更新默认模型配置");

    Ok(())
}

/// 获取 Agents 默认配置
pub fn get_agents_config(ctx: &crate::output::OutputContext) -> Result<()> {
    let defaults = get_agents_defaults()?;
    print_json_output(ctx, &defaults)?;
    Ok(())
}

/// 设置 Agents 默认配置
pub fn set_agents_config(
    _ctx: &crate::output::OutputContext,
    model: Option<String>,
    timeout_seconds: Option<u64>,
) -> Result<()> {
    let current = get_agents_defaults()?;

    let mut new_defaults = current.unwrap_or_else(|| ccswitch_core::OpenClawAgentsDefaults {
        model: None,
        models: None,
        timeout_seconds: None,
        extra: Default::default(),
    });

    if let Some(m) = model {
        new_defaults.model = Some(OpenClawDefaultModel::new(m));
    }
    if let Some(t) = timeout_seconds {
        new_defaults.timeout_seconds = Some(t);
    }

    set_agents_defaults(&new_defaults)?;
    print_success("已更新 Agents 默认配置");

    Ok(())
}

/// 获取环境变量配置
pub fn get_env_config_cmd(ctx: &crate::output::OutputContext) -> Result<()> {
    let env = get_openclaw_env_config()?;
    print_json_output(ctx, &env)?;
    Ok(())
}

/// 设置环境变量配置
pub fn set_env_config_cmd(
    _ctx: &crate::output::OutputContext,
    key: &str,
    value: &str,
) -> Result<()> {
    let mut env = get_openclaw_env_config()?;
    env.vars.insert(key.to_string(), json!(value));
    set_openclaw_env_config(&env)?;
    print_success(&format!("已设置环境变量 '{}'", key));

    Ok(())
}

/// 删除环境变量
pub fn remove_env_config(_ctx: &crate::output::OutputContext, key: &str) -> Result<()> {
    let mut env = get_openclaw_env_config()?;
    if env.vars.remove(key).is_none() {
        return Err(anyhow!("环境变量 '{}' 不存在", key));
    }
    set_openclaw_env_config(&env)?;
    print_success(&format!("已删除环境变量 '{}'", key));

    Ok(())
}

/// 获取工具配置
pub fn get_tools_config_cmd(ctx: &crate::output::OutputContext) -> Result<()> {
    let tools = get_openclaw_tools_config()?;
    print_json_output(ctx, &tools)?;
    Ok(())
}

/// 设置工具配置
pub fn set_tools_config_cmd(
    _ctx: &crate::output::OutputContext,
    profile: Option<String>,
    add_allow: Vec<String>,
    remove_allow: Vec<String>,
    add_deny: Vec<String>,
    remove_deny: Vec<String>,
) -> Result<()> {
    let mut tools = get_openclaw_tools_config()?;

    if let Some(p) = profile {
        tools.profile = Some(p);
    }

    for t in add_allow {
        if !tools.allow.contains(&t) {
            tools.allow.push(t);
        }
    }
    for t in remove_allow {
        tools.allow.retain(|x| x != &t);
    }
    for t in add_deny {
        if !tools.deny.contains(&t) {
            tools.deny.push(t);
        }
    }
    for t in remove_deny {
        tools.deny.retain(|x| x != &t);
    }

    set_openclaw_tools_config(&tools)?;
    print_success("已更新工具配置");

    Ok(())
}

/// 获取模型目录配置
pub fn get_model_catalog_cmd(ctx: &crate::output::OutputContext) -> Result<()> {
    let catalog = get_openclaw_model_catalog()?;

    if let Some(c) = catalog {
        if c.is_empty() {
            print_success("模型目录为空");
        } else {
            print_json_output(ctx, &c)?;
        }
    } else {
        print_success("未配置模型目录");
    }

    Ok(())
}

/// 添加模型到目录
pub fn add_model_to_catalog_cmd(
    _ctx: &crate::output::OutputContext,
    model_id: &str,
    alias: Option<String>,
) -> Result<()> {
    let entry = OpenClawModelCatalogEntry {
        alias,
        extra: Default::default(),
    };

    add_openclaw_model_to_catalog(model_id, &entry)?;
    print_success(&format!("已添加模型 '{}' 到目录", model_id));

    Ok(())
}

/// 从目录移除模型
pub fn remove_model_from_catalog_cmd(
    _ctx: &crate::output::OutputContext,
    model_id: &str,
) -> Result<()> {
    let catalog = get_openclaw_model_catalog()?.unwrap_or_default();

    if !catalog.contains_key(model_id) {
        return Err(anyhow!("模型 '{}' 不在目录中", model_id));
    }

    remove_openclaw_model_from_catalog(model_id)?;
    print_success(&format!("已从目录移除模型 '{}'", model_id));

    Ok(())
}

/// 健康检查
pub fn health_check(_ctx: &crate::output::OutputContext, fix: bool) -> Result<()> {
    let warnings = scan_openclaw_config_health()?;

    if warnings.is_empty() {
        print_success("OpenClaw 配置健康，没有发现问题");
        return Ok(());
    }

    print_info(&format!("发现 {} 个警告:", warnings.len()));

    for warning in &warnings {
        let path_info = warning.path.as_deref().unwrap_or("-");
        print_warning(&format!("- [{}] {} (路径: {})", warning.code, warning.message, path_info));
    }

    if fix {
        print_info("正在尝试修复...");
        // 健康检查的修复逻辑会在 scan_openclaw_config_health 中自动处理
        // 这里我们重新检查
        let new_warnings = scan_openclaw_config_health()?;
        let fixed_count = warnings.len() - new_warnings.len();
        print_success(&format!("已修复 {} 个问题", fixed_count));
    }

    Ok(())
}

/// 显示配置文件路径
pub fn show_config_path(ctx: &crate::output::OutputContext) -> Result<()> {
    let main_path = get_openclaw_config_path();
    let providers_path = get_openclaw_providers_path();

    match ctx.format {
        OutputFormat::Json => {
            print_json_output(ctx, &json!({
                "main_config": main_path.display().to_string(),
                "providers_config": providers_path.display().to_string()
            }))?;
        }
        _ => {
            print_info(&format!("主配置文件: {}", main_path.display()));
            print_info(&format!("供应商配置: {}", providers_path.display()));
        }
    }

    Ok(())
}

/// 导出完整配置
pub fn export_config(ctx: &crate::output::OutputContext) -> Result<()> {
    let config = read_openclaw_config()?;
    print_json_output(ctx, &config)?;
    Ok(())
}

/// 导入完整配置
pub fn import_config(_ctx: &crate::output::OutputContext, file: &str) -> Result<()> {
    let content = std::fs::read_to_string(file)?;
    let config: serde_json::Value = json5::from_str(&content)
        .map_err(|e| anyhow!("解析配置文件失败: {}", e))?;

    write_openclaw_config(&config)?;
    print_success("配置导入成功");

    Ok(())
}

// ============================================================================
// Helper Functions
// ============================================================================

/// 打印 JSON 输出
fn print_json_output<T: serde::Serialize>(ctx: &crate::output::OutputContext, data: &T) -> Result<()> {
    match ctx.format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(data)?;
            println!("{}", json);
        }
        OutputFormat::Yaml => {
            let yaml = serde_yaml::to_string(data)?;
            println!("{}", yaml);
        }
        OutputFormat::Table => {
            let json = serde_json::to_string_pretty(data)?;
            println!("{}", json);
        }
    }
    Ok(())
}

/// 打印表格输出
fn print_table_output(_ctx: &crate::output::OutputContext, headers: &[&str], rows: Vec<Vec<String>>) {
    // 构建简单表格
    if rows.is_empty() {
        println!("没有数据");
        return;
    }

    let header_str = headers.join(" | ");
    println!("{}", header_str);
    println!("{}", "-".repeat(header_str.len()));

    for row in rows {
        let row_str = row.join(" | ");
        println!("{}", row_str);
    }
}

/// 确认操作
fn confirm_action(message: &str) -> Result<()> {
    use std::io::{self, BufRead, Write};

    print!("{} [y/N] ", message);
    io::stdout().flush()?;

    let stdin = io::stdin();
    let line = stdin.lock().lines().next()
        .ok_or_else(|| anyhow!("无法读取输入"))??;

    if line.to_lowercase() != "y" {
        return Err(anyhow!("操作已取消"));
    }

    Ok(())
}