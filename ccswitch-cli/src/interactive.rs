//! 交互式引导模块
//!
//! 为新手提供友好的交互式操作体验。

use anyhow::{bail, Result};
use colored::Colorize;
use std::io::{self, Write};

use ccswitch_core::{AppState, AppType};

use crate::cli::AppTypeArg;
use crate::commands;
use crate::output::OutputContext;

/// 读取用户输入
fn read_input(prompt: &str) -> Result<String> {
    print!("{}", prompt);
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}

/// 读取可选输入（允许空）
fn read_optional(prompt: &str, default: Option<&str>) -> Result<Option<String>> {
    let prompt_with_default = if let Some(d) = default {
        format!("{} [{}]: ", prompt, d.dimmed())
    } else {
        format!("{} (可选): ", prompt)
    };
    
    let input = read_input(&prompt_with_default)?;
    if input.is_empty() {
        Ok(default.map(|s| s.to_string()))
    } else {
        Ok(Some(input))
    }
}

/// 读取必填输入
fn read_required(prompt: &str) -> Result<String> {
    loop {
        let input = read_input(&format!("{}: ", prompt))?;
        if !input.is_empty() {
            return Ok(input);
        }
        println!("{}", "此项为必填，请输入内容".yellow());
    }
}

/// 选择应用类型
fn select_app_type() -> Result<AppType> {
    println!("\n{}", "选择应用类型:".cyan().bold());
    println!("  {} Claude Code (Anthropic 官方 CLI)", "1.".green());
    println!("  {} Codex (OpenAI CLI)", "2.".green());
    println!("  {} Gemini CLI (Google)", "3.".green());
    println!("  {} OpenCode", "4.".green());
    
    loop {
        let choice = read_input("\n请输入数字 [1]: ")?;
        let choice = if choice.is_empty() { "1".to_string() } else { choice };
        
        match choice.as_str() {
            "1" | "claude" => return Ok(AppType::Claude),
            "2" | "codex" => return Ok(AppType::Codex),
            "3" | "gemini" => return Ok(AppType::Gemini),
            "4" | "opencode" => return Ok(AppType::OpenCode),
            _ => println!("{}", "无效选择，请输入 1-4".yellow()),
        }
    }
}

/// 选择供应商
fn select_provider(state: &AppState, app_type: AppType) -> Result<String> {
    let providers = ccswitch_core::ProviderService::list(state, app_type)?;
    let current_id = ccswitch_core::ProviderService::current(state, app_type).unwrap_or_default();
    
    if providers.is_empty() {
        bail!("没有找到 {} 的供应商配置", app_type.display_name());
    }
    
    println!("\n{}", format!("可用的 {} 供应商:", app_type.display_name()).cyan().bold());
    
    let provider_list: Vec<_> = providers.iter().collect();
    for (i, (id, p)) in provider_list.iter().enumerate() {
        let is_current = *id == &current_id;
        let status = if is_current { "●".green() } else { "○".dimmed() };
        let current = if is_current { " (当前)".green().to_string() } else { String::new() };
        println!("  {} {} {}{}", format!("{}.", i + 1).green(), status, p.name, current);
    }
    
    loop {
        let choice = read_input("\n请输入序号或名称: ")?;
        
        // 尝试解析为数字
        if let Ok(num) = choice.parse::<usize>() {
            if num > 0 && num <= provider_list.len() {
                return Ok(provider_list[num - 1].1.name.clone());
            }
        }
        
        // 尝试匹配名称
        if providers.values().any(|p| p.name == choice) || providers.contains_key(&choice) {
            return Ok(choice);
        }
        
        println!("{}", "无效选择，请重新输入".yellow());
    }
}

/// 主菜单
pub fn main_menu() -> Result<()> {
    let ctx = OutputContext::new(crate::cli::OutputFormat::Table, false);
    
    println!();
    println!("{}", "╔════════════════════════════════════════╗".cyan());
    println!("{}", "║     CC-Switch - AI CLI 配置管理器      ║".cyan().bold());
    println!("{}", "╚════════════════════════════════════════╝".cyan());
    println!();
    println!("{}", "请选择操作:".white().bold());
    println!();
    println!("  {} {} - 查看所有供应商配置", "1.".green(), "列出供应商".white());
    println!("  {} {} - 查看当前使用的供应商", "2.".green(), "查看状态".white());
    println!("  {} {} - 切换到其他供应商", "3.".green(), "切换供应商".white());
    println!("  {} {} - 添加新的供应商配置", "4.".green(), "添加供应商".white());
    println!("  {} {} - 删除供应商配置", "5.".green(), "删除供应商".white());
    println!("  {} {} - 查看配置文件路径", "6.".green(), "查看配置".white());
    println!("  {} {} - 退出程序", "0.".green(), "退出".white());
    println!();
    
    loop {
        let choice = read_input("请输入操作编号: ")?;
        
        match choice.as_str() {
            "1" | "list" | "ls" => {
                commands::list::list_providers(&ctx, AppTypeArg::All, false)?;
                return Ok(());
            }
            "2" | "status" => {
                commands::status::show_status(&ctx, AppTypeArg::All)?;
                return Ok(());
            }
            "3" | "use" | "switch" => {
                return interactive_switch(&ctx);
            }
            "4" | "add" => {
                return interactive_add(&ctx);
            }
            "5" | "remove" | "rm" => {
                return interactive_remove(&ctx);
            }
            "6" | "config" => {
                commands::config::show_paths(&ctx, AppTypeArg::All)?;
                return Ok(());
            }
            "0" | "q" | "quit" | "exit" => {
                println!("{}", "再见！".green());
                return Ok(());
            }
            "" => {
                // 空输入显示提示
                println!("{}", "请输入 1-6 选择操作，或输入 0 退出".dimmed());
            }
            _ => {
                println!("{}", "无效选择，请输入 0-6".yellow());
            }
        }
    }
}

