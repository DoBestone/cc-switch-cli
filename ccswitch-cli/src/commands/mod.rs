//! 命令执行模块
//!
//! 实现各个 CLI 子命令的具体逻辑。

pub mod config;
pub mod env;
pub mod list;
pub mod mcp;
pub mod prompt;
pub mod provider;
pub mod proxy;
pub mod skill;
pub mod speedtest;
pub mod status;

use anyhow::Result;

use crate::cli::{Cli, Commands, EnvAction, McpAction, PromptAction, ProxyAction, SkillAction};
use crate::output::OutputContext;

/// 执行 CLI 命令
pub fn execute(cli: Cli) -> Result<()> {
    let ctx = OutputContext::new(cli.format, cli.no_color);

    // command 现在是 Option，但在 main.rs 中已经处理了 None 的情况
    let command = cli.command.expect("command should be Some when execute is called");

    match command {
        Commands::List { app, detail } => list::list_providers(&ctx, app, detail),
        Commands::Status { app } => status::show_status(&ctx, app),
        Commands::Use { name, app } => provider::switch(&ctx, &name, app),
        Commands::Add {
            name,
            app,
            api_key,
            base_url,
            model,
            small_model,
            from_file,
        } => provider::add(&ctx, &name, app, api_key, base_url, model, small_model, from_file),
        Commands::Remove { name, app, yes } => provider::remove(&ctx, &name, app, yes),
        Commands::Update { app } => provider::update(&ctx, app),
        Commands::Export { format, out, app } => config::export(&ctx, format, out, app),
        Commands::Import { file, app } => config::import(&ctx, &file, app),
        Commands::Config { action } => config::execute(&ctx, action),
        Commands::Mcp { action } => execute_mcp(&ctx, action),
        Commands::Prompt { action } => execute_prompt(&ctx, action),
        Commands::Proxy { action } => execute_proxy(&ctx, action),
        Commands::Speedtest {
            urls,
            timeout,
            proxy,
        } => execute_speedtest(&ctx, urls, timeout, proxy),
        Commands::Env { action } => execute_env(&ctx, action),
        Commands::Skill { action } => execute_skill(&ctx, action),
        Commands::Version => {
            println!("cc-switch {}", ccswitch_core::VERSION);
            Ok(())
        }
    }
}

/// 执行 MCP 子命令
fn execute_mcp(ctx: &OutputContext, action: McpAction) -> Result<()> {
    match action {
        McpAction::List { app, detail } => mcp::list(ctx, app, detail),
        McpAction::Add {
            id,
            command,
            args,
            env,
            name,
            description,
        } => mcp::add(ctx, &id, &command, args, env, name, description),
        McpAction::Update {
            id,
            name,
            command,
            args,
            description,
        } => mcp::update(ctx, &id, name, command, args, description),
        McpAction::Remove { id, yes } => mcp::remove(ctx, &id, yes),
        McpAction::Toggle {
            id,
            app,
            enable,
            disable,
        } => {
            let enable_flag = if enable {
                true
            } else if disable {
                false
            } else {
                true
            };
            mcp::toggle(ctx, &id, app, enable_flag)
        }
        McpAction::Import { from } => mcp::import(ctx, from),
        McpAction::Show { id } => mcp::show(ctx, &id),
    }
}

/// 执行 Prompt 子命令
fn execute_prompt(ctx: &OutputContext, action: PromptAction) -> Result<()> {
    match action {
        PromptAction::List { app } => prompt::list(ctx, app),
        PromptAction::Add {
            name,
            app,
            content,
            file,
            description,
        } => prompt::add(ctx, app, &name, content, file, description),
        PromptAction::Update {
            id,
            app,
            name,
            content,
            description,
        } => prompt::update(ctx, app, &id, name, content, description),
        PromptAction::Remove { id, app, yes } => prompt::remove(ctx, app, &id, yes),
        PromptAction::Enable { id, app } => prompt::enable(ctx, app, &id),
        PromptAction::Show { id, app } => prompt::show(ctx, app, &id),
        PromptAction::Import { app } => prompt::import(ctx, app),
    }
}

/// 执行 Proxy 子命令
fn execute_proxy(ctx: &OutputContext, action: ProxyAction) -> Result<()> {
    match action {
        ProxyAction::Get => proxy::get(ctx),
        ProxyAction::Set { url } => proxy::set(ctx, &url),
        ProxyAction::Clear => proxy::clear(ctx),
        ProxyAction::Test { url } => {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(proxy::test(ctx, url))
        }
        ProxyAction::Scan => {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(proxy::scan(ctx))
        }
    }
}

/// 执行 Speedtest 命令
fn execute_speedtest(ctx: &OutputContext, urls: Vec<String>, timeout: u64, use_proxy: bool) -> Result<()> {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(speedtest::test(ctx, urls, timeout, use_proxy))
}

/// 执行 Env 子命令
fn execute_env(ctx: &OutputContext, action: EnvAction) -> Result<()> {
    match action {
        EnvAction::Check { app } => env::check(ctx, app),
        EnvAction::List { app } => env::list(ctx, app),
    }
}

/// 执行 Skill 子命令
fn execute_skill(ctx: &OutputContext, action: SkillAction) -> Result<()> {
    match action {
        SkillAction::List { app, detail } => skill::list(ctx, app, detail),
        SkillAction::Install { repo, branch, app } => skill::install(ctx, &repo, branch, app),
        SkillAction::Uninstall { id, yes } => skill::uninstall(ctx, &id, yes),
        SkillAction::Toggle {
            id,
            app,
            enable,
            disable,
        } => {
            let enable_flag = if enable {
                true
            } else if disable {
                false
            } else {
                true
            };
            skill::toggle(ctx, &id, app, enable_flag)
        }
        SkillAction::Scan => skill::scan(ctx),
        SkillAction::Sync => skill::sync(ctx),
        SkillAction::Show { id } => skill::show(ctx, &id),
    }
}
