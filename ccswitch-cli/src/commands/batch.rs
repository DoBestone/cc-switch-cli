//! 批量操作模块
//!
//! 提供批量管理供应商的功能，提高操作效率。

use anyhow::{bail, Result};
use colored::Colorize;
use std::collections::HashMap;

use ccswitch_core::{AppState, AppType, Provider};

use crate::cli::AppTypeArg;
use crate::output::OutputContext;

/// 批量切换所有应用到指定供应商
pub fn batch_switch(_ctx: &OutputContext, name: &str) -> Result<()> {
    let state = AppState::init()?;

    println!();
    println!("{}", "═══ 批量切换供应商 ═══".cyan().bold());
    println!();
    println!("将切换所有应用到供应商: {}", name.green().bold());
    println!();

    let mut success_count = 0;
    let mut failed_apps = Vec::new();

    for app_type in AppType::all() {
        let display_name = app_type.display_name();
        print!("  {} {} ... ", "→".blue(), display_name);

        match ccswitch_core::ProviderService::switch(&state, *app_type, name) {
            Ok(_) => {
                println!("{}", "✓".green());
                success_count += 1;
            }
            Err(e) => {
                println!("{} {}", "✗".red(), format!("({})", e).dimmed());
                failed_apps.push(display_name);
            }
        }
    }

    println!();
    if failed_apps.is_empty() {
        println!("{}", format!("✓ 成功切换 {} 个应用", success_count).green().bold());
    } else {
        println!("{}", format!("⚠ 成功切换 {} 个应用", success_count).yellow());
        println!("{}", format!("  失败的应用: {}", failed_apps.join(", ")).red());
    }
    println!();

    Ok(())
}

/// 批量测试所有供应商的 API
pub async fn batch_test(
    _ctx: &OutputContext,
    app_type: AppTypeArg,
    timeout: u64,
    verbose: bool,
) -> Result<()> {
    let state = AppState::init()?;

    println!();
    println!("{}", "═══ 批量测试供应商 API ═══".cyan().bold());
    println!();

    let mut total_tested = 0;
    let mut total_success = 0;
    let mut results = Vec::new();

    for app in app_type.to_app_types() {
        let providers = ccswitch_core::ProviderService::list(&state, app)?;

        if providers.is_empty() {
            println!("{}: {}", app.display_name().yellow(), "无供应商".dimmed());
            continue;
        }

        println!("{}: 测试 {} 个供应商", app.display_name().cyan().bold(), providers.len());

        for (id, provider) in providers.iter() {
            print!("  {} {} ... ", "→".blue(), provider.name);
            total_tested += 1;

            let result = test_provider_api(id, &provider, app, timeout).await;

            match result {
                Ok(latency) => {
                    let latency_str = format!("{}ms", latency);
                    let latency_colored = if latency < 200 {
                        latency_str.green()
                    } else if latency < 500 {
                        latency_str.yellow()
                    } else {
                        latency_str.red()
                    };

                    println!("{} ({})", "✓".green(), latency_colored);
                    total_success += 1;
                    results.push((app.display_name(), provider.name.clone(), true, Some(latency)));
                }
                Err(e) => {
                    println!("{} {}", "✗".red(), if verbose {
                        format!("({})", e)
                    } else {
                        "(失败)".to_string()
                    }.dimmed());
                    results.push((app.display_name(), provider.name.clone(), false, None));
                }
            }
        }

        println!();
    }

    // 显示汇总
    println!("{}", "═══ 测试汇总 ═══".cyan().bold());
    println!();
    println!("  总计测试: {}", total_tested);
    println!("  成功: {}", format!("{}", total_success).green());
    println!("  失败: {}", format!("{}", total_tested - total_success).red());
    println!("  成功率: {}%", (total_success * 100 / total_tested.max(1)));
    println!();

    // 显示详细结果（仅失败的）
    let failed: Vec<_> = results.iter().filter(|r| !r.2).collect();
    if !failed.is_empty() {
        println!("{}", "失败的供应商:".yellow().bold());
        for (app, name, _, _) in failed {
            println!("  {} - {}", app.dimmed(), name.red());
        }
        println!();
    }

    Ok(())
}

/// 测试单个供应商的 API
async fn test_provider_api(
    _id: &str,
    provider: &Provider,
    app_type: AppType,
    timeout: u64,
) -> Result<u64> {
    let start = std::time::Instant::now();

    // 从 settings_config 中提取配置
    let api_key = provider.settings_config.get("apiKey")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("缺少 API Key"))?;

    let base_url = provider.settings_config.get("baseUrl")
        .and_then(|v| v.as_str());

    // 根据应用类型调用不同的 API 测试
    match app_type {
        AppType::Claude => {
            let url = base_url.unwrap_or("https://api.anthropic.com");
            test_anthropic_api(api_key, url, timeout).await?;
        }
        AppType::Codex => {
            let url = base_url.unwrap_or("https://api.openai.com/v1");
            test_openai_api(api_key, url, timeout).await?;
        }
        AppType::Gemini => {
            let url = base_url.unwrap_or("https://generativelanguage.googleapis.com");
            test_gemini_api(api_key, url, timeout).await?;
        }
        AppType::OpenCode => {
            bail!("OpenCode 不支持 API 测试");
        }
    }

    let elapsed = start.elapsed().as_millis() as u64;
    Ok(elapsed)
}

