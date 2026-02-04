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
    println!("{}", "── 供应商管理 ──".dimmed());
    println!("  {} {} - 查看所有供应商配置", "1.".green(), "列出供应商".white());
    println!("  {} {} - 查看当前使用的供应商", "2.".green(), "查看状态".white());
    println!("  {} {} - 切换到其他供应商", "3.".green(), "切换供应商".white());
    println!("  {} {} - 添加新的供应商配置", "4.".green(), "添加供应商".white());
    println!("  {} {} - 编辑供应商配置", "5.".green(), "编辑供应商".white());
    println!("  {} {} - 测试供应商 API", "6.".green(), "测试供应商".white());
    println!("  {} {} - 删除供应商配置", "7.".green(), "删除供应商".white());
    println!();
    println!("{}", "── 扩展功能 ──".dimmed());
    println!("  {} {} - 管理 MCP 服务器", "8.".green(), "MCP 服务器".white());
    println!("  {} {} - 管理系统提示词", "9.".green(), "Prompts".white());
    println!(" {} {} - 管理 Skills 扩展", "10.".green(), "Skills".white());
    println!();
    println!("{}", "── 工具 ──".dimmed());
    println!(" {} {} - 设置全局代理", "11.".green(), "代理设置".white());
    println!(" {} {} - 测试 API 端点延迟", "12.".green(), "端点测速".white());
    println!(" {} {} - 检测环境变量冲突", "13.".green(), "环境检测".white());
    println!(" {} {} - 查看配置文件路径", "14.".green(), "查看配置".white());
    println!();
    println!("  {} {} - 退出程序", "0.".green(), "退出".white());
    println!();

    loop {
        let choice = read_input("请输入操作编号: ")?;

        match choice.as_str() {
            "1" | "list" | "ls" => {
                commands::list::list_providers(&ctx, AppTypeArg::All, false, true)?;
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
            "5" | "edit" => {
                return interactive_edit(&ctx);
            }
            "6" | "test" => {
                return interactive_test(&ctx);
            }
            "7" | "remove" | "rm" => {
                return interactive_remove(&ctx);
            }
            "8" | "mcp" => {
                return interactive_mcp(&ctx);
            }
            "9" | "prompt" | "prompts" => {
                return interactive_prompt(&ctx);
            }
            "10" | "skill" | "skills" => {
                return interactive_skill(&ctx);
            }
            "11" | "proxy" => {
                return interactive_proxy(&ctx);
            }
            "12" | "speedtest" | "speed" => {
                return interactive_speedtest(&ctx);
            }
            "13" | "env" => {
                return interactive_env(&ctx);
            }
            "14" | "config" => {
                commands::config::show_paths(&ctx, AppTypeArg::All)?;
                return Ok(());
            }
            "0" | "q" | "quit" | "exit" => {
                println!("{}", "再见！".green());
                return Ok(());
            }
            "" => {
                // 空输入显示提示
                println!("{}", "请输入 1-14 选择操作，或输入 0 退出".dimmed());
            }
            _ => {
                println!("{}", "无效选择，请输入 0-14".yellow());
            }
        }
    }
}

