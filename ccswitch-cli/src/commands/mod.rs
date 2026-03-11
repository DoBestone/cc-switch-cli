//! 命令执行模块
//!
//! 实现各个 CLI 子命令的具体逻辑。

pub mod batch;
pub mod config;
pub mod env;
pub mod failover;
pub mod list;
pub mod mcp;
pub mod openclaw;
pub mod prompt;
pub mod provider;
pub mod proxy;
pub mod skill;
pub mod speedtest;
pub mod status;
pub mod update;
pub mod usage;
pub mod webdav;

use anyhow::Result;

use crate::cli::{Cli, Commands, BatchAction, EnvAction, FailoverAction, McpAction, OpenclawAction, PromptAction, ProxyAction, SkillAction, SelfUpdateAction, UsageAction, WebdavAction};
use crate::output::OutputContext;

/// 执行 CLI 命令
pub fn execute(cli: Cli) -> Result<()> {
    let ctx = OutputContext::new(cli.format, cli.no_color);

    // command 现在是 Option，但在 main.rs 中已经处理了 None 的情况
    let command = cli.command.expect("command should be Some when execute is called");

    match command {
        Commands::List { app, detail, show_key } => list::list_providers(&ctx, app, detail, show_key),
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
            skip_test,
        } => provider::add(&ctx, &name, app, api_key, base_url, model, small_model, from_file, skip_test),
        Commands::Edit {
            name,
            app,
            api_key,
            base_url,
            model,
            small_model,
            new_name,
        } => provider::edit(&ctx, &name, app, api_key, base_url, model, small_model, new_name),
        Commands::Test {
            name,
            app,
            api_key,
            base_url,
            model,
            timeout,
        } => execute_test(&ctx, name, app, api_key, base_url, model, timeout),
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
        Commands::Openclaw { action } => execute_openclaw(&ctx, action),
        Commands::SelfUpdate { action, check, force } => execute_self_update(&ctx, action, check, force),
        Commands::Batch { action } => execute_batch(&ctx, action),
        Commands::Failover { action } => execute_failover(&ctx, action),
        Commands::Usage { action } => execute_usage(&ctx, action),
        Commands::Webdav { action } => execute_webdav(&ctx, action),
        Commands::Web { port, host, user, pass } => execute_web(&ctx, port, &host, &user, &pass),
        Commands::Version => {
            println!("cc-switch {}", ccswitch_core::VERSION);
            Ok(())
        }
    }
}