/// 交互式切换供应商
fn interactive_switch(ctx: &OutputContext) -> Result<()> {
    println!("\n{}", "═══ 切换供应商 ═══".cyan().bold());
    
    let app_type = select_app_type()?;
    let state = AppState::init()?;
    let name = select_provider(&state, app_type)?;
    
    commands::provider::switch(ctx, &name, AppTypeArg::Claude)?;
    Ok(())
}

/// 交互式添加供应商
fn interactive_add(ctx: &OutputContext) -> Result<()> {
    println!("\n{}", "═══ 添加新供应商 ═══".cyan().bold());
    
    let app_type = select_app_type()?;
    let name = read_required("供应商名称")?;
    
    let app_arg = match app_type {
        AppType::Claude => AppTypeArg::Claude,
        AppType::Codex => AppTypeArg::Codex,
        AppType::Gemini => AppTypeArg::Gemini,
        AppType::OpenCode => AppTypeArg::Opencode,
    };
    
    // 根据应用类型收集不同的配置
    match app_type {
        AppType::Claude => {
            println!("\n{}", "配置 Claude Code:".white().bold());
            let api_key = read_required("API Key")?;
            let base_url = read_optional("Base URL", Some("https://api.anthropic.com"))?;
            let model = read_optional("主模型", Some("claude-sonnet-4-20250514"))?;
            let small_model = read_optional("小模型", None)?;
            
            commands::provider::add(
                ctx, &name, app_arg,
                Some(api_key), base_url, model, small_model, None
            )?;
        }
        AppType::Codex => {
            println!("\n{}", "配置 Codex:".white().bold());
            let api_key = read_required("API Key")?;
            let base_url = read_optional("Base URL", Some("https://api.openai.com/v1"))?;
            let model = read_optional("模型", Some("gpt-4"))?;
            
            commands::provider::add(
                ctx, &name, app_arg,
                Some(api_key), base_url, model, None, None
            )?;
        }
        AppType::Gemini => {
            println!("\n{}", "配置 Gemini CLI:".white().bold());
            let api_key = read_required("API Key")?;
            let base_url = read_optional("Base URL", Some("https://generativelanguage.googleapis.com"))?;
            let model = read_optional("模型", Some("gemini-2.0-flash"))?;
            
            commands::provider::add(
                ctx, &name, app_arg,
                Some(api_key), base_url, model, None, None
            )?;
        }
        AppType::OpenCode => {
            println!("{}", "OpenCode 配置暂不支持交互式添加".yellow());
            println!("请使用: cc-switch add <名称> --app opencode --from-file <配置文件>");
        }
    }
    
    Ok(())
}

/// 交互式删除供应商
fn interactive_remove(ctx: &OutputContext) -> Result<()> {
    println!("\n{}", "═══ 删除供应商 ═══".cyan().bold());
    
    let app_type = select_app_type()?;
    let state = AppState::init()?;
    let name = select_provider(&state, app_type)?;
    
    let app_arg = match app_type {
        AppType::Claude => AppTypeArg::Claude,
        AppType::Codex => AppTypeArg::Codex,
        AppType::Gemini => AppTypeArg::Gemini,
        AppType::OpenCode => AppTypeArg::Opencode,
    };
    
    commands::provider::remove(ctx, &name, app_arg, false)?;
    Ok(())
}

/// 快速开始引导
pub fn quick_start() -> Result<()> {
    println!();
    println!("{}", "🚀 欢迎使用 CC-Switch!".cyan().bold());
    println!();
    println!("CC-Switch 帮助你管理多个 AI CLI 工具的供应商配置。");
    println!("支持: {} | {} | {} | {}", 
        "Claude Code".green(), 
        "Codex".blue(), 
        "Gemini".yellow(),
        "OpenCode".magenta()
    );
    println!();
    
    println!("{}", "常用命令:".white().bold());
    println!();
    println!("  {}      列出所有供应商", "cc-switch list".green());
    println!("  {}    查看当前状态", "cc-switch status".green());
    println!("  {} 切换供应商", "cc-switch use <名称>".green());
    println!("  {}           进入交互模式", "cc-switch".green());
    println!();
    
    let input = read_input("是否进入交互模式? [Y/n]: ")?;
    if input.is_empty() || input.to_lowercase() == "y" || input.to_lowercase() == "yes" {
        main_menu()?;
    }
    
    Ok(())
}