/// 交互式切换供应商
fn interactive_switch(ctx: &OutputContext) -> Result<()> {
    println!("\n{}", "═══ 切换供应商 ═══".cyan().bold());
    
    let app_type = select_app_type()?;
    let state = AppState::init()?;
    let name = select_provider(&state, app_type.clone())?;
    
    let app_arg = match app_type {
        AppType::Claude => AppTypeArg::Claude,
        AppType::Codex => AppTypeArg::Codex,
        AppType::Gemini => AppTypeArg::Gemini,
        AppType::OpenCode => AppTypeArg::Opencode,
    };
    
    commands::provider::switch(ctx, &name, app_arg)?;
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
                Some(api_key), base_url, model, small_model, None, false
            )?;
        }
        AppType::Codex => {
            println!("\n{}", "配置 Codex:".white().bold());
            let api_key = read_required("API Key")?;
            let base_url = read_optional("Base URL", Some("https://api.openai.com/v1"))?;
            let model = read_optional("模型", Some("gpt-4"))?;
            
            commands::provider::add(
                ctx, &name, app_arg,
                Some(api_key), base_url, model, None, None, false
            )?;
        }
        AppType::Gemini => {
            println!("\n{}", "配置 Gemini CLI:".white().bold());
            let api_key = read_required("API Key")?;
            let base_url = read_optional("Base URL", Some("https://generativelanguage.googleapis.com"))?;
            let model = read_optional("模型", Some("gemini-2.0-flash"))?;
            
            commands::provider::add(
                ctx, &name, app_arg,
                Some(api_key), base_url, model, None, None, false
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
    let name = select_provider(&state, app_type.clone())?;

    let app_arg = match app_type {
        AppType::Claude => AppTypeArg::Claude,
        AppType::Codex => AppTypeArg::Codex,
        AppType::Gemini => AppTypeArg::Gemini,
        AppType::OpenCode => AppTypeArg::Opencode,
    };

    commands::provider::remove(ctx, &name, app_arg, false)?;
    Ok(())
}

/// 交互式编辑供应商
fn interactive_edit(ctx: &OutputContext) -> Result<()> {
    println!("\n{}", "═══ 编辑供应商 ═══".cyan().bold());

    let app_type = select_app_type()?;
    let state = AppState::init()?;
    let name = select_provider(&state, app_type.clone())?;

    let app_arg = match app_type {
        AppType::Claude => AppTypeArg::Claude,
        AppType::Codex => AppTypeArg::Codex,
        AppType::Gemini => AppTypeArg::Gemini,
        AppType::OpenCode => AppTypeArg::Opencode,
    };

    println!("\n{}", "修改配置 (留空保持不变):".white().bold());

    let new_name = read_optional("新名称", None)?;
    let api_key = read_optional("新 API Key", None)?;
    let base_url = read_optional("新 Base URL", None)?;
    let model = read_optional("新模型", None)?;
    let small_model = if matches!(app_type, AppType::Claude) {
        read_optional("新小模型", None)?
    } else {
        None
    };

    // 检查是否有任何修改
    if new_name.is_none() && api_key.is_none() && base_url.is_none() && model.is_none() && small_model.is_none() {
        println!("{}", "没有进行任何修改".yellow());
        return Ok(());
    }

    commands::provider::edit(ctx, &name, app_arg, api_key, base_url, model, small_model, new_name)?;
    Ok(())
}

/// 交互式测试供应商
fn interactive_test(ctx: &OutputContext) -> Result<()> {
    println!("\n{}", "═══ 测试供应商 API ═══".cyan().bold());
    println!();
    println!("  {} {} - 测试已配置的供应商", "1.".green(), "选择供应商".white());
    println!("  {} {} - 直接输入 API Key 测试", "2.".green(), "手动测试".white());
    println!("  {} {} - 返回主菜单", "0.".green(), "返回".white());
    println!();

    loop {
        let choice = read_input("请选择: ")?;
        match choice.as_str() {
            "1" => {
                let app_type = select_app_type()?;
                let state = AppState::init()?;
                let name = select_provider(&state, app_type.clone())?;

                let app_arg = match app_type {
                    AppType::Claude => AppTypeArg::Claude,
                    AppType::Codex => AppTypeArg::Codex,
                    AppType::Gemini => AppTypeArg::Gemini,
                    AppType::OpenCode => AppTypeArg::Opencode,
                };

                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(commands::provider::test_api(ctx, Some(name), app_arg, None, None, None, 30))?;
                return Ok(());
            }
            "2" => {
                let app_type = select_app_type()?;
                let api_key = read_required("API Key")?;
                let base_url = read_optional("Base URL", None)?;
                let model = read_optional("测试模型", None)?;

                let app_arg = match app_type {
                    AppType::Claude => AppTypeArg::Claude,
                    AppType::Codex => AppTypeArg::Codex,
                    AppType::Gemini => AppTypeArg::Gemini,
                    AppType::OpenCode => AppTypeArg::Opencode,
                };

                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(commands::provider::test_api(ctx, None, app_arg, Some(api_key), base_url, model, 30))?;
                return Ok(());
            }
            "0" | "q" | "back" => return Ok(()),
            _ => println!("{}", "无效选择".yellow()),
        }
    }
}

/// 交互式 MCP 管理
fn interactive_mcp(ctx: &OutputContext) -> Result<()> {
    println!("\n{}", "═══ MCP 服务器管理 ═══".cyan().bold());
    println!();
    println!("  {} {} - 列出所有 MCP 服务器", "1.".green(), "列出".white());
    println!("  {} {} - 添加 MCP 服务器", "2.".green(), "添加".white());
    println!("  {} {} - 从应用导入", "3.".green(), "导入".white());
    println!("  {} {} - 返回主菜单", "0.".green(), "返回".white());
    println!();

    loop {
        let choice = read_input("请选择: ")?;
        match choice.as_str() {
            "1" | "list" => {
                commands::mcp::list(ctx, AppTypeArg::All, false)?;
                return Ok(());
            }
            "2" | "add" => {
                println!("\n{}", "添加 MCP 服务器:".white().bold());
                let id = read_required("服务器 ID")?;
                let command = read_required("执行命令")?;
                let args_str = read_optional("命令参数 (空格分隔)", None)?;
                let args: Vec<String> = args_str
                    .map(|s| s.split_whitespace().map(|s| s.to_string()).collect())
                    .unwrap_or_default();
                let name = read_optional("显示名称", None)?;
                let description = read_optional("描述", None)?;

                commands::mcp::add(ctx, &id, &command, args, vec![], name, description)?;
                return Ok(());
            }
            "3" | "import" => {
                commands::mcp::import(ctx, None)?;
                return Ok(());
            }
            "0" | "q" | "back" => return Ok(()),
            _ => println!("{}", "无效选择".yellow()),
        }
    }
}

/// 交互式 Prompt 管理
fn interactive_prompt(ctx: &OutputContext) -> Result<()> {
    println!("\n{}", "═══ Prompts 管理 ═══".cyan().bold());
    println!();
    println!("  {} {} - 列出所有 Prompts", "1.".green(), "列出".white());
    println!("  {} {} - 添加 Prompt", "2.".green(), "添加".white());
    println!("  {} {} - 从应用导入", "3.".green(), "导入".white());
    println!("  {} {} - 返回主菜单", "0.".green(), "返回".white());
    println!();

    loop {
        let choice = read_input("请选择: ")?;
        match choice.as_str() {
            "1" | "list" => {
                commands::prompt::list(ctx, AppTypeArg::All)?;
                return Ok(());
            }
            "2" | "add" => {
                println!("\n{}", "添加 Prompt:".white().bold());
                let app_type = select_app_type()?;
                let app_arg = match app_type {
                    AppType::Claude => AppTypeArg::Claude,
                    AppType::Codex => AppTypeArg::Codex,
                    AppType::Gemini => AppTypeArg::Gemini,
                    AppType::OpenCode => AppTypeArg::Opencode,
                };
                let name = read_required("Prompt 名称")?;
                let content = read_required("Prompt 内容")?;
                let description = read_optional("描述", None)?;

                commands::prompt::add(ctx, app_arg, &name, Some(content), None, description)?;
                return Ok(());
            }
            "3" | "import" => {
                commands::prompt::import(ctx, AppTypeArg::All)?;
                return Ok(());
            }
            "0" | "q" | "back" => return Ok(()),
            _ => println!("{}", "无效选择".yellow()),
        }
    }
}

/// 交互式 Skill 管理
fn interactive_skill(ctx: &OutputContext) -> Result<()> {
    println!("\n{}", "═══ Skills 管理 ═══".cyan().bold());
    println!();
    println!("  {} {} - 列出所有 Skills", "1.".green(), "列出".white());
    println!("  {} {} - 从 GitHub 安装", "2.".green(), "安装".white());
    println!("  {} {} - 扫描本地目录", "3.".green(), "扫描".white());
    println!("  {} {} - 同步到所有应用", "4.".green(), "同步".white());
    println!("  {} {} - 返回主菜单", "0.".green(), "返回".white());
    println!();

    loop {
        let choice = read_input("请选择: ")?;
        match choice.as_str() {
            "1" | "list" => {
                commands::skill::list(ctx, AppTypeArg::All, true)?;
                return Ok(());
            }
            "2" | "install" => {
                println!("\n{}", "安装 Skill:".white().bold());
                let repo = read_required("GitHub 仓库 (owner/name)")?;
                let branch = read_optional("分支", Some("main"))?;

                commands::skill::install(ctx, &repo, branch, None)?;
                return Ok(());
            }
            "3" | "scan" => {
                commands::skill::scan(ctx)?;
                return Ok(());
            }
            "4" | "sync" => {
                commands::skill::sync(ctx)?;
                return Ok(());
            }
            "0" | "q" | "back" => return Ok(()),
            _ => println!("{}", "无效选择".yellow()),
        }
    }
}

/// 交互式代理设置
fn interactive_proxy(ctx: &OutputContext) -> Result<()> {
    println!("\n{}", "═══ 代理设置 ═══".cyan().bold());
    println!();
    println!("  {} {} - 查看当前代理", "1.".green(), "查看".white());
    println!("  {} {} - 设置代理", "2.".green(), "设置".white());
    println!("  {} {} - 清除代理", "3.".green(), "清除".white());
    println!("  {} {} - 测试代理", "4.".green(), "测试".white());
    println!("  {} {} - 扫描本地代理", "5.".green(), "扫描".white());
    println!("  {} {} - 返回主菜单", "0.".green(), "返回".white());
    println!();

    loop {
        let choice = read_input("请选择: ")?;
        match choice.as_str() {
            "1" | "get" => {
                commands::proxy::get(ctx)?;
                return Ok(());
            }
            "2" | "set" => {
                let url = read_required("代理 URL (如 http://127.0.0.1:7890)")?;
                commands::proxy::set(ctx, &url)?;
                return Ok(());
            }
            "3" | "clear" => {
                commands::proxy::clear(ctx)?;
                return Ok(());
            }
            "4" | "test" => {
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(commands::proxy::test(ctx, None))?;
                return Ok(());
            }
            "5" | "scan" => {
                tokio::runtime::Runtime::new()
                    .unwrap()
                    .block_on(commands::proxy::scan(ctx))?;
                return Ok(());
            }
            "0" | "q" | "back" => return Ok(()),
            _ => println!("{}", "无效选择".yellow()),
        }
    }
}

/// 交互式端点测速
fn interactive_speedtest(ctx: &OutputContext) -> Result<()> {
    println!("\n{}", "═══ 端点测速 ═══".cyan().bold());
    println!();
    println!("将测试以下端点的延迟:");
    println!("  - https://api.anthropic.com");
    println!("  - https://api.openai.com");
    println!("  - https://generativelanguage.googleapis.com");
    println!();

    let input = read_input("是否开始测试? [Y/n]: ")?;
    if input.is_empty() || input.to_lowercase() == "y" {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(commands::speedtest::test(ctx, vec![], 10, false))?;
    }

    Ok(())
}

/// 交互式环境检测
fn interactive_env(ctx: &OutputContext) -> Result<()> {
    println!("\n{}", "═══ 环境变量检测 ═══".cyan().bold());
    println!();
    println!("  {} {} - 检查环境变量冲突", "1.".green(), "检查".white());
    println!("  {} {} - 列出相关环境变量", "2.".green(), "列出".white());
    println!("  {} {} - 返回主菜单", "0.".green(), "返回".white());
    println!();

    loop {
        let choice = read_input("请选择: ")?;
        match choice.as_str() {
            "1" | "check" => {
                commands::env::check(ctx, AppTypeArg::All)?;
                return Ok(());
            }
            "2" | "list" => {
                commands::env::list(ctx, AppTypeArg::All)?;
                return Ok(());
            }
            "0" | "q" | "back" => return Ok(()),
            _ => println!("{}", "无效选择".yellow()),
        }
    }
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
