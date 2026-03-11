//! 故障转移命令
//!
//! 管理供应商的故障转移队列。

use anyhow::Result;
use ccswitch_core::{AppType, AppState, FailoverService};
use crate::output::{print_success, print_info};
use crate::cli::OutputFormat;
use crate::output::OutputContext;

/// 列出故障转移队列
pub fn list(ctx: &OutputContext, app: AppType) -> Result<()> {
    let state = AppState::init()?;
    let queue = FailoverService::get_queue(&state, app)?;

    if queue.is_empty() {
        print_success("故障转移队列为空");
        return Ok(());
    }

    match ctx.format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&queue)?;
            println!("{}", json);
        }
        _ => {
            print_info(&format!("故障转移队列 ({} 个供应商):", queue.len()));
            for (i, item) in queue.iter().enumerate() {
                println!("  {}. {} ({})", i + 1, item.provider_name, item.provider_id);
            }
        }
    }

    Ok(())
}

/// 添加到故障转移队列
pub fn add(_ctx: &OutputContext, app: AppType, provider_id: &str) -> Result<()> {
    let state = AppState::init()?;
    FailoverService::add_to_queue(&state, app, provider_id)?;
    print_success(&format!("已将 '{}' 添加到故障转移队列", provider_id));
    Ok(())
}

/// 从故障转移队列移除
pub fn remove(_ctx: &OutputContext, app: AppType, provider_id: &str) -> Result<()> {
    let state = AppState::init()?;
    FailoverService::remove_from_queue(&state, app, provider_id)?;
    print_success(&format!("已将 '{}' 从故障转移队列移除", provider_id));
    Ok(())
}

/// 清空故障转移队列
pub fn clear(_ctx: &OutputContext, app: AppType) -> Result<()> {
    let state = AppState::init()?;
    FailoverService::clear_queue(&state, app)?;
    print_success("已清空故障转移队列");
    Ok(())
}