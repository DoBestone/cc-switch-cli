//! MCP 服务器命令模块
//!
//! 实现 MCP 服务器管理的 CLI 命令。

use anyhow::{bail, Result};
use ccswitch_core::{AppState, AppType, McpServer, McpService, McpStdioConfig};
use serde_json::json;

use crate::cli::AppTypeArg;
use crate::output::{print_error, print_info, print_success, print_warning, OutputContext};

/// 列出所有 MCP 服务器
pub fn list(_ctx: &OutputContext, app: AppTypeArg, detail: bool) -> Result<()> {
    let state = AppState::init()?;
    let servers = McpService::list(&state)?;

    if servers.is_empty() {
        print_info("暂无 MCP 服务器配置");
        print_info("使用 'cc-switch mcp add <id> --command <cmd>' 添加");
        return Ok(());
    }

    let app_types = app.to_app_types();

    // 筛选服务器
    let filtered: Vec<_> = if matches!(app, AppTypeArg::All) {
        servers.values().collect()
    } else {
        servers
            .values()
            .filter(|s| app_types.iter().any(|a| s.apps.is_enabled_for(a)))
            .collect()
    };

    if filtered.is_empty() {
        print_info(&format!("没有为 {:?} 启用的 MCP 服务器", app));
        return Ok(());
    }

    println!("\n📦 MCP 服务器列表 ({} 个)\n", filtered.len());
    println!("{:<20} {:<20} {:<30}", "ID", "名称", "启用的应用");
    println!("{}", "-".repeat(70));

    for server in &filtered {
        let apps_str = server.enabled_apps_str();
        println!("{:<20} {:<20} {:<30}", server.id, server.name, apps_str);

        if detail {
            if let Some(desc) = &server.description {
                println!("  描述: {}", desc);
            }
            if let Some(cmd) = server.server_config.get("command") {
                println!("  命令: {}", cmd);
            }
            if let Some(args) = server.server_config.get("args") {
                if let Some(arr) = args.as_array() {
                    if !arr.is_empty() {
                        let args_str: Vec<_> = arr.iter().filter_map(|v| v.as_str()).collect();
                        println!("  参数: {}", args_str.join(" "));
                    }
                }
            }
            println!();
        }
    }

    if !detail {
        println!("\n💡 使用 --detail 查看详细配置");
    }

    Ok(())
}

/// 显示单个 MCP 服务器详情
pub fn show(_ctx: &OutputContext, id: &str) -> Result<()> {
    let state = AppState::init()?;
    let server = McpService::get(&state, id)?;

    match server {
        Some(s) => {
            println!("\n📦 MCP 服务器: {}\n", s.name);
            println!("ID:       {}", s.id);
            println!("名称:     {}", s.name);
            println!("启用应用: {}", s.enabled_apps_str());

            if let Some(desc) = &s.description {
                println!("描述:     {}", desc);
            }
            if let Some(homepage) = &s.homepage {
                println!("主页:     {}", homepage);
            }
            if !s.tags.is_empty() {
                println!("标签:     {}", s.tags.join(", "));
            }

            println!("\n配置:");
            let config_str = serde_json::to_string_pretty(&s.server_config)?;
            println!("{}", config_str);

            Ok(())
        }
        None => {
            print_error(&format!("MCP 服务器 '{}' 不存在", id));
            bail!("服务器不存在")
        }
    }
}

/// 添加 MCP 服务器
pub fn add(
    _ctx: &OutputContext,
    id: &str,
    command: &str,
    args: Vec<String>,
    env: Vec<String>,
    name: Option<String>,
    description: Option<String>,
) -> Result<()> {
    let state = AppState::init()?;

    // 构建服务器配置
    let mut config = McpStdioConfig::new(command).with_args(args);

    // 解析环境变量
    for env_str in env {
        if let Some((key, value)) = env_str.split_once('=') {
            config = config.with_env(key, value);
        } else {
            print_warning(&format!("忽略无效的环境变量格式: {}", env_str));
        }
    }

    let display_name = name.unwrap_or_else(|| id.to_string());
    let mut server = McpServer::new(id, &display_name, config.to_json());

    if let Some(desc) = description {
        server = server.with_description(desc);
    }

    McpService::add(&state, server)?;

    print_success(&format!("已添加 MCP 服务器: {}", display_name));
    print_info("使用 'cc-switch mcp toggle <id> --app <app> --enable' 启用");

    Ok(())
}