/// 执行批量操作子命令
fn execute_batch(ctx: &OutputContext, action: BatchAction) -> Result<()> {
    match action {
        BatchAction::Switch { name } => batch::batch_switch(ctx, &name),
        BatchAction::Test { app, timeout, verbose } => {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(batch::batch_test(ctx, app, timeout, verbose))
        }
        BatchAction::Export { output, app } => batch::batch_export(ctx, &output, app),
        BatchAction::Import { input, overwrite } => batch::batch_import(ctx, &input, overwrite),
        BatchAction::Remove { names, app, force } => batch::batch_remove(ctx, &names, app, force),
        BatchAction::Sync { from, to, overwrite } => {
            // 处理 from 和 to
            let from_app = from.to_app_types().into_iter().next()
                .ok_or_else(|| anyhow::anyhow!("源应用类型无效"))?;

            let mut target_apps = Vec::new();
            for t in to {
                target_apps.extend(t.to_app_types());
            }

            // 去重
            target_apps.sort();
            target_apps.dedup();

            // 移除源应用（不能同步到自己）
            target_apps.retain(|app| *app != from_app);

            if target_apps.is_empty() {
                anyhow::bail!("没有有效的目标应用");
            }

            batch::batch_sync(ctx, from_app, target_apps, overwrite)
        }
        BatchAction::Edit { field, value, app, pattern } => {
            batch::batch_edit(ctx, app, &field, &value, pattern.as_deref())
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

/// 执行 API Test 命令
fn execute_test(
    ctx: &OutputContext,
    name: Option<String>,
    app: crate::cli::AppTypeArg,
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    timeout: u64,
) -> Result<()> {
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(provider::test_api(ctx, name, app, api_key, base_url, model, timeout))
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

/// 执行 SelfUpdate 子命令
fn execute_self_update(
    ctx: &OutputContext,
    action: Option<SelfUpdateAction>,
    check: bool,
    force: bool,
) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().unwrap();

    match action {
        Some(SelfUpdateAction::Check) => {
            rt.block_on(update::show_status(ctx, true))
        }
        Some(SelfUpdateAction::Run { force }) => {
            rt.block_on(update::self_update(ctx, force))
        }
        None => {
            if check {
                rt.block_on(update::show_status(ctx, true))
            } else {
                rt.block_on(update::self_update(ctx, force))
            }
        }
    }
}

/// 执行 Openclaw 子命令
fn execute_openclaw(ctx: &OutputContext, action: OpenclawAction) -> Result<()> {
    match action {
        OpenclawAction::List { detail } => openclaw::list_providers(ctx, detail),
        OpenclawAction::Add { id, base_url, api_key, api, models } => {
            openclaw::add_provider(ctx, &id, base_url, api_key, api, models)
        }
        OpenclawAction::Update { id, base_url, api_key, api, add_models, remove_models } => {
            openclaw::update_provider(ctx, &id, base_url, api_key, api, add_models, remove_models)
        }
        OpenclawAction::Remove { id, yes } => openclaw::remove_provider(ctx, &id, yes),
        OpenclawAction::Show { id } => openclaw::show_provider(ctx, &id),
        OpenclawAction::DefaultModel { primary, fallbacks } => {
            if primary.is_none() && fallbacks.is_empty() {
                openclaw::get_default_model_cmd(ctx)
            } else {
                openclaw::set_default_model_cmd(ctx, primary, fallbacks)
            }
        }
        OpenclawAction::Agents { model, timeout } => {
            if model.is_none() && timeout.is_none() {
                openclaw::get_agents_config(ctx)
            } else {
                openclaw::set_agents_config(ctx, model, timeout)
            }
        }
        OpenclawAction::Env { key, value, remove } => {
            match (key, value, remove) {
                (Some(k), Some(v), None) => openclaw::set_env_config_cmd(ctx, &k, &v),
                (Some(k), None, None) => openclaw::remove_env_config(ctx, &k),
                (None, Some(_v), None) => {
                    anyhow::bail!("必须指定 --key 来设置环境变量")
                }
                (None, None, Some(k)) => openclaw::remove_env_config(ctx, &k),
                (None, None, None) => openclaw::get_env_config_cmd(ctx),
                _ => anyhow::bail!("参数冲突"),
            }
        }
        OpenclawAction::Tools { profile, add_allow, remove_allow, add_deny, remove_deny } => {
            if profile.is_none() && add_allow.is_empty() && remove_allow.is_empty()
                && add_deny.is_empty() && remove_deny.is_empty() {
                openclaw::get_tools_config_cmd(ctx)
            } else {
                openclaw::set_tools_config_cmd(ctx, profile, add_allow, remove_allow, add_deny, remove_deny)
            }
        }
        OpenclawAction::Catalog { add, remove } => {
            match (add, remove) {
                (Some(model_spec), None) => {
                    let parts: Vec<&str> = model_spec.splitn(2, ':').collect();
                    let model_id = parts[0];
                    let alias = parts.get(1).map(|s| s.to_string());
                    openclaw::add_model_to_catalog_cmd(ctx, model_id, alias)
                }
                (None, Some(model_id)) => openclaw::remove_model_from_catalog_cmd(ctx, &model_id),
                (None, None) => openclaw::get_model_catalog_cmd(ctx),
                (Some(_), Some(_)) => anyhow::bail!("不能同时指定 --add 和 --remove"),
            }
        }
        OpenclawAction::Health { fix } => openclaw::health_check(ctx, fix),
        OpenclawAction::Path => openclaw::show_config_path(ctx),
        OpenclawAction::Export => openclaw::export_config(ctx),
        OpenclawAction::Import { file } => openclaw::import_config(ctx, &file),
    }
}

/// 执行 Failover 子命令
fn execute_failover(ctx: &OutputContext, action: FailoverAction) -> Result<()> {
    match action {
        FailoverAction::List { app } => {
            let app_type = app.to_app_types().into_iter().next()
                .ok_or_else(|| anyhow::anyhow!("无效的应用类型"))?;
            failover::list(ctx, app_type)
        }
        FailoverAction::Add { provider_id, app } => {
            let app_type = app.to_app_types().into_iter().next()
                .ok_or_else(|| anyhow::anyhow!("无效的应用类型"))?;
            failover::add(ctx, app_type, &provider_id)
        }
        FailoverAction::Remove { provider_id, app } => {
            let app_type = app.to_app_types().into_iter().next()
                .ok_or_else(|| anyhow::anyhow!("无效的应用类型"))?;
            failover::remove(ctx, app_type, &provider_id)
        }
        FailoverAction::Clear { app } => {
            let app_type = app.to_app_types().into_iter().next()
                .ok_or_else(|| anyhow::anyhow!("无效的应用类型"))?;
            failover::clear(ctx, app_type)
        }
    }
}

/// 执行 Usage 子命令
fn execute_usage(ctx: &OutputContext, action: UsageAction) -> Result<()> {
    match action {
        UsageAction::Summary { days } => usage::summary(ctx, days),
        UsageAction::Trends { days } => usage::trends(ctx, days),
        UsageAction::Provider => usage::provider_stats(ctx),
        UsageAction::Limit { provider_id, app } => {
            let app_type = app.to_app_types().into_iter().next()
                .ok_or_else(|| anyhow::anyhow!("无效的应用类型"))?;
            usage::check_limit(ctx, app_type, &provider_id)
        }
        UsageAction::SetLimit { provider_id, daily, monthly } => {
            usage::set_limit(ctx, &provider_id, daily, monthly)
        }
    }
}

/// 执行 Webdav 子命令
fn execute_webdav(ctx: &OutputContext, action: WebdavAction) -> Result<()> {
    match action {
        WebdavAction::Config => webdav::show_config(ctx),
        WebdavAction::Setup { url, username, password, remote_root } => {
            webdav::configure(&url, &username, &password, remote_root.as_deref())
        }
        WebdavAction::Toggle { enable, disable } => {
            let enable_flag = if enable {
                true
            } else if disable {
                false
            } else {
                true
            };
            webdav::toggle(enable_flag)
        }
        WebdavAction::Test => webdav::test(),
        WebdavAction::Upload => webdav::upload(ctx),
        WebdavAction::Download => webdav::download(ctx),
        WebdavAction::Info => webdav::remote_info(ctx),
    }
}

/// 执行 Web 控制器命令
fn execute_web(_ctx: &OutputContext, port: u16, host: &str, user: &str, pass: &str) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        // 创建路由
        let app = crate::web::create_router(user, pass);

        let addr: std::net::SocketAddr = format!("{}:{}", host, port)
            .parse()
            .map_err(|e| anyhow::anyhow!("无效的地址: {}", e))?;

        println!();
        println!("╔══════════════════════════════════════════════════════════════╗");
        println!("║           🌐 CC-Switch Web 控制器已启动                      ║");
        println!("╠══════════════════════════════════════════════════════════════╣");
        println!("║  访问地址: http://{}:{}                                   ║", host, port);
        println!("║  登录账号: {}                                               ║", user);
        println!("║                                                              ║");
        println!("║  ⚠️  安全提示:                                               ║");
        println!("║  • 此服务绑定所有网络接口，可从公网访问                       ║");
        println!("║  • 已启用身份验证，请使用设置的账号密码登录                   ║");
        println!("║  • 配置完成后请及时关闭 (Ctrl+C)                             ║");
        println!("║  • 建议在防火墙后使用或使用临时会话                          ║");
        println!("╚══════════════════════════════════════════════════════════════╝");
        println!();

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    })
}
