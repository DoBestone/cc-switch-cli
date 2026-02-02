//! Prompt 命令模块
//!
//! 实现 Prompt 管理的 CLI 命令。

use anyhow::{bail, Result};
use ccswitch_core::{AppState, AppType, Prompt, PromptService};
use std::fs;

use crate::cli::AppTypeArg;
use crate::output::{print_error, print_info, print_success, print_warning, OutputContext};

/// 列出所有 Prompts
pub fn list(_ctx: &OutputContext, app: AppTypeArg) -> Result<()> {
    let state = AppState::init()?;
    let app_types = app.to_app_types();

    let mut has_prompts = false;

    for app_type in app_types {
        let prompts = PromptService::list(&state, app_type)?;

        if prompts.is_empty() {
            continue;
        }

        has_prompts = true;

        println!("\n📝 {} Prompts ({} 个)\n", app_type.display_name(), prompts.len());
        println!("{:<20} {:<30} {:<10}", "ID", "名称", "状态");
        println!("{}", "-".repeat(60));

        for (_, prompt) in &prompts {
            let status = if prompt.enabled { "✓ 启用" } else { "○ 禁用" };
            println!("{:<20} {:<30} {:<10}", prompt.id, prompt.name, status);
        }
    }

    if !has_prompts {
        print_info("暂无 Prompt 配置");
        print_info("使用 'cc-switch prompt add <name> --content <content>' 添加");
    }

    Ok(())
}

/// 显示单个 Prompt 详情
pub fn show(_ctx: &OutputContext, app: AppTypeArg, id: &str) -> Result<()> {
    let state = AppState::init()?;
    let app_types = app.to_app_types();

    for app_type in app_types {
        if let Some(prompt) = PromptService::get(&state, app_type, id)? {
            println!("\n📝 Prompt: {}\n", prompt.name);
            println!("ID:     {}", prompt.id);
            println!("名称:   {}", prompt.name);
            println!("状态:   {}", if prompt.enabled { "启用" } else { "禁用" });
            println!("应用:   {}", app_type.display_name());

            if let Some(desc) = &prompt.description {
                println!("描述:   {}", desc);
            }

            println!("\n内容:\n{}", "-".repeat(40));
            println!("{}", prompt.content);

            return Ok(());
        }
    }

    print_error(&format!("Prompt '{}' 不存在", id));
    bail!("Prompt 不存在")
}

/// 添加 Prompt
pub fn add(
    _ctx: &OutputContext,
    app: AppTypeArg,
    name: &str,
    content: Option<String>,
    file: Option<String>,
    description: Option<String>,
) -> Result<()> {
    let state = AppState::init()?;

    let app_types = app.to_app_types();
    if app_types.is_empty() || matches!(app, AppTypeArg::All) {
        print_error("请指定具体的应用类型 (claude/codex/gemini/opencode)");
        bail!("需要指定应用类型")
    }

    let app_type = app_types[0];

    // 获取内容
    let prompt_content = if let Some(c) = content {
        c
    } else if let Some(f) = file {
        fs::read_to_string(&f).map_err(|e| anyhow::anyhow!("读取文件失败: {}", e))?
    } else {
        print_error("请提供 --content 或 --file 参数");
        bail!("需要提供内容")
    };

    // 生成 ID
    let id = ccswitch_core::config::sanitize_name(name);

    let mut prompt = Prompt::new(&id, name, prompt_content);

    if let Some(desc) = description {
        prompt = prompt.with_description(desc);
    }

    PromptService::add(&state, app_type, prompt)?;

    print_success(&format!("已添加 Prompt: {}", name));
    print_info(&format!(
        "使用 'cc-switch prompt enable {} --app {}' 启用",
        id,
        app_type.as_str()
    ));

    Ok(())
}

/// 更新 Prompt
pub fn update(
    _ctx: &OutputContext,
    app: AppTypeArg,
    id: &str,
    name: Option<String>,
    content: Option<String>,
    description: Option<String>,
) -> Result<()> {
    let state = AppState::init()?;

    let app_types = app.to_app_types();
    if app_types.is_empty() || matches!(app, AppTypeArg::All) {
        print_error("请指定具体的应用类型 (claude/codex/gemini/opencode)");
        bail!("需要指定应用类型")
    }

    let app_type = app_types[0];

    let mut prompt = PromptService::get(&state, app_type, id)?
        .ok_or_else(|| anyhow::anyhow!("Prompt '{}' 不存在", id))?;

    if let Some(n) = name {
        prompt.name = n;
    }

    if let Some(c) = content {
        prompt.content = c;
    }

    if let Some(d) = description {
        prompt.description = Some(d);
    }

    prompt.updated_at = Some(chrono::Utc::now().timestamp());

    PromptService::update(&state, app_type, prompt)?;

    print_success(&format!("已更新 Prompt: {}", id));

    Ok(())
}

/// 删除 Prompt
pub fn remove(_ctx: &OutputContext, app: AppTypeArg, id: &str, yes: bool) -> Result<()> {
    let state = AppState::init()?;

    let app_types = app.to_app_types();
    if app_types.is_empty() || matches!(app, AppTypeArg::All) {
        print_error("请指定具体的应用类型 (claude/codex/gemini/opencode)");
        bail!("需要指定应用类型")
    }

    let app_type = app_types[0];

    // 检查是否存在
    if PromptService::get(&state, app_type, id)?.is_none() {
        print_error(&format!("Prompt '{}' 不存在", id));
        bail!("Prompt 不存在")
    }

    // 确认删除
    if !yes {
        print!("确定要删除 Prompt '{}' 吗? [y/N] ", id);
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            print_info("已取消删除");
            return Ok(());
        }
    }

    PromptService::remove(&state, app_type, id)?;

    print_success(&format!("已删除 Prompt: {}", id));

    Ok(())
}

/// 启用 Prompt
pub fn enable(_ctx: &OutputContext, app: AppTypeArg, id: &str) -> Result<()> {
    let state = AppState::init()?;

    let app_types = app.to_app_types();
    if app_types.is_empty() || matches!(app, AppTypeArg::All) {
        print_error("请指定具体的应用类型 (claude/codex/gemini/opencode)");
        bail!("需要指定应用类型")
    }

    let app_type = app_types[0];

    PromptService::enable(&state, app_type, id)?;

    print_success(&format!(
        "已为 {} 启用 Prompt: {}",
        app_type.display_name(),
        id
    ));

    Ok(())
}

/// 从应用导入 Prompt
pub fn import(_ctx: &OutputContext, app: AppTypeArg) -> Result<()> {
    let state = AppState::init()?;

    let apps = match app {
        AppTypeArg::All => AppType::all().to_vec(),
        _ => app.to_app_types(),
    };

    let mut total_imported = 0;

    for app_type in apps {
        match PromptService::import_from_app(&state, app_type) {
            Ok(Some(id)) => {
                print_success(&format!(
                    "从 {} 导入了 Prompt: {}",
                    app_type.display_name(),
                    id
                ));
                total_imported += 1;
            }
            Ok(None) => {
                // 没有内容可导入
            }
            Err(e) => {
                print_warning(&format!("从 {} 导入失败: {}", app_type.display_name(), e));
            }
        }
    }

    if total_imported == 0 {
        print_info("没有新的 Prompt 需要导入");
    }

    Ok(())
}