/// 测试 Anthropic API
async fn test_anthropic_api(api_key: &str, base_url: &str, timeout: u64) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .build()?;

    let response = client
        .post(format!("{}/v1/messages", base_url))
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&serde_json::json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 10,
            "messages": [{"role": "user", "content": "Hi"}]
        }))
        .send()
        .await?;

    if !response.status().is_success() {
        bail!("API 返回错误: {}", response.status());
    }

    Ok(())
}

/// 测试 OpenAI API
async fn test_openai_api(api_key: &str, base_url: &str, timeout: u64) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .build()?;

    let response = client
        .get(format!("{}/models", base_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .send()
        .await?;

    if !response.status().is_success() {
        bail!("API 返回错误: {}", response.status());
    }

    Ok(())
}

/// 测试 Gemini API
async fn test_gemini_api(api_key: &str, base_url: &str, timeout: u64) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout))
        .build()?;

    let response = client
        .get(format!("{}/v1/models?key={}", base_url, api_key))
        .send()
        .await?;

    if !response.status().is_success() {
        bail!("API 返回错误: {}", response.status());
    }

    Ok(())
}

/// 批量导出配置
pub fn batch_export(_ctx: &OutputContext, output_file: &str, app_type: AppTypeArg) -> Result<()> {
    let state = AppState::init()?;

    println!();
    println!("{}", "═══ 批量导出配置 ═══".cyan().bold());
    println!();

    let mut all_configs = HashMap::new();

    for app in app_type.to_app_types() {
        let providers = ccswitch_core::ProviderService::list(&state, app)?;

        if !providers.is_empty() {
            println!("  {} 导出 {} 个供应商", app.display_name().cyan(), providers.len());
            all_configs.insert(app.to_string(), providers);
        }
    }

    // 序列化为 YAML
    let yaml = serde_yaml::to_string(&all_configs)?;
    std::fs::write(output_file, yaml)?;

    println!();
    println!("{}", format!("✓ 配置已导出到: {}", output_file).green().bold());
    println!();

    Ok(())
}

/// 批量导入配置
pub fn batch_import(_ctx: &OutputContext, input_file: &str, overwrite: bool) -> Result<()> {
    let state = AppState::init()?;

    println!();
    println!("{}", "═══ 批量导入配置 ═══".cyan().bold());
    println!();

    // 读取文件
    let content = std::fs::read_to_string(input_file)?;
    let configs: HashMap<String, indexmap::IndexMap<String, Provider>> = serde_yaml::from_str(&content)?;

    let mut total_imported = 0;
    let mut total_skipped = 0;

    for (app_str, providers) in configs.into_iter() {
        let app_type: AppType = match app_str.as_str() {
            "claude" => AppType::Claude,
            "codex" => AppType::Codex,
            "gemini" => AppType::Gemini,
            "opencode" => AppType::OpenCode,
            _ => {
                println!("  {} 未知的应用类型: {}", "⚠".yellow(), app_str);
                continue;
            }
        };

        println!("{}: 导入 {} 个供应商", app_type.display_name().cyan(), providers.len());

        for (_id, provider) in providers {
            // 检查是否已存在
            let existing = ccswitch_core::ProviderService::find(&state, app_type, &provider.name)?;
            let exists = existing.is_some();

            if exists && !overwrite {
                println!("  {} {} (已跳过)", "→".dimmed(), provider.name.dimmed());
                total_skipped += 1;
                continue;
            }

            match ccswitch_core::ProviderService::add(&state, app_type, provider.clone()) {
                Ok(_) => {
                    println!("  {} {} {}",
                        "→".blue(),
                        provider.name,
                        if exists { "(已覆盖)".yellow() } else { "(新增)".green() }
                    );
                    total_imported += 1;
                }
                Err(e) => {
                    println!("  {} {} {}", "✗".red(), provider.name, format!("({})", e).dimmed());
                }
            }
        }

        println!();
    }

    println!("{}", "═══ 导入汇总 ═══".cyan().bold());
    println!();
    println!("  导入成功: {}", format!("{}", total_imported).green());
    println!("  已跳过: {}", format!("{}", total_skipped).yellow());
    println!();

    Ok(())
}

