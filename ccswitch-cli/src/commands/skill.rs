//! Skill 命令实现模块
//!
//! 实现 Skills 管理相关的 CLI 命令。

use anyhow::Result;
use colored::Colorize;
use std::io::{self, Write};

use ccswitch_core::{AppState, AppType, Skill, SkillService};

use crate::cli::AppTypeArg;
use crate::output::{print_info, print_success, OutputContext};

/// 列出所有 Skills
pub fn list(_ctx: &OutputContext, app: AppTypeArg, detail: bool) -> Result<()> {
    let state = AppState::init()?;
    let skills = SkillService::list(&state)?;

    if skills.is_empty() {
        print_info("暂无已安装的 Skills");
        print_info("使用 'cc-switch skill install owner/repo' 安装 Skill");
        return Ok(());
    }

    // 根据应用筛选
    let app_types = app.to_app_types();
    let filtered: Vec<_> = if app_types.len() == 4 {
        // All apps
        skills.values().collect()
    } else {
        skills
            .values()
            .filter(|s| app_types.iter().any(|a| s.apps.is_enabled_for(a)))
            .collect()
    };

    if filtered.is_empty() {
        print_info("没有符合条件的 Skills");
        return Ok(());
    }

    println!("{}", format!("Skills 列表 (共 {} 个)", filtered.len()).bold());
    println!();

    for skill in filtered {
        print_skill_summary(skill, detail);
    }

    Ok(())
}

/// 打印 Skill 摘要
fn print_skill_summary(skill: &Skill, detail: bool) {
    let enabled = skill.enabled_apps_str();
    println!(
        "  {} - {} [{}]",
        skill.id.cyan(),
        skill.name,
        enabled.dimmed()
    );

    if detail {
        if let Some(desc) = &skill.description {
            println!("    {}: {}", "描述".dimmed(), desc);
        }
        println!("    {}: {}", "目录".dimmed(), skill.directory);
        if let Some(url) = skill.repo_url() {
            println!("    {}: {}", "仓库".dimmed(), url);
        }
        println!();
    }
}

/// 安装 Skill
pub fn install(
    _ctx: &OutputContext,
    repo: &str,
    branch: Option<String>,
    app: Option<AppTypeArg>,
) -> Result<()> {
    let state = AppState::init()?;

    print_info(&format!("正在从 {} 安装 Skill...", repo));

    let skill = SkillService::install(&state, repo, branch)?;

    print_success(&format!("Skill '{}' 安装成功", skill.id));
    println!("  {}: {}", "目录".dimmed(), skill.directory);

    // 如果指定了应用，自动启用
    if let Some(app_arg) = app {
        for app_type in app_arg.to_app_types() {
            SkillService::toggle(&state, &skill.id, app_type.clone(), true)?;
            print_success(&format!("已为 {} 启用", app_type.display_name()));
        }
    }

    Ok(())
}

/// 卸载 Skill
pub fn uninstall(_ctx: &OutputContext, id: &str, yes: bool) -> Result<()> {
    let state = AppState::init()?;

    // 检查是否存在
    let skill = SkillService::get(&state, id)?
        .ok_or_else(|| anyhow::anyhow!("Skill '{}' 不存在", id))?;

    // 确认删除
    if !yes {
        print!("确定要卸载 Skill '{}' 吗? [y/N] ", skill.name);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            print_info("已取消");
            return Ok(());
        }
    }

    SkillService::uninstall(&state, id)?;
    print_success(&format!("Skill '{}' 已卸载", id));

    Ok(())
}

/// 切换 Skill 启用状态
pub fn toggle(_ctx: &OutputContext, id: &str, app: AppTypeArg, enable: bool) -> Result<()> {
    let state = AppState::init()?;

    for app_type in app.to_app_types() {
        SkillService::toggle(&state, id, app_type.clone(), enable)?;

        let action = if enable { "启用" } else { "禁用" };
        print_success(&format!(
            "已为 {} {} Skill '{}'",
            app_type.display_name(),
            action,
            id
        ));
    }

    Ok(())
}

/// 扫描本地 Skills
pub fn scan(_ctx: &OutputContext) -> Result<()> {
    let state = AppState::init()?;

    print_info("正在扫描本地 Skills 目录...");

    let found = SkillService::scan(&state)?;

    if found.is_empty() {
        print_info("未发现新的 Skills");
    } else {
        print_success(&format!("发现 {} 个新 Skills:", found.len()));
        for id in found {
            println!("  - {}", id.cyan());
        }
    }

    Ok(())
}

/// 同步 Skills 到所有应用
pub fn sync(_ctx: &OutputContext) -> Result<()> {
    let state = AppState::init()?;

    print_info("正在同步 Skills 到所有应用...");

    SkillService::sync_all(&state)?;

    print_success("Skills 同步完成");

    Ok(())
}

/// 显示 Skill 详情
pub fn show(_ctx: &OutputContext, id: &str) -> Result<()> {
    let state = AppState::init()?;

    let skill = SkillService::get(&state, id)?
        .ok_or_else(|| anyhow::anyhow!("Skill '{}' 不存在", id))?;

    println!("{}", format!("Skill: {}", skill.name).bold());
    println!("{}: {}", "ID".dimmed(), skill.id);

    if let Some(desc) = &skill.description {
        println!("{}: {}", "描述".dimmed(), desc);
    }

    println!("{}: {}", "目录".dimmed(), skill.directory);

    if let Some(url) = skill.repo_url() {
        println!("{}: {}", "仓库".dimmed(), url);
    }

    if let Some(branch) = &skill.repo_branch {
        println!("{}: {}", "分支".dimmed(), branch);
    }

    println!("{}: {}", "启用应用".dimmed(), skill.enabled_apps_str());

    if let Some(ts) = skill.installed_at {
        let dt = chrono::DateTime::from_timestamp(ts, 0)
            .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "未知".to_string());
        println!("{}: {}", "安装时间".dimmed(), dt);
    }

    Ok(())
}
