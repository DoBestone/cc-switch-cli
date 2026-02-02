//! 测速命令模块
//!
//! 实现端点测速的 CLI 命令。

use anyhow::Result;
use ccswitch_core::{AppState, ProxyService, SpeedtestService};

use crate::output::{print_info, OutputContext};

/// 测试端点延迟
pub async fn test(
    _ctx: &OutputContext,
    urls: Vec<String>,
    timeout: u64,
    use_proxy: bool,
) -> Result<()> {
    let state = AppState::init()?;

    // 获取代理设置
    let proxy = if use_proxy {
        ProxyService::get(&state)?
    } else {
        None
    };

    // 使用提供的 URL 或默认目标
    let targets = if urls.is_empty() {
        None
    } else {
        Some(urls)
    };

    println!("\n⚡ 端点测速\n");

    if let Some(ref p) = proxy {
        println!("使用代理: {}\n", p);
    }

    let results = SpeedtestService::test_endpoints(targets, timeout, proxy.as_deref()).await;

    // 按延迟排序
    let mut sorted_results = results.clone();
    sorted_results.sort_by(|a, b| {
        match (a.latency_ms, b.latency_ms) {
            (Some(a_ms), Some(b_ms)) => a_ms.cmp(&b_ms),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });

    println!("{:<45} {:<10} {:<10}", "端点", "状态", "延迟");
    println!("{}", "-".repeat(65));

    for result in &sorted_results {
        let status = if result.success { "✓ 成功" } else { "✗ 失败" };
        let latency = result
            .latency_ms
            .map(|ms| format!("{}ms", ms))
            .unwrap_or_else(|| "-".to_string());

        println!("{:<45} {:<10} {:<10}", result.url, status, latency);

        if !result.success {
            if let Some(err) = &result.error {
                println!("  错误: {}", err);
            }
        }
    }

    // 统计
    let success_count = sorted_results.iter().filter(|r| r.success).count();
    let total_count = sorted_results.len();

    println!();
    print_info(&format!(
        "测试完成: {}/{} 成功",
        success_count, total_count
    ));

    if let Some(fastest) = sorted_results.iter().find(|r| r.success && r.latency_ms.is_some()) {
        print_info(&format!(
            "最快端点: {} ({}ms)",
            fastest.url,
            fastest.latency_ms.unwrap()
        ));
    }

    Ok(())
}