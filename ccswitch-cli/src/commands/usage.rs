//! 使用统计命令
//!
//! 显示 API 使用量和统计信息。

use anyhow::Result;
use ccswitch_core::{AppType, AppState, UsageStatsService};
use crate::output::{print_success, print_info};
use crate::cli::OutputFormat;
use crate::output::OutputContext;

/// 显示使用量汇总
pub fn summary(ctx: &OutputContext, days: Option<u64>) -> Result<()> {
    let state = AppState::init()?;

    let (start_date, end_date) = if let Some(d) = days {
        let end = chrono::Utc::now().timestamp();
        let start = end - (d as i64 * 24 * 60 * 60);
        (Some(start), Some(end))
    } else {
        (None, None)
    };

    let summary = UsageStatsService::get_summary(&state, start_date, end_date)?;

    match ctx.format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&summary)?;
            println!("{}", json);
        }
        _ => {
            print_info("使用量汇总:");
            println!("  总请求数: {}", summary.total_requests);
            println!("  总 Tokens: {}", summary.total_tokens);
            println!("  输入 Tokens: {}", summary.input_tokens);
            println!("  输出 Tokens: {}", summary.output_tokens);
            println!("  总费用: ${:.4}", summary.total_cost);
        }
    }

    Ok(())
}

/// 显示每日趋势
pub fn trends(ctx: &OutputContext, days: u64) -> Result<()> {
    let state = AppState::init()?;

    let end_date = chrono::Utc::now().timestamp();
    let start_date = end_date - (days as i64 * 24 * 60 * 60);

    let trends = UsageStatsService::get_daily_trends(&state, Some(start_date), Some(end_date))?;

    if trends.is_empty() {
        print_success("暂无使用记录");
        return Ok(());
    }

    match ctx.format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&trends)?;
            println!("{}", json);
        }
        _ => {
            print_info(&format!("每日趋势 (最近 {} 天):", days));
            println!("{:<12} {:>10} {:>12} {:>12} {:>10}",
                "日期", "请求数", "输入Tokens", "输出Tokens", "费用");
            println!("{}", "-".repeat(60));
            for t in trends {
                println!("{:<12} {:>10} {:>12} {:>12} ${:>8.4}",
                    t.date, t.requests, t.input_tokens, t.output_tokens, t.cost);
            }
        }
    }

    Ok(())
}

/// 显示供应商统计
pub fn provider_stats(ctx: &OutputContext) -> Result<()> {
    let state = AppState::init()?;
    let stats = UsageStatsService::get_provider_stats(&state)?;

    if stats.is_empty() {
        print_success("暂无供应商使用记录");
        return Ok(());
    }

    match ctx.format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&stats)?;
            println!("{}", json);
        }
        _ => {
            print_info("供应商统计:");
            println!("{:<20} {:>10} {:>12} {:>10}",
                "供应商", "请求数", "Tokens", "费用");
            println!("{}", "-".repeat(55));
            for s in stats {
                println!("{:<20} {:>10} {:>12} ${:>8.4}",
                    s.provider_name, s.requests, s.tokens, s.cost);
            }
        }
    }

    Ok(())
}

/// 检查限额
pub fn check_limit(ctx: &OutputContext, app: AppType, provider_id: &str) -> Result<()> {
    let state = AppState::init()?;
    let status = UsageStatsService::check_limits(&state, provider_id, app.as_str())?;

    match ctx.format {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&status)?;
            println!("{}", json);
        }
        _ => {
            print_info(&format!("供应商 '{}' 限额状态:", provider_id));
            if let Some(daily) = status.daily_limit {
                println!("  日限额: ${:.2} / 已使用: ${:.2}", daily, status.daily_used);
            }
            if let Some(monthly) = status.monthly_limit {
                println!("  月限额: ${:.2} / 已使用: ${:.2}", monthly, status.monthly_used);
            }
            if status.is_exceeded {
                println!("  状态: ⚠️ 已超限");
            } else {
                println!("  状态: ✓ 正常");
            }
        }
    }

    Ok(())
}

/// 设置限额
pub fn set_limit(
    _ctx: &OutputContext,
    provider_id: &str,
    daily: Option<f64>,
    monthly: Option<f64>,
) -> Result<()> {
    let state = AppState::init()?;

    if let Some(d) = daily {
        UsageStatsService::set_daily_limit(&state, provider_id, d)?;
        println!("已设置日限额: ${:.2}", d);
    }

    if let Some(m) = monthly {
        UsageStatsService::set_monthly_limit(&state, provider_id, m)?;
        println!("已设置月限额: ${:.2}", m);
    }

    print_success("限额设置完成");
    Ok(())
}