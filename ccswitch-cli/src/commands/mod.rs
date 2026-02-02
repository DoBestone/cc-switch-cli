//! 命令执行模块
//!
//! 实现各个 CLI 子命令的具体逻辑。

pub mod config;
pub mod list;
pub mod provider;
pub mod status;

use anyhow::Result;

use crate::cli::{Cli, Commands};
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
        Commands::Version => {
            println!("cc-switch {}", ccswitch_core::VERSION);
            Ok(())
        }
    }
}