/// 批量删除供应商
pub fn batch_remove(
    _ctx: &OutputContext,
    names: &[String],
    app_type: AppTypeArg,
    force: bool,
) -> Result<()> {
    let state = AppState::init()?;

    println!();
    println!("{}", "═══ 批量删除供应商 ═══".cyan().bold());
    println!();

    if !force {
        println!("将删除以下供应商:");
        for name in names {
            println!("  {} {}", "×".red(), name);
        }
        println!();

        print!("确认删除? [y/N]: ");
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("{}", "已取消".yellow());
            return Ok(());
        }
        println!();
    }

    let mut success_count = 0;
    let mut failed = Vec::new();

    for app in app_type.to_app_types() {
        for name in names {
            match ccswitch_core::ProviderService::delete(&state, app, name) {
                Ok(_) => {
                    println!("  {} {} - {}", "✓".green(), app.display_name().dimmed(), name);
                    success_count += 1;
                }
                Err(e) => {
                    println!("  {} {} - {} {}", "✗".red(), app.display_name().dimmed(), name, format!("({})", e).dimmed());
                    failed.push((app.display_name(), name.clone()));
                }
            }
        }
    }

    println!();
    if failed.is_empty() {
        println!("{}", format!("✓ 成功删除 {} 个供应商", success_count).green().bold());
    } else {
        println!("{}", format!("⚠ 成功删除 {} 个，失败 {} 个", success_count, failed.len()).yellow());
    }
    println!();

    Ok(())
}

/// 批量同步配置（从一个应用复制到其他应用）
pub fn batch_sync(
    _ctx: &OutputContext,
    source_app: AppType,
    target_apps: Vec<AppType>,
    overwrite: bool,
) -> Result<()> {
    let state = AppState::init()?;

    println!();
    println!("{}", "═══ 批量同步配置 ═══".cyan().bold());
    println!();

    // 获取源应用的所有供应商
    let source_providers = ccswitch_core::ProviderService::list(&state, source_app)?;

    if source_providers.is_empty() {
        bail!("源应用 {} 没有供应商配置", source_app.display_name());
    }

    println!("从 {} 同步 {} 个供应商到:",
        source_app.display_name().cyan().bold(),
        source_providers.len()
    );

    for target in &target_apps {
        println!("  → {}", target.display_name());
    }
    println!();

    let mut total_synced = 0;
    let mut total_skipped = 0;

    for target in target_apps {
        println!("{}", target.display_name().cyan().bold());

        for (_id, provider) in &source_providers {
            let existing = ccswitch_core::ProviderService::find(&state, target, &provider.name)?;
            let exists = existing.is_some();

            if exists && !overwrite {
                println!("  {} {} (已跳过)", "→".dimmed(), provider.name.dimmed());
                total_skipped += 1;
                continue;
            }

            match ccswitch_core::ProviderService::add(&state, target, provider.clone()) {
                Ok(_) => {
                    println!("  {} {} {}",
                        "→".blue(),
                        provider.name,
                        if exists { "(已覆盖)".yellow() } else { "(新增)".green() }
                    );
                    total_synced += 1;
                }
                Err(e) => {
                    println!("  {} {} {}", "✗".red(), provider.name, format!("({})", e).dimmed());
                }
            }
        }

        println!();
    }

    println!("{}", "═══ 同步汇总 ═══".cyan().bold());
    println!();
    println!("  同步成功: {}", format!("{}", total_synced).green());
    println!("  已跳过: {}", format!("{}", total_skipped).yellow());
    println!();

    Ok(())
}

/// 批量编辑供应商配置
pub fn batch_edit(
    _ctx: &OutputContext,
    app_type: AppTypeArg,
    field: &str,
    value: &str,
    pattern: Option<&str>,
) -> Result<()> {
    let state = AppState::init()?;

    println!();
    println!("{}", "═══ 批量编辑配置 ═══".cyan().bold());
    println!();
    println!("修改字段: {}", field.cyan());
    println!("新值: {}", value.green());
    if let Some(p) = pattern {
        println!("匹配模式: {}", p.yellow());
    }
    println!();

    let mut total_updated = 0;

    for app in app_type.to_app_types() {
        let providers = ccswitch_core::ProviderService::list(&state, app)?;

        if providers.is_empty() {
            continue;
        }

        println!("{}", app.display_name().cyan().bold());

        for (_id, mut provider) in providers {
            // 检查是否匹配模式
            if let Some(p) = pattern {
                if !provider.name.contains(p) {
                    continue;
                }
            }

            // 根据字段名更新配置
            let config = provider.settings_config.as_object_mut()
                .ok_or_else(|| anyhow::anyhow!("配置格式错误"))?;

            match field {
                "base_url" | "base-url" | "baseUrl" => {
                    config.insert("baseUrl".to_string(), serde_json::Value::String(value.to_string()));
                }
                "model" => {
                    config.insert("model".to_string(), serde_json::Value::String(value.to_string()));
                }
                "small_model" | "small-model" | "smallModel" => {
                    config.insert("smallModel".to_string(), serde_json::Value::String(value.to_string()));
                }
                _ => {
                    bail!("不支持的字段: {}。支持的字段: base-url, model, small-model", field);
                }
            }

            match ccswitch_core::ProviderService::update(&state, app, provider.clone()) {
                Ok(_) => {
                    println!("  {} {}", "✓".green(), provider.name);
                    total_updated += 1;
                }
                Err(e) => {
                    println!("  {} {} {}", "✗".red(), provider.name, format!("({})", e).dimmed());
                }
            }
        }

        println!();
    }

    println!("{}", format!("✓ 成功更新 {} 个供应商", total_updated).green().bold());
    println!();

    Ok(())
}
