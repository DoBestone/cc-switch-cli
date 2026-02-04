//! CC-Switch CLI
//!
//! 纯命令行工具，用于管理 Claude Code、Codex、Gemini CLI 配置。
//! 支持在 Linux 服务器（无图形界面）上通过 SSH 直接操作。
//!
//! # 使用示例
//!
//! ```bash
//! # 进入交互式菜单（推荐新手）
//! cc-switch
//!
//! # 列出所有供应商
//! cc-switch list
//!
//! # 显示当前状态
//! cc-switch status
//!
//! # 切换供应商
//! cc-switch use my-provider --app claude
//! ```

mod cli;
mod commands;
mod interactive;
mod output;
mod tui;

use anyhow::Result;
use clap::Parser;

use cli::Cli;
use commands::execute;

fn main() -> Result<()> {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .format_timestamp(None)
        .init();

    // 解析命令行参数
    let cli = Cli::parse();

    // 如果没有子命令，检查是否启用 TUI 模式
    if cli.command.is_none() {
        if cli.tui {
            return tui::run_tui();
        } else {
            return interactive::main_menu();
        }
    }

    // 执行命令
    execute(cli)
}