/// 更新 MCP 服务器
pub fn update(
    _ctx: &OutputContext,
    id: &str,
    name: Option<String>,
    command: Option<String>,
    args: Option<Vec<String>>,
    description: Option<String>,
) -> Result<()> {
    let state = AppState::init()?;

    let mut server = McpService::get(&state, id)?
        .ok_or_else(|| anyhow::anyhow!("MCP 服务器 '{}' 不存在", id))?;

    if let Some(n) = name {
        server.name = n;
    }

    if let Some(cmd) = command {
        if let Some(obj) = server.server_config.as_object_mut() {
            obj.insert("command".to_string(), json!(cmd));
        }
    }

    if let Some(a) = args {
        if let Some(obj) = server.server_config.as_object_mut() {
            obj.insert("args".to_string(), json!(a));
        }
    }

    if let Some(desc) = description {
        server.description = Some(desc);
    }

    McpService::update(&state, server)?;

    print_success(&format!("已更新 MCP 服务器: {}", id));

    Ok(())
}

/// 删除 MCP 服务器
pub fn remove(_ctx: &OutputContext, id: &str, yes: bool) -> Result<()> {
    let state = AppState::init()?;

    // 检查是否存在
    let server = McpService::get(&state, id)?;
    if server.is_none() {
        print_error(&format!("MCP 服务器 '{}' 不存在", id));
        bail!("服务器不存在")
    }

    // 确认删除
    if !yes {
        print!("确定要删除 MCP 服务器 '{}' 吗? [y/N] ", id);
        std::io::Write::flush(&mut std::io::stdout())?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            print_info("已取消删除");
            return Ok(());
        }
    }

    McpService::remove(&state, id)?;

    print_success(&format!("已删除 MCP 服务器: {}", id));

    Ok(())
}

/// 切换 MCP 服务器的应用启用状态
pub fn toggle(_ctx: &OutputContext, id: &str, app: AppTypeArg, enable: bool) -> Result<()> {
    let state = AppState::init()?;

    let app_types = app.to_app_types();
    if app_types.is_empty() || matches!(app, AppTypeArg::All) {
        print_error("请指定具体的应用类型 (claude/codex/gemini/opencode)");
        bail!("需要指定应用类型")
    }

    let app_type = app_types[0];

    McpService::toggle(&state, id, app_type, enable)?;

    let action = if enable { "启用" } else { "禁用" };
    print_success(&format!(
        "已为 {} {} MCP 服务器: {}",
        app_type.display_name(),
        action,
        id
    ));

    Ok(())
}

/// 从应用导入 MCP 服务器
pub fn import(_ctx: &OutputContext, app: Option<AppTypeArg>) -> Result<()> {
    let state = AppState::init()?;

    let apps = match app {
        Some(a) => a.to_app_types(),
        None => AppType::all().to_vec(),
    };

    let mut total_imported = 0;

    for app_type in apps {
        match McpService::import_from_app(&state, app_type) {
            Ok(imported) => {
                if !imported.is_empty() {
                    print_success(&format!(
                        "从 {} 导入了 {} 个 MCP 服务器: {}",
                        app_type.display_name(),
                        imported.len(),
                        imported.join(", ")
                    ));
                    total_imported += imported.len();
                }
            }
            Err(e) => {
                print_warning(&format!("从 {} 导入失败: {}", app_type.display_name(), e));
            }
        }
    }

    if total_imported == 0 {
        print_info("没有新的 MCP 服务器需要导入");
    }

    Ok(())
}